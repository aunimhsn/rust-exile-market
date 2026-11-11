//! The tables of the game as the embedded file holds them: what the application expects of
//! them, to look at again when the game data are generated again.

use super::*;
use crate::game_data::bases;
use crate::game_data::stats::{Stats, TradeGroup};
use crate::language::Language;

#[test]
fn every_table_of_the_embedded_file_is_read() {
    let text = include_str!("../../../../assets/game-data/modifiers.ndjson");

    assert_eq!(TABLES.len(), text.lines().count());
}

#[test]
fn a_table_rolls_on_the_bases_of_its_domain_only() {
    let charges = TABLES
        .iter()
        .find(|table| table.kind == "FlaskIncreasedMaxCharges")
        .expect("the charges of a flask");
    let base = |name| bases::spawn(Language::English, name).expect("a base");

    // Its only tag, `default`, is a tag of every base.
    assert_eq!(charges.spawn, [("default".to_owned(), 1)]);
    assert!(charges.rolls_on(base("Ultimate Life Flask")));
    assert!(!charges.rolls_on(base("Gold Ring")));
    assert!(!charges.rolls_on(base("Ruby")));

    // A desecrated table rolls on the equipment and the jewels: the presence of Amanamu on
    // the helmets, and a hybrid of the Abyss on the jewels.
    let desecrated = |kind: &str| {
        TABLES
            .iter()
            .find(|table| table.kind == kind && table.domain == Domain::Desecrated)
            .expect(kind)
    };
    let presence = desecrated("PresenceRadius");
    assert!(presence.rolls_on(base("Divine Crown")));
    assert!(!presence.rolls_on(base("Gold Ring")));
    let jewel = desecrated("HybridAbyssModJewelSpellDamageArmour");
    assert!(jewel.rolls_on(base("Ruby")));
    assert!(!jewel.rolls_on(base("Ultimate Life Flask")));
}

#[test]
fn the_tables_are_ordinary_then_of_the_genesis_tree_the_essences_and_the_alloys_then_desecrated() {
    // Of two tables that hold a copied modifier as well, the first is taken: the ordinary one,
    // then the minion one of the Genesis tree; but for a header written without tier, which
    // takes one whose tier the game doesn't write (an essence's).
    let order: Vec<_> = TABLES
        .iter()
        .map(|table| (table.domain == Domain::Desecrated, table.source))
        .collect();

    assert!(order.is_sorted());
    let genesis = Some(TableSource::Genesis);
    let essence = Some(TableSource::Essence);
    let alloy = Some(TableSource::Alloy);
    for kind in [
        (false, genesis),
        (false, essence),
        (false, alloy),
        (true, None),
        (true, genesis),
    ] {
        assert!(order.contains(&kind), "{kind:?}");
    }
}

#[test]
fn no_line_in_doubt_can_be_the_modifier_of_two_lords_of_one_affix() {
    // A line in doubt names the lord of each affix it can be: the modifiers of two lords of one
    // affix that hold it as well name none, but show the range of the first; none on 2026-10-30
    // (the flask charges of Kurgal and Ulaman on the belts are prefixes only). Checked on every
    // pair of lords' tables with the same lines, a tag of a base in common and ranges that meet,
    // whose lines a table of the other affix holds too.
    let tags = |table: &Table| -> Vec<String> {
        table
            .spawn
            .iter()
            .filter(|(_, weight)| *weight > 0)
            .map(|(tag, _)| tag.clone())
            .collect()
    };
    let meet = |one: &[f64; 2], other: &[f64; 2]| {
        one[0].min(one[1]) <= other[0].max(other[1]) && other[0].min(other[1]) <= one[0].max(one[1])
    };
    let overlap = |one: &Table, other: &Table| {
        one.tiers.iter().any(|tier| {
            other.tiers.iter().any(|other| {
                tier.ranges.iter().zip(&other.ranges).all(|(line, other)| {
                    line.iter().zip(other).all(|(one, other)| meet(one, other))
                })
            })
        })
    };
    for (index, one) in TABLES.iter().enumerate() {
        let Some(lord) = one.lord else {
            continue;
        };
        let either = TABLES
            .iter()
            .any(|table| table.affix != one.affix && table.lines == one.lines);
        for other in TABLES.iter().skip(index + 1) {
            let two_lords = other.lord.is_some_and(|other| other != lord)
                && other.affix == one.affix
                && other.lines == one.lines;
            let on_one_base = tags(one).iter().any(|tag| tags(other).contains(tag));
            assert!(
                !(two_lords && on_one_base && overlap(one, other) && either),
                "{} and {}",
                one.kind,
                other.kind
            );
        }
    }
}

#[test]
fn a_line_with_a_divisor_has_one_value() {
    // The divisor is for the value of a line written without number, a single value.
    for table in TABLES.iter() {
        for (index, line) in table.lines.iter().enumerate() {
            if (line.divisor - 1.0).abs() > f64::EPSILON {
                for tier in &table.tiers {
                    assert_eq!(
                        tier.ranges.get(index).map(Vec::len),
                        Some(1),
                        "{}",
                        line.template
                    );
                }
            }
        }
    }
}

#[test]
fn every_template_of_the_tables_is_reached_by_a_french_stat() {
    // A table no French line reaches would leave that line of a French copy without tier. A
    // line is read with the ids of its group ([`TradeGroup::of`]): crafted for an alloy,
    // explicit for the others, also desecrated or fractured.
    let reached = |group| -> std::collections::BTreeSet<&str> {
        Stats::of(Language::French)
            .all()
            .flat_map(|stat| Stats::english_references(stat, group))
            .collect()
    };
    let explicit = reached(TradeGroup::Explicit);
    let crafted = reached(TradeGroup::Crafted);
    let missed: std::collections::BTreeSet<&str> = TABLES
        .iter()
        .flat_map(|table| {
            let reached = if table.source == Some(TableSource::Alloy) {
                &crafted
            } else {
                &explicit
            };
            table
                .lines
                .iter()
                .map(|line| line.template.as_str())
                .filter(|template| !reached.contains(template))
        })
        .collect();

    assert!(missed.is_empty(), "{missed:?}");
}
