//! The lines of a copied modifier, on lines and tiers written by hand: what the tables are
//! searched with, whatever the patch of the game. The lines find their stat in the embedded
//! stats of the game: the templates of these tests must stay among them.

use super::*;
use crate::game_data::bases::Domain;
use crate::game_data::lines::modifier_stats;
use crate::game_data::modifiers::fixtures::{
    LIFE, RARITY, copied, desecrated, modifier, range, table,
};
use crate::game_data::modifiers::tables::TableLord;
use crate::game_data::trade_stats;
use crate::item::{Affix, StatLine};
use crate::language::Language;

/// The template of the added lightning damage, a line of two values.
const LIGHTNING: &str = "Adds # to # Lightning Damage";
/// The template of the bolts of a crossbow, whose string for one bolt has no number.
const BOLT: &str = "Loads an additional bolt";

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
    const RAGE: &str = "Inherent Rage loss starts 1 second later";
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
