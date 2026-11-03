//! The copy sent to the game under Windows: C pressed through `SendInput`, as the player would
//! press it, in the window that has the focus, with Ctrl when the player already released it;
//! and the foreground window followed, so that the hotkey is registered only while it is the
//! game.

use std::collections::HashSet;
use std::error::Error;
use std::io;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

use enigo::{Direction, Enigo, Keyboard as _, Settings};
use global_hotkey::HotKeyState;
use winsafe::{HWND, co};

use super::grab::{Foreground, GAME_TITLE};
use super::{Key, copy_keys_after_focus, send_all};
use crate::overlay::{KeyboardBack, Point, ScreenRect, Target};

/// How often the foreground window is read. Windows tells its changes only to a hook
/// (`SetWinEventHook`), whose callback can't be written without `unsafe`: read 20 times a second
/// instead, a few microseconds each, so that the hotkey is registered within 50 ms of the game
/// coming to the front (100 ms more while the main window is minimized, which eframe updates ten
/// times a second then).
const FOREGROUND_POLL: Duration = Duration::from_millis(50);

/// At the press of D: `RegisterHotKey` takes D only, so that the keys sent then reach the game,
/// and Ctrl is held, since the hotkey needs it. The press reaches the application as a message of
/// the thread of its window, handled within a few milliseconds, and the clipboard is prepared
/// before the keys: Ctrl is read just before them, and pressed by the application when the
/// player released it meanwhile.
pub const SENT_AT: HotKeyState = HotKeyState::Pressed;

/// Sends the keys of the copy.
pub struct CopySender {
    keyboard: Enigo,
}

impl CopySender {
    /// # Errors
    ///
    /// When the keys can't be sent on this system.
    pub fn connect() -> Result<Self, Box<dyn Error>> {
        let settings = Settings {
            // The keys it holds when dropped: none, since every key it presses is released.
            release_keys_when_dropped: false,
            ..Settings::default()
        };
        Ok(Self {
            keyboard: Enigo::new(&settings)?,
        })
    }

    /// Sends C to the window that has the focus, with the Ctrl the player holds, or pressed around
    /// it when they released it already, so that the game reads Ctrl+C and the keyboard is left as
    /// the player holds it: a Ctrl released while the player holds it would make their next D
    /// reach the game alone, as Exiled Exchange 2 avoids on Windows (`keepModKeys` in
    /// `main/src/shortcuts/Shortcuts.ts`).
    ///
    /// While a field of the window over the game is edited, the game gets the focus back first;
    /// when Windows refuses it, nothing is copied. Once the window is read, `prepare` runs (it may
    /// take a moment), then the pointer is read, as the game sees it when the keys leave, and
    /// `tell` gets where the window over the game opens, when the pointer and the window could
    /// be read, with what `prepare` gave; Ctrl is read last, just before the keys.
    pub fn send_copy<P>(
        &mut self,
        prepare: impl FnOnce() -> P,
        tell: impl FnOnce(Option<Target>, P),
    ) {
        // While a field of the window over the game is edited, the keys of the copy would go to
        // it: the game gets the focus back first. Refused, the copy would copy nothing, and the
        // marker would take the place of the item of the player in the clipboard.
        let given_back = match crate::overlay::give_keyboard_back() {
            KeyboardBack::NotHeld => false,
            KeyboardBack::Given => true,
            KeyboardBack::Refused => return,
        };
        // The window the keys go to, read before them; for the log and the window over the
        // game, so that a failure doesn't stop the copy.
        let window = HWND::GetForegroundWindow();
        // Its class, never its title, which may be the page of a browser (Ctrl+D adds a bookmark
        // there) or the subject of a mail: the log may be sent with a bug report.
        let class = window.as_ref().map_or_else(
            || "none".to_owned(),
            |window| {
                window
                    .GetClassName()
                    .unwrap_or_else(|error| format!("unknown ({error})"))
            },
        );
        let game = window.as_ref().and_then(|window| {
            client_rect(window).unwrap_or_else(|error| {
                log::warn!("The window of the game can't be measured: {error}");
                None
            })
        });
        let prepared = prepare();
        // Read after the preparation, which may take a moment: where the item is when the keys
        // leave.
        let pointer = winsafe::GetCursorPos()
            .inspect_err(|error| log::warn!("The pointer can't be read: {error}"))
            .ok();
        let target = game.zip(pointer).map(|(game, pointer)| Target {
            pointer: Point {
                x: pointer.x,
                y: pointer.y,
            },
            game,
        });
        tell(target, prepared);
        // Read last, just before the keys, as the player holds it: a Ctrl released meanwhile
        // and pressed once more for a game given the focus back would stay held.
        let control_held = winsafe::GetAsyncKeyState(co::VK::CONTROL);
        // Windows doesn't tell a key it drops: a game run as administrator never gets it.
        match self.send_keys(control_held, given_back) {
            Ok(()) => log::info!(
                "Copy requested by the price-check hotkey from the window of class {class} (Ctrl {})",
                if control_held {
                    "still held"
                } else {
                    "pressed by the application"
                }
            ),
            Err(error) => log::warn!("The copy can't be sent: {error}"),
        }
    }

    /// The keys of [`copy_keys`], as virtual keys: C, which Windows maps to the key that writes C
    /// in the layout of the window that has the focus; after a failure, only the releases
    /// ([`send_all`]). When the game was just `given_back` the focus, a Ctrl the player holds is
    /// pressed once more ([`copy_keys_after_focus`]).
    fn send_keys(&mut self, control_held: bool, given_back: bool) -> enigo::InputResult<()> {
        let keys =
            copy_keys_after_focus(enigo::Key::Control, enigo::Key::C, control_held, given_back);
        send_all(keys, |key| match key {
            Key::Down(key) => self.keyboard.key(key, Direction::Press),
            Key::Up(key) => self.keyboard.key(key, Direction::Release),
        })
    }
}

/// Follows the foreground window on a thread of its own, and sends what it is: at the start, then
/// at each change, calling `wake` after each, so that the thread of the window registers or
/// unregisters the hotkey (Windows allows it only there). The thread ends at the first change
/// after the receiver is dropped.
///
/// # Errors
///
/// When the thread can't start.
pub fn follow_foreground(wake: impl Fn() + Send + 'static) -> io::Result<Receiver<Foreground>> {
    let (sender, receiver) = mpsc::channel();
    thread::Builder::new()
        .name("game-window".to_owned())
        .spawn(move || {
            let mut classes_told = HashSet::new();
            let mut last = None;
            loop {
                let foreground = foreground(&mut classes_told);
                if last != Some(foreground) {
                    if sender.send(foreground).is_err() {
                        return;
                    }
                    wake();
                    last = Some(foreground);
                }
                thread::sleep(FOREGROUND_POLL);
            }
        })?;
    Ok(receiver)
}

/// What the foreground window is: the game by its exact title, read for the windows of the other
/// applications only (the title of a window of this process is read through a message to the
/// thread of the window, which this one would wait for). The
/// class of another window is written to the log the first time it is in front, never its title
/// (the page of a browser), so that a client of the game titled otherwise can be found there.
fn foreground(classes_told: &mut HashSet<String>) -> Foreground {
    let Some(window) = HWND::GetForegroundWindow() else {
        return Foreground::None;
    };
    if crate::overlay::is_ours(&window) {
        return Foreground::Ours;
    }
    if window
        .GetWindowText()
        .is_ok_and(|title| title == GAME_TITLE)
    {
        return Foreground::Game;
    }
    let class = window
        .GetClassName()
        .unwrap_or_else(|error| format!("unknown ({error})"));
    if !classes_told.contains(&class) {
        log::info!("The foreground window, of class {class}, isn't the game");
        classes_told.insert(class);
    }
    Foreground::Other
}

/// Where the inside of `window` is on the screen, without its frame and title bar, in physical
/// pixels; none when it has no size (minimized).
fn client_rect(window: &HWND) -> winsafe::SysResult<Option<ScreenRect>> {
    let rect = window.ClientToScreenRc(window.GetClientRect()?)?;
    let rect = ScreenRect {
        x: rect.left,
        y: rect.top,
        width: rect.right - rect.left,
        height: rect.bottom - rect.top,
    };
    Ok((rect.width > 0 && rect.height > 0).then_some(rect))
}
