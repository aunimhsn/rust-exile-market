//! The search of the tiers on tables written by hand, or on none: what it does whatever the
//! patch of the game. The tests of `super` read the tables and the items of the game, and are
//! the ones to look at when the game data are generated again.

use super::*;

#[test]
fn the_value_of_a_line_without_number_is_divided_as_its_table_shows_it() {
    let tables = read(concat!(
        r#"{"type": "Rage", "affix": "suffix", "lines": [{"template": "Inherent Rage loss starts 1 second later", "divisor": 1000}], "#,
        r#""spawn": [["weapon", 1]], "tiers": [{"level": 1, "ranges": [[[1, 1]]]}, {"level": 50, "ranges": [[[2, 3]]]}]}"#
    ));
    let copied = |values: Vec<f64>, written| Copied {
        lines: vec![CopiedLine {
            templates: vec!["Inherent Rage loss starts 1 second later"],
            values,
            written,
        }],
        ranges: None,
        tier: CopiedTier::Unknown,
        origin: None,
    };
    let number = |copied| {
        find(
            &tables,
            TableAffix::Suffix,
            &equipment(&["weapon"]),
            &copied,
            Tie::Best,
        )
        .and_then(|tiers| Some(tiers.tier()?.number))
    };

    // `Inherent Rage loss starts 1 second later` gives the 1000 milliseconds of the stat.
    assert_eq!(number(copied(Vec::new(), Some(1000.0))), Some(2));
    assert_eq!(number(copied(vec![2.5], None)), Some(1));
}

#[test]
fn of_two_tiers_as_good_the_one_the_advanced_copy_wrote_is_taken() {
    // A desecrated table of one tier first, then an ordinary one whose T3 has the same range.
    let tables = read(concat!(
        r##"{"type": "Lord", "affix": "prefix", "domain": "desecrated", "lines": [{"template": "# to maximum Life"}], "##,
        r#""spawn": [["helmet", 1]], "tiers": [{"level": 65, "ranges": [[[10, 20]]]}]}"#,
        "\n",
        r##"{"type": "Life", "affix": "prefix", "lines": [{"template": "# to maximum Life"}], "##,
        r#""spawn": [["helmet", 1]], "tiers": [{"level": 1, "ranges": [[[10, 20]]]}, {"level": 30, "ranges": [[[21, 30]]]}, {"level": 60, "ranges": [[[31, 40]]]}]}"#
    ));
    let copied = |tier: Option<u32>| Copied {
        lines: vec![CopiedLine {
            templates: vec!["# to maximum Life"],
            values: vec![15.0],
            written: None,
        }],
        ranges: Some(vec![range(10.0, 20.0)]),
        tier: tier.map_or(CopiedTier::Unknown, CopiedTier::Written),
        // A desecrated line: an ordinary modifier, or a lord's.
        origin: Some(ModifierOrigin::Desecrated),
    };
    let found = |tier| {
        find(
            &tables,
            TableAffix::Prefix,
            &equipment(&["helmet"]),
            &copied(tier),
            Tie::Best,
        )
        .and_then(|tiers| Some((tiers.single(), tiers.tier()?.number)))
    };

    // A T3 isn't hidden by the modifier of one tier before it.
    assert_eq!(found(Some(3)), Some((false, 3)));
    assert_eq!(found(Some(1)), Some((true, 1)));
    // Without the number, the first of the file.
    assert_eq!(found(None), Some((true, 1)));
}

#[test]
fn two_tables_with_the_same_lines_in_another_order_are_two_modifiers() {
    let table = |kind: &str, lines: [&str; 2], level: u32| {
        format!(
            r#"{{"type": "{kind}", "affix": "prefix", "lines": [{{"template": "{}"}}, {{"template": "{}"}}], "spawn": [["weapon", 1]], "tiers": [{{"level": {level}, "ranges": [[[1, 5]], [[10, 20]]]}}]}}"#,
            lines[0], lines[1]
        )
    };
    let tables = read(
        &[
            table("Hybrid", ["A #", "B #"], 1),
            table("Hybrid", ["B #", "A #"], 50),
        ]
        .join("\n"),
    );
    let copied = Copied {
        lines: vec![
            CopiedLine {
                templates: vec!["B #"],
                values: vec![15.0],
                written: None,
            },
            CopiedLine {
                templates: vec!["A #"],
                values: vec![3.0],
                written: None,
            },
        ],
        ranges: None,
        tier: CopiedTier::Unknown,
        origin: None,
    };

    let tiers = find(
        &tables,
        TableAffix::Prefix,
        &equipment(&["weapon"]),
        &copied,
        Tie::Best,
    )
    .expect("its tiers");

    // The first table holds the copied values, its lines put in the copied order; the
    // second, its own lines in the copied order, holds neither 15 in 1–5 nor 3 in 10–20.
    assert_eq!(tiers.tiers.len(), 1);
    assert_eq!(tiers.tiers.first().map(|tier| tier.level), Some(1));
    assert_eq!(
        tiers.tier().map(|tier| tier.ranges.clone()),
        Some(vec![range(10.0, 20.0), range(1.0, 5.0)])
    );
}

#[test]
fn a_value_in_several_tiers_takes_the_worst_without_the_ranges_of_the_copy() {
    let tables = read(concat!(
        r##"{"type": "Life", "affix": "prefix", "lines": [{"template": "# to maximum Life"}], "##,
        r#""spawn": [["helmet", 1]], "tiers": [{"level": 1, "ranges": [[[1, 10]]]}, {"level": 50, "ranges": [[[5, 15]]]}]}"#
    ));
    let copied = Copied {
        lines: vec![CopiedLine {
            templates: vec!["# to maximum Life"],
            values: vec![7.0],
            written: None,
        }],
        ranges: None,
        tier: CopiedTier::Unknown,
        origin: None,
    };
    let number = |tie| {
        find(
            &tables,
            TableAffix::Prefix,
            &equipment(&["helmet"]),
            &copied,
            tie,
        )
        .and_then(|tiers| Some(tiers.tier()?.number))
    };

    assert_eq!(number(Tie::Best), Some(1));
    assert_eq!(number(Tie::Worst { item_level: None }), Some(2));
}

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
        let tables = read(&tables.join("\n"));
        hybrid_tiers(
            &tables,
            &equipment(&["shield"]),
            copied(),
            (1, &whole),
            item_level,
        )
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

    let ways = splits(&tables, &equipment(&["shield"]), &copied, (1, 1), Some(82));

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
fn a_line_split_in_two_ways_as_good_is_ambiguous() {
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
    assert_eq!(differently_split(&candidates, &[0, 1], &[0, 2]), [1, 2, 2]);
    assert_eq!(differently_split(&candidates, &[0, 1], &[0, 1]), [0; 0]);
    assert_eq!(differently_split(&candidates, &[0], &[0]), [0; 0]);
    // Room for one split only, of either line: both ambiguous, the line 0 without tier
    // looked at first.
    let candidates = [
        split(0..1, Prefix, Prefix, 0),
        split(1..2, Prefix, Prefix, 1),
    ];
    assert_eq!(search(&candidates, room(2, 0)), (Some(vec![1]), vec![1, 0]));
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
fn the_first_reading_of_the_most_modifiers_that_fits_is_taken() {
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

    // The three lines alone make three prefixes; then `A, BC` and `AB, C`, the first one
    // looked at taken.
    assert_eq!(search.readings, 3);
    assert_eq!(search.best.map(|(_, reading)| reading), Some(vec![0, 3]));
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

/// What [`read_in`] gives: for each modifier read, how many lines it has and the tier number
/// of each affix it can be; and the text of each line split with the affix and tier number of
/// its pure modifier.
type ReadIn = (Vec<(usize, AffixTiers)>, Added);

/// The tiers of the lines `texts` of an item of `item_level` copied from the in-game trade,
/// found among `tables` on a helmet with at most `limits` prefixes and suffixes. The tables
/// are written by hand, but the lines find their stat in the embedded stats of the game: the
/// templates of these tests ([`LIFE`], [`ENERGY_SHIELD`], [`RARITY`]) must stay among them.
fn read_in(tables: &[Table], item_level: u32, texts: &[&str], limits: (u32, u32)) -> ReadIn {
    let modifiers: Vec<Modifier> = texts.iter().map(|text| modifier(None, &[text])).collect();
    read_modifiers(tables, item_level, &modifiers, limits, false).0
}

/// [`read_in`] for `modifiers`, of a `sanctified` item or not, with what the search took.
fn read_modifiers(
    tables: &[Table],
    item_level: u32,
    modifiers: &[Modifier],
    (prefixes, suffixes): (u32, u32),
    sanctified: bool,
) -> (ReadIn, Effort) {
    let item = TradeItem {
        language: Language::English,
        base_type: "A helmet",
        base: &equipment(&["helmet"]),
        item_level: Some(item_level),
    };
    let limits = AffixLimits::new(prefixes, suffixes);
    let (found, effort) = trade_tiers_in(tables, &item, modifiers, limits, sanctified);
    (
        (summary(modifiers, &found), added(modifiers, &found)),
        effort,
    )
}

/// A table of one or two lines that rolls on the helmets: its type, its affix, the template
/// of each line, and for each tier its level and the range of each line.
fn table(kind: &str, affix: &str, templates: &[&str], tiers: &[(u32, &[(u32, u32)])]) -> String {
    let lines: Vec<String> = templates
        .iter()
        .map(|template| format!(r#"{{"template": "{template}"}}"#))
        .collect();
    let tiers: Vec<String> = tiers
        .iter()
        .map(|(level, ranges)| {
            let ranges: Vec<String> = ranges
                .iter()
                .map(|(min, max)| format!("[[{min}, {max}]]"))
                .collect();
            format!(r#"{{"level": {level}, "ranges": [{}]}}"#, ranges.join(", "))
        })
        .collect();
    format!(
        r#"{{"type": "{kind}", "affix": "{affix}", "lines": [{}], "spawn": [["helmet", 1]], "tiers": [{}]}}"#,
        lines.join(", "),
        tiers.join(", ")
    )
}

const LIFE: &str = "# to maximum Life";
const ENERGY_SHIELD: &str = "#% increased Energy Shield";
const RARITY: &str = "#% increased Rarity of Items found";

#[test]
fn a_line_copied_from_the_in_game_trade_takes_the_worst_tier_its_item_level_allows() {
    use Affix::Prefix;
    let tables = read(&table(
        "Life",
        "prefix",
        &[LIFE],
        &[(1, &[(1, 10)]), (50, &[(5, 15)])],
    ));
    assert_eq!(tables.len(), 1);
    let read = |item_level, text| read_in(&tables, item_level, &[text], (3, 3)).0;

    // In both tiers: the worst.
    assert_eq!(read(60, "+7 to maximum Life"), [(1, vec![(Prefix, 2)])]);
    assert_eq!(read(60, "+12 to maximum Life"), [(1, vec![(Prefix, 1)])]);
    // A tier above the level of the item can't have rolled.
    assert_eq!(read(40, "+12 to maximum Life"), []);
    // No tier holds the value.
    assert_eq!(read(60, "+16 to maximum Life"), []);
}

#[test]
fn the_lines_of_a_hybrid_are_one_modifier_only_when_the_room_needs_it() {
    use Affix::Prefix;
    let tables = read(
        &[
            // Too high to add up with a tier of the hybrid on a line of 25.
            table("Life", "prefix", &[LIFE], &[(1, &[(21, 30)])]),
            table(
                "EnergyShield",
                "prefix",
                &[ENERGY_SHIELD],
                &[(1, &[(21, 30)])],
            ),
            table(
                "Hybrid",
                "prefix",
                &[ENERGY_SHIELD, LIFE],
                &[(1, &[(10, 19), (10, 19)]), (20, &[(20, 29), (20, 29)])],
            ),
        ]
        .join("\n"),
    );
    assert_eq!(tables.len(), 3);
    let lines = ["25% increased Energy Shield", "+25 to maximum Life"];

    // Two pure prefixes while the item has room for them.
    let (read, split) = read_in(&tables, 60, &lines, (2, 0));
    assert_eq!(read, [(1, vec![(Prefix, 1)]), (1, vec![(Prefix, 1)])]);
    assert_eq!(split, []);
    // One prefix only: the hybrid, in its tier 1.
    let (read, split) = read_in(&tables, 60, &lines, (1, 0));
    assert_eq!(read, [(2, vec![(Prefix, 1)])]);
    assert_eq!(split, []);
    // In the other order of its lines too.
    let (read, _) = read_in(&tables, 60, &[lines[1], lines[0]], (1, 0));
    assert_eq!(read, [(2, vec![(Prefix, 1)])]);
    // A life above its pure tier adds up the hybrid and the pure modifier, after the other
    // line of the hybrid.
    let lines = ["25% increased Energy Shield", "+45 to maximum Life"];
    let (read, split) = read_in(&tables, 60, &lines, (2, 0));
    assert_eq!(read, [(2, vec![(Prefix, 1)])]);
    assert_eq!(split, [("+45 to maximum Life".to_owned(), Prefix, 1)]);
}

#[test]
fn a_line_no_tier_holds_alone_adds_up_a_pure_modifier_and_its_hybrid() {
    use Affix::Prefix;
    let tables = read(
        &[
            table(
                "EnergyShield",
                "prefix",
                &[ENERGY_SHIELD],
                &[(1, &[(10, 20)]), (30, &[(21, 30)])],
            ),
            table(
                "Hybrid",
                "prefix",
                &[ENERGY_SHIELD, LIFE],
                &[(1, &[(10, 19), (20, 29)])],
            ),
        ]
        .join("\n"),
    );
    assert_eq!(tables.len(), 2);
    let lines = ["45% increased Energy Shield", "+25 to maximum Life"];

    // 45% is above the pure tier 1: the 26 to 30 of that tier, with the 15 to 19 of the hybrid.
    let (read, split) = read_in(&tables, 60, &lines, (2, 0));
    assert_eq!(read, [(2, vec![(Prefix, 1)])]);
    assert_eq!(
        split,
        [("45% increased Energy Shield".to_owned(), Prefix, 1)]
    );
    // The two modifiers of a split take two places: without them, no tier.
    let (read, split) = read_in(&tables, 60, &lines, (1, 0));
    assert_eq!(read, []);
    assert_eq!(split, []);
    // The pure tier 1 above the level of the item: no way to add up 45%.
    let (read, _) = read_in(&tables, 25, &lines, (2, 0));
    assert_eq!(read, []);
}

#[test]
fn a_line_a_prefix_and_a_suffix_hold_keeps_the_affix_the_room_leaves() {
    use Affix::{Prefix, Suffix};
    let tables = read(
        &[
            table("Life", "prefix", &[LIFE], &[(1, &[(1, 30)])]),
            table("RarityPrefix", "prefix", &[RARITY], &[(1, &[(10, 20)])]),
            table(
                "RaritySuffix",
                "suffix",
                &[RARITY],
                &[(1, &[(6, 12)]), (40, &[(13, 18)])],
            ),
        ]
        .join("\n"),
    );
    assert_eq!(tables.len(), 3);
    let rarity = "15% increased Rarity of Items found";

    // Room for either.
    let (read, _) = read_in(&tables, 60, &[rarity], (1, 1));
    assert_eq!(read, [(1, vec![(Prefix, 1), (Suffix, 1)])]);
    // The life takes the only prefix.
    let (read, _) = read_in(&tables, 60, &["+25 to maximum Life", rarity], (1, 1));
    assert_eq!(read, [(1, vec![(Prefix, 1)]), (1, vec![(Suffix, 1)])]);
    // A value no tier holds alone adds up the prefix and the suffix, the prefix first.
    let (read, split) = read_in(
        &tables,
        60,
        &["38% increased Rarity of Items found"],
        (1, 1),
    );
    assert_eq!(read, [(1, vec![(Prefix, 1)])]);
    assert_eq!(
        split,
        [("38% increased Rarity of Items found".to_owned(), Suffix, 1)]
    );
}

/// Two pure prefixes of one tier, `21–30`, and their hybrid of two tiers, `10–19` and
/// `20–29` on each line.
fn pure_and_hybrid_tables() -> Vec<Table> {
    read(
        &[
            table("Life", "prefix", &[LIFE], &[(1, &[(21, 30)])]),
            table(
                "EnergyShield",
                "prefix",
                &[ENERGY_SHIELD],
                &[(1, &[(21, 30)])],
            ),
            table(
                "Hybrid",
                "prefix",
                &[ENERGY_SHIELD, LIFE],
                &[(1, &[(10, 19), (10, 19)]), (20, &[(20, 29), (20, 29)])],
            ),
        ]
        .join("\n"),
    )
}

#[test]
fn two_fractured_lines_are_one_hybrid_whatever_the_room() {
    use Affix::Prefix;
    let tables = pure_and_hybrid_tables();
    let fractured = |text: &str| Modifier {
        origin: Some(ModifierOrigin::Fractured),
        ..modifier(None, &[text])
    };
    let lines = [
        fractured("25% increased Energy Shield"),
        fractured("+25 to maximum Life"),
    ];

    // An item has one fractured modifier: not two pure ones, even with room for them.
    let ((read, _), _) = read_modifiers(&tables, 60, &lines, (3, 3), false);
    assert_eq!(read, [(2, vec![(Prefix, 1)])]);
    // A fractured line and another aren't the lines of one modifier, even when the item has
    // room for one prefix only: no reading fits, and both stay pure.
    let lines = [
        fractured("25% increased Energy Shield"),
        modifier(None, &["+25 to maximum Life"]),
    ];
    let ((read, _), _) = read_modifiers(&tables, 60, &lines, (1, 3), false);
    assert_eq!(read, [(1, vec![(Prefix, 1)]), (1, vec![(Prefix, 1)])]);
}

#[test]
fn no_line_of_a_sanctified_item_adds_up_two_modifiers() {
    use Affix::Prefix;
    let tables = pure_and_hybrid_tables();
    let lines = [
        modifier(None, &["25% increased Energy Shield"]),
        modifier(None, &["+45 to maximum Life"]),
    ];

    // The life adds up the hybrid and the pure modifier.
    let ((read, split), _) = read_modifiers(&tables, 60, &lines, (2, 0), false);
    assert_eq!(read, [(2, vec![(Prefix, 1)])]);
    assert_eq!(split, [("+45 to maximum Life".to_owned(), Prefix, 1)]);
    // Sanctified, its values can leave their tiers: the life is read by no tier rather than
    // split, the energy shield alone.
    let ((read, split), _) = read_modifiers(&tables, 60, &lines, (2, 0), true);
    assert_eq!(read, [(1, vec![(Prefix, 1)])]);
    assert_eq!(split, []);
}

#[test]
fn the_lines_another_reading_adds_up_in_as_many_modifiers_have_no_tier() {
    use Affix::Prefix;
    // A pure life from 1: with the hybrid of tier 1, it adds up to the 25 of the life as
    // well, two modifiers as the two pure ones.
    let tables = read(
        &[
            table("Life", "prefix", &[LIFE], &[(1, &[(1, 30)])]),
            table(
                "EnergyShield",
                "prefix",
                &[ENERGY_SHIELD],
                &[(1, &[(21, 30)])],
            ),
            table(
                "Hybrid",
                "prefix",
                &[ENERGY_SHIELD, LIFE],
                &[(1, &[(20, 29), (20, 29)])],
            ),
            table("Rarity", "suffix", &[RARITY], &[(1, &[(10, 20)])]),
        ]
        .join("\n"),
    );
    assert_eq!(tables.len(), 4);
    let lines = [
        "25% increased Energy Shield",
        "+25 to maximum Life",
        "15% increased Rarity of Items found",
    ];

    // Both lines in doubt; the rarity, which no other reading touches, keeps its tier.
    let (read, split) = read_in(&tables, 60, &lines, (2, 1));
    assert_eq!(read, [(1, vec![(Affix::Suffix, 1)])]);
    assert_eq!(split, []);
    // With one prefix only, the hybrid alone is the only reading.
    let (read, _) = read_in(&tables, 60, &lines, (1, 1));
    assert_eq!(
        read,
        [(2, vec![(Prefix, 1)]), (1, vec![(Affix::Suffix, 1)])]
    );
}

#[test]
fn the_search_tells_when_the_readings_stop_at_their_bound() {
    let tables = pure_and_hybrid_tables();
    let lines = |count: usize| -> Vec<Modifier> {
        ["25% increased Energy Shield", "+25 to maximum Life"]
            .iter()
            .cycle()
            .take(count)
            .map(|text| modifier(None, &[text]))
            .collect()
    };

    let (_, effort) = read_modifiers(&tables, 60, &lines(2), (3, 3), false);
    assert_eq!(effort.readings, 2);
    assert!(!effort.readings_cut);
    // Each line a hybrid with the one before or after: more readings than the bound.
    let (_, effort) = read_modifiers(&tables, 60, &lines(24), (3, 3), false);
    assert_eq!(effort.readings, MAX_READINGS);
    assert!(effort.readings_cut);
}
