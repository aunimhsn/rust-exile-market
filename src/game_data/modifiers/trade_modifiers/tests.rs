//! The modifiers written again, on tiers written by hand: what the writing does whatever the
//! tables of the game. The lines still find their stat in the embedded stats of the game.

use super::*;
use crate::game_data::modifiers::fixtures::{modifier, part, range, tier, trade_lines};
use crate::item::{ModifierOrigin, StatLine};

/// `modifiers`, their lines written in English, written again with the tiers `found`.
fn written_again(modifiers: &[Modifier], found: &[FoundTiers]) -> Vec<Modifier> {
    with_found_tiers(Language::English, "Test Base", modifiers, found)
}

/// The modifiers `modifiers` of an item read as one modifier of `affix`, in `tiers`.
fn found(modifiers: std::ops::Range<usize>, affix: Affix, tiers: ModifierTiers) -> FoundTiers {
    FoundTiers {
        modifiers,
        affixes: vec![(affix, tiers)],
        added: None,
    }
}

/// The tiers of a modifier, numbered from 1, the best first, each with the range of each of
/// its values (`ranges`); the copied modifier in the tier of index `found`, each of its lines
/// of `counts` values.
fn tiers(counts: &[usize], found: usize, ranges: &[&[(f64, f64)]]) -> ModifierTiers {
    ModifierTiers {
        tiers: (1..)
            .zip(ranges)
            .map(|(number, ranges)| tier(number, 1, ranges))
            .collect(),
        found,
        neighbour: None,
        counts: counts.to_vec(),
        tier_unwritten: false,
        currency: None,
        lord: None,
    }
}

/// The tiers of a modifier of one line of one value, each over its range of `ranges`, the best
/// first; the copied modifier in the tier of index `found`.
fn line_tiers(found: usize, ranges: &[(f64, f64)]) -> ModifierTiers {
    let ranges: Vec<&[(f64, f64)]> = ranges.iter().map(std::slice::from_ref).collect();
    tiers(&[1], found, &ranges)
}

/// A modifier of `affix` and `tier` as the advanced copy writes it, each of `lines` with the
/// range of a value after it (`+85(80-89) to maximum Life`): what a modifier written again is
/// compared with.
fn advanced(affix: Option<Affix>, tier: Option<u32>, lines: &[&str]) -> Modifier {
    Modifier {
        tier,
        lines: lines.iter().map(|line| StatLine::parse(line)).collect(),
        ..modifier(affix, &[])
    }
}

#[test]
fn a_line_becomes_a_modifier_of_its_affix_with_its_tier_and_the_range_of_that_tier() {
    let life = trade_lines(&["+85 to maximum Life"]);
    // The second of three tiers: neither the best nor the worst.
    let second = line_tiers(1, &[(90.0, 99.0), (80.0, 89.0), (70.0, 79.0)]);

    for affix in [Affix::Prefix, Affix::Suffix] {
        assert_eq!(
            written_again(&life, &[found(0..1, affix, second.clone())]),
            [advanced(
                Some(affix),
                Some(2),
                &["+85(80-89) to maximum Life"]
            )]
        );
    }
}

#[test]
fn a_modifier_the_game_writes_without_tier_gets_its_ranges_and_no_tier() {
    let life = trade_lines(&["+85 to maximum Life"]);
    let unwritten = ModifierTiers {
        tier_unwritten: true,
        ..line_tiers(1, &[(90.0, 99.0), (80.0, 89.0)])
    };

    assert_eq!(
        written_again(&life, &[found(0..1, Affix::Suffix, unwritten)]),
        [advanced(
            Some(Affix::Suffix),
            None,
            &["+85(80-89) to maximum Life"]
        )]
    );
}

#[test]
fn the_lines_of_a_hybrid_become_one_modifier_each_line_with_its_range() {
    // Three lines, as the longest modifier of the game, each marked fractured.
    let mut lines = trade_lines(&[
        "+50 to Armour",
        "+41 to Evasion Rating",
        "115% increased Armour and Evasion",
    ]);
    for line in &mut lines {
        line.origin = Some(ModifierOrigin::Fractured);
    }
    let hybrid = tiers(
        &[1, 1, 1],
        0,
        &[&[(42.0, 52.0), (40.0, 50.0), (110.0, 120.0)]],
    );

    assert_eq!(
        written_again(&lines, &[found(0..3, Affix::Prefix, hybrid)]),
        [Modifier {
            origin: Some(ModifierOrigin::Fractured),
            ..advanced(
                Some(Affix::Prefix),
                Some(1),
                &[
                    "+50(42-52) to Armour",
                    "+41(40-50) to Evasion Rating",
                    "115(110-120)% increased Armour and Evasion",
                ]
            )
        }]
    );
}

#[test]
fn a_modifier_written_again_takes_the_place_of_its_lines_and_the_others_stay_as_they_were() {
    let mut lines = trade_lines(&[
        "+30 to Spirit",
        "+34 to maximum Mana",
        "+50 to Armour",
        "+41 to Evasion Rating",
        "+85 to maximum Life",
        "18% increased Rarity of Items found",
    ]);
    // An implicit, which the search doesn't read; the rarity is a line it found no tier for.
    lines[0].kind = ModifierKind::Implicit;
    let hybrid = tiers(&[1, 1], 0, &[&[(42.0, 52.0), (40.0, 50.0)]]);
    // In another order than the lines.
    let found = [
        found(4..5, Affix::Prefix, line_tiers(0, &[(80.0, 89.0)])),
        found(2..4, Affix::Prefix, hybrid),
        found(1..2, Affix::Prefix, line_tiers(0, &[(30.0, 39.0)])),
    ];

    assert_eq!(
        written_again(&lines, &found),
        [
            lines[0].clone(),
            advanced(
                Some(Affix::Prefix),
                Some(1),
                &["+34(30-39) to maximum Mana"]
            ),
            advanced(
                Some(Affix::Prefix),
                Some(1),
                &["+50(42-52) to Armour", "+41(40-50) to Evasion Rating"]
            ),
            advanced(
                Some(Affix::Prefix),
                Some(1),
                &["+85(80-89) to maximum Life"]
            ),
            lines[5].clone(),
        ]
    );
}

#[test]
fn each_value_of_a_line_takes_its_range_in_the_order_of_the_values() {
    let lines = trade_lines(&["Adds 3 to 9 Physical Damage", "+195 to Accuracy Rating"]);
    // Two values for the first line, one for the second.
    let hybrid = tiers(&[2, 1], 0, &[&[(1.0, 4.0), (5.0, 12.0), (175.0, 200.0)]]);

    assert_eq!(
        written_again(&lines, &[found(0..2, Affix::Prefix, hybrid)]),
        [advanced(
            Some(Affix::Prefix),
            Some(1),
            &[
                "Adds 3(1-4) to 9(5-12) Physical Damage",
                "+195(175-200) to Accuracy Rating",
            ]
        )]
    );
}

#[test]
fn a_range_is_written_without_its_sign() {
    // A stat that rolls negative values, as its tiers hold them.
    let rarity = trade_lines(&["15% reduced Rarity of Items found"]);
    let reduced = line_tiers(0, &[(-19.0, -10.0)]);

    assert_eq!(
        written_again(&rarity, &[found(0..1, Affix::Suffix, reduced)]),
        [advanced(
            Some(Affix::Suffix),
            Some(1),
            &["15(10-19)% reduced Rarity of Items found"]
        )]
    );
}

#[test]
fn a_range_across_zero_isnt_written_on_its_value() {
    // The range of the rarity needs its sign; the life keeps its own.
    let lines = trade_lines(&["5% increased Rarity of Items found", "+85 to maximum Life"]);
    let hybrid = tiers(&[1, 1], 0, &[&[(-10.0, 10.0), (80.0, 89.0)]]);

    assert_eq!(
        written_again(&lines, &[found(0..2, Affix::Prefix, hybrid)]),
        [advanced(
            Some(Affix::Prefix),
            Some(1),
            &[
                "5% increased Rarity of Items found",
                "+85(80-89) to maximum Life",
            ]
        )]
    );
}

#[test]
fn a_number_the_string_of_a_stat_writes_takes_no_range() {
    // The 40 is written by the string of the stat: the range after the 10–15 is the life's.
    let lines = trade_lines(&[
        "12% chance for Skills to retain 40% of Glory on use",
        "+85 to maximum Life",
    ]);
    let hybrid = tiers(&[1, 1], 0, &[&[(10.0, 15.0), (80.0, 89.0)]]);

    assert_eq!(
        written_again(&lines, &[found(0..2, Affix::Suffix, hybrid)]),
        [advanced(
            Some(Affix::Suffix),
            Some(1),
            &[
                "12(10-15)% chance for Skills to retain 40% of Glory on use",
                "+85(80-89) to maximum Life",
            ]
        )]
    );
}

#[test]
fn a_line_without_number_takes_no_range_and_leaves_the_next_line_its_own() {
    // The string of the bolt gives its value, one, which the tiers count and range.
    let lines = trade_lines(&["Loads an additional bolt", "+85 to maximum Life"]);
    let hybrid = tiers(&[1, 1], 0, &[&[(1.0, 1.0), (80.0, 89.0)]]);

    assert_eq!(
        written_again(&lines, &[found(0..2, Affix::Prefix, hybrid)]),
        [advanced(
            Some(Affix::Prefix),
            Some(1),
            &["Loads an additional bolt", "+85(80-89) to maximum Life"]
        )]
    );
}

#[test]
fn a_line_whose_values_arent_as_many_as_their_ranges_keeps_them_without_range() {
    let lines = trade_lines(&["Adds 3 to 9 Physical Damage", "+85 to maximum Life"]);
    let written =
        |tiers: ModifierTiers| written_again(&lines, &[found(0..2, Affix::Prefix, tiers)]);
    let damage_without_range = [advanced(
        Some(Affix::Prefix),
        Some(1),
        &["Adds 3 to 9 Physical Damage", "+85(80-89) to maximum Life"],
    )];

    // One value counted for the damage, which has two: the range of the first isn't written.
    let one_counted = tiers(&[1, 1], 0, &[&[(1.0, 4.0), (80.0, 89.0)]]);
    assert_eq!(written(one_counted), damage_without_range);
    // Three counted: nor are the ranges of the first two.
    let three_counted = tiers(
        &[3, 1],
        0,
        &[&[(1.0, 4.0), (5.0, 12.0), (20.0, 30.0), (80.0, 89.0)]],
    );
    assert_eq!(written(three_counted), damage_without_range);
    // A tier with one range for the damage, and none left for the life.
    let short = tiers(&[2, 1], 0, &[&[(1.0, 4.0)]]);
    assert_eq!(
        written(short),
        [advanced(
            Some(Affix::Prefix),
            Some(1),
            &["Adds 3 to 9 Physical Damage", "+85 to maximum Life"],
        )]
    );
}

#[test]
fn modifiers_whose_tiers_cant_be_written_stay_as_they_were() {
    let lines = trade_lines(&["+50 to Armour", "+41 to Evasion Rating"]);
    let hybrid = || tiers(&[1, 1], 0, &[&[(42.0, 52.0), (40.0, 50.0)]]);
    let unwritten = |lines: &[Modifier], found: FoundTiers| {
        let found = [found];
        assert_eq!(written_again(lines, &found), lines, "{found:?}");
    };

    // Written again as they are, but for what each case changes.
    let whole = [found(0..2, Affix::Prefix, hybrid())];
    assert_ne!(written_again(&lines, &whole), lines);
    // The tier found isn't among the tiers.
    let beyond = ModifierTiers {
        found: 1,
        ..hybrid()
    };
    unwritten(&lines, found(0..2, Affix::Prefix, beyond));
    // No affix.
    let no_affix = FoundTiers {
        affixes: Vec::new(),
        ..found(0..2, Affix::Prefix, hybrid())
    };
    unwritten(&lines, no_affix);
    // The values of the first line only are counted.
    let one_line = tiers(&[1], 0, &[&[(42.0, 52.0)]]);
    unwritten(&lines, found(0..2, Affix::Prefix, one_line));
    // More modifiers than the item has.
    unwritten(&lines, found(0..3, Affix::Prefix, hybrid()));
    // A line no stat of the game data writes.
    let unknown = trade_lines(&["+50 to Armour", "+41 to the Rating of no Stat"]);
    unwritten(&unknown, found(0..2, Affix::Prefix, hybrid()));
}

#[test]
fn the_modifiers_after_some_that_cant_be_written_are_written_again() {
    let lines = trade_lines(&[
        "+50 to Armour",
        "+41 to the Rating of no Stat",
        "+85 to maximum Life",
    ]);
    let hybrid = tiers(&[1, 1], 0, &[&[(42.0, 52.0), (40.0, 50.0)]]);
    let found = [
        found(0..2, Affix::Prefix, hybrid),
        found(2..3, Affix::Prefix, line_tiers(0, &[(80.0, 89.0)])),
    ];

    assert_eq!(
        written_again(&lines, &found),
        [
            lines[0].clone(),
            lines[1].clone(),
            advanced(
                Some(Affix::Prefix),
                Some(1),
                &["+85(80-89) to maximum Life"]
            ),
        ]
    );
}

#[test]
fn the_lines_find_their_stat_in_the_language_of_the_item() {
    let mana = trade_lines(&["+34 de Mana maximal"]);
    let found = [found(0..1, Affix::Prefix, line_tiers(0, &[(30.0, 39.0)]))];
    let written = |language| with_found_tiers(language, "Test Base", &mana, &found);

    assert_eq!(
        written(Language::French),
        [advanced(
            Some(Affix::Prefix),
            Some(1),
            &["+34(30-39) de Mana maximal"]
        )]
    );
    // No English stat writes the line: as it was.
    assert_eq!(written(Language::English), mana);
}

#[test]
fn a_part_whose_two_tiers_give_its_line_other_ranges_isnt_read() {
    let rarity = trade_lines(&["18% increased Rarity of Items found"]);
    let tier = |number, ranges: Vec<Range>| Tier {
        number,
        level: 1,
        ranges,
    };
    let tiers = |second: Vec<Range>| ModifierTiers {
        tiers: vec![tier(1, vec![range(16.0, 19.0)]), tier(2, second)],
        found: 1,
        neighbour: Some(0),
        counts: vec![1],
        tier_unwritten: false,
        currency: None,
        lord: None,
    };
    let rarity = Run {
        language: Language::English,
        modifiers: &rarity,
    };
    let read = |tiers: &ModifierTiers| rarity.part_tier(Affix::Prefix, tiers, 0);

    assert_eq!(
        read(&tiers(vec![range(12.0, 15.0)])),
        Some(part(Affix::Prefix, &[1, 2], &[(12.0, 19.0)]))
    );
    // A tier without the range of the value: the ranges of the line can't be told apart.
    assert_eq!(read(&tiers(Vec::new())), None);

    // Each tier without the range of another value (across zero, which needs its sign): as
    // many ranges, but not of the same values.
    let damage = trade_lines(&["Adds 3 to 9 Physical Damage"]);
    let tiers = |first: f64, second: f64| ModifierTiers {
        tiers: vec![
            tier(1, vec![range(first, 2.0), range(5.0, 10.0)]),
            tier(2, vec![range(1.0, 4.0), range(second, 12.0)]),
        ],
        found: 1,
        neighbour: Some(0),
        counts: vec![2],
        tier_unwritten: false,
        currency: None,
        lord: None,
    };
    let damage = Run {
        language: Language::English,
        modifiers: &damage,
    };
    let read_damage = |tiers: &ModifierTiers| damage.part_tier(Affix::Prefix, tiers, 0);
    assert_eq!(
        read_damage(&tiers(1.0, 6.0)),
        Some(part(Affix::Prefix, &[1, 2], &[(1.0, 4.0), (5.0, 12.0)]))
    );
    assert_eq!(read_damage(&tiers(-1.0, -2.0)), None);
}
