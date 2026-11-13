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

/// The class of the window of the game run by Steam through Proton, logged by step 14a1 in the
/// owner's game on 2026-11-01; the bug report keeps it in the log, as the name of the game.
pub const GAME_CLASS: &str = "steam_app_2694490";

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
#[cfg(any(target_os = "linux", target_os = "windows"))]
mod sent_keys;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "linux")]
mod x11;

// The copy of each system: `CopySender` (`connect` once at the start, `send_copy` at each
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
    /// The presses on their way: counted by the handler of the hotkey events before it touches
    /// the active window (on a single screen, it makes the game active again), and until the
    /// window receives them, so that the window over the game doesn't take the game made active
    /// for a copy for a window the player went to.
    on_their_way: OnTheirWay,
    /// Its keys, also read by the handler of the hotkey events, which only copies at a press of
    /// them: the keys replaced, a press of the previous ones still on its way is ignored.
    shortcut: SharedShortcut,
}

/// The presses on their way, shared with the handler of the hotkey events.
type OnTheirWay = std::sync::Arc<std::sync::atomic::AtomicUsize>;

/// A press on its way, counted twice while it lives: once for its keys, given back when it is
/// dropped, once more for the window, given back when the window receives it
/// ([`PriceCheckHotkey::receive`]), or when it is dropped without being told. Dropped also when
/// the copy panics, so that the window over the game is never left waiting.
#[cfg(any(target_os = "linux", target_os = "windows"))]
struct OnItsWay<'a> {
    count: &'a std::sync::atomic::AtomicUsize,
    told: bool,
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
impl<'a> OnItsWay<'a> {
    fn new(count: &'a std::sync::atomic::AtomicUsize) -> Self {
        count.fetch_add(2, std::sync::atomic::Ordering::SeqCst);
        Self { count, told: false }
    }

    /// The press is told to the window, which gives its count back once it receives it.
    fn told(&mut self) {
        self.told = true;
    }
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
impl Drop for OnItsWay<'_> {
    fn drop(&mut self) {
        let back = if self.told { 1 } else { 2 };
        self.count
            .fetch_sub(back, std::sync::atomic::Ordering::SeqCst);
    }
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

    /// Replaces its keys at once; the keys of the grab after the change, the previous ones when
    /// they couldn't be released.
    fn change_keys(&mut self, keys: Shortcut) -> Shortcut {
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
        grab.keys()
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

    /// Replaces its keys at once; the keys of the grab after the change, the previous ones when
    /// they couldn't be released.
    fn change_keys(&mut self, keys: Shortcut) -> Shortcut {
        let (manager, previous) = (&self.manager, self.follow.keys().hotkey());
        self.follow.change_keys(
            keys,
            || manager.register(keys.hotkey()),
            || manager.unregister(previous),
        );
        self.follow.keys()
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
        let on_their_way = OnTheirWay::default();
        let off = || Self {
            on: Arc::clone(&on),
            in_game: None,
            targets: None,
            on_their_way: Arc::clone(&on_their_way),
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
        let on_their_way_at_press = Arc::clone(&on_their_way);
        GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
            let shortcut = *keys_at_press.lock().unwrap_or_else(PoisonError::into_inner);
            if copies_at(event, shortcut, on_at_press.load(Ordering::Relaxed)) {
                let mut sender = sender.lock().unwrap_or_else(PoisonError::into_inner);
                let (copy_sender, marker) = &mut *sender;
                // Counted before the active window changes, until its keys are sent and the
                // window received it.
                let mut on_its_way = OnItsWay::new(&on_their_way_at_press);
                copy_sender.send_copy(
                    shortcut,
                    // Once the keys are sure to leave: a copy that doesn't leave would leave
                    // the marker in place of the item of the player.
                    || marker.before_copy(),
                    |target, copy| {
                        // The application stopped listening: nothing left to tell, and no
                        // window to give the count of the press back (the guard gives both).
                        if targets.send(Press { target, copy }).is_ok() {
                            on_its_way.told();
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
            on_their_way,
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
            on_their_way: OnTheirWay::default(),
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
    /// time it becomes it. When the previous ones can't be released, they stay and still copy;
    /// the new ones wait for the next launch, which reads them from the settings.
    pub fn change_keys(&mut self, keys: Shortcut) {
        // The keys the grab holds, or will hold, written for the handler once it decided: the
        // keys it grabs and the keys it copies at stay the same.
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        let keys = self
            .in_game
            .as_mut()
            .map_or(keys, |in_game| in_game.change_keys(keys));
        *self
            .shortcut
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = keys;
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
                self.on_their_way
                    .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                handle(target);
            }
        }
    }

    /// Whether a press is on its way: its keys are being sent, or the window hasn't received it
    /// yet ([`Self::receive`]). Read after the active window, which the copy may have changed.
    #[must_use]
    pub fn on_its_way(&self) -> bool {
        self.on_their_way.load(std::sync::atomic::Ordering::SeqCst) > 0
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

/// Grabs the keys of `grab` only while the game is the active window, followed on a thread of its
/// own until `manager` is dropped, so that Ctrl+D stays to the other windows; `wake` is called
/// when its key becomes taken or free, which the window tells. When the X server can't be
/// reached, the hotkey stays off, written to the log; when the desktop doesn't tell its active
/// window, it is grabbed for every window.
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

/// Whether `event` sends the copy, the hotkey of `keys` turned `on` or off: a press of these keys
/// only, so that a press of the previous ones still on its way once they are replaced is ignored;
/// at the press or at the release of D, as each system lets the keys sent reach the game; none
/// while the hotkey is off.
#[cfg(any(target_os = "linux", target_os = "windows"))]
fn copies_at(event: global_hotkey::GlobalHotKeyEvent, keys: Shortcut, on: bool) -> bool {
    on && event.id == keys.hotkey().id() && event.state == system::SENT_AT
}

#[cfg(all(test, any(target_os = "linux", target_os = "windows")))]
mod tests {
    use super::*;
    use global_hotkey::{GlobalHotKeyEvent, HotKeyState};
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// An event of the hotkey of `keys`, in `state`.
    fn event(keys: &str, state: HotKeyState) -> GlobalHotKeyEvent {
        let keys: Shortcut = keys.parse().expect("keys the hotkey can have");
        GlobalHotKeyEvent {
            id: keys.hotkey().id(),
            state,
        }
    }

    /// The state of the key at which the copy isn't sent.
    fn not_sent_at() -> HotKeyState {
        match system::SENT_AT {
            HotKeyState::Pressed => HotKeyState::Released,
            HotKeyState::Released => HotKeyState::Pressed,
        }
    }

    #[test]
    fn a_press_told_is_on_its_way_until_its_keys_are_sent_and_the_window_received_it() {
        let count = AtomicUsize::new(0);
        let mut on_its_way = OnItsWay::new(&count);
        on_its_way.told();
        // The window received it (`PriceCheckHotkey::receive`), the keys not sent yet.
        count.fetch_sub(1, Ordering::SeqCst);
        assert_eq!(count.load(Ordering::SeqCst), 1);

        drop(on_its_way);

        assert_eq!(count.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn a_press_never_told_is_no_longer_on_its_way_once_given_up() {
        let count = AtomicUsize::new(0);

        drop(OnItsWay::new(&count));

        assert_eq!(count.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn a_copy_that_panics_leaves_no_press_on_its_way() {
        let count = AtomicUsize::new(0);

        let panicked = std::panic::catch_unwind(|| {
            let _on_its_way = OnItsWay::new(&count);
            panic!("the copy failed");
        });

        assert!(panicked.is_err());
        assert_eq!(count.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn a_press_of_the_keys_copies() {
        let ctrl_g = "Ctrl+G".parse().unwrap();
        assert!(copies_at(event("Ctrl+G", system::SENT_AT), ctrl_g, true));
    }

    #[test]
    fn a_press_of_the_previous_keys_still_on_its_way_copies_nothing() {
        let ctrl_g = "Ctrl+G".parse().unwrap();
        assert!(!copies_at(event("Ctrl+D", system::SENT_AT), ctrl_g, true));
    }

    #[test]
    fn the_other_state_of_the_key_copies_nothing() {
        let ctrl_d = Shortcut::default();
        assert!(!copies_at(event("Ctrl+D", not_sent_at()), ctrl_d, true));
    }

    #[test]
    fn the_hotkey_off_copies_nothing() {
        let ctrl_d = Shortcut::default();
        assert!(!copies_at(event("Ctrl+D", system::SENT_AT), ctrl_d, false));
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
}
