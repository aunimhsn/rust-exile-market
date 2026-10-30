//! The pieces of the search of the other readings: the comparison with the best reading, the
//! parts of the item, the pruning, the ways a modifier sits on the lines, what a modifier may be
//! added to, and the bound of the search.

use super::*;
use crate::game_data::bases::{self, Domain};
use crate::game_data::lines::modifier_stats;
use crate::game_data::modifiers::{AddedTiers, ModifierTiers, TableAffix, TableLine, Tier};
use crate::item::{ModifierKind, StatLine};
use crate::language::Language;

fn range(min: f64, max: f64) -> Range {
    Range { min, max }
}

fn shown(affix: Affix, tier: u32, modifiers: &[usize]) -> Shown {
    Shown {
        affix,
        tier,
        modifiers: modifiers.to_vec(),
        ranges: vec![range(1.0, 2.0)],
    }
}

#[test]
fn a_line_is_read_the_same_with_one_modifier_for_each_of_the_best_one() {
    use Affix::{Prefix, Suffix};
    let prefix = shown(Prefix, 1, &[0]);
    let suffix = shown(Suffix, 1, &[0]);
    let hybrid = shown(Prefix, 4, &[0, 1]);

    // A modifier that can be a prefix or a suffix: either is the same.
    let either = [vec![prefix.clone(), suffix.clone()]];
    assert!(same(&either, &[&prefix]));
    assert!(same(&either, &[&suffix]));
    assert!(!same(&either, &[&shown(Prefix, 2, &[0])]));
    // A split line: its two modifiers in either order, each once.
    let split = [vec![hybrid.clone()], vec![prefix.clone()]];
    assert!(same(&split, &[&prefix, &hybrid]));
    assert!(same(&split, &[&hybrid, &prefix]));
    assert!(!same(&split, &[&hybrid, &hybrid]));
    assert!(!same(&split, &[&hybrid]));
    // A line the best reading doesn't read.
    assert!(same(&[], &[]));
    assert!(!same(&[], &[&prefix]));
}

#[test]
fn a_part_of_a_split_in_two_neighbouring_tiers_is_read_the_same_in_either() {
    use Affix::Prefix;
    let tier = |number| Tier {
        number,
        level: 1,
        ranges: vec![range(1.0, 2.0)],
    };
    let tiers = |found, neighbour| ModifierTiers {
        tiers: vec![tier(1), tier(2)],
        found,
        neighbour,
        counts: vec![1],
        tier_unwritten: false,
        lord: None,
    };
    // A line split into two pure prefixes, the first a T2 or a T1, the other a T1: no base
    // rolls two pure prefixes of one stat today, but the comparison mustn't depend on the
    // order the search reads them in.
    let split = Candidate {
        modifiers: 0..1,
        fractured: false,
        affixes: vec![(Prefix, tiers(1, Some(0)))],
        added: Some(AddedTiers {
            line: 0,
            affix: Prefix,
            tiers: tiers(0, None),
        }),
    };

    let best = best_shown(&[&split], 1);

    let (first, second) = (shown(Prefix, 1, &[0]), shown(Prefix, 2, &[0]));
    let line = best.first().expect("the line");
    assert!(same(line, &[&second, &first]));
    assert!(same(line, &[&first, &second]));
    assert!(same(line, &[&first, &first]));
    assert!(!same(line, &[&second, &second]));
    assert!(!same(line, &[&first]));
    assert!(!same(line, &[&first, &first, &first]));
}

#[test]
fn the_parts_of_the_item_are_the_lines_a_modifier_joins() {
    let mut parts = Parts::new(5);
    parts.join([0, 1]);
    parts.join([3, 1]);
    parts.join([2]);
    parts.join([4, 4]);
    parts.join([]);

    let roots: Vec<usize> = (0..5).map(|line| parts.root(line)).collect();
    assert_eq!(roots[0], roots[1]);
    assert_eq!(roots[1], roots[3]);
    assert_ne!(roots[0], roots[2]);
    assert_ne!(roots[0], roots[4]);
    assert_ne!(roots[2], roots[4]);
    // A line out of the item is its own part.
    assert_eq!(parts.root(9), 9);
}

#[test]
fn lines_are_in_doubt_where_the_best_reading_splits_or_makes_as_many_modifiers() {
    // The best reading splits a line of the part: in doubt, whatever the other reading makes.
    assert!(in_doubt(true, false, 4, 2));
    // No split: only another reading adding up modifiers, with no more of them.
    assert!(in_doubt(false, true, 3, 3));
    assert!(!in_doubt(false, true, 4, 3));
    // A hybrid in place of two pure modifiers: the lines alone first.
    assert!(!in_doubt(false, false, 1, 2));
}

/// A line of the modifier `modifier` of the item, of `template`, its values at `values` among
/// those of every line.
fn line(
    modifier: usize,
    template: &'static str,
    values: std::ops::Range<usize>,
    origin: Option<ModifierOrigin>,
) -> Line<'static> {
    Line {
        modifier,
        origin,
        copied: CopiedLine {
            template,
            values: Vec::new(),
            written: None,
        },
        values,
    }
}

/// A placement of `kind` and `affix` on `lines`, adding `ranges` to the values.
fn placement(
    kind: &'static str,
    affix: Affix,
    lines: &[usize],
    ranges: &[(usize, Range)],
    fractured: bool,
) -> Placement<'static> {
    Placement {
        kind,
        fractured,
        shown: Shown {
            affix,
            tier: 1,
            modifiers: lines.to_vec(),
            ranges: ranges.iter().map(|&(_, range)| range).collect(),
        },
        lines: lines.to_vec(),
        ranges: ranges.to_vec(),
    }
}

#[test]
fn a_tier_past_its_value_is_left_out_only_when_every_range_rises_or_falls() {
    let placements = [
        placement(
            "Life",
            Affix::Prefix,
            &[0],
            &[(0, range(10.0, 20.0))],
            false,
        ),
        placement(
            "Mixed",
            Affix::Prefix,
            &[1],
            &[(1, range(-5.0, 3.0))],
            false,
        ),
        placement(
            "Reduced",
            Affix::Prefix,
            &[2],
            &[(2, range(-20.0, -10.0))],
            false,
        ),
    ];
    let (rising, falling) = directions(&placements, 3);
    assert_eq!(rising, [true, false, false]);
    assert_eq!(falling, [false, false, true]);

    let values = [15.0, 0.0, -15.0];
    let is_past = |index, sum| past(&rising, &falling, &values, index, sum);
    // A sum that only rises, already above its value.
    assert!(is_past(0, range(16.0, 30.0)));
    assert!(!is_past(0, range(15.03, 30.0)));
    // Ranges of both signs: another modifier may still bring the sum back.
    assert!(!is_past(1, range(9.0, 12.0)));
    // A sum that only falls, already below its value.
    assert!(is_past(2, range(-30.0, -16.0)));
    assert!(!is_past(2, range(-30.0, -14.0)));
    // A value out of the item.
    assert!(is_past(5, range(0.0, 0.0)));
}

#[test]
fn a_modifier_sits_on_lines_of_its_templates_of_one_origin() {
    let table = |templates: &[&str]| Table {
        kind: "Kind".to_owned(),
        affix: TableAffix::Prefix,
        domain: Domain::Item,
        lines: templates
            .iter()
            .map(|template| TableLine {
                template: (*template).to_owned(),
                divisor: 1.0,
            })
            .collect(),
        spawn: Vec::new(),
        genesis: false,
        lord: None,
        tiers: Vec::new(),
    };
    let fractured = Some(ModifierOrigin::Fractured);
    let lines = [
        line(0, "+# to Armour", 0..1, None),
        line(1, "#% increased Armour", 1..2, None),
        line(2, "+# to Armour", 2..3, fractured),
        line(3, "#% increased Armour", 3..4, fractured),
    ];

    // Each pair of lines of one origin, the lines in their order.
    let hybrid = table(&["#% increased Armour", "+# to Armour"]);
    assert_eq!(mappings(&hybrid, &lines), [vec![0, 1], vec![2, 3]]);
    // A pure modifier on each line of its template.
    let pure = table(&["+# to Armour"]);
    assert_eq!(mappings(&pure, &lines), [vec![0], vec![2]]);
    // A template the item doesn't have.
    assert_eq!(
        mappings(&table(&["+# to maximum Life"]), &lines),
        [] as [Vec<usize>; 0]
    );
}

#[test]
fn only_the_tiers_the_item_level_allows_are_placed() {
    let modifiers = [Modifier {
        kind: ModifierKind::Explicit(None),
        origin: None,
        name: None,
        tier: None,
        tags: Vec::new(),
        unrevealed: false,
        lines: vec![StatLine {
            text: "175% increased Physical Damage".to_owned(),
            rolls: Vec::new(),
        }],
        doubt: None,
    }];
    let copied = Copied::of(
        &modifiers[0],
        &modifier_stats(Language::English, &modifiers[0], None),
    )
    .expect("a line of the game data");
    let lines: Vec<Line<'_>> = copied
        .lines
        .iter()
        .map(|copied| Line {
            modifier: 0,
            origin: None,
            copied: copied.clone(),
            values: 0..1,
        })
        .collect();
    let base = bases::spawn(Language::English, "Warmonger Bow").expect("a base");
    // The pure T1 `170–179` needs the item level 82.
    let pure_t1 = |item_level| {
        placements(&TABLES, base, Some(item_level), &lines, &[175.0])
            .iter()
            .any(|placement| placement.shown.ranges == [range(170.0, 179.0)])
    };

    assert!(pure_t1(82));
    assert!(!pure_t1(81));
}

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
        lines: &lines,
        values: &values,
        placements: &placements,
        starting: vec![Vec::new(); lines.len()],
        room: Room {
            prefixes: 2,
            suffixes: 3,
        },
        fractured: 1,
        rising: vec![true; values.len()],
        falling: vec![false; values.len()],
        best: Vec::new(),
        best_candidates: Vec::new(),
        max_steps: MAX_STEPS,
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
            unrevealed: false,
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
    let candidates = super::super::candidates(&modifiers, &copied, base, Some(82), true);
    let room = Room {
        prefixes: 3,
        suffixes: 3,
    };
    let mut search = super::super::Search::new(&candidates, modifiers.len(), room, 0);
    search.walk(0, &mut Vec::new());
    let (key, reading) = search.best.expect("a reading");
    let (room, fits_room) = searched.unwrap_or((room, key.room));
    let best = Best {
        candidates: &candidates,
        reading: &reading,
        room,
        fits_room,
    };
    doubts_within(base, Some(82), &modifiers, &copied, &best, max_steps)
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

#[test]
fn a_reading_leaves_the_same_room_to_a_prefix_or_suffix_with_as_many_of_each_elsewhere() {
    use Affix::{Prefix, Suffix};
    let lines = [
        line(0, "#% increased Rarity of Items found", 0..1, None),
        line(1, "+# to maximum Life", 1..2, None),
        line(2, "+# to Accuracy Rating", 2..3, None),
    ];
    let values = [16.0, 50.0, 50.0];
    let on =
        |line: usize, affix| placement("Kind", affix, &[line], &[(line, range(1.0, 100.0))], false);
    let ground = Ground {
        lines: &lines,
        values: &values,
        placements: &[],
        starting: Vec::new(),
        room: Room {
            prefixes: 3,
            suffixes: 3,
        },
        fractured: 1,
        rising: Vec::new(),
        falling: Vec::new(),
        best: Vec::new(),
        // The rarity a prefix or a suffix, the life a prefix, the accuracy a suffix.
        best_candidates: vec![
            BestCandidate {
                modifiers: 0..1,
                sets: vec![AffixSet::Both],
            },
            BestCandidate {
                modifiers: 1..2,
                sets: vec![AffixSet::Only(Prefix)],
            },
            BestCandidate {
                modifiers: 2..3,
                sets: vec![AffixSet::Only(Suffix)],
            },
        ],
        max_steps: MAX_STEPS,
    };
    let same =
        |placements: &[Placement<'_>]| ground.same_room(&placements.iter().collect::<Vec<_>>());

    // The rarity either affix, the others as the best reading.
    assert!(same(&[on(0, Prefix), on(1, Prefix), on(2, Suffix)]));
    assert!(same(&[on(0, Suffix), on(1, Prefix), on(2, Suffix)]));
    // The same totals, a prefix of the others moved to a suffix.
    assert!(!same(&[on(0, Prefix), on(1, Suffix), on(2, Suffix)]));
    // A prefix less on the others.
    assert!(!same(&[on(0, Prefix), on(2, Suffix)]));
    // A hybrid of the rarity and the life takes the prefix of the life.
    let hybrid = placement(
        "Hybrid",
        Prefix,
        &[0, 1],
        &[(0, range(1.0, 10.0)), (1, range(1.0, 10.0))],
        false,
    );
    assert!(same(&[hybrid, on(2, Suffix)]));
    // A prefix and a suffix added up on the rarity line.
    assert!(same(&[
        on(0, Prefix),
        on(0, Suffix),
        on(1, Prefix),
        on(2, Suffix)
    ]));
}
