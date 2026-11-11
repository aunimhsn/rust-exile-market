use super::fixtures::{BASES, modifier, tables};
use super::*;

#[test]
fn the_tiers_of_a_type_on_the_same_bases_make_one_table_sorted_by_level() {
    let weapon = r#"[{"tag": "weapon", "weight": 1}, {"tag": "default", "weight": 0}]"#;
    let ring = r#"[{"tag": "ring", "weight": 1}, {"tag": "default", "weight": 0}]"#;
    let speed = |min: u32, max: u32| {
        format!(r#"[{{"id": "attack_speed_+%", "min": {min}, "max": {max}}}]"#)
    };
    let modifiers = format!(
        r#"{{"Speed2": {}, "Speed1": {}, "RingSpeed1": {}}}"#,
        modifier("Speed", 20, weapon, &speed(8, 10)),
        modifier("Speed", 1, weapon, &speed(5, 7)),
        modifier("Speed", 1, ring, &speed(3, 4)),
    );
    let translations = r#"[{"ids": ["attack_speed_+%"], "English": [{"string": "{0}% increased Attack Speed", "index_handlers": [[]]}], "hidden": null}]"#;

    let (tables, _) = tables(
        &modifiers,
        BASES,
        translations,
        &[("#% increased Attack Speed", "#% increased Attack Speed")],
    );

    assert_eq!(
        tables,
        // In the order of their spawn weights.
        serde_json::json!([
            {"type": "Speed", "affix": "suffix", "lines": [{"template": "#% increased Attack Speed"}],
             "spawn": [["ring", 1], ["default", 0]],
             "tiers": [{"level": 1, "ranges": [[[3, 4]]]}]},
            {"type": "Speed", "affix": "suffix", "lines": [{"template": "#% increased Attack Speed"}],
             "spawn": [["weapon", 1], ["default", 0]],
             "tiers": [{"level": 1, "ranges": [[[5, 7]]]}, {"level": 20, "ranges": [[[8, 10]]]}]}
        ])
    );
}

#[test]
fn a_table_rolling_on_no_released_base_is_left_out_with_the_tags_of_no_base() {
    let modifiers = format!(
        r#"{{"Fishing": {}, "Ring": {}}}"#,
        modifier(
            "Fishing",
            1,
            r#"[{"tag": "fishing_rod", "weight": 1}, {"tag": "default", "weight": 0}]"#,
            r#"[{"id": "fish_quantity_+%", "min": 5, "max": 10}]"#
        ),
        modifier(
            "Life",
            1,
            r#"[{"tag": "amulet", "weight": 1}, {"tag": "ring", "weight": 1}, {"tag": "default", "weight": 0}]"#,
            r#"[{"id": "base_maximum_life", "min": 5, "max": 10}]"#
        ),
    );
    let translations = r#"[
        {"ids": ["fish_quantity_+%"], "English": [{"string": "{0}% increased Quantity of Fish Caught", "index_handlers": [[]]}], "hidden": null},
        {"ids": ["base_maximum_life"], "English": [{"string": "{0:+d} to maximum Life", "index_handlers": [[]]}], "hidden": null}
    ]"#;

    let (tables, tags) = tables(
        &modifiers,
        BASES,
        translations,
        &[("# to maximum Life", "# to maximum Life")],
    );

    let types: Vec<_> = tables
        .as_array()
        .expect("tables")
        .iter()
        .map(|table| (table["type"].clone(), table["spawn"].clone()))
        .collect();
    assert_eq!(
        types,
        [(
            serde_json::json!("Life"),
            serde_json::json!([["ring", 1], ["default", 0]])
        )]
    );
    // The tags of the tables only, of the released bases only, with their domain.
    let spawn = |domain: &str, tags: &[&str]| BaseSpawn {
        domain: domain.to_owned(),
        tags: tags.iter().map(|&tag| tag.to_owned()).collect(),
    };
    assert_eq!(tags.get("Ring"), Some(&spawn("item", &["ring", "default"])));
    assert_eq!(tags.get("Bow"), Some(&spawn("item", &["default"])));
    assert_eq!(tags.get("Flask"), Some(&spawn("flask", &["default"])));
    assert_eq!(tags.get("Old"), None);
}

#[test]
fn a_name_of_several_bases_with_other_tags_gets_none() {
    let bases = r#"{
        "StrHelmet": {"domain": "item", "name": "Random Helmet", "release_state": "released", "tags": ["str_armour", "default"]},
        "IntHelmet": {"domain": "item", "name": "Random Helmet", "release_state": "released", "tags": ["int_armour", "default"]},
        "Cap": {"domain": "item", "name": "Cap", "release_state": "released", "tags": ["int_armour", "default"]}
    }"#;
    let modifiers = format!(
        r#"{{"Shield": {}}}"#,
        modifier(
            "EnergyShield",
            1,
            r#"[{"tag": "int_armour", "weight": 1}, {"tag": "default", "weight": 0}]"#,
            r#"[{"id": "local_energy_shield", "min": 5, "max": 10}]"#
        ),
    );
    let translations = r#"[{"ids": ["local_energy_shield"], "English": [{"string": "{0:+d} to maximum Energy Shield", "index_handlers": [[]]}], "hidden": null}]"#;

    let (_, tags) = tables(
        &modifiers,
        bases,
        translations,
        &[("# to maximum Energy Shield", "# to maximum Energy Shield")],
    );

    let tags_of = |name: &str| tags.get(name).map(|spawn| spawn.tags.clone());
    assert_eq!(tags_of("Random Helmet"), Some(Vec::new()));
    assert_eq!(
        tags_of("Cap"),
        Some(vec!["int_armour".to_owned(), "default".to_owned()])
    );
}

#[test]
fn a_table_keeps_its_domain_when_it_isnt_that_of_the_equipment() {
    let charges = |domain: &str| {
        modifier(
            "Charges",
            1,
            r#"[{"tag": "default", "weight": 1}]"#,
            r#"[{"id": "charges_gained_+%", "min": 23, "max": 30}]"#,
        )
        .replace(r#""domain": "item""#, &format!(r#""domain": "{domain}""#))
    };
    let modifiers = format!(
        r#"{{"FlaskCharges": {}, "ItemCharges": {}}}"#,
        charges("flask"),
        charges("item"),
    );
    let translations = r#"[{"ids": ["charges_gained_+%"], "English": [{"string": "{0}% increased Charges gained", "index_handlers": [[]]}], "hidden": null}]"#;

    let (tables, _) = tables(
        &modifiers,
        BASES,
        translations,
        &[("#% increased Charges gained", "#% increased Charges gained")],
    );

    let domains: Vec<_> = tables
        .as_array()
        .expect("tables")
        .iter()
        .map(|table| table.get("domain").cloned())
        .collect();
    // One table per domain, the domain of the equipment unwritten.
    assert_eq!(domains, [Some(serde_json::json!("flask")), None]);
}

#[test]
fn a_desecrated_table_rolls_on_the_equipment_and_the_jewels_after_the_ordinary_ones() {
    let bases = r#"{
        "Ring": {"domain": "item", "name": "Ring", "release_state": "released", "tags": ["ring", "default"]},
        "Ruby": {"domain": "misc", "name": "Ruby", "release_state": "released", "tags": ["strjewel", "default"]},
        "Waystone": {"domain": "area", "name": "Waystone", "release_state": "released", "tags": ["map", "default"]}
    }"#;
    let desecrated = |spawn: &str| {
        modifier(
            "Leech",
            65,
            spawn,
            r#"[{"id": "life_leech_+%", "min": 12, "max": 20}]"#,
        )
        .replace(r#""domain": "item""#, r#""domain": "desecrated""#)
    };
    let modifiers = format!(
        // The tag of a lord comes after `default`, on no base.
        r#"{{"Abyss": {}, "Lightless": {}, "MapAbyss": {}, "Ordinary": {}}}"#,
        desecrated(
            r#"[{"tag": "ring", "weight": 1}, {"tag": "default", "weight": 0}, {"tag": "kurgal_mod", "weight": 1}]"#
        ),
        // A tag of a lord at 0 names no lord.
        desecrated(
            r#"[{"tag": "strjewel", "weight": 1}, {"tag": "default", "weight": 0}, {"tag": "ulaman_mod", "weight": 0}]"#
        ),
        desecrated(r#"[{"tag": "map", "weight": 1}, {"tag": "default", "weight": 0}]"#),
        modifier(
            "Leech",
            1,
            r#"[{"tag": "ring", "weight": 1}, {"tag": "default", "weight": 0}]"#,
            r#"[{"id": "life_leech_+%", "min": 5, "max": 10}]"#
        ),
    );
    let translations = r#"[{"ids": ["life_leech_+%"], "English": [{"string": "{0}% increased amount of Life Leeched", "index_handlers": [[]]}], "hidden": null}]"#;

    let (tables, tags) = tables(
        &modifiers,
        bases,
        translations,
        &[(
            "#% increased amount of Life Leeched",
            "#% increased amount of Life Leeched",
        )],
    );

    let spawns: Vec<_> = tables
        .as_array()
        .expect("tables")
        .iter()
        .map(|table| (table.get("domain").cloned(), table["spawn"].clone()))
        .collect();
    let lords: Vec<_> = tables
        .as_array()
        .expect("tables")
        .iter()
        .map(|table| table.get("lord").cloned())
        .collect();
    // The name of the lord, of the first table only.
    assert_eq!(lords, [None, Some(serde_json::json!("Kurgal")), None]);
    let desecrated = Some(serde_json::json!("desecrated"));
    // Without the tag of the lord, nor the table of the waystones, a base of another
    // domain.
    assert_eq!(
        spawns,
        [
            (None, serde_json::json!([["ring", 1], ["default", 0]])),
            (
                desecrated.clone(),
                serde_json::json!([["ring", 1], ["default", 0]])
            ),
            (
                desecrated,
                serde_json::json!([["strjewel", 1], ["default", 0]])
            ),
        ]
    );
    assert_eq!(
        tags.get("Ruby").map(|spawn| spawn.tags.clone()),
        Some(vec!["strjewel".to_owned(), "default".to_owned()])
    );
}

#[test]
fn a_modifier_of_the_genesis_tree_rolls_on_the_rings_the_amulets_and_the_belts_it_grows() {
    let bases = r#"{
        "Ring": {"domain": "item", "name": "Ring", "release_state": "released", "tags": ["ring", "default"]},
        "Amulet": {"domain": "item", "name": "Amulet", "release_state": "released", "tags": ["amulet", "default"]},
        "Belt": {"domain": "item", "name": "Belt", "release_state": "released", "tags": ["belt", "default"]}
    }"#;
    let life = |level: u32, min: u32, max: u32, spawn: &str| {
        modifier(
            "MinionLife",
            level,
            spawn,
            &format!(r#"[{{"id": "minion_maximum_life_+%", "min": {min}, "max": {max}}}]"#),
        )
    };
    let tree = r#"[{"tag": "genesis_tree_minion", "weight": 1}, {"tag": "default", "weight": 0}]"#;
    let modifiers = format!(
        r#"{{"MinionLifeRing2": {}, "MinionLifeRing1": {}, "Amulet": {}, "Exposure": {}, "Sceptre": {}}}"#,
        life(29, 11, 14, tree),
        life(18, 7, 10, tree),
        // Ruled out of the rings and the belts before the tag of the tree.
        modifier(
            "MinionLife",
            1,
            r#"[{"tag": "ring", "weight": 0}, {"tag": "belt", "weight": 0}, {"tag": "genesis_tree_minion", "weight": 1}, {"tag": "default", "weight": 0}]"#,
            r#"[{"id": "minion_maximum_life_+%", "min": 1, "max": 2}]"#
        ),
        modifier(
            "Exposure",
            1,
            r#"[{"tag": "amulet", "weight": 0}, {"tag": "belt", "weight": 0}, {"tag": "breach_desecration", "weight": 1}, {"tag": "default", "weight": 0}]"#,
            r#"[{"id": "exposure_effect_+%", "min": 25, "max": 35}]"#
        )
        .replace(r#""domain": "item""#, r#""domain": "desecrated""#),
        // An ordinary one, with a tag of the tree at 0 that decides nothing.
        life(
            2,
            21,
            25,
            r#"[{"tag": "genesis_tree_minion", "weight": 0}, {"tag": "ring", "weight": 1}, {"tag": "default", "weight": 0}]"#
        ),
    );
    let translations = r#"[
        {"ids": ["minion_maximum_life_+%"], "English": [{"string": "Minions have {0}% increased maximum Life", "index_handlers": [[]]}], "hidden": null},
        {"ids": ["exposure_effect_+%"], "English": [{"string": "{0}% increased Exposure Effect", "index_handlers": [[]]}], "hidden": null}
    ]"#;

    let (tables, tags) = tables(
        &modifiers,
        bases,
        translations,
        &[
            (
                "Minions have #% increased maximum Life",
                "Minions have #% increased maximum Life",
            ),
            (
                "#% increased Exposure Effect",
                "#% increased Exposure Effect",
            ),
        ],
    );

    let summary: Vec<_> = tables
        .as_array()
        .expect("tables")
        .iter()
        .map(|table| {
            let levels: Vec<_> = table["tiers"]
                .as_array()
                .expect("tiers")
                .iter()
                .map(|tier| tier["level"].clone())
                .collect();
            serde_json::json!([table["type"], table["spawn"], table.get("source"), levels])
        })
        .collect();
    // The ordinary table first, then those of the tree, the desecrated one last; the tiers
    // of the tree on the same bases one table.
    assert_eq!(
        summary,
        [
            serde_json::json!(["MinionLife", [["ring", 1], ["default", 0]], null, [2]]),
            serde_json::json!([
                "MinionLife",
                [["ring", 0], ["belt", 0], ["amulet", 1], ["default", 0]],
                "genesis",
                [1]
            ]),
            serde_json::json!([
                "MinionLife",
                [["ring", 1], ["amulet", 1], ["belt", 1], ["default", 0]],
                "genesis",
                [18, 29]
            ]),
            serde_json::json!([
                "Exposure",
                [["amulet", 0], ["belt", 0], ["ring", 1], ["default", 0]],
                "genesis",
                [1]
            ]),
        ]
    );
    assert_eq!(
        tags.get("Belt").map(|spawn| spawn.tags.clone()),
        Some(vec!["belt".to_owned(), "default".to_owned()])
    );
}

#[test]
fn the_tags_of_the_genesis_tree_give_way_to_the_bases_it_grows_the_first_tag_deciding() {
    let spawn = |tags: &[(&str, u32)]| -> Vec<SpawnWeight> {
        tags.iter()
            .map(|&(tag, weight)| SpawnWeight {
                tag: tag.to_owned(),
                weight,
            })
            .collect()
    };

    // A tag of a base after the tag of the tree: the tree's weight, written first, decides;
    // the second tag of the tree adds no base twice.
    assert_eq!(
        spawn_on_bases(&spawn(&[
            ("breach_desecration", 1),
            ("ring", 0),
            ("genesis_tree_minion", 2),
            ("default", 0),
        ])),
        (
            spawn(&[
                ("ring", 1),
                ("amulet", 1),
                ("belt", 1),
                ("ring", 0),
                ("default", 0),
            ]),
            true
        )
    );
    // A tag of the tree at 0 left out, the others as they are.
    let ordinary = spawn(&[("genesis_tree_minion", 0), ("ring", 1), ("default", 0)]);
    assert_eq!(
        spawn_on_bases(&ordinary),
        (spawn(&[("ring", 1), ("default", 0)]), false)
    );
}

#[test]
fn only_the_prefixes_and_suffixes_that_roll_are_kept_without_their_hidden_stats() {
    let life = r#"[{"id": "base_maximum_life", "min": 5, "max": 10}, {"id": "hidden_stat", "min": 1, "max": 1}]"#;
    let ring = r#"[{"tag": "ring", "weight": 1}, {"tag": "default", "weight": 0}]"#;
    let kept = modifier("Life", 1, ring, life);
    let other = |field: &str, value: &str| {
        let text = kept.replace(field, value);
        assert_ne!(text, kept);
        text
    };
    let modifiers = format!(
        r#"{{"Kept": {kept}, "Monster": {}, "Unique": {}, "Essence": {}, "Nowhere": {}}}"#,
        other(r#""domain": "item""#, r#""domain": "monster""#),
        other(
            r#""generation_type": "suffix""#,
            r#""generation_type": "unique""#
        ),
        other(r#""is_essence_only": false"#, r#""is_essence_only": true"#),
        other(
            r#"{"tag": "ring", "weight": 1}"#,
            r#"{"tag": "ring", "weight": 0}"#
        ),
    );
    let translations = r#"[
        {"ids": ["base_maximum_life"], "English": [{"string": "{0:+d} to maximum Life", "index_handlers": [[]]}], "hidden": null},
        {"ids": ["hidden_stat"], "English": [{"string": "Hidden {0}", "index_handlers": [[]]}], "hidden": true}
    ]"#;

    let (tables, _) = tables(
        &modifiers,
        BASES,
        translations,
        &[("# to maximum Life", "# to maximum Life")],
    );

    assert_eq!(
        tables,
        serde_json::json!([
            {"type": "Life", "affix": "suffix", "lines": [{"template": "# to maximum Life"}],
             "spawn": [["ring", 1], ["default", 0]],
             "tiers": [{"level": 1, "ranges": [[[5, 10]]]}]}
        ])
    );
}

#[test]
fn a_modifier_with_a_line_without_template_is_left_out() {
    let modifiers = format!(
        r#"{{"Hook": {}}}"#,
        modifier(
            "Hook",
            1,
            r#"[{"tag": "weapon", "weight": 1}]"#,
            r#"[{"id": "fishing_hook_type", "min": 1, "max": 2}]"#
        )
    );
    let translations = r#"[{"ids": ["fishing_hook_type"], "English": [
        {"string": "Karui Stone Hook", "index_handlers": [[]]}
    ], "hidden": null}]"#;

    let (tables, _) = tables(&modifiers, BASES, translations, &[]);

    assert_eq!(tables, serde_json::json!([]));
}

#[test]
fn a_modifier_with_a_stat_that_rolls_without_translation_is_left_out() {
    let ring = r#"[{"tag": "ring", "weight": 1}]"#;
    let modifiers = format!(
        r#"{{"Hybrid": {}, "Flag": {}}}"#,
        modifier(
            "MinionAreaAndLife",
            1,
            ring,
            r#"[{"id": "minion_area_of_effect_+%", "min": 3, "max": 6}, {"id": "minion_maximum_life_+%", "min": 4, "max": 8}]"#
        ),
        modifier(
            "MinionLife",
            1,
            ring,
            r#"[{"id": "dummy_stat_display_nothing", "min": 1, "max": 1}, {"id": "minion_maximum_life_+%", "min": 5, "max": 15}]"#
        ),
    );
    let translations = r#"[{"ids": ["minion_maximum_life_+%"], "English": [{"string": "Minions have {0}% increased maximum Life", "index_handlers": [[]]}], "hidden": null}]"#;

    let (tables, _) = tables(
        &modifiers,
        BASES,
        translations,
        &[(
            "Minions have #% increased maximum Life",
            "Minions have #% increased maximum Life",
        )],
    );

    // A stat without translation that doesn't roll is left out of its lines only.
    let types: Vec<_> = tables
        .as_array()
        .expect("tables")
        .iter()
        .map(|table| table["type"].clone())
        .collect();
    assert_eq!(types, [serde_json::json!("MinionLife")]);
}

/// The parser review of 2026-11-10: the new modifiers of the essences and of the alloys
/// were found by comparing lists by hand.
#[test]
fn a_modifier_of_a_currency_not_listed_and_the_other_domains_are_noted() {
    let source = |domain: &str, kind: &str, essence_only: bool| -> SourceModifier {
        serde_json::from_str(&format!(
            r#"{{"domain": "{domain}", "generation_type": "{kind}", "is_essence_only": {essence_only},
                "type": "Life", "required_level": 1, "spawn_weights": [], "stats": []}}"#
        ))
        .expect("a modifier")
    };
    let modifiers = [
        ("EssenceNewOne1", source("item", "prefix", true), false),
        ("EssenceListed1", source("item", "prefix", true), true),
        ("AlloyNewOne1", source("item", "suffix", false), false),
        ("OddlyNamed1", source("delve", "suffix", true), false),
        ("Ordinary1", source("item", "suffix", false), false),
        ("Heist1", source("heist", "suffix", false), false),
        ("Heist2", source("heist", "prefix", false), false),
        ("Unique1", source("heist", "unique", true), false),
    ];
    let mut report = Report::default();
    let mut other_domains = BTreeMap::new();

    for (id, modifier, listed) in &modifiers {
        note_left_out(id, modifier, *listed, &mut other_domains, &mut report);
    }

    let text = report.to_markdown("a commit");
    assert!(
        text.ends_with("left out\n\n- `AlloyNewOne1`\n- `EssenceNewOne1`\n"),
        "{text}"
    );
    assert_eq!(other_domains, BTreeMap::from([("delve", 1), ("heist", 2)]));
}
