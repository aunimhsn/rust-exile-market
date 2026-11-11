//! The data the tests of the modifier tables are written with.

use super::*;

/// The files of RePoE written as JSON: `modifiers` of `mods.json`, `bases` of `base_items.json`
/// and `translations` of `stat_descriptions.json`, without translation of the tablets.
pub(super) fn repoe(modifiers: &str, bases: &str, translations: &str) -> RepoeFiles {
    RepoeFiles {
        modifiers: serde_json::from_str(modifiers).expect("modifiers"),
        bases: serde_json::from_str(bases).expect("bases"),
        translations: serde_json::from_str(translations).expect("translations"),
        tablet_translations: Vec::new(),
    }
}

/// The tables and the spawns of the bases from `modifiers` of `mods.json`, `bases` of
/// `base_items.json` and `translations` of `stat_descriptions.json`, written as JSON, the
/// templates of Exiled Exchange 2 being `templates` (string, template).
pub(super) fn tables(
    modifiers: &str,
    bases: &str,
    translations: &str,
    templates: &[(&str, &str)],
) -> (serde_json::Value, BaseSpawns) {
    let templates = templates
        .iter()
        .map(|&(string, template)| (string.to_owned(), template.to_owned()))
        .collect();
    let (tables, base_spawns) = reduce(
        &repoe(modifiers, bases, translations),
        &templates,
        &[],
        &mut Report::default(),
    )
    .expect("reduced");
    (serde_json::to_value(tables).expect("written"), base_spawns)
}

/// A modifier of `mods.json`, rolling where `spawn` says, with `stats`.
pub(super) fn modifier(kind: &str, level: u32, spawn: &str, stats: &str) -> String {
    format!(
        r#"{{"domain": "item", "generation_type": "suffix", "is_essence_only": false, "type": "{kind}", "required_level": {level}, "spawn_weights": {spawn}, "stats": {stats}}}"#
    )
}

/// Four bases of `base_items.json`: a bow, a ring and a flask, and a base not released.
pub(super) const BASES: &str = r#"{
    "Bow": {"domain": "item", "name": "Bow", "release_state": "released", "tags": ["bow", "weapon", "default"]},
    "Ring": {"domain": "item", "name": "Ring", "release_state": "released", "tags": ["ring", "default"]},
    "Flask": {"domain": "flask", "name": "Flask", "release_state": "released", "tags": ["life_flask", "default"]},
    "Old": {"domain": "item", "name": "Old", "release_state": "unreleased", "tags": ["fishing_rod", "default"]}
}"#;
