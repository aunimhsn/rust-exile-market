//! The hotkey grabbed only while the game is the active window, so that Ctrl+D stays to the other
//! windows (`specs.md`, Item detection): grabbed when the game becomes the active window, released
//! when another one does; registered and unregistered on Windows alike.
//!
//! Without the system, so that the rules are tested: the grab and the release come as closures.

use std::fmt::Display;

use super::Shortcut;

/// The exact title of the window of the game: the title Exiled Exchange 2 looks for
/// (`attachByTitle`), the standalone client and the one of Steam alike on Windows, and the
/// standalone client run by Wine on Linux, whose class is unknown. Never a part of the title: the
/// page of a browser about the game is titled `Path of Exile 2 - …`.
pub const GAME_TITLE: &str = "Path of Exile 2";

/// The class of a window of the File Explorer of Windows, titled after its folder: the folder of
/// the game is titled `Path of Exile 2` too, and Ctrl+D there must stay to it (a copy sent there
/// would copy the files selected).
#[cfg(any(target_os = "windows", test))]
const EXPLORER_CLASS: &str = "CabinetWClass";

/// Whether a window of another application, of `title` and `class`, is the game on Windows: its
/// exact title, unless it is a folder of the File Explorer.
#[cfg(any(target_os = "windows", test))]
pub fn is_game_window(title: &str, class: &str) -> bool {
    title == GAME_TITLE && class != EXPLORER_CLASS
}

/// The active window, as the desktop tells it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Active {
    /// The game.
    Game,
    /// Another window.
    Other,
    /// No active window told: a window outside X11 (Wayland), or maybe the window over the game
    /// keeping the keyboard on a desktop other than `KWin`; on Windows, no foreground window for
    /// a moment, or the window over the game in front (a field edited).
    Unknown,
    /// The active window can't be followed (a desktop that doesn't tell it, the X server lost):
    /// the key falls back to every window, as before it followed the game, rather than never
    /// working.
    NotFollowed,
}

/// The foreground window on Windows, as the thread that follows it reads it.
#[cfg(any(target_os = "windows", test))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Foreground {
    /// The game, by its exact title ([`GAME_TITLE`]).
    Game,
    /// A window of this application: the window over the game, in front after a click in it
    /// (until the game gets the focus back, or while one of its fields is edited), or the main
    /// window.
    Ours,
    /// Another window.
    Other,
    /// No foreground window: Windows tells none for a moment while the foreground changes.
    NoWindow,
}

#[cfg(any(target_os = "windows", test))]
impl Foreground {
    /// The active window it is for the hotkey, the main window having the keyboard or not
    /// (`main_focused`, as egui tells it): a window of this application in front while the main
    /// window hasn't the keyboard is the window over the game, which leaves the key as it is, so
    /// that Ctrl+D still copies while one of its fields is edited (giving the keyboard back to the
    /// game first); the main window in front releases the key, as another window.
    pub fn active(self, main_focused: bool) -> Active {
        match self {
            Self::Game => Active::Game,
            Self::Ours if !main_focused => Active::Unknown,
            Self::Ours | Self::Other => Active::Other,
            Self::NoWindow => Active::Unknown,
        }
    }
}

/// What a change of the active window did, for the tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// Nothing to do: the key is already as the active window wants it.
    Nothing,
    /// The key grabbed, the game being the active window, or the active window not followed.
    Grabbed,
    /// The key released, another window being the active window.
    Released,
    /// The key couldn't be grabbed (taken by another application) or released.
    Failed,
}

/// Whether the key is grabbed, followed at each change of the active window.
#[derive(Debug)]
pub struct Grab {
    /// Whether the player wants the hotkey (the settings): off, the key is never grabbed.
    on: bool,
    /// Whether the key is grabbed.
    held: bool,
    /// The last active window told, so that the hotkey turned on grabs the key at once while the
    /// game is the active window.
    last: Option<Active>,
    /// The last failure to grab it, written to the log once until it changes: the key taken by
    /// another application fails at each activation of the game. Kept until the key is grabbed or
    /// the hotkey turned off, so that the player is told the key is taken.
    failure: Option<String>,
    /// Whether the log already said the key is listened to, so that it doesn't say it at each
    /// activation of the game.
    told: bool,
    /// The keys grabbed, or to grab.
    keys: Shortcut,
}

impl Default for Grab {
    fn default() -> Self {
        Self::new(true, Shortcut::default())
    }
}

impl Grab {
    /// The key not grabbed yet, the hotkey `on` or off, its `keys` written in the log.
    pub fn new(on: bool, keys: Shortcut) -> Self {
        Self {
            on,
            held: false,
            last: None,
            failure: None,
            told: false,
            keys,
        }
    }

    /// Whether the hotkey is on and its key couldn't be grabbed the last time it was tried: taken
    /// by another application, the likeliest case.
    pub fn taken(&self) -> bool {
        self.on && self.failure.is_some()
    }

    /// The keys grabbed, or to grab.
    pub fn keys(&self) -> Shortcut {
        self.keys
    }

    /// Replaces the keys with `keys`, chosen by the player: the previous ones released at once with
    /// `release` when they are held, then the new ones grabbed with `grab` as the last active
    /// window wants them, at once while the game is the active window. Whether the new keys are
    /// taken is only known once they are tried: the failure of the previous ones is forgotten.
    /// When the previous keys can't be released, they stay, tried again at the next change; the
    /// new keys wait for the next launch, which reads them from the settings.
    pub fn change_keys<E: Display>(
        &mut self,
        keys: Shortcut,
        grab: impl FnOnce() -> Result<(), E>,
        release: impl FnOnce() -> Result<(), E>,
    ) -> Change {
        if keys == self.keys {
            return Change::Nothing;
        }
        if self.held && self.release(release) == Change::Failed {
            return Change::Failed;
        }
        log::info!("Price-check hotkey changed from {} to {keys}", self.keys);
        self.keys = keys;
        self.failure = None;
        self.told = false;
        match self.last {
            // Not held any more: `follow` releases nothing.
            Some(active) if self.on => self.follow(active, grab, || Ok::<(), E>(())),
            _ => Change::Nothing,
        }
    }

    /// Turns the hotkey on or off, as the player sets it: off, the key is released at once (a
    /// failed release tried again at the next change); on, it is grabbed at once while the game is
    /// the active window, or the active window not followed.
    pub fn turn<E: Display>(
        &mut self,
        on: bool,
        grab: impl FnOnce() -> Result<(), E>,
        release: impl FnOnce() -> Result<(), E>,
    ) -> Change {
        if std::mem::replace(&mut self.on, on) == on {
            return Change::Nothing;
        }
        if on {
            log::info!("Price-check hotkey turned on");
            match self.last {
                Some(active) => self.follow(active, grab, release),
                None => Change::Nothing,
            }
        } else {
            log::info!("Price-check hotkey turned off");
            self.failure = None;
            if self.held {
                self.release(release)
            } else {
                Change::Nothing
            }
        }
    }

    /// Follows the active window: grabs the key with `grab` when the game becomes the active
    /// window, and releases it with `release` when another window does. A failed grab is tried
    /// again at the next change, so that the hotkey comes back once the application that took the
    /// key quits. With no active window told, the key stays as it is, so that Ctrl+D still gives
    /// the keyboard back to the game from the window over the game; a window outside X11 never
    /// hears a key grabbed under X11 anyway. Once the active window can't be followed, the key is
    /// grabbed for every window. Each call tries a failed grab again: call it once per change of
    /// the active window.
    pub fn follow<E: Display>(
        &mut self,
        active: Active,
        grab: impl FnOnce() -> Result<(), E>,
        release: impl FnOnce() -> Result<(), E>,
    ) -> Change {
        self.last = Some(active);
        if !self.on {
            // A release that failed when the hotkey was turned off.
            return if self.held {
                self.release(release)
            } else {
                Change::Nothing
            };
        }
        match (active, self.held) {
            (Active::Game, false) => self.grab(grab, false),
            (Active::NotFollowed, false) => self.grab(grab, true),
            (Active::NotFollowed, true) => {
                log::info!(
                    "Price-check hotkey {} listened to in every window",
                    self.keys
                );
                Change::Nothing
            }
            (Active::Other, true) => self.release(release),
            (Active::Game, true) | (Active::Other, false) | (Active::Unknown, _) => Change::Nothing,
        }
    }

    /// Releases the key with `release`; still held after a failure, tried again at the next
    /// change.
    fn release<E: Display>(&mut self, release: impl FnOnce() -> Result<(), E>) -> Change {
        match release() {
            Ok(()) => {
                self.held = false;
                Change::Released
            }
            Err(error) => {
                log::warn!(
                    "The price-check hotkey {} can't be released: {error}",
                    self.keys
                );
                Change::Failed
            }
        }
    }

    /// Grabs the key with `grab`, for the game only or for `every_window`, which the log always
    /// says, since no change of the active window follows it.
    fn grab<E: Display>(
        &mut self,
        grab: impl FnOnce() -> Result<(), E>,
        every_window: bool,
    ) -> Change {
        match grab() {
            Ok(()) => {
                self.held = true;
                let first = !std::mem::replace(&mut self.told, true);
                // No longer taken, whatever window it is grabbed for.
                let recovered = self.failure.take().is_some();
                if every_window {
                    log::info!(
                        "Price-check hotkey {} listened to in every window",
                        self.keys
                    );
                } else if recovered || first {
                    log::info!(
                        "Price-check hotkey {} listened to while the game is the active window",
                        self.keys
                    );
                }
                Change::Grabbed
            }
            Err(error) => {
                let error = error.to_string();
                if every_window || self.failure.as_ref() != Some(&error) {
                    log::warn!(
                        "The price-check hotkey {} can't be listened to: {error}",
                        self.keys
                    );
                    self.failure = Some(error);
                }
                Change::Failed
            }
        }
    }
}

/// The foreground window followed on Windows, on the thread of the window: the changes the thread
/// that reads it sends are taken in, then the key follows the active window they make, once per
/// change (a failed grab is tried again at the next change, as on Linux, not at each frame).
#[cfg(any(target_os = "windows", test))]
#[derive(Debug)]
pub struct ForegroundFollow {
    /// The last foreground window told.
    last: Foreground,
    /// The foreground window can no longer be followed: the key is grabbed for every window.
    lost: bool,
    /// The active window the key last followed.
    followed: Option<Active>,
    grab: Grab,
}

#[cfg(any(target_os = "windows", test))]
impl Default for ForegroundFollow {
    fn default() -> Self {
        Self::new(true, Shortcut::default())
    }
}

#[cfg(any(target_os = "windows", test))]
impl ForegroundFollow {
    /// No foreground window told yet, the hotkey `on` or off, its `keys` written in the log.
    pub fn new(on: bool, keys: Shortcut) -> Self {
        Self {
            last: Foreground::NoWindow,
            lost: false,
            followed: None,
            grab: Grab::new(on, keys),
        }
    }
}

#[cfg(any(target_os = "windows", test))]
impl ForegroundFollow {
    /// A change of the foreground window, told by the thread that reads it.
    pub fn told(&mut self, foreground: Foreground) {
        self.last = foreground;
    }

    /// The foreground window can no longer be followed (its thread didn't start, or stopped):
    /// from the next [`Self::follow`], the key is grabbed for every window, as before step 14c2.
    pub fn lose(&mut self) {
        self.lost = true;
    }

    /// Whether the key of the hotkey is taken ([`Grab::taken`]).
    pub fn taken(&self) -> bool {
        self.grab.taken()
    }

    /// The keys grabbed, or to grab.
    pub fn keys(&self) -> Shortcut {
        self.grab.keys()
    }

    /// Replaces the keys ([`Grab::change_keys`]): `grab` grabs the new ones, `release` releases
    /// the previous ones.
    pub fn change_keys<E: Display>(
        &mut self,
        keys: Shortcut,
        grab: impl FnOnce() -> Result<(), E>,
        release: impl FnOnce() -> Result<(), E>,
    ) -> Change {
        self.grab.change_keys(keys, grab, release)
    }

    /// Turns the hotkey on or off ([`Grab::turn`]).
    pub fn turn<E: Display>(
        &mut self,
        on: bool,
        grab: impl FnOnce() -> Result<(), E>,
        release: impl FnOnce() -> Result<(), E>,
    ) -> Change {
        self.grab.turn(on, grab, release)
    }

    /// Grabs or releases the key for the active window that the last foreground window told and
    /// `main_focused` make, when it changed since the last call.
    pub fn follow<E: Display>(
        &mut self,
        main_focused: bool,
        grab: impl FnOnce() -> Result<(), E>,
        release: impl FnOnce() -> Result<(), E>,
    ) -> Change {
        let active = if self.lost {
            Active::NotFollowed
        } else {
            self.last.active(main_focused)
        };
        if self.followed.replace(active) == Some(active) {
            return Change::Nothing;
        }
        self.grab.follow(active, grab, release)
    }
}

#[cfg(test)]
mod tests;
