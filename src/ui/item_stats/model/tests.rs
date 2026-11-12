//! The text of the stats and the words of the tiers: the word of the origin of a modifier, the
//! lines of a Time-Lost jewel, the word after the ranges of a tier, the width of a range field
//! and its minus sign.

use super::*;
use crate::language::Language;
use crate::ui::item_stats::fixtures::{parse, read_again, texts, texts_from};

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
fn a_time_lost_jewel_shows_its_lines_over_their_ranges_in_both_copies() {
    // The owner's Time-Lost Sapphire, every modifier of one tier: its bar and its field over
    // the range of that tier, nothing under its fixed `1%`; each line without the words of
    // its radius, its tag after its range, alone under the fixed line; the same from the
    // in-game trade.
    let text = include_str!("../../../item/test-items/en-owner-rare-jewel-time-lost.txt");
    let lines: Vec<&str> = text.lines().filter(|line| !line.starts_with('{')).collect();
    let in_game_trade = read_again(parse(&lines.join("\n")));
    let modifiers = [
        "Prefixes 2/2",
        "□ 2% increased Elemental Damage ≥2 [1.00 1–2] <Small>",
        "□ 7% faster start of Energy Shield Recharge ≥6 [1.00 5–7] <Notable>",
        "Suffixes 2/2",
        "□ 1% increased Mana Regeneration Rate ≥1 [0.00 1–2] <Small>",
        "□ 1% of Damage taken Recouped as Life <Notable>",
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
        "../../../item/test-items/fr-rare-jewel-time-lost.txt"
    ));
    let large = parse(include_str!(
        "../../../item/test-items/fr-rare-jewel-time-lost-large.txt"
    ));

    assert_eq!(
        texts_from(&jewel, "Prefixes"),
        [
            "Prefixes 2/2",
            "□ 2% d'Augmentation des Dégâts élémentaires ≥2 [1.00 1–2] <Small>",
            "□ Le Démarrage de la Recharge du Bouclier d'énergie est 7% plus rapide ≥6 [1.00 5–7] <Notable>",
            "Suffixes 2/2",
            "□ 1% d'Augmentation du Taux de Régénération de Mana ≥1 [0.00 1–2] <Small>",
            "□ 1% des Dégâts subis sont Recouvrés sous forme de Vie <Notable>",
        ]
    );
    assert_eq!(
        texts_from(&large, "Prefixes"),
        [
            "Prefixes 2/2",
            "□ 2% d'Augmentation des Dégâts des Sorts ≥2 [1.00 1–2] <Small>",
            "□ Améliore le Rayon en Grand [T1]",
            "Suffixes 2/2",
            "□ Vous gagnez du Seuil d'Étourdissement équivalent à 2% de votre Bouclier d'énergie maximal ≥2 [1.00 1–2] <Small>",
            "□ 2% d'Augmentation de l'Amplitude de l'Inflammabilité ≥2 [0.00 2–3] <Small>",
        ]
    );
}

#[test]
fn every_line_of_a_time_lost_jewel_in_the_game_data_is_shown_short() {
    // Each matcher of a stat granted to the passive skills in a radius, in both languages, so
    // that game data written again with other words fails here rather than shows long lines.
    for data in [
        include_str!("../../../../assets/game-data/stats-en.ndjson"),
        include_str!("../../../../assets/game-data/stats-fr.ndjson"),
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
    let words = |row: &TierRow, i18n: &Translator| tier_word(row, i18n);
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
    assert!(t2.is_tier_in_word);
    assert_eq!(words(&ranged, &english).as_deref(), Some("Prefix"));
    assert!(!ranged.is_tier_in_word);
}

#[test]
fn a_range_field_takes_a_minus_sign_when_its_value_or_its_range_is_negative() {
    let line = StatLine::parse("-15% to Fire Resistance");
    let allows_minus_sign =
        |value, bounds| Limit::new(value, Half::Minimum, bounds, &line).allows_minus_sign();

    assert!(allows_minus_sign(-15.0, None));
    assert!(!allows_minus_sign(15.0, None));
    // A range across zero.
    assert!(allows_minus_sign(
        2.0,
        Some(Range {
            min: -5.0,
            max: 10.0
        })
    ));
}

#[test]
fn a_range_field_holds_four_characters_a_side_when_its_stat_reaches_them() {
    let line = StatLine::parse("+2250 Armour if you've Blocked Recently");
    let wide = |value, min, max| {
        Limit::new(value, Half::Minimum, Some(Range { min, max }), &line).is_wide()
    };

    assert!(!wide(245.0, 200.0, 320.0));
    assert!(!wide(-15.0, -20.0, -10.0));
    assert!(wide(2250.0, 1500.0, 3000.0));
    // A decimal point or a minus sign counts.
    assert!(wide(16.3, 15.0, 20.0));
    assert!(wide(-20.5, -25.0, -10.0));
    // Without a range, e.g. a rare at 90 % before the modifier tables: its value.
    assert!(
        Limit::new(7.59, Half::Minimum, None, &line)
            .at(6.83)
            .is_wide()
    );
    assert!(
        !Limit::new(181.0, Half::Minimum, None, &line)
            .at(163.0)
            .is_wide()
    );
}
