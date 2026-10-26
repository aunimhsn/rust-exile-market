//! The official trade site of Path of Exile 2, in each language of the game client: for now,
//! the names of its items.
//!
//! The rules of its use are in `roadmap.md` (trade site usage).

use std::collections::{HashMap, HashSet};
use std::fmt;
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

/// Why the names of the trade site couldn't be read.
#[derive(Debug)]
pub enum NamesError {
    Http(HttpError),
    /// An answer without the names of a kind, in a language: the trade site is wrong for a
    /// moment, and the names of the cache must not be replaced for that.
    Missing(&'static str, Language),
    /// No unique of English lines up with its French name.
    NoUniquePairs,
}

impl NamesError {
    /// The trade site refused the request (403, 429): it must not be asked again soon.
    #[must_use]
    pub fn is_refusal(&self) -> bool {
        matches!(
            self,
            Self::Http(HttpError::Request(ureq::Error::StatusCode(403 | 429)))
        )
    }
}

impl fmt::Display for NamesError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Http(error) => error.fmt(formatter),
            Self::Missing(kind, language) => write!(formatter, "no {kind} in {language:?}"),
            Self::NoUniquePairs => formatter.write_str("no unique lines up in both languages"),
        }
    }
}

impl std::error::Error for NamesError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Http(error) => Some(error),
            Self::Missing(..) | Self::NoUniquePairs => None,
        }
    }
}

impl From<HttpError> for NamesError {
    fn from(error: HttpError) -> Self {
        Self::Http(error)
    }
}

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
    language: Language,
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
    pub fn item_names(&mut self) -> Result<ItemNames, NamesError> {
        let english = self.language_data(Language::English)?;
        let french = self.language_data(Language::French)?;
        item_names(SystemTime::now(), &english, &french)
    }

    fn language_data(&mut self, language: Language) -> Result<LanguageData, HttpError> {
        Ok(LanguageData {
            language,
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
        let url = format!("{}/api/trade2/data/{name}", super::site(language));
        let answer: ApiResult<T> = http::get_json(&self.agent, &url, &[])?;
        Ok(answer.result)
    }
}

/// The names of the data of both languages; an error when a kind of names is missing.
fn item_names(
    updated: SystemTime,
    english: &LanguageData,
    french: &LanguageData,
) -> Result<ItemNames, NamesError> {
    // Each language first: an empty answer is told once, with its language.
    let english_names = names(english)?;
    let french_names = names(french)?;
    let uniques = unique_pairs(&english.items, &french.items);
    if uniques.is_empty() {
        return Err(NamesError::NoUniquePairs);
    }
    Ok(ItemNames::new(
        updated,
        english_names,
        french_names,
        uniques,
    ))
}

/// The names of one language: the items of the currency exchange, and the base types of the
/// equipment, each once.
fn names(data: &LanguageData) -> Result<Names, NamesError> {
    let language = data.language;
    let exchange = data
        .exchange
        .iter()
        .flat_map(|group| &group.entries)
        .filter(|entry| !entry.text.is_empty())
        .map(|entry| (entry.text.clone(), entry.id.clone()))
        .collect();
    let mut seen = HashSet::new();
    let base_types = data
        .items
        .iter()
        .filter(|category| !NOT_EQUIPMENT.contains(&category.id.as_str()))
        .flat_map(|category| &category.entries)
        .filter(|item| item.name.is_none())
        .map(|item| item.base_type.clone())
        .filter(|base_type| seen.insert(base_type.clone()))
        .collect();
    let names = Names {
        exchange,
        base_types,
    };
    if names.exchange.is_empty() {
        return Err(NamesError::Missing(
            "items of the currency exchange",
            language,
        ));
    }
    if names.base_types.is_empty() {
        return Err(NamesError::Missing("base types", language));
    }
    Ok(names)
}

/// The uniques of both languages side by side. They have no id, but each category lists
/// them in the same order in every language (checked on 2026-10-25). A category whose lists
/// don't line up is left out, rather than pairing the wrong names: the order is checked again
/// at each download.
fn unique_pairs(english: &[ApiItemCategory], french: &[ApiItemCategory]) -> Vec<UniqueNames> {
    let mut pairs = Vec::new();
    for category in english {
        let english_uniques: Vec<&ApiItem> = uniques(category).collect();
        if english_uniques.is_empty() {
            continue;
        }
        let Some(translated) = french.iter().find(|other| other.id == category.id) else {
            log::warn!(
                "No category {} in French: its uniques are left out",
                category.id
            );
            continue;
        };
        let french_uniques: Vec<&ApiItem> = uniques(translated).collect();
        if let Some(reason) = misalignment(&english_uniques, &french_uniques) {
            log::warn!(
                "The uniques of {} don't line up in English and French ({reason}): left out",
                category.id
            );
            continue;
        }
        pairs.extend(
            english_uniques
                .into_iter()
                .zip(french_uniques)
                // The older versions of a unique have the same name as the current one.
                .filter(|(english, _)| english.disc.is_none())
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

/// Why two lists of uniques of a category don't line up, if they don't: another number of
/// uniques, a variant (`legacy`) in one language only, or a base type with two translations,
/// either way. Two uniques of the same base swapped in one language can't be seen: their
/// names would be paired wrong (checked on the real data of 2026-10-25, where every category
/// lines up).
fn misalignment(english: &[&ApiItem], french: &[&ApiItem]) -> Option<String> {
    if english.len() != french.len() {
        return Some(format!("{} and {} uniques", english.len(), french.len()));
    }
    let mut translations: HashMap<&str, &str> = HashMap::new();
    let mut originals: HashMap<&str, &str> = HashMap::new();
    for (english, french) in english.iter().zip(french) {
        let (english_base, french_base) = (english.base_type.as_str(), french.base_type.as_str());
        if english.disc != french.disc {
            return Some(format!("a variant of {english_base} in one language only"));
        }
        if *translations.entry(english_base).or_insert(french_base) != french_base
            || *originals.entry(french_base).or_insert(english_base) != english_base
        {
            return Some(format!(
                "{english_base} and {french_base} paired differently"
            ));
        }
    }
    None
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

    fn data(language: Language, items: &str, exchange: &str) -> LanguageData {
        let items: ApiResult<ApiItemCategory> = serde_json::from_str(items).expect("items");
        let exchange: ApiResult<ApiStaticGroup> = serde_json::from_str(exchange).expect("static");
        LanguageData {
            language,
            items: items.result,
            exchange: exchange.result,
        }
    }

    fn names_of_both() -> ItemNames {
        item_names(
            SystemTime::UNIX_EPOCH,
            &data(Language::English, ENGLISH_ITEMS, ENGLISH_STATIC),
            &data(Language::French, FRENCH_ITEMS, FRENCH_STATIC),
        )
        .expect("every kind of names")
    }

    /// The uniques of a category, as `misalignment` takes them.
    fn list(items: &[ApiItem]) -> Vec<&ApiItem> {
        items.iter().collect()
    }

    /// A unique of the item data.
    fn item(base_type: &str, name: &str, disc: Option<&str>) -> ApiItem {
        ApiItem {
            base_type: base_type.to_owned(),
            name: Some(name.to_owned()),
            disc: disc.map(str::to_owned),
        }
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
        assert_eq!(names.counts().uniques, 2);
    }

    #[test]
    fn lists_of_the_same_length_that_dont_line_up_are_found() {
        let english = [
            item("Emerald", "Grand Spectrum", None),
            item("Ruby", "Lioneye's Fall", None),
            item("Emerald", "Prism Guardian", None),
        ];
        // Shifted by one: the emerald of English is paired with the ruby of French.
        let shifted = [
            item("Rubis", "Chute d'Œil-de-lion", None),
            item("Émeraude", "Grand spectre", None),
            item("Émeraude", "Gardien prismatique", None),
        ];
        let legacy = [
            item("Émeraude", "Grand spectre", Some("legacy")),
            item("Rubis", "Chute d'Œil-de-lion", None),
            item("Émeraude", "Gardien prismatique", None),
        ];
        let lined_up = [
            item("Émeraude", "Grand spectre", None),
            item("Rubis", "Chute d'Œil-de-lion", None),
            item("Émeraude", "Gardien prismatique", None),
        ];

        assert!(misalignment(&list(&english), &list(&shifted)).is_some());
        assert!(misalignment(&list(&english), &list(&legacy)).is_some());
        assert_eq!(misalignment(&list(&english), &list(&lined_up)), None);
    }

    #[test]
    fn a_base_type_paired_with_two_others_is_found_either_way() {
        let english = [
            item("Emerald", "Grand Spectrum", None),
            item("Ruby", "Lioneye's Fall", None),
        ];
        // Each English base has one translation, but the French one has two originals.
        let french = [
            item("Émeraude", "Grand spectre", None),
            item("Émeraude", "Chute d'Œil-de-lion", None),
        ];

        assert!(misalignment(&list(&english), &list(&french)).is_some());
    }

    #[test]
    fn the_uniques_of_a_category_missing_in_french_are_left_out() {
        let english: ApiResult<ApiItemCategory> = serde_json::from_str(ENGLISH_ITEMS).unwrap();
        let french: ApiResult<ApiItemCategory> = serde_json::from_str(FRENCH_ITEMS).unwrap();
        let without_weapons: Vec<ApiItemCategory> = french
            .result
            .into_iter()
            .filter(|category| category.id != "weapon")
            .collect();

        let pairs = unique_pairs(&english.result, &without_weapons);

        let names: Vec<&str> = pairs
            .iter()
            .map(|pair| pair.english.name.as_str())
            .collect();
        assert_eq!(names, ["Andvarius"]);
    }

    #[test]
    fn a_refusal_of_the_trade_site_is_told_apart() {
        let status = |code| NamesError::Http(HttpError::Request(ureq::Error::StatusCode(code)));

        assert!(status(429).is_refusal());
        assert!(status(403).is_refusal());
        assert!(!status(503).is_refusal());
        assert!(!NamesError::NoUniquePairs.is_refusal());
    }

    #[test]
    fn an_answer_without_a_kind_of_names_is_an_error() {
        let empty = r#"{"result": []}"#;

        let names = item_names(
            SystemTime::UNIX_EPOCH,
            &data(Language::English, ENGLISH_ITEMS, ENGLISH_STATIC),
            &data(Language::French, FRENCH_ITEMS, empty),
        );

        assert!(matches!(
            names,
            Err(NamesError::Missing(_, Language::French))
        ));

        let english_empty = item_names(
            SystemTime::UNIX_EPOCH,
            &data(Language::English, empty, ENGLISH_STATIC),
            &data(Language::French, FRENCH_ITEMS, FRENCH_STATIC),
        );
        assert!(matches!(
            english_empty,
            Err(NamesError::Missing(_, Language::English))
        ));
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
