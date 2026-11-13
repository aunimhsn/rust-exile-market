use super::*;
use crate::estimate::{Reference, Worth};
use crate::prices::{CurrencyPrice, UniqueModifiers, UniquePrice};
use std::time::Duration;

const RATES: Rates = Rates {
    chaos: 75.0,
    divine: 750.0,
};
const WORTH: Worth = Worth {
    rates: RATES,
    currencies: &[],
};
const KNOWN: PricesState = PricesState::Known {
    age: Duration::ZERO,
    offline: false,
};
/// The complete prices of Standard.
const STANDARD: LeaguePrices = LeaguePrices {
    state: KNOWN,
    league: Some("Standard"),
};
const DOWNLOADING: PricesState = PricesState::Downloading { progress: 0.5 };

fn notice<'a>(
    price: &'a Result<(FixedPrice<'a>, Rates), NoFixedPrice>,
    prices: PricesState,
    league: Option<&'a str>,
) -> PriceView<'a> {
    view(
        price,
        None,
        Rarity::Currency,
        LeaguePrices {
            state: prices,
            league,
        },
    )
}

#[test]
fn a_rare_item_has_no_market_price() {
    let price = Err(NoFixedPrice::NotFixedName);

    assert_eq!(
        view(&price, None, Rarity::Magic, STANDARD),
        PriceView::Notice(Notice::NoMarketPrice(Rarity::Magic))
    );
}

#[test]
fn a_rare_or_magic_item_asks_for_a_stat_to_estimate_it_and_says_nothing_when_none_can_be() {
    use StatChecks::{Checked, NoneToCheck, Unchecked};

    // A rare or magic item.
    assert_eq!(
        estimate_hint(true, Checked),
        Some(Message::SearchToEstimate)
    );
    assert_eq!(
        estimate_hint(true, Unchecked),
        Some(Message::CheckToEstimate)
    );
    assert_eq!(estimate_hint(true, NoneToCheck), None);
    // A normal item, a gem or a waystone: priced by its base, its quality, its level or
    // its tier, not by modifiers.
    for checks in [Checked, Unchecked, NoneToCheck] {
        assert_eq!(
            estimate_hint(false, checks),
            Some(Message::SearchToEstimate),
            "{checks:?}"
        );
    }
}

#[test]
fn a_skill_gem_has_no_market_price_either() {
    let price = Err(NoFixedPrice::NotFixedName);

    assert_eq!(
        view(&price, None, Rarity::Gem, STANDARD),
        PriceView::Notice(Notice::NoMarketPrice(Rarity::Gem))
    );
}

#[test]
fn once_its_offers_are_searched_a_rare_item_has_an_estimated_price() {
    let estimate = Estimate {
        exalted: 1680.0,
        counted: 5,
        total: 18,
        reference: Reference {
            exalted: 1925.0,
            worth: WORTH,
        },
        searched_catalyst: None,
    };

    assert_eq!(
        view(
            &Err(NoFixedPrice::NotFixedName),
            Some(estimate),
            Rarity::Rare,
            STANDARD
        ),
        PriceView::Estimated(estimate)
    );
}

#[test]
fn an_item_missing_from_the_complete_prices_has_no_price_found() {
    let price = Err(NoFixedPrice::NotPriced);

    assert_eq!(
        notice(&price, KNOWN, Some("Standard")),
        PriceView::Notice(Notice::NoPriceFound("Standard"))
    );
    // Its category may still come.
    assert_eq!(
        notice(&price, DOWNLOADING, Some("Standard")),
        PriceView::Notice(Notice::PricesDownloading(Some("Standard")))
    );
    assert_eq!(
        notice(&price, PricesState::Unavailable, Some("Standard")),
        PriceView::Notice(Notice::PricesUnavailable)
    );
}

#[test]
fn before_the_prices_the_block_says_they_are_coming() {
    let price = Err(NoFixedPrice::PricesUnknown);

    assert_eq!(
        notice(&price, DOWNLOADING, None),
        PriceView::Notice(Notice::PricesDownloading(None))
    );
    assert_eq!(
        notice(&price, PricesState::Unavailable, None),
        PriceView::Notice(Notice::PricesUnavailable)
    );
    assert_eq!(
        notice(&Err(NoFixedPrice::NamesUnknown), KNOWN, Some("Standard")),
        PriceView::Notice(Notice::FrenchNamesDownloading)
    );
}

#[test]
fn the_price_is_written_in_each_currency() {
    assert_eq!(in_currency(9000.0, Currency::Divine, RATES).0, "12");
    assert_eq!(in_currency(9000.0, Currency::Chaos, RATES).0, "120");
    assert_eq!(in_currency(9000.0, Currency::Exalted, RATES).0, "9K");
}

#[test]
fn an_amount_under_a_tenth_of_divine_or_chaos_orb_is_hidden() {
    let shown =
        |exalted, currency| shown_amount(exalted, currency, RATES).map(|(amount, _)| amount);
    // 0.1 div, 1 c: all shown.
    assert_eq!(shown(75.0, Currency::Divine).as_deref(), Some("0.1"));
    // 0.08 div, 0.8 c: the Divine Orbs hidden.
    assert_eq!(shown(60.0, Currency::Divine), None);
    assert_eq!(shown(60.0, Currency::Chaos).as_deref(), Some("0.8"));
    // 0.0013 div, 0.013 c: only the Exalted Orbs.
    assert_eq!(shown(1.0, Currency::Divine), None);
    assert_eq!(shown(1.0, Currency::Chaos), None);
    assert_eq!(shown(1.0, Currency::Exalted).as_deref(), Some("1"));
    assert_eq!(shown(0.004, Currency::Exalted).as_deref(), Some("<0.01"));
}

#[test]
fn the_threshold_reads_the_amount_as_written() {
    // 0.0953 div, written 0.1: shown; 0.0947 div, written 0.09: hidden.
    assert!(shown_amount(71.5, Currency::Divine, RATES).is_some());
    assert!(shown_amount(71.0, Currency::Divine, RATES).is_none());
}

#[test]
fn the_unit_price_leaves_out_the_small_amounts() {
    assert_eq!(unit_amounts(20.0, RATES), "0.27 c · 20 ex");
    assert_eq!(unit_amounts(187.5, RATES), "0.25 div · 2.5 c · 188 ex");
}

#[test]
fn a_row_of_a_list_is_in_divine_orbs_from_one_divine_orb() {
    assert_eq!(
        list_price(3000.0, RATES),
        ("4".to_owned(), Currency::Divine)
    );
    assert_eq!(
        list_price(640.0, RATES),
        ("640".to_owned(), Currency::Exalted)
    );
}

#[test]
fn a_change_is_rounded_to_a_whole_percent() {
    assert_eq!(rounded_percent(0.044), 4);
    assert_eq!(rounded_percent(0.106), 11);
}

#[test]
fn a_fixed_price_on_too_little_is_told_uncertain() {
    let price = |quantity| Price {
        exalted: 120.0,
        quantity,
        history: Vec::new(),
    };
    let currency = |quantity| CurrencyPrice {
        name: "Lesser Charging Rune".to_owned(),
        exchange_id: "lesser-charging-rune".to_owned(),
        price: price(quantity),
    };
    let unique = |quantity| UniquePrice {
        name: "Headhunter".to_owned(),
        base_type: "Heavy Belt".to_owned(),
        price: price(quantity),
        modifiers: UniqueModifiers::default(),
    };
    let of = |fixed: FixedPrice| match view(&Ok((fixed, RATES)), None, Rarity::Unique, STANDARD) {
        PriceView::Priced(Priced { of, .. }) => of,
        other => panic!("{other:?}"),
    };

    assert_eq!(
        of(FixedPrice::Currency(&currency(9))),
        Fixed::Exchange { thin: true }
    );
    assert_eq!(
        of(FixedPrice::Currency(&currency(10))),
        Fixed::Exchange { thin: false }
    );
    assert_eq!(
        of(FixedPrice::Unique(&unique(4))),
        Fixed::Unique { uncertain: true }
    );
    assert_eq!(
        of(FixedPrice::Unique(&unique(5))),
        Fixed::Unique { uncertain: false }
    );
}
