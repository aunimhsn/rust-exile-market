//! The public API of poe2scout, the price database of the Path of Exile 2 economy.
//!
//! The rules of its use (pace, identification) are in `roadmap.md` (poe2scout usage).

use std::fmt::{self, Write};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use serde::Deserialize;
use serde::de::DeserializeOwned;

use super::{Download, PriceDatabase};
use crate::league::League;
use crate::prices::{CurrencyPrice, Price, PriceList, Rates, UniquePrice};

const BASE_URL: &str = "https://api.poe2scout.com/poe2";
/// A request that hangs is given up, and tried again later.
const TIMEOUT: Duration = Duration::from_secs(30);
/// A server that doesn't answer is given up sooner.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// The time between two requests, at least.
const PACE: Duration = Duration::from_secs(1);
/// The most items per page the API gives: most categories fit in one page.
const PER_PAGE: &str = "250";
/// The most pages read in a category, in case the API gave a wrong count.
const MAX_PAGES: u32 = 10;
/// The ids of the three currencies of the prices in `/ReferenceCurrencies`.
const EXALTED: &str = "exalted";
const CHAOS: &str = "chaos";
const DIVINE: &str = "divine";

/// Why the price database couldn't be read.
#[derive(Debug)]
pub enum FetchError {
    /// No answer, or an error status (e.g. 429 when its limits are reached).
    Http(ureq::Error),
    /// An answer that isn't the expected JSON: the API changed.
    Json(serde_json::Error),
    /// No current league in the answer: the database is wrong for a moment, and the league of
    /// the player must not be changed for that.
    NoCurrentLeague,
    /// No rate for this currency in the answer, so no price can be shown in it.
    MissingRate(&'static str),
}

impl fmt::Display for FetchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Http(error) => write!(formatter, "request failed: {error}"),
            Self::Json(error) => write!(formatter, "unexpected answer: {error}"),
            Self::NoCurrentLeague => formatter.write_str("no current league in the answer"),
            Self::MissingRate(currency) => write!(formatter, "no rate for the {currency} orb"),
        }
    }
}

impl std::error::Error for FetchError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Http(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::NoCurrentLeague | Self::MissingRate(_) => None,
        }
    }
}

impl From<ureq::Error> for FetchError {
    fn from(error: ureq::Error) -> Self {
        Self::Http(error)
    }
}

impl From<serde_json::Error> for FetchError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
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
    unique_categories: Vec<ApiCategory>,
    currency_categories: Vec<ApiCategory>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ApiCategory {
    api_id: String,
}

/// A currency of the answer of `/ReferenceCurrencies`, worth `relative_price` of the currency
/// the prices are given in.
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ApiReferenceCurrency {
    api_id: String,
    relative_price: f64,
}

/// A page of the items of a category.
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ApiPage<T> {
    pages: u32,
    items: Vec<T>,
}

/// A unique in the answer of `/Uniques/ByCategory`.
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ApiUnique {
    name: String,
    #[serde(rename = "Type")]
    base_type: String,
    #[serde(flatten)]
    price: ApiPrice,
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
    /// The daily prices, the most recent first; `null` for a day without price.
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

/// The client of poe2scout, which identifies the application in each request and spaces its
/// requests.
pub struct Poe2Scout {
    agent: ureq::Agent,
    last_request: Option<Instant>,
}

impl Poe2Scout {
    #[must_use]
    pub fn new() -> Self {
        let config = ureq::Agent::config_builder()
            .timeout_global(Some(TIMEOUT))
            .timeout_connect(Some(CONNECT_TIMEOUT))
            .user_agent(format!(
                "{}/{} (+{})",
                crate::FOLDER_NAME,
                crate::VERSION,
                env!("CARGO_PKG_REPOSITORY")
            ))
            .build();
        Self {
            agent: ureq::Agent::new_with_config(config),
            last_request: None,
        }
    }

    /// The answer of `path` with the parameters `query`, once [`PACE`] has passed since the
    /// last request.
    fn get<T: DeserializeOwned>(
        &mut self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<T, FetchError> {
        if let Some(last) = self.last_request {
            thread::sleep(PACE.saturating_sub(last.elapsed()));
        }
        // Counted even when it fails: a refused request counts in the limits too.
        self.last_request = Some(Instant::now());
        let mut response = self
            .agent
            .get(format!("{BASE_URL}{path}"))
            .query_pairs(query.iter().copied())
            .call()?;
        let text = response.body_mut().read_to_string()?;
        Ok(serde_json::from_str(&text)?)
    }

    /// The items of every page of a category; `None` when `download` is interrupted before a
    /// request.
    fn category<T: DeserializeOwned>(
        &mut self,
        league: &str,
        kind: Kind,
        category: &str,
        download: &mut dyn Download,
    ) -> Result<Option<Vec<T>>, FetchError> {
        let path = format!("/Leagues/{league}/{}/ByCategory", kind.path());
        let mut items = Vec::new();
        for page in 1..=MAX_PAGES {
            if download.interrupted() {
                return Ok(None);
            }
            let page_text = page.to_string();
            let answer: ApiPage<T> = self.get(
                &path,
                &[
                    ("category", category),
                    ("perPage", PER_PAGE),
                    ("page", &page_text),
                ],
            )?;
            items.extend(answer.items);
            if page >= answer.pages {
                return Ok(Some(items));
            }
        }
        log::warn!("More than {MAX_PAGES} pages of {category}: only the first ones are read");
        Ok(Some(items))
    }
}

impl PriceDatabase for Poe2Scout {
    fn leagues(&mut self) -> Result<Vec<League>, FetchError> {
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
        download: &mut dyn Download,
    ) -> Result<Option<PriceList>, FetchError> {
        let path = path_segment(league.id());
        if download.interrupted() {
            return Ok(None);
        }
        let categories: ApiCategories =
            self.get(&format!("/Leagues/{path}/Items/Categories"), &[])?;
        if download.interrupted() {
            return Ok(None);
        }
        let currencies: Vec<ApiReferenceCurrency> =
            self.get(&format!("/Leagues/{path}/ReferenceCurrencies"), &[])?;
        let conversion = Conversion::new(&currencies)?;
        let mut prices = PriceList {
            league: league.clone(),
            updated: SystemTime::now(),
            rates: conversion.rates,
            uniques: Vec::new(),
            currencies: Vec::new(),
        };
        // The two requests above, then one per category (most fit in one page).
        let requests =
            2 + categories.currency_categories.len() + categories.unique_categories.len();
        let mut done = 2;
        #[expect(clippy::cast_precision_loss, reason = "a few dozen requests")]
        let share = |done: usize| done as f32 / requests as f32;
        download.progress(&prices, share(done));
        for category in &categories.currency_categories {
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
            prices.uniques.extend(items.into_iter().filter_map(|item| {
                Some(UniquePrice {
                    price: conversion.price(item.price)?,
                    name: item.name,
                    base_type: item.base_type,
                })
            }));
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

/// `text` as one segment of a path, e.g. a league name with spaces.
fn path_segment(text: &str) -> String {
    let mut segment = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
            segment.push(char::from(byte));
        } else {
            // Writing to a `String` never fails.
            let _ = write!(segment, "%{byte:02X}");
        }
    }
    segment
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
    use super::*;

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
    fn a_league_name_is_escaped_in_the_path() {
        assert_eq!(path_segment("HC Forbidden Rites"), "HC%20Forbidden%20Rites");
        assert_eq!(
            path_segment("Rise of the Abyssal (PL1)"),
            "Rise%20of%20the%20Abyssal%20%28PL1%29"
        );
    }

    #[test]
    fn an_answer_without_the_expected_fields_is_an_error() {
        let parsed: Result<Vec<ApiLeague>, _> = serde_json::from_str(r#"[{"Name": "x"}]"#);

        assert!(parsed.is_err());
    }
}
