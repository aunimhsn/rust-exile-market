//! The official trade site of Path of Exile 2, in each language of the game client: for now,
//! the names of its items and the templates of its stats.
//!
//! The rules of its use are in `docs/trade-site-usage.md`.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::time::{Duration, Instant, SystemTime};

use serde::Deserialize;
use serde::de::DeserializeOwned;

use super::refusal::Refusal;
use super::transport::{Attempt, Guarded, Network, Request, Transport};
use crate::game_data::TradeGroup;
use crate::http::{self, HttpError, Pace};
use crate::item_names::{ItemNames, Names, StatTemplate, UniqueName, UniqueNames};
use crate::language::Language;
use crate::logging;

/// The time between two requests, at least: the data endpoints announce no limit.
const PACE: Duration = Duration::from_secs(1);
/// The categories of the item data whose items have no magic version: their entries aren't
/// base types of equipment.
const NOT_EQUIPMENT: [&str; 2] = ["currency", "gem"];
/// The most stat templates of the modifier lines an answer may hold in one language, six times
/// the real ones (7,984 in English and 7,985 in French, the security audit of 2026-11-14): more
/// is an answer gone wrong, refused, since its stats are grouped on the thread of the window.
const MAX_STAT_TEMPLATES: usize = 50_000;

/// Why the names of the trade site couldn't be read.
#[derive(Debug)]
pub enum NamesError {
    /// No answer, an error status or an answer that isn't the expected JSON, to the request
    /// of this path; `after_answers` when earlier requests of the same download were answered:
    /// the trade site can be reached then, even if this request hung.
    Http {
        path: String,
        error: HttpError,
        after_answers: bool,
    },
    /// The trade site refused the request (403, 429): nothing is sent to it before this
    /// time.
    Refused(u16, Instant),
    /// The trade site refused a request a moment ago: this one wasn't sent, and none is
    /// before this time.
    Wait(Instant),
    /// An answer without the names of a kind, in a language: the trade site is wrong for a
    /// moment, and the names of the cache must not be replaced for that.
    Missing(&'static str, Language),
    /// An answer with more stat templates than [`MAX_STAT_TEMPLATES`] in a language: the
    /// trade site is wrong for a moment, as for a kind of names missing.
    TooManyStatTemplates(usize, Language),
    /// No unique of English lines up with its French name.
    NoUniquePairs,
}

impl NamesError {
    /// The end of the wait, when the trade site refused the request: it must not be asked
    /// again soon.
    #[must_use]
    pub fn refused_until(&self) -> Option<Instant> {
        match self {
            Self::Refused(_, until) => Some(*until),
            Self::Http { .. }
            | Self::Wait(_)
            | Self::Missing(..)
            | Self::TooManyStatTemplates(..)
            | Self::NoUniquePairs => None,
        }
    }

    /// The trade site answered: see [`HttpError::answered`]. An answer that lacks what the
    /// application needs is one too.
    #[must_use]
    pub fn answered(&self) -> bool {
        match self {
            Self::Http {
                error,
                after_answers,
                ..
            } => error.answered() || *after_answers,
            Self::Refused(..)
            | Self::Wait(_)
            | Self::Missing(..)
            | Self::TooManyStatTemplates(..)
            | Self::NoUniquePairs => true,
        }
    }
}

impl fmt::Display for NamesError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Http { path, error, .. } => write!(formatter, "{error} ({path})"),
            Self::Refused(status, until) => write!(
                formatter,
                "refused with the status {status}, {} s to wait",
                until.saturating_duration_since(Instant::now()).as_secs()
            ),
            Self::Wait(until) => write!(
                formatter,
                "not sent, {} s to wait",
                until.saturating_duration_since(Instant::now()).as_secs()
            ),
            Self::Missing(kind, language) => write!(formatter, "no {kind} in {language:?}"),
            Self::TooManyStatTemplates(count, language) => write!(
                formatter,
                "{count} stat templates in {language:?}, more than {MAX_STAT_TEMPLATES}"
            ),
            Self::NoUniquePairs => formatter.write_str("no unique lines up in both languages"),
        }
    }
}

impl std::error::Error for NamesError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Http { error, .. } => Some(error),
            Self::Refused(..)
            | Self::Wait(_)
            | Self::Missing(..)
            | Self::TooManyStatTemplates(..)
            | Self::NoUniquePairs => None,
        }
    }
}

/// A list of the data endpoints. In it and in its groups, an element the application can't
/// read is left out ([`http::each`]): the other names are still worth having.
#[derive(Deserialize)]
#[serde(bound(deserialize = "T: DeserializeOwned"))]
struct ApiResult<T> {
    #[serde(deserialize_with = "http::each")]
    result: Vec<T>,
}

/// A category of `/data/items`.
#[derive(Deserialize)]
struct ApiItemCategory {
    id: String,
    #[serde(deserialize_with = "http::each")]
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
    #[serde(deserialize_with = "http::each")]
    entries: Vec<ApiStatic>,
}

#[derive(Deserialize)]
struct ApiStatic {
    id: String,
    #[serde(default)]
    text: String,
}

/// A group of `/data/stats`, e.g. `explicit`.
#[derive(Deserialize)]
struct ApiStatGroup {
    id: String,
    #[serde(deserialize_with = "http::each")]
    entries: Vec<ApiStat>,
}

#[derive(Deserialize)]
struct ApiStat {
    id: String,
    text: String,
}

/// The data of one language.
struct LanguageData {
    language: Language,
    items: Vec<ApiItemCategory>,
    exchange: Vec<ApiStaticGroup>,
    stats: Vec<ApiStatGroup>,
}

/// The client of the data of the trade site, which identifies the application, spaces its
/// requests, follows no redirection and sends nothing during the wait of a refusal.
pub struct TradeSite<T = Network> {
    transport: Guarded<T>,
    pace: Pace,
    /// A request of the download under way was answered ([`NamesError::Http`]).
    answered: bool,
}

impl TradeSite {
    #[must_use]
    pub fn new(refusal: Refusal) -> Self {
        Self::with_guarded(Guarded::network(refusal))
    }
}

impl<T: Transport> TradeSite<T> {
    /// The client of the data of the fake trade site `transport`, for the tests.
    #[cfg(test)]
    fn with_transport(transport: T, refusal: Refusal) -> Self {
        Self::with_guarded(Guarded::new(transport, refusal))
    }

    /// The client of the data of the trade site behind `transport`, at its pace.
    fn with_guarded(transport: Guarded<T>) -> Self {
        Self {
            transport,
            pace: Pace::new(PACE),
            answered: false,
        }
    }

    /// The names of the items and the templates of the stats in English and in French: six
    /// requests.
    pub fn item_names(&mut self) -> Result<ItemNames, NamesError> {
        self.answered = false;
        let english = self.language_data(Language::English)?;
        let french = self.language_data(Language::French)?;
        item_names(SystemTime::now(), &english, &french)
    }

    fn language_data(&mut self, language: Language) -> Result<LanguageData, NamesError> {
        Ok(LanguageData {
            language,
            items: self.data(language, "items")?,
            exchange: self.data(language, "static")?,
            stats: self.data(language, "stats")?,
        })
    }

    /// The result of the data endpoint `name` on the trade site of `language`. Not asked
    /// during the wait of a refusal; a refusal (403, 429) starts one, of the delay of its
    /// `Retry-After`, for the searches of offers too.
    fn data<A: DeserializeOwned>(
        &mut self,
        language: Language,
        name: &str,
    ) -> Result<Vec<A>, NamesError> {
        self.pace.wait();
        let path = format!("/api/trade2/data/{name}");
        let request = Request {
            url: format!("{}{path}", super::host(language)),
            body: None,
        };
        // Which of the six requests failed, and in which language.
        let after_answers = self.answered;
        let failed = |error: HttpError| NamesError::Http {
            path: format!("{path}, {language:?}"),
            error,
            after_answers,
        };
        let answer = match self.transport.send(&request) {
            Attempt::Waiting(until) => return Err(NamesError::Wait(until)),
            Attempt::Unanswered(error) => return Err(failed(HttpError::Request(error))),
            Attempt::Refused { answer, until } => {
                return Err(NamesError::Refused(answer.status, until));
            }
            Attempt::Answered(answer) => answer,
        };
        let result: ApiResult<A> = answer.into_json().map_err(failed)?;
        self.answered = true;
        Ok(result.result)
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

/// The names of one language: the items of the currency exchange, the base types of the
/// equipment, each once, and the templates of the stats of the modifier lines (not those of
/// the pseudo stats or of the Sanctum).
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
    let stats = data
        .stats
        .iter()
        .filter(|group| TradeGroup::of_line(&group.id).is_some())
        .flat_map(|group| &group.entries)
        .map(|stat| StatTemplate {
            id: stat.id.clone(),
            text: stat.text.clone(),
        })
        .collect();
    let names = Names {
        exchange,
        base_types,
        stats,
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
    if names.stats.is_empty() {
        return Err(NamesError::Missing("stat templates", language));
    }
    if names.stats.len() > MAX_STAT_TEMPLATES {
        return Err(NamesError::TooManyStatTemplates(
            names.stats.len(),
            language,
        ));
    }
    Ok(names)
}

/// The uniques of both languages side by side. They have no id, but each category lists
/// them in the same order in every language (checked on 2026-10-25). A category whose lists
/// don't line up is left out, rather than pairing the wrong names: the order is checked again
/// at each download.
fn unique_pairs(english: &[ApiItemCategory], french: &[ApiItemCategory]) -> Vec<UniqueNames> {
    let mut french_categories: HashMap<&str, &ApiItemCategory> = HashMap::new();
    for category in french {
        // The first of an id, as a search of the list would find.
        french_categories
            .entry(category.id.as_str())
            .or_insert(category);
    }
    let mut pairs = Vec::new();
    for category in english {
        let english_uniques: Vec<&ApiItem> = uniques(category).collect();
        if english_uniques.is_empty() {
            continue;
        }
        let Some(translated) = french_categories.get(category.id.as_str()) else {
            log::warn!(
                "No category {} in French: its uniques are left out",
                logging::plain(&category.id)
            );
            continue;
        };
        let french_uniques: Vec<&ApiItem> = uniques(translated).collect();
        if let Some(reason) = misalignment(&english_uniques, &french_uniques) {
            log::warn!(
                "The uniques of {} don't line up in English and French ({reason}): left out",
                logging::plain(&category.id)
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
mod tests;
