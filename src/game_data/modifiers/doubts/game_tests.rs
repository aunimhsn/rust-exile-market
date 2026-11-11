//! The search of the other readings on the tables of the game, against its bound and a best
//! reading it doesn't meet: the numbers of a patch, to look at again when the game data are
//! generated again.

use super::*;
use crate::game_data::bases;
use crate::game_data::lines::modifier_stats;
use crate::game_data::modifiers::tables::TABLES;
use crate::item::{ModifierKind, StatLine};
use crate::language::Language;

/// The lines `texts` of an item of `base_type` of item level 82, with 3 prefixes and 3
/// suffixes open: what the search of the other readings finds after the best one, within
/// `max_steps`, in the room and with the fit of `searched` instead of those of the best reading
/// when given.
fn doubts_of(
    base_type: &str,
    texts: &[&str],
    max_steps: usize,
    searched: Option<(Room, bool)>,
) -> Doubts {
    let modifiers: Vec<Modifier> = texts
        .iter()
        .map(|text| Modifier {
            kind: ModifierKind::Explicit(None),
            origin: None,
            name: None,
            tier: None,
            tags: Vec::new(),
            is_unrevealed: false,
            lines: vec![StatLine {
                text: (*text).to_owned(),
                rolls: Vec::new(),
            }],
            doubt: None,
        })
        .collect();
    let copied: Vec<Option<Copied<'_>>> = modifiers
        .iter()
        .map(|modifier| Copied::of(modifier, &modifier_stats(Language::English, modifier, None)))
        .collect();
    let base = bases::spawn(Language::English, base_type).expect("a base");
    let candidates =
        super::super::candidates::candidates(&TABLES, &modifiers, &copied, base, Some(82));
    let room = Room {
        prefixes: 3,
        suffixes: 3,
    };
    let mut search = super::super::best_reading::Search::new(&candidates, modifiers.len(), room, 0);
    search.walk(0, &mut Vec::new());
    let (key, reading) = search.best.expect("a reading");
    let (room, fits_room) = searched.unwrap_or((room, key.room));
    let best = Best {
        candidates: &candidates,
        reading: &reading,
        room,
        fits_room,
    };
    let item = TradeItem {
        language: Language::English,
        base,
        item_level: Some(82),
    };
    doubts_within(&TABLES, &item, &modifiers, &copied, &best, max_steps)
}

/// The lines of a bow with three prefixes, read one way only.
const BOW: [&str; 3] = [
    "Adds 36 to 56 Physical Damage",
    "217% increased Physical Damage",
    "+200 to Accuracy Rating",
];

#[test]
fn a_search_that_cant_look_at_every_reading_leaves_every_line_in_doubt() {
    // Every reading looked at: the best one is the only one, three prefixes.
    let doubts = doubts_of("Warmonger Bow", &BOW, MAX_STEPS, None);
    assert_eq!(doubts.outcome, Outcome::Concluded);
    assert_eq!(doubts.lines, [] as [usize; 0]);
    assert!(!doubts.other_room);

    // Cut short, the best reading not met: no line keeps its tier.
    let doubts = doubts_of("Warmonger Bow", &BOW, 1, None);
    assert_eq!(doubts.outcome, Outcome::Cut);
    assert_eq!(doubts.lines, [0, 1, 2]);
    assert!(doubts.other_room);
}

#[test]
fn a_search_that_doesnt_meet_the_best_reading_leaves_every_line_in_doubt() {
    let two = Room {
        prefixes: 2,
        suffixes: 3,
    };

    // The best reading said to fit two prefixes, which no reading of the search does.
    let doubts = doubts_of("Warmonger Bow", &BOW, MAX_STEPS, Some((two, true)));
    assert_eq!(doubts.outcome, Outcome::BestNotMet);
    assert_eq!(doubts.lines, [0, 1, 2]);

    // A best reading that doesn't fit the room isn't among those of the search, which fit it.
    let doubts = doubts_of("Warmonger Bow", &BOW, MAX_STEPS, Some((two, false)));
    assert_eq!(doubts.outcome, Outcome::Concluded);
    assert_eq!(doubts.lines, [] as [usize; 0]);
}
