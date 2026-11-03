//! The price-check hotkey: Ctrl+D pressed in the game copies the item under the cursor, so
//! that the player no longer presses Ctrl+C.
//!
//! The application sends Ctrl+C to the game itself, and the item arrives through the clipboard
//! like any copy ([`crate::clipboard`]). On Linux through X11 (XWayland under Wayland), like the
//! game run by Proton, and on Windows. None on macOS, where Path of Exile 2 has no client.
//!
//! The key is grabbed only while the game is the active window, so that Ctrl+D stays to the other
//! windows.
//!
//! Each hotkey tells the application where it was pressed, so that the item it copied opens the
//! window over the game there ([`crate::overlay`]).

use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use crate::clipboard::CopyMarker;
use crate::overlay::Target;

/// How long the game may take to copy the item after the hotkey: past it, the hotkey copied
/// nothing (an empty cell of the inventory). The clipboard watching takes an item read within it
/// for the copy of the hotkey, and tells it with [`Press::copy`].
pub const COPY_WAIT: Duration = Duration::from_secs(1);

/// A press of the hotkey, told just before its keys, so that its item never comes first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Press {
    /// Where it was pressed, when the system could tell it.
    pub target: Option<Target>,
    /// When the clipboard was prepared for its copy, which the clipboard watching gives back
    /// with the item it copied ([`crate::clipboard::ClipboardEvent::Copied`]); none when it
    /// couldn't be prepared (then its item only shows in the main window).
    pub copy: Option<Instant>,
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
mod grab;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "linux")]
mod x11;

// The keys of each system: `CopySender` (`connect` once at the start, `send_copy` at each
// hotkey) and `SENT_AT`, the press or the release of D at which the copy is sent.
#[cfg(target_os = "windows")]
use self::windows as keys;
#[cfg(target_os = "linux")]
use self::x11 as keys;

/// The hotkeys of the system, shared on Linux with the thread that grabs the key while the game is
/// the active window, which stops at the next change of the active window once they are dropped.
#[cfg(target_os = "linux")]
type Manager = std::sync::Arc<global_hotkey::GlobalHotKeyManager>;
/// The hotkeys of the system, heard by a hidden window of the thread of the window.
#[cfg(target_os = "windows")]
type Manager = global_hotkey::GlobalHotKeyManager;

/// What the hotkey does now, which the window tells the player (`specs.md`, Settings).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyState {
    /// On: its key is grabbed while the game is the active window, or will be the next time.
    Listening,
    /// Turned off in the settings.
    Off,
    /// On, but its key couldn't be grabbed the last time the game became the active window:
    /// taken by another application (Exiled Exchange 2 takes Ctrl+D too).
    Taken,
    /// None on this system (macOS), or it couldn't start (written to the log).
    Unavailable,
}

/// The hotkey listened to for as long as this value lives.
pub struct PriceCheckHotkey {
    /// Whether the player wants it, as the settings say.
    on: bool,
    /// Listens until it is dropped, grabbed while the game is the active window by a thread of its
    /// own, which shares the grab.
    #[cfg(target_os = "linux")]
    in_game: Option<(Manager, global_hotkey::hotkey::HotKey, SharedGrab)>,
    /// Listens until it is dropped, registered while the game is the foreground window.
    #[cfg(target_os = "windows")]
    in_game: Option<InGame>,
    /// Each press of the hotkey.
    targets: Option<Receiver<Press>>,
}

/// The grab of the key, shared on Linux between the thread that follows the active window and the
/// thread of the window, which turns the hotkey on or off and reads whether its key is taken.
#[cfg(target_os = "linux")]
type SharedGrab = std::sync::Arc<std::sync::Mutex<grab::Grab>>;

/// The hotkey registered only while the game is the foreground window, on the thread of the
/// window (Windows registers it only for a window of the thread that calls it), followed by
/// [`PriceCheckHotkey::follow_game`].
#[cfg(target_os = "windows")]
struct InGame {
    manager: Manager,
    hotkey: global_hotkey::hotkey::HotKey,
    /// The foreground window, read by a thread of its own; none once it can't be followed.
    foreground: Option<Receiver<grab::Foreground>>,
    follow: grab::ForegroundFollow,
}

#[cfg(target_os = "windows")]
impl InGame {
    /// Starts following the foreground window, `wake` called at each change; when it can't be
    /// followed, the key is registered for every window, as before step 14c2. The hotkey `on` or
    /// off.
    fn start(
        manager: Manager,
        hotkey: global_hotkey::hotkey::HotKey,
        on: bool,
        wake: impl Fn() + Send + 'static,
    ) -> Self {
        let foreground = keys::follow_foreground(wake)
            .inspect_err(|error| {
                log::warn!("The price-check hotkey can't follow the game window: {error}");
            })
            .ok();
        let mut follow = grab::ForegroundFollow::new(on);
        if foreground.is_none() {
            follow.lose();
        }
        let mut in_game = Self {
            manager,
            hotkey,
            foreground,
            follow,
        };
        // At once when the key falls back to every window; otherwise the first window told
        // follows.
        in_game.follow(false);
        in_game
    }

    /// Takes in the changes of the foreground window, then registers or unregisters the key for
    /// the active window they and `main_focused` make.
    fn follow(&mut self, main_focused: bool) {
        if let Some(foreground) = &self.foreground {
            loop {
                match foreground.try_recv() {
                    Ok(window) => self.follow.told(window),
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        log::warn!("The price-check hotkey can't follow the game window any more");
                        self.foreground = None;
                        self.follow.lose();
                        break;
                    }
                }
            }
        }
        let (manager, hotkey) = (&self.manager, self.hotkey);
        self.follow.follow(
            main_focused,
            || manager.register(hotkey),
            || manager.unregister(hotkey),
        );
    }

    /// Turns the hotkey on or off at once.
    fn turn(&mut self, on: bool) {
        let (manager, hotkey) = (&self.manager, self.hotkey);
        self.follow.turn(
            on,
            || manager.register(hotkey),
            || manager.unregister(hotkey),
        );
    }
}

impl PriceCheckHotkey {
    /// Starts listening to Ctrl+D, only while the game is the active window, and only once
    /// turned on when it starts `on` off (the settings). A key taken by another application, such
    /// as Exiled Exchange 2, is written to the log and told by [`Self::state`]: the player can
    /// still copy with Ctrl+C.
    /// `marker` prepares the clipboard before each copy; `wake` is called after each hotkey, when
    /// the key becomes taken or free on Linux, and on Windows at each change of the foreground
    /// window, which [`Self::follow_game`] then takes in.
    ///
    /// Called once: the handler of the hotkey events can only be set once. Called on the thread
    /// of the window: on Windows, the hotkey is heard by a hidden window of that thread, through
    /// its event loop, and registered there.
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    #[must_use]
    pub fn start(marker: CopyMarker, on: bool, wake: impl Fn() + Send + Sync + 'static) -> Self {
        use global_hotkey::hotkey::{Code, HotKey, Modifiers};
        use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager};
        use std::sync::{Arc, Mutex, PoisonError};

        let off = || Self {
            on,
            in_game: None,
            targets: None,
        };
        let sender = match keys::CopySender::connect() {
            Ok(sender) => Mutex::new((sender, marker)),
            Err(error) => {
                log::warn!("The price-check hotkey can't listen: {error}");
                return off();
            }
        };
        let manager = match GlobalHotKeyManager::new() {
            Ok(manager) => manager,
            Err(error) => {
                log::warn!("The price-check hotkey can't listen: {error}");
                return off();
            }
        };
        let (targets, receiver) = std::sync::mpsc::channel();
        let hotkey = HotKey::new(Some(Modifiers::CONTROL), Code::KeyD);
        let id = hotkey.id();
        let wake = Arc::new(wake);
        #[cfg(target_os = "windows")]
        let wake_at_foreground = Arc::clone(&wake);
        #[cfg(target_os = "linux")]
        let wake_at_taken = Arc::clone(&wake);
        GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
            // At the press or at the release of D, as each system lets the keys sent reach the
            // game.
            if event.id == id && event.state == keys::SENT_AT {
                let mut sender = sender.lock().unwrap_or_else(PoisonError::into_inner);
                let (keys, marker) = &mut *sender;
                keys.send_copy(
                    || prepare_copy(marker),
                    |target, copy| {
                        // The application stopped listening: nothing left to tell.
                        if targets.send(Press { target, copy }).is_ok() {
                            wake();
                        }
                    },
                );
            }
        }));
        #[cfg(target_os = "linux")]
        let in_game = {
            let manager = Arc::new(manager);
            let grab = Arc::new(Mutex::new(grab::Grab::new(on)));
            grab_in_game(&manager, hotkey, &grab, move || wake_at_taken());
            (manager, hotkey, grab)
        };
        #[cfg(target_os = "windows")]
        let in_game = InGame::start(manager, hotkey, on, move || {
            wake_at_foreground();
        });
        Self {
            on,
            in_game: Some(in_game),
            targets: Some(receiver),
        }
    }

    /// Nothing to listen to on this system.
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    #[must_use]
    pub fn start(_marker: CopyMarker, on: bool, _wake: impl Fn() + Send + Sync + 'static) -> Self {
        log::info!("No price-check hotkey on this system");
        Self { on, targets: None }
    }

    /// Turns the hotkey on or off, as the player sets it: off, its key is released at once, left
    /// to the game and to the other applications; on, it is grabbed at once while the game is the
    /// active window.
    pub fn turn(&mut self, on: bool) {
        self.on = on;
        #[cfg(target_os = "linux")]
        if let Some((manager, hotkey, grab)) = &self.in_game {
            grab.lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .turn(
                    on,
                    || manager.register(*hotkey),
                    || manager.unregister(*hotkey),
                );
        }
        #[cfg(target_os = "windows")]
        if let Some(in_game) = &mut self.in_game {
            in_game.turn(on);
        }
    }

    /// What the hotkey does now.
    #[must_use]
    pub fn state(&self) -> HotkeyState {
        if self.targets.is_none() {
            HotkeyState::Unavailable
        } else if !self.on {
            HotkeyState::Off
        } else if self.taken() {
            HotkeyState::Taken
        } else {
            HotkeyState::Listening
        }
    }

    /// Whether its key couldn't be grabbed the last time it was tried.
    fn taken(&self) -> bool {
        #[cfg(target_os = "linux")]
        if let Some((_, _, grab)) = &self.in_game {
            return grab
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .taken();
        }
        #[cfg(target_os = "windows")]
        if let Some(in_game) = &self.in_game {
            return in_game.follow.taken();
        }
        false
    }
}

/// Grabs `hotkey` only while the game is the active window, followed on a thread of its own until
/// `manager` is dropped, so that Ctrl+D stays to the other windows; `wake` is called when its key
/// becomes taken or free, which the window tells. When the X server can't be reached, the hotkey
/// stays off, written to the log; when the desktop doesn't tell its active window, it is grabbed
/// for every window.
#[cfg(target_os = "linux")]
fn grab_in_game(
    manager: &Manager,
    hotkey: global_hotkey::hotkey::HotKey,
    grab: &SharedGrab,
    wake: impl Fn() + Send + 'static,
) {
    let manager = std::sync::Arc::downgrade(manager);
    let grab = std::sync::Arc::clone(grab);
    let followed = keys::follow_game(move |active| {
        let Some(manager) = manager.upgrade() else {
            return false;
        };
        let mut grab = grab
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let was_taken = grab.taken();
        grab.follow(
            active,
            || manager.register(hotkey),
            || manager.unregister(hotkey),
        );
        if grab.taken() != was_taken {
            wake();
        }
        true
    });
    if let Err(error) = followed {
        log::warn!("The price-check hotkey can't follow the game window: {error}");
    }
}

/// Prepares the clipboard for the copy, just before the keys, once they are sure to leave: a
/// copy that doesn't leave would leave the marker in place of the item of the player; when,
/// which the item copied carries back.
#[cfg(any(target_os = "linux", target_os = "windows"))]
fn prepare_copy(marker: &mut CopyMarker) -> Option<Instant> {
    marker.before_copy()
}

/// A key pressed or released.
#[cfg(any(target_os = "linux", target_os = "windows"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Key<K> {
    Down(K),
    Up(K),
}

/// The keys that copy the item, the keys of each system as `K`: C alone while the player still
/// holds Ctrl, otherwise Ctrl pressed around it, then released, so that the keyboard is left as
/// the player holds it. Releasing a Ctrl the player holds would make the system take it for
/// released, and the next D of the player would reach the game alone.
#[cfg(any(target_os = "linux", target_os = "windows"))]
fn copy_keys<K: Copy>(control: K, c: K, control_held: bool) -> Vec<Key<K>> {
    if control_held {
        vec![Key::Down(c), Key::Up(c)]
    } else {
        vec![
            Key::Down(control),
            Key::Down(c),
            Key::Up(c),
            Key::Up(control),
        ]
    }
}

/// The keys of [`copy_keys`] for a game just `given_back` the focus by the window over the game:
/// a Ctrl the player holds is pressed once more, never released, since the game may not know it
/// is held, pressed while the window over the game had the focus, and would read C alone.
#[cfg(any(target_os = "windows", all(test, target_os = "linux")))]
fn copy_keys_after_focus<K: Copy>(
    control: K,
    c: K,
    control_held: bool,
    given_back: bool,
) -> Vec<Key<K>> {
    let mut keys = copy_keys(control, c, control_held);
    if given_back && control_held {
        keys.insert(0, Key::Down(control));
    }
    keys
}

/// Sends `keys` one by one with `send`, the call of each system: after a failure, only the
/// releases, so that no key is left held (a Ctrl left held would make the next D of the player
/// reach the game alone); the first error.
#[cfg(any(target_os = "linux", target_os = "windows"))]
fn send_all<K, E>(
    keys: Vec<Key<K>>,
    mut send: impl FnMut(Key<K>) -> Result<(), E>,
) -> Result<(), E> {
    let mut result = Ok(());
    for key in keys {
        if result.is_err() && matches!(key, Key::Down(_)) {
            continue;
        }
        // Sent even after a failure: `and` keeps the first error.
        result = result.and(send(key));
    }
    result
}

impl PriceCheckHotkey {
    /// At each frame, on the thread of the window: on Windows, registers the key while the game
    /// is the foreground window, or the window over the game after a click in it; unregisters it
    /// otherwise, as for the main window when it has the keyboard (`main_focused`). Nothing on the
    /// other systems, which follow the game on a thread of their own.
    #[cfg_attr(
        not(target_os = "windows"),
        expect(clippy::unused_self, reason = "the same call on every system")
    )]
    pub fn follow_game(&mut self, main_focused: bool) {
        #[cfg(target_os = "windows")]
        if let Some(in_game) = &mut self.in_game {
            in_game.follow(main_focused);
        }
        #[cfg(not(target_os = "windows"))]
        let _ = main_focused;
    }

    /// Gives each press of the hotkey since the last call to `handle`, without waiting.
    pub fn receive(&mut self, mut handle: impl FnMut(Press)) {
        if let Some(targets) = &self.targets {
            while let Ok(target) = targets.try_recv() {
                handle(target);
            }
        }
    }
}

#[cfg(all(test, any(target_os = "linux", target_os = "windows")))]
mod tests {
    use super::*;

    const CONTROL: u8 = 37;
    const C: u8 = 54;

    #[test]
    fn presses_ctrl_once_more_for_a_game_given_the_focus_back() {
        assert_eq!(
            copy_keys_after_focus(CONTROL, C, true, true),
            [Key::Down(CONTROL), Key::Down(C), Key::Up(C)]
        );
    }

    #[test]
    fn the_game_given_the_focus_back_gets_ctrl_c_as_usual_once_ctrl_is_released() {
        assert_eq!(
            copy_keys_after_focus(CONTROL, C, false, true),
            copy_keys(CONTROL, C, false)
        );
        assert_eq!(
            copy_keys_after_focus(CONTROL, C, true, false),
            copy_keys(CONTROL, C, true)
        );
    }

    #[test]
    fn taps_c_alone_while_the_player_holds_ctrl() {
        assert_eq!(copy_keys(CONTROL, C, true), [Key::Down(C), Key::Up(C)]);
    }

    /// The keys `send_all` sends, failing on `fails`, and what it returns.
    fn sent(control_held: bool, fails: Option<Key<u8>>) -> (Vec<Key<u8>>, Result<(), Key<u8>>) {
        let mut sent = Vec::new();
        let result = send_all(copy_keys(CONTROL, C, control_held), |key| {
            sent.push(key);
            if Some(key) == fails { Err(key) } else { Ok(()) }
        });
        (sent, result)
    }

    #[test]
    fn sends_every_key_when_none_fails() {
        assert_eq!(sent(false, None), (copy_keys(CONTROL, C, false), Ok(())));
    }

    #[test]
    fn after_a_failed_c_only_the_releases_are_sent() {
        assert_eq!(
            sent(false, Some(Key::Down(C))),
            (
                vec![
                    Key::Down(CONTROL),
                    Key::Down(C),
                    Key::Up(C),
                    Key::Up(CONTROL)
                ],
                Err(Key::Down(C))
            )
        );
    }

    #[test]
    fn after_a_failed_ctrl_c_is_never_pressed() {
        assert_eq!(
            sent(false, Some(Key::Down(CONTROL))),
            (
                vec![Key::Down(CONTROL), Key::Up(C), Key::Up(CONTROL)],
                Err(Key::Down(CONTROL))
            )
        );
    }

    #[test]
    fn keeps_the_first_error() {
        let mut calls = 0;
        let result = send_all(copy_keys(CONTROL, C, false), |_| {
            calls += 1;
            Err(calls)
        });

        // Ctrl pressed, then the two releases.
        assert_eq!((calls, result), (3, Err(1)));
    }

    #[test]
    fn presses_ctrl_around_c_once_the_player_released_it() {
        assert_eq!(
            copy_keys(CONTROL, C, false),
            [
                Key::Down(CONTROL),
                Key::Down(C),
                Key::Up(C),
                Key::Up(CONTROL)
            ]
        );
    }
}
