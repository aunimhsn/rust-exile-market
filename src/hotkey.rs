//! The price-check hotkey: Ctrl+D (or the keys of the settings, [`Shortcut`]) pressed in the
//! game copies the item under the cursor, so that the player no longer presses Ctrl+C.
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

mod shortcut;

pub use shortcut::Shortcut;

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
use self::windows as system;
#[cfg(target_os = "linux")]
use self::x11 as system;

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
    /// Whether the player wants it, as the settings say: also read by the handler of the hotkey
    /// events, which ignores a press while it is off (its key not released yet after a failure).
    on: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// Listens until it is dropped, its key grabbed only while the game is the active window;
    /// none when it couldn't start.
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    in_game: Option<InGame>,
    /// Each press of the hotkey.
    targets: Option<Receiver<Press>>,
    /// Its keys, also read by the handler of the hotkey events, which only copies at a press of
    /// them: the keys replaced, a press of the previous ones still on its way is ignored.
    shortcut: SharedShortcut,
}

/// The keys of the hotkey, shared with the handler of the hotkey events.
type SharedShortcut = std::sync::Arc<std::sync::Mutex<Shortcut>>;

/// The grab of the key, shared on Linux between the thread that follows the active window and the
/// thread of the window, which turns the hotkey on or off and reads whether its key is taken.
/// Held across `register` and `unregister`, which wait for the thread of `global-hotkey`, so that
/// whether the hotkey is on, its key held and its failure change together; never locked by the
/// handler of the hotkey events, nor while calling out (the wake of the window).
#[cfg(target_os = "linux")]
type SharedGrab = std::sync::Arc<std::sync::Mutex<grab::Grab>>;

/// The hotkey grabbed only while the game is the active window, followed by a thread of its own
/// until it is dropped.
#[cfg(target_os = "linux")]
struct InGame {
    manager: Manager,
    grab: SharedGrab,
    /// [`grab::Grab::taken`], written under the lock of the grab and read by each frame without
    /// it: the follow thread holds the lock while `global-hotkey` answers, up to 50 ms.
    taken: SharedTaken,
}

/// Whether the key is taken, published on Linux by whoever changes the grab.
#[cfg(target_os = "linux")]
type SharedTaken = std::sync::Arc<std::sync::atomic::AtomicBool>;

#[cfg(target_os = "linux")]
impl InGame {
    /// Starts following the active window, the hotkey `on` or off, `wake` called when its key
    /// becomes taken or free.
    fn start(
        manager: global_hotkey::GlobalHotKeyManager,
        shortcut: Shortcut,
        on: bool,
        wake: impl Fn() + Send + 'static,
    ) -> Self {
        let manager = std::sync::Arc::new(manager);
        let grab = std::sync::Arc::new(std::sync::Mutex::new(grab::Grab::new(on, shortcut)));
        let taken = SharedTaken::default();
        grab_in_game(&manager, &grab, &taken, wake);
        Self {
            manager,
            grab,
            taken,
        }
    }

    /// Turns the hotkey on or off at once.
    fn turn(&mut self, on: bool) {
        let manager = &self.manager;
        let mut grab = self.lock();
        let hotkey = grab.keys().hotkey();
        grab.turn(
            on,
            || manager.register(hotkey),
            || manager.unregister(hotkey),
        );
        self.taken
            .store(grab.taken(), std::sync::atomic::Ordering::Relaxed);
    }

    /// Replaces its keys at once.
    fn change_keys(&mut self, keys: Shortcut) {
        let manager = &self.manager;
        let mut grab = self.lock();
        let previous = grab.keys().hotkey();
        grab.change_keys(
            keys,
            || manager.register(keys.hotkey()),
            || manager.unregister(previous),
        );
        self.taken
            .store(grab.taken(), std::sync::atomic::Ordering::Relaxed);
    }

    /// The grab, held by the thread of the window (see [`SharedGrab`]).
    fn lock(&self) -> std::sync::MutexGuard<'_, grab::Grab> {
        self.grab
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Whether its key couldn't be grabbed the last time it was tried.
    fn taken(&self) -> bool {
        self.taken.load(std::sync::atomic::Ordering::Relaxed)
    }
}

/// The hotkey registered only while the game is the foreground window, on the thread of the
/// window (Windows registers it only for a window of the thread that calls it), followed by
/// [`PriceCheckHotkey::follow_game`].
#[cfg(target_os = "windows")]
struct InGame {
    manager: Manager,
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
        shortcut: Shortcut,
        on: bool,
        wake: impl Fn() + Send + 'static,
    ) -> Self {
        let foreground = system::follow_foreground(wake)
            .inspect_err(|error| {
                log::warn!("The price-check hotkey can't follow the game window: {error}");
            })
            .ok();
        let mut follow = grab::ForegroundFollow::new(on, shortcut);
        if foreground.is_none() {
            follow.lose();
        }
        let mut in_game = Self {
            manager,
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
        let (manager, hotkey) = (&self.manager, self.follow.keys().hotkey());
        self.follow.follow(
            main_focused,
            || manager.register(hotkey),
            || manager.unregister(hotkey),
        );
    }

    /// Turns the hotkey on or off at once.
    fn turn(&mut self, on: bool) {
        let (manager, hotkey) = (&self.manager, self.follow.keys().hotkey());
        self.follow.turn(
            on,
            || manager.register(hotkey),
            || manager.unregister(hotkey),
        );
    }

    /// Replaces its keys at once.
    fn change_keys(&mut self, keys: Shortcut) {
        let (manager, previous) = (&self.manager, self.follow.keys().hotkey());
        self.follow.change_keys(
            keys,
            || manager.register(keys.hotkey()),
            || manager.unregister(previous),
        );
    }

    /// Whether its key couldn't be grabbed the last time it was tried.
    fn taken(&self) -> bool {
        self.follow.taken()
    }
}

impl PriceCheckHotkey {
    /// Starts listening to `shortcut`, only while the game is the active window, and only once
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
    pub fn start(
        marker: CopyMarker,
        on: bool,
        shortcut: Shortcut,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> Self {
        use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager};
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::{Arc, Mutex, PoisonError};

        let on = Arc::new(AtomicBool::new(on));
        let keys = Arc::new(Mutex::new(shortcut));
        let off = || Self {
            on: Arc::clone(&on),
            in_game: None,
            targets: None,
            shortcut: Arc::clone(&keys),
        };
        let sender = match system::CopySender::connect() {
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
        log::info!("Price-check hotkey: {shortcut}");
        let wake = Arc::new(wake);
        let wake_in_game = Arc::clone(&wake);
        let on_at_press = Arc::clone(&on);
        let keys_at_press = Arc::clone(&keys);
        GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
            let shortcut = *keys_at_press.lock().unwrap_or_else(PoisonError::into_inner);
            // At the press or at the release of D, as each system lets the keys sent reach the
            // game; none while the hotkey is off.
            if event.id == shortcut.hotkey().id()
                && event.state == system::SENT_AT
                && on_at_press.load(Ordering::Relaxed)
            {
                let mut sender = sender.lock().unwrap_or_else(PoisonError::into_inner);
                let (copy_sender, marker) = &mut *sender;
                copy_sender.send_copy(
                    shortcut,
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
        let in_game = InGame::start(manager, shortcut, on.load(Ordering::Relaxed), move || {
            wake_in_game();
        });
        Self {
            on,
            in_game: Some(in_game),
            targets: Some(receiver),
            shortcut: keys,
        }
    }

    /// Nothing to listen to on this system.
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    #[must_use]
    pub fn start(
        _marker: CopyMarker,
        on: bool,
        shortcut: Shortcut,
        _wake: impl Fn() + Send + Sync + 'static,
    ) -> Self {
        log::info!("No price-check hotkey on this system");
        Self {
            on: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(on)),
            targets: None,
            shortcut: std::sync::Arc::new(std::sync::Mutex::new(shortcut)),
        }
    }

    /// Its keys.
    #[must_use]
    pub fn shortcut(&self) -> Shortcut {
        *self
            .shortcut
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Replaces its keys with `keys`, as the player chooses them: the previous ones released at
    /// once, the new ones grabbed at once while the game is the active window, otherwise the next
    /// time it becomes it.
    pub fn change_keys(&mut self, keys: Shortcut) {
        *self
            .shortcut
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = keys;
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        if let Some(in_game) = &mut self.in_game {
            in_game.change_keys(keys);
        }
    }

    /// Turns the hotkey on or off, as the player sets it: off, its key is released at once, left
    /// to the game and to the other applications; on, it is grabbed at once while the game is the
    /// active window.
    pub fn turn(&mut self, on: bool) {
        self.on.store(on, std::sync::atomic::Ordering::Relaxed);
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        if let Some(in_game) = &mut self.in_game {
            in_game.turn(on);
        }
    }

    /// What the hotkey does now.
    #[must_use]
    pub fn state(&self) -> HotkeyState {
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        let taken = self.in_game.as_ref().is_some_and(InGame::taken);
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        let taken = false;
        state_of(
            self.targets.is_some(),
            self.on.load(std::sync::atomic::Ordering::Relaxed),
            taken,
        )
    }
}

/// What the hotkey does, `available` when it started, `on` as the settings say, its key `taken`
/// or not: a hotkey that didn't start says nothing of its setting.
fn state_of(available: bool, on: bool, taken: bool) -> HotkeyState {
    match (available, on, taken) {
        (false, _, _) => HotkeyState::Unavailable,
        (true, false, _) => HotkeyState::Off,
        (true, true, true) => HotkeyState::Taken,
        (true, true, false) => HotkeyState::Listening,
    }
}

/// Grabs the keys of `grab` only while the game is the active window, followed on a thread of its own until
/// `manager` is dropped, so that Ctrl+D stays to the other windows; `wake` is called when its key
/// becomes taken or free, which the window tells. When the X server can't be reached, the hotkey
/// stays off, written to the log; when the desktop doesn't tell its active window, it is grabbed
/// for every window.
#[cfg(target_os = "linux")]
fn grab_in_game(
    manager: &Manager,
    grab: &SharedGrab,
    taken: &SharedTaken,
    wake: impl Fn() + Send + 'static,
) {
    let manager = std::sync::Arc::downgrade(manager);
    let grab = std::sync::Arc::clone(grab);
    let taken = std::sync::Arc::clone(taken);
    let followed = system::follow_game(move |active| {
        let Some(manager) = manager.upgrade() else {
            return false;
        };
        // The window woken once the grab is released (see `SharedGrab`).
        let changed = {
            let mut grab = grab
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            // The keys of the moment: the player may have replaced them.
            let hotkey = grab.keys().hotkey();
            grab.follow(
                active,
                || manager.register(hotkey),
                || manager.unregister(hotkey),
            );
            let now = grab.taken();
            taken.swap(now, std::sync::atomic::Ordering::Relaxed) != now
        };
        if changed {
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

/// The keys the player holds when the copy leaves, the keys of each system as `K`.
#[cfg(any(target_os = "linux", target_os = "windows"))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct Held<K> {
    control: bool,
    /// The Shift keys held, which would change the copy: Ctrl+Shift+C isn't the copy. Alt is left
    /// held, never released: Ctrl+Alt+C is the advanced copy of the game, the same text as
    /// Ctrl+C (Exiled Exchange 2 sends it on purpose), and an Alt released then pressed again
    /// could stay held for the system when the player releases it meanwhile.
    others: Vec<K>,
    /// The modifiers of the hotkey the player released before its key, while X11 gave every key
    /// to the application (until D is released): the game never saw them released, and would
    /// take them for held. Released once more for it, first; none on Windows, which leaves them
    /// to the game.
    missed: Vec<K>,
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
impl<K> Held<K> {
    /// Ctrl alone held, or none.
    #[cfg(test)]
    fn control(control: bool) -> Self {
        Self {
            control,
            others: Vec::new(),
            missed: Vec::new(),
        }
    }
}

/// The keys that copy the item, the keys of each system as `K`: C alone while the player still
/// holds Ctrl, otherwise Ctrl pressed around it, then released, so that the keyboard is left as
/// the player holds it. Releasing a Ctrl the player holds would make the system take it for
/// released, and the next D of the player would reach the game alone. The Shift the player holds
/// (Ctrl+Shift+D) is released first, so that the game reads Ctrl+C, then pressed again last, for
/// the same reason; Alt stays held ([`Held::others`]); the modifiers the game missed released
/// ([`Held::missed`]) are released before all.
#[cfg(any(target_os = "linux", target_os = "windows"))]
fn copy_keys<K: Copy>(control: K, c: K, held: &Held<K>) -> Vec<Key<K>> {
    let mut keys: Vec<_> = held
        .missed
        .iter()
        .chain(&held.others)
        .map(|&key| Key::Up(key))
        .collect();
    if held.control {
        keys.extend([Key::Down(c), Key::Up(c)]);
    } else {
        keys.extend([
            Key::Down(control),
            Key::Down(c),
            Key::Up(c),
            Key::Up(control),
        ]);
    }
    keys.extend(held.others.iter().rev().map(|&key| Key::Down(key)));
    keys
}

/// The keys of [`copy_keys`] for a game just `given_back` the focus by the window over the game:
/// a Ctrl the player holds is pressed once more, never released, since the game may not know it
/// is held, pressed while the window over the game had the focus, and would read C alone.
#[cfg(any(target_os = "windows", all(test, target_os = "linux")))]
fn copy_keys_after_focus<K: Copy>(
    control: K,
    c: K,
    held: &Held<K>,
    given_back: bool,
) -> Vec<Key<K>> {
    let mut keys = copy_keys(control, c, held);
    if given_back && held.control {
        // After the other modifiers released, just before C.
        keys.insert(held.missed.len() + held.others.len(), Key::Down(control));
    }
    keys
}

/// The modifiers released around the copy and those released once more for the game
/// ([`Held::missed`]), by their names, for the end of the line of the log: `, Shift released
/// around it, Alt released for the game`; each name once, the left and right keys alike.
#[cfg(any(target_os = "linux", target_os = "windows"))]
fn modifiers_for_log(released: &[&str], missed: &[&str]) -> String {
    let names = |names: &[&str]| {
        let mut once: Vec<&str> = Vec::new();
        for &name in names {
            if !once.contains(&name) {
                once.push(name);
            }
        }
        once.join(" and ")
    };
    [(released, "around it"), (missed, "for the game")]
        .into_iter()
        .filter(|(keys, _)| !keys.is_empty())
        .map(|(keys, what)| format!(", {} released {what}", names(keys)))
        .collect::<Vec<_>>()
        .concat()
}

/// Sends `keys` one by one with `send`, the call of each system: after a failure, only the
/// releases, so that no key is left held (a Ctrl left held would make the next D of the player
/// reach the game alone; an Alt released and not pressed again only asks the player to press it
/// again); the first error.
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
    const ALT: u8 = 64;
    const SHIFT: u8 = 50;
    const RIGHT_SHIFT: u8 = 62;

    #[test]
    fn releases_once_more_the_alt_the_game_missed_released() {
        // Alt released before D, while X11 gave the keys to the application.
        let held = Held {
            control: false,
            others: Vec::new(),
            missed: vec![ALT],
        };
        assert_eq!(
            copy_keys(CONTROL, C, &held),
            [
                Key::Up(ALT),
                Key::Down(CONTROL),
                Key::Down(C),
                Key::Up(C),
                Key::Up(CONTROL),
            ]
        );
    }

    #[test]
    fn writes_the_modifiers_released_at_the_end_of_the_log_line() {
        assert_eq!(modifiers_for_log(&[], &[]), "");
        // Both Shift keys held, named once.
        assert_eq!(
            modifiers_for_log(&["Shift", "Shift"], &[]),
            ", Shift released around it"
        );
        assert_eq!(
            modifiers_for_log(&["Shift"], &["Alt", "Alt"]),
            ", Shift released around it, Alt released for the game"
        );
    }

    #[test]
    fn releases_shift_around_the_copy_then_presses_it_again() {
        // Shift+F5: Ctrl pressed by the application, Shift released first.
        let held = Held {
            control: false,
            others: vec![SHIFT],
            missed: Vec::new(),
        };
        assert_eq!(
            copy_keys(CONTROL, C, &held),
            [
                Key::Up(SHIFT),
                Key::Down(CONTROL),
                Key::Down(C),
                Key::Up(C),
                Key::Up(CONTROL),
                Key::Down(SHIFT),
            ]
        );
    }

    #[test]
    fn keeps_the_ctrl_held_and_presses_the_others_again_in_reverse() {
        // Ctrl+Shift+D, both Shift keys held.
        let held = Held {
            control: true,
            others: vec![SHIFT, RIGHT_SHIFT],
            missed: Vec::new(),
        };
        assert_eq!(
            copy_keys(CONTROL, C, &held),
            [
                Key::Up(SHIFT),
                Key::Up(RIGHT_SHIFT),
                Key::Down(C),
                Key::Up(C),
                Key::Down(RIGHT_SHIFT),
                Key::Down(SHIFT),
            ]
        );
    }

    #[test]
    fn presses_ctrl_once_more_after_the_others_released_for_a_game_given_the_focus_back() {
        let held = Held {
            control: true,
            others: vec![SHIFT],
            missed: Vec::new(),
        };
        assert_eq!(
            copy_keys_after_focus(CONTROL, C, &held, true),
            [
                Key::Up(SHIFT),
                Key::Down(CONTROL),
                Key::Down(C),
                Key::Up(C),
                Key::Down(SHIFT),
            ]
        );
    }

    #[test]
    fn after_a_failure_the_modifiers_released_are_never_pressed_again() {
        let held = Held {
            control: false,
            others: vec![SHIFT],
            missed: Vec::new(),
        };
        let mut sent = Vec::new();
        let result = send_all(copy_keys(CONTROL, C, &held), |key| {
            sent.push(key);
            if key == Key::Down(C) {
                Err(key)
            } else {
                Ok(())
            }
        });

        assert_eq!(result, Err(Key::Down(C)));
        assert_eq!(
            sent,
            [
                Key::Up(SHIFT),
                Key::Down(CONTROL),
                Key::Down(C),
                Key::Up(C),
                Key::Up(CONTROL),
            ]
        );
    }

    #[test]
    fn the_state_told_goes_from_what_started_to_the_setting_then_to_the_key() {
        // Not started: unavailable, whatever the setting.
        assert_eq!(state_of(false, true, false), HotkeyState::Unavailable);
        assert_eq!(state_of(false, false, true), HotkeyState::Unavailable);
        // Off: never taken.
        assert_eq!(state_of(true, false, true), HotkeyState::Off);
        assert_eq!(state_of(true, true, true), HotkeyState::Taken);
        assert_eq!(state_of(true, true, false), HotkeyState::Listening);
    }

    #[test]
    fn presses_ctrl_once_more_for_a_game_given_the_focus_back() {
        assert_eq!(
            copy_keys_after_focus(CONTROL, C, &Held::control(true), true),
            [Key::Down(CONTROL), Key::Down(C), Key::Up(C)]
        );
    }

    #[test]
    fn the_game_given_the_focus_back_gets_ctrl_c_as_usual_once_ctrl_is_released() {
        assert_eq!(
            copy_keys_after_focus(CONTROL, C, &Held::control(false), true),
            copy_keys(CONTROL, C, &Held::control(false))
        );
        assert_eq!(
            copy_keys_after_focus(CONTROL, C, &Held::control(true), false),
            copy_keys(CONTROL, C, &Held::control(true))
        );
    }

    #[test]
    fn taps_c_alone_while_the_player_holds_ctrl() {
        assert_eq!(
            copy_keys(CONTROL, C, &Held::control(true)),
            [Key::Down(C), Key::Up(C)]
        );
    }

    /// The keys `send_all` sends, failing on `fails`, and what it returns.
    fn sent(control_held: bool, fails: Option<Key<u8>>) -> (Vec<Key<u8>>, Result<(), Key<u8>>) {
        let mut sent = Vec::new();
        let result = send_all(copy_keys(CONTROL, C, &Held::control(control_held)), |key| {
            sent.push(key);
            if Some(key) == fails { Err(key) } else { Ok(()) }
        });
        (sent, result)
    }

    #[test]
    fn sends_every_key_when_none_fails() {
        assert_eq!(
            sent(false, None),
            (copy_keys(CONTROL, C, &Held::control(false)), Ok(()))
        );
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
        let result = send_all(copy_keys(CONTROL, C, &Held::control(false)), |_| {
            calls += 1;
            Err(calls)
        });

        // Ctrl pressed, then the two releases.
        assert_eq!((calls, result), (3, Err(1)));
    }

    #[test]
    fn presses_ctrl_around_c_once_the_player_released_it() {
        assert_eq!(
            copy_keys(CONTROL, C, &Held::control(false)),
            [
                Key::Down(CONTROL),
                Key::Down(C),
                Key::Up(C),
                Key::Up(CONTROL)
            ]
        );
    }
}
