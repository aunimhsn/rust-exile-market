//! The ways a modifier sits on the lines and the pruning of a tier past its value, on lines
//! and tables written by hand.

use super::*;
use crate::game_data::bases::Domain;
use crate::game_data::modifiers::fixtures::range;
use crate::game_data::modifiers::other_readings::fixtures::{line, placement};
use crate::game_data::modifiers::tables::{TableAffix, TableLine};
use crate::item::Affix;

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
    let (rising, falling) = directions(&placements, 3);
    assert_eq!(rising, [true, false, false]);
    assert_eq!(falling, [false, false, true]);

    let values = [15.0, 0.0, -15.0];
    let is_past = |index, sum| past(&rising, &falling, &values, index, sum);
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
    let table = |templates: &[&str]| Table {
        kind: "Kind".to_owned(),
        affix: TableAffix::Prefix,
        domain: Domain::Item,
        lines: templates
            .iter()
            .map(|template| TableLine {
                template: (*template).to_owned(),
                divisor: 1.0,
            })
            .collect(),
        spawn: Vec::new(),
        source: None,
        lord: None,
        tiers: Vec::new(),
    };
    let fractured = Some(ModifierOrigin::Fractured);
    let lines = [
        line(0, "+# to Armour", 0..1, None),
        line(1, "#% increased Armour", 1..2, None),
        line(2, "+# to Armour", 2..3, fractured),
        line(3, "#% increased Armour", 3..4, fractured),
    ];

    // Each pair of lines of one origin, the lines in their order.
    let hybrid = table(&["#% increased Armour", "+# to Armour"]);
    assert_eq!(mappings(&hybrid, &lines), [vec![0, 1], vec![2, 3]]);
    // A pure modifier on each line of its template.
    let pure = table(&["+# to Armour"]);
    assert_eq!(mappings(&pure, &lines), [vec![0], vec![2]]);
    // A template the item doesn't have.
    assert_eq!(
        mappings(&table(&["+# to maximum Life"]), &lines),
        [] as [Vec<usize>; 0]
    );
}
