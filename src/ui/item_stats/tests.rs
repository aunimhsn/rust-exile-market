//! The stat rows of the item block, on the test items and on items built for a case: the
//! groups and their order, the filters at first and their range fields, the bars over the
//! tiers, the tiers found from the values of an item copied from the in-game trade, the lines
//! in doubt and the notes.

use super::*;
use crate::item::{CopiedItem, ItemState};

fn parse(text: &str) -> Item {
    Item::parse(&CopiedItem::from_text(text).expect("an item")).expect("read")
}

/// The rows of `item`, its lines read in English.
fn rows(item: &Item) -> Vec<Row<'_>> {
    let lines = CopiedLines::new(
        item.language(),
        item.modifiers(),
        &LineStats::default(),
        None,
    );
    let stats = LineStats::new(item.language(), item.modifiers(), None);
    let tiers = TierRolls::new(item.language(), item.modifiers(), base_type(item));
    super::rows(item, &lines, &stats, &tiers, base_type(item))
}

/// The base type the search of `item` looks for, among its filters: none for a unique.
fn base_type(item: &Item) -> Option<&str> {
    (item.rarity() != Rarity::Unique).then(|| item.base_type())
}

/// The rows of `item` as texts, as [`row_texts`] writes them.
fn texts(item: &Item) -> Vec<String> {
    row_texts(rows(item))
}

/// The rows as texts: a title (`Prefixes 2/3`), a note, or a stat,
/// first `■ ` for a checked filter, `□ ` for an unchecked one or `⊘ ` for a line the trade
/// site can't search, then its minimum or its maximum and its roll (`■ +55 to maximum Life
/// ≥55 [40–60 0.75]`, `■ 18% reduced Movement Speed ≤18`, a line of a prefix or suffix
/// with its bar over every tier and, under its last line, its tier, its ranges and its
/// origin: `■ +86 to maximum Life ≥77 [0.37 T7 85–99]`, `[0.80 T2 71–79 Desecrated]`; the
/// tiers of a line that can be a prefix or a suffix, each row after a ` / `, its origin a
/// row of its own: `[T1 16–19 AffixPrefix / T1 15–18 AffixSuffix / Desecrated]`; the word
/// after the ranges followed by its tooltip, and two neighbouring tiers `T1/T2 155–179`).
fn row_texts<'a>(rows: impl IntoIterator<Item = Row<'a>>) -> Vec<String> {
    rows.into_iter()
        .map(|row| match row {
            Row::Group(Group::Prefixes { filled, max }) => format!("Prefixes {filled}/{max}"),
            Row::Group(Group::Suffixes { filled, max }) => format!("Suffixes {filled}/{max}"),
            Row::Group(group) => format!("{group:?}"),
            Row::UnrevealedNote => "Unrevealed note".to_owned(),
            Row::TradeCopyNote => "Trade copy note".to_owned(),
            Row::Stat(Stat {
                text,
                filter,
                unsearchable,
                roll,
                ..
            }) => {
                let check = match filter {
                    _ if unsearchable => "⊘ ",
                    Some(StatFilter { checked: true, .. }) => "■ ",
                    Some(StatFilter { checked: false, .. }) => "□ ",
                    None => "",
                };
                let limit = filter
                    .and_then(|filter| filter.limit)
                    .map(|limit| {
                        let sign = match limit.half {
                            Half::Minimum => '≥',
                            Half::Maximum => '≤',
                        };
                        format!(" {sign}{}", format::number(limit.value))
                    })
                    .unwrap_or_default();
                let roll = match roll {
                    None => String::new(),
                    Some(Roll::Bar {
                        share,
                        ranges,
                        origin,
                    }) => {
                        let origin = origin
                            .map(|origin| format!(" {origin:?}"))
                            .unwrap_or_default();
                        format!(" [{} {share:.2}{origin}]", ranges_text(&ranges))
                    }
                    Some(Roll::Affix {
                        share,
                        tier,
                        origin,
                    }) => {
                        let share = share.map(|share| format!("{share:.2}"));
                        let tier = tier.as_deref().map(tier_row_text);
                        let origin = origin.map(|origin| format!("{origin:?}"));
                        let parts: Vec<String> =
                            [share, tier, origin].into_iter().flatten().collect();
                        format!(" [{}]", parts.join(" "))
                    }
                    Some(Roll::Tiers { rows, origin }) => {
                        // The origin on a line of its own, after the tiers.
                        let rows: Vec<String> = rows
                            .iter()
                            .map(tier_row_text)
                            .chain(origin.map(|origin| format!("{origin:?}")))
                            .collect();
                        format!(" [{}]", rows.join(" / "))
                    }
                };
                let text = match text {
                    StatText::Line(text) => text.to_owned(),
                    StatText::BaseType { base_type, .. } => format!("Base: {base_type}"),
                    StatText::Property(property, value) => {
                        format!("{property:?}: {}", format::number(value))
                    }
                    StatText::Pseudo(pseudo, value) => {
                        format!("{pseudo:?}: {}", format::number(value))
                    }
                    StatText::Unrevealed(affix) => format!("Unrevealed {affix:?}"),
                };
                format!("{check}{text}{limit}{roll}")
            }
        })
        .collect()
}

/// A tier under a line as [`row_texts`] writes it: `T1/T2 155–179`, then its word and its
/// tooltip, `AffixSuffix AddedUpWithPrefix`, `Hybrid (with +195 to Accuracy Rating)`, `Hybrid
/// (with 75–79 of 258% increased Physical Damage)`.
fn tier_row_text(row: &TierRow) -> String {
    let tier = tier_text(&row.tier);
    let tier = match row.better {
        Some(better) => format!("T{better}/{tier}"),
        None => tier,
    };
    let word = row.word.map(|word| format!("{word:?}"));
    let tip = row.tip.as_ref().map(|tip| match tip {
        WordTip::Text(message) => format!("{message:?}"),
        WordTip::HybridLines(lines) => format!("(with {})", lines.join(" and ")),
        WordTip::HybridPart { ranges, line } => {
            format!("(with {} of {line})", ranges_text(ranges))
        }
    });
    [word, tip]
        .into_iter()
        .flatten()
        .fold(tier, |text, part| format!("{text} {part}"))
}

/// A tier as [`row_texts`] writes it: `T1 15–18`.
fn tier_text(tier: &ModifierTier) -> String {
    [format!("T{}", tier.number), ranges_text(&tier.ranges)]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// `item`, copied from the in-game trade, read again with the tiers found from its values,
/// as the session does.
fn read_again(mut item: Item) -> Item {
    let stats = LineStats::new(item.language(), item.modifiers(), None);
    let modifiers = crate::game_data::with_trade_tiers(
        item.language(),
        item.base_type(),
        item.item_level(),
        item.modifiers(),
        stats.affix_limits(&item),
        item.has_state(crate::item::ItemState::Sanctified),
    );
    item.set_modifiers(modifiers);
    item
}

#[test]
fn an_item_copied_from_the_in_game_trade_shows_its_tiers_found_and_a_note() {
    let tiara = read_again(parse(include_str!(
        "../../item/test-items/en-owner-trade-rare-helmet-fractured.txt"
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
            "■ 41% increased Energy Shield ≥37 [0.97]",
            "□ +34 to maximum Mana ≥31 [0.85 T1 39–42 · 33–39 Fractured]",
            "■ 98% increased Energy Shield ≥88 [0.98 T1 92–100]",
            "■ +69 to maximum Energy Shield ≥62 [0.94 T1 61–73 Desecrated]",
            "Suffixes 3/3",
            // A prefix too, but the prefixes are full.
            "■ 18% increased Rarity of Items found ≥16 [1.00 T1 15–18]",
            "□ +43% to Fire Resistance ≥39 [0.95 T1 41–45]",
            "□ +40% to Cold Resistance ≥36 [0.87 T2 36–40]",
            "Trade copy note",
        ]
    );
}

#[test]
fn a_line_of_the_in_game_trade_that_can_be_a_prefix_or_a_suffix_shows_the_tier_of_each() {
    let tiara = include_str!("../../item/test-items/en-owner-trade-rare-helmet-fractured.txt");
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
            "■ 18% increased Rarity of Items found ≥16 [T1 16–19 AffixPrefix / T1 15–18 AffixSuffix]",
            "Trade copy note",
        ]
    );
}

#[test]
fn the_origin_of_a_line_in_doubt_comes_after_its_tiers() {
    let tiara = include_str!("../../item/test-items/en-owner-trade-rare-helmet-fractured.txt");
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
        &"■ 18% increased Rarity of Items found ≥16 [T1 16–19 AffixPrefix / T1 15–18 AffixSuffix / Desecrated]"
            .to_owned()
    ));
}

#[test]
fn a_line_that_can_be_a_prefix_is_one_once_the_suffixes_are_full() {
    let tiara = include_str!("../../item/test-items/en-owner-trade-rare-helmet-fractured.txt");
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
fn the_note_shows_also_when_the_only_tiers_found_are_those_of_a_line_in_doubt() {
    // A helmet with one modifier, which a prefix and a suffix both roll.
    let helmet = read_again(parse(
        "Item Class: Helmets\nRarity: Rare\nChimeric Visage\nAncestral Tiara\n--------\n\
         Item Level: 82\n--------\n18% increased Rarity of Items found",
    ));

    let texts = texts(&helmet);
    assert_eq!(
        texts[texts.len() - 3..],
        [
            "Explicit",
            "■ 18% increased Rarity of Items found ≥16 [T1 16–19 AffixPrefix / T1 15–18 AffixSuffix]",
            "Trade copy note",
        ]
    );
}

#[test]
fn only_the_tiers_found_from_the_values_have_the_note() {
    // Copied from the in-game trade, not read again yet: no tier found.
    let tiara = parse(include_str!(
        "../../item/test-items/en-owner-trade-rare-helmet-fractured.txt"
    ));
    assert!(!texts(&tiara).contains(&"Trade copy note".to_owned()));
    for text in [
        include_str!("../../item/test-items/en-rare-crossbow-all-modifiers.txt"),
        include_str!("../../item/test-items/en-owner-trade-unique-belt-mutated.txt"),
        include_str!("../../item/test-items/en-normal-helmet.txt"),
    ] {
        let item = parse(text);

        assert!(
            !texts(&item).contains(&"Trade copy note".to_owned()),
            "{text}"
        );
    }
}

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
        "../../item/test-items/fr-unique-belt-vaal.txt"
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
fn the_line_of_a_unique_without_header_never_varies() {
    let ring = parse(include_str!(
        "../../item/test-items/fr-unique-ring-mirrored.txt"
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
    let focus = parse(include_str!("../../item/test-items/en-unique-focus.txt"));

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
fn a_rare_shows_its_enchantment_runes_implicit_prefixes_and_suffixes() {
    let crossbow = parse(include_str!(
        "../../item/test-items/en-rare-crossbow-all-modifiers.txt"
    ));

    assert_eq!(
        texts(&crossbow),
        [
            // The base type checked, the item level unchecked at the level of the item.
            "Item",
            "■ Base: Gemini Crossbow",
            "□ ItemLevel: 80 ≥80",
            // Corrupted: no socket can be added any more.
            "■ Sockets: 2 ≥2",
            // 90 % of the DPS, the elemental one unchecked.
            "Weapon",
            "■ PhysicalDps: 288 ≥259",
            "□ ElementalDps: 267 ≥240",
            "Enchant",
            // The enchantment and the runes unchecked, with their value.
            "□ 45% increased Elemental Damage with Attacks ≥45",
            "Runes",
            "□ 18% increased Physical Damage ≥18",
            "□ Gain 30 Mana per enemy killed ≥30",
            "Implicit",
            // A line without number: no minimum.
            "■ Loads an additional bolt",
            "Prefixes 3/3",
            // 90 % of the copied value, of the average of two numbers; the damage and
            // the attack speed of the weapon unchecked, since its DPS counts them. The bar
            // over every tier of the crossbow: of two numbers, the average of where each
            // falls (10 in 1–19, 273 in 7–358). Its origin after the range of its tier.
            "□ Adds 10 to 273 Lightning Damage ≥127 [0.63 T2 1–16 · 239–300 Fractured]",
            "□ Adds 24 to 51 Physical Damage ≥34 [0.47 T3 23–35 · 39–59]",
            // Not a damage of the weapon: its attacks get it from any item.
            "■ 109% increased Elemental Damage with Attacks ≥98 [0.71 T2 100–119]",
            "Suffixes 3/3",
            // Copied before a tier was added above it: tier 1 then, 7.59 in 5–8.9 now.
            "■ Leeches 7.59% of Physical Damage as Mana ≥6.83 [0.66 T1 7–7.9]",
            "■ Gain 21 Mana per enemy killed ≥19 [0.44 T3 21–27 Desecrated]",
            "□ 18% increased Attack Speed ≥16 [0.93 T1 17–19]",
        ]
    );
}

#[test]
fn the_origin_of_a_modifier_follows_the_range_of_its_tier_under_its_last_line() {
    let armour = parse(include_str!(
        "../../item/test-items/fr-rare-body-armour.txt"
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
            "■ ElementalResistance: 24 ≥22",
            "■ Strength: 12 ≥11",
            "Implicit",
            "□ +8% to all Elemental Resistances ≥7 [7–10 0.33]",
            "Suffixes 1/3",
            "□ +12 to Strength ≥11 [0.25 T7 9–12]",
            // Neither a prefix nor a suffix: no tier, like a modifier the tables don't
            // know.
            "Explicit",
            "■ +143 to Evasion Rating ≥129",
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
            "■ Dexterity: 12 ≥11",
            "Implicit",
            // 90 % of 12, within the range of the implicit.
            "□ +12 to Dexterity ≥11 [10–15 0.40]",
            // A fixed line keeps its checkbox, without a minimum.
            "■ 5% increased Movement Speed",
            "Prefixes 2/3",
            // Its bar over every tier of the life of boots, from 10 (T9) to 149 (T1).
            "□ +86 to maximum Life ≥77 [0.55 T3 85–99]",
            // A hybrid modifier: its tier and the ranges of its lines under the last one,
            // each line with its checkbox and its minimum; without bar, since no modifier
            // of the tables rolls these ranges.
            "■ +20 to Evasion Rating ≥18",
            "■ +10% increased Evasion Rating ≥9 [T5 15–25 · 8–12]",
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
            "■ +98 to Armour ≥88 [0.55]",
            "■ +88 to Evasion Rating ≥79 [0.53 T4 86–102 · 79–94]",
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
        Some("■ +3 to Level of all Projectile Skills ≥3 [0.67 T2]")
    );
}

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
            // Not added up by a total.
            "■ +60 to Evasion Rating ≥54 [0.23 T6 52–79]",
            "Suffixes 3/3",
            "□ +38% to Fire Resistance ≥34 [0.82 T2 36–40]",
            "□ +23% to Lightning Resistance ≥21 [0.44 T5 21–25]",
            "□ +12 to Dexterity ≥11 [0.25 T7 9–12]",
        ]
    );
}

#[test]
fn a_unique_has_no_pseudo_totals() {
    let unique = parse(
        "Item Class: Rings\nRarity: Unique\nTest Loop\nGold Ring\n--------\n\
         Item Level: 80\n--------\n{ Unique Modifier }\n+52(40-60) to maximum Life\n\
         { Unique Modifier }\n+30(25-35)% to Fire Resistance",
    );

    assert_eq!(
        texts(&unique),
        [
            "Unique",
            "■ +52 to maximum Life ≥52 [40–60 0.60]",
            "■ +30% to Fire Resistance ≥30 [25–35 0.50]",
        ]
    );
}

#[test]
fn a_jewel_has_two_prefixes_and_two_suffixes_at_most() {
    let english = parse(
        "Item Class: Jewels\nRarity: Rare\nGlow Spark\nRuby\n--------\n\
         { Prefix Modifier \"Hale\" (Tier: 2) — Life }\n+12(10-14) to maximum Life",
    );
    let french = parse(
        "Classe d'objet: Joyaux\nRareté: Rare\nÉclat\nRubis\n--------\n\
         { Préfixe : « Sain » (Palier : 2) — Vie }\n+12(10-14) de Vie maximale",
    );

    assert!(english.is_jewel());
    assert_eq!(
        texts(&english),
        [
            "Item",
            "■ Base: Ruby",
            "Pseudo",
            "■ Life: 12 ≥11",
            "Prefixes 1/2",
            "□ +12 to maximum Life ≥11 [T2 10–14]"
        ]
    );
    assert_eq!(
        texts(&french),
        [
            "Item",
            "■ Base: Rubis",
            "Pseudo",
            "■ Life: 12 ≥11",
            "Prefixes 1/2",
            "□ +12 de Vie maximale ≥11 [T2 10–14]"
        ]
    );
}

#[test]
fn a_weapon_mostly_elemental_is_searched_by_its_elemental_dps_too() {
    let weapon = |physical: &str, lightning: &str| {
        parse(&format!(
            "Item Class: Two Hand Maces\nRarity: Rare\nStorm Fall\nTemple Maul\n--------\n\
             Physical Damage: {physical}\nLightning Damage: {lightning} (lightning)\n\
             Attacks per Second: 1.00\n--------\nItem Level: 80"
        ))
    };
    let dps = |item: &Item| texts(item)[4..6].to_vec();

    assert_eq!(
        dps(&weapon("10-20", "1-59")),
        ["■ PhysicalDps: 15 ≥13", "■ ElementalDps: 30 ≥27"]
    );
    // 90 % of a small DPS, of 2.5 and not of the 3 shown, rounded down: the weapon
    // passes its own filter.
    assert_eq!(
        dps(&weapon("10-20", "1-4")),
        ["■ PhysicalDps: 15 ≥13", "□ ElementalDps: 3 ≥2"]
    );
}

#[test]
fn an_unrevealed_modifier_is_a_line_of_its_group_and_unchecks_the_other_stats() {
    let english = parse(include_str!(
        "../../item/test-items/en-owner-rare-focus-unrevealed.txt"
    ));
    let french = parse(include_str!(
        "../../item/test-items/fr-rare-focus-unrevealed.txt"
    ));
    assert_eq!(
        texts(&english),
        [
            "Item",
            "■ Base: Plumed Focus",
            "□ ItemLevel: 39 ≥39",
            "□ RequiredLevel: 36 ≤36",
            // Searched with its number as minimum.
            "■ UnrevealedModifiers: 1 ≥1",
            "Pseudo",
            "□ ElementalResistance: 42 ≥38",
            // Counted in the fill, last of its group, without filter.
            "Prefixes 3/3",
            "□ 52% increased Spell Damage ≥47 [0.42 T4 45–54]",
            "□ +36 to maximum Energy Shield ≥32 [0.32 T6 36–41]",
            "Unrevealed Prefix",
            "Suffixes 2/3",
            "□ +24% to Cold Resistance ≥22 [0.46 T5 21–25]",
            "□ +18% to Fire Resistance ≥16 [0.31 T6 16–20]",
            "Unrevealed note",
        ]
    );
    let french = texts(&french);
    assert_eq!(french[4], "■ UnrevealedModifiers: 1 ≥1");
    assert_eq!(french[10], "Unrevealed Prefix");
    assert_eq!(french.last().map(String::as_str), Some("Unrevealed note"));
}

#[test]
fn a_corrupted_item_counts_each_unrevealed_modifier() {
    let ring = parse(
        "Item Class: Rings\nRarity: Rare\nWoe Loop\nRuby Ring\n--------\n\
         Item Level: 80\n--------\n\
         { Prefix Modifier \"Veiled\" }\nDesecrated Prefix\n\
         { Suffix Modifier \"of the Veil\" }\nDesecrated Suffix\n--------\nCorrupted",
    );
    assert_eq!(
        texts(&ring),
        [
            "Item",
            "■ Base: Ruby Ring",
            "□ ItemLevel: 80 ≥80",
            "■ UnrevealedModifiers: 2 ≥2",
            "Prefixes 1/3",
            "Unrevealed Prefix",
            "Suffixes 1/3",
            "Unrevealed Suffix",
            "Unrevealed note",
        ]
    );
}

#[test]
fn a_weapon_with_an_unrevealed_modifier_starts_with_its_dps_unchecked() {
    let weapon = parse(
        "Item Class: Two Hand Maces\nRarity: Rare\nStorm Fall\nTemple Maul\n--------\n\
         Physical Damage: 10-20\nLightning Damage: 1-59 (lightning)\n\
         Attacks per Second: 1.00\n--------\nItem Level: 80\n--------\n\
         { Prefix Modifier \"Veiled\" }\nDesecrated Prefix",
    );
    assert_eq!(
        texts(&weapon),
        [
            "Item",
            "■ Base: Temple Maul",
            "□ ItemLevel: 80 ≥80",
            "■ UnrevealedModifiers: 1 ≥1",
            "Weapon",
            // An unrevealed prefix can add damage.
            "□ PhysicalDps: 15 ≥13",
            "□ ElementalDps: 30 ≥27",
            "Prefixes 1/3",
            "Unrevealed Prefix",
            "Unrevealed note",
        ]
    );
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

/// The rows of the Item group of `item`, without its title.
fn item_group(item: &Item) -> Vec<String> {
    item_group_of(item, base_type(item))
}

/// The rows of the Item group of `item` searched by `base_type`, without its title: the
/// base type of a magic item is found among the names of the trade site.
fn item_group_of<'a>(item: &'a Item, base_type: Option<&'a str>) -> Vec<String> {
    let lines = CopiedLines::default();
    let stats = LineStats::new(item.language(), item.modifiers(), None);
    let rows = super::rows(item, &lines, &stats, &TierRolls::default(), base_type);
    row_texts(
        rows.into_iter()
            .skip_while(|row| *row != Row::Group(Group::Item))
            .skip(1)
            .take_while(|row| matches!(row, Row::Stat(_))),
    )
}

#[test]
fn a_normal_item_is_searched_by_its_base_sockets_item_level_and_quality() {
    let shield = parse(include_str!(
        "../../item/test-items/fr-normal-exceptional-shield.txt"
    ));
    let helmet = parse(include_str!("../../item/test-items/en-normal-helmet.txt"));
    let targe = parse(
        "Item Class: Shields\nRarity: Normal\nExceptional Mammoth Targe\n--------\n\
         Quality: +24% (augmented)\nArmour: 139 (augmented)\n--------\nItem Level: 82",
    );

    // An exceptional base by its sockets: the trade site doesn't know its name.
    assert_eq!(
        item_group(&shield),
        [
            "■ Base: Targe de mammouth",
            "■ ItemLevel: 82 ≥82",
            "■ Sockets: 2 ≥2",
        ]
    );
    assert_eq!(
        item_group(&helmet),
        [
            "■ Base: Divine Crown",
            "■ ItemLevel: 81 ≥81",
            "□ Quality: 9 ≥9",
        ]
    );
    // An exceptional base by its quality.
    assert_eq!(
        item_group(&targe),
        [
            "■ Base: Mammoth Targe",
            "■ ItemLevel: 82 ≥82",
            "■ Quality: 24 ≥24",
        ]
    );
}

#[test]
fn the_base_type_tells_the_item_class_searched_when_it_is_unchecked() {
    // The class under the base type `base_type` of `item`, when it is unchecked.
    let class_of = |item: &Item, base_type: &str| {
        let lines = CopiedLines::default();
        let stats = LineStats::new(item.language(), item.modifiers(), None);
        let tiers = TierRolls::default();
        super::rows(item, &lines, &stats, &tiers, Some(base_type))
            .into_iter()
            .find_map(|row| match row {
                Row::Stat(Stat {
                    text: StatText::BaseType { class, .. },
                    ..
                }) => Some(class.map(str::to_owned)),
                _ => None,
            })
    };

    // The item class of the copied text, in its language.
    let armour = parse(include_str!(
        "../../item/test-items/fr-rare-body-armour.txt"
    ));
    assert_eq!(
        class_of(&armour, armour.base_type()),
        Some(Some("Armures".to_owned()))
    );
    // A magic item, by the base type its name holds.
    let mace = parse(include_str!("../../item/test-items/en-magic-mace.txt"));
    assert_eq!(
        class_of(&mace, "Temple Maul"),
        Some(Some("Two Hand Maces".to_owned()))
    );
    // A base the game data doesn't know is searched among any base instead.
    let ring = parse("Item Class: Rings\nRarity: Rare\nWoe Loop\nMystery Ring");
    assert_eq!(class_of(&ring, ring.base_type()), Some(None));
}

/// The filter of `property` among the filters of `item`.
fn property_filter_of(item: &Item, property: Property) -> StatFilter {
    rows(item)
        .into_iter()
        .find_map(|row| match row {
            Row::Stat(Stat {
                key: FilterKey::Property(key),
                filter: Some(filter),
                ..
            }) if key == property => Some(filter),
            _ => None,
        })
        .expect("the filter of the property")
}

/// The value the player types in `half` of the field of `property`, as it is when they
/// leave it.
fn settled(item: &Item, property: Property, half: Half, typed: &str) -> String {
    let mut choice = property_filter_of(item, property).default_choice();
    *choice.text_mut(half) = typed.to_owned();
    choice.settle(half);
    choice.text(half).to_owned()
}

#[test]
fn the_fields_of_the_properties_stay_within_the_values_of_the_game() {
    let crossbow = parse(include_str!(
        "../../item/test-items/en-rare-crossbow-all-modifiers.txt"
    ));
    let leveling = parse(
        "Item Class: Body Armours\nRarity: Rare\nHate Shell\nWaxed Jacket\n--------\n\
         Quality: +23% (augmented)\nEvasion Rating: 120\n--------\n\
         Requires: Level 40, 30 Dex\n--------\nItem Level: 45",
    );

    assert_eq!(
        settled(&crossbow, Property::ItemLevel, Half::Minimum, "5000"),
        "100"
    );
    assert_eq!(
        settled(&crossbow, Property::ItemLevel, Half::Minimum, "0"),
        "1"
    );
    assert_eq!(
        settled(&crossbow, Property::Sockets, Half::Minimum, "9"),
        "6"
    );
    assert_eq!(
        settled(&leveling, Property::Quality, Half::Minimum, "80"),
        "75"
    );
    assert_eq!(
        settled(&leveling, Property::RequiredLevel, Half::Maximum, "95"),
        "90"
    );
}

#[test]
fn a_property_out_of_the_values_of_the_game_widens_its_field() {
    let item = parse(
        "Item Class: Rings\nRarity: Rare\nStorm Loop\nGold Ring\n--------\n\
         Item Level: 101",
    );

    // Its own value stays; a higher one is brought back to it.
    assert_eq!(
        settled(&item, Property::ItemLevel, Half::Minimum, "101"),
        "101"
    );
    assert_eq!(
        settled(&item, Property::ItemLevel, Half::Minimum, "5000"),
        "101"
    );
}

#[test]
fn the_sockets_are_checked_above_those_of_the_base_or_on_a_corrupted_item() {
    let armour = |sockets: &str, corrupted: &str| {
        parse(&format!(
            "Item Class: Body Armours\nRarity: Rare\nHate Shell\nWaxed Jacket\n--------\n\
             Sockets: {sockets}\n--------\nItem Level: 80\n--------\n\
             {{ Prefix Modifier }}\n+80 to maximum Life{corrupted}"
        ))
    };
    let sockets = |item: &Item| {
        item_group(item)
            .into_iter()
            .find(|text| text.contains("Sockets"))
    };

    // A body armour usually has two.
    assert_eq!(
        sockets(&armour("S S", "")).as_deref(),
        Some("□ Sockets: 2 ≥2")
    );
    assert_eq!(
        sockets(&armour("S S S", "")).as_deref(),
        Some("■ Sockets: 3 ≥3")
    );
    assert_eq!(
        sockets(&armour("S", "\n--------\nCorrupted")).as_deref(),
        Some("■ Sockets: 1 ≥1")
    );
    let none = parse(
        "Item Class: Body Armours\nRarity: Rare\nHate Shell\nWaxed Jacket\n--------\n\
         Item Level: 80\n--------\n{ Prefix Modifier }\n+80 to maximum Life",
    );
    assert_eq!(sockets(&none), None);
}

#[test]
fn a_sanctified_item_keeps_its_values_out_of_their_tier() {
    for text in [
        include_str!("../../item/test-items/en-owner-rare-sceptre-sanctified.txt"),
        include_str!("../../item/test-items/fr-rare-sceptre-sanctified.txt"),
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
    let text = include_str!("../../item/test-items/en-owner-rare-sceptre-sanctified.txt")
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
fn an_exceptional_quality_is_checked_except_on_a_rare() {
    let bow = parse(include_str!(
        "../../item/test-items/en-rare-bow-fractured.txt"
    ));
    let magic = parse(
        "Item Class: Gloves\nRarity: Magic\nVaal Mitts of the Fox\n--------\n\
         Quality: +23% (augmented)\n--------\nItem Level: 70\n--------\n\
         { Suffix Modifier }\n+20 to Dexterity",
    );
    let twenty = parse(
        "Item Class: Gloves\nRarity: Magic\nVaal Mitts of the Fox\n--------\n\
         Quality: +20% (augmented)\n--------\nItem Level: 70\n--------\n\
         { Suffix Modifier }\n+20 to Dexterity",
    );

    // Its bow usually has two sockets; its required level is above 75.
    assert_eq!(
        item_group(&bow),
        [
            "■ Base: Obliterator Bow",
            "□ ItemLevel: 81 ≥81",
            "□ Sockets: 2 ≥2",
            "□ Quality: 25 ≥25",
        ]
    );
    assert!(item_group(&magic).contains(&"■ Quality: 23 ≥23".to_owned()));
    assert!(
        !item_group(&twenty)
            .iter()
            .any(|text| text.contains("Quality"))
    );
}

#[test]
fn a_rare_for_leveling_has_its_required_level_as_a_maximum() {
    let ring = |required: u32, item_level: u32| {
        parse(&format!(
            "Item Class: Rings\nRarity: Rare\nHate Loop\nRuby Ring\n--------\n\
             Requires: Level {required}\n--------\nItem Level: {item_level}\n--------\n\
             {{ Prefix Modifier }}\n+30 to maximum Life"
        ))
    };
    let required = |item: &Item| {
        item_group(item)
            .into_iter()
            .find(|text| text.contains("RequiredLevel"))
    };

    assert_eq!(
        required(&ring(40, 45)).as_deref(),
        Some("□ RequiredLevel: 40 ≤40")
    );
    assert_eq!(
        required(&ring(75, 75)).as_deref(),
        Some("□ RequiredLevel: 75 ≤75")
    );
    assert_eq!(required(&ring(40, 80)), None);
    assert_eq!(required(&ring(78, 80)), None);
    // A magic item, even for leveling.
    let magic = parse(
        "Item Class: Rings\nRarity: Magic\nVital Ruby Ring\n--------\n\
         Requires: Level 40\n--------\nItem Level: 45\n--------\n\
         { Prefix Modifier }\n+30 to maximum Life",
    );
    assert_eq!(required(&magic), None);
}

#[test]
fn the_sockets_of_a_base_the_data_doesnt_know_are_checked_only_on_a_corrupted_item() {
    let armour = |corrupted: &str| {
        parse(&format!(
            "Item Class: Body Armours\nRarity: Rare\nHate Shell\nPatch Jacket\n--------\n\
             Sockets: S S S\n--------\nItem Level: 80\n--------\n\
             {{ Prefix Modifier }}\n+80 to maximum Life{corrupted}"
        ))
    };
    let sockets = |item: &Item| {
        item_group(item)
            .into_iter()
            .find(|text| text.contains("Sockets"))
    };

    assert_eq!(sockets(&armour("")).as_deref(), Some("□ Sockets: 3 ≥3"));
    assert_eq!(
        sockets(&armour("\n--------\nCorrupted")).as_deref(),
        Some("■ Sockets: 3 ≥3")
    );
}

#[test]
fn the_quality_of_a_charm_counts_from_10_and_of_a_flask_from_20() {
    let quality = |class: &str, name: &str, base: &str, quality: u32| {
        let item = parse(&format!(
            "Item Class: {class}\nRarity: Magic\n{name}\n--------\n\
             Quality: +{quality}% (augmented)\n--------\nItem Level: 60\n--------\n\
             {{ Suffix Modifier }}\n+10% to Fire Resistance"
        ));
        item_group_of(&item, Some(base))
            .into_iter()
            .find(|text| text.contains("Quality"))
    };
    let charm = |value| quality("Charms", "Ruby Charm of the Fox", "Ruby Charm", value);
    let flask = |value| {
        quality(
            "Life Flasks",
            "Ultimate Life Flask of the Fox",
            "Ultimate Life Flask",
            value,
        )
    };

    assert_eq!(charm(5).as_deref(), Some("□ Quality: 5 ≥5"));
    assert_eq!(charm(10).as_deref(), Some("■ Quality: 10 ≥10"));
    assert_eq!(flask(15), None);
    assert_eq!(flask(20).as_deref(), Some("□ Quality: 20 ≥20"));
    assert_eq!(flask(23).as_deref(), Some("■ Quality: 23 ≥23"));
}

#[test]
fn a_magic_item_has_one_prefix_and_one_suffix_at_most() {
    let mace = parse(include_str!("../../item/test-items/en-magic-mace.txt"));

    assert_eq!(
        texts(&mace),
        [
            // Its whole name here: the application finds its base type among the names of
            // the trade site.
            "Item",
            "■ Base: Crackling Temple Maul of the Brute",
            "□ ItemLevel: 32 ≥32",
            "Weapon",
            "■ PhysicalDps: 64 ≥57",
            "□ ElementalDps: 31 ≥27",
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

/// The limit of the filter of `row`.
fn limit(row: &Row<'_>) -> Limit {
    match row {
        Row::Stat(Stat {
            filter: Some(StatFilter {
                limit: Some(limit), ..
            }),
            ..
        }) => *limit,
        row => panic!("{row:?}"),
    }
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
            "■ 12% increased Charm Charges used ≤13 [10–15 0.40]",
            "Suffixes 1/3",
            "■ 18% reduced Movement Speed ≤20 [T2 15–25]",
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
fn a_rare_limit_leaves_10_percent_on_the_worse_side_also_for_a_negative_value() {
    let whole = |value, half| comparable(value, true, half);

    assert!((whole(12.0, Half::Minimum) - 11.0).abs() < 1e-9);
    assert!((whole(18.0, Half::Maximum) - 20.0).abs() < 1e-9);
    // The item passes its own filter.
    assert!((whole(-15.0, Half::Minimum) - -17.0).abs() < 1e-9);
    assert!((whole(-15.0, Half::Maximum) - -14.0).abs() < 1e-9);
    assert!((comparable(-7.5, false, Half::Minimum) - -8.25).abs() < 1e-9);
}

#[test]
fn the_tooltip_of_a_range_field_tells_its_rolls_and_its_better_side() {
    let i18n = Translator::new(crate::language::Language::English);
    let range = Some(Range {
        min: 15.0,
        max: 25.0,
    });
    let tooltip = |half, rolls, tiers| {
        let limit = Limit {
            value: 18.0,
            half,
            bounds: range,
            rolls,
            tiers,
        };
        tooltip_lines(limit, &i18n)
    };
    let rolls = TooltipLine::new("Rolls 15 to 25".to_owned(), color::TEXT);
    let lower = TooltipLine::new("Lower is better".to_owned(), color::WEAK);

    assert_eq!(
        tooltip(Half::Minimum, range, None),
        std::slice::from_ref(&rolls)
    );
    assert_eq!(
        tooltip(Half::Maximum, range, None),
        [rolls.clone(), lower.clone()]
    );
    // A line of several numbers.
    assert_eq!(
        tooltip(Half::Maximum, None, None),
        std::slice::from_ref(&lower)
    );
    assert_eq!(tooltip(Half::Minimum, None, None), []);
    // The required level of a rare for leveling, searched with a maximum: lower is better.
    assert_eq!(
        tooltip_lines(Limit::maximum(40.0), &i18n),
        std::slice::from_ref(&lower)
    );
    // A prefix or suffix: the tiers of its rolls right under them, small and faint.
    let tiers = TooltipLine {
        text: "T5–T1".to_owned(),
        tint: color::FAINT,
        small: true,
    };
    assert_eq!(
        tooltip(
            Half::Maximum,
            range,
            Some(TierSpan {
                lowest: 5,
                highest: 1,
            })
        ),
        [rolls, tiers, lower]
    );
}

#[test]
fn a_range_field_holds_four_characters_a_side_when_its_stat_reaches_them() {
    let line = StatLine::parse("+2250 Armour if you've Blocked Recently");
    let wide = |value, min, max| {
        Limit::new(value, Half::Minimum, Some(Range { min, max }), value, &line).wide()
    };

    assert!(!wide(245.0, 200.0, 320.0));
    assert!(!wide(-15.0, -20.0, -10.0));
    assert!(wide(2250.0, 1500.0, 3000.0));
    // A decimal point or a minus sign counts.
    assert!(wide(16.3, 15.0, 20.0));
    assert!(wide(-20.5, -25.0, -10.0));
    // Without a range, e.g. a rare at 90 % before the modifier tables: its value.
    assert!(Limit::new(6.83, Half::Minimum, None, 7.59, &line).wide());
    assert!(!Limit::new(163.0, Half::Minimum, None, 181.0, &line).wide());
}

#[test]
fn a_negative_stat_takes_a_minus_sign() {
    let line = StatLine::parse("-15% to Fire Resistance");
    let negative =
        |value, bounds| Limit::new(value, Half::Minimum, bounds, value, &line).negative();

    assert!(negative(-15.0, None));
    assert!(!negative(15.0, None));
    // A range across zero.
    assert!(negative(
        2.0,
        Some(Range {
            min: -5.0,
            max: 10.0
        })
    ));
    // The game writes a negative range without its sign.
    assert_eq!(
        signed(
            Range {
                min: 10.0,
                max: 20.0
            },
            -15.0
        ),
        Range {
            min: -20.0,
            max: -10.0
        }
    );
}

#[test]
fn a_corrupted_item_has_no_note_and_a_lit_state_tag_counts_among_the_active_filters() {
    let headhunter = parse(
        "Item Class: Belts\nRarity: Unique\nHeadhunter\nHeavy Belt\n--------\n\
         Item Level: 80\n--------\n{ Unique Modifier }\n+55(40-60) to maximum Life\n\
         --------\nCorrupted",
    );
    let staff = parse(
        "Item Class: Staves\nRarity: Normal\nChiming Staff\n--------\n\
         Item Level: 82\n--------\nCorrupted",
    );
    let stats = |item| {
        Stats::new(
            item,
            &CopiedLines::default(),
            &LineStats::default(),
            &TierRolls::default(),
            base_type(item),
        )
    };

    // The Corrupted tag says what is searched.
    assert_eq!(
        texts(&headhunter),
        ["Unique", "■ +55 to maximum Life ≥55 [40–60 0.75]"]
    );
    let headhunter_stats = Stats {
        item: &headhunter,
        rows: rows(&headhunter),
    };
    let mut filters = Filters::default();
    assert_eq!(headhunter_stats.active_filters(&filters), 1);
    filters.toggle_state(ItemState::Corrupted);
    assert_eq!(headhunter_stats.active_filters(&filters), 2);
    // A state the item doesn't have isn't counted.
    filters.toggle_state(ItemState::Mirrored);
    assert_eq!(headhunter_stats.active_filters(&filters), 2);
    // An item without modifiers has its filters to show.
    assert!(!stats(&staff).is_empty());
    assert_eq!(
        texts(&staff),
        ["Item", "■ Base: Chiming Staff", "■ ItemLevel: 82 ≥82"]
    );
    let orb = parse("Item Class: Stackable Currency\nRarity: Currency\nExalted Orb");
    assert!(stats(&orb).is_empty());
}

#[test]
fn a_line_the_trade_site_cant_search_isnt_a_filter() {
    let unique = parse(
        "Item Class: Boots\nRarity: Unique\nStormtread\nLattice Sandals\n--------\n\
         Item Level: 72\n--------\n\
         { Unique Modifier }\n+52(40-60) to maximum Life\n\
         { Unique Modifier }\nGain 3(2-4) Thunder Charges when you Dodge Roll\n\
         { Unique Modifier }\n20(15-25)% chance to gain Onslaught for 3 seconds when you kill an\n\
         enemy affected by Abyssal Wasting\n\
         { Unique Modifier }\nLife that would be lost by taking Damage is instead Reserved\n\
         until you take no Damage to Life for 5(4-6) seconds",
    );
    let lines = CopiedLines::new(
        unique.language(),
        unique.modifiers(),
        &LineStats::default(),
        None,
    );
    let line_stats = LineStats::new(unique.language(), unique.modifiers(), None);
    let stats = Stats::new(&unique, &lines, &line_stats, &TierRolls::default(), None);

    assert_eq!(
        texts(&unique),
        [
            "Unique",
            "■ +52 to maximum Life ≥52 [40–60 0.60]",
            // Its roll still shows, without a filter.
            "⊘ Gain 3 Thunder Charges when you Dodge Roll [2–4 0.50]",
            // The second line of a stat is searched by the first.
            // Its field holds the 20, not the 3 its string writes.
            "■ 20% chance to gain Onslaught for 3 seconds when you kill an ≥20 [15–25 0.50]",
            "enemy affected by Abyssal Wasting",
            // Or by the line with its numbers.
            "Life that would be lost by taking Damage is instead Reserved",
            "■ until you take no Damage to Life for 5 seconds ≤5 [4–6 0.50]",
        ]
    );
    let filters = Filters::default();
    assert_eq!(stats.active_filters(&filters), 3);
    let checked: Vec<FilterKey> = stats
        .checked(&filters)
        .into_iter()
        .map(|(key, _)| key)
        .collect();
    let key = |modifier, line| FilterKey::Stat(StatKey { modifier, line });
    assert_eq!(checked, [key(0, 0), key(2, 0), key(3, 1)]);
}

#[test]
fn the_active_filters_count_the_choices_of_the_player() {
    let crossbow = parse(include_str!(
        "../../item/test-items/en-rare-crossbow-all-modifiers.txt"
    ));
    let mut filters = Filters::default();

    let lines = CopiedLines::new(
        crossbow.language(),
        crossbow.modifiers(),
        &LineStats::default(),
        None,
    );
    let line_stats = LineStats::new(crossbow.language(), crossbow.modifiers(), None);
    let tiers = TierRolls::new(
        crossbow.language(),
        crossbow.modifiers(),
        base_type(&crossbow),
    );
    let stats = Stats::new(&crossbow, &lines, &line_stats, &tiers, base_type(&crossbow));
    let keys: Vec<FilterKey> = stats
        .rows
        .iter()
        .filter_map(|row| match row {
            Row::Stat(stat) => Some(stat.key),
            _ => None,
        })
        .collect();
    let choice = |checked| Filter {
        checked,
        minimum: String::new(),
        maximum: String::new(),
        bounds: None,
    };

    // The base type, the sockets, the physical DPS, the implicit and three modifiers; the
    // item level, the elemental DPS, the enchantment, the two runes, the damage and the
    // attack speed unchecked.
    assert_eq!(stats.active_filters(&filters), 7);
    // A rune checked by the player.
    filters.set(keys[6], choice(true));
    assert_eq!(stats.active_filters(&filters), 8);
    // The implicit unchecked.
    filters.set(keys[8], choice(false));
    assert_eq!(stats.active_filters(&filters), 7);
    filters.clear();
    assert_eq!(stats.active_filters(&filters), 7);
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

#[test]
fn a_range_of_one_value_is_full() {
    assert!((share(5.0, Range { min: 5.0, max: 5.0 }) - 1.0).abs() < 1e-9);
    assert!(
        (share(
            30.0,
            Range {
                min: 20.0,
                max: 40.0
            }
        ) - 0.5)
            .abs()
            < 1e-9
    );
}

/// The rows of `item` from its first group `title` on.
fn texts_from(item: &Item, title: &str) -> Vec<String> {
    let texts = texts(item);
    let start = texts
        .iter()
        .position(|text| text.starts_with(title))
        .unwrap_or_else(|| panic!("{title}: {texts:?}"));
    texts[start..].to_vec()
}

#[test]
fn a_line_that_adds_up_two_modifiers_shows_the_tier_of_each_and_counts_both() {
    let bow = read_again(parse(include_str!(
        "../../item/test-items/en-owner-trade-rare-bow-mirrored.txt"
    )));

    assert_eq!(
        texts_from(&bow, "Prefixes")[..4],
        [
            // The split line and the accuracy, two modifiers: 3 prefixes with the added damage.
            "Prefixes 3/3",
            // Without bar, the pure part first, then the part of the hybrid, its tooltip
            // naming its other line; the damage unchecked since the DPS counts it.
            "□ 258% increased Physical Damage ≥232 [T1 170–179 / T1 75–79 Hybrid (with +195 to Accuracy Rating)]",
            // The other line of the hybrid: its bar over the tiers of the hybrid, its tier and
            // its range, its tooltip naming the part of the summed line.
            "■ +195 to Accuracy Rating ≥176 [0.97 T1 175–200 Hybrid (with 75–79 of 258% increased Physical Damage)]",
            "□ Adds 36 to 56 Physical Damage ≥41 [0.88 T1 26–39 · 44–66 Desecrated]",
        ]
    );
}

#[test]
fn the_other_line_of_a_hybrid_whose_table_isnt_found_keeps_its_tier_without_bar() {
    let mut bow = read_again(parse(include_str!(
        "../../item/test-items/en-owner-trade-rare-bow-mirrored.txt"
    )));
    // A part whose ranges no tier of the hybrid gives the summed line: another table.
    let mut modifiers = bow.modifiers().to_vec();
    for modifier in &mut modifiers {
        if let Some(TierDoubt::Split { whole, .. }) = &mut modifier.doubt {
            whole.ranges = vec![Range {
                min: 76.0,
                max: 79.0,
            }];
        }
    }
    bow.set_modifiers(modifiers);

    // Its tier, its range and "Hybrid" from the reading, without bar.
    assert_eq!(
        texts_from(&bow, "Prefixes")[1..3],
        [
            "□ 258% increased Physical Damage ≥232 [T1 170–179 / T1 76–79 Hybrid (with +195 to Accuracy Rating)]",
            "■ +195 to Accuracy Rating ≥176 [T1 175–200 Hybrid (with 76–79 of 258% increased Physical Damage)]",
        ]
    );
}

#[test]
fn a_part_of_a_summed_line_in_two_neighbouring_tiers_shows_both() {
    let bow = include_str!("../../item/test-items/en-owner-trade-rare-bow-mirrored.txt");
    // The 248% is a pure T1 or T2 with the hybrid T1.
    let bow = read_again(parse(&bow.replace("258% increased", "248% increased")));

    assert_eq!(
        texts_from(&bow, "Prefixes")[..2],
        [
            "Prefixes 3/3",
            "□ 248% increased Physical Damage ≥223 [T1/T2 155–179 / T1 75–79 Hybrid (with +195 to Accuracy Rating)]",
        ]
    );
}

#[test]
fn a_line_that_adds_up_a_prefix_and_a_suffix_is_explicit_and_counts_in_both() {
    let tiara = include_str!("../../item/test-items/en-owner-trade-rare-helmet-fractured.txt");
    // A prefix open, filled by the rarity, desecrated, the prefix and the suffix of tier 1.
    let tiara = read_again(parse(
        &tiara
            .replace("+69 to maximum Energy Shield (desecrated)\n", "")
            .replace(
                "18% increased Rarity of Items found\n",
                "37% increased Rarity of Items found (desecrated)\n",
            ),
    ));

    let texts = texts_from(&tiara, "Prefixes");
    let title = |start: &str| texts.iter().find(|text| text.starts_with(start)).cloned();
    assert_eq!(title("Prefixes").as_deref(), Some("Prefixes 3/3"));
    assert_eq!(title("Suffixes").as_deref(), Some("Suffixes 3/3"));
    assert_eq!(
        texts[texts.len() - 3..],
        [
            "Explicit",
            // The prefix first; the tooltip of "Suffix" says they are added up; the origin on
            // a row of its own.
            "■ 37% increased Rarity of Items found ≥33 [T1 16–19 AffixPrefix / T1 15–18 AffixSuffix AddedUpWithPrefix / Desecrated]",
            "Trade copy note",
        ]
    );
}

#[test]
fn a_hybrid_prefix_added_up_with_a_pure_suffix_says_hybrid_prefix_on_both_its_lines() {
    // The prefixes and suffixes full: +310 is the hybrid T2 with the 35% and the pure
    // suffix T2.
    let shield = read_again(parse(
        "Item Class: Shields\nRarity: Rare\nDread Bulwark\nAvian Targe\n--------\n\
         Item Level: 80\n--------\n\
         +132 to maximum Life\n26% increased Block chance\n\
         35% increased Armour and Evasion\n+310 to Stun Threshold\n\
         +38% to Fire Resistance\n+32% to Cold Resistance\n",
    ));

    let texts = texts_from(&shield, "Prefixes");
    assert_eq!(texts[0], "Prefixes 3/3");
    assert_eq!(texts[3], "Suffixes 3/3");
    assert_eq!(
        texts[6..],
        [
            "Explicit",
            // The other line: its bar over the tiers of the hybrid, its tier and range, its
            // tooltip naming the part of the summed line.
            "■ 35% increased Armour and Evasion ≥32 [0.81 T2 33–38 HybridPrefix (with 64–94 of +310 to Stun Threshold)]",
            // The prefix first, "Hybrid prefix" naming the other line; the suffix added up.
            "■ +310 to Stun Threshold ≥279 [T2 64–94 HybridPrefix (with 35% increased Armour and Evasion) / T2 207–253 AffixSuffix AddedUpWithPrefix]",
            "Trade copy note",
        ]
    );
}

#[test]
fn a_hybrid_of_three_lines_has_its_tier_under_the_last_other_line() {
    // The flat Armour and Evasion before the %, as the in-game trade sorts them; 115% is the
    // pure T4 and the hybrid T1.
    let armour = read_again(parse(
        "Item Class: Body Armours\nRarity: Rare\nStorm Shell\nAncestral Mail\n--------\n\
         Item Level: 80\n--------\n\
         +180 to maximum Life\n+50 to Armour\n+41 to Evasion Rating\n\
         115% increased Armour and Evasion\n+38% to Fire Resistance\n+29 to Strength\n",
    ));

    assert_eq!(
        texts_from(&armour, "Prefixes")[..5],
        [
            "Prefixes 3/3",
            "□ +180 to maximum Life ≥162 [0.83 T3 175–189]",
            // Each other line with its bar, their tier and ranges under the last.
            "■ +50 to Armour ≥45 [0.96]",
            "■ +41 to Evasion Rating ≥37 [0.81 T1 42–52 · 40–50 Hybrid (with 39–42 of 115% increased Armour and Evasion)]",
            // Both other lines named.
            "■ 115% increased Armour and Evasion ≥104 [T4 68–79 / T1 39–42 Hybrid (with +50 to Armour and +41 to Evasion Rating)]",
        ]
    );
}

#[test]
fn a_pure_prefix_added_up_with_a_hybrid_suffix_says_hybrid_suffix_on_both_its_lines() {
    let bow = include_str!("../../item/test-items/en-owner-trade-rare-bow-mirrored.txt");
    // A prefix and a suffix open: +600 is the pure prefix T1 or T2 (451–650) and the hybrid
    // suffix T1 (41–60) with the Light Radius.
    let bow = read_again(parse(
        &bow.replace("258% increased Physical Damage\n", "")
            .replace("+25% to Critical Damage Bonus\n", "")
            .replace(
                "+195 to Accuracy Rating\n",
                "+600 to Accuracy Rating\n15% increased Light Radius\n",
            ),
    ));

    let accuracy = "■ +600 to Accuracy Rating ≥540 [T1/T2 451–650 AffixPrefix / T1 41–60 HybridSuffix (with 15% increased Light Radius)]";
    let radius = "■ 15% increased Light Radius ≥14 [1.00 T1 15 HybridSuffix (with 41–60 of +600 to Accuracy Rating)]";
    assert_eq!(
        texts_from(&bow, "Explicit"),
        [
            "Explicit",
            // The prefix first, without tooltip; "Hybrid suffix" naming the other line.
            accuracy,
            radius,
            "Trade copy note",
        ]
    );

    // The summed line after the other line of its hybrid: the same tiers.
    let reversed = include_str!("../../item/test-items/en-owner-trade-rare-bow-mirrored.txt");
    let reversed = read_again(parse(
        &reversed
            .replace("258% increased Physical Damage\n", "")
            .replace("+25% to Critical Damage Bonus\n", "")
            .replace(
                "+195 to Accuracy Rating\n",
                "15% increased Light Radius\n+600 to Accuracy Rating\n",
            ),
    ));
    assert_eq!(
        texts_from(&reversed, "Explicit"),
        ["Explicit", radius, accuracy, "Trade copy note"]
    );
}

#[test]
fn a_line_that_adds_up_the_middle_line_of_a_hybrid_has_its_tier_under_the_last_one() {
    let armour = read_again(parse(
        "Item Class: Body Armours\nRarity: Rare\nStorm Shell\nAncestral Mail\n--------\n\
         Item Level: 80\n--------\n\
         +180 to maximum Life\n+50 to Armour\n115% increased Armour and Evasion\n\
         +41 to Evasion Rating\n+38% to Fire Resistance\n+29 to Strength\n",
    ));

    assert_eq!(
        texts_from(&armour, "Prefixes")[2..5],
        [
            "■ +50 to Armour ≥45 [0.96]",
            "■ 115% increased Armour and Evasion ≥104 [T4 68–79 / T1 39–42 Hybrid (with +50 to Armour and +41 to Evasion Rating)]",
            "■ +41 to Evasion Rating ≥37 [0.81 T1 42–52 · 40–50 Hybrid (with 39–42 of 115% increased Armour and Evasion)]",
        ]
    );
}

/// A part of a split line of `affix` in tier 1, over 1–2.
fn part(affix: Affix) -> PartTier {
    PartTier {
        affix,
        tiers: vec![1],
        ranges: vec![Range { min: 1.0, max: 2.0 }],
    }
}

/// The rows of the parts `whole` and `added` of a split line: the tier, the word and its
/// tooltip of each, the ranges left out.
fn split_texts(whole: Affix, added: Affix) -> Vec<String> {
    split_rows(&part(whole), &part(added), &[])
        .iter()
        .map(|row| format!("{:?} {:?} {:?}", row.tier.number, row.word, row.tip))
        .collect()
}

#[test]
fn the_parts_of_a_split_line_show_the_prefix_first_then_the_pure_one() {
    // A pure prefix with a suffix hybrid: the prefix first, though it is the added part.
    assert_eq!(
        split_texts(Affix::Suffix, Affix::Prefix),
        [
            "1 Some(AffixPrefix) None",
            "1 Some(AffixSuffix) Some(Text(AddedUpWithPrefix))",
        ]
    );
    // A pure prefix with a hybrid suffix: "Hybrid suffix" in place of "Suffix", its tooltip
    // naming the other line.
    let others = ["+10% to Light Radius"];
    let rows = split_rows(&part(Affix::Suffix), &part(Affix::Prefix), &others);
    let words: Vec<_> = rows.iter().map(|row| (row.word, row.tip.clone())).collect();
    assert_eq!(
        words,
        [
            (Some(Message::AffixPrefix), None),
            (
                Some(Message::HybridSuffix),
                Some(WordTip::HybridLines(others.to_vec()))
            ),
        ]
    );
    let mut added = part(Affix::Suffix);
    added.tiers = vec![2];
    // Two suffixes: the pure part (`added`) first, though of a worse tier, without words.
    let rows = split_rows(&part(Affix::Suffix), &added, &[]);
    let tiers: Vec<_> = rows.iter().map(|row| (row.tier.number, row.word)).collect();
    assert_eq!(tiers, [(2, None), (1, None)]);
}

#[test]
fn a_modifier_counts_the_affixes_it_takes_in_the_room_of_its_item() {
    let modifier = |kind, doubt| Modifier {
        kind,
        origin: None,
        name: None,
        tier: None,
        tags: Vec::new(),
        unrevealed: false,
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
        ranges: Vec::new(),
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
        unrevealed: true,
        ..modifier(prefix, None)
    };
    assert_eq!(counts(&unrevealed), (1, 0));
}
