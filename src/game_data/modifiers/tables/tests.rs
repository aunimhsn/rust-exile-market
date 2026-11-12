//! The modifier tables on tables written by hand: what a table rolls on, what it may hold,
//! which tables are one modifier and what of a file is read, whatever the patch of the game.

use super::*;
use crate::game_data::modifiers::fixtures::{LIFE, MANA, equipment, table, table_json};

/// The life as a prefix, in one tier: an ordinary table, which rolls on the helmets.
fn life() -> Table {
    table("Life", "prefix", &[LIFE], &[(1, &[(1, 10)])])
}

/// A table of the same type, affix and lines as [`life`], desecrated: a lord's.
fn desecrated_life() -> Table {
    Table {
        domain: Domain::Desecrated,
        ..life()
    }
}

/// The `tags` with their weights, as the list of a table.
fn spawn(tags: &[(&str, u32)]) -> Vec<(String, u32)> {
    tags.iter()
        .map(|&(tag, weight)| (tag.to_owned(), weight))
        .collect()
}

#[test]
fn the_first_tag_of_its_list_a_base_has_tells_whether_a_table_rolls_on_it() {
    let table = Table {
        spawn: spawn(&[("ring", 0), ("armour", 1), ("default", 0)]),
        ..life()
    };
    let rolls_on = |tags: &[&str]| table.rolls_on(&equipment(tags));

    // The first of the list of the table, whatever the order of the tags of the base: `armour`
    // before `default`, and `ring` before `armour`.
    assert!(rolls_on(&["armour", "default"]));
    assert!(rolls_on(&["default", "armour"]));
    assert!(!rolls_on(&["armour", "ring"]));
    assert!(!rolls_on(&["default", "armour", "ring"]));
    assert!(!rolls_on(&["default"]));
    // No tag of the list.
    assert!(!rolls_on(&["amulet"]));
    assert!(!rolls_on(&[]));
}

#[test]
fn a_table_rolls_on_the_bases_of_its_domain_a_desecrated_one_on_the_equipment_and_the_jewels() {
    // The domains a base can be of: none is desecrated.
    const BASES: [Domain; 5] = [
        Domain::Item,
        Domain::Flask,
        Domain::Misc,
        Domain::Area,
        Domain::Tablet,
    ];
    // The domains of the bases a table of `domain` rolls on, each base with the tag of the
    // table.
    let bases = |domain| -> Vec<Domain> {
        let table = Table { domain, ..life() };
        BASES
            .into_iter()
            .filter(|&base| {
                table.rolls_on(&BaseSpawn {
                    domain: base,
                    tags: vec!["helmet".to_owned()],
                })
            })
            .collect()
    };

    for domain in BASES {
        assert_eq!(bases(domain), [domain]);
    }
    assert_eq!(bases(Domain::Desecrated), [Domain::Item, Domain::Misc]);

    // A base of its domain without a tag of its list: the domain alone isn't enough.
    let jewel = BaseSpawn {
        domain: Domain::Misc,
        tags: vec!["jewel".to_owned()],
    };
    assert!(!desecrated_life().rolls_on(&jewel));
}

#[test]
fn a_table_may_hold_a_modifier_by_the_origin_the_game_marks() {
    let of_source = |source| Table {
        source: Some(source),
        ..life()
    };
    let tables = [
        ("ordinary", life()),
        ("genesis", of_source(TableSource::Genesis)),
        ("essence", of_source(TableSource::Essence)),
        ("alloy", of_source(TableSource::Alloy)),
        ("desecrated", desecrated_life()),
    ];
    let holding = |origin| -> Vec<&str> {
        tables
            .iter()
            .filter(|(_, table)| table.may_hold(origin))
            .map(|&(name, _)| name)
            .collect()
    };

    // An unmarked line is never one of an alloy, which the game marks crafted, nor a
    // desecrated one.
    assert_eq!(holding(None), ["ordinary", "genesis", "essence"]);
    // A marked one may be of any table but a desecrated one.
    for marked in [ModifierOrigin::Crafted, ModifierOrigin::Fractured] {
        assert_eq!(
            holding(Some(marked)),
            ["ordinary", "genesis", "essence", "alloy"]
        );
    }
    // A desecration reveals ordinary modifiers too, never one of a currency.
    assert_eq!(
        holding(Some(ModifierOrigin::Desecrated)),
        ["ordinary", "genesis", "desecrated"]
    );
}

#[test]
fn the_source_of_a_table_tells_its_currency_and_whether_the_game_writes_its_tier() {
    let told = |source| {
        let table = Table { source, ..life() };
        (table.tier_unwritten(), table.currency())
    };

    assert_eq!(told(None), (false, None));
    assert_eq!(told(Some(TableSource::Genesis)), (true, None));
    assert_eq!(
        told(Some(TableSource::Essence)),
        (true, Some(Currency::Essence))
    );
    assert_eq!(
        told(Some(TableSource::Alloy)),
        (true, Some(Currency::Alloy))
    );
}

#[test]
fn tables_of_one_source_type_affix_and_lines_are_tiers_of_one_modifier() {
    let line = |template: &str| TableLine {
        template: template.to_owned(),
        divisor: 1.0,
    };
    let hybrid = |templates: [&str; 2]| Table {
        lines: templates.into_iter().map(line).collect(),
        ..life()
    };
    let same = |one: &Table, other: &Table| (one.same_modifier(other), other.same_modifier(one));

    // On other bases, with other tiers.
    let on_the_rings = Table {
        spawn: spawn(&[("ring", 1)]),
        tiers: Vec::new(),
        ..life()
    };
    assert_eq!(same(&life(), &on_the_rings), (true, true));

    let others = [
        (
            "type",
            Table {
                kind: "Mana".to_owned(),
                ..life()
            },
        ),
        (
            "affix",
            Table {
                affix: TableAffix::Suffix,
                ..life()
            },
        ),
        (
            "source",
            Table {
                source: Some(TableSource::Essence),
                ..life()
            },
        ),
        (
            "line",
            Table {
                lines: vec![line(MANA)],
                ..life()
            },
        ),
        // Its line shown in another unit: the copied values are divided as the lines of the
        // first table of a modifier tell.
        (
            "divisor",
            Table {
                lines: vec![TableLine {
                    divisor: 1000.0,
                    ..line(LIFE)
                }],
                ..life()
            },
        ),
        ("lines", hybrid([LIFE, MANA])),
    ];
    for (difference, other) in &others {
        assert_eq!(same(&life(), other), (false, false), "{difference}");
    }
    // The same lines in another order are another modifier.
    assert_eq!(
        same(&hybrid([LIFE, MANA]), &hybrid([MANA, LIFE])),
        (false, false)
    );
    assert_eq!(
        same(&hybrid([LIFE, MANA]), &hybrid([LIFE, MANA])),
        (true, true)
    );
}

#[test]
fn a_desecrated_table_is_a_modifier_of_its_own() {
    // Two tables of one type, affix and lines, as two lords have them on one base.
    let tables = [desecrated_life(), desecrated_life(), life()];
    let same = |one: usize, other: usize| tables[one].same_modifier(&tables[other]);

    assert!(same(0, 0));
    assert!(!same(0, 1));
    assert!(!same(1, 0));
    // Nor the ordinary modifier with its lines, either way.
    assert!(!same(0, 2));
    assert!(!same(2, 0));
}

#[test]
fn a_modifier_is_looked_at_once_at_its_first_table() {
    let tables = [
        life(),
        Table {
            kind: "Mana".to_owned(),
            ..life()
        },
        // The life again, on other bases: the modifier of the first table.
        Table {
            spawn: spawn(&[("ring", 1)]),
            ..life()
        },
        desecrated_life(),
        desecrated_life(),
    ];

    // Past the last table too: no table, no modifier.
    let firsts: Vec<usize> = (0..=tables.len())
        .filter(|&index| first_of_its_modifier(&tables, index))
        .collect();

    assert_eq!(firsts, [0, 1, 3, 4]);
}

#[test]
fn a_table_written_with_its_type_affix_lines_tags_and_tiers_alone_is_an_ordinary_one() {
    let table = life();

    assert_eq!(table.domain, Domain::Item);
    assert_eq!(table.source, None);
    assert_eq!(table.lord, None);
    // Its values shown as the stat stores them.
    let divisors: Vec<f64> = table.lines.iter().map(|line| line.divisor).collect();
    assert_eq!(divisors, [1.0]);
}

#[test]
fn a_table_written_with_its_domain_source_lord_and_divisor_is_read_with_them() {
    let line_with = |domain: &str, source: &str, lord: &str| {
        format!(
            r#"{{"type": "Lord", "affix": "suffix", "domain": "{domain}", "lines": [{{"template": "{LIFE}", "divisor": 1000}}], "spawn": [["helmet", 1]], "source": "{source}", "lord": "{lord}", "tiers": [{{"level": 65, "ranges": [[[1, 2]]]}}]}}"#
        )
    };
    let text = [
        line_with("desecrated", "genesis", "Amanamu"),
        line_with("flask", "essence", "Kurgal"),
        line_with("misc", "alloy", "Ulaman"),
    ]
    .join("\n");

    let read: Vec<_> = read(&text)
        .iter()
        .map(|table| {
            let divisors: Vec<f64> = table.lines.iter().map(|line| line.divisor).collect();
            (table.domain, table.source, table.lord, divisors)
        })
        .collect();

    let thousand = vec![1000.0];
    assert_eq!(
        read,
        [
            (
                Domain::Desecrated,
                Some(TableSource::Genesis),
                Some(TableLord::Amanamu),
                thousand.clone()
            ),
            (
                Domain::Flask,
                Some(TableSource::Essence),
                Some(TableLord::Kurgal),
                thousand.clone()
            ),
            (
                Domain::Misc,
                Some(TableSource::Alloy),
                Some(TableLord::Ulaman),
                thousand
            ),
        ]
    );
}

#[test]
fn a_table_names_its_affix_and_its_lord_as_an_item_does_the_prefix_first() {
    assert_eq!(
        [TableAffix::Prefix, TableAffix::Suffix].map(Affix::from),
        [Affix::Prefix, Affix::Suffix]
    );
    assert_eq!(
        [TableLord::Amanamu, TableLord::Kurgal, TableLord::Ulaman].map(Lord::from),
        [Lord::Amanamu, Lord::Kurgal, Lord::Ulaman]
    );
    // The order the two pure modifiers of a split line are read in.
    assert!(TableAffix::Prefix < TableAffix::Suffix);
}

#[test]
fn a_line_of_the_file_that_cant_be_read_is_left_out() {
    let life = table_json("Life", "prefix", &[LIFE], &[(1, &[(1, 10)])]);
    let mana = table_json("Mana", "prefix", &[MANA], &[(1, &[(1, 10)])]);
    // A field the application doesn't know, in the table, in a line and in a tier: the
    // generator and the application would no longer describe the same file.
    let renamed = [
        life.replacen(r#""type""#, r#""weight": 1, "type""#, 1),
        life.replacen(r#""template""#, r#""id": 1, "template""#, 1),
        life.replacen(r#""level""#, r#""name": "Hale", "level""#, 1),
    ];
    for line in &renamed {
        assert_ne!(line, &life);
    }
    let [in_table, in_line, in_tier] = renamed;
    let text = [
        life,
        in_table,
        in_line,
        "not a table".to_owned(),
        in_tier,
        mana,
    ]
    .join("\n");

    let kinds: Vec<String> = read(&text).into_iter().map(|table| table.kind).collect();

    assert_eq!(kinds, ["Life", "Mana"]);
}
