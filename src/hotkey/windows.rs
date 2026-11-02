//! The copy sent to the game under Windows: C pressed through `SendInput`, as the player would
//! press it, in the window that has the focus, with Ctrl when the player already released it.

use std::error::Error;

use enigo::{Direction, Enigo, Keyboard as _, Settings};
use global_hotkey::HotKeyState;
use winsafe::{HWND, co};

use super::{Key, copy_keys};
use crate::overlay::{Point, ScreenRect, Target};

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
    /// Once the window is read, `prepare` runs (it may take a moment), then the pointer and Ctrl
    /// are read, as the game sees them when the keys leave, and `tell` gets where the window over
    /// the game opens, when the pointer and the window could be read, with what `prepare` gave,
    /// just before the keys.
    pub fn send_copy<P>(
        &mut self,
        prepare: impl FnOnce() -> P,
        tell: impl FnOnce(Option<Target>, P),
    ) {
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
        // Read after the preparation, which may take a moment: where the item is, and Ctrl as
        // the player holds it, when the keys leave.
        let pointer = winsafe::GetCursorPos()
            .inspect_err(|error| log::warn!("The pointer can't be read: {error}"))
            .ok();
        let control_held = winsafe::GetAsyncKeyState(co::VK::CONTROL);
        let target = game.zip(pointer).map(|(game, pointer)| Target {
            pointer: Point {
                x: pointer.x,
                y: pointer.y,
            },
            game,
        });
        tell(target, prepared);
        // Windows doesn't tell a key it drops: a game run as administrator never gets it.
        match self.send_keys(control_held) {
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
    /// in the layout of the window that has the focus. After a failure, only the releases are
    /// sent, so that Ctrl is never left held.
    fn send_keys(&mut self, control_held: bool) -> enigo::InputResult<()> {
        let mut result = Ok(());
        for key in copy_keys(enigo::Key::Control, enigo::Key::C, control_held) {
            let (key, direction) = match key {
                Key::Down(key) => (key, Direction::Press),
                Key::Up(key) => (key, Direction::Release),
            };
            if result.is_err() && direction == Direction::Press {
                continue;
            }
            let sent = self.keyboard.key(key, direction);
            if result.is_ok() {
                result = sent;
            }
        }
        result
    }
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
