//! The rows of a unique, copied with its ranges, without them, or from the in-game trade.

use super::*;
use crate::filters::Half;
use crate::item::Range;
use crate::ui::item_stats::fixtures::{parse, rows, texts, with_soul_tether_rolls};

#[test]
fn a_line_of_a_unique_without_range_never_varies() {
    let headhunter = parse(
        "Item Class: Belts\nRarity: Unique\nHeadhunter\nHeavy Belt\n--------\n\
         Item Level: 80\n--------\n{ Implicit Modifier }\n\
         26(20-30)% increased Stun Threshold\n--------\n{ Unique Modifier }\n\
         When you kill a Rare monster, you gain its Modifiers for 60 seconds\n\
         { Unique Modifier }\n+55(40-60) to maximum Life",
    );

    assert_eq!(
        texts(&headhunter),
        [
            "Implicit",
            "■ 26% increased Stun Threshold ≥26 [20–30 0.60]",
            "Unique",
            // The stats that vary first.
            "■ +55 to maximum Life ≥55 [40–60 0.75]",
            // The string of its stat writes the 60: it never varies.
            "When you kill a Rare monster, you gain its Modifiers for 60 seconds",
        ]
    );
}

#[test]
fn the_vaal_modifier_of_a_unique_says_vaal_after_its_range() {
    let belt = parse(include_str!(
        "../../../item/test-items/fr-unique-belt-vaal.txt"
    ));

    let texts = texts(&belt);
    let vaal: Vec<&String> = texts.iter().filter(|text| text.contains("Vaal")).collect();
    // A filter like the other modifiers of the unique, its origin after its range.
    assert_eq!(
        vaal,
        ["■ +112 au Score d'Évasion tant que votre Vie est Basse ≥112 [100–150 0.24 Vaal]"]
    );
}

#[test]
fn the_vaal_modifier_of_a_unique_says_vaal_under_a_fixed_line() {
    let unique = parse(
        "Item Class: Rings\nRarity: Unique\nTest Loop\nGold Ring\n--------\n\
         Item Level: 80\n--------\n{ Vaal Unique Modifier }\n\
         +5% to Maximum Lightning Resistance\n--------\nCorrupted",
    );

    assert_eq!(
        texts(&unique),
        ["Unique", "+5% to Maximum Lightning Resistance [Vaal]"]
    );
}

#[test]
fn a_unique_copied_from_the_in_game_trade_has_unchecked_filters_before_its_rolls() {
    let belt = parse(include_str!(
        "../../../item/test-items/en-owner-trade-unique-belt-mutated.txt"
    ));

    assert_eq!(
        texts(&belt),
        [
            "Enchant",
            "□ 15% increased Evasion Rating ≥15",
            "Implicit",
            "□ 16% increased Charm Effect Duration ≥16",
            "□ Has 3 Charm Slots ≥3",
            "Unique",
            "□ 14% increased Energy Shield Recovery rate ≥14 [Vaal]",
            "□ +51 to maximum Energy Shield ≥51",
            "□ You lose 5% of maximum Energy Shield per second ≤5",
            // No number: the same on every copy.
            "Excess Life Recovery from Leech is applied to Energy Shield",
        ]
    );
}

#[test]
fn a_french_unique_copied_from_the_in_game_trade_gets_the_rolls_by_its_stats() {
    let belt = with_soul_tether_rolls(parse(include_str!(
        "../../../item/test-items/fr-trade-unique-belt-mutated.txt"
    )));

    assert_eq!(
        texts(&belt),
        [
            "Enchant",
            "□ 15% d'Augmentation de l'Armure ≥15",
            "Implicit",
            "■ 16% d'Augmentation de la Durée de l'Effet des Charmes ≥16 [15–20 0.20]",
            "■ Possède 2 Emplacements de Charme ≥2 [1–3 0.50]",
            "Unique",
            "■ +40 de Bouclier d'énergie maximal ≥40 [40–60 0.00]",
            // A line poe2scout doesn't hold.
            "□ +20 d'Intelligence ≥20",
            "□ 14% d'Augmentation de la Récupération de Bouclier d'énergie ≥14 [Vaal]",
            "Vous perdez 5% de Bouclier d'énergie par seconde",
            "Unique rolls note",
        ]
    );
}

#[test]
fn the_line_of_a_unique_without_header_never_varies() {
    let ring = parse(include_str!(
        "../../../item/test-items/fr-unique-ring-mirrored.txt"
    ));

    // Like a stat that never varies: no checkbox, from the left edge.
    assert_eq!(texts(&ring), ["Unique", "Reflète la Bague opposée"]);
}

#[test]
fn a_unique_copied_without_any_range_has_only_fixed_lines() {
    let unique = parse(
        "Item Class: Rings\nRarity: Unique\nTest Loop\nGold Ring\n--------\n\
         Item Level: 80\n--------\n{ Unique Modifier }\n\
         +5% to Maximum Lightning Resistance\n{ Unique Modifier }\n+52 to maximum Life",
    );

    // The advanced copy writes a range for every number that rolls.
    assert_eq!(
        texts(&unique),
        [
            "Unique",
            "+5% to Maximum Lightning Resistance",
            "+52 to maximum Life"
        ]
    );
}

#[test]
fn a_number_written_by_the_string_of_a_stat_has_no_range() {
    let unique = parse(
        "Item Class: Gloves\nRarity: Unique\nTest Grip\nSuede Bracers\n--------\n\
         Item Level: 80\n--------\n{ Unique Modifier }\n\
         1% more Unarmed Damage per 5(4-6) Strength",
    );

    // The 1 is written by the string of the stat: the field holds the 5, in its range.
    assert_eq!(
        texts(&unique),
        [
            "Unique",
            "■ 1% more Unarmed Damage per 5 Strength ≥5 [4–6 0.50]"
        ]
    );
}

#[test]
fn an_implicit_and_an_explicit_of_the_same_stat_keep_their_own_ranges() {
    let ring = parse(
        "Item Class: Rings\nRarity: Unique\nVenom Loop\nAmethyst Ring\n--------\n\
         Item Level: 80\n--------\n{ Implicit Modifier }\n\
         +10(7-13)% to Chaos Resistance\n--------\n{ Unique Modifier }\n\
         +20(17-23)% to Chaos Resistance\n{ Unique Modifier }\n\
         Adds 1 to 55(50-60) Lightning damage to Attacks",
    );

    assert_eq!(
        texts(&ring),
        [
            "Implicit",
            "■ +10% to Chaos Resistance ≥10 [7–13 0.50]",
            "Unique",
            "■ +20% to Chaos Resistance ≥20 [17–23 0.50]",
            // The fixed number before the roll keeps its place; the minimum is the
            // average of both numbers, as on the trade site.
            "■ Adds 1 to 55 Lightning damage to Attacks ≥28 [50–60 0.50]",
        ]
    );
}

#[test]
fn a_unique_copied_with_ctrl_alt_c_has_its_own_ranges() {
    let focus = parse(include_str!("../../../item/test-items/en-unique-focus.txt"));

    assert_eq!(
        texts(&focus),
        [
            "Unique",
            "■ 56% increased Energy Shield ≥56 [50–70 0.30]",
            "■ +26% to Lightning Resistance ≥26 [20–30 0.60]",
            "40% increased Mana Regeneration Rate while stationary",
            "+5% to Maximum Lightning Resistance",
            "40% increased Mana Regeneration Rate",
        ]
    );
}

#[test]
fn a_negative_range_of_the_advanced_copy_is_without_its_sign() {
    let ring = parse(
        "Item Class: Rings\nRarity: Unique\nCold Loop\nRuby Ring\n--------\n\
         { Unique Modifier — Elemental, Fire, Resistance }\n-15(-20--10)% to Fire Resistance\n\
         { Unique Modifier — Elemental, Cold, Resistance }\n2(-5-10)% to Cold Resistance",
    );

    assert_eq!(
        texts(&ring),
        [
            "Unique",
            "■ -15% to Fire Resistance ≥-15 [10–20 0.50]",
            // A range across zero needs its sign: no bar.
            "2% to Cold Resistance",
        ]
    );
}

#[test]
fn a_legacy_unique_keeps_its_value_out_of_the_range_of_today() {
    let belt = parse(
        "Item Class: Belts\nRarity: Unique\nHeadhunter\nHeavy Belt\n--------\n\
         Item Level: 80\n--------\n{ Unique Modifier }\n+65(40-60) to maximum Life",
    );
    let rows = rows(&belt);

    assert_eq!(
        rows[1],
        Row::Stat(Stat {
            key: FilterKey::Stat(StatKey {
                modifier: 0,
                line: 0
            }),
            text: StatText::Line("+65 to maximum Life"),
            filter: Some(StatFilter {
                checked: true,
                limit: Some(Limit {
                    value: 65.0,
                    half: Half::Minimum,
                    bounds: Some(Range {
                        min: 40.0,
                        max: 65.0
                    }),
                    rolls: Some(Range {
                        min: 40.0,
                        max: 60.0
                    }),
                    tiers: None,
                }),
            }),
            unsearchable: false,
            roll: Some(Roll::Bar {
                share: 1.0,
                ranges: vec![Range {
                    min: 40.0,
                    max: 60.0
                }],
                origin: None,
            }),
        })
    );
}
