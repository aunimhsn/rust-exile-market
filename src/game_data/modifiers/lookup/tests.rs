//! The lookup of a copied modifier on tables written by hand: what it does whatever the patch
//! of the game.

use super::*;
use crate::game_data::lines::modifier_stats;
use crate::game_data::modifiers::fixtures::{equipment, modifier, range};
use crate::game_data::modifiers::tables::read;
use crate::item::{Currency, ModifierOrigin};

#[test]
fn the_value_of_a_line_without_number_is_divided_as_its_table_shows_it() {
    let tables = read(concat!(
        r#"{"type": "Rage", "affix": "suffix", "lines": [{"template": "Inherent Rage loss starts 1 second later", "divisor": 1000}], "#,
        r#""spawn": [["weapon", 1]], "tiers": [{"level": 1, "ranges": [[[1, 1]]]}, {"level": 50, "ranges": [[[2, 3]]]}]}"#
    ));
    let copied = |values: Vec<f64>, written| Copied {
        lines: vec![CopiedLine {
            templates: vec!["Inherent Rage loss starts 1 second later"],
            values,
            written,
        }],
        ranges: None,
        tier: CopiedTier::Unknown,
        origin: None,
    };
    let number = |copied| {
        find(
            &tables,
            TableAffix::Suffix,
            &equipment(&["weapon"]),
            &copied,
            Tie::Best,
        )
        .and_then(|tiers| Some(tiers.tier()?.number))
    };

    // `Inherent Rage loss starts 1 second later` gives the 1000 milliseconds of the stat.
    assert_eq!(number(copied(Vec::new(), Some(1000.0))), Some(2));
    assert_eq!(number(copied(vec![2.5], None)), Some(1));
}

#[test]
fn of_two_tiers_as_good_the_one_the_advanced_copy_wrote_is_taken() {
    // A desecrated table of one tier first, then an ordinary one whose T3 has the same range.
    let tables = read(concat!(
        r##"{"type": "Lord", "affix": "prefix", "domain": "desecrated", "lines": [{"template": "# to maximum Life"}], "##,
        r#""spawn": [["helmet", 1]], "tiers": [{"level": 65, "ranges": [[[10, 20]]]}]}"#,
        "\n",
        r##"{"type": "Life", "affix": "prefix", "lines": [{"template": "# to maximum Life"}], "##,
        r#""spawn": [["helmet", 1]], "tiers": [{"level": 1, "ranges": [[[10, 20]]]}, {"level": 30, "ranges": [[[21, 30]]]}, {"level": 60, "ranges": [[[31, 40]]]}]}"#
    ));
    let copied = |tier: Option<u32>| Copied {
        lines: vec![CopiedLine {
            templates: vec!["# to maximum Life"],
            values: vec![15.0],
            written: None,
        }],
        ranges: Some(vec![range(10.0, 20.0)]),
        tier: tier.map_or(CopiedTier::Unknown, CopiedTier::Written),
        // A desecrated line: an ordinary modifier, or a lord's.
        origin: Some(ModifierOrigin::Desecrated),
    };
    let found = |tier| {
        find(
            &tables,
            TableAffix::Prefix,
            &equipment(&["helmet"]),
            &copied(tier),
            Tie::Best,
        )
        .and_then(|tiers| Some((tiers.single(), tiers.tier()?.number)))
    };

    // A T3 isn't hidden by the modifier of one tier before it.
    assert_eq!(found(Some(3)), Some((false, 3)));
    assert_eq!(found(Some(1)), Some((true, 1)));
    // Without the number, the first of the file.
    assert_eq!(found(None), Some((true, 1)));
}

#[test]
fn two_tables_with_the_same_lines_in_another_order_are_two_modifiers() {
    let table = |kind: &str, lines: [&str; 2], level: u32| {
        format!(
            r#"{{"type": "{kind}", "affix": "prefix", "lines": [{{"template": "{}"}}, {{"template": "{}"}}], "spawn": [["weapon", 1]], "tiers": [{{"level": {level}, "ranges": [[[1, 5]], [[10, 20]]]}}]}}"#,
            lines[0], lines[1]
        )
    };
    let tables = read(
        &[
            table("Hybrid", ["A #", "B #"], 1),
            table("Hybrid", ["B #", "A #"], 50),
        ]
        .join("\n"),
    );
    let copied = Copied {
        lines: vec![
            CopiedLine {
                templates: vec!["B #"],
                values: vec![15.0],
                written: None,
            },
            CopiedLine {
                templates: vec!["A #"],
                values: vec![3.0],
                written: None,
            },
        ],
        ranges: None,
        tier: CopiedTier::Unknown,
        origin: None,
    };

    let tiers = find(
        &tables,
        TableAffix::Prefix,
        &equipment(&["weapon"]),
        &copied,
        Tie::Best,
    )
    .expect("its tiers");

    // The first table holds the copied values, its lines put in the copied order; the
    // second, its own lines in the copied order, holds neither 15 in 1–5 nor 3 in 10–20.
    assert_eq!(tiers.tiers.len(), 1);
    assert_eq!(tiers.tiers.first().map(|tier| tier.level), Some(1));
    assert_eq!(
        tiers.tier().map(|tier| tier.ranges.clone()),
        Some(vec![range(10.0, 20.0), range(1.0, 5.0)])
    );
}

#[test]
fn a_value_in_several_tiers_takes_the_worst_without_the_ranges_of_the_copy() {
    let tables = read(concat!(
        r##"{"type": "Life", "affix": "prefix", "lines": [{"template": "# to maximum Life"}], "##,
        r#""spawn": [["helmet", 1]], "tiers": [{"level": 1, "ranges": [[[1, 10]]]}, {"level": 50, "ranges": [[[5, 15]]]}]}"#
    ));
    let copied = Copied {
        lines: vec![CopiedLine {
            templates: vec!["# to maximum Life"],
            values: vec![7.0],
            written: None,
        }],
        ranges: None,
        tier: CopiedTier::Unknown,
        origin: None,
    };
    let number = |tie| {
        find(
            &tables,
            TableAffix::Prefix,
            &equipment(&["helmet"]),
            &copied,
            tie,
        )
        .and_then(|tiers| Some(tiers.tier()?.number))
    };

    assert_eq!(number(Tie::Best), Some(1));
    assert_eq!(number(Tie::Worst { item_level: None }), Some(2));
}

#[test]
fn a_line_an_essence_and_an_alloy_hold_as_well_names_no_currency() {
    let table = |source: &str| {
        format!(
            r##"{{"type": "Speed{source}", "affix": "suffix", "lines": [{{"template": "#% increased Attack Speed"}}], "spawn": [["bow", 1]], "source": "{source}", "tiers": [{{"level": 1, "ranges": [[[5, 10]]]}}]}}"##
        )
    };
    let essence = read(&table("essence"));
    let both = read(&format!("{}\n{}", table("essence"), table("alloy")));
    // Crafted, as the game writes the modifier of an alloy.
    let suffix = Modifier {
        origin: Some(ModifierOrigin::Crafted),
        ..modifier(Some(Affix::Suffix), &["8% increased Attack Speed"])
    };
    let matches = modifier_stats(Language::English, &suffix, None);
    let copied = Copied::of(&suffix, &matches).expect("copied");
    let currency = |tables: &[Table]| {
        find(
            tables,
            TableAffix::Suffix,
            &equipment(&["bow"]),
            &copied,
            Tie::Best,
        )
        .map(|tiers| tiers.currency)
    };

    assert_eq!(currency(&essence), Some(Some(Currency::Essence)));
    assert_eq!(currency(&both), Some(None));
}
