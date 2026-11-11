//! The best reading on candidates written by hand: the room, the readings as good, and the
//! bound of the readings looked at.

use super::*;
use crate::game_data::modifiers::ModifierTiers;
use crate::game_data::modifiers::split_lines::AddedTiers;

/// A candidate that splits the line `line` into a modifier of `affix` over the modifiers
/// `modifiers` and a pure one of `added`, without tiers.
fn split(modifiers: std::ops::Range<usize>, affix: Affix, added: Affix, line: usize) -> Candidate {
    let whole = candidate(modifiers, &[affix], false);
    let tiers = whole.affixes.first().map(|(_, tiers)| tiers.clone());
    Candidate {
        added: tiers.map(|tiers| AddedTiers {
            line,
            affix: added,
            tiers,
        }),
        ..whole
    }
}

#[test]
fn the_lines_two_readings_as_good_read_otherwise_are_ambiguous() {
    use Affix::{Prefix, Suffix};
    let room = |prefixes, suffixes| Room { prefixes, suffixes };
    let search = |candidates: &[Candidate], room| {
        let mut search = Search::new(candidates, 2, room, 0);
        search.walk(0, &mut Vec::new());
        let best = search.best.map(|(_, reading)| reading);
        (best, search.ambiguous)
    };
    // The line 1 split into two prefixes, or a prefix and a suffix; the line 0 alone.
    let candidates = [
        candidate(0..1, &[Prefix], false),
        split(1..2, Prefix, Prefix, 1),
        split(1..2, Prefix, Suffix, 1),
    ];

    // Room for both ways.
    assert_eq!(search(&candidates, room(3, 3)), (Some(vec![0, 1]), vec![1]));
    // Room for the prefix and the suffix only: one way fits.
    assert_eq!(search(&candidates, room(2, 1)), (Some(vec![0, 2]), vec![]));
    // A line split in another way by worse readings only isn't ambiguous: its split
    // alone reads a line fewer than the one with the line 0.
    let candidates = [
        split(0..2, Prefix, Prefix, 1),
        split(1..2, Prefix, Suffix, 1),
    ];
    assert_eq!(search(&candidates, room(3, 3)), (Some(vec![0]), vec![]));
    // A split over the lines 1 and 2, a hybrid, against one of the line 2 only, alone: every
    // line of both.
    let candidates = [
        candidate(0..1, &[Prefix], false),
        split(1..3, Prefix, Prefix, 2),
        split(2..3, Prefix, Prefix, 2),
    ];
    assert_eq!(differently_read(&candidates, &[0, 1], &[0, 2]), [1, 2, 2]);
    assert_eq!(differently_read(&candidates, &[0, 1], &[0, 1]), [0; 0]);
    assert_eq!(differently_read(&candidates, &[0], &[0]), [0; 0]);
    // Room for one split only, of either line: both ambiguous, the line 0 without tier
    // looked at first.
    let candidates = [
        split(0..1, Prefix, Prefix, 0),
        split(1..2, Prefix, Prefix, 1),
    ];
    assert_eq!(search(&candidates, room(2, 0)), (Some(vec![1]), vec![1, 0]));
    // No split: the line 0 a hybrid with the line 1, or the line 1 alone, the line 0 then
    // without tier. Two lines read against one: the hybrid, nothing ambiguous.
    let candidates = [
        candidate(0..2, &[Prefix], false),
        candidate(1..2, &[Prefix], false),
    ];
    assert_eq!(search(&candidates, room(3, 3)), (Some(vec![0]), vec![]));
}

/// A candidate of the modifiers `modifiers` that can be `affixes`, without tiers.
fn candidate(modifiers: std::ops::Range<usize>, affixes: &[Affix], fractured: bool) -> Candidate {
    let tiers = ModifierTiers {
        tiers: Vec::new(),
        found: 0,
        neighbour: None,
        counts: Vec::new(),
        tier_unwritten: false,
        currency: None,
        lord: None,
    };
    Candidate {
        modifiers,
        fractured,
        affixes: affixes
            .iter()
            .map(|&affix| (affix, tiers.clone()))
            .collect(),
        added: None,
    }
}

#[test]
fn the_room_tells_the_affix_of_the_modifiers_that_can_be_both() {
    use Affix::{Prefix, Suffix};
    let candidates = [
        candidate(0..1, &[Prefix], false),
        candidate(1..2, &[Prefix, Suffix], false),
        candidate(2..3, &[Prefix, Suffix], false),
        candidate(3..4, &[Suffix], false),
    ];
    let all = [0, 1, 2, 3];
    let room = |prefixes, suffixes| Room { prefixes, suffixes };

    // One more prefix and one more suffix: one of the two is a prefix, either.
    assert_eq!(room(2, 2).bounds(&all, &candidates), Some((1, 1)));
    assert_eq!(room(2, 2).both(&all, &candidates), (true, true));
    // No room for another suffix: both are prefixes.
    assert_eq!(room(3, 1).bounds(&all, &candidates), Some((2, 2)));
    assert_eq!(room(3, 1).both(&all, &candidates), (true, false));
    // No room for another prefix: both are suffixes.
    assert_eq!(room(1, 3).both(&all, &candidates), (false, true));
    // Too few: none fits, every affix kept.
    assert_eq!(room(1, 2).bounds(&all, &candidates), None);
    assert_eq!(room(1, 2).both(&all, &candidates), (true, true));
    assert_eq!(room(0, 5).bounds(&all, &candidates), None);
    assert_eq!(room(0, 0).bounds(&[], &candidates), Some((0, 0)));
}

#[test]
fn of_two_readings_as_good_the_first_is_kept_and_their_lines_are_ambiguous() {
    use Affix::Prefix;
    let candidates = [
        candidate(0..1, &[Prefix], false),
        candidate(0..2, &[Prefix], false),
        candidate(1..2, &[Prefix], false),
        candidate(1..3, &[Prefix], false),
        candidate(2..3, &[Prefix], false),
    ];
    let room = Room {
        prefixes: 2,
        suffixes: 0,
    };
    let mut search = Search::new(&candidates, 3, room, 0);

    search.walk(0, &mut Vec::new());

    // The three lines alone make three prefixes; then `A, BC` and `AB, C`, as good: the
    // first one looked at kept, every line of the candidates they don't share ambiguous.
    assert_eq!(search.readings, 3);
    assert_eq!(search.best.map(|(_, reading)| reading), Some(vec![0, 3]));
    assert_eq!(search.ambiguous, [0, 1, 2]);
    assert!(!search.room_in_doubt);
}

#[test]
fn two_readings_as_good_that_read_other_lines_leave_them_and_their_room_in_doubt() {
    use Affix::{Prefix, Suffix};
    // The line 1 a hybrid with the line 0, a prefix, or with the line 2, a suffix: no line
    // has a tier alone, so that each reading leaves one without tier. The line 3 can be a
    // prefix or a suffix.
    let candidates = [
        candidate(0..2, &[Prefix], false),
        candidate(1..3, &[Suffix], false),
        candidate(3..4, &[Prefix, Suffix], false),
    ];
    let room = Room {
        prefixes: 1,
        suffixes: 1,
    };
    let mut search = Search::new(&candidates, 4, room, 0);

    search.walk(0, &mut Vec::new());

    // The order of the lines made one the first: the lines of both hybrids are ambiguous,
    // and the room of the line 3 too, a suffix after one and a prefix after the other.
    assert_eq!(search.best.map(|(_, reading)| reading), Some(vec![1, 2]));
    assert_eq!(search.ambiguous, [1, 2, 0]);
    assert!(search.room_in_doubt);

    // A hybrid of the three lines, looked at after them, reads one line more: the doubts
    // of the readings it beats go with them.
    let candidates = [
        candidate(0..2, &[Prefix], false),
        candidate(0..3, &[Prefix], false),
        candidate(1..3, &[Suffix], false),
        candidate(3..4, &[Prefix, Suffix], false),
    ];
    let mut search = Search::new(&candidates, 4, room, 0);

    search.walk(0, &mut Vec::new());

    assert_eq!(search.best.map(|(_, reading)| reading), Some(vec![1, 3]));
    assert_eq!(search.ambiguous, [] as [usize; 0]);
    assert!(!search.room_in_doubt);
}

#[test]
fn the_readings_looked_at_are_bounded() {
    use Affix::Prefix;
    let count = 30;
    let candidates: Vec<_> = (0..count)
        .flat_map(|start| {
            let pair = (start + 2 <= count).then(|| candidate(start..start + 2, &[Prefix], false));
            [Some(candidate(start..start + 1, &[Prefix], false)), pair]
        })
        .flatten()
        .collect();
    let room = Room {
        prefixes: 0,
        suffixes: 0,
    };
    let mut search = Search::new(&candidates, count, room, 0);

    search.walk(0, &mut Vec::new());

    assert_eq!(search.readings, MAX_READINGS);
    assert!(search.cut);
    // The lines alone, looked at first.
    assert_eq!(search.best.map(|(key, _)| key.modifiers), Some(count));
}

#[test]
fn as_many_readings_as_the_bound_all_looked_at_arent_cut() {
    use Affix::Prefix;
    // Twelve pairs of lines, each read alone or as one modifier: 2 to the 12th readings.
    let candidates: Vec<_> = (0..24)
        .flat_map(|start| {
            let pair = (start % 2 == 0).then(|| candidate(start..start + 2, &[Prefix], false));
            [Some(candidate(start..start + 1, &[Prefix], false)), pair]
        })
        .flatten()
        .collect();
    let room = Room {
        prefixes: 0,
        suffixes: 0,
    };
    let mut search = Search::new(&candidates, 24, room, 0);

    search.walk(0, &mut Vec::new());

    assert_eq!(search.readings, MAX_READINGS);
    assert!(!search.cut);
}
