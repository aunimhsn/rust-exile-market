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

/// How a price changed lately.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Trend {
    /// The change, e.g. `0.04` for +4 %.
    pub change: f64,
    /// The days it covers, today included.
    pub days: u32,
}

/// Where a price goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Rise,
    Fall,
    /// Under 1 % of change.
    Stable,
}

/// The days of the history the trend looks at, today included.
const TREND_DAYS: usize = 7;

impl Trend {
    #[must_use]
    pub fn direction(self) -> Direction {
        match self.change {
            change if change.abs() < 0.01 => Direction::Stable,
            change if change > 0.0 => Direction::Rise,
            _ => Direction::Fall,
        }
    }
}

impl Price {
    /// The change from the oldest known price of the history; none without a past price.
    #[must_use]
    pub fn trend(&self) -> Option<Trend> {
        let (days_ago, oldest) = self
            .history
            .iter()
            .enumerate()
            .take(TREND_DAYS)
            // The first price is today's.
            .skip(1)
            .rev()
            .find_map(|(index, price)| price.filter(|price| *price > 0.0).map(|p| (index, p)))?;
        Some(Trend {
            change: self.exalted / oldest - 1.0,
            days: u32::try_from(days_ago + 1).unwrap_or(u32::MAX),
        })
    }
}

/// A unique item, known by its name and its base type: an unidentified unique shows only its
/// base type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniquePrice {
    pub name: String,
    pub base_type: String,
    pub price: Price,
    /// Its implicit and explicit modifiers with their ranges, in English, e.g.
    /// `+(17-23)% to Chaos Resistance`: the rolls of an item copied without them.
    pub modifiers: Vec<String>,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn price(exalted: f64, history: &[Option<f64>]) -> Price {
        Price {
            exalted,
            quantity: 10,
            history: history.to_vec(),
        }
    }

    #[test]
    fn the_trend_compares_with_the_oldest_known_price() {
        let week = price(
            104.0,
            &[
                Some(103.0),
                Some(101.0),
                None,
                Some(99.0),
                None,
                None,
                Some(100.0),
            ],
        );
        let recent = price(
            90.0,
            &[Some(91.0), Some(95.0), Some(100.0), None, None, None, None],
        );

        let trend = week.trend().expect("a trend");
        assert!((trend.change - 0.04).abs() < 1e-9);
        assert_eq!(trend.days, 7);
        let trend = recent.trend().expect("a trend");
        assert!((trend.change + 0.1).abs() < 1e-9);
        assert_eq!(trend.days, 3);
    }

    #[test]
    fn without_a_past_price_there_is_no_trend() {
        assert_eq!(price(10.0, &[Some(10.0), None]).trend(), None);
        assert_eq!(price(10.0, &[]).trend(), None);
    }

    #[test]
    fn under_one_percent_the_price_is_stable() {
        let direction = |change| Trend { change, days: 7 }.direction();

        assert_eq!(direction(0.009), Direction::Stable);
        assert_eq!(direction(-0.009), Direction::Stable);
        assert_eq!(direction(0.01), Direction::Rise);
        assert_eq!(direction(-0.2), Direction::Fall);
    }

    #[test]
    fn the_trend_looks_at_seven_days_only() {
        let mut history = vec![None; 9];
        history[8] = Some(50.0);
        history[3] = Some(100.0);

        let trend = price(110.0, &history).trend().expect("a trend");

        assert_eq!(trend.days, 4);
    }
}
