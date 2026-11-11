//! The bases of Exiled Exchange 2 (`renderer/public/data/<language>/items.ndjson`), of the
//! categories the application searches by their filters.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::Result;
use crate::modifiers;
use crate::report::Report;

/// The categories of the bases the application searches by their filters: the equipment,
/// the jewels, the flasks, the charms, the waystones (`Map`) and the tablets (`TowerAugment`).
const CATEGORIES: [&str; 32] = [
    "Amulet",
    "Belt",
    "Body Armour",
    "Boots",
    "Bow",
    "Buckler",
    "Charm",
    "Crossbow",
    "Dagger",
    "Flail",
    "Flask",
    "Focus",
    "Gloves",
    "Helmet",
    "Jewel",
    "Map",
    "One Hand Axe",
    "One Hand Mace",
    "One Hand Sword",
    "Quiver",
    "Ring",
    "Sceptre",
    "Shield",
    "Spear",
    "Staff",
    "Talisman",
    "TowerAugment",
    "Two Hand Axe",
    "Two Hand Mace",
    "Two Hand Sword",
    "Wand",
    "Warstaff",
];
/// The categories left out: the currencies, the gems and the other items without base filters.
const LEFT_OUT_CATEGORIES: [&str; 14] = [
    "Breachstone",
    "BrequelFruit",
    "Currency",
    "ExpeditionLogbook",
    "Incubator",
    "MapFragment",
    "MiscMapItem",
    "Omen",
    "PinnacleKey",
    "QuestItem",
    "Relic",
    "SoulCore",
    "UncutSkillGem",
    "VaultKey",
];

/// An item of `renderer/public/data/<language>/items.ndjson`, with the fields the generator
/// reads.
#[derive(Deserialize)]
struct SourceItem {
    /// None for a few uniques without data.
    name: Option<String>,
    /// The English name.
    #[serde(rename = "refName")]
    ref_name: Option<String>,
    namespace: String,
    craftable: Option<SourceCraftable>,
}

/// What the game crafts of an item: the category of its base.
#[derive(Deserialize)]
struct SourceCraftable {
    category: String,
}

/// A base as the application reads it (`src/game_data/bases.rs`): its name in the language
/// of the file, its category, and the tags that tell the modifiers it rolls.
#[derive(Serialize)]
pub struct Base {
    name: String,
    category: String,
    /// The domain of its modifiers, written when it isn't `item`.
    #[serde(skip_serializing_if = "modifiers::is_equipment")]
    domain: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tags: Vec<String>,
}

/// The bases of an `items.ndjson` in `language`, of the categories the application searches,
/// each name once, in the category most of its entries have ([`main_category`]). Each takes the
/// domain and the tags of its English name in `spawns`.
pub fn reduce(
    source: &str,
    spawns: &modifiers::BaseSpawns,
    language: &str,
    report: &mut Report,
) -> Result<Vec<Base>> {
    let Entries {
        categories,
        english_names,
    } = entries(source)?;
    let mut bases = Vec::new();
    for (name, counts) in categories {
        if counts.len() > 1 {
            report.note(
                "Bases in several categories",
                format!("{language}: {name} {counts:?}"),
            );
        }
        let category = main_category(counts)?;
        let english = english_names.get(&name).unwrap_or(&name);
        let (domain, tags) = spawns.get(english).map_or_else(
            || (modifiers::EQUIPMENT.to_owned(), Vec::new()),
            |spawn| (spawn.domain.clone(), spawn.tags.clone()),
        );
        // The application reads the domains of the modifiers kept only.
        if !modifiers::is_kept_domain(&domain) {
            return Err(format!("unknown domain {domain} of {name}").into());
        }
        if tags.is_empty() {
            report.note("Bases without tags", format!("{language}: {name}"));
        }
        bases.push(Base {
            name,
            category,
            domain,
            tags,
        });
    }
    Ok(bases)
}

/// The entries of the bases of an `items.ndjson`, by their name in its language.
struct Entries {
    /// The count of the entries of each base in each of its categories.
    categories: BTreeMap<String, BTreeMap<String, usize>>,
    /// The English name of each base.
    english_names: BTreeMap<String, String>,
}

/// The entries of the bases of `source`, of the categories the application searches; an error
/// for a category it doesn't know.
fn entries(source: &str) -> Result<Entries> {
    let mut categories: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    let mut english_names = BTreeMap::new();
    for (index, line) in source.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let item: SourceItem =
            serde_json::from_str(line).map_err(|error| format!("line {}: {error}", index + 1))?;
        let (Some(name), Some(craftable)) = (item.name, item.craftable) else {
            continue;
        };
        if item.namespace != "ITEM" || LEFT_OUT_CATEGORIES.contains(&craftable.category.as_str()) {
            continue;
        }
        // A new category of the game is decided before it is embedded.
        if !CATEGORIES.contains(&craftable.category.as_str()) {
            return Err(format!("unknown category {} of {name}", craftable.category).into());
        }
        if let Some(english) = item.ref_name {
            english_names.insert(name.clone(), english);
        }
        *categories
            .entry(name)
            .or_default()
            .entry(craftable.category)
            .or_default() += 1;
    }
    Ok(Entries {
        categories,
        english_names,
    })
}

/// The category of a base, of the `counts` of its entries in each: the one most of them have
/// (`Runemastered Venerable Defender`, three times a shield and once a buckler), the first in
/// alphabetical order on a tie.
fn main_category(counts: BTreeMap<String, usize>) -> Result<String> {
    counts
        .into_iter()
        .max_by(|(first, first_count), (second, second_count)| {
            first_count
                .cmp(second_count)
                .then_with(|| second.cmp(first))
        })
        .map(|(category, _)| category)
        .ok_or_else(|| "a base without category".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bases of `lines`, as name and category.
    fn bases(lines: &[&str]) -> Result<Vec<(String, String)>> {
        Ok(reduce(
            &lines.join("\n"),
            &BTreeMap::new(),
            "en",
            &mut Report::default(),
        )?
        .into_iter()
        .map(|base| (base.name, base.category))
        .collect())
    }

    fn base(name: &str, category: &str) -> (String, String) {
        (name.to_owned(), category.to_owned())
    }

    #[test]
    fn a_base_of_a_domain_the_application_doesnt_know_stops_the_generator() {
        let spawn = |domain: &str| modifiers::BaseSpawn {
            domain: domain.to_owned(),
            tags: vec!["ring".to_owned()],
        };
        let ring =
            r#"{"name": "Gold Ring", "namespace": "ITEM", "craftable": {"category": "Ring"}}"#;
        let reduced = |domain| {
            reduce(
                ring,
                &BTreeMap::from([("Gold Ring".to_owned(), spawn(domain))]),
                "en",
                &mut Report::default(),
            )
        };

        assert!(reduced("item").is_ok());
        assert_eq!(
            reduced("heist").err().map(|error| error.to_string()),
            Some("unknown domain heist of Gold Ring".to_owned())
        );
    }

    #[test]
    fn the_bases_of_the_searched_categories_are_kept_each_name_once() {
        let kept = bases(&[
            r#"{"name": "Mammoth Targe", "namespace": "ITEM", "craftable": {"category": "Shield"}}"#,
            r#"{"name": "Mammoth Targe", "namespace": "ITEM", "craftable": {"category": "Shield"}}"#,
            // The waystones and the tablets.
            r#"{"name": "Waystone (Tier 12)", "namespace": "ITEM", "craftable": {"category": "Map"}}"#,
            r#"{"name": "Irradiated Tablet", "namespace": "ITEM", "craftable": {"category": "TowerAugment"}}"#,
            // Another category, another namespace, no category, no name.
            r#"{"name": "Exalted Orb", "namespace": "ITEM", "craftable": {"category": "Currency"}}"#,
            r#"{"name": "Headhunter", "namespace": "UNIQUE", "craftable": {"category": "Belt"}}"#,
            r#"{"name": "Gold", "namespace": "ITEM"}"#,
            r#"{"namespace": "UNIQUE", "unique": {"base": "Ring"}}"#,
            "",
        ])
        .expect("read");

        assert_eq!(
            kept,
            [
                base("Irradiated Tablet", "TowerAugment"),
                base("Mammoth Targe", "Shield"),
                base("Waystone (Tier 12)", "Map"),
            ]
        );
    }

    #[test]
    fn a_base_in_several_categories_takes_the_one_most_of_its_entries_have() {
        let defender = |category: &str| {
            format!(
                r#"{{"name": "Venerable Defender", "namespace": "ITEM", "craftable": {{"category": "{category}"}}}}"#
            )
        };
        let most = bases(&[
            &defender("Shield"),
            &defender("Buckler"),
            &defender("Shield"),
        ]);
        // On a tie, the first in alphabetical order.
        let tie = bases(&[&defender("Shield"), &defender("Buckler")]);

        assert_eq!(most.expect("read"), [base("Venerable Defender", "Shield")]);
        assert_eq!(tie.expect("read"), [base("Venerable Defender", "Buckler")]);
    }

    #[test]
    fn a_new_category_stops_the_generator() {
        let new =
            bases(&[r#"{"name": "Idol", "namespace": "ITEM", "craftable": {"category": "Idol"}}"#]);

        assert_eq!(
            new.expect_err("a new category").to_string(),
            "unknown category Idol of Idol"
        );
    }
}
