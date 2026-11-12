//! The tiers under a line that adds up two modifiers, and under the other lines of its hybrid.

use super::*;
use crate::item::TierDoubt;
use crate::ui::item_stats::fixtures::{parse, part, read_again, texts, texts_from};

#[test]
fn a_line_that_adds_up_two_modifiers_shows_the_tier_of_each_and_counts_both() {
    let bow = read_again(parse(include_str!(
        "../../../item/test-items/en-owner-trade-rare-bow-mirrored.txt"
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
            "□ +195 to Accuracy Rating ≥176 [0.97 T1 175–200 Hybrid (with 75–79 of 258% increased Physical Damage)]",
            "□ Adds 36 to 56 Physical Damage ≥41 [0.88 T1 26–39 · 44–66 Desecrated]",
        ]
    );
}

#[test]
fn the_other_line_of_a_hybrid_whose_table_isnt_found_keeps_its_tier_without_bar() {
    let mut bow = read_again(parse(include_str!(
        "../../../item/test-items/en-owner-trade-rare-bow-mirrored.txt"
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
            "□ +195 to Accuracy Rating ≥176 [T1 175–200 Hybrid (with 76–79 of 258% increased Physical Damage)]",
        ]
    );
}

#[test]
fn a_part_of_a_summed_line_of_one_tier_only_writes_no_tier_number() {
    let mut bow = read_again(parse(include_str!(
        "../../../item/test-items/en-owner-trade-rare-bow-mirrored.txt"
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
    let bow = include_str!("../../../item/test-items/en-owner-trade-rare-bow-mirrored.txt");
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
    let tiara = include_str!("../../../item/test-items/en-owner-trade-rare-helmet-fractured.txt");
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
            "□ 37% increased Rarity of Items found ≥33 [T1 16–19 AffixPrefix / T1 15–18 AffixSuffix AddedUpWithPrefix / Desecrated]",
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
            "□ 35% increased Armour and Evasion ≥32 [0.81 T2 33–38 HybridPrefix (with 64–94 of +310 to Stun Threshold)]",
            // The prefix first, "Hybrid prefix" naming the other line; the suffix added up.
            "□ +310 to Stun Threshold ≥279 [T2 64–94 HybridPrefix (with 35% increased Armour and Evasion) / T2 207–253 AffixSuffix AddedUpWithPrefix]",
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
            "□ +50 to Armour ≥45 [0.96]",
            "□ +41 to Evasion Rating ≥37 [0.81 T1 42–52 · 40–50 Hybrid (with 39–42 of 115% increased Armour and Evasion)]",
            // Both other lines named.
            "□ 115% increased Armour and Evasion ≥104 [T4 68–79 / T1 39–42 Hybrid (with +50 to Armour and +41 to Evasion Rating)]",
        ]
    );
}

#[test]
fn a_pure_prefix_added_up_with_a_hybrid_suffix_says_hybrid_suffix_on_both_its_lines() {
    let bow = include_str!("../../../item/test-items/en-owner-trade-rare-bow-mirrored.txt");
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

    let accuracy = "□ +600 to Accuracy Rating ≥540 [T1/T2 451–650 AffixPrefix / T1 41–60 HybridSuffix (with 15% increased Light Radius)]";
    // A fixed value, without range under it.
    let radius = "□ 15% increased Light Radius ≥14 [1.00 T1 HybridSuffix (with 41–60 of +600 to Accuracy Rating)]";
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
    let reversed = include_str!("../../../item/test-items/en-owner-trade-rare-bow-mirrored.txt");
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
            "□ +50 to Armour ≥45 [0.96]",
            "□ 115% increased Armour and Evasion ≥104 [T4 68–79 / T1 39–42 Hybrid (with +50 to Armour and +41 to Evasion Rating)]",
            "□ +41 to Evasion Rating ≥37 [0.81 T1 42–52 · 40–50 Hybrid (with 39–42 of 115% increased Armour and Evasion)]",
        ]
    );
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
fn the_part_of_an_alloy_in_a_summed_line_names_its_currency() {
    let bow = include_str!("../../../item/test-items/en-owner-trade-rare-bow-mirrored.txt");
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
        texts.contains(&"□ +800 to Accuracy Rating ≥720 [T2/T3 347–550 / 327–427 Hybrid (with 6% increased Attack Speed) Alloy]".to_owned()),
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
