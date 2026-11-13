use egui::Pos2;

use super::*;
use crate::price::Rates;
use crate::price::estimate::Worth;
use crate::trade::offers::fixtures::item_offer;

const RATES: Rates = Rates {
    chaos: 75.0,
    divine: 750.0,
};
const WORTH: Worth = Worth {
    rates: RATES,
    currencies: &[],
};
/// 12.4 Divine Orbs.
const REFERENCE: Reference = Reference {
    exalted: 9300.0,
    worth: WORTH,
};

#[test]
fn an_offer_under_a_quarter_of_the_market_price_is_suspicious() {
    let offers = [
        item_offer(6.0, "exalted"),
        item_offer(1.0, "chaos"),
        item_offer(3.0, "divine"),
        ItemOffer {
            seller: SellerStatus::Away,
            ..item_offer(3.2, "divine")
        },
    ];
    let now = SystemTime::UNIX_EPOCH + Duration::from_hours(2);

    let rows = item_rows(&offers, Some(REFERENCE), None, now);

    let suspicious: Vec<bool> = rows.iter().map(|row| row.is_suspicious).collect();
    assert_eq!(suspicious, [true, true, true, false]);
    assert_eq!(
        rows[3],
        Row {
            price: Some(RowPrice {
                amount: "3.2".to_owned(),
                currency: exchange_currencies::shown("divine"),
            }),
            is_suspicious: false,
            other_quality: None,
            stock: None,
            is_away: true,
            listed: Some(Duration::from_hours(2)),
        }
    );
    assert!(
        item_rows(&offers, None, None, now)
            .iter()
            .all(|row| !row.is_suspicious),
        "without a market price, nothing tells"
    );
}

#[test]
fn an_offer_of_another_kind_of_quality_is_marked_while_the_quality_is_searched() {
    use crate::item::QualityKind;
    use crate::trade::offers::OfferQuality;

    let of_kind = |amount, catalyst| {
        let mut offer = item_offer(amount, "divine");
        offer.item.quality = Some(OfferQuality {
            value: 40,
            kind: Some(QualityKind {
                catalyst: Some(catalyst),
                words: String::new(),
            }),
        });
        offer
    };
    // An attack amulet far below the market of the mana amulets: another market, not
    // bait.
    let offers = [
        of_kind(1.0, Catalyst::Attack),
        of_kind(12.0, Catalyst::Mana),
    ];
    let now = SystemTime::UNIX_EPOCH;

    let searched = item_rows(&offers, Some(REFERENCE), Some(Catalyst::Mana), now);
    let unsearched = item_rows(&offers, Some(REFERENCE), None, now);

    assert_eq!(searched[0].other_quality, Some(Catalyst::Attack));
    assert!(!searched[0].is_suspicious);
    assert_eq!(searched[1].other_quality, None);
    assert_eq!(unsearched[0].other_quality, None, "every offer counts");
    assert!(unsearched[0].is_suspicious);
}

#[test]
fn another_currency_has_its_icon_and_its_name_and_isnt_suspicious_without_price() {
    let rows = item_rows(
        &[item_offer(1.0, "annul")],
        Some(REFERENCE),
        None,
        SystemTime::UNIX_EPOCH,
    );

    let price = rows[0].price.as_ref().unwrap();
    assert_eq!(
        price.currency.map(|currency| currency.id.as_str()),
        Some("annul")
    );
    assert_eq!(price.label(Language::English), "1 Orb of Annulment");
    assert_eq!(price.label(Language::French), "1 Orbe d'annulation");
    assert!(!rows[0].is_suspicious);
}

#[test]
fn an_offer_in_a_currency_of_the_price_database_may_be_suspicious() {
    // 12.4 div of reference: 10 Orbs of Annulment at 150 ex are 1,500 ex, under a quarter.
    let currencies = [crate::price::CurrencyPrice {
        name: "Orb of Annulment".to_owned(),
        exchange_id: "annul".to_owned(),
        price: crate::price::Price {
            exalted: 150.0,
            quantity: 100,
            history: Vec::new(),
        },
    }];
    let reference = Reference {
        exalted: 9300.0,
        worth: Worth {
            rates: RATES,
            currencies: &currencies,
        },
    };
    let offers = [item_offer(10.0, "annul"), item_offer(60.0, "annul")];

    let rows = item_rows(&offers, Some(reference), None, SystemTime::UNIX_EPOCH);

    let suspicious: Vec<bool> = rows.iter().map(|row| row.is_suspicious).collect();
    assert_eq!(suspicious, [true, false]);
}

#[test]
fn every_currency_is_named_the_three_orbs_too() {
    let price = row_price(12.4, "divine");

    assert_eq!(price.label(Language::English), "12.4 Divine Orb");
    assert_eq!(price.label(Language::French), "12.4 Orbe divin");
}

#[test]
fn each_column_has_its_title() {
    let i18n = Translator::new(Language::English);
    let row = Rect::from_min_size(Pos2::ZERO, vec2(400.0, 20.0));
    for kind in [SearchKind::Exchange, SearchKind::Items] {
        // The first title is on the left, each other one over a right edge.
        assert_eq!(
            column_titles(&i18n, kind).len(),
            Columns::of(row, kind).right_edges().len() + 1,
            "{kind:?}"
        );
    }
}

#[test]
fn a_currency_the_application_doesnt_know_has_no_icon_nor_name() {
    let price = row_price(4.0, "not-a-currency");

    assert_eq!(price.currency, None);
    assert_eq!(price.label(Language::English), "4");
}

#[test]
fn an_offer_of_the_exchange_shows_the_price_of_one_item_and_its_stock() {
    let offer = ExchangeOffer {
        seller: SellerStatus::Online,
        listed: None,
        pay: Price {
            amount: 350.0,
            currency: "exalted".to_owned(),
        },
        get: Price {
            amount: 2.0,
            currency: "annul".to_owned(),
        },
        stock: 40,
        whisper: String::new(),
    };
    let reference = Reference {
        exalted: 182.0,
        worth: WORTH,
    };

    let rows = exchange_rows(&[offer], Some(reference), SystemTime::UNIX_EPOCH);

    assert_eq!(
        rows,
        [Row {
            price: Some(RowPrice {
                amount: "175".to_owned(),
                currency: exchange_currencies::shown("exalted"),
            }),
            is_suspicious: false,
            other_quality: None,
            stock: Some(40),
            is_away: false,
            listed: None,
        }]
    );
}
