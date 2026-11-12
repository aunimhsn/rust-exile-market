//! The lines of a stat the game writes on several lines, joined or kept apart.

use super::*;
use crate::filters::Filters;
use crate::i18n::Translator;
use crate::item::{ModifierOrigin, Range};
use crate::language::Language;
use crate::roll_difference::CopiedLines;
use crate::tier_rolls::TierRolls;
use crate::ui::item_stats::Stats;
use crate::ui::item_stats::fixtures::{parse, rows, texts};
use crate::ui::item_stats::model::{ModifierTier, Origin, TierRow};

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
    let line_stats = LineStats::new(&unique, unique.modifiers(), None);
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
    let affix = |share, tier, origin| StatRoll::Affix {
        share,
        tier,
        origin,
    };
    let bar = |origin| StatRoll::Bar {
        share: 0.5,
        ranges: ranges(),
        origin,
    };
    let desecrated = Some(Origin::Written(ModifierOrigin::Desecrated));
    let vaal = Some(Origin::Written(ModifierOrigin::Vaal));
    let tiers = StatRoll::Tiers {
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
        "../../../item/test-items/en-owner-rare-focus-kurgal.txt"
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
