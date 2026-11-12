//! The rows of the modifiers of a rare, magic or normal item: their groups and their room, the
//! bars over the tiers, the tiers found from the values of an item copied from the in-game
//! trade, the lines in doubt, their origins and their limits.

use super::*;

use crate::filters::Half;
use crate::item::AffixTier;
use crate::roll_difference::CopiedLines;
use crate::tier_rolls::{TierRolls, TierSpan};
use crate::trade::stat_filters::LineStats;
use crate::ui::item_stats::LineFacts;
use crate::ui::item_stats::fixtures::{
    explicit_rows, limit, parse, part, read_again, row_texts, rows, rows_of, texts,
};
use crate::ui::item_stats::model::Limit;

#[test]
fn an_item_copied_from_the_in_game_trade_shows_its_tiers_found_and_a_note() {
    let tiara = read_again(parse(include_str!(
        "../../../item/test-items/en-owner-trade-rare-helmet-fractured.txt"
    )));

    let texts = texts(&tiara);
    let prefixes = texts
        .iter()
        .position(|text| text.starts_with("Prefixes"))
        .expect("its prefixes");
    assert_eq!(
        texts[prefixes..],
        [
            "Prefixes 3/3",
            // The fractured hybrid, its lines read together.
            "□ 41% increased Energy Shield ≥37 [0.97]",
            "□ +34 to maximum Mana ≥31 [0.85 T1 39–42 · 33–39 Fractured]",
            "□ 98% increased Energy Shield ≥88 [0.98 T1 92–100]",
            "□ +69 to maximum Energy Shield ≥62 [0.94 T1 61–73 Desecrated]",
            "Suffixes 3/3",
            // A prefix too, but the prefixes are full.
            "□ 18% increased Rarity of Items found ≥16 [1.00 T1 15–18]",
            "□ +43% to Fire Resistance ≥39 [0.95 T1 41–45]",
            "□ +40% to Cold Resistance ≥36 [0.87 T2 36–40]",
            "Trade copy note",
        ]
    );
}

#[test]
fn a_modifier_of_one_tier_only_shows_its_ranges_without_tier() {
    let texts_of = |text: &str| {
        let texts = texts(&parse(text));
        let prefixes = texts
            .iter()
            .position(|text| text.starts_with("Prefixes"))
            .expect("its prefixes");
        texts[prefixes + 1..].to_vec()
    };

    // A jewel: every modifier of one tier.
    let jewel = texts_of(
        "Item Class: Jewels\nRarity: Rare\nViper Glimmer\nRuby\n--------\n\
         Item Level: 43\n--------\n\
         { Prefix Modifier \"Brutish\" (Tier: 1) — Minion, Life }\n\
         Minions have 12(5-15)% increased maximum Life",
    );
    assert_eq!(
        jewel,
        ["□ Minions have 12% increased maximum Life ≥11 [0.70 5–15]"]
    );

    // The Genesis tree, written without tier, next to a modifier of several tiers.
    let ring = texts_of(
        "Item Class: Rings\nRarity: Rare\nGlyph Circle\nBreach Ring\n--------\n\
         Item Level: 77\n--------\n\
         { Prefix Modifier \"Reinforcing\" — Life, Minion }\n\
         Minions have 21(19-22)% increased maximum Life\n\
         { Prefix Modifier \"Aqua\" (Tier: 7) — Mana }\n\
         +76(65-79) to maximum Mana",
    );
    assert_eq!(
        ring,
        [
            "□ Minions have 21% increased maximum Life ≥19 [0.67 19–22]",
            "□ +76 to maximum Mana ≥68 [0.39 T7 65–79]",
        ]
    );

    // A fixed value of one tier only: nothing under it.
    let quiver = texts(&parse(
        "Item Class: Quivers\nRarity: Rare\nStorm Song\nFire Quiver\n\
         --------\nItem Level: 40\n--------\n\
         { Suffix Modifier \"of the Archer\" (Tier: 1) — Attack, Gem }\n\
         +1 to Level of all Projectile Skills",
    ));
    assert_eq!(
        quiver.last().map(String::as_str),
        Some("□ +1 to Level of all Projectile Skills")
    );

    // A hybrid of one tier only, a line without number and a fixed value, `50(50)%`: nothing
    // under its last line, as under any fixed value.
    let flask = parse(
        "Item Class: Life Flasks\nRarity: Magic\nSaturated Ultimate Life Flask\n\
         --------\nItem Level: 82\n--------\n\
         { Prefix Modifier \"Saturated\" (Tier: 1) — Life }\n\
         Instant Recovery\n50(50)% reduced Amount Recovered",
    );
    // The base of a magic item, from the names of the trade site.
    let base = Some("Ultimate Life Flask");
    let lines = CopiedLines::new(
        flask.language(),
        flask.modifiers(),
        &LineStats::default(),
        None,
    );
    let stats = LineStats::new(&flask, flask.modifiers(), None);
    let tiers = TierRolls::new(
        flask.language(),
        flask.modifiers(),
        base,
        flask.item_level(),
    );
    let flask = row_texts(crate::ui::item_stats::rows(
        &flask,
        LineFacts {
            copied_lines: &lines,
            line_stats: &stats,
            tier_rolls: &tiers,
        },
        base,
    ));
    assert_eq!(
        flask[flask.len() - 2..],
        ["■ Instant Recovery", "■ 50% reduced Amount Recovered"]
    );

    // Copied from the in-game trade: its tier found from its value.
    let jewel = texts(&read_again(parse(
        "Item Class: Jewels\nRarity: Rare\nViper Glimmer\nRuby\n--------\n\
         Item Level: 43\n--------\nMinions have 12% increased maximum Life",
    )));
    assert!(
        jewel.contains(&"□ Minions have 12% increased maximum Life ≥11 [0.70 5–15]".to_owned()),
        "{jewel:?}"
    );

    // The essences of a bow, written without tier, named after their ranges.
    let bow = texts(&parse(include_str!(
        "../../../item/test-items/en-rare-bow-fractured.txt"
    )));
    assert!(
        bow.iter().any(|text| text
            .starts_with("□ 20% chance to gain Onslaught on Killing Hits with this Weapon ≥")
            && text.ends_with(" [0.00 20–25 Essence]")),
        "{bow:?}"
    );
    // The `+3` of an older patch, which no table holds today (the bows take `+2`): nothing.
    assert!(
        bow.contains(&"□ +3 to Level of all Attack Skills ≥3".to_owned()),
        "{bow:?}"
    );
}

#[test]
fn a_modifier_of_an_essence_or_an_alloy_names_its_currency() {
    // The owner's gloves: the alloy written crafted, its currency in place of "Crafted".
    let gloves = texts(&parse(include_str!(
        "../../../item/test-items/fr-rare-gloves-alloy.txt"
    )));
    let alloy = "11% d'Augmentation de Vitesse d'Attaque tant qu'il vous manque de la \
                 Sauvegarde runique";
    assert!(
        gloves.contains(&format!("□ {alloy} ≥10 [0.20 10–15 Alloy]")),
        "{gloves:?}"
    );

    // Copied from the in-game trade, with an essence: both found in the tables.
    let trade = texts(&read_again(parse(
        "Item Class: Gloves\nRarity: Rare\nPandemonium Grip\nAdorned Wraps\n--------\n\
         Item Level: 80\n--------\n\
         13% increased Quantity of Gold Dropped by Slain Enemies\n\
         11% increased Attack Speed while missing Runic Ward (crafted)",
    )));
    assert!(
        trade.contains(
            &"□ 13% increased Quantity of Gold Dropped by Slain Enemies ≥12 [0.60 10–15 Essence]"
                .to_owned()
        ),
        "{trade:?}"
    );
    assert!(
        trade.iter().any(|text| text
            .starts_with("□ 11% increased Attack Speed while missing Runic Ward ≥")
            && text.ends_with(" [0.20 10–15 Alloy]")),
        "{trade:?}"
    );
}

#[test]
fn a_line_of_the_in_game_trade_that_can_be_a_prefix_or_a_suffix_shows_the_tier_of_each() {
    let tiara = include_str!("../../../item/test-items/en-owner-trade-rare-helmet-fractured.txt");
    // Without the desecrated `+69 to maximum Energy Shield`, a prefix is open.
    let open = read_again(parse(
        &tiara.replace("+69 to maximum Energy Shield (desecrated)\n", ""),
    ));

    let texts = texts(&open);
    let explicit = texts
        .iter()
        .position(|text| text == "Explicit")
        .expect("its explicit modifiers");
    assert_eq!(
        texts[explicit..],
        [
            "Explicit",
            // Without bar, the tier and range of each affix one under the other.
            "□ 18% increased Rarity of Items found ≥16 [T1 16–19 AffixPrefix / T1 15–18 AffixSuffix]",
            "Trade copy note",
        ]
    );
}

#[test]
fn a_line_that_can_be_a_modifier_of_one_tier_only_writes_no_tier_number_on_its_row() {
    // `+1` desecrated is the prefix of Amanamu, the only one, named, or the T2 of the suffix of
    // a cannon: a fixed value, without range under it.
    let cannon = parse(
        "Item Class: Crossbows\nRarity: Rare\nDoom Bolt\nTrarthan Cannon\n--------\n\
         Item Level: 82\n--------\nGrenade Skills have +1 Cooldown Use (desecrated)\n",
    );

    let texts = texts(&read_again(cannon));
    assert!(
        texts.contains(
            &"□ Grenade Skills have +1 Cooldown Use ≥1 [{Amanamu AffixPrefix} / {T2 AffixSuffix} / Desecrated]"
                .to_owned()
        ),
        "{texts:?}"
    );
}

#[test]
fn the_origin_of_a_line_in_doubt_comes_after_its_tiers() {
    let tiara = include_str!("../../../item/test-items/en-owner-trade-rare-helmet-fractured.txt");
    // A prefix open, and the rarity desecrated.
    let open = read_again(parse(
        &tiara
            .replace("+69 to maximum Energy Shield (desecrated)\n", "")
            .replace(
                "18% increased Rarity of Items found\n",
                "18% increased Rarity of Items found (desecrated)\n",
            ),
    ));

    // The origin after the rows of the tiers: `roll_line` draws it on a row of its own,
    // since the line has it whichever affix it is (checked on a screenshot).
    assert!(texts(&open).contains(
        &"□ 18% increased Rarity of Items found ≥16 [T1 16–19 AffixPrefix / T1 15–18 AffixSuffix / Desecrated]"
            .to_owned()
    ));
}

#[test]
fn a_line_that_can_be_a_prefix_is_one_once_the_suffixes_are_full() {
    let tiara = include_str!("../../../item/test-items/en-owner-trade-rare-helmet-fractured.txt");
    // A third resistance in place of the desecrated line: the suffixes full, a prefix open.
    let resistances = read_again(parse(&tiara.replace(
        "+69 to maximum Energy Shield (desecrated)",
        "+30% to Lightning Resistance",
    )));

    let texts = texts(&resistances);
    let rarity = texts
        .iter()
        .position(|text| text.contains("Rarity"))
        .expect("the rarity");
    let suffixes = texts
        .iter()
        .position(|text| text.starts_with("Suffixes"))
        .expect("its suffixes");
    assert!(rarity < suffixes, "a prefix: {texts:?}");
    assert!(texts[rarity].ends_with("T1 16–19]"), "{}", texts[rarity]);
}

#[test]
fn a_minion_modifier_of_the_genesis_tree_found_from_its_value_shows_no_tier_and_the_note() {
    // `Reinforcing`, the only line of a Breach Ring: the range of its tier, without its number,
    // which the game doesn't write.
    let ring = read_again(parse(
        "Item Class: Rings\nRarity: Rare\nGlyph Circle\nBreach Ring\n--------\n\
         Item Level: 77\n--------\nMinions have 21% increased maximum Life",
    ));

    let texts = texts(&ring);
    assert_eq!(
        texts[texts.len() - 3..],
        [
            "Prefixes 1/3",
            "□ Minions have 21% increased maximum Life ≥19 [0.67 19–22]",
            "Trade copy note",
        ]
    );
}

#[test]
fn a_fixed_value_shows_the_same_in_both_copies() {
    // `+2 to Level of all Minion Skills`: the ordinary T2 of 3 of an amulet, `of the Despot` of
    // the Genesis tree on a belt, whose tier the game doesn't write; the advanced copy writes no
    // range for a fixed value, the in-game trade none at all.
    let advanced = |class: &str, base: &str, header: &str| {
        format!(
            "Item Class: {class}\nRarity: Rare\nDoom Idol\n{base}\n--------\nItem Level: 79\n\
             --------\n{{ Suffix Modifier \"of the Despot\"{header} — Minion, Gem }}\n\
             +2 to Level of all Minion Skills"
        )
    };
    let in_game_trade = |text: &str| {
        let lines: Vec<&str> = text.lines().filter(|line| !line.starts_with('{')).collect();
        read_again(parse(&lines.join("\n")))
    };
    let last = |item: &Item| {
        texts(item)
            .into_iter()
            .rfind(|text| text.starts_with('■') || text.starts_with('□'))
    };

    // A tier without range: the bar over the three tiers, "T2", the field from 2.
    let amulet = advanced("Amulets", "Gold Amulet", " (Tier: 2)");
    let t2 = Some("□ +2 to Level of all Minion Skills ≥2 [0.50 T2]".to_owned());
    assert_eq!(last(&parse(&amulet)), t2);
    assert_eq!(last(&in_game_trade(&amulet)), t2);
    // Fixed in its tier, which the game doesn't write: no field, nothing under it.
    let belt = advanced("Belts", "Heavy Belt", "");
    let fixed = Some("□ +2 to Level of all Minion Skills".to_owned());
    assert_eq!(last(&parse(&belt)), fixed);
    assert_eq!(last(&in_game_trade(&belt)), fixed);
}

#[test]
fn the_ranges_written_under_a_modifier_are_those_of_its_numbers_that_vary() {
    // The ranges the advanced copy writes, on every line of the modifier (`Range::unless_fixed`):
    // a ring whose prefix adds a fixed and a varying number, then a flask whose prefix of two
    // lines has a fixed value only.
    let ranges = |text: &str| {
        parse(text)
            .modifiers()
            .iter()
            .find(|modifier| modifier.kind == ModifierKind::Explicit(Some(Affix::Prefix)))
            .map(tier_ranges)
            .unwrap_or_default()
    };
    let range = |min, max| Range { min, max };

    assert_eq!(
        ranges(
            "Item Class: Rings\nRarity: Rare\nGlyph Circle\nGold Ring\n--------\n\
             Item Level: 80\n--------\n{ Prefix Modifier \"Glinting\" (Tier: 6) — Damage }\n\
             Adds 5(5) to 10(8-12) Physical Damage to Attacks"
        ),
        [range(5.0, 5.0), range(8.0, 12.0)]
    );
    assert_eq!(
        ranges(
            "Item Class: Life Flasks\nRarity: Magic\nSaturated Ultimate Life Flask\n--------\n\
             Item Level: 82\n--------\n{ Prefix Modifier \"Saturated\" (Tier: 1) — Life }\n\
             Instant Recovery\n50(50)% reduced Amount Recovered"
        ),
        []
    );
}

#[test]
fn the_origin_of_a_modifier_follows_the_range_of_its_tier_under_its_last_line() {
    let armour = parse(include_str!(
        "../../../item/test-items/fr-rare-body-armour.txt"
    ));
    let origins: Vec<String> = texts(&armour)
        .into_iter()
        .filter(|text| text.ends_with("Crafted]") || text.ends_with("Desecrated]"))
        .collect();
    assert_eq!(origins.len(), 2, "{origins:?}");
    assert!(origins[0].contains("76% d'Augmentation du Bouclier d'énergie"));
    assert!(origins[1].contains("+14% aux Résistances au feu et au chaos"));

    let ring = parse(
        "Item Class: Rings\nRarity: Rare\nRune Loop\nGold Ring\n--------\n\
         Item Level: 79\n--------\n\
         { Fractured Prefix Modifier \"Hale\" (Tier: 9) — Life, Mana }\n\
         +12(10-14) to maximum Life\n+10(9-13) to maximum Mana",
    );
    let lines: Vec<String> = texts(&ring)
        .into_iter()
        .filter(|text| text.contains("to maximum"))
        .collect();
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert!(!lines[0].contains("Fractured"), "{lines:?}");
    assert!(lines[1].ends_with("Fractured]"), "{lines:?}");
}

#[test]
fn a_modifier_whose_header_is_unknown_is_explicit_without_tier() {
    let ring = parse(
        "Item Class: Rings\nRarity: Rare\nRune Loop\nPrismatic Ring\n--------\n\
         Item Level: 79\n--------\n{ Implicit Modifier }\n\
         +8(7-10)% to all Elemental Resistances\n--------\n{ Strange Modifier }\n\
         +143(120-150) to Evasion Rating\n\
         { Suffix Modifier \"of the Brute\" (Tier: 7) — Attribute }\n+12(9-12) to Strength\n\
         --------\nCorrupted",
    );

    assert_eq!(
        texts(&ring),
        [
            "Item",
            "■ Base: Prismatic Ring",
            "□ ItemLevel: 79 ≥79",
            // The totals in place of the lines they add up: each element of the
            // resistances counts.
            "Pseudo",
            // Of its implicit only, which every Prismatic Ring has.
            "□ ElementalResistance: 24 ≥22",
            "■ Strength: 12 ≥11",
            "Implicit",
            "□ +8% to all Elemental Resistances ≥7 [7–10 0.33]",
            "Suffixes 1/3",
            "□ +12 to Strength ≥11 [0.25 T7 9–12]",
            // Neither a prefix nor a suffix: no tier, like a modifier the tables don't
            // know.
            "Explicit",
            "□ +143 to Evasion Rating ≥129",
        ]
    );
}

#[test]
fn the_affixes_a_rune_or_an_implicit_allows_count_in_the_fill() {
    let english = parse(
        "Item Class: Amulets\nRarity: Rare\nDoom Idol\nGloam Amulet\n--------\n\
         Item Level: 79\n--------\n+1 Suffix Modifier allowed (rune)\n--------\n\
         { Implicit Modifier }\n-1 Prefix Modifier allowed\n--------\n\
         { Prefix Modifier \"Hale\" (Tier: 9) }\n+12(10-14) to maximum Life\n\
         { Suffix Modifier \"of the Brute\" (Tier: 7) — Attribute }\n+12(9-12) to Strength",
    );
    let french = parse(
        "Classe d'objet: Amulettes\nRareté: Rare\nIdole funeste\nAmulette crépusculaire\n\
         --------\nNiveau de l'objet: 79\n--------\n\
         +1 Modificateur suffixe autorisé (rune)\n--------\n\
         { Modificateur Implicite }\n-1 Modificateur préfixe autorisé\n--------\n\
         { Préfixe : « Sain » (Palier : 9) }\n+12(10-14) de Vie maximale\n\
         { Suffixe : « de la Brute » (Palier : 7) — Attribut }\n+12(9-12) de Force",
    );

    let groups = |item: &Item| -> Vec<String> {
        texts(item)
            .into_iter()
            .filter(|text| text.starts_with("Prefixes") || text.starts_with("Suffixes"))
            .collect()
    };
    assert_eq!(groups(&english), ["Prefixes 1/2", "Suffixes 1/4"]);
    assert_eq!(groups(&french), ["Prefixes 1/2", "Suffixes 1/4"]);
}

#[test]
fn a_fixed_implicit_of_a_rare_is_a_filter_without_bar_after_those_that_vary() {
    let boots = parse(
        "Item Class: Boots\nRarity: Rare\nGale Tread\nLattice Sandals\n--------\n\
         { Implicit Modifier }\n5(5-5)% increased Movement Speed\n\
         { Implicit Modifier }\n+12(10-15) to Dexterity\n--------\n\
         { Prefix Modifier \"Hale\" (Tier: 3) — Life }\n+86(85-99) to maximum Life\n\
         { Prefix Modifier \"Sturdy\" (Tier: 5) — Defences }\n\
         +20(15-25) to Evasion Rating\n+10(8-12)% increased Evasion Rating",
    );

    assert_eq!(
        texts(&boots),
        [
            "Item",
            "■ Base: Lattice Sandals",
            "Pseudo",
            "■ Life: 86 ≥77",
            // Of its implicit only, which comes with the base.
            "□ Dexterity: 12 ≥11",
            "Implicit",
            // 90 % of 12, within the range of the implicit.
            "□ +12 to Dexterity ≥11 [10–15 0.40]",
            // A fixed line keeps its checkbox, without a minimum.
            "□ 5% increased Movement Speed",
            "Prefixes 2/3",
            // Its bar over every tier of the life of boots, from 10 (T9) to 149 (T1).
            "□ +86 to maximum Life ≥77 [0.55 T3 85–99]",
            // A hybrid modifier: its tier and the ranges of its lines under the last one,
            // each line with its checkbox and its minimum; without bar, since no modifier
            // of the tables rolls these ranges.
            "□ +20 to Evasion Rating ≥18",
            "□ +10% increased Evasion Rating ≥9 [T5 15–25 · 8–12]",
        ]
    );
}

#[test]
fn a_prefix_or_suffix_rolls_over_every_tier_of_its_modifier_on_the_base() {
    let mail = parse(
        "Item Class: Body Armours\nRarity: Rare\nDoom Shell\nAncestral Mail\n--------\n\
         Item Level: 82\n--------\n\
         { Prefix Modifier \"Vigorous\" (Tier: 7) — Life }\n+86(85-99) to maximum Life\n\
         { Prefix Modifier \"Lacquered\" (Tier: 4) — Defences }\n\
         +98(86-102) to Armour\n+88(79-94) to Evasion Rating\n\
         { Suffix Modifier \"of the Penguin\" (Tier: 1) — Elemental, Cold, Resistance }\n\
         +41(41-45)% to Cold Resistance",
    );
    let rows = rows(&mail);
    let texts = row_texts(rows.iter().cloned());
    let prefixes = texts
        .iter()
        .position(|text| text == "Prefixes 2/3")
        .expect("its prefixes");

    assert_eq!(
        texts[prefixes..],
        [
            "Prefixes 2/3",
            // 76 of the 204 between 10 (T13) and 214 (T1); a pseudo total adds it up.
            "□ +86 to maximum Life ≥77 [0.37 T7 85–99]",
            // Each line of a hybrid with its bar over its own tiers.
            "□ +98 to Armour ≥88 [0.55]",
            "□ +88 to Evasion Rating ≥79 [0.53 T4 86–102 · 79–94]",
            "Suffixes 1/3",
            "□ +41% to Cold Resistance ≥37 [0.90 T1 41–45]",
        ]
    );
    let limit_of = |text: &str| {
        let row = rows
            .iter()
            .find(|row| matches!(row, Row::Stat(Stat { text: StatText::Line(line), .. }) if *line == text))
            .expect("its row");
        limit(row)
    };
    // The field stays within the rolls of every tier, which its tooltip tells.
    let life = Range {
        min: 10.0,
        max: 214.0,
    };
    assert_eq!(
        limit_of("+86 to maximum Life"),
        Limit {
            value: 77.0,
            half: Half::Minimum,
            bounds: Some(life),
            rolls: Some(life),
            tiers: Some(TierSpan {
                lowest: 13,
                highest: 1,
            }),
        }
    );
    assert_eq!(
        limit_of("+88 to Evasion Rating").tiers,
        Some(TierSpan {
            lowest: 8,
            highest: 1,
        })
    );
}

#[test]
fn a_line_fixed_in_its_tier_varies_over_the_tiers() {
    let bow = parse(
        "Item Class: Bows\nRarity: Rare\nStorm Bite\nGemini Bow\n--------\n\
         Item Level: 81\n--------\n\
         { Suffix Modifier \"of the Sniper\" (Tier: 2) — Gem }\n\
         +3 to Level of all Projectile Skills",
    );

    // A bar and a range field: the other tiers roll 1, 2 and 4.
    assert_eq!(
        texts(&bow).last().map(String::as_str),
        Some("□ +3 to Level of all Projectile Skills ≥3 [0.67 T2]")
    );
}

#[test]
fn a_jewel_has_two_prefixes_and_two_suffixes_at_most() {
    // A modifier of a jewel has one tier.
    let english = parse(
        "Item Class: Jewels\nRarity: Rare\nGlow Spark\nRuby\n--------\n\
         { Prefix Modifier \"Brutish\" (Tier: 1) — Minion, Life }\n\
         Minions have 12(5-15)% increased maximum Life",
    );
    let french = parse(
        "Classe d'objet: Joyaux\nRareté: Rare\nÉclat\nRubis\n--------\n\
         { Préfixe : « Brutal » (Palier : 1) — Créature, Vie }\n\
         Les Créatures ont 12(5-15)% d'Augmentation de Vie maximale",
    );

    assert!(english.is_jewel());
    assert_eq!(
        texts(&english),
        [
            "Item",
            "■ Base: Ruby",
            "Prefixes 1/2",
            "□ Minions have 12% increased maximum Life ≥11 [0.70 5–15]"
        ]
    );
    assert_eq!(
        texts(&french),
        [
            "Item",
            "■ Base: Rubis",
            "Prefixes 1/2",
            "□ Les Créatures ont 12% d'Augmentation de Vie maximale ≥11 [0.70 5–15]"
        ]
    );
}

#[test]
fn a_sanctified_item_keeps_its_values_out_of_their_tier() {
    for text in [
        include_str!("../../../item/test-items/en-owner-rare-sceptre-sanctified.txt"),
        include_str!("../../../item/test-items/fr-rare-sceptre-sanctified.txt"),
    ] {
        let rows = texts(&parse(text));
        let line = |end: &str| {
            rows.iter()
                .find(|row| row.ends_with(end))
                .unwrap_or_else(|| panic!("{end}: {rows:#?}"))
                .clone()
        };

        // `+74(80-89) to maximum Mana`: its tier, its value under its range.
        assert!(line(" ≥67 [0.42 T5 80–89]").starts_with("□ +74 "));
        // `29(30)% reduced Attribute Requirements`, a range of one value; unchecked, as
        // every stat of an item with an unrevealed modifier.
        assert!(line(" ≥26 [0.70 T2 30]").starts_with("□ 29% "));
    }
}

#[test]
fn a_sanctified_value_above_every_tier_keeps_its_field_and_a_full_bar() {
    let text = include_str!("../../../item/test-items/en-owner-rare-sceptre-sanctified.txt")
        .replace("+74(80-89)", "+250(80-89)");
    let item = parse(&text);

    assert!(
        texts(&item).contains(&"□ +250 to maximum Mana ≥225 [1.00 T5 80–89]".to_owned()),
        "{:#?}",
        texts(&item)
    );
    let limit = rows(&item)
        .into_iter()
        .find_map(|row| match row {
            Row::Stat(Stat {
                text: StatText::Line("+250 to maximum Mana"),
                filter,
                ..
            }) => filter?.limit,
            _ => None,
        })
        .expect("its limit");
    // The field holds the value, above the 164 of tier 1; the tooltip tells the rolls.
    assert_eq!(
        limit.bounds,
        Some(Range {
            min: 10.0,
            max: 250.0
        })
    );
    assert_eq!(
        limit.rolls,
        Some(Range {
            min: 10.0,
            max: 164.0
        })
    );
}

#[test]
fn a_magic_item_has_one_prefix_and_one_suffix_at_most() {
    let mace = parse(include_str!("../../../item/test-items/en-magic-mace.txt"));

    assert_eq!(
        texts(&mace),
        [
            // Its whole name here: the application finds its base type among the names of
            // the trade site.
            "Item",
            "■ Base: Crackling Temple Maul of the Brute",
            "□ ItemLevel: 32 ≥32",
            "Weapon",
            // The physical DPS of its base, which every maul passes.
            "□ PhysicalDps: 64 ≥57",
            "■ ElementalDps: 31 ≥27",
            "Pseudo",
            "■ Strength: 8 ≥7",
            "Prefixes 1/1",
            "□ Adds 1 to 50 Lightning Damage ≥23 [T7 1–4 · 46–66]",
            "Suffixes 1/1",
            "□ +8 to Strength ≥7 [T8 5–8]",
        ]
    );
}

#[test]
fn a_limit_stays_within_the_range_of_its_stat() {
    let ring = parse(
        "Item Class: Rings\nRarity: Unique\nVenom Loop\nAmethyst Ring\n--------\n\
         Item Level: 80\n--------\n{ Unique Modifier }\n\
         Adds 1 to 55(50-60) Lightning damage to Attacks",
    );
    let boots = parse(
        "Item Class: Boots\nRarity: Rare\nGale Tread\nLattice Sandals\n--------\n\
         { Implicit Modifier }\n+10(10-15) to Dexterity",
    );
    // The fixed number counts in the range of the average; the tooltip doesn't tell it,
    // since it isn't a roll.
    assert_eq!(
        limit(&rows(&ring)[1]),
        Limit {
            value: 28.0,
            half: Half::Minimum,
            bounds: Some(Range {
                min: 25.5,
                max: 30.5
            }),
            rolls: None,
            tiers: None,
        }
    );
    // 90 % of 10 is under the lowest roll of the implicit.
    let range = Some(Range {
        min: 10.0,
        max: 15.0,
    });
    assert_eq!(
        limit(rows(&boots).last().expect("its implicit")),
        Limit {
            value: 10.0,
            half: Half::Minimum,
            bounds: range,
            rolls: range,
            tiers: None,
        }
    );
}

#[test]
fn a_modifier_counts_the_affixes_it_takes_in_the_room_of_its_item() {
    let modifier = |kind, doubt| Modifier {
        kind,
        origin: None,
        name: None,
        tier: None,
        tags: Vec::new(),
        is_unrevealed: false,
        lines: Vec::new(),
        doubt,
    };
    let split = |whole, added| {
        Some(TierDoubt::Split {
            line: 0,
            whole: part(whole),
            added: part(added),
        })
    };
    let counts = |modifier: &Modifier| {
        (
            affix_count(modifier, Affix::Prefix),
            affix_count(modifier, Affix::Suffix),
        )
    };
    let tier = AffixTier {
        affix: Affix::Prefix,
        tier: 1,
        hides_number: false,
        ranges: Vec::new(),
        lord: None,
    };

    let prefix = ModifierKind::Explicit(Some(Affix::Prefix));
    let explicit = ModifierKind::Explicit(None);
    assert_eq!(counts(&modifier(prefix, None)), (1, 0));
    assert_eq!(
        counts(&modifier(prefix, split(Affix::Prefix, Affix::Prefix))),
        (2, 0)
    );
    assert_eq!(
        counts(&modifier(explicit, split(Affix::Prefix, Affix::Suffix))),
        (1, 1)
    );
    // A prefix or a suffix, and an affix unknown: not counted.
    let either = TierDoubt::EitherAffix {
        prefix: tier.clone(),
        suffix: AffixTier {
            affix: Affix::Suffix,
            ..tier
        },
    };
    assert_eq!(counts(&modifier(explicit, Some(either))), (0, 0));
    assert_eq!(counts(&modifier(explicit, None)), (0, 0));
    assert_eq!(counts(&modifier(ModifierKind::Implicit, None)), (0, 0));
    // An unrevealed prefix takes its room.
    let unrevealed = Modifier {
        is_unrevealed: true,
        ..modifier(prefix, None)
    };
    assert_eq!(counts(&unrevealed), (1, 0));
}

#[test]
fn a_waystone_shows_its_modifiers_over_their_ranges_in_both_copies() {
    // The owner's waystones of tier 12, in the band of tiers 11 to 15: every modifier of one
    // tier there, its bar and its field over the range of that tier, without tier; from the
    // in-game trade, the same, each line found a prefix or a suffix from its value.
    let english = include_str!("../../../item/test-items/en-owner-rare-waystone.txt");
    let french = include_str!("../../../item/test-items/fr-rare-waystone.txt");
    let in_game_trade = |text: &str| {
        let lines: Vec<&str> = text.lines().filter(|line| !line.starts_with('{')).collect();
        texts(&read_again(parse(&lines.join("\n"))))
    };
    let explicit = |texts: Vec<String>| -> Vec<String> {
        texts
            .into_iter()
            .skip_while(|text| !text.starts_with("Prefixes"))
            .collect()
    };
    // Unchecked: the buyer of a waystone pays for its properties, not for its monsters.
    let english_modifiers = [
        "Prefixes 2/3",
        "□ Monsters have 91% increased Stun Buildup ≥90 [0.10 90–100]",
        "□ 18% increased Monster Damage ≥16 [0.75 15–19]",
        "Suffixes 1/3",
        "□ Monster have 106% increased Elemental Ailment Application ≥100 [0.32 100–119]",
    ];
    let french_modifiers = [
        "Prefixes 2/3",
        "□ Les Monstres ont 13% d'Augmentation de la Vitesse dAttaque, d'incantation et de \
         déplacement ≥12 [0.60 10–15]",
        "□ 24% Davantage de Vie des Monstres ≥22 [0.80 20–25]",
        "Suffixes 2/3",
        "□ Les Monstres ont 101% d'Augmentation d'Application des Altérations Élémentaires ≥100 \
         [0.05 100–119]",
        // Written with a `+` the template of Exiled Exchange 2 doesn't have.
        "□ +33% à la Résistance élémentaire des Monstres ≥30 [0.75 30–34]",
    ];
    let with_note = |modifiers: &[&str]| {
        let mut texts: Vec<String> = modifiers.iter().map(|&text| text.to_owned()).collect();
        texts.push("Trade copy note".to_owned());
        texts
    };

    assert_eq!(explicit(texts(&parse(english))), english_modifiers);
    assert_eq!(
        explicit(in_game_trade(english)),
        with_note(&english_modifiers)
    );
    assert_eq!(explicit(texts(&parse(french))), french_modifiers);
    assert_eq!(
        explicit(in_game_trade(french)),
        with_note(&french_modifiers)
    );
}

#[test]
fn a_tablet_shows_its_modifiers_over_their_ranges() {
    // The owner's magic tablet, its base from the names of the trade site, in both languages.
    let english = parse(include_str!(
        "../../../item/test-items/en-owner-magic-tablet.txt"
    ));
    let french = parse(include_str!("../../../item/test-items/fr-magic-tablet.txt"));
    let last_two = |rows: Vec<String>| rows[rows.len() - 2..].to_vec();

    assert_eq!(
        last_two(row_texts(rows_of(&english, Some("Irradiated Tablet")))),
        [
            "Prefixes 1/1",
            "■ 30% increased Gold found in Map ≥27 [0.50 25–35]"
        ]
    );
    assert_eq!(
        last_two(row_texts(rows_of(&french, Some("Tablette irradiée")))),
        [
            "Prefixes 1/1",
            "■ 30% d'Augmentation de l'Or trouvé dans la Carte ≥27 [0.50 25–35]"
        ]
    );

    // A rare tablet, seen on the trade site on 2026-10-31, copied from the in-game trade: two
    // prefixes and two suffixes at most, each line found from its value.
    let rare = texts(&read_again(parse(
        "Item Class: Tablet\nRarity: Rare\nDread Sigil\nIrradiated Tablet\n--------\n\
         Item Level: 80\n--------\n\
         9% increased Rarity of Items found in Map\n15% increased Experience gain in Map\n\
         34% increased Quantity of Waystones found in Map\n\
         Map has 85% increased chance to contain Shrines",
    )));
    assert_eq!(
        explicit_rows(rare),
        [
            "Prefixes 2/2",
            "□ 9% increased Rarity of Items found in Map ≥8 [0.25 8–12]",
            "□ 15% increased Experience gain in Map ≥14 [0.50 12–18]",
            "Suffixes 2/2",
            "□ 34% increased Quantity of Waystones found in Map ≥31 [0.40 30–40]",
            "□ Map has 85% increased chance to contain Shrines ≥77 [0.50 70–100]",
            "Trade copy note",
        ]
    );
}

#[test]
fn a_french_line_finds_the_table_of_its_english_stat() {
    // French stats whose template isn't that of the English stat: copied from the in-game
    // trade, each line finds its table as in English.
    let tablet = texts(&read_again(parse(
        "Classe d'objet: Tablette\nRareté: Rare\nSceau sinistre\nTablette irradiée\n--------\n\
         Niveau de l'objet: 80\n--------\n\
         9% d'Augmentation de la Rareté des Objets trouvés dans la Carte",
    )));
    let waystone = texts(&read_again(parse(
        "Classe d'objet: Pierres de téléportation\nRareté: Rare\nChemin sinistre\n\
         Pierre de téléportation (Palier 15)\n--------\nNiveau de l'objet: 82\n--------\n\
         -8% aux Résistances maximales des joueurs",
    )));

    assert_eq!(
        explicit_rows(tablet),
        [
            "Prefixes 1/2",
            "□ 9% d'Augmentation de la Rareté des Objets trouvés dans la Carte ≥8 [0.25 8–12]",
            "Trade copy note",
        ]
    );
    // One French line of two English stats: the mana recovery of the flasks of a belt, a
    // prefix of six tiers, and of the jewels, a suffix of one tier.
    let belt = texts(&read_again(parse(
        "Classe d'objet: Ceintures\nRareté: Rare\nLien sinistre\nCeinture ample\n--------\n\
         Niveau de l'objet: 80\n--------\n\
         25% d'Augmentation de la Récupération de Mana des Flacons",
    )));
    assert_eq!(
        explicit_rows(belt),
        [
            "Prefixes 1/3",
            "□ 25% d'Augmentation de la Récupération de Mana des Flacons ≥23 [0.57 T3 23–28]",
            "Trade copy note",
        ]
    );
    let jewel = texts(&read_again(parse(
        "Classe d'objet: Joyaux\nRareté: Rare\nÉclat sinistre\nÉmeraude\n--------\n\
         Niveau de l'objet: 80\n--------\n\
         10% d'Augmentation de la Récupération de Mana des Flacons",
    )));
    assert_eq!(
        explicit_rows(jewel),
        [
            "Suffixes 1/2",
            "□ 10% d'Augmentation de la Récupération de Mana des Flacons ≥9 [0.50 5–15]",
            "Trade copy note",
        ]
    );
    assert_eq!(
        explicit_rows(waystone),
        [
            "Suffixes 1/3",
            "□ -8% aux Résistances maximales des joueurs ≥-8 [1.00 6–8]",
            "Trade copy note",
        ]
    );
}

#[test]
fn a_corrupted_waystone_shows_the_count_of_its_affixes_alone() {
    // Six suffixes and no prefix, as several corrupted waystones on the trade site on
    // 2026-10-31, copied from the in-game trade: the count alone, each line found a suffix
    // from its value, the lines without number after the others, without range.
    let corrupted = texts(&read_again(parse(
        "Item Class: Waystones\nRarity: Rare\nGrim Path\nWaystone (Tier 15)\n--------\n\
             Item Level: 82\n--------\n\
             Area has patches of Shocked Ground\nMonsters are Evasive\nMonsters are Armoured\n\
             47% less effect of Curses on Monsters\n\
             Players have 25% less Cooldown Recovery Rate\n\
         Players have 40% less Recovery Rate of Life and Energy Shield\n--------\nCorrupted",
    )));

    assert_eq!(
        explicit_rows(corrupted),
        [
            "Suffixes 6",
            "□ 47% less effect of Curses on Monsters ≥42 [0.70 40–50]",
            "□ Players have 25% less Cooldown Recovery Rate ≤28 [0.00 25–30]",
            "□ Players have 40% less Recovery Rate of Life and Energy Shield ≤40 [1.00 36–40]",
            "□ Area has patches of Shocked Ground",
            "□ Monsters are Evasive",
            "□ Monsters are Armoured",
            "Trade copy note",
        ]
    );
}

#[test]
fn the_minimum_of_a_decimal_stat_of_a_rare_fits_in_its_field() {
    // The owner's Demon Garb: 90 % of 12.2 written 10.98 overflowed the half of its field.
    let armour = texts(&parse(include_str!(
        "../../../item/test-items/fr-rare-body-armour-two-hybrids.txt"
    )));
    assert!(
        armour.contains(&"□ 12.2 de Vie Régénérée par seconde ≥11 [0.32 T6 9.1–13]".to_owned())
    );
}
