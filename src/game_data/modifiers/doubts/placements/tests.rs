//! The ways a modifier sits on the lines, the tiers the level of the item allows and the
//! pruning of a tier past its value, on lines and tables written by hand.

use super::*;
use crate::game_data::modifiers::doubts::fixtures::{line, placement};
use crate::game_data::modifiers::fixtures::{LIFE, copied_line, equipment, range, table};
use crate::item::Affix;

#[test]
fn only_the_tiers_the_item_level_allows_are_placed() {
    // Two tiers that both hold 12, the best one from the level 50.
    let tables = [table(
        "Life",
        "prefix",
        &[LIFE],
        &[(1, &[(1, 15)]), (50, &[(5, 20)])],
    )];
    let lines = ItemLines {
        all: vec![Line {
            copied: copied_line(LIFE, &[12.0]),
            ..line(0, LIFE, 0..1, None)
        }],
        values: vec![12.0],
        modifier_count: 1,
    };
    // The number and the range of each tier placed.
    let placed = |item_level| {
        let tables = ItemTables {
            all: &tables,
            base: &equipment(&["helmet"]),
            item_level,
        };
        let placements = placements(&tables, &lines);
        let tiers = placements.iter().map(|placement| &placement.shown);
        tiers
            .map(|shown| (shown.tier, shown.ranges.clone()))
            .collect::<Vec<_>>()
    };
    let (best, worst) = ((1, vec![range(5.0, 20.0)]), (2, vec![range(1.0, 15.0)]));

    assert_eq!(placed(Some(50)), [best.clone(), worst.clone()]);
    assert_eq!(placed(Some(49)), std::slice::from_ref(&worst));
    // A copy that doesn't write its level: an item of any level.
    assert_eq!(placed(None), [best, worst]);
}

#[test]
fn a_tier_past_its_value_is_left_out_only_when_every_range_rises_or_falls() {
    let placements = [
        placement(
            "Life",
            Affix::Prefix,
            &[0],
            &[(0, range(10.0, 20.0))],
            false,
        ),
        placement(
            "Mixed",
            Affix::Prefix,
            &[1],
            &[(1, range(-5.0, 3.0))],
            false,
        ),
        placement(
            "Reduced",
            Affix::Prefix,
            &[2],
            &[(2, range(-20.0, -10.0))],
            false,
        ),
    ];
    let directions = Directions::of(&placements, 3);
    assert_eq!(directions.rising, [true, false, false]);
    assert_eq!(directions.falling, [false, false, true]);

    let values = [15.0, 0.0, -15.0];
    let is_past = |index, sum| directions.is_past(&values, index, sum);
    // A sum that only rises, already above its value.
    assert!(is_past(0, range(16.0, 30.0)));
    assert!(!is_past(0, range(15.03, 30.0)));
    // Ranges of both signs: another modifier may still bring the sum back.
    assert!(!is_past(1, range(9.0, 12.0)));
    // A sum that only falls, already below its value.
    assert!(is_past(2, range(-30.0, -16.0)));
    assert!(!is_past(2, range(-30.0, -14.0)));
    // A value out of the item.
    assert!(is_past(5, range(0.0, 0.0)));
}

#[test]
fn a_modifier_sits_on_lines_of_its_templates_of_one_origin() {
    let table_of = |templates: &[&str]| table("Kind", "prefix", templates, &[]);
    let fractured = Some(ModifierOrigin::Fractured);
    let lines = [
        line(0, "+# to Armour", 0..1, None),
        line(1, "#% increased Armour", 1..2, None),
        line(2, "+# to Armour", 2..3, fractured),
        line(3, "#% increased Armour", 3..4, fractured),
    ];

    // Each pair of lines of one origin, the lines in their order.
    let hybrid = table_of(&["#% increased Armour", "+# to Armour"]);
    assert_eq!(mappings(&hybrid, &lines), [vec![0, 1], vec![2, 3]]);
    // A pure modifier on each line of its template.
    let pure = table_of(&["+# to Armour"]);
    assert_eq!(mappings(&pure, &lines), [vec![0], vec![2]]);
    // A template the item doesn't have.
    assert_eq!(
        mappings(&table_of(&["+# to maximum Life"]), &lines),
        [] as [Vec<usize>; 0]
    );
}
