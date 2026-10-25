//! The prices of the fixed-name items of a league, from the price database: the uniques and
//! the items of the currency exchange, in Exalted Orbs, with the rates of the Chaos and Divine
//! Orbs to show them in the three currencies.

use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

use crate::league::League;

/// How many Exalted Orbs a Chaos Orb and a Divine Orb are worth.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rates {
    pub chaos: f64,
    pub divine: f64,
}

/// The price of an item, in Exalted Orbs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Price {
    pub exalted: f64,
    /// How many were listed behind the price: the number of offers of a unique.
    pub quantity: u32,
    /// The daily prices of the last days, the most recent first; `None` for a day without
    /// price.
    pub history: Vec<Option<f64>>,
}

/// A unique item, known by its name and its base type: an unidentified unique shows only its
/// base type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniquePrice {
    pub name: String,
    pub base_type: String,
    pub price: Price,
}

/// An item of the currency exchange: currency, fragments, essences, runes…
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CurrencyPrice {
    pub name: String,
    /// Its id on the currency exchange of the trade site, e.g. `divine`.
    pub exchange_id: String,
    pub price: Price,
}

/// The prices of a league, as the price database gave them at one time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PriceList {
    pub league: League,
    pub updated: SystemTime,
    pub rates: Rates,
    pub uniques: Vec<UniquePrice>,
    pub currencies: Vec<CurrencyPrice>,
}

impl PriceList {
    /// How long ago the price database gave them; none for prices dated in the future, e.g.
    /// after the clock of the system went back.
    #[must_use]
    pub fn age(&self, now: SystemTime) -> Duration {
        now.duration_since(self.updated).unwrap_or_default()
    }

    /// The number of priced items, for the log.
    #[must_use]
    pub fn item_count(&self) -> usize {
        self.uniques.len() + self.currencies.len()
    }

    /// The item of the currency exchange named `name` in English.
    #[must_use]
    pub fn currency_named(&self, name: &str) -> Option<&CurrencyPrice> {
        self.currencies
            .iter()
            .find(|currency| currency.name == name)
    }

    /// The item of the currency exchange whose id is `exchange_id`.
    #[must_use]
    pub fn currency(&self, exchange_id: &str) -> Option<&CurrencyPrice> {
        self.currencies
            .iter()
            .find(|currency| currency.exchange_id == exchange_id)
    }

    /// The unique named `name` in English: the price database has one per name.
    #[must_use]
    pub fn unique(&self, name: &str) -> Option<&UniquePrice> {
        self.uniques.iter().find(|unique| unique.name == name)
    }
}
