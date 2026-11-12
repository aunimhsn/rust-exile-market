//! The lookup on the tables of the game and the test items: the tier and the ranges the
//! advanced copy writes, found again. The numbers of a patch, to record again when the game
//! data are generated again.

use super::*;
use crate::game_data::lines::modifier_stats;
use crate::game_data::modifiers::fixtures::{desecrated, modifier, parse, range, tiers};
use crate::item::{Currency, ModifierOrigin, Range};

/// A tier number and the ranges of its values, as the game writes them.
type TierRanges = (u32, Vec<Range>);

/// The tier found from the tables for each prefix and suffix of `text`, and the tier the
/// advanced copy writes, with its ranges: (found, written). A magic item gives its
/// `base_type`, found among the names of the trade site in the application.
fn tiers_of(text: &str, base_type: Option<&str>) -> Vec<(Option<TierRanges>, TierRanges)> {
    let item = parse(text);
    let base = ItemBase {
        language: item.language(),
        base_type: base_type.unwrap_or(item.base_type()),
        item_level: None,
    };
    item.modifiers()
        .iter()
        .filter(|modifier| {
            matches!(modifier.kind, ModifierKind::Explicit(Some(_))) && modifier.tier.is_some()
        })
        .map(|modifier| {
            let matches = modifier_stats(item.language(), modifier, None);
            let found = modifier_tiers(base, modifier, &matches).and_then(|tiers| {
                let tier = tiers.tier()?;
                Some((
                    tier.number,
                    tier.ranges
                        .iter()
                        .filter_map(|range| range.magnitude())
                        .collect(),
                ))
            });
            let written = (
                modifier.tier.expect("a tier"),
                modifier
                    .lines
                    .iter()
                    .flat_map(|line| &line.rolls)
                    .filter_map(|roll| roll.range)
                    .collect(),
            );
            (found, written)
        })
        .collect()
}

#[test]
fn the_tables_find_the_tier_written_by_the_advanced_copy() {
    let mut found = 0;
    let mut missing = Vec::new();
    for (text, base_type) in [
        (
            include_str!("../../../item/test-items/en-rare-crossbow-all-modifiers.txt"),
            None,
        ),
        (
            include_str!("../../../item/test-items/en-rare-bow-fractured.txt"),
            None,
        ),
        (
            include_str!("../../../item/test-items/en-magic-mace.txt"),
            Some("Temple Maul"),
        ),
        (
            include_str!("../../../item/test-items/fr-rare-staff.txt"),
            None,
        ),
        (
            include_str!("../../../item/test-items/fr-rare-body-armour.txt"),
            None,
        ),
        (
            include_str!("../../../item/test-items/fr-rare-body-armour-two-hybrids.txt"),
            None,
        ),
        (
            include_str!("../../../item/test-items/en-owner-rare-sceptre-sanctified.txt"),
            None,
        ),
        (
            include_str!("../../../item/test-items/fr-rare-sceptre-sanctified.txt"),
            None,
        ),
    ] {
        for (tier, (number, ranges)) in tiers_of(text, base_type) {
            // The crossbow was copied before a patch added a tier above its leech, `of
            // the Desperate` (8–8.9), the S1 of the offers checked on the trade site on
            // 2026-10-27.
            let number = if ranges == [range(7.0, 7.9)] {
                2
            } else {
                number
            };
            if let Some(tier) = tier {
                assert_eq!(tier, (number, ranges));
                found += 1;
            } else {
                missing.push(ranges);
            }
        }
    }
    // With the two modifiers of Amanamu, a lord of the Abyss, among the desecrated tables.
    assert_eq!(found, 39);
    // A line whose stat the tables have under another tier is a mistake; a line the tables
    // don't have isn't: the lightning damage of the mace, copied before a patch changed
    // its ranges.
    assert_eq!(missing, [vec![range(1.0, 4.0), range(46.0, 66.0)]]);
}

#[test]
fn a_sanctified_value_out_of_its_tier_finds_it_by_the_copied_ranges() {
    for text in [
        include_str!("../../../item/test-items/en-owner-rare-sceptre-sanctified.txt"),
        include_str!("../../../item/test-items/fr-rare-sceptre-sanctified.txt"),
    ] {
        let found: Vec<_> = tiers_of(text, None)
            .into_iter()
            .filter_map(|(found, _)| found)
            .collect();
        // `+74(80-89) to maximum Mana` and `29(30)% reduced Attribute Requirements`.
        assert!(found.contains(&(5, vec![range(80.0, 89.0)])), "{found:?}");
        assert!(found.contains(&(2, vec![range(30.0, 30.0)])), "{found:?}");
    }

    // A line of two values, one of them out of its tier: `4(1-4) to 75(60-71)`.
    let text = include_str!("../../../item/test-items/en-owner-rare-sceptre-sanctified.txt")
        .replace("60(60-71)", "75(60-71)");
    let (found, written) = tiers_of(&text, None).swap_remove(0);
    assert_eq!(written, (1, vec![range(1.0, 4.0), range(60.0, 71.0)]));
    assert_eq!(found, Some(written));

    // Without the ranges of the advanced copy, a value out of every tier finds none.
    let mana = modifier(Some(Affix::Prefix), &["+74 to maximum Mana"]);
    assert_eq!(
        tiers("Rattling Sceptre", &mana).and_then(|tiers| Some(tiers.tier()?.number)),
        Some(6),
        "74 is in the range of tier 6"
    );
    let mana = modifier(Some(Affix::Prefix), &["+400 to maximum Mana"]);
    assert_eq!(tiers("Rattling Sceptre", &mana), None);
}

#[test]
fn a_line_written_without_number_finds_its_tier_by_the_value_of_its_string() {
    let number_of = |line: &str| {
        let bolts = modifier(Some(Affix::Suffix), &[line]);
        tiers("Gemini Crossbow", &bolts).and_then(|tiers| Some(tiers.tier()?.number))
    };

    assert_eq!(number_of("Loads an additional bolt"), Some(2));
    assert_eq!(number_of("Loads 2 additional bolts"), Some(1));
    assert_eq!(number_of("Loads 3 additional bolts"), None);
}

#[test]
fn the_lines_of_a_hybrid_in_another_order_are_the_same_modifier() {
    let champion = modifier(
        Some(Affix::Prefix),
        &["+100 to Accuracy Rating", "50% increased Physical Damage"],
    );

    let tiers = tiers("Gemini Crossbow", &champion).expect("its tiers");

    let tier = tiers.tier().expect("its tier");
    assert_eq!(tier.number, 4);
    // In the order of the copied lines.
    assert_eq!(tier.ranges, [range(98.0, 123.0), range(45.0, 54.0)]);
}

#[test]
fn a_modifier_outside_the_prefixes_and_suffixes_has_no_tiers() {
    let mut implicit = modifier(None, &["Loads an additional bolt"]);
    implicit.kind = ModifierKind::Implicit;
    // A header the parser doesn't know tells no affix.
    let unknown = modifier(None, &["Adds 10 to 273 Lightning Damage"]);
    let mut desecrated = modifier(Some(Affix::Prefix), &["Adds 10 to 273 Lightning Damage"]);
    desecrated.origin = Some(ModifierOrigin::Desecrated);

    assert_eq!(tiers("Gemini Crossbow", &implicit), None);
    assert_eq!(tiers("Gemini Crossbow", &unknown), None);
    // A base the data doesn't know.
    assert_eq!(tiers("Unknown Crossbow", &desecrated), None);
    // An ordinary modifier desecrated is in the tables.
    assert!(tiers("Gemini Crossbow", &desecrated).is_some());
}

#[test]
fn a_line_without_value_is_found_with_the_other_lines_of_its_modifier() {
    let instant = modifier(
        Some(Affix::Prefix),
        &["Instant Recovery", "50% reduced Amount Recovered"],
    );

    let tiers = tiers("Ultimate Life Flask", &instant).expect("its tiers");

    assert_eq!(tiers.tier().map(|tier| tier.number), Some(1));
    assert_eq!(
        tiers.tier().map(|tier| tier.ranges.clone()),
        Some(vec![range(-50.0, -50.0)])
    );
}

#[test]
fn a_modifier_of_a_time_lost_jewel_is_found_within_the_words_of_the_passive_skills_it_grants() {
    // Of one tier, granted by the notables in the radius of the jewel; the same line alone is
    // a modifier of the other jewels, not of a Time-Lost one.
    let granted = modifier(
        Some(Affix::Suffix),
        &["Notable Passive Skills in Radius also grant 5% increased chance to inflict Ailments"],
    );
    let alone = modifier(
        Some(Affix::Suffix),
        &["5% increased chance to inflict Ailments"],
    );
    let found = |base_type: &str, modifier: &Modifier| {
        tiers(base_type, modifier).map(|tiers| {
            let ranges = tiers.tier().map(|tier| tier.ranges.clone());
            (tiers.single(), ranges)
        })
    };

    assert_eq!(
        found("Time-Lost Emerald", &granted),
        Some((true, Some(vec![range(3.0, 7.0)])))
    );
    assert_eq!(found("Time-Lost Emerald", &alone), None);
    assert_eq!(found("Emerald", &granted), None);
    assert_eq!(
        found("Emerald", &alone),
        Some((true, Some(vec![range(5.0, 15.0)])))
    );
}

#[test]
fn a_desecrated_modifier_of_a_time_lost_jewel_is_found_among_the_notables_it_grants() {
    // A hybrid of the Abyss on the jewels, which Exiled Exchange 2 knows granted by both kinds
    // of passive skills; and `1% of Damage taken Recouped as Life`, an ordinary suffix of the
    // Sapphires or a desecrated prefix, the desecrated tables holding only a line marked so.
    let life = desecrated(
        Affix::Prefix,
        &["Notable Passive Skills in Radius also grant 1% increased maximum Life"],
    );
    let recoup = "Notable Passive Skills in Radius also grant 1% of Damage taken Recouped as Life";
    let single = |modifier: &Modifier| {
        tiers("Time-Lost Sapphire", modifier).map(|tiers| {
            let ranges = tiers.tier().map(|tier| tier.ranges.clone());
            (tiers.single(), ranges)
        })
    };
    let one = Some((true, Some(vec![range(1.0, 1.0)])));

    assert_eq!(single(&life), one);
    assert_eq!(single(&desecrated(Affix::Prefix, &[recoup])), one);
    assert_eq!(single(&modifier(Some(Affix::Suffix), &[recoup])), one);
    assert_eq!(single(&modifier(Some(Affix::Prefix), &[recoup])), None);
}

#[test]
fn a_desecrated_table_is_a_modifier_of_one_tier_of_its_own() {
    // The cost efficiency of Kurgal, written twice with the same range on the maces.
    let efficiency = desecrated(Affix::Suffix, &["12% increased Cost Efficiency of Attacks"]);
    // The flask charges of Kurgal (10–15) and of Ulaman (10–18) on the belts.
    let charges = |value: u32| {
        desecrated(
            Affix::Prefix,
            &[&format!(
                "{value}% chance for Flasks you use to not consume Charges"
            )],
        )
    };
    let only = |base_type: &str, modifier: &Modifier| {
        tiers(base_type, modifier).map(|tiers| {
            let ranges = tiers.tier().map(|tier| tier.ranges.clone());
            (tiers.single(), ranges)
        })
    };

    assert_eq!(
        only("Temple Maul", &efficiency),
        Some((true, Some(vec![range(8.0, 15.0)])))
    );
    // Not a T1 and a T2 of one modifier: each its own tier, the first of the file on a tie.
    assert_eq!(
        only("Heavy Belt", &charges(12)),
        Some((true, Some(vec![range(10.0, 15.0)])))
    );
    assert_eq!(
        only("Heavy Belt", &charges(17)),
        Some((true, Some(vec![range(10.0, 18.0)])))
    );
    // Named only when one lord holds it: 12 is Kurgal's or Ulaman's, 17 Ulaman's.
    let lord = |modifier: &Modifier| tiers("Heavy Belt", modifier).and_then(|tiers| tiers.lord);
    assert_eq!(lord(&charges(12)), None);
    assert_eq!(lord(&charges(17)), Some(Lord::Ulaman));
    // The presence of Amanamu on a helmet, which the game always marks desecrated.
    let presence = ["30% increased Presence Area of Effect"];
    assert_eq!(
        only("Divine Crown", &desecrated(Affix::Suffix, &presence)),
        Some((true, Some(vec![range(25.0, 35.0)])))
    );
    assert_eq!(
        only("Divine Crown", &modifier(Some(Affix::Suffix), &presence)),
        None
    );
}

#[test]
fn a_modifier_of_an_essence_written_without_tier_is_not_the_ordinary_tier_of_its_value() {
    let numbers = |base_type: &str, line: &str, tier: Option<u32>| {
        let suffix = Modifier {
            tier,
            ..modifier(Some(Affix::Suffix), &[line])
        };
        tiers(base_type, &suffix).map(|tiers| {
            let number = tiers.tier().map(|tier| tier.number);
            (
                tiers.tiers.len(),
                number,
                tiers.tier_unwritten,
                tiers.currency,
            )
        })
    };
    let essence = Some(Currency::Essence);

    // `of the Essence` on a wand, its one value, the T2 of 4 of the ordinary modifier as well.
    let wand = "+3 to Level of all Spell Skills";
    assert_eq!(
        numbers("Acrid Wand", wand, None),
        Some((1, Some(1), true, essence))
    );
    assert_eq!(
        numbers("Acrid Wand", wand, Some(2)),
        Some((4, Some(2), false, None))
    );
    // On a staff, in the T1 of the ordinary one, `5–6`: the tier written, the ordinary one,
    // also when the essence's ties with it.
    let staff = "+5 to Level of all Spell Skills";
    assert_eq!(
        numbers("Ashen Staff", staff, None),
        Some((1, Some(1), true, essence))
    );
    assert_eq!(
        numbers("Ashen Staff", staff, Some(1)),
        Some((4, Some(1), false, None))
    );
}

#[test]
fn a_modifier_of_an_alloy_is_one_of_its_own_whose_tier_the_game_doesnt_write() {
    // Written crafted, as the game writes it.
    let crafted = |affix, lines: &[&str]| Modifier {
        origin: Some(ModifierOrigin::Crafted),
        ..modifier(Some(affix), lines)
    };
    let found = |base_type: &str, affix, lines: &[&str]| {
        tiers(base_type, &crafted(affix, lines)).map(|tiers| {
            let ranges = tiers.tier().map(|tier| tier.ranges.clone());
            (
                tiers.tiers.len(),
                ranges,
                tiers.hides_number(),
                tiers.currency,
            )
        })
    };
    let alloy = Some(Currency::Alloy);

    // The owner's gloves (`fr-rare-gloves-alloy.txt`), `of the Stars`.
    assert_eq!(
        found(
            "Adorned Wraps",
            Affix::Suffix,
            &["11% increased Attack Speed while missing Runic Ward"]
        ),
        Some((1, Some(vec![range(10.0, 15.0)]), true, alloy))
    );
    // Of the type of an ordinary modifier of other bases (`ManaCostEfficiency` of the
    // essences on the wands), apart from it.
    assert_eq!(
        found(
            "Face Mask",
            Affix::Prefix,
            &["26% increased Mana Cost Efficiency"]
        ),
        Some((1, Some(vec![range(18.0, 29.0)]), true, alloy))
    );
    // The cast speed and cold damage of the Transcendent Alloy: on a wand and a focus, the
    // values of the staves of an older patch as well as the one-handed ones, two tiers of one
    // modifier; on a staff, its values only.
    let cold = |speed: u32, cold: u32| {
        [
            format!("{speed}% increased Cast Speed"),
            format!("Gain {cold}% of Elemental Damage as Extra Cold Damage"),
        ]
    };
    let hybrid = |base_type: &str, (speed, gain): (u32, u32)| {
        let lines = cold(speed, gain);
        let lines: Vec<&str> = lines.iter().map(String::as_str).collect();
        found(base_type, Affix::Suffix, &lines)
    };
    let staves = Some(vec![range(39.0, 47.0), range(11.0, 16.0)]);
    let one_hand = Some(vec![range(26.0, 31.0), range(7.0, 11.0)]);
    for base_type in ["Acrid Wand", "Sacred Focus"] {
        assert_eq!(
            hybrid(base_type, (41, 15)),
            Some((2, staves.clone(), true, alloy)),
            "{base_type}"
        );
        assert_eq!(
            hybrid(base_type, (28, 9)),
            Some((2, one_hand.clone(), true, alloy)),
            "{base_type}"
        );
    }
    assert_eq!(
        hybrid("Ashen Staff", (41, 15)),
        Some((1, staves, true, alloy))
    );
}
