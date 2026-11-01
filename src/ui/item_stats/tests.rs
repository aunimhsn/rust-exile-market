//! The stat rows of the item block, on the test items and on items built for a case: the
//! groups and their order, the filters at first and their range fields, the bars over the
//! tiers, the tiers found from the values of an item copied from the in-game trade, the lines
//! in doubt and the notes.

use super::*;
use crate::item::trade_copy::as_in_game_trade;
use crate::item::{CopiedItem, ItemState};
use crate::language::Language;

fn parse(text: &str) -> Item {
    Item::parse(&CopiedItem::from_text(text).expect("an item")).expect("read")
}

/// The lines of a stat written on several lines, a ` / ` between them.
fn lines_text(lines: &[StatLine]) -> String {
    let texts: Vec<_> = lines.iter().map(|line| line.text.as_str()).collect();
    texts.join(" / ")
}

/// The rows of `item`, its lines read in English.
fn rows(item: &Item) -> Vec<Row<'_>> {
    rows_of(item, base_type(item))
}

/// The rows of `item` of the base type `base_type`, its lines read in English: the base type
/// of a magic item is found among the names of the trade site.
fn rows_of<'a>(item: &'a Item, base_type: Option<&'a str>) -> Vec<Row<'a>> {
    let lines = CopiedLines::new(
        item.language(),
        item.modifiers(),
        &LineStats::default(),
        None,
    );
    let stats = LineStats::new(item.language(), item.modifiers(), None);
    let tiers = TierRolls::new(
        item.language(),
        item.modifiers(),
        base_type,
        item.item_level(),
    );
    super::rows(item, &lines, &stats, &tiers, base_type)
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
/// after the ranges followed by its tooltip, and two neighbouring tiers `T1/T2 155–179`); last,
/// the tag of the radius of a line of a Time-Lost jewel, `<Small>`.
fn row_texts<'a>(rows: impl IntoIterator<Item = Row<'a>>) -> Vec<String> {
    // `2/3`, or the count alone: `6`.
    let fill = |filled: u32, max: Option<u32>| match max {
        Some(max) => format!("{filled}/{max}"),
        None => filled.to_string(),
    };
    rows.into_iter()
        .map(|row| match row {
            Row::Group(Group::Prefixes { filled, max }) => {
                format!("Prefixes {}", fill(filled, max))
            }
            Row::Group(Group::Suffixes { filled, max }) => {
                format!("Suffixes {}", fill(filled, max))
            }
            Row::Group(group) => format!("{group:?}"),
            Row::UnrevealedNote => "Unrevealed note".to_owned(),
            Row::TradeCopyNote => "Trade copy note".to_owned(),
            Row::UniqueRollsNote => "Unique rolls note".to_owned(),
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
                    Some(Roll::WaystoneTier(tier)) => format!(" [Waystone T{tier}]"),
                };
                let radius = text
                    .radius_skills()
                    .map(|skills| format!(" <{skills:?}>"))
                    .unwrap_or_default();
                let text = match text {
                    StatText::Line(line) | StatText::RadiusLine { line, .. } => line.to_owned(),
                    StatText::Lines(lines) => lines_text(lines),
                    StatText::BaseType { base_type, .. } => format!("Base: {base_type}"),
                    StatText::Property(property, value) => {
                        format!("{property:?}: {}", format::number(value))
                    }
                    StatText::CatalystQuality(kind, value) => {
                        let kind = kind
                            .catalyst
                            .map_or_else(|| kind.words.clone(), |catalyst| format!("{catalyst:?}"));
                        format!("{kind} quality: {}", format::number(value))
                    }
                    StatText::Pseudo(pseudo, value) => {
                        format!("{pseudo:?}: {}", format::number(value))
                    }
                    StatText::Unrevealed(affix) => format!("Unrevealed {affix:?}"),
                };
                format!("{check}{text}{limit}{roll}{radius}")
            }
        })
        .collect()
}

/// A tier under a line as [`row_texts`] writes it: `T1/T2 155–179`, then its word and its
/// tooltip, `AffixSuffix AddedUpWithPrefix`, `Hybrid (with +195 to Accuracy Rating)`, `Hybrid
/// (with 75–79 of 258% increased Physical Damage)`; the lord before the word, `Amanamu
/// AffixPrefix`, and a tier written in the style of its word between braces, `{T2 AffixSuffix}`.
fn tier_row_text(row: &TierRow) -> String {
    let tier = tier_text(&row.tier);
    let tier = match row.better {
        Some(better) => format!("T{better}/{tier}"),
        None => tier,
    };
    let word = row.word.map(|word| match row.lord {
        Some(lord) => format!("{} {word:?}", lord.name()),
        None => format!("{word:?}"),
    });
    let tip = row.tip.as_ref().map(|tip| match tip {
        WordTip::Text(message) => format!("{message:?}"),
        WordTip::HybridLines(lines) => format!("(with {})", lines.join(" and ")),
        WordTip::HybridPart { ranges, line } => {
            format!("(with {} of {line})", ranges_text(ranges))
        }
    });
    let currency = row.currency.map(|currency| format!("{currency:?}"));
    let text = [Some(tier), word, tip, currency]
        .into_iter()
        .flatten()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    // A tier written in the style of its word, between braces.
    if row.quiet_tier() {
        format!("{{{text}}}")
    } else {
        text
    }
}

/// A tier as [`row_texts`] writes it: `T1 15–18`, `15–18` for a modifier of one tier only.
fn tier_text(tier: &ModifierTier) -> String {
    let number = tier.number.map(|number| format!("T{number}"));
    [number.unwrap_or_default(), ranges_text(&tier.ranges)]
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
        ["■ Minions have 12% increased maximum Life ≥11 [0.70 5–15]"]
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
            "■ Minions have 21% increased maximum Life ≥19 [0.67 19–22]",
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
        Some("■ +1 to Level of all Projectile Skills")
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
    let stats = LineStats::new(flask.language(), flask.modifiers(), None);
    let tiers = TierRolls::new(
        flask.language(),
        flask.modifiers(),
        base,
        flask.item_level(),
    );
    let flask = row_texts(super::rows(&flask, &lines, &stats, &tiers, base));
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
        jewel.contains(&"■ Minions have 12% increased maximum Life ≥11 [0.70 5–15]".to_owned()),
        "{jewel:?}"
    );

    // The essences of a bow, written without tier, named after their ranges.
    let bow = texts(&parse(include_str!(
        "../../item/test-items/en-rare-bow-fractured.txt"
    )));
    assert!(
        bow.iter().any(|text| text
            .starts_with("■ 20% chance to gain Onslaught on Killing Hits with this Weapon ≥")
            && text.ends_with(" [0.00 20–25 Essence]")),
        "{bow:?}"
    );
    // The `+3` of an older patch, which no table holds today (the bows take `+2`): nothing.
    assert!(
        bow.contains(&"■ +3 to Level of all Attack Skills ≥3".to_owned()),
        "{bow:?}"
    );
}

#[test]
fn a_modifier_of_an_essence_or_an_alloy_names_its_currency() {
    // The owner's gloves: the alloy written crafted, its currency in place of "Crafted".
    let gloves = texts(&parse(include_str!(
        "../../item/test-items/fr-rare-gloves-alloy.txt"
    )));
    let alloy = "11% d'Augmentation de Vitesse d'Attaque tant qu'il vous manque de la \
                 Sauvegarde runique";
    assert!(
        gloves.contains(&format!("■ {alloy} ≥10 [0.20 10–15 Alloy]")),
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
            &"■ 13% increased Quantity of Gold Dropped by Slain Enemies ≥12 [0.60 10–15 Essence]"
                .to_owned()
        ),
        "{trade:?}"
    );
    assert!(
        trade.iter().any(|text| text
            .starts_with("■ 11% increased Attack Speed while missing Runic Ward ≥")
            && text.ends_with(" [0.20 10–15 Alloy]")),
        "{trade:?}"
    );
}

#[test]
fn a_modifier_of_a_currency_fractured_or_desecrated_keeps_its_origin() {
    let essence = Some(Currency::Essence);

    assert_eq!(
        Origin::of_affix(None, essence),
        Some(Origin::Currency(Currency::Essence))
    );
    assert_eq!(
        Origin::of_affix(Some(ModifierOrigin::Crafted), Some(Currency::Alloy)),
        Some(Origin::Currency(Currency::Alloy))
    );
    assert_eq!(
        Origin::of_affix(Some(ModifierOrigin::Fractured), essence),
        Some(Origin::Written(ModifierOrigin::Fractured))
    );
    // A crafted modifier of no currency (a craft of the Runes of Aldur, `Motivating`).
    assert_eq!(
        Origin::of_affix(Some(ModifierOrigin::Crafted), None),
        Some(Origin::Written(ModifierOrigin::Crafted))
    );
    assert_eq!(Origin::of_affix(None, None), None);
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
            &"■ Grenade Skills have +1 Cooldown Use ≥1 [{Amanamu AffixPrefix} / {T2 AffixSuffix} / Desecrated]"
                .to_owned()
        ),
        "{texts:?}"
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
            "■ Minions have 21% increased maximum Life ≥19 [0.67 19–22]",
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
    let last = |item: &Item| texts(item).into_iter().rfind(|text| text.starts_with('■'));

    // A tier without range: the bar over the three tiers, "T2", the field from 2.
    let amulet = advanced("Amulets", "Gold Amulet", " (Tier: 2)");
    let t2 = Some("■ +2 to Level of all Minion Skills ≥2 [0.50 T2]".to_owned());
    assert_eq!(last(&parse(&amulet)), t2);
    assert_eq!(last(&in_game_trade(&amulet)), t2);
    // Fixed in its tier, which the game doesn't write: no field, nothing under it.
    let belt = advanced("Belts", "Heavy Belt", "");
    let fixed = Some("■ +2 to Level of all Minion Skills".to_owned());
    assert_eq!(last(&parse(&belt)), fixed);
    assert_eq!(last(&in_game_trade(&belt)), fixed);
}

#[test]
fn a_time_lost_jewel_shows_its_lines_over_their_ranges_in_both_copies() {
    // The owner's Time-Lost Sapphire, every modifier of one tier: its bar and its field over
    // the range of that tier, nothing under its fixed `1%`; each line without the words of
    // its radius, its tag after its range, alone under the fixed line; the same from the
    // in-game trade.
    let text = include_str!("../../item/test-items/en-owner-rare-jewel-time-lost.txt");
    let lines: Vec<&str> = text.lines().filter(|line| !line.starts_with('{')).collect();
    let in_game_trade = read_again(parse(&lines.join("\n")));
    let modifiers = [
        "Prefixes 2/2",
        "■ 2% increased Elemental Damage ≥2 [1.00 1–2] <Small>",
        "■ 7% faster start of Energy Shield Recharge ≥6 [1.00 5–7] <Notable>",
        "Suffixes 2/2",
        "■ 1% increased Mana Regeneration Rate ≥1 [0.00 1–2] <Small>",
        "■ 1% of Damage taken Recouped as Life <Notable>",
    ];
    let explicit = |texts: Vec<String>| -> Vec<String> {
        texts
            .into_iter()
            .skip_while(|text| !text.starts_with("Prefixes"))
            .take(modifiers.len())
            .collect()
    };

    assert_eq!(explicit(texts(&parse(text))), modifiers);
    assert_eq!(explicit(texts(&in_game_trade)), modifiers);
}

#[test]
fn a_french_time_lost_jewel_shows_its_lines_without_the_words_of_its_radius() {
    // Both radiuses, the notable written `éminents` by the game; a line without these words
    // (`Améliore le Rayon en Grand`) stays whole, without tag.
    let jewel = parse(include_str!(
        "../../item/test-items/fr-rare-jewel-time-lost.txt"
    ));
    let large = parse(include_str!(
        "../../item/test-items/fr-rare-jewel-time-lost-large.txt"
    ));

    assert_eq!(
        texts_from(&jewel, "Prefixes"),
        [
            "Prefixes 2/2",
            "■ 2% d'Augmentation des Dégâts élémentaires ≥2 [1.00 1–2] <Small>",
            "■ Le Démarrage de la Recharge du Bouclier d'énergie est 7% plus rapide ≥6 [1.00 5–7] <Notable>",
            "Suffixes 2/2",
            "■ 1% d'Augmentation du Taux de Régénération de Mana ≥1 [0.00 1–2] <Small>",
            "■ 1% des Dégâts subis sont Recouvrés sous forme de Vie <Notable>",
        ]
    );
    assert_eq!(
        texts_from(&large, "Prefixes"),
        [
            "Prefixes 2/2",
            "■ 2% d'Augmentation des Dégâts des Sorts ≥2 [1.00 1–2] <Small>",
            "■ Améliore le Rayon en Grand [T1]",
            "Suffixes 2/2",
            "■ Vous gagnez du Seuil d'Étourdissement équivalent à 2% de votre Bouclier d'énergie maximal ≥2 [1.00 1–2] <Small>",
            "■ 2% d'Augmentation de l'Amplitude de l'Inflammabilité ≥2 [0.00 2–3] <Small>",
        ]
    );
}

#[test]
fn every_line_of_a_time_lost_jewel_in_the_game_data_is_shown_short() {
    // Each matcher of a stat granted to the passive skills in a radius, in both languages, so
    // that game data written again with other words fails here rather than shows long lines.
    for data in [
        include_str!("../../../assets/game-data/stats-en.ndjson"),
        include_str!("../../../assets/game-data/stats-fr.ndjson"),
    ] {
        let mut lines = 0;
        for stat in data.lines() {
            let stat: serde_json::Value = serde_json::from_str(stat).expect("a stat");
            let reference = stat["ref"].as_str().expect("its reference");
            let skills = if reference.starts_with("Small Passive Skills in Radius also grant ") {
                RadiusSkills::Small
            } else if reference.starts_with("Notable Passive Skills in Radius also grant ") {
                RadiusSkills::Notable
            } else {
                continue;
            };
            for matcher in stat["matchers"].as_array().expect("its matchers") {
                let line = matcher["string"].as_str().expect("a line");
                assert_eq!(StatText::line(line).radius_skills(), Some(skills), "{line}");
                lines += 1;
            }
        }
        assert!(lines > 100, "{lines}");
    }
}

#[test]
fn a_line_of_a_time_lost_jewel_is_named_whole_to_a_screen_reader() {
    let i18n = Translator::new(Language::English);
    let line = "Small Passive Skills in Radius also grant 2% increased Elemental Damage";

    assert_eq!(
        StatText::line(line).text(&i18n),
        "2% increased Elemental Damage"
    );
    assert_eq!(StatText::line(line).label(&i18n), line);
    assert_eq!(
        StatText::line("+20 to Strength").label(&i18n),
        "+20 to Strength"
    );
}

#[test]
fn the_words_of_a_tier_name_the_lord_and_take_a_tier_without_range() {
    let row = |number: Option<u32>, ranges: Vec<Range>, affix, lord| TierRow {
        lord,
        ..TierRow::affix(&AffixTier {
            affix,
            tier: number.unwrap_or(1),
            hides_number: number.is_none(),
            ranges,
            lord: None,
        })
    };
    let english = Translator::new(Language::English);
    let french = Translator::new(Language::French);
    let words = |row: &TierRow, i18n: &Translator| row_word(row, i18n);
    let amanamu = row(None, Vec::new(), Affix::Prefix, Some(Lord::Amanamu));
    let kurgal = row(None, Vec::new(), Affix::Suffix, Some(Lord::Kurgal));
    let t2 = row(Some(2), Vec::new(), Affix::Suffix, None);
    let ranged = row(
        Some(1),
        vec![Range {
            min: 16.0,
            max: 19.0,
        }],
        Affix::Prefix,
        None,
    );

    assert_eq!(words(&amanamu, &english).as_deref(), Some("Amanamu Prefix"));
    assert_eq!(
        words(&amanamu, &french).as_deref(),
        Some("Préfixe d'Amanamu")
    );
    assert_eq!(
        words(&kurgal, &french).as_deref(),
        Some("Suffixe de Kurgal")
    );
    assert_eq!(words(&kurgal, &english).as_deref(), Some("Kurgal Suffix"));
    let ulaman = row(None, Vec::new(), Affix::Prefix, Some(Lord::Ulaman));
    assert_eq!(words(&ulaman, &french).as_deref(), Some("Préfixe d'Ulaman"));
    // A tier without range written as the word, in its style; with a range, apart.
    assert_eq!(words(&t2, &english).as_deref(), Some("T2 Suffix"));
    assert!(t2.quiet_tier());
    assert_eq!(words(&ranged, &english).as_deref(), Some("Prefix"));
    assert!(!ranged.quiet_tier());
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

/// `item`, a unique copied from the in-game trade, with the rolls poe2scout writes for Soul
/// Tether.
fn with_soul_tether_rolls(mut item: Item) -> Item {
    let written = |modifiers: &[&str]| -> Vec<String> {
        modifiers
            .iter()
            .map(|&modifier| modifier.to_owned())
            .collect()
    };
    let modifiers = game_data::with_unique_rolls(
        item.language(),
        item.modifiers(),
        &written(&[
            "(15-20)% increased Charm Effect Duration",
            "Has (1-3) Charm Slots",
        ]),
        &written(&[
            "+(40-60) to maximum Energy Shield",
            "You lose (5-5)% of maximum Energy Shield per second",
            "Excess Life Recovery from Leech is applied to Energy Shield",
        ]),
        None,
    );
    item.set_modifiers(modifiers);
    item
}

#[test]
fn a_unique_copied_from_the_in_game_trade_has_unchecked_filters_before_its_rolls() {
    let belt = parse(include_str!(
        "../../item/test-items/en-owner-trade-unique-belt-mutated.txt"
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
fn a_unique_copied_from_the_in_game_trade_shows_the_rolls_of_poe2scout_and_a_note() {
    let belt = with_soul_tether_rolls(parse(include_str!(
        "../../item/test-items/en-owner-trade-unique-belt-mutated.txt"
    )));

    assert_eq!(
        texts(&belt),
        [
            "Enchant",
            "□ 15% increased Evasion Rating ≥15",
            "Implicit",
            "■ 16% increased Charm Effect Duration ≥16 [15–20 0.20]",
            // poe2scout writes the plural.
            "■ Has 3 Charm Slots ≥3 [1–3 1.00]",
            "Unique",
            // The Vaal modifier, which poe2scout never holds.
            "□ 14% increased Energy Shield Recovery rate ≥14 [Vaal]",
            "■ +51 to maximum Energy Shield ≥51 [40–60 0.55]",
            // `(5-5)`: it never varies.
            "You lose 5% of maximum Energy Shield per second",
            "Excess Life Recovery from Leech is applied to Energy Shield",
            "Unique rolls note",
        ]
    );
}

#[test]
fn a_french_unique_copied_from_the_in_game_trade_gets_the_rolls_by_its_stats() {
    let belt = with_soul_tether_rolls(parse(include_str!(
        "../../item/test-items/fr-trade-unique-belt-mutated.txt"
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
            "■ Minions have 12% increased maximum Life ≥11 [0.70 5–15]"
        ]
    );
    assert_eq!(
        texts(&french),
        [
            "Item",
            "■ Base: Rubis",
            "Prefixes 1/2",
            "■ Les Créatures ont 12% d'Augmentation de Vie maximale ≥11 [0.70 5–15]"
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
    // Up to the 30 % of an exceptional base or of a Vaal Infuser.
    assert_eq!(
        settled(&leveling, Property::Quality, Half::Minimum, "80"),
        "30"
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
fn the_quality_of_a_catalyst_is_a_filter_with_its_kind_unchecked() {
    let amulet = parse(include_str!(
        "../../item/test-items/en-owner-rare-amulet-catalyst.txt"
    ));
    let ring = parse(include_str!(
        "../../item/test-items/fr-rare-ring-catalyst.txt"
    ));

    assert_eq!(
        item_group(&amulet),
        [
            "■ Base: Gloam Amulet",
            "□ ItemLevel: 79 ≥79",
            "□ Mana quality: 40 ≥40",
        ]
    );
    // 20 %, a quality a base of a ring has only from a catalyst, is a filter too.
    assert_eq!(
        item_group(&ring),
        [
            "■ Base: Bague prismatique",
            "□ ItemLevel: 82 ≥82",
            "□ Cold quality: 20 ≥20",
        ]
    );
    // Its text says that the trade site searches any kind.
    let catalyst = rows(&amulet)
        .into_iter()
        .find_map(|row| match row {
            Row::Stat(stat) if matches!(stat.text, StatText::CatalystQuality(..)) => {
                Some(stat.text)
            }
            _ => None,
        })
        .expect("the filter of the quality");
    assert_eq!(catalyst.tip(), Some(Message::FilterAnyQualityKind));
}

#[test]
fn the_field_of_the_quality_stays_within_the_highest_quality_of_the_base() {
    // A normal item, whose quality is a filter.
    let normal = |class: &str, base: &str| {
        parse(&format!(
            "Item Class: {class}\nRarity: Normal\n{base}\n--------\n\
             Quality: +10% (augmented)\n--------\nItem Level: 80"
        ))
    };
    let highest = |item: &Item| settled(item, Property::Quality, Half::Minimum, "99");
    assert_eq!(highest(&normal("Bows", "Gemini Bow")), "30");
    assert_eq!(highest(&normal("One Hand Maces", "Flanged Mace")), "40");
    assert_eq!(highest(&normal("Life Flasks", "Ultimate Life Flask")), "23");
    assert_eq!(highest(&normal("Charms", "Ruby Charm")), "23");
    let jewel = parse(
        "Item Class: Jewels\nRarity: Rare\nWoe Spark\nSapphire\n--------\n\
         Quality (Caster Modifiers): +20% (augmented)\n--------\nItem Level: 80",
    );
    assert_eq!(highest(&jewel), "20");
    // A quality above the highest of its base widens its field.
    let bow = parse(
        "Item Class: Bows\nRarity: Normal\nGemini Bow\n--------\n\
         Quality: +45% (augmented)\n--------\nItem Level: 80",
    );
    assert_eq!(highest(&bow), "45");
    assert_eq!(
        highest(&parse(include_str!(
            "../../item/test-items/en-owner-rare-amulet-catalyst.txt"
        ))),
        "50"
    );
    assert_eq!(
        highest(&parse(include_str!(
            "../../item/test-items/en-owner-rare-ring-catalyst.txt"
        ))),
        "75"
    );
    // A base the game data doesn't know: the highest quality of any item.
    assert_eq!(highest(&normal("Rings", "Mystery Ring")), "75");
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
fn the_lines_of_a_stat_with_a_number_on_two_of_them_stay_apart() {
    // Two bars can't be told under one text: a row per line, the first without filter.
    let unique = parse(
        "Item Class: Body Armours\nRarity: Unique\nTest\nGarment\n--------\n\
         Item Level: 72\n--------\n\
         { Unique Modifier }\nAlternating every 5 seconds:\n\
         Take 20(10-20)% more Damage from Hits\nTake 20(10-20)% more Damage over time",
    );

    assert_eq!(
        texts(&unique),
        [
            "Unique",
            "Alternating every 5 seconds:",
            "■ Take 20% more Damage from Hits ≥20 [10–20 1.00]",
            "Take 20% more Damage over time [10–20 1.00]",
        ]
    );
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
            // A stat on two lines is one row, searched by its first line: its field holds the
            // 20, not the 3 its string writes.
            "■ 20% chance to gain Onslaught for 3 seconds when you kill an / enemy affected by \
             Abyssal Wasting ≥20 [15–25 0.50]",
            // Or by the line with its numbers.
            "■ Life that would be lost by taking Damage is instead Reserved / until you take no \
             Damage to Life for 5 seconds ≤5 [4–6 0.50]",
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
fn the_stats_of_one_modifier_are_joined_each_on_its_own_lines() {
    // Three stats in one modifier, the second searched by its first line, the third by its
    // second: one row each, never one across two stats.
    let unique = parse(
        "Item Class: Boots\nRarity: Unique\nTest\nLattice Sandals\n--------\n\
         Item Level: 72\n--------\n\
         { Unique Modifier }\n+52(40-60) to maximum Life\n\
         20(15-25)% chance to gain Onslaught for 3 seconds when you kill an\n\
         enemy affected by Abyssal Wasting\n\
         Life that would be lost by taking Damage is instead Reserved\n\
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
            "■ 20% chance to gain Onslaught for 3 seconds when you kill an / enemy affected by \
             Abyssal Wasting ≥20 [15–25 0.50]",
            "■ Life that would be lost by taking Damage is instead Reserved / until you take no \
             Damage to Life for 5 seconds ≤5 [4–6 0.50]",
        ]
    );
    let filters = Filters::default();
    let checked: Vec<FilterKey> = stats
        .checked(&filters)
        .into_iter()
        .map(|(key, _)| key)
        .collect();
    let key = |modifier, line| FilterKey::Stat(StatKey { modifier, line });
    assert_eq!(checked, [key(0, 0), key(0, 1), key(0, 4)]);
}

#[test]
fn a_stat_on_two_lines_the_trade_site_cant_search_is_one_row() {
    // The stat of a unique as an implicit: no implicit id, each line unsearchable, joined
    // with its sign once.
    let unique = parse(
        "Item Class: Boots\nRarity: Unique\nTest\nLattice Sandals\n--------\n\
         Item Level: 72\n--------\n\
         { Implicit Modifier }\n\
         20(15-25)% chance to gain Onslaught for 3 seconds when you kill an\n\
         enemy affected by Abyssal Wasting",
    );

    assert_eq!(
        texts(&unique),
        [
            "Implicit",
            "⊘ 20% chance to gain Onslaught for 3 seconds when you kill an / enemy affected by \
             Abyssal Wasting [15–25 0.50]",
        ]
    );
}

#[test]
fn the_rolls_of_the_lines_of_a_stat_are_joined_when_one_text_can_tell_them() {
    let ranges = || {
        vec![Range {
            min: 15.0,
            max: 25.0,
        }]
    };
    let tier = || {
        Some(Box::new(TierRow::plain(ModifierTier {
            number: Some(2),
            ranges: ranges(),
        })))
    };
    let affix = |share, tier, origin| Roll::Affix {
        share,
        tier,
        origin,
    };
    let bar = |origin| Roll::Bar {
        share: 0.5,
        ranges: ranges(),
        origin,
    };
    let desecrated = Some(Origin::Written(ModifierOrigin::Desecrated));
    let vaal = Some(Origin::Written(ModifierOrigin::Vaal));
    let tiers = Roll::Tiers {
        rows: Vec::new(),
        origin: desecrated,
    };

    for (roll, next, joined) in [
        // The bar of the first line, the tier and the origin under the last.
        (
            affix(Some(0.8), None, None),
            affix(None, tier(), desecrated),
            Some(affix(Some(0.8), tier(), desecrated)),
        ),
        // The bar on the last line, under the words before it.
        (
            affix(None, None, None),
            affix(Some(0.8), tier(), None),
            Some(affix(Some(0.8), tier(), None)),
        ),
        // The bar of a unique, the origin of its Vaal modifier under its last line.
        (bar(None), affix(None, None, vaal), Some(bar(vaal))),
        (affix(None, None, vaal), bar(None), Some(bar(vaal))),
        // Two bars or two tiers can't be told under one text.
        (
            affix(Some(0.8), None, None),
            affix(Some(0.2), None, None),
            None,
        ),
        (affix(None, tier(), None), affix(None, tier(), None), None),
        (bar(None), bar(None), None),
        (bar(None), affix(None, tier(), None), None),
        // The tiers of a line that adds up two modifiers, under its bar: kept apart.
        (affix(Some(0.8), None, None), tiers.clone(), None),
        (tiers.clone(), affix(None, None, None), None),
    ] {
        assert_eq!(
            joined_roll(roll.clone(), next.clone()),
            joined,
            "{roll:?} then {next:?}"
        );
    }
}

#[test]
fn the_lines_of_a_stat_read_as_one_sentence() {
    // Joined by a space, also for a screen reader.
    let item = parse(include_str!(
        "../../item/test-items/en-owner-rare-focus-kurgal.txt"
    ));
    let rows = rows(&item);
    let text = rows
        .iter()
        .find_map(|row| match row {
            Row::Stat(stat) if matches!(stat.text, StatText::Lines(_)) => Some(stat.text),
            _ => None,
        })
        .expect("the stat of Kurgal");
    let i18n = Translator::new(Language::English);
    let sentence = "23% chance when collecting an Elemental Infusion to gain an additional \
                    Elemental Infusion of the same type";
    assert_eq!(text.text(&i18n), sentence);
    assert_eq!(text.label(&i18n), sentence);
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
        crossbow.item_level(),
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
fn a_part_of_a_summed_line_of_one_tier_only_writes_no_tier_number() {
    let mut bow = read_again(parse(include_str!(
        "../../item/test-items/en-owner-trade-rare-bow-mirrored.txt"
    )));
    // As if each part were a modifier of one tier only, a lord's.
    let mut modifiers = bow.modifiers().to_vec();
    for modifier in &mut modifiers {
        if let Some(TierDoubt::Split { whole, added, .. }) = &mut modifier.doubt {
            whole.hides_number = true;
            added.hides_number = true;
        }
    }
    bow.set_modifiers(modifiers);

    // The ranges of each part without "T1", under the summed line and the other line of the
    // hybrid.
    let texts = texts_from(&bow, "Prefixes");
    assert_eq!(
        texts.get(1).map(String::as_str),
        Some(
            "□ 258% increased Physical Damage ≥232 [170–179 / 75–79 Hybrid (with +195 to Accuracy Rating)]"
        )
    );
    assert!(
        texts.get(2).is_some_and(|text| text
            .ends_with(" 175–200 Hybrid (with 75–79 of 258% increased Physical Damage)]")
            && !text.contains("T1")),
        "{texts:?}"
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
    // A fixed value, without range under it.
    let radius = "■ 15% increased Light Radius ≥14 [1.00 T1 HybridSuffix (with 41–60 of +600 to Accuracy Rating)]";
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
        hides_number: false,
        ranges: vec![Range { min: 1.0, max: 2.0 }],
        currency: None,
    }
}

/// The rows of the parts `whole` and `added` of a split line: the tier, the word and its
/// tooltip of each, the ranges left out.
fn split_texts(whole: Affix, added: Affix) -> Vec<String> {
    split_rows(&part(whole), &part(added), &[])
        .iter()
        .map(|row| {
            format!(
                "{:?} {:?} {:?}",
                row.tier.number.unwrap_or_default(),
                row.word,
                row.tip
            )
        })
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
    assert_eq!(tiers, [(Some(2), None), (Some(1), None)]);
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
        unrevealed: true,
        ..modifier(prefix, None)
    };
    assert_eq!(counts(&unrevealed), (1, 0));
}

#[test]
fn the_part_of_an_alloy_in_a_summed_line_names_its_currency() {
    let bow = include_str!("../../item/test-items/en-owner-trade-rare-bow-mirrored.txt");
    // Two prefixes open: +800 is a pure prefix T2 or T3 (347–550) and the accuracy of the
    // hybrid of the Celestial Alloy (327–427), with its attack speed (5–8), both crafted.
    let bow = read_again(parse(
        &bow.replace("258% increased Physical Damage\n", "")
            .replace("+25% to Critical Damage Bonus\n", "")
            .replace(
                "+195 to Accuracy Rating\n",
                "+800 to Accuracy Rating (crafted)\n6% increased Attack Speed (crafted)\n",
            ),
    ));

    let texts = texts(&bow);
    // "Alloy" after the part of the alloy, in place of "Crafted" under the line.
    assert!(
        texts.contains(&"■ +800 to Accuracy Rating ≥720 [T2/T3 347–550 / 327–427 Hybrid (with 6% increased Attack Speed) Alloy]".to_owned()),
        "{texts:?}"
    );
    assert!(
        texts.contains(&"□ 6% increased Attack Speed ≥5 [0.33 5–8 Hybrid (with 327–427 of +800 to Accuracy Rating) Alloy]".to_owned()),
        "{texts:?}"
    );
}

#[test]
fn a_line_of_the_in_game_trade_also_an_ordinary_tier_names_no_currency() {
    // `+3 to Level of all Spell Skills` on a wand: the ordinary T2 of 4, or the essence.
    let wand = texts(&read_again(parse(
        "Item Class: Wands\nRarity: Rare\nStorm Song\nAcrid Wand\n--------\n\
         Item Level: 82\n--------\n+3 to Level of all Spell Skills",
    )));
    let line = wand
        .iter()
        .find(|text| text.contains("+3 to Level of all Spell Skills"))
        .expect("the line");
    assert!(
        line.contains(" T2") && !line.contains("Essence"),
        "{wand:?}"
    );
}

#[test]
fn a_waystone_shows_its_modifiers_over_their_ranges_in_both_copies() {
    // The owner's waystones of tier 12, in the band of tiers 11 to 15: every modifier of one
    // tier there, its bar and its field over the range of that tier, without tier; from the
    // in-game trade, the same, each line found a prefix or a suffix from its value.
    let english = include_str!("../../item/test-items/en-owner-rare-waystone.txt");
    let french = include_str!("../../item/test-items/fr-rare-waystone.txt");
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
fn a_waystone_is_searched_by_its_properties_and_its_tier() {
    // The owner's waystones of tier 12: the base without its tier, a bar of the tier under it;
    // the properties in a group of their own, the item rarity and the drop chance checked, a
    // percentage from 90 % of its value, rounded down, the revives from their number.
    let english = parse(include_str!(
        "../../item/test-items/en-owner-rare-waystone.txt"
    ));
    let french = parse(include_str!("../../item/test-items/fr-rare-waystone.txt"));
    let properties = |texts: Vec<String>| -> Vec<String> {
        texts
            .into_iter()
            .take_while(|text| !text.starts_with("Prefixes"))
            .collect()
    };

    assert_eq!(
        properties(texts(&english)),
        [
            "Item",
            "■ Base: Waystone [Waystone T12]",
            "□ ItemLevel: 80 ≥80",
            "Waystone",
            "□ Waystone(Revives): 3 ≥3",
            "■ Waystone(ItemRarity): 11 ≥9",
            "□ Waystone(MonsterRarity): 25 ≥22",
            "□ Waystone(MonsterEffectiveness): 13 ≥11",
            "■ Waystone(DropChance): 50 ≥45",
        ]
    );
    assert_eq!(
        properties(texts(&french)),
        [
            "Item",
            "■ Base: Pierre de téléportation [Waystone T12]",
            "□ ItemLevel: 78 ≥78",
            "Waystone",
            "□ Waystone(Revives): 2 ≥2",
            "■ Waystone(ItemRarity): 25 ≥22",
            "□ Waystone(PackSize): 9 ≥8",
            "□ Waystone(MonsterRarity): 23 ≥20",
            "■ Waystone(DropChance): 65 ≥58",
        ]
    );

    // A corrupted waystone raised to tier 16, without revives left: a property of zero
    // searches nothing.
    let corrupted = parse(
        "Item Class: Waystones\nRarity: Normal\nWaystone (Tier 16)\n--------\n\
         Revives Available: 0 (augmented)\nItem Rarity: +38% (augmented)\n--------\n\
         Item Level: 82\n--------\nCorrupted",
    );
    assert_eq!(
        properties(texts(&corrupted)),
        [
            "Item",
            "■ Base: Waystone [Waystone T16]",
            "■ ItemLevel: 82 ≥82",
            "Waystone",
            "■ Waystone(ItemRarity): 38 ≥34",
        ]
    );
}

#[test]
fn a_waystone_searches_its_base_and_its_checked_properties() {
    // From the copied waystone to the query sent: the whole base type, with its tier; the
    // item rarity and the drop chance among the endgame filters; no modifier.
    for (text, base_type, item_rarity, drop_chance) in [
        (
            include_str!("../../item/test-items/en-owner-rare-waystone.txt"),
            "Waystone (Tier 12)",
            9.0,
            45.0,
        ),
        (
            include_str!("../../item/test-items/fr-rare-waystone.txt"),
            "Pierre de téléportation (Palier 12)",
            22.0,
            58.0,
        ),
    ] {
        let item = parse(text);
        let lines = CopiedLines::new(
            item.language(),
            item.modifiers(),
            &LineStats::default(),
            None,
        );
        let line_stats = LineStats::new(item.language(), item.modifiers(), None);
        let tiers = TierRolls::new(
            item.language(),
            item.modifiers(),
            Some(base_type),
            item.item_level(),
        );
        let stats = Stats::new(&item, &lines, &line_stats, &tiers, Some(base_type));
        let target = link::target(&item, None, None).expect("a target");
        let query = |filters: &Filters| match link::query(
            &item,
            &target,
            line_stats.search_filters(&stats.filters(filters)),
        ) {
            crate::trade::query::Query::Items(items) => items.to_json(),
            crate::trade::query::Query::Exchange(_) => panic!("an item search"),
        };
        let mut filters = Filters::default();

        let json = query(&filters);
        assert_eq!(json["query"]["type"], base_type);
        assert_eq!(
            json["query"]["filters"]["map_filters"],
            serde_json::json!({ "filters": {
                "map_iir": { "min": item_rarity },
                "map_bonus": { "min": drop_chance }
            } })
        );
        assert!(json["query"].get("stats").is_none(), "{json}");

        // The base unchecked: any waystone, of any tier.
        let (_, mut base) = stats
            .filters(&filters)
            .into_iter()
            .find(|(key, _)| *key == FilterKey::BaseType)
            .expect("the filter of the base");
        base.checked = false;
        filters.set(FilterKey::BaseType, base);
        let json = query(&filters);
        assert!(json["query"].get("type").is_none(), "{json}");
        assert_eq!(
            json["query"]["filters"]["type_filters"]["filters"]["category"],
            serde_json::json!({ "option": "map.waystone" })
        );
    }
}

#[test]
fn the_filter_of_a_waystone_property_writes_its_value_as_the_game() {
    let english = Translator::new(Language::English);
    let french = Translator::new(Language::French);
    let text = |property, value, i18n| {
        StatText::Property(Property::Waystone(property), value)
            .text(i18n)
            .into_owned()
    };

    assert_eq!(
        text(WaystoneProperty::ItemRarity, 11.0, &english),
        "Item rarity: +11%"
    );
    assert_eq!(
        text(WaystoneProperty::Revives, 3.0, &english),
        "Revives available: 3"
    );
    assert_eq!(
        text(WaystoneProperty::DropChance, 65.0, &french),
        "Chances de trouver des pierres de téléportation\u{a0}: +65%"
    );
    // A zero, as the tooltip of an offer writes a property it lacks, and a negative value,
    // without a sign of their own.
    assert_eq!(
        text(WaystoneProperty::PackSize, 0.0, &english),
        "Pack size: 0%"
    );
    assert_eq!(
        text(WaystoneProperty::ItemRarity, -5.0, &english),
        "Item rarity: -5%"
    );
    assert_eq!(
        text(WaystoneProperty::Revives, 0.0, &english),
        "Revives available: 0"
    );
}

#[test]
fn the_bar_of_the_tier_of_a_waystone_goes_from_tier_1_to_tier_16() {
    assert!(waystone_tier_share(1).abs() < f32::EPSILON);
    assert!((waystone_tier_share(16) - 1.0).abs() < f32::EPSILON);
    assert!((waystone_tier_share(12) - 11.0 / 15.0).abs() < 1e-6);
    // Never out of the bar.
    assert!(waystone_tier_share(0).abs() < f32::EPSILON);
    assert!((waystone_tier_share(17) - 1.0).abs() < f32::EPSILON);
}

#[test]
fn a_tablet_shows_its_modifiers_over_their_ranges() {
    // The owner's magic tablet, its base from the names of the trade site, in both languages.
    let english = parse(include_str!(
        "../../item/test-items/en-owner-magic-tablet.txt"
    ));
    let french = parse(include_str!("../../item/test-items/fr-magic-tablet.txt"));
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
            "■ 9% increased Rarity of Items found in Map ≥8 [0.25 8–12]",
            "■ 15% increased Experience gain in Map ≥14 [0.50 12–18]",
            "Suffixes 2/2",
            "■ 34% increased Quantity of Waystones found in Map ≥31 [0.40 30–40]",
            "■ Map has 85% increased chance to contain Shrines ≥77 [0.50 70–100]",
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
            "■ 9% d'Augmentation de la Rareté des Objets trouvés dans la Carte ≥8 [0.25 8–12]",
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
            "■ 25% d'Augmentation de la Récupération de Mana des Flacons ≥23 [0.57 T3 23–28]",
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
            "■ 10% d'Augmentation de la Récupération de Mana des Flacons ≥9 [0.50 5–15]",
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

/// The rows of `texts` from the first group of prefixes, suffixes or explicit modifiers.
fn explicit_rows(texts: Vec<String>) -> Vec<String> {
    texts
        .into_iter()
        .skip_while(|text| {
            !["Prefixes", "Suffixes", "Explicit"]
                .iter()
                .any(|group| text.starts_with(group))
        })
        .collect()
}

/// The rows of `text`, an advanced copy, and of the same copied from the in-game trade, its
/// tiers found from its values.
fn both(text: &str) -> (Vec<String>, Vec<String>) {
    (
        texts(&parse(text)),
        texts(&read_again(parse(
            &as_in_game_trade(text).expect("a known copy"),
        ))),
    )
}

#[test]
fn the_owners_rare_items_show_the_same_rows_from_the_in_game_trade() {
    // A fractured suffix among modifiers of several tiers: the same rows from the in-game
    // trade, every tier found from its value.
    let mace = [
        "Prefixes 3/3",
        "□ Adds 12 to 26 Physical Damage ≥17 [0.21 T6 11–17 · 20–30]",
        "□ Adds 2 to 6 Cold Damage ≥4 [0.01 T10 2–3 · 4–6]",
        "□ 68% increased Physical Damage ≥61 [0.20 T6 65–84]",
        "Suffixes 3/3",
        "■ Gain 3 Mana per enemy killed ≥3 [0.02 T8 2–3 Fractured]",
        "■ +12% to Critical Damage Bonus ≥11 [0.13 T5 12–13]",
        "■ +3 to Level of all Melee Skills ≥3 [0.33 T3]",
    ];
    let (advanced, in_game_trade) = both(include_str!(
        "../../item/test-items/en-owner-rare-two-hand-mace-fractured.txt"
    ));
    assert_eq!(explicit_rows(advanced), mace);
    assert_eq!(
        explicit_rows(in_game_trade),
        [mace.as_slice(), &["Trade copy note"]].concat()
    );

    // The desecrated modifier of each lord, over its range without tier, in both copies and
    // both languages; the stat of Kurgal written on two lines, one row with its bar.
    for (text, desecrated) in [
        (
            include_str!("../../item/test-items/en-owner-rare-helmet-ulaman.txt"),
            "■ 16% increased Critical Damage Bonus ≥14 [0.43 13–20 Desecrated]",
        ),
        (
            include_str!("../../item/test-items/fr-rare-helmet-ulaman.txt"),
            "■ 16% d'Augmentation du Bonus de Dégâts Critiques ≥14 [0.43 13–20 Desecrated]",
        ),
        (
            include_str!("../../item/test-items/en-owner-rare-focus-kurgal.txt"),
            "■ 23% chance when collecting an Elemental Infusion to gain an / additional \
             Elemental Infusion of the same type ≥21 [0.80 15–25 Desecrated]",
        ),
        (
            include_str!("../../item/test-items/fr-rare-focus-kurgal.txt"),
            "■ Lorsque vous collectez une Infusion élémentaire, vous avez 23% de chances de \
             gagner une / Infusion élémentaire supplémentaire du même type ≥21 [0.80 15–25 \
             Desecrated]",
        ),
        (
            include_str!("../../item/test-items/en-owner-rare-helmet-amanamu.txt"),
            "■ 35% increased Presence Area of Effect ≥32 [1.00 25–35 Desecrated]",
        ),
        (
            include_str!("../../item/test-items/fr-rare-helmet-amanamu.txt"),
            "■ Votre Présence a 35% d'Augmentation de Zone d'effet ≥32 [1.00 25–35 Desecrated]",
        ),
    ] {
        let (advanced, in_game_trade) = both(text);
        assert!(advanced.contains(&desecrated.to_owned()), "{advanced:?}");
        assert!(
            in_game_trade.contains(&desecrated.to_owned()),
            "{in_game_trade:?}"
        );
    }
}

#[test]
fn the_owners_unset_ring_and_helmet_of_amanamu_show_their_rows_from_the_in_game_trade() {
    // The French Unset Ring, rare: its implicit without number, then its modifiers.
    let (advanced, in_game_trade) = both(include_str!(
        "../../item/test-items/fr-rare-ring-implicit.txt"
    ));
    let ring = [
        "Implicit",
        "■ Octroie 1 Emplacement d'Aptitude supplémentaire",
        "Prefixes 2/3",
        "■ +210 au Score de Précision ≥189 [0.45 T3 168–236]",
        "□ +79 de Mana maximal ≥71 [0.41 T7 65–79]",
        "Suffixes 3/3",
        "□ +13% à toutes les Résistances Élémentaires ≥12 [0.77 T2 12–14]",
        "■ 49% d'Augmentation du Taux de Régénération de Mana ≥44 [0.66 T3 40–49]",
        "□ +32% à la Résistance au froid ≥29 [0.67 T3 31–35]",
    ];
    let from_implicit = |texts: Vec<String>| -> Vec<String> {
        texts
            .into_iter()
            .skip_while(|text| text != "Implicit")
            .collect()
    };
    assert_eq!(from_implicit(advanced), ring);
    assert_eq!(
        from_implicit(in_game_trade),
        [ring.as_slice(), &["Trade copy note"]].concat()
    );

    // From the in-game trade, the rarity of the helmet of Amanamu, at the top of a prefix and
    // the bottom of a suffix, with room for both: in doubt.
    let (_, in_game_trade) = both(include_str!(
        "../../item/test-items/en-owner-rare-helmet-amanamu.txt"
    ));
    assert_eq!(
        explicit_rows(in_game_trade),
        [
            "Prefixes 2/3",
            "□ +94 to maximum Mana ≥85 [0.60 T3 90–104]",
            "■ 69% increased Armour and Energy Shield ≥62 [0.64 T3 68–79]",
            "Suffixes 2/3",
            "□ +28% to Cold Resistance ≥25 [0.56 T4 26–30]",
            "■ 35% increased Presence Area of Effect ≥32 [1.00 25–35 Desecrated]",
            "Explicit",
            "■ 15% increased Rarity of Items found ≥14 [T2 12–15 AffixPrefix / T1 15–18 \
             AffixSuffix]",
            "Trade copy note",
        ]
    );
}

#[test]
fn the_owners_normal_items_unique_charm_and_gems_show_their_rows() {
    // A normal quarterstaff: its rune, unchecked, and its implicit over its range, without it
    // from the in-game trade.
    let (advanced, in_game_trade) = both(include_str!(
        "../../item/test-items/fr-normal-quarterstaff-rune.txt"
    ));
    let quarterstaff = |implicit: &str| {
        [
            "Item",
            "■ Base: Bâton de combat d'écraseur de crânes",
            "■ ItemLevel: 80 ≥80",
            "■ Sockets: 1 ≥1",
            "■ Quality: 25 ≥25",
            "Weapon",
            "■ PhysicalDps: 148 ≥132",
            "□ ElementalDps: 19 ≥16",
            "Runes",
            "□ Ajoute 13 à 16 Dégâts de Feu ≥14.5",
            "Implicit",
            implicit,
        ]
        .map(str::to_owned)
    };
    assert_eq!(
        advanced,
        quarterstaff("■ 27% de chances d'Hébéter au Toucher ≥24 [20–50 0.23]")
    );
    assert_eq!(
        in_game_trade,
        quarterstaff("■ 27% de chances d'Hébéter au Toucher ≥24")
    );
    // The implicit of an Unset Ring, without number: searched without limit, in both copies.
    let (advanced, in_game_trade) = both(include_str!(
        "../../item/test-items/en-owner-normal-ring-implicit.txt"
    ));
    assert_eq!(advanced, in_game_trade);
    assert_eq!(
        advanced[advanced.len() - 2..],
        ["Implicit", "■ Grants 1 additional Skill Slot"]
    );

    // A unique charm: its lines of fixed value without filter, its other line over its range;
    // from the in-game trade, unchecked without range until the prices give it, as the other
    // uniques.
    let charm = include_str!("../../item/test-items/en-owner-unique-charm.txt");
    assert_eq!(
        texts(&parse(charm)),
        [
            "Implicit",
            "Used when you become Frozen",
            "Unique",
            "■ 22% Chance to gain a Charge when you kill an enemy ≥22 [20–25 0.40]",
            "Energy Shield Recharge starts on use",
        ]
    );
    assert_eq!(
        texts(&parse(&as_in_game_trade(charm).expect("a known copy"))),
        [
            "Implicit",
            "Used when you become Frozen",
            "Unique",
            "□ 22% Chance to gain a Charge when you kill an enemy ≥22",
            "Energy Shield Recharge starts on use",
        ]
    );

    // An uncut gem, an item of the currency exchange: no filter.
    for text in [
        include_str!("../../item/test-items/en-owner-currency-uncut-gem.txt"),
        include_str!("../../item/test-items/fr-currency-uncut-gem.txt"),
    ] {
        assert_eq!(texts(&parse(text)), Vec::<String>::new(), "{text}");
    }
}

#[test]
fn a_skill_gem_is_searched_by_its_level_quality_and_sockets() {
    // The owner's gem of level 2: its level checked, its 2 sockets, which every gem has, not.
    for text in [
        include_str!("../../item/test-items/en-owner-skill-gem.txt"),
        include_str!("../../item/test-items/fr-skill-gem.txt"),
    ] {
        assert_eq!(
            texts(&parse(text)),
            ["Gem", "■ GemLevel: 2 ≥2", "□ GemSockets: 2 ≥2"],
            "{text}"
        );
    }
    // The quality checked from 16 %, the sockets from 3, the level always.
    let gem = |level: u32, quality: u32, sockets: usize| {
        let sockets = "G ".repeat(sockets);
        parse(&format!(
            "Item Class: Skill Gems\nRarity: Gem\nComet\n--------\nSpell, AoE, Cold\n\
             Level: {level}\nQuality: +{quality}% (augmented)\n--------\nSockets: {sockets}"
        ))
    };
    assert_eq!(
        texts(&gem(15, 12, 3)),
        [
            "Gem",
            "■ GemLevel: 15 ≥15",
            "□ Quality: 12 ≥12",
            "■ GemSockets: 3 ≥3"
        ]
    );
    assert_eq!(
        texts(&gem(20, 16, 2)),
        [
            "Gem",
            "■ GemLevel: 20 ≥20",
            "■ Quality: 16 ≥16",
            "□ GemSockets: 2 ≥2"
        ]
    );
    // The fields within the levels, qualities and sockets of a gem, widened to hold a value
    // beyond them.
    let bounds = |item: &Item| -> Vec<(f64, f64)> {
        rows(item)
            .into_iter()
            .filter_map(|row| match row {
                Row::Stat(stat) => {
                    let bounds = stat.filter?.limit?.bounds?;
                    Some((bounds.min, bounds.max))
                }
                _ => None,
            })
            .collect()
    };
    assert_eq!(
        bounds(&gem(21, 23, 5)),
        [(1.0, 21.0), (0.0, 23.0), (0.0, 5.0)]
    );
    assert_eq!(
        bounds(&gem(22, 25, 6)),
        [(1.0, 22.0), (0.0, 25.0), (0.0, 6.0)]
    );
    // A quality of zero searches nothing: no filter.
    assert_eq!(
        texts(&gem(15, 0, 2)),
        ["Gem", "■ GemLevel: 15 ≥15", "□ GemSockets: 2 ≥2"]
    );
}

#[test]
fn the_search_of_a_skill_gem_sends_its_name_and_its_checked_properties() {
    for (text, name) in [
        (
            include_str!("../../item/test-items/en-owner-skill-gem.txt"),
            "Flame Wall",
        ),
        (
            include_str!("../../item/test-items/fr-skill-gem.txt"),
            "Mur de flammes",
        ),
    ] {
        let item = parse(text);
        let lines = CopiedLines::default();
        let line_stats = LineStats::new(item.language(), item.modifiers(), None);
        let tiers = TierRolls::default();
        let target = link::target(&item, None, None).expect("a target");
        assert_eq!(target, link::TradeTarget::Base { base_type: name });
        let stats = Stats::new(&item, &lines, &line_stats, &tiers, target.base_type());
        let crate::trade::query::Query::Items(query) = link::query(
            &item,
            &target,
            line_stats.search_filters(&stats.filters(&Filters::default())),
        ) else {
            panic!("an item search");
        };
        let json = query.to_json();

        // Its level, checked; its 2 sockets, unchecked, aren't sent; no base type filter.
        assert_eq!(json["query"]["type"], name, "{json}");
        assert_eq!(
            json["query"]["filters"]["misc_filters"],
            serde_json::json!({ "filters": {
                "gem_level": { "min": 2.0 },
                "corrupted": { "option": "false" }
            } }),
            "{json}"
        );
        assert_eq!(
            json["query"]["filters"]["type_filters"],
            serde_json::json!({ "filters": { "rarity": { "option": "nonunique" } } }),
            "{json}"
        );
    }
}

#[test]
fn a_support_gem_without_level_has_no_filter() {
    let support = "Item Class: Support Gems\nRarity: Gem\nFire Penetration I\n--------\n\
                   Support, Fire\n--------\nRequires: Level 5";
    assert_eq!(texts(&parse(support)), Vec::<String>::new());
}
