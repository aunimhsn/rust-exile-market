//! The hotkey grabbed only while the game is the active window, so that Ctrl+D stays to the other
//! windows (`specs.md`, Item detection): grabbed when the game becomes the active window, released
//! when another one does; registered and unregistered on Windows alike.
//!
//! Without the system, so that the rules are tested: the grab and the release come as closures.

use std::fmt::Display;

/// The exact title of the window of the game: the title Exiled Exchange 2 looks for
/// (`attachByTitle`), the standalone client and the one of Steam alike on Windows, and the
/// standalone client run by Wine on Linux, whose class is unknown. Never a part of the title: the
/// page of a browser about the game is titled `Path of Exile 2 - …`.
pub const GAME_TITLE: &str = "Path of Exile 2";

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
    None,
}

#[cfg(any(target_os = "windows", test))]
impl Foreground {
    /// The active window it is for the hotkey, the window over the game being `overlay_shown` or
    /// not: a window of this application in front while the window over the game shows is taken
    /// for that window, and leaves the key as it is, so that Ctrl+D still copies while one of its
    /// fields is edited (giving the keyboard back to the game first); the main window in front
    /// releases the key, as another window, once the window over the game is closed (a click in
    /// the main window closes it; brought to the front by Alt+Tab, it keeps the key meanwhile).
    pub fn active(self, overlay_shown: bool) -> Active {
        match self {
            Self::Game => Active::Game,
            Self::Ours if overlay_shown => Active::Unknown,
            Self::Ours | Self::Other => Active::Other,
            Self::None => Active::Unknown,
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
#[derive(Debug, Default)]
pub struct Grab {
    /// Whether the key is grabbed.
    held: bool,
    /// The last failure to grab it, written to the log once until it changes: the key taken by
    /// another application fails at each activation of the game.
    failure: Option<String>,
    /// Whether the log already said the key is listened to, so that it doesn't say it at each
    /// activation of the game.
    told: bool,
}

impl Grab {
    /// Follows the active window: grabs the key with `grab` when the game becomes the active
    /// window, and releases it with `release` when another window does. A failed grab is tried
    /// again at the next change, so that the hotkey comes back once the application that took the
    /// key quits. With no active window told, the key stays as it is, so that Ctrl+D still gives
    /// the keyboard back to the game from the window over the game; a window outside X11 never
    /// hears a key grabbed under X11 anyway. Once the active window can't be followed, the key is
    /// grabbed for every window. Called at each change only: a failed grab is tried at each call.
    pub fn follow<E: Display>(
        &mut self,
        active: Active,
        grab: impl FnOnce() -> Result<(), E>,
        release: impl FnOnce() -> Result<(), E>,
    ) -> Change {
        match (active, self.held) {
            (Active::Game, false) => self.grab(grab, false),
            (Active::NotFollowed, false) => self.grab(grab, true),
            (Active::NotFollowed, true) => {
                log::info!("Price-check hotkey Ctrl+D listened to in every window");
                Change::Nothing
            }
            (Active::Other, true) => match release() {
                Ok(()) => {
                    self.held = false;
                    Change::Released
                }
                // Still held: tried again at the next change.
                Err(error) => {
                    log::warn!("The price-check hotkey Ctrl+D can't be released: {error}");
                    Change::Failed
                }
            },
            (Active::Game, true) | (Active::Other, false) | (Active::Unknown, _) => Change::Nothing,
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
                if every_window {
                    log::info!("Price-check hotkey Ctrl+D listened to in every window");
                } else if self.failure.take().is_some() || first {
                    log::info!(
                        "Price-check hotkey Ctrl+D listened to while the game is the active window"
                    );
                }
                Change::Grabbed
            }
            Err(error) => {
                let error = error.to_string();
                if every_window || self.failure.as_ref() != Some(&error) {
                    log::warn!("The price-check hotkey Ctrl+D can't be listened to: {error}");
                    self.failure = Some(error);
                }
                Change::Failed
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What `grab` does when the active window is `active`, the grab and the release giving
    /// `result`, and how many times each was called.
    fn follow(grab: &mut Grab, active: Active, result: Result<(), &str>) -> (Change, u32, u32) {
        let (mut grabs, mut releases) = (0, 0);
        let change = grab.follow(
            active,
            || {
                grabs += 1;
                result
            },
            || {
                releases += 1;
                result
            },
        );
        (change, grabs, releases)
    }

    #[test]
    fn grabs_the_key_when_the_game_becomes_the_active_window() {
        let mut grab = Grab::default();

        assert_eq!(
            follow(&mut grab, Active::Game, Ok(())),
            (Change::Grabbed, 1, 0)
        );
    }

    #[test]
    fn leaves_the_key_to_another_window_active_at_the_start() {
        let mut grab = Grab::default();

        assert_eq!(
            follow(&mut grab, Active::Other, Ok(())),
            (Change::Nothing, 0, 0)
        );
    }

    #[test]
    fn grabs_the_key_once_while_the_game_stays_the_active_window() {
        let mut grab = Grab::default();
        follow(&mut grab, Active::Game, Ok(()));

        // A change of its title, or of another property followed.
        assert_eq!(
            follow(&mut grab, Active::Game, Ok(())),
            (Change::Nothing, 0, 0)
        );
    }

    #[test]
    fn releases_the_key_when_another_window_becomes_the_active_window() {
        let mut grab = Grab::default();
        follow(&mut grab, Active::Game, Ok(()));

        assert_eq!(
            follow(&mut grab, Active::Other, Ok(())),
            (Change::Released, 0, 1)
        );
        assert_eq!(
            follow(&mut grab, Active::Other, Ok(())),
            (Change::Nothing, 0, 0)
        );
        assert_eq!(
            follow(&mut grab, Active::Game, Ok(())),
            (Change::Grabbed, 1, 0)
        );
    }

    #[test]
    fn tries_a_key_taken_by_another_application_again_at_the_next_activation_of_the_game() {
        let mut grab = Grab::default();
        assert_eq!(
            follow(&mut grab, Active::Game, Err("taken")),
            (Change::Failed, 1, 0)
        );

        // Never grabbed: nothing to release.
        assert_eq!(
            follow(&mut grab, Active::Other, Ok(())),
            (Change::Nothing, 0, 0)
        );
        assert_eq!(
            follow(&mut grab, Active::Game, Err("taken")),
            (Change::Failed, 1, 0)
        );
        assert_eq!(
            follow(&mut grab, Active::Game, Ok(())),
            (Change::Grabbed, 1, 0)
        );
    }

    #[test]
    fn keeps_the_key_as_it_is_while_the_desktop_tells_no_active_window() {
        let mut grab = Grab::default();
        assert_eq!(
            follow(&mut grab, Active::Unknown, Ok(())),
            (Change::Nothing, 0, 0)
        );
        follow(&mut grab, Active::Game, Ok(()));

        // The window over the game keeping the keyboard, on a desktop other than KWin.
        assert_eq!(
            follow(&mut grab, Active::Unknown, Ok(())),
            (Change::Nothing, 0, 0)
        );
        assert_eq!(
            follow(&mut grab, Active::Other, Ok(())),
            (Change::Released, 0, 1)
        );
    }

    #[test]
    fn grabs_the_key_for_every_window_once_the_active_window_cant_be_followed() {
        let mut grab = Grab::default();
        assert_eq!(
            follow(&mut grab, Active::NotFollowed, Ok(())),
            (Change::Grabbed, 1, 0)
        );

        let mut grab = Grab::default();
        follow(&mut grab, Active::Game, Ok(()));
        // Already grabbed while the game was active: kept.
        assert_eq!(
            follow(&mut grab, Active::NotFollowed, Ok(())),
            (Change::Nothing, 0, 0)
        );

        let mut grab = Grab::default();
        follow(&mut grab, Active::Game, Err("taken"));
        follow(&mut grab, Active::Other, Ok(()));
        // Tried again after a failure.
        assert_eq!(
            follow(&mut grab, Active::NotFollowed, Err("taken")),
            (Change::Failed, 1, 0)
        );
    }

    #[test]
    fn the_window_over_the_game_in_front_keeps_the_key_as_it_is() {
        // In front after a click in it, or while one of its fields is edited.
        assert_eq!(Foreground::Ours.active(true), Active::Unknown);

        let mut grab = Grab::default();
        follow(&mut grab, Foreground::Game.active(false), Ok(()));
        assert_eq!(
            follow(&mut grab, Foreground::Ours.active(true), Ok(())),
            (Change::Nothing, 0, 0)
        );
    }

    #[test]
    fn the_main_window_in_front_releases_the_key_as_another_window() {
        assert_eq!(Foreground::Ours.active(false), Active::Other);
        assert_eq!(Foreground::Other.active(true), Active::Other);

        let mut grab = Grab::default();
        follow(&mut grab, Foreground::Game.active(false), Ok(()));
        // The window over the game in front, then closed by a click in the main window.
        follow(&mut grab, Foreground::Ours.active(true), Ok(()));
        assert_eq!(
            follow(&mut grab, Foreground::Ours.active(false), Ok(())),
            (Change::Released, 0, 1)
        );
    }

    #[test]
    fn no_foreground_window_for_a_moment_keeps_the_key_as_it_is() {
        assert_eq!(Foreground::None.active(false), Active::Unknown);
        assert_eq!(Foreground::Game.active(true), Active::Game);
    }

    #[test]
    fn tries_a_failed_release_again_at_the_next_change() {
        let mut grab = Grab::default();
        follow(&mut grab, Active::Game, Ok(()));

        assert_eq!(
            follow(&mut grab, Active::Other, Err("lost")),
            (Change::Failed, 0, 1)
        );
        assert_eq!(
            follow(&mut grab, Active::Other, Ok(())),
            (Change::Released, 0, 1)
        );
    }
}
