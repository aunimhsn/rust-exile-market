//! Which modifier lines start checked, the half of the range field they fill, and the limit of
//! a line of decimal rolls.

use super::*;
use crate::filters::Filter;
use crate::item;
use crate::ui::item_stats::fixtures::{default_checks, limit, parse, rows, texts};

#[test]
fn the_pseudo_totals_are_checked_and_the_lines_they_add_up_unchecked() {
    let ring = parse(
        "Item Class: Rings\nRarity: Rare\nStorm Spiral\nSapphire Ring\n--------\n\
         Item Level: 77\n--------\n{ Implicit Modifier }\n\
         +22(20-30)% to Cold Resistance\n--------\n\
         { Prefix Modifier \"Rotund\" (Tier: 4) — Life }\n+65(60-69) to maximum Life\n\
         { Prefix Modifier \"Agile\" (Tier: 6) — Defences }\n+60(52-79) to Evasion Rating\n\
         { Suffix Modifier \"of the Furnace\" (Tier: 2) — Elemental, Fire, Resistance }\n\
         +38(36-40)% to Fire Resistance\n\
         { Suffix Modifier \"of the Squall\" (Tier: 5) — Elemental, Lightning, Resistance }\n\
         +23(21-25)% to Lightning Resistance\n\
         { Suffix Modifier \"of the Lynx\" (Tier: 7) — Attribute }\n+12(9-12) to Dexterity",
    );

    assert_eq!(
        texts(&ring),
        [
            "Item",
            "■ Base: Sapphire Ring",
            "□ ItemLevel: 77 ≥77",
            "Pseudo",
            "■ ElementalResistance: 83 ≥75",
            "■ Life: 65 ≥59",
            "■ Dexterity: 12 ≥11",
            "Implicit",
            "□ +22% to Cold Resistance ≥20 [20–30 0.20]",
            "Prefixes 2/3",
            "□ +65 to maximum Life ≥59 [0.50 T4 60–69]",
            // Not added up by a total, unchecked as every modifier of a rare.
            "□ +60 to Evasion Rating ≥54 [0.23 T6 52–79]",
            "Suffixes 3/3",
            "□ +38% to Fire Resistance ≥34 [0.82 T2 36–40]",
            "□ +23% to Lightning Resistance ≥21 [0.44 T5 21–25]",
            "□ +12 to Dexterity ≥11 [0.25 T7 9–12]",
        ]
    );
}

#[test]
fn every_modifier_of_a_rare_starts_unchecked_and_those_of_a_magic_item_checked() {
    // The value of a rare lies in the few modifiers its buyer needs, which the application
    // can't tell: several checked at once left few offers (owner, 2026-11-10). A magic item
    // has two at most.
    let ring = |rarity: &str, name: &str| {
        texts(&parse(&format!(
            "Item Class: Rings\nRarity: {rarity}\n{name}Sapphire Ring\n--------\n\
             Item Level: 77\n--------\n\
             {{ Prefix Modifier \"Agile\" (Tier: 6) — Defences }}\n+60(52-79) to Evasion Rating\n\
             {{ Suffix Modifier \"of Archaeology\" (Tier: 1) }}\n\
             16(15-18)% increased Rarity of Items found"
        )))
    };
    let modifiers = |texts: Vec<String>| -> Vec<String> {
        texts
            .into_iter()
            .filter(|text| text.contains("Evasion") || text.contains("Rarity"))
            .collect()
    };

    assert_eq!(
        modifiers(ring("Rare", "Storm Spiral\n")),
        [
            "□ +60 to Evasion Rating ≥54 [0.23 T6 52–79]",
            "□ 16% increased Rarity of Items found ≥14 [0.83 T1 15–18]",
        ]
    );
    assert_eq!(
        modifiers(ring("Magic", "")),
        [
            "■ +60 to Evasion Rating ≥54 [0.23 T6 52–79]",
            "■ 16% increased Rarity of Items found ≥14 [0.83 T1 15–18]",
        ]
    );
}

#[test]
fn a_line_of_a_magic_item_starts_unchecked_only_when_a_checked_dps_or_total_counts_it() {
    // The rows of `item` that start with one of `starts`, in their order.
    let rows = |item: &Item, starts: &[&str]| -> Vec<String> {
        texts(item)
            .into_iter()
            .filter(|row| starts.iter().any(|start| row[4..].starts_with(start)))
            .collect()
    };
    let maul = |damage: &str, lines: &str| {
        parse(&format!(
            "Item Class: Two Hand Maces\nRarity: Magic\nHeavy Temple Maul\n--------\n\
             {damage}\nAttacks per Second: 1.00\n--------\nItem Level: 80\n--------\n{lines}"
        ))
    };
    // Its DPS, which its base reaches at 20 % quality, compares nothing: the line it would
    // count is checked in its place, as the other modifiers of a magic item.
    let weak = maul(
        "Physical Damage: 10-20 (augmented)",
        "25% increased Physical Damage",
    );
    assert_eq!(
        rows(&weak, &["PhysicalDps", "25%"]),
        [
            "□ PhysicalDps: 15 ≥13",
            "■ 25% increased Physical Damage ≥23"
        ]
    );
    // Its physical DPS checked counts its physical damage; its elemental DPS, smaller, starts
    // unchecked, so that nothing counts its lightning damage: checked.
    let strong = maul(
        "Physical Damage: 20-40 (augmented)\nLightning Damage: 1-4 (lightning)",
        "100% increased Physical Damage\nAdds 1 to 4 Lightning Damage",
    );
    assert_eq!(
        rows(&strong, &["PhysicalDps", "ElementalDps", "100%", "Adds"]),
        [
            "■ PhysicalDps: 30 ≥27",
            "□ ElementalDps: 3 ≥2",
            "□ 100% increased Physical Damage ≥90",
            "■ Adds 1 to 4 Lightning Damage ≥2",
        ]
    );

    // A Prismatic Ring: its total elemental resistance, 30 with its implicit, which any
    // Prismatic Ring reaches, starts unchecked, so that nothing counts its cold resistance nor
    // its implicit: checked. Its total of Life checked counts its line of Life.
    let ring = |lines: &str| {
        parse(&format!(
            "Item Class: Rings\nRarity: Magic\nHale Prismatic Ring of the Seal\n--------\n\
             Item Level: 80\n--------\n{{ Implicit Modifier }}\n\
             +8(7-10)% to all Elemental Resistances\n--------\n{lines}"
        ))
    };
    let life = "{ Prefix Modifier }\n+30 to maximum Life";
    let cold = "{ Suffix Modifier }\n+6% to Cold Resistance";
    let both = ring(&format!("{life}\n{cold}"));
    assert_eq!(
        rows(
            &both,
            &["ElementalResistance", "Life:", "+8%", "+30", "+6%"]
        ),
        [
            "□ ElementalResistance: 30 ≥27",
            "■ Life: 30 ≥27",
            "■ +8% to all Elemental Resistances ≥7 [7–10 0.33]",
            "□ +30 to maximum Life ≥27",
            "■ +6% to Cold Resistance ≥5",
        ]
    );
    // Its cold resistance alone compares the ring.
    assert_eq!(default_checks(&ring(cold)), crate::ui::StatChecks::Checked);
}

#[test]
fn the_attack_speed_of_an_item_that_isnt_a_weapon_stays_checked() {
    let quiver = parse(
        "Item Class: Quivers\nRarity: Magic\nQuick Broadhead Quiver\n--------\n\
         { Suffix Modifier \"of Skill\" (Tier: 4) }\n8(7-9)% increased Attack Speed",
    );

    assert_eq!(
        texts(&quiver),
        [
            "Item",
            "■ Base: Quick Broadhead Quiver",
            "Suffixes 1/1",
            "■ 8% increased Attack Speed ≥7 [T4 7–9]",
        ]
    );
}

#[test]
fn a_line_better_lower_is_searched_with_a_maximum() {
    let unique = parse(
        "Item Class: Boots\nRarity: Unique\nStormtread\nLattice Sandals\n--------\n\
         { Unique Modifier }\n+52(40-60) to maximum Life\n\
         { Unique Modifier }\n18(15-25)% reduced Movement Speed",
    );
    let rare = parse(
        "Item Class: Rings\nRarity: Rare\nGale Loop\nRuby Ring\n--------\n\
         { Implicit Modifier }\n12(10-15)% increased Charm Charges used\n--------\n\
         { Suffix Modifier \"of the Hare\" (Tier: 2) }\n18(15-25)% reduced Movement Speed",
    );

    assert_eq!(
        texts(&unique),
        [
            "Unique",
            "■ +52 to maximum Life ≥52 [40–60 0.60]",
            // Its bar still shows where its value falls in its range.
            "■ 18% reduced Movement Speed ≤18 [15–25 0.30]",
        ]
    );
    // 110 % of the copied value, within the range of the implicit.
    assert_eq!(
        texts(&rare),
        [
            "Item",
            "■ Base: Ruby Ring",
            "Implicit",
            "□ 12% increased Charm Charges used ≤13 [10–15 0.40]",
            "Suffixes 1/3",
            "□ 18% reduced Movement Speed ≤20 [T2 15–25]",
        ]
    );
    let lower = limit(&rows(&unique)[2]);
    assert_eq!(lower.half, Half::Maximum);
    assert_eq!(
        StatFilter {
            checked: true,
            limit: Some(lower),
        }
        .default_choice(),
        Filter {
            checked: true,
            minimum: String::new(),
            maximum: "18".to_owned(),
            bounds: Some(Range {
                min: 15.0,
                max: 25.0
            }),
        }
    );
}

#[test]
fn a_line_of_two_decimal_rolls_gets_a_limit_of_their_decimals() {
    let line = StatLine {
        text: "Adds # to # Fire Damage".to_owned(),
        rolls: vec![
            item::Roll {
                value: 2.4,
                range: None,
            },
            item::Roll {
                value: 4.4,
                range: None,
            },
        ],
    };
    let limit = limit_of_line(&line, None, Half::Minimum);
    // 90 % of their average, 3.4: 3.06, written with one decimal.
    assert!((limit.unwrap().value - 3.1).abs() < 1e-9);
    // Two whole rolls averaged to 37.5 get a whole minimum: 34, not 33.75.
    let whole = StatLine {
        rolls: vec![
            item::Roll {
                value: 30.0,
                range: None,
            },
            item::Roll {
                value: 45.0,
                range: None,
            },
        ],
        ..line
    };
    let limit = limit_of_line(&whole, None, Half::Minimum);
    assert!((limit.unwrap().value - 34.0).abs() < 1e-9);
}
