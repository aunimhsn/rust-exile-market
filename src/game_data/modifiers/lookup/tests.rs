//! The lookup of a copied modifier on tables written by hand: what it does whatever the patch
//! of the game.

use super::*;
use crate::game_data::bases::BaseSpawn;
use crate::game_data::lines::modifier_stats;
use crate::game_data::modifiers::fixtures::{
    copied, copied_line_without_number, copied_lines, equipment, modifier, range,
};
use crate::game_data::modifiers::tables::read;
use crate::item::{Currency, ModifierOrigin};
use crate::language::Language;

/// `tables` as an item of any level sees them on `base`.
fn any_level<'a>(tables: &'a [Table], base: &'a BaseSpawn) -> ItemTables<'a> {
    ItemTables {
        all: tables,
        base,
        item_level: None,
    }
}

#[test]
fn the_value_of_a_line_without_number_is_divided_as_its_table_shows_it() {
    const RAGE: &str = "Inherent Rage loss starts 1 second later";
    let tables = read(concat!(
        r#"{"type": "Rage", "affix": "suffix", "lines": [{"template": "Inherent Rage loss starts 1 second later", "divisor": 1000}], "#,
        r#""spawn": [["weapon", 1]], "tiers": [{"level": 1, "ranges": [[[1, 1]]]}, {"level": 50, "ranges": [[[2, 3]]]}]}"#
    ));
    let number = |copied| {
        find(
            &any_level(&tables, &equipment(&["weapon"])),
            TableAffix::Suffix,
            &copied,
            Tie::Best,
        )
        .and_then(|tiers| Some(tiers.tier()?.number))
    };

    // `Inherent Rage loss starts 1 second later` gives the 1000 milliseconds of the stat.
    let one_second = copied_lines(vec![copied_line_without_number(RAGE, 1000.0)]);
    assert_eq!(number(one_second), Some(2));
    assert_eq!(number(copied(&[(RAGE, 2.5)])), Some(1));
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
        ranges: Some(vec![range(10.0, 20.0)]),
        tier: tier.map_or(CopiedTier::Unknown, CopiedTier::Written),
        // A desecrated line: an ordinary modifier, or a lord's.
        origin: Some(ModifierOrigin::Desecrated),
        ..copied(&[("# to maximum Life", 15.0)])
    };
    let found = |tier| {
        find(
            &any_level(&tables, &equipment(&["helmet"])),
            TableAffix::Prefix,
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
    let copied = copied(&[("B #", 15.0), ("A #", 3.0)]);

    let tiers = find(
        &any_level(&tables, &equipment(&["weapon"])),
        TableAffix::Prefix,
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
    let copied = copied(&[("# to maximum Life", 7.0)]);
    let number = |tie| {
        find(
            &any_level(&tables, &equipment(&["helmet"])),
            TableAffix::Prefix,
            &copied,
            tie,
        )
        .and_then(|tiers| Some(tiers.tier()?.number))
    };

    assert_eq!(number(Tie::Best), Some(1));
    assert_eq!(number(Tie::Worst), Some(2));
}

#[test]
fn only_the_worst_tie_is_bound_by_the_level_of_the_item() {
    // The T1 `5–15` needs the item level 50.
    let tables = read(concat!(
        r##"{"type": "Life", "affix": "prefix", "lines": [{"template": "# to maximum Life"}], "##,
        r#""spawn": [["helmet", 1]], "tiers": [{"level": 1, "ranges": [[[1, 10]]]}, {"level": 50, "ranges": [[[5, 15]]]}]}"#
    ));
    let copied = copied(&[("# to maximum Life", 12.0)]);
    // The number of the tier found, then the number and the level of every tier.
    let found = |tie, item_level| {
        let tables = ItemTables {
            all: &tables,
            base: &equipment(&["helmet"]),
            item_level,
        };
        let tiers = find(&tables, TableAffix::Prefix, &copied, tie)?;
        let all: Vec<(u32, u32)> = tiers
            .tiers
            .iter()
            .map(|tier| (tier.number, tier.level))
            .collect();
        Some((tiers.tier()?.number, all))
    };
    // Every tier of the base, those above the level of the item too: the bar of the rolls
    // shows them all.
    let first_of_two = Some((1, vec![(1, 50), (2, 1)]));

    // The advanced copy wrote the tier: found whatever the level of the item.
    assert_eq!(found(Tie::Best, Some(10)), first_of_two);
    // From its values alone, a tier the item can't roll isn't one of its own.
    assert_eq!(found(Tie::Worst, Some(10)), None);
    assert_eq!(found(Tie::Worst, Some(50)), first_of_two);
    assert_eq!(found(Tie::Worst, None), first_of_two);
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
            &any_level(tables, &equipment(&["bow"])),
            TableAffix::Suffix,
            &copied,
            Tie::Best,
        )
        .map(|tiers| tiers.currency)
    };

    assert_eq!(currency(&essence), Some(Some(Currency::Essence)));
    assert_eq!(currency(&both), Some(None));
}
