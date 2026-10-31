//! The value of the offers of the trade site: the suspicious ones, far below the reference
//! price, and the estimated price of an item without a market price (a rare, magic or
//! normal item, a jewel), the median of its cheapest similar offers.
//!
//! When the quality of a catalyst is searched, which the trade site searches of any kind, the
//! offers of another kind are left out: their quality raises other modifiers.
//!
//! The estimate is itself the reference of its offers: the median of the first page of
//! offers, the answer to the search, tells the suspicious ones, which it leaves out. That
//! median holds as long as fewer than half the offers of the page are bait, and doesn't move
//! when the next offers load.

use crate::format::Currency;
use crate::item::Catalyst;
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
fn in_exalted(amount: f64, id: &str, rates: Rates) -> Option<f64> {
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
    /// The kind of the quality of a catalyst of the offers counted, when it was searched.
    pub searched_catalyst: Option<Catalyst>,
}

/// The estimate from the `first_page` of offers of a search of items, the cheapest first, of
/// the `total` found, with the `rates` of the currencies; the offers with the quality of a
/// catalyst of another kind than `searched_catalyst`, when it was searched, are left out. `None`
/// without an offer priced in one of the three currencies.
#[must_use]
pub fn estimate(
    first_page: &[ItemOffer],
    total: u64,
    rates: Rates,
    searched_catalyst: Option<Catalyst>,
) -> Option<Estimate> {
    let mut prices: Vec<f64> = first_page
        .iter()
        .filter(|offer| searched_catalyst.is_none_or(|catalyst| !offer.has_other_quality(catalyst)))
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
        searched_catalyst,
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
    use crate::item::QualityKind;
    use crate::trade::offers::{OfferItem, OfferQuality, Price, SellerStatus};

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
                quality: None,
                waystone_properties: Vec::new(),
                waystone_unread: false,
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

        let bow = estimate(&offers, 18, RATES, None).unwrap();

        assert!((bow.exalted - 2.4 * 700.0).abs() < 1e-9);
        assert_eq!(bow.counted, 5);
        assert_eq!(bow.total, 18);
        // The median of the ten: (2.5 + 3) / 2 div.
        assert!((bow.reference.exalted - 2.75 * 700.0).abs() < 1e-9);
        assert!(bow.reference.suspicious(300.0, "exalted"));
        assert!(!bow.reference.suspicious(2.0, "divine"));
    }

    #[test]
    fn the_offers_are_compared_in_exalted_orbs() {
        // The magic item of the mock-up: 1 c is worth 70 ex, and comes after 50 ex.
        let offers = [
            offer(50.0, "exalted"),
            offer(70.0, "exalted"),
            offer(1.0, "chaos"),
        ];

        let magic = estimate(&offers, 3, RATES, None).unwrap();

        assert!((magic.exalted - 70.0).abs() < 1e-9);
        assert_eq!(magic.counted, 3);
    }

    #[test]
    fn the_offers_are_sorted_again_with_the_rates_of_the_prices() {
        // The trade site put 1 div before 650 ex: its rate of the Divine Orb is lower.
        let offers = [
            offer(1.0, "divine"),
            offer(650.0, "exalted"),
            offer(1.1, "divine"),
        ];

        let sorted = estimate(&offers, 3, RATES, None).unwrap();

        assert!((sorted.exalted - 700.0).abs() < 1e-9);
    }

    #[test]
    fn an_even_count_of_offers_gives_the_mean_of_the_two_middle_ones() {
        let even = estimate(&divines(&[1.0, 2.0, 3.0, 5.0]), 4, RATES, None).unwrap();

        assert!((even.exalted - 2.5 * 700.0).abs() < 1e-9);
        assert_eq!(even.counted, 4);
    }

    #[test]
    fn the_suspicious_offers_are_left_out_of_the_count() {
        // Six offers, two of them bait: four are counted, of the six found.
        let mut offers = vec![offer(10.0, "exalted"), offer(20.0, "exalted")];
        offers.extend(divines(&[1.0, 1.2, 1.4, 1.6]));

        let baited = estimate(&offers, 6, RATES, None).unwrap();

        assert_eq!(baited.counted, 4);
        assert_eq!(baited.total, 6);
        assert!((baited.exalted - 1.3 * 700.0).abs() < 1e-9);
    }

    #[test]
    fn an_offer_without_a_known_price_isnt_counted() {
        let mut offers = vec![offer(1.0, "annul"), offer(2.0, "divine")];
        offers[0].price = None;
        offers.push(offer(3.0, "annul"));

        assert_eq!(estimate(&[offer(1.0, "annul")], 1, RATES, None), None);
        assert_eq!(estimate(&[], 0, RATES, None), None);

        let known = estimate(&offers, 3, RATES, None).unwrap();

        assert_eq!(known.counted, 1);
        assert!((known.exalted - 2.0 * 700.0).abs() < 1e-9);
    }

    #[test]
    fn the_offers_of_another_kind_of_quality_are_left_out_when_it_is_searched() {
        // The amulet of mana quality of the mock-up: three offers of another kind among the
        // eight cheapest.
        let mut offers = divines(&[1.5, 2.0, 2.4, 2.5, 3.0, 3.2, 3.5, 4.0]);
        let with = |offer: &mut ItemOffer, catalyst| {
            offer.item.quality = Some(OfferQuality {
                value: 40,
                kind: Some(QualityKind {
                    catalyst: Some(catalyst),
                    words: String::new(),
                }),
            });
        };
        for (index, catalyst) in [
            (0, Catalyst::Caster),
            (1, Catalyst::Attack),
            (4, Catalyst::Minion),
        ] {
            with(&mut offers[index], catalyst);
        }
        with(&mut offers[2], Catalyst::Mana);

        let mana = estimate(&offers, 27, RATES, Some(Catalyst::Mana)).unwrap();
        let any = estimate(&offers, 27, RATES, None).unwrap();

        assert!((mana.exalted - 3.2 * 700.0).abs() < 1e-9);
        assert_eq!(mana.counted, 5);
        assert_eq!(mana.searched_catalyst, Some(Catalyst::Mana));
        assert_eq!(mana.total, 27, "every offer found");
        assert!((any.exalted - 2.4 * 700.0).abs() < 1e-9);
        assert_eq!(any.searched_catalyst, None);
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
