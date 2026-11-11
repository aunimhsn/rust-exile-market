use super::*;
use crate::modifiers::fixtures::{BASES, modifier, repoe, tables};
use crate::modifiers::{RepoeFiles, reduce};
use crate::report::Report;

/// A waystone of the band of tiers 11 to 15 and a tablet, of `base_items.json`.
const MAP_BASES: &str = r#"{
    "Waystone": {"domain": "area", "name": "Waystone (Tier 12)", "release_state": "released", "tags": ["map_key_high", "map", "default"]},
    "Tablet": {"domain": "tablet", "name": "Irradiated Tablet", "release_state": "released", "tags": ["tower_augment", "default"]}
}"#;

/// The lines of each table of `modifiers` rolling on [`MAP_BASES`], with `translations`
/// and `tablet_translations`, the templates of Exiled Exchange 2 being `templates`.
fn map_lines(
    modifiers: &str,
    translations: &str,
    tablet_translations: &str,
    templates: &[&str],
) -> Vec<serde_json::Value> {
    let (tables, _) = reduce(
        &RepoeFiles {
            tablet_translations: serde_json::from_str(tablet_translations)
                .expect("tablet translations"),
            ..repoe(modifiers, MAP_BASES, translations)
        },
        &templates
            .iter()
            .map(|&template| (template.to_owned(), template.to_owned()))
            .collect(),
        &[],
        &mut Report::default(),
    )
    .expect("reduced");
    tables
        .iter()
        .map(|table| serde_json::json!([table.domain, table.lines, table.tiers]))
        .collect()
}

#[test]
fn a_tablet_takes_the_lines_of_its_own_translations_first() {
    let gold = |domain: &str| {
        modifier(
            "Gold",
            1,
            r#"[{"tag": "default", "weight": 1}]"#,
            r#"[{"id": "map_extra_gold_piles_chance_%", "min": 25, "max": 35}, {"id": "map_verisium_stack_size_+%", "min": 15, "max": 25}]"#,
        )
        .replace(r#""domain": "item""#, &format!(r#""domain": "{domain}""#))
    };
    let modifiers = format!(
        r#"{{"TowerGold": {}, "MapGold": {}}}"#,
        gold("tablet"),
        gold("area")
    );
    let translations = r#"[
        {"ids": ["map_extra_gold_piles_chance_%"], "English": [{"string": "{0}% increased Gold found in this Area", "index_handlers": [[]]}], "hidden": null},
        {"ids": ["map_verisium_stack_size_+%"], "English": [{"string": "{0}% increased Verisium", "index_handlers": [[]]}], "hidden": null}
    ]"#;
    let tablet_translations = r#"[{"ids": ["map_extra_gold_piles_chance_%"], "English": [{"string": "{0}% increased Gold found in Map", "index_handlers": [[]]}], "hidden": null}]"#;

    let lines = map_lines(
        &modifiers,
        translations,
        tablet_translations,
        &[
            "#% increased Gold found in this Area",
            "#% increased Gold found in Map",
            "#% increased Verisium",
        ],
    );

    // The tablet with its own line of gold, and the line of the other file it lacks; the
    // waystone with the lines of the other file only.
    let tiers = serde_json::json!([{"level": 1, "ranges": [[[25, 35]], [[15, 25]]]}]);
    assert_eq!(
        lines,
        [
            serde_json::json!(["area", [{"template": "#% increased Gold found in this Area"}, {"template": "#% increased Verisium"}], tiers]),
            serde_json::json!(["tablet", [{"template": "#% increased Gold found in Map"}, {"template": "#% increased Verisium"}], tiers]),
        ]
    );
}

#[test]
fn a_waystone_modifier_writes_only_the_lines_the_game_writes() {
    // `Monsters are Armoured`: a value its line doesn't write, the share of the strength
    // of the monsters added to their armour; a fixed value without translation, folded
    // into the properties of the waystone; a value of 0 the game doesn't write.
    let armoured = modifier(
        "Armoured",
        1,
        r#"[{"tag": "default", "weight": 1}]"#,
        r#"[
            {"id": "map_monsters_additional_strength_ratio_%_for_armour", "min": 35, "max": 55},
            {"id": "map_map_item_drop_chance_+%_final_from_map", "min": 15, "max": 15},
            {"id": "map_item_drop_rarity_+%", "min": 0, "max": 0}
        ]"#,
    )
    .replace(r#""domain": "item""#, r#""domain": "area""#);
    // Every value 0: never written, no table.
    let nothing = modifier(
        "Nothing",
        1,
        r#"[{"tag": "default", "weight": 1}]"#,
        r#"[{"id": "map_item_drop_rarity_+%", "min": 0, "max": 0}]"#,
    )
    .replace(r#""domain": "item""#, r#""domain": "area""#);
    let modifiers = format!(r#"{{"MapArmoured": {armoured}, "MapNothing": {nothing}}}"#);
    let translations = r#"[
        {"ids": ["map_monsters_additional_strength_ratio_%_for_armour"], "English": [{"string": "Monsters are [MonsterExtraArmour1|Armoured]", "index_handlers": [[]]}], "hidden": null},
        {"ids": ["map_item_drop_rarity_+%"], "English": [{"string": "{0}% increased Rarity of Items found in this Area", "index_handlers": [[]]}], "hidden": null}
    ]"#;

    let lines = map_lines(
        &modifiers,
        translations,
        "[]",
        &[
            "Monsters are Armoured",
            "#% increased Rarity of Items found in this Area",
        ],
    );

    assert_eq!(
        lines,
        [
            serde_json::json!(["area", [{"template": "Monsters are Armoured"}], [{"level": 1, "ranges": [[]]}]])
        ]
    );
}

#[test]
fn a_line_of_two_values_is_written_when_one_of_them_isnt_0() {
    let damage = modifier(
        "Damage",
        1,
        r#"[{"tag": "default", "weight": 1}]"#,
        r#"[{"id": "minimum_damage", "min": 0, "max": 0}, {"id": "maximum_damage", "min": 5, "max": 10}]"#,
    )
    .replace(r#""domain": "item""#, r#""domain": "area""#);
    let translations = r#"[{"ids": ["minimum_damage", "maximum_damage"], "English": [{"string": "Adds {0} to {1} Damage", "index_handlers": [[], []]}], "hidden": null}]"#;

    let lines = map_lines(
        &format!(r#"{{"MapDamage": {damage}}}"#),
        translations,
        "[]",
        &["Adds # to # Damage"],
    );

    assert_eq!(
        lines,
        [
            serde_json::json!(["area", [{"template": "Adds # to # Damage"}], [{"level": 1, "ranges": [[[0, 0], [5, 10]]]}]])
        ]
    );
}

#[test]
fn a_line_known_with_the_plus_sign_of_its_number_keeps_that_template() {
    let resistances = modifier(
        "Resistances",
        1,
        r#"[{"tag": "map_key_high", "weight": 1}, {"tag": "default", "weight": 0}]"#,
        r#"[{"id": "map_monsters_additional_elemental_resistance", "min": 30, "max": 34}]"#,
    )
    .replace(r#""domain": "item""#, r#""domain": "area""#);
    let translations = r#"[{"ids": ["map_monsters_additional_elemental_resistance"], "English": [{"string": "+{0}% Monster Elemental Resistances", "index_handlers": [[]]}], "hidden": null}]"#;

    let lines = map_lines(
        &format!(r#"{{"MapResistances": {resistances}}}"#),
        translations,
        "[]",
        &[
            "#% Monster Elemental Resistances",
            "+#% Monster Elemental Resistances",
        ],
    );

    assert_eq!(
        lines,
        [
            serde_json::json!(["area", [{"template": "+#% Monster Elemental Resistances"}], [{"level": 1, "ranges": [[[30, 34]]]}]])
        ]
    );
}

#[test]
fn a_line_is_known_without_the_plus_sign_of_its_number() {
    // The application reads the sign in the value: `+33% Monster Elemental Resistances`
    // finds `#% Monster Elemental Resistances`.
    let resistances = modifier(
        "Resistances",
        1,
        r#"[{"tag": "map_key_high", "weight": 1}, {"tag": "default", "weight": 0}]"#,
        r#"[{"id": "map_monsters_additional_elemental_resistance", "min": 30, "max": 34}]"#,
    )
    .replace(r#""domain": "item""#, r#""domain": "area""#);
    let translations = r#"[{"ids": ["map_monsters_additional_elemental_resistance"], "English": [{"string": "+{0}% Monster Elemental Resistances", "index_handlers": [[]]}], "hidden": null}]"#;

    let lines = map_lines(
        &format!(r#"{{"MapResistances": {resistances}}}"#),
        translations,
        "[]",
        &["#% Monster Elemental Resistances"],
    );

    assert_eq!(
        lines,
        [
            serde_json::json!(["area", [{"template": "#% Monster Elemental Resistances"}], [{"level": 1, "ranges": [[[30, 34]]]}]])
        ]
    );
}

/// The lines and the tiers of the one table of a modifier of `stats`, written by
/// `translation`, with `templates`.
fn one_table(stats: &[&str], translation: &str, templates: &[(&str, &str)]) -> serde_json::Value {
    let weapon = r#"[{"tag": "weapon", "weight": 1}, {"tag": "default", "weight": 0}]"#;
    let modifiers: Vec<String> = stats
        .iter()
        .enumerate()
        .map(|(index, stats)| {
            format!(
                r#""Tier{index}": {}"#,
                modifier(
                    "Kind",
                    10 * u32::try_from(index).expect("small"),
                    weapon,
                    stats
                )
            )
        })
        .collect();
    let (tables, _) = tables(
        &format!("{{{}}}", modifiers.join(", ")),
        BASES,
        &format!("[{translation}]"),
        templates,
    );
    serde_json::json!([tables[0]["lines"].clone(), tables[0]["tiers"].clone()])
}

#[test]
fn a_line_written_without_number_takes_its_values_from_the_string_that_writes_them() {
    let bolts = one_table(
        &[
            r#"[{"id": "base_number_of_crossbow_bolts", "min": 1, "max": 1}]"#,
            r#"[{"id": "base_number_of_crossbow_bolts", "min": 2, "max": 2}]"#,
        ],
        r#"{"ids": ["base_number_of_crossbow_bolts", "quality_display"], "English": [
            {"string": "Loads an additional bolt", "index_handlers": [[], []]},
            {"string": "Loads {0} additional bolts", "index_handlers": [[], []]}
        ], "hidden": null}"#,
        &[
            ("Loads an additional bolt", "Loads an additional bolt"),
            ("Loads # additional bolts", "Loads an additional bolt"),
        ],
    );

    assert_eq!(
        bolts,
        serde_json::json!([
            [{"template": "Loads an additional bolt"}],
            [{"level": 0, "ranges": [[[1, 1]]]}, {"level": 10, "ranges": [[[2, 2]]]}]
        ])
    );
}

#[test]
fn a_line_shown_scaled_keeps_its_divisor_for_the_value_of_its_string_without_number() {
    let rage = one_table(
        &[r#"[{"id": "rage_loss_delay_ms_+", "min": 1000, "max": 2000}]"#],
        r#"{"ids": ["rage_loss_delay_ms_+"], "English": [
            {"string": "Inherent [Rage|Rage] loss starts 1 second later", "index_handlers": [[]]},
            {"string": "Inherent [Rage|Rage] loss starts {0} seconds later", "index_handlers": [["milliseconds_to_seconds"]]}
        ], "hidden": null}"#,
        &[(
            "Inherent Rage loss starts 1 second later",
            "Inherent Rage loss starts 1 second later",
        )],
    );

    assert_eq!(
        rage,
        serde_json::json!([
            [{"template": "Inherent Rage loss starts 1 second later", "divisor": 1000}],
            [{"level": 0, "ranges": [[[1, 2]]]}]
        ])
    );
}

#[test]
fn a_line_without_number_in_every_string_has_the_values_of_its_stat() {
    let radius = one_table(
        &[r#"[{"id": "local_jewel_display_radius_change", "min": 1, "max": 1}]"#],
        r#"{"ids": ["local_jewel_display_radius_change"], "English": [
            {"string": "Upgrades Radius to Medium", "index_handlers": [[]]},
            {"string": "Upgrades Radius to Large", "index_handlers": [[]]}
        ], "hidden": null}"#,
        // Found by a string of Exiled Exchange 2, not by its template.
        &[("Upgrades Radius to Medium", "Upgrades Radius to #")],
    );

    assert_eq!(
        radius,
        serde_json::json!([
            [{"template": "Upgrades Radius to #"}],
            [{"level": 0, "ranges": [[[1, 1]]]}]
        ])
    );
}

#[test]
fn a_number_of_a_stat_the_modifier_doesnt_have_is_zero() {
    let added = one_table(
        &[r#"[{"id": "local_maximum_added_fire_damage", "min": 10, "max": 15}]"#],
        r#"{"ids": ["local_minimum_added_fire_damage", "local_maximum_added_fire_damage"], "English": [
            {"string": "Adds {0} to {1} [Fire|Fire] Damage", "index_handlers": [[], []]}
        ], "hidden": null}"#,
        &[("Adds # to # Fire Damage", "Adds # to # Fire Damage")],
    );

    assert_eq!(
        added,
        serde_json::json!([
            [{"template": "Adds # to # Fire Damage"}],
            [{"level": 0, "ranges": [[[0, 0], [10, 15]]]}]
        ])
    );
}

#[test]
fn a_string_of_the_game_becomes_the_template_of_exiled_exchange_2() {
    assert_eq!(
        template_of("{0:+d} to [Accuracy|Accuracy] Rating"),
        "# to Accuracy Rating"
    );
    assert_eq!(
        template_of("[LifeLeech|Leech] {0}% of [Physical] [Attack] Damage as Life"),
        "Leech #% of Physical Attack Damage as Life"
    );
    assert_eq!(placeholders("Adds {1} to {0} Damage, {} more"), [1, 0, 2]);
    // A brace or a bracket not closed is written as it is.
    assert_eq!(template_of("abc {0"), "abc {0");
    assert_eq!(template_of("abc [Fire"), "abc [Fire");
}

#[test]
fn a_value_is_shown_as_the_handlers_of_its_stat_convert_it() {
    let per_second = shown_value(1386, &["per_minute_to_per_second".to_owned()]).expect("scaled");
    let permyriad = shown_value(
        -790,
        &["divide_by_one_hundred".to_owned(), "negate".to_owned()],
    )
    .expect("scaled");

    assert_eq!(serde_json::to_string(&per_second).expect("written"), "23.1");
    assert_eq!(serde_json::to_string(&permyriad).expect("written"), "-7.9");
}

#[test]
fn an_unknown_handler_stops_the_generator() {
    assert!(shown_value(1, &["new_handler".to_owned()]).is_err());
}

#[test]
fn a_line_without_value_has_no_range() {
    let instant = one_table(
        &[r#"[{"id": "local_flask_recovers_instantly", "min": 1, "max": 1}]"#],
        r#"{"ids": ["local_flask_recovers_instantly"], "English": [
            {"string": "Instant Recovery", "index_handlers": [[]]}
        ], "hidden": null}"#,
        &[("Instant Recovery", "Instant Recovery")],
    );

    assert_eq!(
        instant,
        serde_json::json!([
            [{"template": "Instant Recovery"}],
            [{"level": 0, "ranges": [[]]}]
        ])
    );
}

#[test]
fn a_stat_that_rolls_without_being_written_stops_the_generator() {
    let modifiers = format!(
        r#"{{"Bolts": {}}}"#,
        modifier(
            "Bolts",
            1,
            r#"[{"tag": "weapon", "weight": 1}]"#,
            r#"[{"id": "base_number_of_crossbow_bolts", "min": 1, "max": 2}]"#
        )
    );
    let translations = r#"[{"ids": ["base_number_of_crossbow_bolts"], "English": [
        {"string": "Loads an additional bolt", "index_handlers": [[]]}
    ], "hidden": null}]"#;

    let error = reduce(
        &repoe(&modifiers, BASES, translations),
        &[(
            "Loads an additional bolt".to_owned(),
            "Loads an additional bolt".to_owned(),
        )]
        .into(),
        &[],
        &mut Report::default(),
    )
    .err()
    .map(|error| error.to_string());

    assert_eq!(
        error.as_deref(),
        Some(
            "Bolts: base_number_of_crossbow_bolts rolls but Loads an additional bolt doesn't write it"
        )
    );
}

#[test]
fn a_line_of_several_values_with_a_string_without_number_stops_the_generator() {
    let modifiers = format!(
        r#"{{"Thorns": {}}}"#,
        modifier(
            "Thorns",
            1,
            r#"[{"tag": "weapon", "weight": 1}]"#,
            r#"[{"id": "minimum_thorns", "min": 1, "max": 2}, {"id": "maximum_thorns", "min": 3, "max": 4}]"#
        )
    );
    let translations = r#"[{"ids": ["minimum_thorns", "maximum_thorns"], "English": [
        {"string": "Thorns", "index_handlers": [[], []]},
        {"string": "{0} to {1} Thorns damage", "index_handlers": [[], []]}
    ], "hidden": null}]"#;

    let error = reduce(
        &repoe(&modifiers, BASES, translations),
        &[("Thorns".to_owned(), "# to # Thorns damage".to_owned())].into(),
        &[],
        &mut Report::default(),
    )
    .err()
    .map(|error| error.to_string());

    assert_eq!(
        error.as_deref(),
        Some("Thorns: {0} to {1} Thorns damage has several values and a string without number")
    );
}

/// The templates of the lines of the tables of the Time-Lost modifiers of `stats` (each
/// written `{0}% increased <stat>`), desecrated or not, on a Time-Lost Ruby, Exiled Exchange 2
/// knowing `templates`.
fn time_lost_templates(
    stats: &[&str],
    desecrated: bool,
    templates: &[&str],
) -> Result<Vec<serde_json::Value>> {
    let spawn = r#"[{"tag": "str_radius_jewel", "weight": 1}, {"tag": "default", "weight": 0}]"#;
    time_lost_tables(stats, spawn, desecrated, templates)
}

/// [`time_lost_templates`] with the modifiers rolling where `spawn` says.
fn time_lost_tables(
    stats: &[&str],
    spawn: &str,
    desecrated: bool,
    templates: &[&str],
) -> Result<Vec<serde_json::Value>> {
    let bases = r#"{"TimeLostRuby": {"domain": "misc", "name": "Time-Lost Ruby", "release_state": "released", "tags": ["jewel", "str_radius_jewel", "radius_jewel", "default"]}}"#;
    let modifiers: Vec<String> = stats
        .iter()
        .map(|stat| {
            let modifier = modifier(
                stat,
                1,
                spawn,
                &format!(r#"[{{"id": "{stat}", "min": 1, "max": 2}}]"#),
            )
            .replace(
                r#""domain": "item""#,
                if desecrated {
                    r#""domain": "desecrated""#
                } else {
                    r#""domain": "misc""#
                },
            );
            format!(r#""{stat}": {modifier}"#)
        })
        .collect();
    let translations: Vec<String> = stats
        .iter()
        .map(|stat| {
            format!(
                r#"{{"ids": ["{stat}"], "English": [{{"string": "{{0}}% increased {stat}", "index_handlers": [[]]}}], "hidden": null}}"#
            )
        })
        .collect();
    let (tables, _) = reduce(
        &repoe(
            &format!("{{{}}}", modifiers.join(", ")),
            bases,
            &format!("[{}]", translations.join(", ")),
        ),
        &templates
            .iter()
            .map(|&template| (template.to_owned(), template.to_owned()))
            .collect(),
        &[],
        &mut Report::default(),
    )?;
    Ok(serde_json::to_value(tables)
        .expect("written")
        .as_array()
        .expect("tables")
        .iter()
        .map(|table| table["lines"][0]["template"].clone())
        .collect())
}

#[test]
fn a_line_of_a_time_lost_jewel_is_written_within_the_words_of_the_passive_skills_it_grants() {
    // The small passive skills or the notables, whichever Exiled Exchange 2 knows, also when
    // it knows the line alone; a line about the radius itself alone when it knows it only,
    // another line left out (`Light Radius`, an ordinary modifier of other bases).
    let templates = time_lost_templates(
        &[
            "Armour",
            "Evasion",
            "Effect of Small Passive Skills in Radius",
            "Life",
            "Light Radius",
        ],
        false,
        &[
            "#% increased Armour",
            "Small Passive Skills in Radius also grant #% increased Armour",
            "Notable Passive Skills in Radius also grant #% increased Evasion",
            "#% increased Effect of Small Passive Skills in Radius",
            "#% increased Life",
            "#% increased Light Radius",
        ],
    )
    .expect("tables");

    assert_eq!(
        templates,
        [
            "Small Passive Skills in Radius also grant #% increased Armour",
            "#% increased Effect of Small Passive Skills in Radius",
            "Notable Passive Skills in Radius also grant #% increased Evasion",
        ]
    );
}

#[test]
fn a_line_of_a_time_lost_jewel_known_without_its_number_only_is_left_out() {
    // The game writes `… 4% chance for Attack Hits to apply Incision`, which Exiled Exchange 2
    // knows alone only, within the words without number (`value` 100).
    let modifiers = format!(
        r#"{{"Incision": {}}}"#,
        modifier(
            "Incision",
            1,
            r#"[{"tag": "str_radius_jewel", "weight": 1}, {"tag": "default", "weight": 0}]"#,
            r#"[{"id": "incision_chance", "min": 3, "max": 5}]"#
        )
        .replace(r#""domain": "item""#, r#""domain": "misc""#)
    );
    let bases = r#"{"TimeLostRuby": {"domain": "misc", "name": "Time-Lost Ruby", "release_state": "released", "tags": ["str_radius_jewel", "radius_jewel", "default"]}}"#;
    let translations = r#"[{"ids": ["incision_chance"], "English": [
        {"string": "Attack Hits apply Incision", "index_handlers": [[]]},
        {"string": "{0}% chance for Attack Hits to apply Incision", "index_handlers": [[]]}
    ], "hidden": null}]"#;

    let (tables, _) = tables(
        &modifiers,
        bases,
        translations,
        &[
            ("Attack Hits apply Incision", "Attack Hits apply Incision"),
            (
                "#% chance for Attack Hits to apply Incision",
                "Attack Hits apply Incision",
            ),
            (
                "Small Passive Skills in Radius also grant Attack Hits apply Incision",
                "Small Passive Skills in Radius also grant Attack Hits apply Incision",
            ),
        ],
    );

    assert_eq!(tables, serde_json::json!([]));
}

#[test]
fn a_modifier_of_the_time_lost_jewels_rolling_on_other_bases_stops_the_generator() {
    let spawn = r#"[{"tag": "str_radius_jewel", "weight": 1}, {"tag": "ring", "weight": 1}, {"tag": "default", "weight": 0}]"#;

    let error = time_lost_tables(
        &["Armour"],
        spawn,
        false,
        &["Small Passive Skills in Radius also grant #% increased Armour"],
    )
    .err()
    .map(|error| error.to_string());

    assert_eq!(
        error.as_deref(),
        Some("Armour: a modifier of the Time-Lost jewels that rolls on other bases")
    );
}

#[test]
fn a_line_of_a_time_lost_jewel_known_for_both_kinds_of_passive_skills_is_a_notable_if_desecrated() {
    let both = [
        "Small Passive Skills in Radius also grant #% increased Life",
        "Notable Passive Skills in Radius also grant #% increased Life",
    ];

    let desecrated = time_lost_templates(&["Life"], true, &both).expect("tables");
    let ordinary = time_lost_templates(&["Life"], false, &both)
        .err()
        .map(|error| error.to_string());

    assert_eq!(
        desecrated,
        ["Notable Passive Skills in Radius also grant #% increased Life"]
    );
    assert_eq!(
        ordinary.as_deref(),
        Some("Life: #% increased Life is known within the words of both kinds of passive skills")
    );
}

#[test]
fn a_line_of_a_time_lost_jewel_known_for_both_kinds_by_two_strings_stops_the_generator() {
    // Its number under the small passive skills, its string without number under the
    // notables.
    let modifiers = format!(
        r#"{{"Daze": {}}}"#,
        modifier(
            "Daze",
            1,
            r#"[{"tag": "str_radius_jewel", "weight": 1}, {"tag": "default", "weight": 0}]"#,
            r#"[{"id": "daze_chance", "min": 1, "max": 2}]"#
        )
        .replace(r#""domain": "item""#, r#""domain": "misc""#)
    );
    let bases = r#"{"TimeLostRuby": {"domain": "misc", "name": "Time-Lost Ruby", "release_state": "released", "tags": ["str_radius_jewel", "radius_jewel", "default"]}}"#;
    let translations = r#"[{"ids": ["daze_chance"], "English": [
        {"string": "Dazes on Hit", "index_handlers": [[]]},
        {"string": "{0}% chance to Daze on Hit", "index_handlers": [[]]}
    ], "hidden": null}]"#;
    let templates: Templates = [
        "Small Passive Skills in Radius also grant #% chance to Daze on Hit",
        "Notable Passive Skills in Radius also grant Dazes on Hit",
    ]
    .into_iter()
    .map(|template| (template.to_owned(), template.to_owned()))
    .collect();

    let error = reduce(
        &repoe(&modifiers, bases, translations),
        &templates,
        &[],
        &mut Report::default(),
    )
    .err()
    .map(|error| error.to_string());

    assert_eq!(
        error.as_deref(),
        Some(
            "Daze: #% chance to Daze on Hit is known within the words of both kinds of passive skills"
        )
    );
}

#[test]
fn a_string_without_number_of_another_template_gives_no_divisor() {
    // `Rage loss starts 1 second later` is the string of another stat: the line of
    // `Rage loss starts # seconds later` keeps its values as they are shown, undivided.
    let delay = one_table(
        &[r#"[{"id": "rage_delay_ms", "min": 1000, "max": 2000}]"#],
        r#"{"ids": ["rage_delay_ms"], "English": [
            {"string": "Rage loss starts {0} seconds later", "index_handlers": [["milliseconds_to_seconds"]]},
            {"string": "Rage loss starts 1 second later", "index_handlers": [["milliseconds_to_seconds"]]}
        ], "hidden": null}"#,
        &[
            (
                "Rage loss starts # seconds later",
                "Rage loss starts # seconds later",
            ),
            (
                "Rage loss starts 1 second later",
                "Inherent Rage loss starts 1 second later",
            ),
        ],
    );

    assert_eq!(
        delay,
        serde_json::json!([
            [{"template": "Rage loss starts # seconds later"}],
            [{"level": 0, "ranges": [[[1, 2]]]}]
        ])
    );
}
