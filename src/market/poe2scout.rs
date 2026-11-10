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
use crate::prices::{CurrencyPrice, Price, PriceList, Rates, UniqueModifiers, UniquePrice};

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
    fn path(self) -> &'static str {
        match self {
            Self::Uniques => "Uniques",
            Self::Currencies => "Currencies",
        }
    }
}

/// Where the answers come from: the API over the network, or a fake one in the tests.
pub(super) trait Source {
    /// The text of the answer of `path` with the parameters `query`.
    fn get(&mut self, path: &str, query: &[(&str, &str)]) -> Result<String, HttpError>;
}

/// The API of poe2scout, asked by an agent that identifies the application in each request.
pub struct Network(ureq::Agent);

impl Source for Network {
    fn get(&mut self, path: &str, query: &[(&str, &str)]) -> Result<String, HttpError> {
        http::get_text(&self.0, &format!("{BASE_URL}{path}"), query)
    }
}

/// The client of poe2scout, which spaces its requests, in the order that prices the most
/// copied items first, and stops a download at its first failure.
pub struct Poe2Scout<S = Network> {
    source: S,
    pace: Pace,
    /// A request of the download under way was answered ([`FetchError::Http`]).
    answered: bool,
}

impl Poe2Scout {
    #[must_use]
    pub fn new() -> Self {
        Self::with_source(Network(http::agent()), PACE)
    }
}

impl<S: Source> Poe2Scout<S> {
    /// A client that asks `source`, `interval` between two requests at least.
    fn with_source(source: S, interval: Duration) -> Self {
        Self {
            source,
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
            .source
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

    /// The items of every page of a category; `None` when `download` is interrupted before a
    /// request.
    fn category<T: DeserializeOwned>(
        &mut self,
        league: &str,
        kind: Kind,
        category: &str,
        download: &mut dyn DownloadObserver,
    ) -> Result<Option<Vec<T>>, FetchError> {
        let path = format!("/Leagues/{league}/{}/ByCategory", kind.path());
        let mut items = Vec::new();
        for page in 1..=MAX_PAGES {
            let page_text = page.to_string();
            let query = [
                ("category", category),
                ("perPage", PER_PAGE),
                ("page", page_text.as_str()),
            ];
            let Some(answer) =
                self.get_unless_interrupted::<ApiPage<T>>(&path, &query, download)?
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

impl<S: Source> PriceDatabase for Poe2Scout<S> {
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
        for category in &currency_categories {
            let Some(items) =
                self.category::<ApiCurrency>(&path, Kind::Currencies, &category.api_id, download)?
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
        for category in &categories.unique_categories {
            let Some(items) =
                self.category::<ApiUnique>(&path, Kind::Uniques, &category.api_id, download)?
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
mod tests {
    use std::time::Instant;

    use super::*;

    /// The security audit of 2026-11-10: each category is a request, a second apart.
    #[test]
    fn an_answer_with_too_many_categories_is_refused() {
        let categories = |uniques: usize, currencies: usize| {
            let list = |count: usize| {
                (0..count)
                    .map(|index| ApiCategory {
                        api_id: index.to_string(),
                    })
                    .collect()
            };
            ApiCategories {
                unique_categories: list(uniques),
                currency_categories: list(currencies),
            }
        };

        assert!(categories(40, MAX_CATEGORIES - 40).checked().is_ok());
        assert!(matches!(
            categories(40, MAX_CATEGORIES - 39).checked(),
            Err(FetchError::TooManyCategories(count)) if count == MAX_CATEGORIES + 1
        ));
    }

    /// The shape of the answer of `/Leagues` on 2026-10-25, without the fields not read.
    const LEAGUES: &str = r#"[
        {"Value": "Forbidden Rites", "ShortName": "forbiddenrites", "IsCurrent": true, "DivinePrice": 718.4},
        {"Value": "HC Forbidden Rites", "ShortName": "forbiddenriteshc", "IsCurrent": true},
        {"Value": "Dawn of the Hunt", "ShortName": "hunt", "IsCurrent": false},
        {"Value": "Standard", "ShortName": "standard", "IsCurrent": false},
        {"Value": "Runes of Aldur", "ShortName": "runes", "IsCurrent": true}
    ]"#;

    #[test]
    fn reads_the_current_leagues_in_their_order() {
        let leagues: Vec<ApiLeague> = serde_json::from_str(LEAGUES).expect("the answer parses");

        assert_eq!(
            current_leagues(leagues),
            ["Forbidden Rites", "HC Forbidden Rites", "Runes of Aldur"].map(League::new)
        );
    }

    /// The shape of the answer of `/ReferenceCurrencies` on 2026-10-25.
    const REFERENCE_CURRENCIES: &str = r#"[
        {"ApiId": "exalted", "Text": "Exalted Orb", "RelativePrice": 1},
        {"ApiId": "chaos", "Text": "Chaos Orb", "RelativePrice": 75.4},
        {"ApiId": "divine", "Text": "Divine Orb", "RelativePrice": 754.0}
    ]"#;

    /// The shape of a page of `/Uniques/ByCategory` on 2026-10-25, without most of the
    /// fields not read.
    const UNIQUES: &str = r#"{"CurrentPage": 1, "Pages": 1, "Total": 2, "Items": [
        {"UniqueItemId": 361, "Text": "Bursting Decay Unset Ring", "Name": "Bursting Decay",
         "Type": "Unset Ring", "IsChanceable": false,
         "ItemMetadata": {"name": "Bursting Decay", "base_type": "Unset Ring",
          "implicit_mods": ["Grants (1-1) additional Skill Slot"],
          "explicit_mods": ["(15-25)% increased Rarity of Items found", "+(17-23)% to Chaos Resistance"]},
         "PriceLogs": [{"Price": 149.5, "Time": "2026-10-25T00:00:00Z", "Quantity": 4}, null],
         "CurrentPrice": 150.8, "CurrentQuantity": 4},
        {"UniqueItemId": 1, "Name": "Unlisted", "Type": "Ruby Ring", "PriceLogs": [],
         "CurrentPrice": null, "CurrentQuantity": null}
    ]}"#;

    /// The shape of a page of `/Currencies/ByCategory` on 2026-10-25.
    const CURRENCIES: &str = r#"{"CurrentPage": 1, "Pages": 1, "Total": 1, "Items": [
        {"CurrencyItemId": 18, "ApiId": "mirror", "Text": "Mirror of Kalandra",
         "CategoryApiId": "currency", "ItemMetadata": null,
         "PriceLogs": [{"Price": 4207102, "Time": "2026-10-25T00:00:00Z", "Quantity": 125}],
         "CurrentPrice": 4259889, "CurrentQuantity": 15}
    ]}"#;

    fn conversion() -> Conversion {
        let currencies: Vec<ApiReferenceCurrency> =
            serde_json::from_str(REFERENCE_CURRENCIES).expect("the answer parses");
        Conversion::new(&currencies).expect("every rate is there")
    }

    #[test]
    fn reads_the_rates_of_the_chaos_and_divine_orbs_in_exalted_orbs() {
        assert_eq!(
            conversion().rates,
            Rates {
                chaos: 75.4,
                divine: 754.0
            }
        );
    }

    #[test]
    fn a_missing_rate_is_an_error() {
        let currencies: Vec<ApiReferenceCurrency> =
            serde_json::from_str(r#"[{"ApiId": "exalted", "RelativePrice": 1}]"#).unwrap();

        assert!(matches!(
            Conversion::new(&currencies),
            Err(FetchError::MissingRate(CHAOS))
        ));
    }

    #[test]
    fn the_prices_are_converted_to_exalted_orbs() {
        // Prices given in Chaos Orbs, in a league whose base currency would be the chaos.
        let currencies: Vec<ApiReferenceCurrency> = serde_json::from_str(
            r#"[{"ApiId": "exalted", "RelativePrice": 0.5},
                {"ApiId": "chaos", "RelativePrice": 1},
                {"ApiId": "divine", "RelativePrice": 10}]"#,
        )
        .unwrap();
        let conversion = Conversion::new(&currencies).unwrap();
        let page: ApiPage<ApiUnique> = serde_json::from_str(UNIQUES).unwrap();
        let price = page.items.into_iter().next().unwrap().price;

        let price = conversion.price(price).expect("a current price");

        assert!((price.exalted - 301.6).abs() < 1e-9);
        assert_eq!(conversion.rates.divine, 20.0);
    }

    #[test]
    fn reads_the_uniques_and_skips_those_without_a_price() {
        let page: ApiPage<ApiUnique> = serde_json::from_str(UNIQUES).expect("the answer parses");
        let conversion = conversion();
        let prices: Vec<(String, String, Option<Price>)> = page
            .items
            .into_iter()
            .map(|item| (item.name, item.base_type, conversion.price(item.price)))
            .collect();

        assert_eq!(
            prices,
            [
                (
                    "Bursting Decay".to_owned(),
                    "Unset Ring".to_owned(),
                    Some(Price {
                        exalted: 150.8,
                        quantity: 4,
                        history: vec![Some(149.5), None],
                    })
                ),
                ("Unlisted".to_owned(), "Ruby Ring".to_owned(), None),
            ]
        );
    }

    #[test]
    fn reads_the_implicit_and_explicit_modifiers_of_a_unique() {
        let page: ApiPage<ApiUnique> = serde_json::from_str(UNIQUES).expect("the answer parses");
        let modifiers: Vec<UniqueModifiers> = page
            .items
            .into_iter()
            .map(|item| unique_modifiers(item.item_metadata.as_ref()))
            .collect();

        assert_eq!(
            modifiers[0].implicit,
            ["Grants (1-1) additional Skill Slot"]
        );
        assert_eq!(
            modifiers[0].explicit,
            [
                "(15-25)% increased Rarity of Items found",
                "+(17-23)% to Chaos Resistance"
            ]
        );
        assert_eq!(modifiers[1], UniqueModifiers::default());
    }

    #[test]
    fn modifiers_null_or_unexpected_lose_only_themselves() {
        let page: ApiPage<ApiUnique> = serde_json::from_str(
            r#"{"Pages": 1, "Items": [{"Name": "A", "Type": "Ruby Ring",
                "ItemMetadata": {"implicit_mods": null, "explicit_mods": ["+(5-10) to Life", 3]},
                "PriceLogs": [], "CurrentPrice": 1.0, "CurrentQuantity": 1}]}"#,
        )
        .expect("the page still parses");
        let modifiers = unique_modifiers(page.items[0].item_metadata.as_ref());

        assert_eq!(modifiers.implicit, Vec::<String>::new());
        assert_eq!(modifiers.explicit, ["+(5-10) to Life"]);
        for metadata in ["[]", r#"{"implicit_mods": "text"}"#, "null"] {
            let metadata: serde_json::Value = serde_json::from_str(metadata).expect("json");
            assert_eq!(
                unique_modifiers(Some(&metadata)),
                UniqueModifiers::default(),
                "{metadata}"
            );
        }
    }

    #[test]
    fn a_unique_keeps_its_price_and_its_modifiers() {
        let page: ApiPage<ApiUnique> = serde_json::from_str(UNIQUES).expect("the answer parses");
        let conversion = conversion();
        let uniques: Vec<UniquePrice> = page
            .items
            .into_iter()
            .filter_map(|item| unique_price(&conversion, item))
            .collect();

        assert_eq!(uniques.len(), 1, "the unique without a price is skipped");
        assert_eq!(uniques[0].name, "Bursting Decay");
        assert_eq!(uniques[0].price.exalted, 150.8);
        assert_eq!(
            uniques[0].modifiers.implicit,
            ["Grants (1-1) additional Skill Slot"]
        );
        assert_eq!(uniques[0].modifiers.explicit.len(), 2);
    }

    #[test]
    fn a_unique_with_unexpected_modifiers_keeps_its_price() {
        let page: ApiPage<ApiUnique> = serde_json::from_str(
            r#"{"Pages": 1, "Items": [{"Name": "A", "Type": "Ruby Ring",
                "ItemMetadata": {"implicit_mods": "text", "explicit_mods": {"mod": 3}},
                "PriceLogs": [], "CurrentPrice": 2.0, "CurrentQuantity": 1}]}"#,
        )
        .expect("the page still parses");
        let item = page.items.into_iter().next().unwrap();

        let unique = unique_price(&conversion(), item).expect("a current price");

        assert_eq!(unique.price.exalted, 2.0);
        assert_eq!(unique.modifiers, UniqueModifiers::default());
    }

    #[test]
    fn reads_the_items_of_the_currency_exchange() {
        let page: ApiPage<ApiCurrency> =
            serde_json::from_str(CURRENCIES).expect("the answer parses");
        let mirror = page.items.into_iter().next().unwrap();

        assert_eq!(
            (mirror.text.as_str(), mirror.api_id.as_str()),
            ("Mirror of Kalandra", "mirror")
        );
        assert_eq!(
            conversion().price(mirror.price).map(|price| price.quantity),
            Some(15)
        );
    }

    #[test]
    fn the_currency_category_is_downloaded_first() {
        let categories: ApiCategories = serde_json::from_str(
            r#"{"UniqueCategories": [], "CurrencyCategories": [
                {"ApiId": "fragments"}, {"ApiId": "runes"}, {"ApiId": "currency"}, {"ApiId": "abyss"}
            ]}"#,
        )
        .expect("the answer parses");

        let order: Vec<String> = download_order(categories.currency_categories)
            .into_iter()
            .map(|category| category.api_id)
            .collect();

        assert_eq!(order, ["currency", "fragments", "runes", "abyss"]);
    }

    #[test]
    fn an_answer_without_the_expected_fields_is_an_error() {
        let parsed: Result<Vec<ApiLeague>, _> = serde_json::from_str(r#"[{"Name": "x"}]"#);

        assert!(parsed.is_err());
    }

    /// The network review of 2026-11-10: one odd item lost the 250 prices of its page, and
    /// the whole download with it.
    #[test]
    fn an_item_that_cant_be_read_is_left_out_of_its_page() {
        // A unique whose base type is `null`, before the two of the answer.
        let page = UNIQUES.replacen(
            r#""Items": ["#,
            r#""Items": [{"Name": "Odd", "Type": null, "PriceLogs": []},"#,
            1,
        );

        let page: ApiPage<ApiUnique> = serde_json::from_str(&page).expect("the answer parses");

        let names: Vec<&str> = page.items.iter().map(|item| item.name.as_str()).collect();
        assert_eq!(names, ["Bursting Decay", "Unlisted"]);
    }

    /// Not a category without prices: the database changed its answer.
    #[test]
    fn a_page_none_of_whose_items_can_be_read_is_an_error() {
        let page = CURRENCIES.replace("ApiId", "Id");

        assert!(serde_json::from_str::<ApiPage<ApiCurrency>>(&page).is_err());
    }

    #[test]
    fn a_price_is_read_without_its_history() {
        let page = CURRENCIES.replace("PriceLogs", "History");
        let page: ApiPage<ApiCurrency> = serde_json::from_str(&page).expect("the answer parses");

        let price = page.items.into_iter().next().map(|item| item.price);

        let price = conversion()
            .price(price.expect("an item"))
            .expect("a price");
        assert!((price.exalted - 4_259_889.0).abs() < 1e-6);
        assert_eq!(price.history, [None; 0]);
    }

    /// Whatever the other currencies of the answer: only the three rates are needed.
    #[test]
    fn the_reference_currencies_that_cant_be_read_are_left_out_however_many() {
        let odd = r#", {"ApiId": "mirror"}, {"ApiId": "vaal"}, {"ApiId": "regal"}, 5]"#;
        let values: Vec<serde_json::Value> =
            serde_json::from_str(&REFERENCE_CURRENCIES.replacen(']', odd, 1)).expect("a list");

        let currencies = reference_currencies(values);

        assert_eq!(currencies.len(), 3);
        assert!(Conversion::new(&currencies).is_ok());
        assert!(matches!(
            Conversion::new(&reference_currencies(vec![serde_json::Value::from(5)])),
            Err(FetchError::MissingRate(_))
        ));
    }

    #[test]
    fn a_failure_names_the_request_and_tells_whether_the_database_answered() {
        let failure = |error, after_answers| FetchError::Http {
            path: "/Leagues".to_owned(),
            error: HttpError::Request(error),
            after_answers,
        };
        let hung = || ureq::Error::Timeout(ureq::Timeout::Global);

        let refused = failure(ureq::Error::StatusCode(500), false);
        assert!(refused.answered());
        assert!(refused.to_string().ends_with("(/Leagues)"), "{refused}");
        assert!(!failure(hung(), false).answered());
        // The twentieth request of a download hangs: the database was there for the others.
        assert!(failure(hung(), true).answered());
    }

    /// The code review of the quality fixes: left out, a league that can't be read was taken
    /// for a league that ended, and the player moved to Standard. `leagues` reads a plain
    /// `Vec`, which this pins; `the_leagues_are_read_whole_or_not_at_all` asks for them.
    #[test]
    fn the_list_of_the_leagues_has_no_lenient_reading() {
        let leagues = r#"[{"Value": "Standard", "IsCurrent": true}, {"Value": "Forbidden Rites"}]"#;

        assert!(serde_json::from_str::<Vec<ApiLeague>>(leagues).is_err());
    }

    /// An API that answers each request it knows with its text, any other with the status
    /// 404, and keeps the requests it got, written `path?name=value&…`, with their times.
    #[derive(Default)]
    struct FakeApi {
        answers: Vec<(String, String)>,
        /// The request that gets no answer.
        hangs_at: Option<String>,
        requests: Vec<String>,
        times: Vec<Instant>,
    }

    impl FakeApi {
        fn with(mut self, request: impl Into<String>, answer: &str) -> Self {
            self.answers.push((request.into(), answer.to_owned()));
            self
        }

        fn hanging_at(mut self, request: &str) -> Self {
            self.hangs_at = Some(request.to_owned());
            self
        }
    }

    impl Source for FakeApi {
        fn get(&mut self, path: &str, query: &[(&str, &str)]) -> Result<String, HttpError> {
            let pairs: Vec<String> = query
                .iter()
                .map(|(name, value)| format!("{name}={value}"))
                .collect();
            let request = if pairs.is_empty() {
                path.to_owned()
            } else {
                format!("{path}?{}", pairs.join("&"))
            };
            self.times.push(Instant::now());
            self.requests.push(request.clone());
            if self.hangs_at.as_deref() == Some(request.as_str()) {
                return Err(ureq::Error::Timeout(ureq::Timeout::Global).into());
            }
            self.answers
                .iter()
                .find(|(known, _)| *known == request)
                .map(|(_, answer)| answer.clone())
                .ok_or_else(|| ureq::Error::StatusCode(404).into())
        }
    }

    /// What a download told, interrupted at its check number `stop_at` (from 0).
    #[derive(Default)]
    struct Download {
        stop_at: Option<usize>,
        checks: usize,
        /// The currencies and the uniques priced so far, and the share done.
        progress: Vec<(usize, usize, f32)>,
    }

    impl DownloadObserver for Download {
        fn interrupted(&mut self) -> bool {
            self.checks += 1;
            self.stop_at.is_some_and(|stop| self.checks > stop)
        }

        fn progress(&mut self, prices: &PriceList, progress: f32) {
            self.progress
                .push((prices.currencies.len(), prices.uniques.len(), progress));
        }
    }

    const CATEGORIES: &str = r#"{"UniqueCategories": [{"ApiId": "accessory"}],
        "CurrencyCategories": [{"ApiId": "fragments"}, {"ApiId": "currency"}]}"#;
    const EMPTY_PAGE: &str = r#"{"CurrentPage": 1, "Pages": 1, "Total": 0, "Items": []}"#;

    const CATEGORIES_PATH: &str = "/Leagues/HC%20Forbidden%20Rites/Items/Categories";
    const RATES_PATH: &str = "/Leagues/HC%20Forbidden%20Rites/ReferenceCurrencies";

    /// The request of the page `page` of a category of `kind` (`Currencies` or `Uniques`).
    fn page_request(kind: &str, category: &str, page: u32) -> String {
        format!(
            "/Leagues/HC%20Forbidden%20Rites/{kind}/ByCategory?category={category}&perPage=250&page={page}"
        )
    }

    /// An API with the five answers of a whole download of the league of [`league`].
    fn api() -> FakeApi {
        FakeApi::default()
            .with(CATEGORIES_PATH, CATEGORIES)
            .with(RATES_PATH, REFERENCE_CURRENCIES)
            .with(page_request("Currencies", "currency", 1), CURRENCIES)
            .with(page_request("Currencies", "fragments", 1), EMPTY_PAGE)
            .with(page_request("Uniques", "accessory", 1), UNIQUES)
    }

    fn league() -> League {
        League::new("HC Forbidden Rites")
    }

    /// A client of `api` that doesn't wait between two requests.
    fn scout(api: FakeApi) -> Poe2Scout<FakeApi> {
        Poe2Scout::with_source(api, Duration::ZERO)
    }

    /// The network review of 2026-11-10: nothing ran the download itself, only its parts.
    #[test]
    fn a_download_asks_for_the_rates_then_the_currency_then_the_uniques() {
        let mut scout = scout(api());
        let mut download = Download::default();

        let prices = scout
            .prices(&league(), &mut download)
            .expect("every request answered")
            .expect("not interrupted");

        assert_eq!(
            scout.source.requests,
            [
                CATEGORIES_PATH.to_owned(),
                RATES_PATH.to_owned(),
                page_request("Currencies", "currency", 1),
                page_request("Currencies", "fragments", 1),
                page_request("Uniques", "accessory", 1),
            ]
        );
        assert_eq!(prices.league, league());
        assert_eq!(prices.rates, conversion().rates);
        let currencies: Vec<&str> = prices
            .currencies
            .iter()
            .map(|currency| currency.exchange_id.as_str())
            .collect();
        assert_eq!(currencies, ["mirror"]);
        let uniques: Vec<&str> = prices
            .uniques
            .iter()
            .map(|unique| unique.name.as_str())
            .collect();
        assert_eq!(uniques, ["Bursting Decay"], "the one with a price");
    }

    /// The prices of the currency reach the application before the uniques are asked for.
    #[test]
    fn a_download_tells_its_prices_after_each_category() {
        let mut download = Download::default();

        scout(api())
            .prices(&league(), &mut download)
            .expect("every request answered");

        assert_eq!(
            download.progress,
            [(0, 0, 0.4), (1, 0, 0.6), (1, 0, 0.8), (1, 1, 1.0)]
        );
    }

    #[test]
    fn every_page_of_a_category_is_read() {
        let page = |number: u32| {
            UNIQUES
                .replace(r#""Pages": 1"#, r#""Pages": 3"#)
                .replace("Bursting Decay", &format!("Bursting Decay {number}"))
        };
        let api = (2..=3).fold(
            api().with(page_request("Uniques", "accessory", 1), &page(1)),
            |api, number| api.with(page_request("Uniques", "accessory", number), &page(number)),
        );
        // The first answer known for a request is the one given: the pages of three first.
        let api = FakeApi {
            answers: api.answers.into_iter().rev().collect(),
            ..FakeApi::default()
        };
        let mut scout = scout(api);

        let prices = scout
            .prices(&league(), &mut Download::default())
            .expect("every request answered")
            .expect("not interrupted");

        assert_eq!(scout.source.requests.len(), 7);
        assert_eq!(
            scout.source.requests[4..],
            [1, 2, 3].map(|number| page_request("Uniques", "accessory", number))
        );
        let names: Vec<&str> = prices
            .uniques
            .iter()
            .map(|unique| unique.name.as_str())
            .collect();
        assert_eq!(
            names,
            ["Bursting Decay 1", "Bursting Decay 2", "Bursting Decay 3"]
        );
    }

    /// A wrong count of pages must not send requests without end.
    #[test]
    fn a_category_stops_at_its_tenth_page() {
        let endless = CURRENCIES.replace(r#""Pages": 1"#, r#""Pages": 4000"#);
        let api = (1..=MAX_PAGES + 5).fold(FakeApi::default(), |api, page| {
            api.with(page_request("Currencies", "currency", page), &endless)
        });
        let mut scout = scout(api);

        let items = scout
            .category::<ApiCurrency>(
                "HC%20Forbidden%20Rites",
                Kind::Currencies,
                "currency",
                &mut Download::default(),
            )
            .expect("every page answered")
            .expect("not interrupted");

        assert_eq!(scout.source.requests.len(), MAX_PAGES as usize);
        assert_eq!(items.len(), MAX_PAGES as usize);
    }

    /// `docs/poe2scout-usage.md`: after a refusal, wait without insisting. The requests left
    /// of the download aren't sent.
    #[test]
    fn a_download_stops_at_its_first_failed_request() {
        let fragments = page_request("Currencies", "fragments", 1);
        let mut api = api();
        api.answers.retain(|(request, _)| *request != fragments);
        let mut scout = scout(api);
        let mut download = Download::default();

        let error = scout
            .prices(&league(), &mut download)
            .expect_err("a request failed");

        assert_eq!(scout.source.requests.len(), 4, "no unique asked for");
        assert_eq!(scout.source.requests.last(), Some(&fragments));
        assert!(
            matches!(
                &error,
                FetchError::Http {
                    path,
                    error: HttpError::Request(ureq::Error::StatusCode(404)),
                    after_answers: true,
                } if path == "/Leagues/HC%20Forbidden%20Rites/Currencies/ByCategory"
            ),
            "{error:?}"
        );
        // The prices of the currency were told before the failure.
        assert_eq!(download.progress.last(), Some(&(1, 0, 0.6)));
    }

    /// The code review of the quality fixes: a request that hangs after the first answers of
    /// its download is a failure the database answered, and waits longer and longer.
    #[test]
    fn a_request_that_hangs_counts_as_answered_after_the_first_answers_of_its_download() {
        let hangs = |request: &str| {
            let mut scout = scout(api().hanging_at(request));
            let error = scout
                .prices(&league(), &mut Download::default())
                .expect_err("a request hung");
            (error.answered(), scout.source.requests.len())
        };

        assert_eq!(hangs(CATEGORIES_PATH), (false, 1));
        assert_eq!(hangs(RATES_PATH), (true, 2));
        assert_eq!(hangs(&page_request("Uniques", "accessory", 1)), (true, 5));
    }

    #[test]
    fn the_answers_of_an_earlier_download_dont_count_for_the_next_one() {
        let mut after_leagues = scout(api().with("/Leagues", LEAGUES).hanging_at(CATEGORIES_PATH));
        after_leagues.leagues().expect("the leagues");

        let error = after_leagues
            .prices(&league(), &mut Download::default())
            .expect_err("a request hung");

        assert!(!error.answered(), "{error:?}");
        // The same for the leagues, after a download that was answered.
        let mut after_prices = scout(api().hanging_at("/Leagues"));
        after_prices
            .prices(&league(), &mut Download::default())
            .expect("every request answered");
        let error = after_prices.leagues().expect_err("a request hung");
        assert!(!error.answered(), "{error:?}");
    }

    #[test]
    fn the_leagues_are_read_whole_or_not_at_all() {
        let leagues = |answer: &str| {
            let mut scout = scout(FakeApi::default().with("/Leagues", answer));
            let leagues = scout.leagues();
            assert_eq!(scout.source.requests, ["/Leagues"]);
            leagues
        };

        assert_eq!(
            leagues(LEAGUES).expect("the leagues"),
            ["Forbidden Rites", "HC Forbidden Rites", "Runes of Aldur"].map(League::new)
        );
        // One league that can't be read: none is, rather than the others without it.
        let odd = LEAGUES.replacen(r#""IsCurrent": true, "DivinePrice": 718.4"#, "", 1);
        let error = leagues(&odd).expect_err("a league without its fields");
        assert!(
            matches!(
                &error,
                FetchError::Http {
                    error: HttpError::Json(_),
                    ..
                }
            ),
            "{error:?}"
        );
        assert!(error.answered());
        assert!(matches!(
            leagues(&LEAGUES.replace("true", "false")),
            Err(FetchError::NoCurrentLeague)
        ));
    }

    /// The pace waits first, then the interruption is checked, right before each request.
    #[test]
    fn an_interrupted_download_sends_no_other_request() {
        for stop_at in 0..5 {
            let mut scout = scout(api());
            let mut download = Download {
                stop_at: Some(stop_at),
                ..Download::default()
            };

            let prices = scout.prices(&league(), &mut download);

            assert!(matches!(prices, Ok(None)), "interrupted at {stop_at}");
            assert_eq!(scout.source.requests.len(), stop_at);
        }
    }

    #[test]
    fn rates_or_categories_that_cant_be_used_stop_the_download_before_any_price() {
        let many: Vec<String> = (0..=MAX_CATEGORIES)
            .map(|index| format!(r#"{{"ApiId": "{index}"}}"#))
            .collect();
        let many = format!(
            r#"{{"UniqueCategories": [{}], "CurrencyCategories": []}}"#,
            many.join(",")
        );
        let failure = |request: &str, answer: &str| {
            // Before the answer of `api` for the same request, so that it is the one given.
            let mut api = api();
            api.answers
                .insert(0, (request.to_owned(), answer.to_owned()));
            let mut scout = scout(api);
            let error = scout
                .prices(&league(), &mut Download::default())
                .expect_err("an answer that can't be used");
            assert_eq!(scout.source.requests.len(), 2, "{error}");
            error
        };

        assert!(matches!(
            failure(CATEGORIES_PATH, &many),
            FetchError::TooManyCategories(_)
        ));
        assert!(matches!(
            failure(RATES_PATH, r#"[{"ApiId": "exalted", "RelativePrice": 1}]"#),
            FetchError::MissingRate(CHAOS)
        ));
    }

    /// `docs/poe2scout-usage.md`: one request a second at most. Each request of a download
    /// waits for the interval since the one before, whatever the request.
    #[test]
    fn the_requests_of_a_download_are_spaced() {
        const INTERVAL: Duration = Duration::from_millis(15);
        let mut scout = Poe2Scout::with_source(api().with("/Leagues", LEAGUES), INTERVAL);

        scout.leagues().expect("the leagues");
        scout
            .prices(&league(), &mut Download::default())
            .expect("every request answered");

        let times = &scout.source.times;
        assert_eq!(times.len(), 6);
        for pair in times.windows(2) {
            assert!(pair[1].duration_since(pair[0]) >= INTERVAL, "{times:?}");
        }
    }

    #[test]
    fn the_pace_of_the_application_is_slower_than_one_request_a_second() {
        assert!(PACE > Duration::from_secs(1));
    }
}
