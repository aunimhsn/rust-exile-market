//! The steps of the walk on a ground written by hand: what a modifier may be added to, and
//! when a line is settled.

use super::*;
use crate::game_data::modifiers::best_reading::Room;
use crate::game_data::modifiers::fixtures::range;
use crate::game_data::modifiers::other_readings::Line;
use crate::game_data::modifiers::other_readings::fixtures::{ground, line, placement};
use crate::item::ModifierOrigin;

#[test]
fn a_modifier_is_added_once_per_type_in_the_room_and_never_on_a_line_without_value() {
    use Affix::{Prefix, Suffix};
    let fractured = Some(ModifierOrigin::Fractured);
    let lines = [
        line(0, "+# to maximum Life", 0..1, None),
        line(1, "+# to maximum Mana", 1..2, None),
        line(2, "Loads an additional bolt", 2..2, None),
        line(3, "+# to Armour", 2..3, fractured),
        line(4, "#% increased Armour", 3..4, fractured),
    ];
    let values = [50.0, 50.0, 20.0, 30.0];
    let ranges = |value: usize| [(value, range(1.0, 100.0))];
    let placements = [
        placement("Life", Prefix, &[0], &ranges(0), false),
        placement("Life", Prefix, &[0], &ranges(0), false),
        placement("Mana", Prefix, &[1], &ranges(1), false),
        placement("Bolt", Suffix, &[2], &[], false),
        placement("Bolts", Suffix, &[2], &[], false),
        placement("Armour", Prefix, &[3], &ranges(2), true),
        placement("Increased", Suffix, &[4], &ranges(3), true),
    ];
    let ground = Ground {
        room: Room {
            prefixes: 2,
            suffixes: 3,
        },
        ..ground(&lines, &values, &placements)
    };
    let mut state = State::new(&ground);

    state.push(&ground, 0);
    // A second modifier of the type of life.
    assert!(!state.fits(&ground, 1));
    assert!(state.fits(&ground, 2));
    // A second fractured modifier.
    state.push(&ground, 5);
    assert!(!state.fits(&ground, 6));
    state.pop(&ground);
    // The prefixes full.
    state.push(&ground, 2);
    assert!(!state.fits(&ground, 5));
    // A line without value holds one modifier only.
    state.push(&ground, 3);
    assert!(!state.fits(&ground, 4));
    state.pop(&ground);
    assert!(state.fits(&ground, 4));
    // Taken away, a modifier leaves its sums and counts as they were.
    state.pop(&ground);
    state.pop(&ground);
    assert_eq!(state.counts, [0; 5]);
    assert_eq!(state.sums, [range(0.0, 0.0); 4]);
    assert_eq!((state.prefixes, state.suffixes, state.fractured), (0, 0, 0));
}

#[test]
fn a_line_is_settled_once_its_modifiers_hold_it_or_left_as_the_best_reading_leaves_it() {
    use Affix::Prefix;
    let lines = [
        line(0, "+# to maximum Life", 0..1, None),
        // A line the best reading leaves without tier.
        Line {
            read_by_best: false,
            ..line(1, "+# to maximum Mana", 1..2, None)
        },
    ];
    let values = [50.0, 50.0];
    let placements = [
        placement("Life", Prefix, &[0], &[(0, range(1.0, 100.0))], false),
        placement("Mana", Prefix, &[1], &[(1, range(60.0, 100.0))], false),
        placement("More", Prefix, &[1], &[(1, range(-20.0, -10.0))], false),
    ];
    let ground = ground(&lines, &values, &placements);
    let mut state = State::new(&ground);

    // Without a modifier: only the line the best reading doesn't read either.
    assert!(!state.line_settled(&ground, 0));
    assert!(state.line_settled(&ground, 1));
    state.push(&ground, 0);
    assert!(state.line_settled(&ground, 0));
    // With modifiers, their ranges hold its value, whichever line it is.
    state.push(&ground, 1);
    assert!(!state.line_settled(&ground, 1));
    state.push(&ground, 2);
    assert!(state.line_settled(&ground, 1));
    // A line out of the item.
    assert!(!state.line_settled(&ground, 2));
}
