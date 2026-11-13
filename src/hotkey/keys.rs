//! The keys that send the copy to the game, the keys of each system as `K`: which are pressed
//! and released around C, as the player holds the modifiers, and their sending, which never
//! leaves a key held after a failure.

/// A key pressed or released.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Key<K> {
    Down(K),
    Up(K),
}

/// The keys the player holds when the copy leaves, the keys of each system as `K`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Held<K> {
    pub(super) control: bool,
    /// The Shift keys held, which would change the copy: Ctrl+Shift+C isn't the copy. Alt is left
    /// held, never released: Ctrl+Alt+C is the advanced copy of the game, the same text as
    /// Ctrl+C (Exiled Exchange 2 sends it on purpose), and an Alt released then pressed again
    /// could stay held for the system when the player releases it meanwhile.
    pub(super) others: Vec<K>,
    /// The modifiers of the hotkey the player released before its key, while X11 gave every key
    /// to the application (until D is released): the game never saw them released, and would
    /// take them for held. Released once more for it, first; none on Windows, which leaves them
    /// to the game.
    pub(super) missed: Vec<K>,
}

impl<K> Held<K> {
    /// Ctrl alone held, or none.
    #[cfg(test)]
    pub(super) fn control(control: bool) -> Self {
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
pub(super) fn copy_keys<K: Copy>(control: K, c: K, held: &Held<K>) -> Vec<Key<K>> {
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
pub(super) fn copy_keys_after_focus<K: Copy>(
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
pub(super) fn modifiers_for_log(released: &[&str], missed: &[&str]) -> String {
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
pub(super) fn send_all<K, E>(
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

#[cfg(test)]
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
