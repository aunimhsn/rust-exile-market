//! The value of the offers of the trade site: the suspicious ones, far below the reference
//! price, and the estimated price of an item without a market price (a rare, magic or
//! normal item, a jewel), the median of its cheapest similar offers.
//!
//! When the quality of a catalyst is searched, which the trade site searches of any kind, the
//! offers of another kind are left out: their quality raises other modifiers.
//!
//! An offer is compared in Exalted Orbs: the Chaos and Divine Orbs at the rates of the price
//! database, another item of the currency exchange at its price there (a Vaal Orb, a
//! Mirror); an offer in a currency without price (a Soul Core, a Thaumaturgic Flux), or whose
//! price stands on too few trades, is left out.
//!
//! The estimate is itself the reference of its offers: the median of the first page of
//! offers, the answer to the search, tells the suspicious ones, which it leaves out. That
//! median holds as long as fewer than half the offers of the page are bait, and doesn't move
//! when the next offers load.

use crate::format::Currency;
use crate::item::Catalyst;
use crate::prices::{self, CurrencyPrice, PriceList, Rates};
use crate::trade::offers::{ItemOffer, currency};

/// An offer under this share of the reference price is suspicious.
const SUSPICIOUS_SHARE: f64 = 0.25;
/// The cheapest offers counted in an estimate; fewer make it uncertain.
pub const COUNTED: usize = 5;
/// The quantity behind the price of a currency, under which its price is too thin to count
/// an offer in it: 118 of the 638 currencies priced by poe2scout stand on one trade (league
/// Forbidden Rites, 2026-11-01), some of them absurd (a Transmutation Shard at 11 ex, a
/// Lesser Charging Rune at 40,156 ex), while the Mirror, the least traded of the currencies
/// of the price filter of the trade site, has 14.
const MIN_QUANTITY: u32 = 10;

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
    /// currency without price, or whose price stands on fewer than [`MIN_QUANTITY`].
    fn in_exalted(self, amount: f64, id: &str) -> Option<f64> {
        let worth = match currency(id) {
            Some(Currency::Exalted) => 1.0,
            Some(Currency::Divine) => self.rates.divine,
            Some(Currency::Chaos) => self.rates.chaos,
            None => {
                let price = &prices::currency(self.currencies, id)?.price;
                (price.quantity >= MIN_QUANTITY).then_some(price.exalted)?
            }
        };
        // A price of nothing, or none, would count an offer as free.
        (worth.is_finite() && worth > 0.0).then_some(amount * worth)
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

/// The estimate from the `first_page` of offers of a search of items, the cheapest first, of
/// the `total` found, with the `worth` of the currencies; the offers with the quality of a
/// catalyst of another kind than `searched_catalyst`, when it was searched, are left out. `None`
/// without an offer priced in a currency with a price.
#[must_use]
pub fn estimate<'a>(
    first_page: &[ItemOffer],
    total: u64,
    worth: Worth<'a>,
    searched_catalyst: Option<Catalyst>,
) -> Option<Estimate<'a>> {
    let mut prices: Vec<f64> = first_page
        .iter()
        .filter(|offer| searched_catalyst.is_none_or(|catalyst| !offer.has_other_quality(catalyst)))
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
    const WORTH: Worth = Worth {
        rates: RATES,
        currencies: &[],
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
                gem_level: None,
                gem_sockets: None,
                requirements: crate::item::Requirements::default(),
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

        let bow = estimate(&offers, 18, WORTH, None).unwrap();

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

        let magic = estimate(&offers, 3, WORTH, None).unwrap();

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

        let sorted = estimate(&offers, 3, WORTH, None).unwrap();

        assert!((sorted.exalted - 700.0).abs() < 1e-9);
    }

    #[test]
    fn an_even_count_of_offers_gives_the_mean_of_the_two_middle_ones() {
        let even = estimate(&divines(&[1.0, 2.0, 3.0, 5.0]), 4, WORTH, None).unwrap();

        assert!((even.exalted - 2.5 * 700.0).abs() < 1e-9);
        assert_eq!(even.counted, 4);
    }

    #[test]
    fn the_suspicious_offers_are_left_out_of_the_count() {
        // Six offers, two of them bait: four are counted, of the six found.
        let mut offers = vec![offer(10.0, "exalted"), offer(20.0, "exalted")];
        offers.extend(divines(&[1.0, 1.2, 1.4, 1.6]));

        let baited = estimate(&offers, 6, WORTH, None).unwrap();

        assert_eq!(baited.counted, 4);
        assert_eq!(baited.total, 6);
        assert!((baited.exalted - 1.3 * 700.0).abs() < 1e-9);
    }

    #[test]
    fn an_offer_without_a_known_price_isnt_counted() {
        let mut offers = vec![offer(1.0, "annul"), offer(2.0, "divine")];
        offers[0].price = None;
        offers.push(offer(3.0, "annul"));

        assert_eq!(estimate(&[offer(1.0, "annul")], 1, WORTH, None), None);
        assert_eq!(estimate(&[], 0, WORTH, None), None);

        let known = estimate(&offers, 3, WORTH, None).unwrap();

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

        let mana = estimate(&offers, 27, WORTH, Some(Catalyst::Mana)).unwrap();
        let any = estimate(&offers, 27, WORTH, None).unwrap();

        assert!((mana.exalted - 3.2 * 700.0).abs() < 1e-9);
        assert_eq!(mana.counted, 5);
        assert_eq!(mana.searched_catalyst, Some(Catalyst::Mana));
        assert_eq!(mana.total, 27, "every offer found");
        assert!((any.exalted - 2.4 * 700.0).abs() < 1e-9);
        assert_eq!(any.searched_catalyst, None);
    }

    /// An item of the currency exchange worth `exalted` Exalted Orbs.
    fn priced(exchange_id: &str, exalted: f64) -> CurrencyPrice {
        CurrencyPrice {
            name: exchange_id.to_owned(),
            exchange_id: exchange_id.to_owned(),
            price: crate::prices::Price {
                exalted,
                quantity: 100,
                history: Vec::new(),
            },
        }
    }

    #[test]
    fn a_currency_without_price_is_never_suspicious() {
        let reference = Reference {
            exalted: 9300.0,
            worth: WORTH,
        };

        assert!(reference.suspicious(6.0, "exalted"));
        assert!(reference.suspicious(3.0, "divine"));
        assert!(!reference.suspicious(1.0, "annul"));
        assert!(!reference.suspicious(1.0, "waystone-12"));
    }

    #[test]
    fn an_offer_in_a_currency_of_the_price_database_is_counted_at_its_price() {
        // A rare bow: 1,000 Vaal Orbs at 0.75 ex are 750 ex, between 1 and 1.1 div; the
        // Waystones have no price.
        let currencies = [priced("vaal", 0.75), priced("mirror", 70_000.0)];
        let worth = Worth {
            rates: RATES,
            currencies: &currencies,
        };
        let offers = [
            offer(1.0, "divine"),
            offer(1.1, "divine"),
            offer(1000.0, "vaal"),
            offer(3.0, "waystone-12"),
        ];

        let bow = estimate(&offers, 4, worth, None).unwrap();

        assert_eq!(bow.counted, 3, "the waystones left out");
        assert!((bow.exalted - 750.0).abs() < 1e-9, "the vaal in the middle");
        assert_eq!(estimate(&[offer(1.0, "waystone-12")], 1, worth, None), None);
        let mirror = estimate(&[offer(1.0, "mirror")], 1, worth, None).unwrap();
        assert!((mirror.exalted - 70_000.0).abs() < 1e-9);
    }

    #[test]
    fn an_offer_in_a_currency_of_the_price_database_may_be_suspicious() {
        let currencies = [priced("vaal", 0.75)];
        let reference = Reference {
            exalted: 9300.0,
            worth: Worth {
                rates: RATES,
                currencies: &currencies,
            },
        };

        // 2,000 Vaal Orbs are 1,500 ex, under a quarter of 9,300; 4,000 are 3,000 ex.
        assert!(reference.suspicious(2000.0, "vaal"));
        assert!(!reference.suspicious(4000.0, "vaal"));
    }

    #[test]
    fn the_chaos_and_divine_orbs_are_worth_their_rates() {
        // The prices of the currency exchange may differ from the rates a little: the price
        // block shows the estimate with the rates, so the offers are compared with them too.
        let currencies = [priced("divine", 650.0), priced("chaos", 60.0)];
        let worth = Worth {
            rates: RATES,
            currencies: &currencies,
        };

        let divine = estimate(&[offer(1.0, "divine")], 1, worth, None).unwrap();
        let chaos = estimate(&[offer(1.0, "chaos")], 1, worth, None).unwrap();

        assert!((divine.exalted - 700.0).abs() < 1e-9);
        assert!((chaos.exalted - 70.0).abs() < 1e-9);
    }

    #[test]
    fn a_price_on_too_few_trades_isnt_counted() {
        // A Lesser Charging Rune at 40,156 ex on one trade, as poe2scout priced it.
        let mut rune = priced("lesser-charging-rune", 40_156.0);
        rune.price.quantity = MIN_QUANTITY - 1;
        let mut mirror = priced("mirror", 70_000.0);
        mirror.price.quantity = MIN_QUANTITY;
        let currencies = [rune, mirror];
        let worth = Worth {
            rates: RATES,
            currencies: &currencies,
        };
        let reference = Reference {
            exalted: 1_000_000.0,
            worth,
        };

        let rune = [offer(1.0, "lesser-charging-rune")];
        assert_eq!(estimate(&rune, 1, worth, None), None);
        assert!(!reference.suspicious(1.0, "lesser-charging-rune"));
        assert!(estimate(&[offer(1.0, "mirror")], 1, worth, None).is_some());
        assert!(reference.suspicious(1.0, "mirror"));
    }

    #[test]
    fn a_suspicious_offer_in_a_currency_of_the_price_database_isnt_counted() {
        // 10 Vaal Orbs at 0.75 ex are 7.5 ex, under a quarter of the median of the page.
        let currencies = [priced("vaal", 0.75)];
        let worth = Worth {
            rates: RATES,
            currencies: &currencies,
        };
        let mut offers = vec![offer(10.0, "vaal")];
        offers.extend(divines(&[1.0, 1.2, 1.4, 1.6]));

        let baited = estimate(&offers, 5, worth, None).unwrap();

        assert_eq!(baited.counted, 4);
        assert!((baited.exalted - 1.3 * 700.0).abs() < 1e-9);
        assert!(baited.reference.suspicious(10.0, "vaal"));
    }

    #[test]
    fn a_rate_of_nothing_counts_no_offer_as_free() {
        for rates in [
            Rates {
                chaos: 0.0,
                divine: f64::NAN,
            },
            Rates {
                chaos: f64::INFINITY,
                divine: -1.0,
            },
        ] {
            let worth = Worth {
                rates,
                currencies: &[],
            };

            for id in ["chaos", "divine"] {
                assert_eq!(estimate(&[offer(1.0, id)], 1, worth, None), None, "{id}");
            }
        }
    }

    #[test]
    fn a_currency_priced_at_nothing_isnt_counted_as_free() {
        let currencies = [
            priced("vaal", 0.0),
            priced("regal", f64::NAN),
            priced("alch", -1.0),
        ];
        let worth = Worth {
            rates: RATES,
            currencies: &currencies,
        };
        let reference = Reference {
            exalted: 9300.0,
            worth,
        };

        for id in ["vaal", "regal", "alch"] {
            assert_eq!(estimate(&[offer(5.0, id)], 1, worth, None), None, "{id}");
            assert!(!reference.suspicious(5.0, id), "{id}");
        }
    }
}
