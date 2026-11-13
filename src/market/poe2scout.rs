//! The public API of poe2scout, the price database of the Path of Exile 2 economy.
//!
//! The rules of its use (pace, identification) are in `docs/poe2scout-usage.md`.

use std::fmt;
use std::time::{Duration, SystemTime};

use serde::Deserialize;
use serde::de::DeserializeOwned;

use super::{DownloadObserver, PriceDatabase};
use crate::http::{self, HttpError, Pace};
use crate::league::League;
use crate::logging;
use crate::price::{CurrencyPrice, Price, PriceList, Rates, UniqueModifiers, UniquePrice};

const BASE_URL: &str = "https://api.poe2scout.com/poe2";
/// The time between two requests, at least.
/// A little more than the second of the rule: a download of several leagues in a row must
/// not fit 61 requests in a minute.
const PACE: Duration = Duration::from_millis(1100);
/// The most items per page the API gives: most categories fit in one page.
const PER_PAGE: &str = "250";
/// The most pages read in a category, in case the API gave a wrong count.
const MAX_PAGES: u32 = 10;
/// The categories read at most, each being a request at least: about 25 are known, and an
/// answer with far more is wrong.
const MAX_CATEGORIES: usize = 100;
/// The ids of the three currencies of the prices in `/ReferenceCurrencies`.
const EXALTED: &str = "exalted";
const CHAOS: &str = "chaos";
const DIVINE: &str = "divine";
/// The id of the category of the currency, downloaded first.
const CURRENCY_CATEGORY: &str = "currency";

/// Why the price database couldn't be read.
#[derive(Debug)]
pub enum FetchError {
    /// No answer, an error status (e.g. 429 when its limits are reached), or an answer that
    /// isn't the expected JSON, to the request of this path; `after_answers` when earlier
    /// requests of the same download were answered: the database can be reached then, even if
    /// this request hung.
    Http {
        path: String,
        error: HttpError,
        after_answers: bool,
    },
    /// No current league in the answer: the database is wrong for a moment, and the league of
    /// the player must not be changed for that.
    NoCurrentLeague,
    /// No rate for this currency in the answer, so no price can be shown in it.
    MissingRate(&'static str),
    /// This many categories in the answer: reading them all would send as many requests.
    TooManyCategories(usize),
}

impl FetchError {
    /// The database answered: see [`HttpError::answered`]. An answer that lacks what the
    /// application needs is one too.
    #[must_use]
    pub fn answered(&self) -> bool {
        match self {
            Self::Http {
                error,
                after_answers,
                ..
            } => error.answered() || *after_answers,
            Self::NoCurrentLeague | Self::MissingRate(_) | Self::TooManyCategories(_) => true,
        }
    }
}

impl fmt::Display for FetchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // The path holds the name of a league, read from the database.
            Self::Http { path, error, .. } => {
                write!(formatter, "{error} ({})", logging::plain(path))
            }
            Self::NoCurrentLeague => formatter.write_str("no current league in the answer"),
            Self::MissingRate(currency) => write!(formatter, "no rate for the {currency} orb"),
            Self::TooManyCategories(count) => {
                write!(formatter, "{count} categories in the answer")
            }
        }
    }
}

impl std::error::Error for FetchError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Http { error, .. } => Some(error),
            Self::NoCurrentLeague | Self::MissingRate(_) | Self::TooManyCategories(_) => None,
        }
    }
}

/// A league in the answer of `/Leagues`.
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ApiLeague {
    value: String,
    is_current: bool,
}

/// The categories of the items in the answer of `/Items/Categories`.
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ApiCategories {
    #[serde(deserialize_with = "http::each")]
    unique_categories: Vec<ApiCategory>,
    #[serde(deserialize_with = "http::each")]
    currency_categories: Vec<ApiCategory>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ApiCategory {
    api_id: String,
}

impl ApiCategories {
    /// An error when the answer has more categories than the database can have.
    fn checked(self) -> Result<Self, FetchError> {
        let count = self.unique_categories.len() + self.currency_categories.len();
        if count > MAX_CATEGORIES {
            return Err(FetchError::TooManyCategories(count));
        }
        Ok(self)
    }
}

/// A currency of the answer of `/ReferenceCurrencies`, worth `relative_price` of the currency
/// the prices are given in.
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ApiReferenceCurrency {
    api_id: String,
    relative_price: f64,
}

/// A page of the items of a category; an item the application can't read is left out.
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase", bound(deserialize = "T: DeserializeOwned"))]
struct ApiPage<T> {
    pages: u32,
    #[serde(deserialize_with = "http::each")]
    items: Vec<T>,
}

/// A unique in the answer of `/Uniques/ByCategory`.
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ApiUnique {
    name: String,
    #[serde(rename = "Type")]
    base_type: String,
    item_metadata: Option<serde_json::Value>,
    #[serde(flatten)]
    price: ApiPrice,
}

/// The modifiers of a unique in its `ItemMetadata`, with their ranges, e.g. `+(17-23)% to
/// Chaos Resistance`. Read leniently: metadata or a list missing, `null` or of another shape,
/// and the values that aren't texts, lose these modifiers only, never the price of the unique.
fn unique_modifiers(metadata: Option<&serde_json::Value>) -> UniqueModifiers {
    let texts = |key: &str| -> Vec<String> {
        metadata
            .and_then(|metadata| metadata.get(key))
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|value| value.as_str().map(str::to_owned))
            .collect()
    };
    UniqueModifiers {
        implicit: texts("implicit_mods"),
        explicit: texts("explicit_mods"),
    }
}

/// The unique of the answer with its price in Exalted Orbs and its modifiers; none without a
/// current price.
fn unique_price(conversion: &Conversion, item: ApiUnique) -> Option<UniquePrice> {
    Some(UniquePrice {
        price: conversion.price(item.price)?,
        modifiers: unique_modifiers(item.item_metadata.as_ref()),
        name: item.name,
        base_type: item.base_type,
    })
}

/// An item of the currency exchange in the answer of `/Currencies/ByCategory`.
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ApiCurrency {
    api_id: String,
    text: String,
    #[serde(flatten)]
    price: ApiPrice,
}

/// The price of an item, in the currency the prices are given in; none for an item not
/// listed lately.
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ApiPrice {
    current_price: Option<f64>,
    current_quantity: Option<u32>,
    /// The daily prices, the most recent first; `null` for a day without price. Only drawn:
    /// a price without them is still a price.
    #[serde(default)]
    price_logs: Vec<Option<ApiPriceLog>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ApiPriceLog {
    price: f64,
}

/// The kinds of categories, each with its own path.
#[derive(Debug, Clone, Copy)]
enum Kind {
    Uniques,
    Currencies,
}

impl Kind {
    /// The path of the pages of a category of this kind in `league`, encoded for a path.
    fn path(self, league: &str) -> String {
        let kind = match self {
            Self::Uniques => "Uniques",
            Self::Currencies => "Currencies",
        };
        format!("/Leagues/{league}/{kind}/ByCategory")
    }
}

/// Where the answers come from: the API over the network, or a fake one in the tests.
pub(super) trait Api {
    /// The text of the answer of `path` with the parameters `query`.
    fn get(&mut self, path: &str, query: &[(&str, &str)]) -> Result<String, HttpError>;
}

/// The API of poe2scout, asked by an agent that identifies the application in each request.
pub struct Network(ureq::Agent);

impl Api for Network {
    fn get(&mut self, path: &str, query: &[(&str, &str)]) -> Result<String, HttpError> {
        http::get_text(&self.0, &format!("{BASE_URL}{path}"), query)
    }
}

/// The client of poe2scout, which spaces its requests, in the order that prices the most
/// copied items first, and stops a download at its first failure.
pub struct Poe2Scout<A = Network> {
    api: A,
    pace: Pace,
    /// A request of the download under way was answered ([`FetchError::Http`]).
    answered: bool,
}

impl Poe2Scout {
    /// The client of poe2scout over the network, at the pace of its rules.
    #[must_use]
    pub fn new() -> Self {
        Self::with_api(Network(http::agent()), PACE)
    }
}

impl<A: Api> Poe2Scout<A> {
    /// A client that asks `api`, `interval` between two requests at least.
    fn with_api(api: A, interval: Duration) -> Self {
        Self {
            api,
            pace: Pace::new(interval),
            answered: false,
        }
    }

    /// The answer of `path` with the parameters `query`, at the pace of the API.
    fn get<T: DeserializeOwned>(
        &mut self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<T, FetchError> {
        self.pace.wait();
        self.fetch(path, query)
    }

    /// The answer of `path`, at the pace of the API; `None` when `download` is interrupted
    /// once the pace waited, right before the request.
    fn get_unless_interrupted<T: DeserializeOwned>(
        &mut self,
        path: &str,
        query: &[(&str, &str)],
        download: &mut dyn DownloadObserver,
    ) -> Result<Option<T>, FetchError> {
        self.pace.wait();
        if download.interrupted() {
            return Ok(None);
        }
        self.fetch(path, query).map(Some)
    }

    /// The answer of `path` with the parameters `query`, at once.
    fn fetch<T: DeserializeOwned>(
        &mut self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<T, FetchError> {
        let answer = self
            .api
            .get(path, query)
            .and_then(|text| Ok(serde_json::from_str(&text)?));
        let after_answers = self.answered;
        self.answered |= answer.is_ok();
        answer.map_err(|error| FetchError::Http {
            path: path.to_owned(),
            error,
            after_answers,
        })
    }

    /// The items of every page of `category`, at `path` (see [`Kind::path`]); `None` when
    /// `download` is interrupted before a request.
    fn category<T: DeserializeOwned>(
        &mut self,
        path: &str,
        category: &ApiCategory,
        download: &mut dyn DownloadObserver,
    ) -> Result<Option<Vec<T>>, FetchError> {
        let category = category.api_id.as_str();
        let mut items = Vec::new();
        for page in 1..=MAX_PAGES {
            let page_text = page.to_string();
            let query = [
                ("category", category),
                ("perPage", PER_PAGE),
                ("page", page_text.as_str()),
            ];
            let Some(answer) = self.get_unless_interrupted::<ApiPage<T>>(path, &query, download)?
            else {
                return Ok(None);
            };
            items.extend(answer.items);
            if page >= answer.pages {
                return Ok(Some(items));
            }
        }
        log::warn!("More than {MAX_PAGES} pages of {category}: only the first ones are read");
        Ok(Some(items))
    }
}

impl<A: Api> PriceDatabase for Poe2Scout<A> {
    fn leagues(&mut self) -> Result<Vec<League>, FetchError> {
        self.answered = false;
        // Read strictly: a league left out because it can't be read would be taken for a
        // league that ended, and the player moved to another one.
        let leagues = current_leagues(self.get("/Leagues", &[])?);
        if leagues.is_empty() {
            return Err(FetchError::NoCurrentLeague);
        }
        Ok(leagues)
    }

    /// The currency categories come first: the most copied items are priced after a few
    /// seconds, before the uniques.
    fn prices(
        &mut self,
        league: &League,
        download: &mut dyn DownloadObserver,
    ) -> Result<Option<PriceList>, FetchError> {
        self.answered = false;
        let path = http::encode(league.id());
        let Some(categories) = self.get_unless_interrupted::<ApiCategories>(
            &format!("/Leagues/{path}/Items/Categories"),
            &[],
            download,
        )?
        else {
            return Ok(None);
        };
        let currencies_path = format!("/Leagues/{path}/ReferenceCurrencies");
        let Some(currencies) = self.get_unless_interrupted(&currencies_path, &[], download)? else {
            return Ok(None);
        };
        let conversion = Conversion::new(&reference_currencies(currencies))?;
        let categories = categories.checked()?;
        let currency_categories = download_order(categories.currency_categories);
        let mut prices = PriceList {
            league: league.clone(),
            updated: SystemTime::now(),
            rates: conversion.rates,
            uniques: Vec::new(),
            currencies: Vec::new(),
        };
        // The two requests above, then one per category (most fit in one page).
        let requests = 2 + currency_categories.len() + categories.unique_categories.len();
        let mut done = 2;
        #[expect(clippy::cast_precision_loss, reason = "a few dozen requests")]
        let share = |done: usize| done as f32 / requests as f32;
        download.progress(&prices, share(done));
        let currencies_pages = Kind::Currencies.path(&path);
        for category in &currency_categories {
            let Some(items) =
                self.category::<ApiCurrency>(&currencies_pages, category, download)?
            else {
                return Ok(None);
            };
            prices
                .currencies
                .extend(items.into_iter().filter_map(|item| {
                    Some(CurrencyPrice {
                        price: conversion.price(item.price)?,
                        name: item.text,
                        exchange_id: item.api_id,
                    })
                }));
            done += 1;
            download.progress(&prices, share(done));
        }
        let uniques_pages = Kind::Uniques.path(&path);
        for category in &categories.unique_categories {
            let Some(items) = self.category::<ApiUnique>(&uniques_pages, category, download)?
            else {
                return Ok(None);
            };
            prices.uniques.extend(
                items
                    .into_iter()
                    .filter_map(|item| unique_price(&conversion, item)),
            );
            done += 1;
            download.progress(&prices, share(done));
        }
        prices.updated = SystemTime::now();
        Ok(Some(prices))
    }
}

/// Turns the prices of the answers, given in the base currency of the league, into Exalted
/// Orbs.
struct Conversion {
    /// The worth of an Exalted Orb in the base currency.
    exalted: f64,
    rates: Rates,
}

impl Conversion {
    fn new(currencies: &[ApiReferenceCurrency]) -> Result<Self, FetchError> {
        let worth = |id: &'static str| {
            currencies
                .iter()
                .find(|currency| currency.api_id == id)
                .map(|currency| currency.relative_price)
                .filter(|worth| worth.is_finite() && *worth > 0.0)
                .ok_or(FetchError::MissingRate(id))
        };
        let exalted = worth(EXALTED)?;
        Ok(Self {
            exalted,
            rates: Rates {
                chaos: worth(CHAOS)? / exalted,
                divine: worth(DIVINE)? / exalted,
            },
        })
    }

    /// The price in Exalted Orbs; none for an item without a current price.
    fn price(&self, price: ApiPrice) -> Option<Price> {
        let current = price.current_price.filter(|price| price.is_finite())?;
        Some(Price {
            exalted: current / self.exalted,
            quantity: price.current_quantity.unwrap_or(0),
            history: price
                .price_logs
                .into_iter()
                .map(|log| log.map(|log| log.price / self.exalted))
                .collect(),
        })
    }
}

/// The reference currencies of the answer that can be read. Three of them are needed, which
/// [`Conversion::new`] asks for by name: the others, however many, are left out without failing
/// the answer.
fn reference_currencies(values: Vec<serde_json::Value>) -> Vec<ApiReferenceCurrency> {
    values
        .into_iter()
        .filter_map(|value| serde_json::from_value(value).ok())
        .collect()
}

/// The currency categories in their download order: the currency first, the most copied
/// items, then the others in the order of the API.
fn download_order(mut categories: Vec<ApiCategory>) -> Vec<ApiCategory> {
    // A stable sort on a `bool`: `false`, the currency, comes first.
    categories.sort_by_key(|category| category.api_id != CURRENCY_CATEGORY);
    categories
}

/// The current leagues of the answer, in its order.
fn current_leagues(leagues: Vec<ApiLeague>) -> Vec<League> {
    leagues
        .into_iter()
        .filter(|league| league.is_current)
        .map(|league| League::new(league.value))
        .collect()
}

#[cfg(test)]
mod tests;
