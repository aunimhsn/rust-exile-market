use super::*;
use crate::item::QualityKind;
use crate::trade::offers::OfferQuality;
use crate::trade::offers::fixtures::item_offer;

const RATES: Rates = Rates {
    chaos: 70.0,
    divine: 700.0,
};
const WORTH: Worth = Worth {
    rates: RATES,
    currencies: &[],
};

fn divines(amounts: &[f64]) -> Vec<ItemOffer> {
    amounts
        .iter()
        .map(|&amount| item_offer(amount, "divine"))
        .collect()
}

#[test]
fn the_estimate_is_the_median_of_the_five_cheapest_offers_but_the_suspicious_ones() {
    // The rare bow of the mock-up: 300 ex is under a quarter of the median of the page.
    let mut offers = vec![item_offer(300.0, "exalted")];
    offers.extend(divines(&[2.0, 2.2, 2.4, 2.5, 3.0, 3.1, 3.5, 4.0, 4.2]));

    let bow = Estimate::of(&offers, 18, WORTH, None).unwrap();

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
        item_offer(50.0, "exalted"),
        item_offer(70.0, "exalted"),
        item_offer(1.0, "chaos"),
    ];

    let magic = Estimate::of(&offers, 3, WORTH, None).unwrap();

    assert!((magic.exalted - 70.0).abs() < 1e-9);
    assert_eq!(magic.counted, 3);
}

#[test]
fn the_offers_are_sorted_again_with_the_rates_of_the_prices() {
    // The trade site put 1 div before 650 ex: its rate of the Divine Orb is lower.
    let offers = [
        item_offer(1.0, "divine"),
        item_offer(650.0, "exalted"),
        item_offer(1.1, "divine"),
    ];

    let sorted = Estimate::of(&offers, 3, WORTH, None).unwrap();

    assert!((sorted.exalted - 700.0).abs() < 1e-9);
}

#[test]
fn an_even_count_of_offers_gives_the_mean_of_the_two_middle_ones() {
    let even = Estimate::of(&divines(&[1.0, 2.0, 3.0, 5.0]), 4, WORTH, None).unwrap();

    assert!((even.exalted - 2.5 * 700.0).abs() < 1e-9);
    assert_eq!(even.counted, 4);
}

#[test]
fn the_suspicious_offers_are_left_out_of_the_count() {
    // Six offers, two of them bait: four are counted, of the six found.
    let mut offers = vec![item_offer(10.0, "exalted"), item_offer(20.0, "exalted")];
    offers.extend(divines(&[1.0, 1.2, 1.4, 1.6]));

    let baited = Estimate::of(&offers, 6, WORTH, None).unwrap();

    assert_eq!(baited.counted, 4);
    assert_eq!(baited.total, 6);
    assert!((baited.exalted - 1.3 * 700.0).abs() < 1e-9);
}

/// The core review of 2026-11-10: "far below" starts under the quarter, not at it.
#[test]
fn an_offer_at_exactly_a_quarter_of_the_reference_is_counted() {
    // The median of the five is 4 div: its quarter is 1 div.
    let offers = divines(&[1.0, 2.0, 4.0, 4.0, 4.0]);

    let at_the_quarter = Estimate::of(&offers, 5, WORTH, None).unwrap();

    assert_eq!(at_the_quarter.counted, 5);
    assert!(!at_the_quarter.reference.suspicious(1.0, "divine"));
    assert!(at_the_quarter.reference.suspicious(699.0, "exalted"));
    let under = Estimate::of(&divines(&[0.99, 2.0, 4.0, 4.0, 4.0]), 5, WORTH, None).unwrap();
    assert_eq!(under.counted, 4);
}

/// The limit written at the top of this file, pinned so that a change of the rule shows:
/// the median of the page tells the bait only while fewer than half its offers are bait.
#[test]
fn past_half_the_page_of_bait_the_estimate_is_the_one_of_the_bait() {
    let real = [2.0, 2.2, 2.4, 2.5, 3.0, 3.1];
    let page = |bait: usize| {
        let mut offers: Vec<ItemOffer> = (0..bait).map(|_| item_offer(1.0, "exalted")).collect();
        offers.extend(divines(&real[..10 - bait]));
        Estimate::of(&offers, 10, WORTH, None).unwrap()
    };

    // Four in ten: left out, and the estimate is the median of the five cheapest real ones.
    let four = page(4);
    assert!(four.reference.suspicious(1.0, "exalted"));
    assert!((four.exalted - 2.4 * 700.0).abs() < 1e-9);
    // Five in ten: the median falls between the bait and the cheapest real offer, far
    // enough to still tell the bait here, since the bait is so cheap.
    let five = page(5);
    assert!(five.reference.suspicious(1.0, "exalted"));
    // Six in ten: the median is the bait, which is then the estimate.
    let six = page(6);
    assert!(!six.reference.suspicious(1.0, "exalted"));
    assert!((six.exalted - 1.0).abs() < 1e-9);
    assert_eq!(six.counted, 5);
}

#[test]
fn an_offer_without_a_known_price_isnt_counted() {
    let mut offers = vec![item_offer(1.0, "annul"), item_offer(2.0, "divine")];
    offers[0].price = None;
    offers.push(item_offer(3.0, "annul"));

    assert_eq!(
        Estimate::of(&[item_offer(1.0, "annul")], 1, WORTH, None),
        None
    );
    assert_eq!(Estimate::of(&[], 0, WORTH, None), None);

    let known = Estimate::of(&offers, 3, WORTH, None).unwrap();

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

    let mana = Estimate::of(&offers, 27, WORTH, Some(Catalyst::Mana)).unwrap();
    let any = Estimate::of(&offers, 27, WORTH, None).unwrap();

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
        price: price::Price {
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
        item_offer(1.0, "divine"),
        item_offer(1.1, "divine"),
        item_offer(1000.0, "vaal"),
        item_offer(3.0, "waystone-12"),
    ];

    let bow = Estimate::of(&offers, 4, worth, None).unwrap();

    assert_eq!(bow.counted, 3, "the waystones left out");
    assert!((bow.exalted - 750.0).abs() < 1e-9, "the vaal in the middle");
    assert_eq!(
        Estimate::of(&[item_offer(1.0, "waystone-12")], 1, worth, None),
        None
    );
    let mirror = Estimate::of(&[item_offer(1.0, "mirror")], 1, worth, None).unwrap();
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

    let divine = Estimate::of(&[item_offer(1.0, "divine")], 1, worth, None).unwrap();
    let chaos = Estimate::of(&[item_offer(1.0, "chaos")], 1, worth, None).unwrap();

    assert!((divine.exalted - 700.0).abs() < 1e-9);
    assert!((chaos.exalted - 70.0).abs() < 1e-9);
}

#[test]
fn a_price_on_too_few_trades_isnt_counted() {
    // A Lesser Charging Rune at 40,156 ex on one trade, as poe2scout priced it.
    let mut rune = priced("lesser-charging-rune", 40_156.0);
    rune.price.quantity = price::MIN_TRADES - 1;
    let mut mirror = priced("mirror", 70_000.0);
    mirror.price.quantity = price::MIN_TRADES;
    let currencies = [rune, mirror];
    let worth = Worth {
        rates: RATES,
        currencies: &currencies,
    };
    let reference = Reference {
        exalted: 1_000_000.0,
        worth,
    };

    let rune = [item_offer(1.0, "lesser-charging-rune")];
    assert_eq!(Estimate::of(&rune, 1, worth, None), None);
    assert!(!reference.suspicious(1.0, "lesser-charging-rune"));
    assert!(Estimate::of(&[item_offer(1.0, "mirror")], 1, worth, None).is_some());
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
    let mut offers = vec![item_offer(10.0, "vaal")];
    offers.extend(divines(&[1.0, 1.2, 1.4, 1.6]));

    let baited = Estimate::of(&offers, 5, worth, None).unwrap();

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
            assert_eq!(
                Estimate::of(&[item_offer(1.0, id)], 1, worth, None),
                None,
                "{id}"
            );
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
        assert_eq!(
            Estimate::of(&[item_offer(5.0, id)], 1, worth, None),
            None,
            "{id}"
        );
        assert!(!reference.suspicious(5.0, id), "{id}");
    }
}

/// The core review of 2026-11-10: only the rate was looked at, not the amount.
#[test]
fn an_amount_of_nothing_or_beyond_every_price_isnt_counted() {
    let worth = Worth {
        rates: Rates {
            chaos: 75.0,
            divine: 700.0,
        },
        currencies: &[],
    };

    for amount in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::MAX] {
        assert_eq!(worth.in_exalted(amount, "divine"), None, "{amount}");
    }
    assert_eq!(worth.in_exalted(2.0, "divine"), Some(1400.0));
}
