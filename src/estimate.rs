//! The value of the offers of the trade site: the suspicious ones, far below the reference
//! price, and the estimated price of an item without a market price (a rare, magic or
//! normal item, a jewel), the median of its cheapest similar offers.
//!
//! The estimate is itself the reference of its offers: the median of the first page of
//! offers tells the suspicious ones, which it leaves out. That median holds as long as fewer
//! than half the offers of the page are bait, and doesn't move when the next offers load.

use crate::format::Currency;
use crate::offers::PAGE;
use crate::prices::Rates;
use crate::trade::offers::{ItemOffer, currency};

/// An offer under this share of the reference price is suspicious.
const SUSPICIOUS_SHARE: f64 = 0.25;
/// The cheapest offers counted in an estimate; fewer make it uncertain.
pub const COUNTED: usize = 5;

/// The price of one item that tells a suspicious offer, in Exalted Orbs, with the rates of
/// its currencies: the price of the price database, or the median of the similar offers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reference {
    pub exalted: f64,
    pub rates: Rates,
}

impl Reference {
    /// Whether one item for `amount` of the currency `id` is far below the reference.
    #[must_use]
    pub fn suspicious(self, amount: f64, id: &str) -> bool {
        in_exalted(amount, id, self.rates).is_some_and(|exalted| self.below(exalted))
    }

    /// Whether `exalted` Exalted Orbs are far below the reference.
    fn below(self, exalted: f64) -> bool {
        exalted < self.exalted * SUSPICIOUS_SHARE
    }
}

/// `amount` of the currency `id`, in Exalted Orbs; `None` for a currency without a rate.
#[must_use]
pub fn in_exalted(amount: f64, id: &str, rates: Rates) -> Option<f64> {
    Some(match currency(id)? {
        Currency::Exalted => amount,
        Currency::Divine => amount * rates.divine,
        Currency::Chaos => amount * rates.chaos,
    })
}

/// The estimated price of an item, from its similar offers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Estimate {
    /// The median of the cheapest offers counted, in Exalted Orbs.
    pub exalted: f64,
    /// The offers counted: [`COUNTED`], or fewer when there are fewer.
    pub counted: usize,
    /// The similar offers found by the trade site.
    pub total: u64,
    /// The median of the first page of offers, which tells the suspicious ones.
    pub reference: Reference,
}

/// The estimate from the `offers` of a search of items, the cheapest first, of the `total`
/// found, with the `rates` of the currencies; `None` without an offer priced in one of the
/// three currencies.
#[must_use]
pub fn estimate(offers: &[ItemOffer], total: u64, rates: Rates) -> Option<Estimate> {
    let mut prices: Vec<f64> = offers
        .iter()
        .take(PAGE)
        .filter_map(|offer| offer.price.as_ref())
        .filter_map(|price| in_exalted(price.amount, &price.currency, rates))
        .collect();
    // The trade site sorts them with its own rates.
    prices.sort_by(f64::total_cmp);
    let reference = Reference {
        exalted: median(&prices)?,
        rates,
    };
    let counted: Vec<f64> = prices
        .into_iter()
        .filter(|&exalted| !reference.below(exalted))
        .take(COUNTED)
        .collect();
    Some(Estimate {
        exalted: median(&counted)?,
        counted: counted.len(),
        total,
        reference,
    })
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
mod tests {
    use std::time::SystemTime;

    use super::*;
    use crate::trade::offers::{OfferItem, Price, SellerStatus};

    const RATES: Rates = Rates {
        chaos: 70.0,
        divine: 700.0,
    };

    fn offer(amount: f64, currency: &str) -> ItemOffer {
        ItemOffer {
            id: "a".to_owned(),
            price: Some(Price {
                amount,
                currency: currency.to_owned(),
            }),
            seller: SellerStatus::Online,
            listed: Some(SystemTime::UNIX_EPOCH),
            whisper: String::new(),
            item: OfferItem {
                rarity: None,
                name: Some("Storm Bite".to_owned()),
                base_type: "Gemini Bow".to_owned(),
                item_level: None,
                corrupted: false,
                modifiers: Vec::new(),
            },
        }
    }

    fn divines(amounts: &[f64]) -> Vec<ItemOffer> {
        amounts
            .iter()
            .map(|&amount| offer(amount, "divine"))
            .collect()
    }

    #[test]
    fn the_estimate_is_the_median_of_the_five_cheapest_offers_but_the_suspicious_ones() {
        // The rare bow of the mock-up: 300 ex is under a quarter of the median of the page.
        let mut offers = vec![offer(300.0, "exalted")];
        offers.extend(divines(&[2.0, 2.2, 2.4, 2.5, 3.0, 3.1, 3.5, 4.0, 4.2]));

        let estimate = estimate(&offers, 18, RATES).unwrap();

        assert!((estimate.exalted - 2.4 * 700.0).abs() < 1e-9);
        assert_eq!(estimate.counted, 5);
        assert_eq!(estimate.total, 18);
        // The median of the ten: (2.5 + 3) / 2 div.
        assert!((estimate.reference.exalted - 2.75 * 700.0).abs() < 1e-9);
        assert!(estimate.reference.suspicious(300.0, "exalted"));
        assert!(!estimate.reference.suspicious(2.0, "divine"));
    }

    #[test]
    fn the_offers_are_compared_in_exalted_orbs() {
        // The magic item of the mock-up: 1 c is worth 70 ex, and comes after 50 ex.
        let offers = [
            offer(50.0, "exalted"),
            offer(70.0, "exalted"),
            offer(1.0, "chaos"),
        ];

        let estimate = estimate(&offers, 3, RATES).unwrap();

        assert!((estimate.exalted - 70.0).abs() < 1e-9);
        assert_eq!(estimate.counted, 3);
    }

    #[test]
    fn the_offers_are_sorted_again_with_the_rates_of_the_prices() {
        // The trade site put 1 div before 650 ex: its rate of the Divine Orb is lower.
        let offers = [
            offer(1.0, "divine"),
            offer(650.0, "exalted"),
            offer(1.1, "divine"),
        ];

        let estimate = estimate(&offers, 3, RATES).unwrap();

        assert!((estimate.exalted - 700.0).abs() < 1e-9);
    }

    #[test]
    fn an_even_count_of_offers_gives_the_mean_of_the_two_middle_ones() {
        let estimate = estimate(&divines(&[1.0, 2.0, 3.0, 5.0]), 4, RATES).unwrap();

        assert!((estimate.exalted - 2.5 * 700.0).abs() < 1e-9);
        assert_eq!(estimate.counted, 4);
    }

    #[test]
    fn the_suspicious_offers_are_left_out_of_the_count() {
        // Six offers, two of them bait: four are counted, of the six found.
        let mut offers = vec![offer(10.0, "exalted"), offer(20.0, "exalted")];
        offers.extend(divines(&[1.0, 1.2, 1.4, 1.6]));

        let estimate = estimate(&offers, 6, RATES).unwrap();

        assert_eq!(estimate.counted, 4);
        assert_eq!(estimate.total, 6);
        assert!((estimate.exalted - 1.3 * 700.0).abs() < 1e-9);
    }

    #[test]
    fn only_the_first_page_tells_the_suspicious_offers() {
        // The next ten offers, far dearer, would raise the median above 4 × 0.6 div.
        let mut offers = divines(&[0.6; PAGE]);
        offers.extend(divines(&[10.0; PAGE]));

        let estimate = estimate(&offers, 40, RATES).unwrap();

        assert!((estimate.reference.exalted - 0.6 * 700.0).abs() < 1e-9);
        assert!((estimate.exalted - 0.6 * 700.0).abs() < 1e-9);
    }

    #[test]
    fn an_offer_without_a_known_price_isnt_counted() {
        let mut offers = vec![offer(1.0, "annul"), offer(2.0, "divine")];
        offers[0].price = None;
        offers.push(offer(3.0, "annul"));

        assert_eq!(estimate(&[offer(1.0, "annul")], 1, RATES), None);
        assert_eq!(estimate(&[], 0, RATES), None);

        let estimate = estimate(&offers, 3, RATES).unwrap();

        assert_eq!(estimate.counted, 1);
        assert!((estimate.exalted - 2.0 * 700.0).abs() < 1e-9);
    }

    #[test]
    fn another_currency_is_never_suspicious() {
        let reference = Reference {
            exalted: 9300.0,
            rates: RATES,
        };

        assert!(reference.suspicious(6.0, "exalted"));
        assert!(reference.suspicious(3.0, "divine"));
        assert!(!reference.suspicious(1.0, "annul"));
    }
}
