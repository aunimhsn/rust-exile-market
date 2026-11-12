//! The lines of a copied modifier, on lines and tiers written by hand: what the tables are
//! searched with, and how well a tier holds it, whatever the patch of the game. The lines find
//! their stat in the embedded stats of the game: the templates of these tests must stay among
//! them.

use super::*;
use crate::game_data::bases::Domain;
use crate::game_data::lines::modifier_stats;
use crate::game_data::modifiers::fixtures::{
    ENERGY_SHIELD, LIFE, MANA, RARITY, copied, desecrated, modifier, range, table, tier,
};
use crate::game_data::modifiers::tables::TableLord;
use crate::game_data::trade_stats;
use crate::item::{Affix, StatLine};
use crate::language::Language;

/// The template of the added lightning damage, a line of two values.
const LIGHTNING: &str = "Adds # to # Lightning Damage";
/// The template of the bolts of a crossbow, whose string for one bolt has no number.
const BOLT: &str = "Loads an additional bolt";
/// The template of the delay of the rage loss, whose string for one second has no number: shown
/// in seconds, stored in milliseconds.
const RAGE: &str = "Inherent Rage loss starts 1 second later";

/// A prefix as the advanced copy writes it: `tier` in its header, and each of `lines` with the
/// ranges of its values (`+25(20-29) to maximum Life`).
fn advanced(tier: u32, lines: &[&str]) -> Modifier {
    Modifier {
        tier: Some(tier),
        lines: lines.iter().map(|line| StatLine::parse(line)).collect(),
        ..modifier(Some(Affix::Prefix), &[])
    }
}

/// The lines of `modifier`, written in English, as the tables are searched with them.
fn copied_of(modifier: &Modifier) -> Option<Copied<'static>> {
    Copied::of(modifier, &modifier_stats(Language::English, modifier, None))
}

/// A copied line of `template` with `values`, one for each `#` of its string.
fn line(template: &'static str, values: &[f64]) -> CopiedLine<'static> {
    CopiedLine {
        templates: vec![template],
        values: values.to_vec(),
        written: None,
    }
}

/// A copied line of `template` without number, whose string gives `value`.
fn line_without_number(template: &'static str, value: f64) -> CopiedLine<'static> {
    CopiedLine {
        templates: vec![template],
        values: Vec::new(),
        written: Some(value),
    }
}

#[test]
fn the_lines_of_an_advanced_copy_give_their_templates_their_values_and_the_ranges_of_their_tier() {
    let hybrid = advanced(
        2,
        &[
            "Adds 10(1-16) to 273(239-300) Lightning Damage",
            "+25(20-29) to maximum Life",
        ],
    );

    assert_eq!(
        copied_of(&hybrid),
        Some(Copied {
            lines: vec![line(LIGHTNING, &[10.0, 273.0]), line(LIFE, &[25.0])],
            // A range for each value of the lines, in their order.
            ranges: Some(vec![
                range(1.0, 16.0),
                range(239.0, 300.0),
                range(20.0, 29.0)
            ]),
            tier: CopiedTier::Written(2),
            origin: None,
        })
    );
}

#[test]
fn the_values_have_the_sign_of_their_stat_and_the_ranges_none_as_the_game_writes_them() {
    let reduced = advanced(1, &["15(10-19)% reduced Rarity of Items found"]);

    assert_eq!(
        copied_of(&reduced),
        Some(Copied {
            ranges: Some(vec![range(10.0, 19.0)]),
            tier: CopiedTier::Written(1),
            ..copied(&[(RARITY, -15.0)])
        })
    );
}

#[test]
fn a_value_without_its_range_leaves_the_modifier_without_ranges() {
    let lightning = |text| copied_of(&advanced(1, &[text]));
    let without_ranges = || {
        Some(Copied {
            lines: vec![line(LIGHTNING, &[10.0, 273.0])],
            tier: CopiedTier::Written(1),
            ..copied(&[])
        })
    };

    // One value of two with its range, either of them.
    assert_eq!(
        lightning("Adds 10(1-16) to 273 Lightning Damage"),
        without_ranges()
    );
    assert_eq!(
        lightning("Adds 10 to 273(239-300) Lightning Damage"),
        without_ranges()
    );
    // None, as a line of the in-game trade.
    assert_eq!(
        lightning("Adds 10 to 273 Lightning Damage"),
        without_ranges()
    );
}

#[test]
fn a_number_the_string_of_a_stat_writes_is_no_value_and_takes_no_range() {
    // The 40 is written by the string of the stat, and has no range in the advanced copy.
    let glory = advanced(
        1,
        &["12(10-15)% chance for Skills to retain 40% of Glory on use"],
    );

    assert_eq!(
        copied_of(&glory),
        Some(Copied {
            ranges: Some(vec![range(10.0, 15.0)]),
            tier: CopiedTier::Written(1),
            ..copied(&[("#% chance for Skills to retain 40% of Glory on use", 12.0)])
        })
    );
}

#[test]
fn a_stat_written_on_several_lines_is_one_line_with_the_values_of_them_all() {
    const ANAEMIA: &str = "Inflict Anaemia on Hit\nAnaemia allows # Corrupted Blood debuffs to be inflicted on enemies";
    // The value of the stat on its second line, then a line of another stat.
    let anaemia = advanced(
        1,
        &[
            "Inflict Anaemia on Hit",
            "Anaemia allows 3(2-4) Corrupted Blood debuffs to be inflicted on enemies",
            "+25(20-29) to maximum Life",
        ],
    );

    assert_eq!(
        copied_of(&anaemia),
        Some(Copied {
            lines: vec![line(ANAEMIA, &[3.0]), line(LIFE, &[25.0])],
            ranges: Some(vec![range(2.0, 4.0), range(20.0, 29.0)]),
            tier: CopiedTier::Written(1),
            origin: None,
        })
    );
}

#[test]
fn a_line_without_number_gives_the_value_its_string_writes_as_the_stat_stores_it() {
    let lines = |text| copied_of(&advanced(1, &[text])).map(|copied| copied.lines);

    assert_eq!(
        lines("Loads an additional bolt"),
        Some(vec![line_without_number(BOLT, 1.0)])
    );
    // A second, stored in milliseconds: its `1` is written by the string.
    assert_eq!(
        lines("Inherent Rage loss starts 1 second later"),
        Some(vec![line_without_number(RAGE, 1000.0)])
    );
    // The same stats with their number: lines as any other.
    assert_eq!(
        lines("Loads 2(2-3) additional bolts"),
        Some(vec![line(BOLT, &[2.0])])
    );
    assert_eq!(
        lines("Inherent Rage loss starts 2.5(2-3) seconds later"),
        Some(vec![line(RAGE, &[2.5])])
    );
}

#[test]
fn a_line_with_a_number_takes_no_value_from_its_string() {
    // `# Charm Slot` is the string of one slot, which gives 1 too: the number alone is read.
    let slot = modifier(Some(Affix::Suffix), &["+1 Charm Slot"]);

    assert_eq!(
        copied_of(&slot).map(|copied| copied.lines),
        Some(vec![line("# Charm Slot", &[1.0])])
    );
}

#[test]
fn a_line_without_number_leaves_the_modifier_without_ranges() {
    // The advanced copy writes a range for the life, none for the bolt.
    let hybrid = advanced(
        1,
        &["+25(20-29) to maximum Life", "Loads an additional bolt"],
    );

    assert_eq!(
        copied_of(&hybrid),
        Some(Copied {
            lines: vec![line(LIFE, &[25.0]), line_without_number(BOLT, 1.0)],
            ranges: None,
            tier: CopiedTier::Written(1),
            origin: None,
        })
    );
}

#[test]
fn a_line_without_a_stat_of_the_game_data_gives_no_copied_modifier() {
    let trade = trade_stats();
    // Where the stat of each line was found, and the copied modifier.
    let read = |lines: &[&str]| {
        let modifier = modifier(None, lines);
        let matches = modifier_stats(Language::English, &modifier, Some(&trade));
        let sources: Vec<Option<StatSource>> = matches
            .iter()
            .map(|found| found.stat.as_ref().map(|stat| stat.source))
            .collect();
        (sources, Copied::of(&modifier, &matches))
    };
    let game_data = Some(StatSource::GameData);

    // The life, which the trade site knows too: read from the game data.
    assert_eq!(
        read(&["+25 to maximum Life"]),
        (vec![game_data], Some(copied(&[(LIFE, 25.0)])))
    );
    // Then a line no stat writes.
    assert_eq!(
        read(&["+25 to maximum Life", "Not a line of the game"]),
        (vec![game_data, None], None)
    );
    // Or one the trade site alone knows: no table has its template.
    assert_eq!(
        read(&["+25 to maximum Life", "12% reduced Damage per Patch"]),
        (vec![game_data, Some(StatSource::TradeSite)], None)
    );
}

#[test]
fn the_tier_and_the_origin_are_those_the_header_of_the_modifier_writes() {
    let life = &["+25 to maximum Life"];
    let read = |tier, origin| {
        Some(Copied {
            tier,
            origin,
            ..copied(&[(LIFE, 25.0)])
        })
    };
    let third = Modifier {
        tier: Some(3),
        ..modifier(Some(Affix::Prefix), life)
    };
    let fractured = Modifier {
        origin: Some(ModifierOrigin::Fractured),
        ..modifier(None, life)
    };

    assert_eq!(copied_of(&third), read(CopiedTier::Written(3), None));
    // A header without tier, as the one of an essence.
    assert_eq!(
        copied_of(&modifier(Some(Affix::Suffix), life)),
        read(CopiedTier::Unwritten, None)
    );
    assert_eq!(
        copied_of(&desecrated(Affix::Prefix, life)),
        read(CopiedTier::Unwritten, Some(ModifierOrigin::Desecrated))
    );
    // No header: a line of the in-game trade, with its marker or without.
    assert_eq!(
        copied_of(&modifier(None, life)),
        read(CopiedTier::Unknown, None)
    );
    assert_eq!(
        copied_of(&fractured),
        read(CopiedTier::Unknown, Some(ModifierOrigin::Fractured))
    );
}

#[test]
fn a_line_counts_its_values_or_the_one_its_string_writes() {
    let lines = Copied {
        lines: vec![
            line(LIGHTNING, &[10.0, 273.0]),
            line_without_number(BOLT, 1.0),
            // A line without number whose string gives no value.
            line("Trigger Lightning Bolt Skill on Critical Hit", &[]),
            line(LIFE, &[25.0]),
        ],
        ..copied(&[])
    };
    let values_before: Vec<usize> = (0..=4).map(|line| lines.values_before(line)).collect();

    assert_eq!(lines.counts(), [2, 1, 0, 1]);
    assert_eq!(values_before, [0, 2, 3, 3, 4]);
}

#[test]
fn a_copied_modifier_may_be_one_of_the_tables_its_origin_allows() {
    let lord = Table {
        domain: Domain::Desecrated,
        lord: Some(TableLord::Kurgal),
        ..table("LordLife", "prefix", &[LIFE], &[(1, &[(1, 30)])])
    };
    let marked = |origin| Copied {
        origin,
        ..copied(&[(LIFE, 25.0)])
    };

    assert!(marked(Some(ModifierOrigin::Desecrated)).may_be(&lord));
    assert!(!marked(None).may_be(&lord));
}

/// The lines of a table of `templates`, each shown as its stat stores it.
fn table_lines(templates: &[&str]) -> Vec<TableLine> {
    templates
        .iter()
        .map(|&template| TableLine {
            template: template.to_owned(),
            divisor: 1.0,
        })
        .collect()
}

#[test]
fn the_order_of_copied_lines_gives_for_each_its_line_of_the_table() {
    let copied = copied(&[(LIFE, 25.0), (ENERGY_SHIELD, 15.0), (MANA, 30.0)]);
    let order = |templates: &[&str]| copied.order(&table_lines(templates));

    assert_eq!(order(&[LIFE, ENERGY_SHIELD, MANA]), Some(vec![0, 1, 2]));
    // The same modifier, its lines in another order: the life is the third line of the table.
    assert_eq!(order(&[ENERGY_SHIELD, MANA, LIFE]), Some(vec![2, 0, 1]));
}

#[test]
fn copied_lines_that_are_not_those_of_a_table_have_no_order_in_it() {
    let hybrid = copied(&[(LIFE, 25.0), (ENERGY_SHIELD, 15.0)]);
    let order = |templates: &[&str]| hybrid.order(&table_lines(templates));

    // Another line, a line more, a line fewer.
    assert_eq!(order(&[LIFE, MANA]), None);
    assert_eq!(order(&[LIFE, ENERGY_SHIELD, MANA]), None);
    assert_eq!(order(&[LIFE]), None);
    // A line alone isn't a hybrid that has it.
    let life = copied(&[(LIFE, 25.0)]);
    assert_eq!(life.order(&table_lines(&[LIFE, ENERGY_SHIELD])), None);
}

#[test]
fn each_line_of_a_table_is_the_line_of_one_copied_line_only() {
    let twice = copied(&[(LIFE, 25.0), (LIFE, 30.0)]);
    let order = |templates: &[&str]| twice.order(&table_lines(templates));

    assert_eq!(order(&[LIFE, MANA]), None);
    assert_eq!(order(&[LIFE, LIFE]), Some(vec![0, 1]));
}

#[test]
fn a_line_of_two_english_templates_is_the_line_of_a_table_of_either() {
    // As a line of another language that two English stats write.
    let either = CopiedLine {
        templates: vec![LIFE, MANA],
        ..line(LIFE, &[25.0])
    };
    assert!(either.is(LIFE));
    assert!(either.is(MANA));
    assert!(!either.is(ENERGY_SHIELD));

    let copied = Copied {
        lines: vec![either],
        ..copied(&[])
    };
    assert_eq!(copied.order(&table_lines(&[MANA])), Some(vec![0]));
    assert_eq!(copied.order(&table_lines(&[ENERGY_SHIELD])), None);
}

#[test]
fn the_values_of_the_copied_lines_are_given_in_their_order() {
    let copied = Copied {
        lines: vec![line(LIFE, &[25.0]), line(LIGHTNING, &[10.0, 273.0])],
        ..copied(&[])
    };
    // The table writes the lightning damage first.
    let lines = table_lines(&[LIGHTNING, LIFE]);

    assert_eq!(copied.values(&lines, &[1, 0]), [25.0, 10.0, 273.0]);
}

#[test]
fn the_value_of_a_line_without_number_is_divided_as_its_own_line_of_the_table_shows_it() {
    // The table writes the rage first, shown in seconds; the life is shown as stored.
    let lines = vec![
        TableLine {
            template: RAGE.to_owned(),
            divisor: 1000.0,
        },
        TableLine {
            template: LIFE.to_owned(),
            divisor: 1.0,
        },
    ];
    let values = |rage| {
        let copied = Copied {
            lines: vec![line(LIFE, &[25.0]), rage],
            ..copied(&[])
        };
        copied.values(&lines, &[1, 0])
    };

    // One second, which its string gives as 1000 milliseconds.
    assert_eq!(values(line_without_number(RAGE, 1000.0)), [25.0, 1.0]);
    // Written with its number, the line reads as the table shows it: not divided.
    assert_eq!(values(line(RAGE, &[2.5])), [25.0, 2.5]);
}

#[test]
fn ranges_hold_values_within_them_or_give_or_take_the_rounding_of_the_game() {
    let ranges = [range(10.0, 20.0), range(2.1, 3.5)];
    let fit = |first, second| holds(&[first, second], &ranges);

    // Within, the bounds too.
    assert_eq!(fit(15.0, 3.0), Some(Fit::Within));
    assert_eq!(fit(10.0, 3.5), Some(Fit::Within));
    assert_eq!(fit(20.0, 2.1), Some(Fit::Within));
    // One value out of its range by the rounding, at either end.
    assert_eq!(fit(15.0, 3.55), Some(Fit::Rounded));
    assert_eq!(fit(15.0, 2.05), Some(Fit::Rounded));
    assert_eq!(fit(9.95, 3.0), Some(Fit::Rounded));
    // By more.
    assert_eq!(fit(15.0, 3.56), None);
    assert_eq!(fit(15.0, 2.04), None);
    assert_eq!(fit(20.06, 3.0), None);
}

#[test]
fn a_tier_that_holds_the_values_is_better_than_one_that_rounds_them_then_one_they_left() {
    assert!(Fit::Within < Fit::Rounded);
    assert!(Fit::Rounded < Fit::Written);
}

#[test]
fn without_the_ranges_of_the_copy_a_tier_fits_when_it_holds_the_values() {
    let life = copied(&[(LIFE, 25.0)]);
    let tier = tier(1, 1, &[(20.0, 29.0)]);
    let fit = |value| life.fit(&[value], &tier);

    assert_eq!(fit(25.0), Some(Fit::Within));
    assert_eq!(fit(29.05), Some(Fit::Rounded));
    assert_eq!(fit(31.0), None);
}

#[test]
fn with_the_ranges_of_the_copy_only_a_tier_of_those_ranges_fits() {
    // The advanced copy wrote `(10-19)` and `(20-29)`.
    let hybrid = Copied {
        ranges: Some(vec![range(10.0, 19.0), range(20.0, 29.0)]),
        ..copied(&[(ENERGY_SHIELD, 15.0), (LIFE, 25.0)])
    };
    let fit = |life: f64, ranges: &[(f64, f64)]| hybrid.fit(&[15.0, life], &tier(1, 1, ranges));
    let written = &[(10.0, 19.0), (20.0, 29.0)];

    assert_eq!(fit(25.0, written), Some(Fit::Within));
    assert_eq!(fit(29.05, written), Some(Fit::Rounded));
    // A value that left its tier, as on a sanctified item: still the tier the copy wrote.
    assert_eq!(fit(31.0, written), Some(Fit::Written));
    // A tier that holds the values over other ranges, by one bound of one line: another tier.
    assert_eq!(fit(25.0, &[(10.0, 19.0), (20.0, 30.0)]), None);
    assert_eq!(fit(25.0, &[(10.0, 19.0), (19.0, 29.0)]), None);
    assert_eq!(fit(25.0, &[(10.0, 20.0), (20.0, 29.0)]), None);
    assert_eq!(fit(25.0, &[(9.0, 19.0), (20.0, 29.0)]), None);
}

#[test]
fn the_ranges_of_the_copy_are_those_of_a_tier_give_or_take_the_rounding_and_without_their_sign() {
    let fit = |copy: (f64, f64), value: f64, ranges: (f64, f64)| {
        let copied = Copied {
            ranges: Some(vec![range(copy.0, copy.1)]),
            ..copied(&[(LIFE, value)])
        };
        copied.fit(&[value], &tier(1, 1, &[ranges]))
    };

    // A tier of 2.08 to 3.54, which the game writes with one decimal.
    assert_eq!(fit((2.1, 3.5), 3.0, (2.08, 3.54)), Some(Fit::Within));
    assert_eq!(fit((2.1, 3.5), 3.0, (2.04, 3.54)), None);
    assert_eq!(fit((2.1, 3.5), 3.0, (2.08, 3.56)), None);
    // A stat that rolls negative values, `15(10-19)% reduced`: the tier from -19 to -10.
    assert_eq!(fit((10.0, 19.0), -15.0, (-19.0, -10.0)), Some(Fit::Within));
    assert_eq!(fit((10.0, 19.0), -15.0, (-20.0, -10.0)), None);
    // A tier across zero has no range without sign: none is known, and it is no tier of an
    // advanced copy.
    assert_eq!(fit((0.0, 5.0), 3.0, (-5.0, 5.0)), None);
}

#[test]
fn a_tier_without_a_range_for_each_value_never_fits() {
    let life = copied(&[(LIFE, 25.0)]);
    let fit = |values: &[f64], ranges: &[(f64, f64)]| life.fit(values, &tier(1, 1, ranges));

    assert_eq!(fit(&[25.0], &[(20.0, 29.0)]), Some(Fit::Within));
    assert_eq!(fit(&[25.0], &[(20.0, 29.0), (1.0, 5.0)]), None);
    assert_eq!(fit(&[25.0, 3.0], &[(20.0, 29.0)]), None);
    assert_eq!(fit(&[], &[(20.0, 29.0)]), None);
}
