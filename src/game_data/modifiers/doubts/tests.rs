//! How the search of the other readings ends, what each end leaves in doubt and what the
//! search takes from the best reading, on readings and tables written by hand.

use super::*;
use crate::game_data::modifiers::best_reading::{Room, best_reading};
use crate::game_data::modifiers::candidates::candidates;
use crate::game_data::modifiers::fixtures::{
    ENERGY_SHIELD, LIFE, MANA, RARITY, copied, equipment, marked, table,
};
use crate::game_data::modifiers::tables::Table;

/// What the search of the other readings finds in at most `max_steps` steps, after the best
/// reading of the modifiers `copied` of a helmet of any level among `tables`, the item having
/// `room` for them.
fn doubts_in(
    tables: &[Table],
    copied: &[Option<Copied<'_>>],
    room: Room,
    max_steps: usize,
) -> Doubts {
    let tables = ItemTables {
        all: tables,
        base: &equipment(&["helmet"]),
        item_level: None,
    };
    let candidates = candidates(&tables, copied);
    let fractured = copied
        .iter()
        .flatten()
        .filter(|copied| copied.origin == Some(ModifierOrigin::Fractured))
        .count();
    let best =
        best_reading(&candidates, copied.len(), room, fractured).expect("every reading looked at");
    doubts_within(&tables, copied, &best, max_steps)
}

/// How the search ended, the modifiers it leaves in doubt and whether it leaves another room,
/// to compare whole.
fn ended(doubts: &Doubts) -> (Outcome, Vec<usize>, bool) {
    (doubts.outcome, doubts.in_doubt.clone(), doubts.other_room)
}

#[test]
fn a_search_cut_at_its_bound_leaves_every_line_read_in_doubt() {
    let tables = [
        table("Life", "prefix", &[LIFE], &[(1, &[(21, 30)])]),
        table(
            "Mana",
            "prefix",
            &[MANA],
            &[(1, &[(10, 25)]), (30, &[(20, 40)])],
        ),
    ];
    // The modifier 1 isn't a line to read: its header tells its affix.
    let lines = [
        Some(copied(&[(LIFE, 25.0)])),
        None,
        Some(copied(&[(MANA, 22.0)])),
    ];
    let room = Room {
        prefixes: 3,
        suffixes: 3,
    };

    // Every reading looked at, the best one among them: the mana read in the other tier of
    // its modifier is no doubt.
    let whole = doubts_in(&tables, &lines, room, MAX_STEPS);
    assert_eq!(ended(&whole), (Outcome::BestMet, vec![], false));
    // The same within the steps it took.
    let within = doubts_in(&tables, &lines, room, whole.steps);
    assert_eq!(ended(&within), (Outcome::BestMet, vec![], false));
    // One step short, some readings not looked at: no line read keeps its tier. The other
    // room comes with them by convention: no line is left with a tier to read it.
    let cut = doubts_in(&tables, &lines, room, whole.steps - 1);
    assert_eq!(ended(&cut), (Outcome::Cut, vec![0, 2], true));
    assert_eq!(cut.steps, whole.steps - 1);
}

#[test]
fn a_search_that_doesnt_meet_the_best_reading_leaves_every_line_read_in_doubt() {
    let tables = [
        table("Life", "prefix", &[LIFE], &[(1, &[(21, 30)])]),
        table("Rarity", "suffix", &[RARITY], &[(1, &[(10, 20)])]),
    ];
    // Two lines of one stat written apart, one fractured: the best reading takes each for the
    // pure life, two modifiers of one type, which the search never makes.
    let lines = [
        Some(marked(ModifierOrigin::Fractured, LIFE, 25.0)),
        Some(copied(&[(LIFE, 26.0)])),
        Some(copied(&[(RARITY, 15.0)])),
    ];
    let room = Room {
        prefixes: 3,
        suffixes: 3,
    };

    // The rarity too, which no reading of the search gets to.
    let doubts = doubts_in(&tables, &lines, room, MAX_STEPS);
    assert_eq!(ended(&doubts), (Outcome::BestNotMet, vec![0, 1, 2], true));
}

#[test]
fn a_search_reads_as_many_fractured_modifiers_as_the_best_reading_or_one() {
    use ModifierOrigin::Fractured;
    let room = Room {
        prefixes: 3,
        suffixes: 3,
    };

    // Two fractured lines no hybrid holds: the best reading takes each for a pure modifier,
    // two fractured ones, where the game fractures one.
    let tables = [
        table(
            "EnergyShield",
            "prefix",
            &[ENERGY_SHIELD],
            &[(1, &[(21, 30)])],
        ),
        table("Life", "prefix", &[LIFE], &[(1, &[(21, 30)])]),
    ];
    let lines = [
        Some(marked(Fractured, ENERGY_SHIELD, 25.0)),
        Some(marked(Fractured, LIFE, 25.0)),
    ];
    // The search reads two as well, and meets the best reading.
    let doubts = doubts_in(&tables, &lines, room, MAX_STEPS);
    assert_eq!(ended(&doubts), (Outcome::BestMet, vec![], false));

    // The fractured lines of a hybrid apart, which no candidate reads, and no pure modifier:
    // the best reading reads the rarity alone, no fractured modifier.
    let tables = [
        table(
            "Hybrid",
            "prefix",
            &[ENERGY_SHIELD, LIFE],
            &[(1, &[(20, 29), (10, 19)])],
        ),
        table("Rarity", "suffix", &[RARITY], &[(1, &[(10, 20)])]),
    ];
    let lines = [
        Some(marked(Fractured, ENERGY_SHIELD, 25.0)),
        Some(copied(&[(RARITY, 15.0)])),
        Some(marked(Fractured, LIFE, 15.0)),
    ];
    // The search still reads one, the hybrid: its lines are told, with the prefix it takes.
    let doubts = doubts_in(&tables, &lines, room, MAX_STEPS);
    assert_eq!(ended(&doubts), (Outcome::BestMet, vec![0, 2], true));
}

#[test]
fn a_search_that_meets_no_reading_in_the_room_leaves_no_line_in_doubt() {
    let tables = [
        table(
            "EnergyShield",
            "prefix",
            &[ENERGY_SHIELD],
            &[(1, &[(21, 30)])],
        ),
        table("Life", "prefix", &[LIFE], &[(1, &[(21, 30)])]),
    ];
    let lines = [
        Some(copied(&[(ENERGY_SHIELD, 25.0)])),
        Some(copied(&[(LIFE, 25.0)])),
    ];
    // Two pure prefixes, the only reading, with room for one: the tables are out of step with
    // the game, and the lines keep the tiers of the best reading.
    let room = Room {
        prefixes: 1,
        suffixes: 3,
    };

    let doubts = doubts_in(&tables, &lines, room, MAX_STEPS);
    assert_eq!(ended(&doubts), (Outcome::NoneFits, vec![], false));
}

#[test]
fn a_best_reading_out_of_the_room_leaves_in_doubt_what_a_reading_in_the_room_reads_otherwise() {
    // No pure life, a hybrid that is a suffix, and a mana whose two tiers both hold 22.
    let tables = [
        table(
            "EnergyShield",
            "prefix",
            &[ENERGY_SHIELD],
            &[(1, &[(21, 30)])],
        ),
        table(
            "Hybrid",
            "suffix",
            &[ENERGY_SHIELD, LIFE],
            &[(1, &[(20, 29), (10, 19)])],
        ),
        table("Rarity", "suffix", &[RARITY], &[(1, &[(10, 20)])]),
        table(
            "Mana",
            "suffix",
            &[MANA],
            &[(1, &[(10, 25)]), (30, &[(20, 40)])],
        ),
    ];
    // The rarity between the lines of the hybrid, which no candidate reads then: the best
    // reading takes the energy shield for the pure prefix and leaves the life without tier.
    let lines = [
        Some(copied(&[(ENERGY_SHIELD, 25.0)])),
        Some(copied(&[(RARITY, 15.0)])),
        Some(copied(&[(LIFE, 15.0)])),
        Some(copied(&[(MANA, 22.0)])),
    ];
    let room = Room {
        prefixes: 0,
        suffixes: 3,
    };

    // No room for a prefix: the readings in the room are the hybrid, the rarity and the mana,
    // in either of its tiers. The energy shield is in doubt, and the life, which has no tier
    // to lose; the mana too, one of them reading it in its other tier, which is no doubt for
    // a best reading in the room; the rarity, read the same, isn't. A prefix fewer and a
    // suffix more: another room.
    let doubts = doubts_in(&tables, &lines, room, MAX_STEPS);
    assert_eq!(
        ended(&doubts),
        (Outcome::OnlyAnotherFits, vec![0, 2, 3], true)
    );
}

#[test]
fn the_search_ends_as_the_readings_it_met_and_the_room_tell() {
    // Stopped at its bound, whatever the room: it can't tell what the others would have been.
    for best_fits_room in [true, false] {
        assert_eq!(Outcome::of(None, best_fits_room), Outcome::Cut);
    }
    // A best reading that fits the room is among the readings of the search, or isn't one.
    assert_eq!(Outcome::of(Some(Met::Best), true), Outcome::BestMet);
    assert_eq!(Outcome::of(Some(Met::Others), true), Outcome::BestNotMet);
    assert_eq!(Outcome::of(Some(Met::Nothing), true), Outcome::BestNotMet);
    // One that doesn't fit it isn't a reading of the item: whether another one is.
    assert_eq!(
        Outcome::of(Some(Met::Others), false),
        Outcome::OnlyAnotherFits
    );
    assert_eq!(
        Outcome::of(Some(Met::Best), false),
        Outcome::OnlyAnotherFits
    );
    assert_eq!(Outcome::of(Some(Met::Nothing), false), Outcome::NoneFits);
}
