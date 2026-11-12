//! The modifiers written again, on tiers written by hand: what the writing does whatever the
//! tables of the game. The lines still find their stat in the embedded stats of the game.

use super::*;
use crate::game_data::modifiers::fixtures::{modifier, part, range, tier, trade_lines};
use crate::item::{Currency, Lord, ModifierOrigin, StatLine};

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

/// The modifiers `modifiers` of an item read as one modifier that can be each of `affixes`,
/// with its tiers as that affix.
fn found_either(
    modifiers: std::ops::Range<usize>,
    affixes: [(Affix, ModifierTiers); 2],
) -> FoundTiers {
    FoundTiers {
        modifiers,
        affixes: affixes.into(),
        added: None,
    }
}

/// `whole`, the line of the modifier `line` of the item adding up its modifier and a pure one
/// of `affix`, in `tiers`.
fn split(whole: FoundTiers, line: usize, affix: Affix, tiers: ModifierTiers) -> FoundTiers {
    FoundTiers {
        added: Some(AddedTiers { line, affix, tiers }),
        ..whole
    }
}

/// `tiers`, the copied modifier in its tier or in the one above it, two neighbouring tiers.
fn or_the_tier_above(tiers: ModifierTiers) -> ModifierTiers {
    ModifierTiers {
        neighbour: tiers.found.checked_sub(1),
        ..tiers
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

/// A modifier without header of `lines`, with their values: more than one line only by hand,
/// the in-game trade writing one modifier a line ([`trade_lines`]).
fn modifier_of_lines(lines: &[&str]) -> Modifier {
    advanced(None, None, lines)
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

/// The tier `tier` of a modifier as `affix`, over `ranges`: its number written, no lord's.
fn affix_tier(affix: Affix, tier: u32, ranges: &[(f64, f64)]) -> AffixTier {
    AffixTier {
        affix,
        tier,
        hides_number: false,
        ranges: ranges.iter().map(|&(min, max)| range(min, max)).collect(),
        lord: None,
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
fn a_line_that_can_be_a_prefix_or_a_suffix_stays_as_it_was_with_the_tier_of_each() {
    let rarity = trade_lines(&["18% increased Rarity of Items found"]);
    // The best of two tiers as a prefix, the second of three as a suffix.
    let prefix = line_tiers(0, &[(16.0, 19.0), (12.0, 15.0)]);
    let suffix = line_tiers(1, &[(19.0, 21.0), (15.0, 18.0), (11.0, 14.0)]);
    let found = found_either(0..1, [(Affix::Prefix, prefix), (Affix::Suffix, suffix)]);

    // Explicit without affix nor tier, its value without range.
    assert_eq!(
        written_again(&rarity, &[found]),
        [Modifier {
            doubt: Some(TierDoubt::EitherAffix {
                prefix: affix_tier(Affix::Prefix, 1, &[(16.0, 19.0)]),
                suffix: affix_tier(Affix::Suffix, 2, &[(15.0, 18.0)]),
            }),
            ..rarity[0].clone()
        }]
    );
}

#[test]
fn the_lines_of_a_hybrid_that_can_be_a_prefix_or_a_suffix_become_one_modifier() {
    let lines = trade_lines(&["+50 to Armour", "+41 to Evasion Rating"]);
    // The best of two tiers as a prefix, the second of two as a suffix.
    let prefix = tiers(
        &[1, 1],
        0,
        &[&[(42.0, 52.0), (40.0, 50.0)], &[(30.0, 41.0), (30.0, 39.0)]],
    );
    let suffix = tiers(
        &[1, 1],
        1,
        &[&[(60.0, 70.0), (55.0, 65.0)], &[(45.0, 55.0), (35.0, 45.0)]],
    );
    let found = found_either(0..2, [(Affix::Prefix, prefix), (Affix::Suffix, suffix)]);

    // The tier of each affix with the range of each line, in their order.
    assert_eq!(
        written_again(&lines, &[found]),
        [Modifier {
            doubt: Some(TierDoubt::EitherAffix {
                prefix: affix_tier(Affix::Prefix, 1, &[(42.0, 52.0), (40.0, 50.0)]),
                suffix: affix_tier(Affix::Suffix, 2, &[(45.0, 55.0), (35.0, 45.0)]),
            }),
            ..advanced(None, None, &["+50 to Armour", "+41 to Evasion Rating"])
        }]
    );
}

#[test]
fn the_tier_of_an_affix_has_no_range_when_its_values_are_fixed() {
    let grenades = trade_lines(&["Grenade Skills have +1 Cooldown Use"]);
    // As a prefix, one cooldown use only in its tier; as a suffix, one or two.
    let prefix = line_tiers(1, &[(2.0, 2.0), (1.0, 1.0)]);
    let suffix = line_tiers(1, &[(3.0, 4.0), (1.0, 2.0)]);
    let found = found_either(0..1, [(Affix::Prefix, prefix), (Affix::Suffix, suffix)]);

    assert_eq!(
        written_again(&grenades, &[found]),
        [Modifier {
            doubt: Some(TierDoubt::EitherAffix {
                prefix: affix_tier(Affix::Prefix, 2, &[]),
                suffix: affix_tier(Affix::Suffix, 2, &[(1.0, 2.0)]),
            }),
            ..grenades[0].clone()
        }]
    );
}

#[test]
fn the_tier_of_an_affix_has_the_ranges_written_on_the_values() {
    let lines = trade_lines(&[
        "15% reduced Rarity of Items found",
        "12% chance for Skills to retain 40% of Glory on use",
    ]);
    // The rarity rolls negative values; the 40 of the glory is written by its string.
    let prefix = tiers(
        &[1, 1],
        0,
        &[&[(-19.0, -10.0), (10.0, 15.0)], &[(-9.0, -5.0), (5.0, 9.0)]],
    );
    let suffix = tiers(
        &[1, 1],
        1,
        &[
            &[(-39.0, -30.0), (14.0, 16.0)],
            &[(-29.0, -14.0), (11.0, 13.0)],
        ],
    );
    let found = found_either(0..2, [(Affix::Prefix, prefix), (Affix::Suffix, suffix)]);

    // Without their sign, and none for the 40.
    let written = written_again(&lines, &[found]);
    assert_eq!(
        written.first().and_then(|modifier| modifier.doubt.clone()),
        Some(TierDoubt::EitherAffix {
            prefix: affix_tier(Affix::Prefix, 1, &[(10.0, 19.0), (10.0, 15.0)]),
            suffix: affix_tier(Affix::Suffix, 2, &[(14.0, 29.0), (11.0, 13.0)]),
        })
    );
}

#[test]
fn the_tier_of_an_affix_hides_its_number_and_names_its_lord_as_its_tiers_tell() {
    let grenades = trade_lines(&["Grenade Skills have +1 Cooldown Use"]);
    // The only tier of the prefix of a lord, the second of two tiers of an ordinary suffix.
    let of_a_lord = ModifierTiers {
        lord: Some(Lord::Amanamu),
        ..line_tiers(0, &[(1.0, 2.0)])
    };
    let ordinary = line_tiers(1, &[(3.0, 4.0), (1.0, 2.0)]);
    let unwritten = ModifierTiers {
        tier_unwritten: true,
        ..ordinary.clone()
    };
    let doubt = |prefix: &ModifierTiers, suffix: &ModifierTiers| {
        let affixes = [
            (Affix::Prefix, prefix.clone()),
            (Affix::Suffix, suffix.clone()),
        ];
        let written = written_again(&grenades, &[found_either(0..1, affixes)]);
        written.first().and_then(|modifier| modifier.doubt.clone())
    };
    let only_tier = |affix, lord| AffixTier {
        hides_number: true,
        lord,
        ..affix_tier(affix, 1, &[(1.0, 2.0)])
    };
    let second_tier = |affix, hides_number| AffixTier {
        hides_number,
        ..affix_tier(affix, 2, &[(1.0, 2.0)])
    };

    assert_eq!(
        doubt(&of_a_lord, &ordinary),
        Some(TierDoubt::EitherAffix {
            prefix: only_tier(Affix::Prefix, Some(Lord::Amanamu)),
            suffix: second_tier(Affix::Suffix, false),
        })
    );
    // The other way round, and a tier the game doesn't write.
    assert_eq!(
        doubt(&unwritten, &of_a_lord),
        Some(TierDoubt::EitherAffix {
            prefix: second_tier(Affix::Prefix, true),
            suffix: only_tier(Affix::Suffix, Some(Lord::Amanamu)),
        })
    );
}

#[test]
fn modifiers_whose_two_affixes_cant_be_written_stay_as_they_were() {
    let lines = trade_lines(&["+50 to Armour", "+41 to Evasion Rating"]);
    let hybrid = || tiers(&[1, 1], 0, &[&[(42.0, 52.0), (40.0, 50.0)]]);
    let beyond = || ModifierTiers {
        found: 1,
        ..hybrid()
    };
    let unwritten = |affixes: Vec<(Affix, ModifierTiers)>| {
        let found = [FoundTiers {
            modifiers: 0..2,
            affixes,
            added: None,
        }];
        assert_eq!(written_again(&lines, &found), lines, "{found:?}");
    };

    // Written again as they are, but for what each case changes.
    let either = [found_either(
        0..2,
        [(Affix::Prefix, hybrid()), (Affix::Suffix, hybrid())],
    )];
    assert_ne!(written_again(&lines, &either), lines);
    // The tier found as a prefix, then as a suffix, isn't among its tiers.
    unwritten(vec![(Affix::Prefix, beyond()), (Affix::Suffix, hybrid())]);
    unwritten(vec![(Affix::Prefix, hybrid()), (Affix::Suffix, beyond())]);
    // The suffix first; an affix twice; a third affix.
    unwritten(vec![(Affix::Suffix, hybrid()), (Affix::Prefix, hybrid())]);
    unwritten(vec![(Affix::Prefix, hybrid()), (Affix::Prefix, hybrid())]);
    unwritten(vec![(Affix::Suffix, hybrid()), (Affix::Suffix, hybrid())]);
    unwritten(vec![
        (Affix::Prefix, hybrid()),
        (Affix::Suffix, hybrid()),
        (Affix::Suffix, hybrid()),
    ]);
}

#[test]
fn a_line_that_adds_up_two_modifiers_of_one_affix_is_of_that_affix_with_the_tiers_of_each_part() {
    let damage = trade_lines(&["258% increased Physical Damage"]);
    // The second of three tiers, and the third of three of a pure modifier.
    let whole = line_tiers(1, &[(80.0, 89.0), (75.0, 79.0), (60.0, 74.0)]);
    let added = line_tiers(2, &[(190.0, 199.0), (180.0, 189.0), (170.0, 179.0)]);

    for affix in [Affix::Prefix, Affix::Suffix] {
        let found = split(found(0..1, affix, whole.clone()), 0, affix, added.clone());

        // Without tier, its value without range.
        assert_eq!(
            written_again(&damage, &[found]),
            [Modifier {
                doubt: Some(TierDoubt::Split {
                    line: 0,
                    whole: part(affix, &[2], &[(75.0, 79.0)]),
                    added: part(affix, &[3], &[(170.0, 179.0)])
                }),
                ..advanced(Some(affix), None, &["258% increased Physical Damage"])
            }]
        );
    }
}

#[test]
fn a_line_that_adds_up_a_prefix_and_a_suffix_is_explicit_without_affix() {
    // Marked desecrated, which the modifier written again stays.
    let mut rarity = trade_lines(&["37% increased Rarity of Items found"]);
    rarity[0].origin = Some(ModifierOrigin::Desecrated);
    let whole = line_tiers(0, &[(16.0, 19.0), (12.0, 15.0)]);
    let added = line_tiers(1, &[(19.0, 21.0), (15.0, 18.0)]);

    for (affix, added_affix) in [
        (Affix::Prefix, Affix::Suffix),
        (Affix::Suffix, Affix::Prefix),
    ] {
        let whole = found(0..1, affix, whole.clone());
        let found = split(whole, 0, added_affix, added.clone());

        assert_eq!(
            written_again(&rarity, &[found]),
            [Modifier {
                origin: Some(ModifierOrigin::Desecrated),
                doubt: Some(TierDoubt::Split {
                    line: 0,
                    whole: part(affix, &[1], &[(16.0, 19.0)]),
                    added: part(added_affix, &[2], &[(15.0, 18.0)])
                }),
                ..advanced(None, None, &["37% increased Rarity of Items found"])
            }]
        );
    }
}

#[test]
fn the_other_lines_of_the_hybrid_of_a_split_take_its_ranges_and_the_summed_line_none() {
    // After the life, a hybrid of three lines, as the longest modifier of the game: its
    // second line, the third modifier of the item, adds up a pure modifier.
    let lines = trade_lines(&[
        "+85 to maximum Life",
        "+50 to Armour",
        "115% increased Armour and Evasion",
        "+41 to Evasion Rating",
    ]);
    let hybrid = tiers(
        &[1, 1, 1],
        1,
        &[
            &[(53.0, 60.0), (43.0, 46.0), (51.0, 60.0)],
            &[(42.0, 52.0), (39.0, 42.0), (40.0, 50.0)],
        ],
    );
    let pure = line_tiers(
        3,
        &[(100.0, 110.0), (92.0, 99.0), (80.0, 91.0), (68.0, 79.0)],
    );
    let found = split(found(1..4, Affix::Prefix, hybrid), 2, Affix::Prefix, pure);

    assert_eq!(
        written_again(&lines, &[found]),
        [
            lines[0].clone(),
            Modifier {
                doubt: Some(TierDoubt::Split {
                    line: 1,
                    whole: part(Affix::Prefix, &[2], &[(39.0, 42.0)]),
                    added: part(Affix::Prefix, &[4], &[(68.0, 79.0)])
                }),
                ..advanced(
                    Some(Affix::Prefix),
                    None,
                    &[
                        "+50(42-52) to Armour",
                        "115% increased Armour and Evasion",
                        "+41(40-50) to Evasion Rating",
                    ]
                )
            }
        ]
    );
}

#[test]
fn the_summed_line_is_counted_among_the_lines_of_the_modifiers_before_it() {
    // A modifier of two lines, which the in-game trade never writes, before the summed line.
    let lines = [
        modifier_of_lines(&["+50 to Armour", "+41 to Evasion Rating"]),
        modifier_of_lines(&["115% increased Armour and Evasion"]),
    ];
    let hybrid = tiers(
        &[1, 1, 1],
        1,
        &[
            &[(53.0, 60.0), (51.0, 60.0), (43.0, 46.0)],
            &[(42.0, 52.0), (40.0, 50.0), (39.0, 42.0)],
        ],
    );
    let pure = line_tiers(1, &[(80.0, 91.0), (68.0, 79.0)]);
    let found = split(found(0..2, Affix::Prefix, hybrid), 1, Affix::Prefix, pure);

    // The third line of the modifier written again, of the second modifier of the item.
    assert_eq!(
        written_again(&lines, &[found]),
        [Modifier {
            doubt: Some(TierDoubt::Split {
                line: 2,
                whole: part(Affix::Prefix, &[2], &[(39.0, 42.0)]),
                added: part(Affix::Prefix, &[2], &[(68.0, 79.0)])
            }),
            ..advanced(
                Some(Affix::Prefix),
                None,
                &[
                    "+50(42-52) to Armour",
                    "+41(40-50) to Evasion Rating",
                    "115% increased Armour and Evasion",
                ]
            )
        }]
    );
}

#[test]
fn a_part_in_two_neighbouring_tiers_has_both_the_best_first_over_the_range_of_both() {
    let rarity = trade_lines(&["18% increased Rarity of Items found"]);
    let sure = line_tiers(1, &[(5.0, 6.0), (3.0, 4.0)]);
    // The second or the third of three tiers.
    let either = or_the_tier_above(line_tiers(2, &[(20.0, 23.0), (16.0, 19.0), (12.0, 15.0)]));
    let doubt = |lines: &[Modifier], whole: &ModifierTiers, added: &ModifierTiers| {
        let whole = found(0..1, Affix::Prefix, whole.clone());
        let found = split(whole, 0, Affix::Suffix, added.clone());
        let written = written_again(lines, &[found]);
        written.first().and_then(|modifier| modifier.doubt.clone())
    };
    let sure_part = |affix| part(affix, &[2], &[(3.0, 4.0)]);
    let either_part = |affix| part(affix, &[2, 3], &[(12.0, 19.0)]);

    // The pure modifier added, then the modifier of every line.
    assert_eq!(
        doubt(&rarity, &sure, &either),
        Some(TierDoubt::Split {
            line: 0,
            whole: sure_part(Affix::Prefix),
            added: either_part(Affix::Suffix)
        })
    );
    assert_eq!(
        doubt(&rarity, &either, &sure),
        Some(TierDoubt::Split {
            line: 0,
            whole: either_part(Affix::Prefix),
            added: sure_part(Affix::Suffix)
        })
    );
    // Each value of a line of two over its own range of both tiers.
    let damage = trade_lines(&["Adds 3 to 9 Physical Damage"]);
    let two_values = tiers(
        &[2],
        1,
        &[&[(1.0, 2.0), (5.0, 10.0)], &[(1.0, 4.0), (6.0, 12.0)]],
    );
    let one_tier = tiers(
        &[2],
        1,
        &[&[(3.0, 4.0), (5.0, 6.0)], &[(1.0, 2.0), (3.0, 4.0)]],
    );
    assert_eq!(
        doubt(&damage, &or_the_tier_above(two_values), &one_tier),
        Some(TierDoubt::Split {
            line: 0,
            whole: part(Affix::Prefix, &[1, 2], &[(1.0, 4.0), (5.0, 12.0)]),
            added: part(Affix::Suffix, &[2], &[(1.0, 2.0), (3.0, 4.0)])
        })
    );
}

#[test]
fn the_other_lines_of_a_hybrid_in_two_neighbouring_tiers_take_the_range_over_both() {
    let lines = trade_lines(&["248% increased Physical Damage", "+195 to Accuracy Rating"]);
    let hybrid = or_the_tier_above(tiers(
        &[1, 1],
        1,
        &[
            &[(80.0, 89.0), (200.0, 220.0)],
            &[(75.0, 79.0), (175.0, 199.0)],
        ],
    ));
    let pure = line_tiers(1, &[(170.0, 179.0), (155.0, 169.0)]);
    let found = split(found(0..2, Affix::Prefix, hybrid), 0, Affix::Prefix, pure);

    assert_eq!(
        written_again(&lines, &[found]),
        [Modifier {
            doubt: Some(TierDoubt::Split {
                line: 0,
                whole: part(Affix::Prefix, &[1, 2], &[(75.0, 89.0)]),
                added: part(Affix::Prefix, &[2], &[(155.0, 169.0)])
            }),
            ..advanced(
                Some(Affix::Prefix),
                None,
                &[
                    "248% increased Physical Damage",
                    "+195(175-220) to Accuracy Rating",
                ]
            )
        }]
    );
}

#[test]
fn another_line_whose_two_tiers_dont_range_the_same_values_keeps_them_without_range() {
    let lines = trade_lines(&[
        "248% increased Physical Damage",
        "5% increased Rarity of Items found",
        "+195 to Accuracy Rating",
    ]);
    // The range of the rarity crosses zero in the first tier, and isn't written: in the
    // second only, it can't be told over both.
    let hybrid = or_the_tier_above(tiers(
        &[1, 1, 1],
        1,
        &[
            &[(80.0, 89.0), (-10.0, 10.0), (200.0, 220.0)],
            &[(75.0, 79.0), (1.0, 9.0), (175.0, 199.0)],
        ],
    ));
    let pure = line_tiers(1, &[(170.0, 179.0), (155.0, 169.0)]);
    let found = split(found(0..3, Affix::Prefix, hybrid), 0, Affix::Prefix, pure);

    // The split is written, the accuracy with its range.
    assert_eq!(
        written_again(&lines, &[found]),
        [Modifier {
            doubt: Some(TierDoubt::Split {
                line: 0,
                whole: part(Affix::Prefix, &[1, 2], &[(75.0, 89.0)]),
                added: part(Affix::Prefix, &[2], &[(155.0, 169.0)])
            }),
            ..advanced(
                Some(Affix::Prefix),
                None,
                &[
                    "248% increased Physical Damage",
                    "5% increased Rarity of Items found",
                    "+195(175-220) to Accuracy Rating",
                ]
            )
        }]
    );
}

#[test]
fn a_part_hides_the_number_of_its_tiers_and_names_its_currency_as_its_tiers_tell() {
    let rarity = trade_lines(&["18% increased Rarity of Items found"]);
    let ordinary = line_tiers(1, &[(16.0, 19.0), (12.0, 15.0)]);
    // The only tier of the modifier of an essence, whose tier the game doesn't write either.
    let of_an_essence = ModifierTiers {
        tier_unwritten: true,
        currency: Some(Currency::Essence),
        ..line_tiers(0, &[(3.0, 6.0)])
    };
    let only_tier = line_tiers(0, &[(3.0, 6.0)]);
    let unwritten = ModifierTiers {
        tier_unwritten: true,
        ..ordinary.clone()
    };
    let doubt = |whole: &ModifierTiers, added: &ModifierTiers| {
        let whole = found(0..1, Affix::Prefix, whole.clone());
        let found = split(whole, 0, Affix::Suffix, added.clone());
        let written = written_again(&rarity, &[found]);
        written.first().and_then(|modifier| modifier.doubt.clone())
    };
    let ordinary_part = |affix| part(affix, &[2], &[(12.0, 15.0)]);
    let hidden = |affix, tier, range: (f64, f64), currency| PartTier {
        hides_number: true,
        currency,
        ..part(affix, &[tier], &[range])
    };

    assert_eq!(
        doubt(&ordinary, &of_an_essence),
        Some(TierDoubt::Split {
            line: 0,
            whole: ordinary_part(Affix::Prefix),
            added: hidden(Affix::Suffix, 1, (3.0, 6.0), Some(Currency::Essence))
        })
    );
    assert_eq!(
        doubt(&of_an_essence, &ordinary),
        Some(TierDoubt::Split {
            line: 0,
            whole: hidden(Affix::Prefix, 1, (3.0, 6.0), Some(Currency::Essence)),
            added: ordinary_part(Affix::Suffix)
        })
    );
    // One tier only on the base, then a tier the game doesn't write: no currency.
    assert_eq!(
        doubt(&only_tier, &unwritten),
        Some(TierDoubt::Split {
            line: 0,
            whole: hidden(Affix::Prefix, 1, (3.0, 6.0), None),
            added: hidden(Affix::Suffix, 2, (12.0, 15.0), None)
        })
    );
}

#[test]
fn a_split_that_cant_be_written_leaves_its_lines_as_they_were() {
    let lines = trade_lines(&[
        "+85 to maximum Life",
        "18% increased Rarity of Items found",
        "Adds 3 to 9 Physical Damage",
    ]);
    // The rarity and the damage read as a hybrid, the life before them.
    let hybrid = || {
        tiers(
            &[1, 2],
            1,
            &[
                &[(16.0, 19.0), (1.0, 2.0), (5.0, 10.0)],
                &[(12.0, 15.0), (1.0, 4.0), (6.0, 12.0)],
            ],
        )
    };
    let rarity = || line_tiers(1, &[(5.0, 6.0), (3.0, 4.0)]);
    let unwritten = |found: FoundTiers| {
        let found = [found];
        assert_eq!(written_again(&lines, &found), lines, "{found:?}");
    };
    let whole = |tiers: ModifierTiers| found(1..3, Affix::Prefix, tiers);
    // The rarity, the second modifier of the item, adds up `added`.
    let split_rarity =
        |tiers: ModifierTiers, added: ModifierTiers| split(whole(tiers), 1, Affix::Suffix, added);

    // Written as it is, but for what each case changes.
    let written = [split_rarity(hybrid(), rarity())];
    assert_ne!(written_again(&lines, &written), lines);
    // A tier of the two of a part without the range of the value.
    let without_range = or_the_tier_above(tiers(&[1], 1, &[&[(5.0, 6.0)], &[]]));
    unwritten(split_rarity(hybrid(), without_range));
    let hybrid_without_range = or_the_tier_above(tiers(
        &[1, 2],
        1,
        &[
            &[(-5.0, 19.0), (1.0, 2.0), (5.0, 10.0)],
            &[(12.0, 15.0), (1.0, 4.0), (6.0, 12.0)],
        ],
    ));
    unwritten(split_rarity(hybrid_without_range, rarity()));
    // Each of the two tiers without the range of another value of the summed line: as many
    // ranges, but not of the same values.
    let damage = or_the_tier_above(tiers(
        &[2],
        1,
        &[&[(-1.0, 2.0), (5.0, 10.0)], &[(1.0, 4.0), (-2.0, 12.0)]],
    ));
    unwritten(split(whole(hybrid()), 2, Affix::Suffix, damage));
    // The tier found of a part isn't among its tiers.
    let beyond = |tiers: ModifierTiers| ModifierTiers { found: 2, ..tiers };
    unwritten(split_rarity(beyond(hybrid()), rarity()));
    unwritten(split_rarity(hybrid(), beyond(rarity())));
    // The line that adds them up before the modifiers of the split, then after them.
    unwritten(split(whole(hybrid()), 0, Affix::Suffix, rarity()));
    unwritten(split(whole(hybrid()), 3, Affix::Suffix, rarity()));
    // A split of a modifier that can be a prefix or a suffix.
    let either = found_either(1..3, [(Affix::Prefix, hybrid()), (Affix::Suffix, hybrid())]);
    unwritten(split(either, 1, Affix::Suffix, rarity()));
}
