//! The official trade site of Path of Exile 2, in each language of the game client: for now,
//! the names of its items.
//!
//! The rules of its use are in `roadmap.md` (trade site usage).

use std::collections::HashSet;
use std::time::{Duration, SystemTime};

use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::http::{self, HttpError, Pace};
use crate::item_names::{ItemNames, Names, UniqueName, UniqueNames};
use crate::language::Language;

/// The time between two requests, at least: the data endpoints announce no limit.
const PACE: Duration = Duration::from_secs(1);
/// The categories of the item data whose items have no magic version: their entries aren't
/// base types of equipment.
const NOT_EQUIPMENT: [&str; 2] = ["currency", "gem"];

/// A list of the data endpoints.
#[derive(Deserialize)]
struct ApiResult<T> {
    result: Vec<T>,
}

/// A category of `/data/items`.
#[derive(Deserialize)]
struct ApiItemCategory {
    id: String,
    entries: Vec<ApiItem>,
}

/// An item of `/data/items`: a base type, or a unique with its name.
#[derive(Deserialize)]
struct ApiItem {
    #[serde(rename = "type")]
    base_type: String,
    name: Option<String>,
    /// A variant of the item, e.g. `legacy` for a unique of an older version.
    disc: Option<String>,
}

/// A group of `/data/static`: the items of the currency exchange.
#[derive(Deserialize)]
struct ApiStaticGroup {
    entries: Vec<ApiStatic>,
}

#[derive(Deserialize)]
struct ApiStatic {
    id: String,
    #[serde(default)]
    text: String,
}

/// The data of one language.
struct LanguageData {
    items: Vec<ApiItemCategory>,
    exchange: Vec<ApiStaticGroup>,
}

/// The client of the trade site, which identifies the application and spaces its requests.
pub struct TradeSite {
    agent: ureq::Agent,
    pace: Pace,
}

impl TradeSite {
    #[must_use]
    pub fn new() -> Self {
        Self {
            agent: http::agent(),
            pace: Pace::new(PACE),
        }
    }

    /// The names of the items in English and in French: four requests.
    pub fn item_names(&mut self) -> Result<ItemNames, HttpError> {
        let english = self.language_data(Language::English)?;
        let french = self.language_data(Language::French)?;
        Ok(item_names(SystemTime::now(), english, french))
    }

    fn language_data(&mut self, language: Language) -> Result<LanguageData, HttpError> {
        Ok(LanguageData {
            items: self.data(language, "items")?,
            exchange: self.data(language, "static")?,
        })
    }

    /// The result of the data endpoint `name` on the trade site of `language`.
    fn data<T: DeserializeOwned>(
        &mut self,
        language: Language,
        name: &str,
    ) -> Result<Vec<T>, HttpError> {
        self.pace.wait();
        let url = format!("{}/api/trade2/data/{name}", host(language));
        let answer: ApiResult<T> = http::get_json(&self.agent, &url, &[])?;
        Ok(answer.result)
    }
}

/// The trade site of `language`, where its names are.
fn host(language: Language) -> &'static str {
    match language {
        Language::English => "https://www.pathofexile.com",
        Language::French => "https://fr.pathofexile.com",
    }
}

/// The names of the data of both languages.
fn item_names(updated: SystemTime, english: LanguageData, french: LanguageData) -> ItemNames {
    let uniques = unique_pairs(&english.items, &french.items);
    ItemNames::new(updated, names(english), names(french), uniques)
}

/// The names of one language: the items of the currency exchange, and the base types of the
/// equipment, each once.
fn names(data: LanguageData) -> Names {
    let exchange = data
        .exchange
        .into_iter()
        .flat_map(|group| group.entries)
        .filter(|entry| !entry.text.is_empty())
        .map(|entry| (entry.text, entry.id))
        .collect();
    let mut seen = HashSet::new();
    let base_types = data
        .items
        .into_iter()
        .filter(|category| !NOT_EQUIPMENT.contains(&category.id.as_str()))
        .flat_map(|category| category.entries)
        .filter(|item| item.name.is_none())
        .map(|item| item.base_type)
        .filter(|base_type| seen.insert(base_type.clone()))
        .collect();
    Names {
        exchange,
        base_types,
    }
}

/// The uniques of both languages side by side. They have no id, but each category lists
/// them in the same order in every language (checked on 2026-10-25); a category whose lists
/// differ in length is left out, rather than pairing the wrong names.
fn unique_pairs(english: &[ApiItemCategory], french: &[ApiItemCategory]) -> Vec<UniqueNames> {
    let mut pairs = Vec::new();
    for category in english {
        let Some(translated) = french.iter().find(|other| other.id == category.id) else {
            continue;
        };
        let english_uniques: Vec<&ApiItem> = uniques(category).collect();
        let french_uniques: Vec<&ApiItem> = uniques(translated).collect();
        if english_uniques.len() != french_uniques.len() {
            log::warn!(
                "The uniques of {} differ between English ({}) and French ({}): left out",
                category.id,
                english_uniques.len(),
                french_uniques.len()
            );
            continue;
        }
        pairs.extend(
            english_uniques
                .into_iter()
                .zip(french_uniques)
                // The older versions of a unique have the same name as the current one.
                .filter(|(english, french)| english.disc.is_none() && french.disc.is_none())
                .filter_map(|(english, french)| {
                    Some(UniqueNames {
                        english: unique_name(english)?,
                        french: unique_name(french)?,
                    })
                }),
        );
    }
    pairs
}

fn uniques(category: &ApiItemCategory) -> impl Iterator<Item = &ApiItem> {
    category.entries.iter().filter(|item| item.name.is_some())
}

fn unique_name(item: &ApiItem) -> Option<UniqueName> {
    Some(UniqueName {
        name: item.name.clone()?,
        base_type: item.base_type.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shape of `/data/items` on www.pathofexile.com on 2026-10-25, a few items of it.
    const ENGLISH_ITEMS: &str = r#"{"result": [
        {"id": "accessory", "label": "Accessories", "entries": [
            {"type": "Ruby Ring"},
            {"type": "Gold Ring", "text": "Andvarius Gold Ring", "name": "Andvarius", "flags": {"unique": true}}
        ]},
        {"id": "weapon", "label": "Weapons", "entries": [
            {"type": "Temple Maul"},
            {"type": "Rippled Greatsword"},
            {"type": "Makeshift Crossbow", "text": "Mist Whisper Makeshift Crossbow", "name": "Mist Whisper", "flags": {"unique": true}},
            {"type": "Shrine Sceptre", "text": "Guiding Palm Shrine Sceptre (Legacy)", "name": "Guiding Palm", "disc": "legacy", "flags": {"unique": true}}
        ]},
        {"id": "jewel", "label": "Jewels", "entries": [
            {"type": "Emerald", "text": "Grand Spectrum Emerald", "name": "Grand Spectrum", "flags": {"unique": true}},
            {"type": "Emerald", "text": "Prism Guardian Emerald", "name": "Prism Guardian", "flags": {"unique": true}}
        ]},
        {"id": "currency", "label": "Currency", "entries": [{"type": "Preserved Rib"}]}
    ]}"#;

    /// The same items on fr.pathofexile.com: a base type is missing, and so is a unique.
    const FRENCH_ITEMS: &str = r#"{"result": [
        {"id": "accessory", "label": "Accessoires", "entries": [
            {"type": "Bague de rubis"},
            {"type": "Bague en or", "text": "Andvarius Bague en or", "name": "Andvarius", "flags": {"unique": true}}
        ]},
        {"id": "weapon", "label": "Armes", "entries": [
            {"type": "Maillet du temple"},
            {"type": "Arbalète de fortune", "text": "Murmure des brumes Arbalète de fortune", "name": "Murmure des brumes", "flags": {"unique": true}},
            {"type": "Sceptre sanctuaire", "text": "Paume directrice Sceptre sanctuaire (Legacy)", "name": "Paume directrice", "disc": "legacy", "flags": {"unique": true}}
        ]},
        {"id": "jewel", "label": "Joyaux", "entries": [
            {"type": "Émeraude", "text": "Grand spectre Émeraude", "name": "Grand spectre", "flags": {"unique": true}}
        ]},
        {"id": "currency", "label": "Objets monétaires", "entries": [{"type": "Côte préservée"}]}
    ]}"#;

    /// The shape of `/data/static` on 2026-10-25.
    const ENGLISH_STATIC: &str = r#"{"result": [
        {"id": "Currency", "label": "Currency", "entries": [
            {"id": "divine", "text": "Divine Orb", "image": "/gen/image/divine.png"},
            {"id": "sep", "image": "/gen/image/separator.png"}
        ]}
    ]}"#;

    const FRENCH_STATIC: &str = r#"{"result": [
        {"id": "Currency", "label": "Objets monétaires", "entries": [
            {"id": "divine", "text": "Orbe divin", "image": "/gen/image/divine.png"}
        ]}
    ]}"#;

    fn data(items: &str, exchange: &str) -> LanguageData {
        let items: ApiResult<ApiItemCategory> = serde_json::from_str(items).expect("items");
        let exchange: ApiResult<ApiStaticGroup> = serde_json::from_str(exchange).expect("static");
        LanguageData {
            items: items.result,
            exchange: exchange.result,
        }
    }

    fn names_of_both() -> ItemNames {
        item_names(
            SystemTime::UNIX_EPOCH,
            data(ENGLISH_ITEMS, ENGLISH_STATIC),
            data(FRENCH_ITEMS, FRENCH_STATIC),
        )
    }

    #[test]
    fn the_items_of_the_currency_exchange_are_found_by_their_name_in_each_language() {
        let names = names_of_both();

        assert_eq!(
            names.exchange_id(Language::English, "Divine Orb"),
            Some("divine")
        );
        assert_eq!(
            names.exchange_id(Language::French, "Orbe divin"),
            Some("divine")
        );
        assert_eq!(names.exchange_id(Language::English, ""), None);
    }

    #[test]
    fn the_uniques_are_paired_in_their_order_without_the_older_versions() {
        let names = names_of_both();

        assert_eq!(
            names.english_unique(Language::French, "Murmure des brumes"),
            Some("Mist Whisper")
        );
        assert_eq!(
            names.english_unique(Language::French, "Andvarius"),
            Some("Andvarius")
        );
        assert_eq!(
            names.english_unique(Language::French, "Paume directrice"),
            None
        );
    }

    #[test]
    fn a_category_whose_uniques_differ_in_number_is_left_out() {
        let names = names_of_both();

        assert_eq!(
            names.english_unique(Language::French, "Grand spectre"),
            None
        );
        assert_eq!(names.counts().2, 2);
    }

    #[test]
    fn the_base_types_are_those_of_the_equipment_in_each_language() {
        let names = names_of_both();

        assert_eq!(
            names.magic_base_type(Language::English, "Crackling Temple Maul of the Brute"),
            Some("Temple Maul")
        );
        assert_eq!(
            names.magic_base_type(Language::French, "Maillet du temple crépitant"),
            Some("Maillet du temple")
        );
        assert_eq!(
            names.magic_base_type(Language::English, "Preserved Rib"),
            None,
            "not equipment"
        );
    }

    #[test]
    fn an_answer_without_the_expected_fields_is_an_error() {
        let parsed: Result<ApiResult<ApiItemCategory>, _> =
            serde_json::from_str(r#"{"result": [{"id": "weapon"}]}"#);

        assert!(parsed.is_err());
    }
}
