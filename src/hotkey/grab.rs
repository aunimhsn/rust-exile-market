//! The hotkey grabbed only while the game is the active window, so that Ctrl+D stays to the other
//! windows (`specs.md`, Item detection): grabbed when the game becomes the active window, released
//! when another one does.
//!
//! Without the system, so that the rules are tested: the grab and the release come as closures.

use std::fmt::Display;

/// What a change of the active window did, for the tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// Nothing to do: the key is already as the active window wants it.
    Nothing,
    /// The key grabbed, the game being the active window.
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
    /// Follows the active window, `game` when it is the game: grabs the key with `grab` when the
    /// game becomes the active window, and releases it with `release` when another window does.
    /// A failed grab is tried again at the next change, so that the hotkey comes back once the
    /// application that took the key quits.
    pub fn follow<E: Display>(
        &mut self,
        game: bool,
        grab: impl FnOnce() -> Result<(), E>,
        release: impl FnOnce() -> Result<(), E>,
    ) -> Change {
        match (game, self.held) {
            (true, false) => match grab() {
                Ok(()) => {
                    self.held = true;
                    if self.failure.take().is_some() || !std::mem::replace(&mut self.told, true) {
                        log::info!(
                            "Price-check hotkey Ctrl+D listened to while the game is the active \
                             window"
                        );
                    }
                    Change::Grabbed
                }
                Err(error) => {
                    let error = error.to_string();
                    if self.failure.as_ref() != Some(&error) {
                        log::warn!("The price-check hotkey Ctrl+D can't be listened to: {error}");
                        self.failure = Some(error);
                    }
                    Change::Failed
                }
            },
            (false, true) => match release() {
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
            _ => Change::Nothing,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What `grab` does when the game is `game`, the grab and the release giving `result`, and
    /// how many times each was called.
    fn follow(grab: &mut Grab, game: bool, result: Result<(), &str>) -> (Change, u32, u32) {
        let (mut grabs, mut releases) = (0, 0);
        let change = grab.follow(
            game,
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

        assert_eq!(follow(&mut grab, true, Ok(())), (Change::Grabbed, 1, 0));
    }

    #[test]
    fn leaves_the_key_to_another_window_active_at_the_start() {
        let mut grab = Grab::default();

        assert_eq!(follow(&mut grab, false, Ok(())), (Change::Nothing, 0, 0));
    }

    #[test]
    fn grabs_the_key_once_while_the_game_stays_the_active_window() {
        let mut grab = Grab::default();
        follow(&mut grab, true, Ok(()));

        // A change of its title, or of another property followed.
        assert_eq!(follow(&mut grab, true, Ok(())), (Change::Nothing, 0, 0));
    }

    #[test]
    fn releases_the_key_when_another_window_becomes_the_active_window() {
        let mut grab = Grab::default();
        follow(&mut grab, true, Ok(()));

        assert_eq!(follow(&mut grab, false, Ok(())), (Change::Released, 0, 1));
        assert_eq!(follow(&mut grab, false, Ok(())), (Change::Nothing, 0, 0));
        assert_eq!(follow(&mut grab, true, Ok(())), (Change::Grabbed, 1, 0));
    }

    #[test]
    fn tries_a_key_taken_by_another_application_again_at_the_next_activation_of_the_game() {
        let mut grab = Grab::default();
        assert_eq!(
            follow(&mut grab, true, Err("taken")),
            (Change::Failed, 1, 0)
        );

        // Never grabbed: nothing to release.
        assert_eq!(follow(&mut grab, false, Ok(())), (Change::Nothing, 0, 0));
        assert_eq!(
            follow(&mut grab, true, Err("taken")),
            (Change::Failed, 1, 0)
        );
        assert_eq!(follow(&mut grab, true, Ok(())), (Change::Grabbed, 1, 0));
    }

    #[test]
    fn tries_a_failed_release_again_at_the_next_change() {
        let mut grab = Grab::default();
        follow(&mut grab, true, Ok(()));

        assert_eq!(
            follow(&mut grab, false, Err("lost")),
            (Change::Failed, 0, 1)
        );
        assert_eq!(follow(&mut grab, false, Ok(())), (Change::Released, 0, 1));
    }
}
