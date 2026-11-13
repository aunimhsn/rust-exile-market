//! The value of the offers of the trade site: the suspicious ones, far below the reference
//! price, and the estimated price of an item without a market price (a rare, magic or
//! normal item, a jewel), the median of its cheapest similar offers.
//!
//! When the quality of a catalyst is searched, which the trade site searches of any kind, the
//! offers of another kind are left out: their quality raises other modifiers.
//!
//! An offer is compared in Exalted Orbs: the Chaos and Divine Orbs at the rates of the price
//! database, another item of the currency exchange at its price there (a Vaal Orb, a
//! Mirror); an offer in a currency without price (a Wombgift), or whose price stands on too
//! few trades ([`price::MIN_TRADES`]), is left out.
//!
//! The estimate is itself the reference of its offers: the median of the first page of
//! offers, the answer to the search, tells the suspicious ones, which it leaves out. That
//! median holds as long as fewer than half the offers of the page are bait, and doesn't move
//! when the next offers load.

use crate::item::Catalyst;
use crate::price::{self, CurrencyPrice, Orb, PriceList, Rates};
use crate::trade::offers::ItemOffer;

/// An offer under this share of the reference price is suspicious.
const SUSPICIOUS_SHARE: f64 = 0.25;
/// The cheapest offers counted in an estimate; fewer make it uncertain.
pub const COUNTED: usize = 5;

/// What the currencies an offer may be priced in are worth, from the prices of a league.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Worth<'a> {
    /// The rates of the Chaos and Divine Orbs.
    pub rates: Rates,
    /// The items of the currency exchange priced by the price database, the Chaos and Divine
    /// Orbs among them, worth their rates instead.
    pub currencies: &'a [CurrencyPrice],
}

impl<'a> Worth<'a> {
    /// What the currencies are worth with the `prices` of a league.
    #[must_use]
    pub fn of(prices: &'a PriceList) -> Self {
        Self {
            rates: prices.rates,
            currencies: &prices.currencies,
        }
    }

    /// `amount` of the currency `id` of the trade site, in Exalted Orbs; `None` for a
    /// currency without price, or whose price stands on too few trades.
    fn in_exalted(self, amount: f64, id: &str) -> Option<f64> {
        let worth = match Orb::of(id) {
            Some(Orb::Exalted) => 1.0,
            Some(Orb::Divine) => self.rates.divine,
            Some(Orb::Chaos) => self.rates.chaos,
            None => price::currency(self.currencies, id)?.trusted_exalted()?,
        };
        // A price of nothing, or none, would count an offer as free; so would an amount of
        // nothing, and an amount beyond every price is no price either.
        let exalted = amount * worth;
        (worth > 0.0 && amount > 0.0 && exalted.is_finite()).then_some(exalted)
    }
}

/// The price of one item that tells a suspicious offer, in Exalted Orbs, with the worth of
/// the currencies: the price of the price database, or the median of the similar offers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reference<'a> {
    pub exalted: f64,
    pub worth: Worth<'a>,
}

impl Reference<'_> {
    /// Whether one item for `amount` of the currency `id` is far below the reference; never
    /// for a currency without price.
    #[must_use]
    pub fn suspicious(self, amount: f64, id: &str) -> bool {
        self.worth
            .in_exalted(amount, id)
            .is_some_and(|exalted| self.below(exalted))
    }

    /// Whether `exalted` Exalted Orbs are far below the reference.
    fn below(self, exalted: f64) -> bool {
        exalted < self.exalted * SUSPICIOUS_SHARE
    }
}

/// The estimated price of an item, from its similar offers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Estimate<'a> {
    /// The median of the cheapest offers counted, in Exalted Orbs.
    pub exalted: f64,
    /// The offers counted: [`COUNTED`], or fewer when there are fewer.
    pub counted: usize,
    /// The similar offers found by the trade site.
    pub total: u64,
    /// The median of the first page of offers, which tells the suspicious ones.
    pub reference: Reference<'a>,
    /// The kind of the quality of a catalyst of the offers counted, when it was searched.
    pub searched_catalyst: Option<Catalyst>,
}

impl<'a> Estimate<'a> {
    /// The estimate from the `first_page` of offers of a search of items, the cheapest first, of
    /// the `total` found, with the `worth` of the currencies; the offers with the quality of a
    /// catalyst of another kind than `searched_catalyst`, when it was searched, are left out. `None`
    /// without an offer priced in a currency with a price on enough trades
    /// ([`price::MIN_TRADES`]).
    #[must_use]
    pub fn of(
        first_page: &[ItemOffer],
        total: u64,
        worth: Worth<'a>,
        searched_catalyst: Option<Catalyst>,
    ) -> Option<Self> {
        let mut prices: Vec<f64> = first_page
            .iter()
            .filter(|offer| {
                searched_catalyst.is_none_or(|catalyst| !offer.has_other_quality(catalyst))
            })
            .filter_map(|offer| offer.price.as_ref())
            .filter_map(|price| worth.in_exalted(price.amount, &price.currency))
            .collect();
        // The trade site sorts them with its own rates.
        prices.sort_by(f64::total_cmp);
        let reference = Reference {
            exalted: median(&prices)?,
            worth,
        };
        let counted: Vec<f64> = prices
            .into_iter()
            .filter(|&exalted| !reference.below(exalted))
            .take(COUNTED)
            .collect();
        Some(Self {
            exalted: median(&counted)?,
            counted: counted.len(),
            total,
            reference,
            searched_catalyst,
        })
    }
}

/// The median of `sorted` values: the middle one, or the mean of the two middle ones.
fn median(sorted: &[f64]) -> Option<f64> {
    let middle = sorted.len() / 2;
    match sorted.len() {
        0 => None,
        len if len % 2 == 1 => Some(sorted[middle]),
        _ => Some(f64::midpoint(sorted[middle - 1], sorted[middle])),
    }
}

#[cfg(test)]
mod tests;
