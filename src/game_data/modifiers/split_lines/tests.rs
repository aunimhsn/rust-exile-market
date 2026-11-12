//! The ways a line splits, and the tiers of the modifier of a split line, on tables written by
//! hand: what they do whatever the patch of the game.

use super::*;
use crate::game_data::modifiers::copied::CopiedLine;
use crate::game_data::modifiers::fixtures::{equipment, part, range};
use crate::game_data::modifiers::tables::read;

#[test]
fn the_hybrid_of_a_split_is_the_first_table_whose_tiers_of_its_part_the_item_level_allows() {
    // Two hybrids of the same lines, both holding the Armour, the first with its own range of
    // Armour: it rolls its T1 from level 80, or gives the Stun Threshold another range.
    let table = |kind: &str, level: u32, armour: u32, stun: u32| {
        format!(
            r##"{{"type": "{kind}", "affix": "prefix", "lines": [{{"template": "#% increased Armour"}}, {{"template": "# to Stun Threshold"}}], "spawn": [["shield", 1]], "tiers": [{{"level": {level}, "ranges": [[[20, {armour}]], [[{stun}, 39]]]}}]}}"##
        )
    };
    let copied = || Copied {
        lines: vec![
            CopiedLine {
                templates: vec!["#% increased Armour"],
                values: vec![25.0],
                written: None,
            },
            CopiedLine {
                templates: vec!["# to Stun Threshold"],
                values: vec![100.0],
                written: None,
            },
        ],
        ranges: None,
        tier: CopiedTier::Unknown,
        origin: None,
    };
    let whole = part(Affix::Prefix, &[1], &[(30.0, 39.0)]);
    // The range of the Armour of the tier found, which tells the table.
    let armour = |tables: &[String], item_level| {
        let tables = ItemTables {
            all: &read(&tables.join("\n")),
            base: &equipment(&["shield"]),
            item_level,
        };
        hybrid_tiers(&tables, copied(), (1, &whole))
            .and_then(|tiers| tiers.tier()?.ranges.first().copied())
    };
    let high = [table("First", 80, 35, 30), table("Second", 1, 30, 30)];
    let other = [table("First", 1, 35, 31), table("Second", 1, 30, 30)];

    // The tier of the first above the item level, as `splits` reads it: the second.
    assert_eq!(armour(&high, Some(50)), Some(range(20.0, 30.0)));
    // Without the item level, any tier.
    assert_eq!(armour(&high, None), Some(range(20.0, 35.0)));
    // The first gives the summed line other ranges: the second.
    assert_eq!(armour(&other, Some(82)), Some(range(20.0, 30.0)));
    // The first holds the Armour give or take the rounding only, the second within its
    // range: the second, as `splits` keeps the ways that fit best.
    let rounded = [
        table("First", 1, 35, 30).replace("[[20, 35]]", "[[20, 24.96]]"),
        table("Second", 1, 30, 30),
    ];
    assert_eq!(armour(&rounded, Some(82)), Some(range(20.0, 30.0)));
}

#[test]
fn two_hybrids_of_the_same_lines_that_split_a_line_are_two_ways() {
    // Two hybrids of the same lines and Armour, other Stun Thresholds, and a pure Stun
    // Threshold: `100` adds up the pure `50–70` with `30–39` or with `40–49`.
    let hybrid = |kind: &str, stun: u32| {
        format!(
            r##"{{"type": "{kind}", "affix": "prefix", "lines": [{{"template": "#% increased Armour"}}, {{"template": "# to Stun Threshold"}}], "spawn": [["shield", 1]], "tiers": [{{"level": 1, "ranges": [[[20, 30]], [[{stun}, {}]]]}}]}}"##,
            stun + 9
        )
    };
    let pure = r##"{"type": "Stun", "affix": "suffix", "lines": [{"template": "# to Stun Threshold"}], "spawn": [["shield", 1]], "tiers": [{"level": 1, "ranges": [[[50, 70]]]}]}"##;
    let tables = read(&[hybrid("First", 30), hybrid("Second", 40), pure.to_owned()].join("\n"));
    let copied = Copied {
        lines: vec![
            CopiedLine {
                templates: vec!["#% increased Armour"],
                values: vec![25.0],
                written: None,
            },
            CopiedLine {
                templates: vec!["# to Stun Threshold"],
                values: vec![100.0],
                written: None,
            },
        ],
        ranges: None,
        tier: CopiedTier::Unknown,
        origin: None,
    };

    let tables = ItemTables {
        all: &tables,
        base: &equipment(&["shield"]),
        item_level: Some(82),
    };

    let ways = splits(&tables, &copied, (1, 1));

    // Two ways as good, of two tables: the line is left without tier by the search, as two
    // candidates split it (`the_lines_split_in_two_ways_as_good_keep_their_room`).
    let pairs: Vec<_> = ways.iter().map(|way| way.tables).collect();
    assert_eq!(pairs, [(0, 2), (1, 2)]);
}

#[test]
fn a_split_holds_its_value_give_or_take_the_rounding() {
    let tier = |ranges: &[(f64, f64)]| Tier {
        number: 1,
        level: 1,
        ranges: ranges.iter().map(|&(min, max)| range(min, max)).collect(),
    };
    // A hybrid of two values, the second one summed with a pure 1–2.
    let whole = tier(&[(10.0, 20.0), (4.41, 5.0)]);
    let added = tier(&[(1.0, 2.0)]);
    let fit = |summed| split_fit(&[15.0, summed], &whole, &added, &(1..2));

    assert_eq!(fit(7.0), Some(Fit::Within));
    assert_eq!(fit(5.41), Some(Fit::Within));
    assert_eq!(fit(7.03), Some(Fit::Rounded));
    assert_eq!(fit(5.3), None);
    assert_eq!(fit(7.1), None);
    // The value of the other line within its own range only.
    assert_eq!(split_fit(&[9.0, 6.0], &whole, &added, &(1..2)), None);
}

/// A way to split a line into the modifiers of the tables `tables`, in the tiers of indexes
/// `found`, without ranges.
fn way(tables: (usize, usize), (found, added_found): (usize, usize)) -> Split {
    let tiers = |found| ModifierTiers {
        tiers: Vec::new(),
        found,
        neighbour: None,
        counts: Vec::new(),
        tier_unwritten: false,
        currency: None,
        lord: None,
    };
    Split {
        tables,
        affix: Affix::Prefix,
        tiers: tiers(found),
        added: AddedTiers {
            line: 0,
            affix: Affix::Prefix,
            tiers: tiers(added_found),
        },
        fit: Fit::Within,
    }
}

/// The tables of each way, and the index of the tier of each of its modifiers with its
/// neighbour.
type WayTiers = (
    (usize, usize),
    (usize, Option<usize>),
    (usize, Option<usize>),
);

fn way_tiers(ways: &[Split]) -> Vec<WayTiers> {
    ways.iter()
        .map(|way| {
            let tiers = |tiers: &ModifierTiers| (tiers.found, tiers.neighbour);
            (way.tables, tiers(&way.tiers), tiers(&way.added.tiers))
        })
        .collect()
}

#[test]
fn two_ways_that_differ_by_a_neighbouring_tier_of_one_modifier_are_one() {
    let joined = |ways: &[Split]| way_tiers(&neighbours_joined(ways));

    // The added modifier in the tier of index 1 or 0, or the summed one: the worse found,
    // the better its neighbour (the summed one on an item:
    // `the_summed_part_of_a_split_line_in_two_neighbouring_tiers_keeps_both`).
    assert_eq!(
        joined(&[way((0, 1), (0, 0)), way((0, 1), (0, 1))]),
        [((0, 1), (0, None), (1, Some(0)))]
    );
    assert_eq!(
        joined(&[way((0, 1), (2, 1)), way((0, 1), (1, 1))]),
        [((0, 1), (2, Some(1)), (1, None))]
    );
    // Both modifiers in other tiers, tiers that aren't neighbours, three ways (also a
    // modifier in three tiers), or two pairs of modifiers: apart.
    let apart = [
        vec![way((0, 1), (0, 1)), way((0, 1), (1, 0))],
        vec![way((0, 1), (0, 0)), way((0, 1), (0, 2))],
        vec![
            way((0, 1), (0, 0)),
            way((0, 1), (0, 1)),
            way((0, 1), (1, 0)),
        ],
        vec![
            way((0, 1), (0, 0)),
            way((0, 1), (0, 1)),
            way((0, 1), (0, 2)),
        ],
        vec![way((0, 1), (0, 0)), way((0, 2), (0, 1))],
    ];
    for ways in apart {
        assert_eq!(joined(&ways), way_tiers(&ways));
    }
}

#[test]
fn the_tiers_of_the_hybrid_of_a_split_are_those_of_its_part() {
    // T2 at level 1, T1 at level 50, the Armour of both touching at 20.
    let tables = read(concat!(
        r##"{"type": "ArmourAndStun", "affix": "prefix", "lines": [{"template": "#% increased Armour"}, {"template": "# to Stun Threshold"}], "##,
        r#""spawn": [["shield", 1]], "tiers": [{"level": 1, "ranges": [[[10, 20]], [[20, 29]]]}, {"level": 50, "ranges": [[[20, 29]], [[30, 39]]]}]}"#
    ));
    let copied = |armour: f64| Copied {
        lines: vec![
            CopiedLine {
                templates: vec!["#% increased Armour"],
                values: vec![armour],
                written: None,
            },
            CopiedLine {
                templates: vec!["# to Stun Threshold"],
                values: vec![100.0],
                written: None,
            },
        ],
        ranges: None,
        tier: CopiedTier::Unknown,
        origin: None,
    };
    let tables = ItemTables {
        all: &tables,
        base: &equipment(&["shield"]),
        item_level: None,
    };
    // The tier and its neighbour found, by their numbers.
    let tiers = |armour, whole: PartTier| {
        hybrid_tiers(&tables, copied(armour), (1, &whole)).map(|tiers| {
            let number = |index: usize| tiers.tiers.get(index).map(|tier| tier.number);
            (number(tiers.found), tiers.neighbour.and_then(number))
        })
    };

    assert_eq!(
        tiers(25.0, part(Affix::Prefix, &[1], &[(30.0, 39.0)])),
        Some((Some(1), None))
    );
    // In two neighbouring tiers, the Armour in both: the worse found, over the range of both.
    assert_eq!(
        tiers(20.0, part(Affix::Prefix, &[1, 2], &[(20.0, 39.0)])),
        Some((Some(2), Some(1)))
    );
    // Ranges the tiers of that table don't give the summed line: another table.
    assert_eq!(
        tiers(25.0, part(Affix::Prefix, &[1], &[(31.0, 39.0)])),
        None
    );
    assert_eq!(
        tiers(25.0, part(Affix::Prefix, &[3], &[(30.0, 39.0)])),
        None
    );
}
