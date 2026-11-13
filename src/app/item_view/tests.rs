use std::time::{Instant, SystemTime};

use super::*;
use crate::clipboard::ClipboardEvent;
use crate::item::CopiedItem;
use crate::trade::search::Search;

#[test]
fn a_lit_state_tag_reaches_the_search() {
    use crate::item::ItemState;
    use crate::trade::query::Query;

    let mut session = Session::new(Some(League::new("Standard")));
    session.apply(ClipboardEvent::Copied {
        item: CopiedItem::from_text(
            "Item Class: Rings\nRarity: Rare\nWoe Loop\nRuby Ring\n--------\n\
             Item Level: 80\n--------\nCorrupted",
        )
        .unwrap(),
        hotkey: None,
    });
    let shown = session.item().expect("an item");
    let item = shown.parsed().expect("read");
    let corrupted = |filters: &Filters| {
        let search = session
            .trade_search(item, search_filters(&session, shown, item, filters))
            .expect("a search");
        let Query::Items(items) = search.query else {
            panic!("a search of items");
        };
        items.states.corrupted
    };
    let mut filters = Filters::default();

    assert_eq!(corrupted(&filters), None);
    filters.toggle_state(ItemState::Corrupted);
    assert_eq!(corrupted(&filters), Some(true));
}

const RATES: Rates = Rates {
    chaos: 70.0,
    divine: 700.0,
};
const WORTH: Worth = Worth {
    rates: RATES,
    currencies: &[],
};

/// Offers for `amounts` Divine Orbs.
fn divines(amounts: &[f64]) -> Vec<crate::trade::offers::ItemOffer> {
    amounts
        .iter()
        .map(|&amount| crate::trade::offers::fixtures::item_offer(amount, "divine"))
        .collect()
}

/// The owner's amulet of mana quality.
fn amulet() -> Item {
    Item::parse(
        &CopiedItem::from_text(include_str!(
            "../../item/test-items/en-owner-rare-amulet-catalyst.txt"
        ))
        .unwrap(),
    )
    .unwrap()
}

/// A search of items with the filters of `properties`.
fn searched(properties: Vec<crate::trade::query::PropertyFilter>) -> Search {
    use crate::trade::query::{ItemQuery, Query, SearchedBase, StatFilter, StateFilters};

    Search {
        language: crate::language::Language::English,
        league: League::new("Standard"),
        query: Query::Items(ItemQuery {
            name: None,
            base: SearchedBase::Type("Gemini Bow".to_owned()),
            properties,
            rarity: None,
            states: StateFilters {
                corrupted: Some(false),
                ..StateFilters::default()
            },
            // A stat compared: without, a rare has no estimate.
            stats: vec![StatFilter {
                ids: vec!["explicit.stat_3917489142".to_owned()],
                minimum: Some(10.0),
                maximum: None,
                rules_out: crate::trade::query::RulesOut::OfTheBase,
            }],
        }),
    }
}

/// Offers left to fetch, or none.
fn rest(more: bool) -> crate::trade::search::Rest {
    let left: &[&str] = if more { &["k"] } else { &[] };
    crate::trade::search::Rest::new(crate::language::Language::English, "Q1", left)
}

/// The answer to a search: `offers` of the `total` found, 10 at most; the next search
/// allowed from `now`.
fn answer(
    offers: Vec<crate::trade::offers::ItemOffer>,
    total: u64,
    more: bool,
    now: Instant,
) -> crate::trade::search::Answer {
    use crate::trade::search::{Answer, NextSearch, Outcome};

    Answer {
        outcome: Outcome::Items {
            offers,
            total,
            rest: rest(more),
        },
        next_search: NextSearch {
            items: now,
            exchange: now,
        },
    }
}

#[test]
fn the_estimate_counts_the_answer_to_the_search_of_an_item_without_a_fixed_price() {
    use crate::trade::search::{Answer, NextSearch, Outcome};

    let amulet = amulet();
    let now = Instant::now();
    let rare = Err(NoFixedPrice::NotFixedName);
    let mut offers = Offers::default();
    let _ = offers.search(searched(Vec::new()), now);
    // One offer of the first ten was gone since the search.
    offers.apply(
        answer(
            divines(&[1.0, 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8]),
            30,
            true,
            now,
        ),
        SystemTime::now(),
    );
    let first = shown_estimate(&offers, &amulet, &rare, Some(WORTH)).unwrap();
    let _ = offers.more();
    // Counted, the first of them would lower the estimate to 1.1 div.
    offers.apply(
        Answer {
            outcome: Outcome::More {
                offers: divines(&[0.5; 10]),
                rest: rest(true),
            },
            next_search: NextSearch {
                items: now,
                exchange: now,
            },
        },
        SystemTime::now(),
    );

    assert_eq!(
        shown_estimate(&offers, &amulet, &rare, Some(WORTH)),
        Some(first)
    );
    assert!((first.exalted - 1.2 * 700.0).abs() < 1e-9);
    assert_eq!(first.total, 30);
    assert_eq!(
        shown_estimate(&offers, &amulet, &rare, None),
        None,
        "no rates yet"
    );
    assert_eq!(
        shown_estimate(&offers, &amulet, &Err(NoFixedPrice::NotPriced), Some(WORTH)),
        None,
        "a fixed-name item isn't estimated"
    );
    let _ = offers.search(searched(Vec::new()), now);
    assert_eq!(
        shown_estimate(&offers, &amulet, &rare, Some(WORTH)),
        None,
        "while searching"
    );
}

#[test]
fn the_price_of_a_unique_tells_its_suspicious_offers_in_every_priced_currency() {
    use crate::price::{CurrencyPrice, Price, UniqueModifiers, UniquePrice};

    let price = |exalted| Price {
        exalted,
        quantity: 100,
        history: Vec::new(),
    };
    let unique = UniquePrice {
        name: "Storm Bite".to_owned(),
        base_type: "Gemini Bow".to_owned(),
        price: price(2000.0),
        modifiers: UniqueModifiers::default(),
    };
    let currencies = [CurrencyPrice {
        name: "Vaal Orb".to_owned(),
        exchange_id: "vaal".to_owned(),
        price: price(10.0),
    }];
    let worth = Worth {
        rates: RATES,
        currencies: &currencies,
    };
    let fixed = Ok((FixedPrice::Unique(&unique), RATES));

    let unique_reference = reference(&fixed, Some(worth)).unwrap();

    // 40 Vaal Orbs are 400 ex, under a quarter of 2,000; 60 are 600 ex.
    assert!(unique_reference.suspicious(40.0, "vaal"));
    assert!(!unique_reference.suspicious(60.0, "vaal"));
    assert_eq!(reference(&fixed, None), None, "the prices not known");
    assert_eq!(
        reference(&Err(NoFixedPrice::NotFixedName), Some(worth)),
        None,
        "the estimate tells them"
    );
}

#[test]
fn a_price_of_the_currency_exchange_on_few_trades_tells_no_suspicious_offer() {
    use crate::price::{CurrencyPrice, MIN_TRADES, Price};

    // A Lesser Charging Rune at 40,156 ex on one trade: its honest offers aren't bait.
    let rune = |quantity| CurrencyPrice {
        name: "Lesser Charging Rune".to_owned(),
        exchange_id: "lesser-charging-rune".to_owned(),
        price: Price {
            exalted: 40_156.0,
            quantity,
            history: Vec::new(),
        },
    };
    let thin = rune(MIN_TRADES - 1);
    let traded = rune(MIN_TRADES);

    assert_eq!(
        reference(&Ok((FixedPrice::Currency(&thin), RATES)), Some(WORTH)),
        None
    );
    let traded = reference(&Ok((FixedPrice::Currency(&traded), RATES)), Some(WORTH)).unwrap();
    assert!((traded.exalted - 40_156.0).abs() < 1e-9);
}

#[test]
fn a_rare_or_magic_item_searched_without_its_stats_has_no_estimate() {
    use crate::trade::query::Query;

    let amulet = amulet();
    let now = Instant::now();
    let estimated = |item: &Item, stats: bool| {
        let mut search = searched(Vec::new());
        if let (false, Query::Items(query)) = (stats, &mut search.query) {
            query.stats.clear();
        }
        let mut offers = Offers::default();
        let _ = offers.search(search, now);
        offers.apply(
            answer(divines(&[1.0, 1.1, 1.2]), 3, false, now),
            SystemTime::now(),
        );
        shown_estimate(&offers, item, &Err(NoFixedPrice::NotFixedName), Some(WORTH))
    };

    // The offers of any item of its base say nothing of its price.
    assert!(estimated(&amulet, true).is_some());
    assert_eq!(estimated(&amulet, false), None);
    let magic = Item::parse(
        &CopiedItem::from_text(include_str!("../../item/test-items/en-magic-mace.txt")).unwrap(),
    )
    .unwrap();
    assert_eq!(estimated(&magic, false), None);
    // A normal item, an exceptional base searched by its sockets or its quality, a skill
    // gem by its level and a waystone by its tier: priced by them, not by modifiers. A
    // magic item is estimated as a rare.
    for text in [
        include_str!("../../item/test-items/en-owner-normal-quarterstaff-exceptional.txt"),
        include_str!("../../item/test-items/en-owner-skill-gem.txt"),
        // A waystone, priced first by its tier, which its base type holds.
        include_str!("../../item/test-items/en-owner-rare-waystone.txt"),
    ] {
        let item = Item::parse(&CopiedItem::from_text(text).unwrap()).unwrap();
        assert!(estimated(&item, false).is_some(), "{:?}", item.rarity());
    }
}

#[test]
fn the_estimate_leaves_out_another_kind_of_quality_only_when_the_quality_is_searched() {
    use crate::filters::Property;
    use crate::item::{Catalyst, QualityKind};
    use crate::trade::offers::OfferQuality;
    use crate::trade::query::PropertyFilter;

    let amulet = amulet();
    // The cheapest offer, of attack quality.
    let mut first_page = divines(&[1.0, 1.1, 1.2]);
    first_page[0].item.quality = Some(OfferQuality {
        value: 40,
        kind: Some(QualityKind {
            catalyst: Some(Catalyst::Attack),
            words: "Attack Modifiers".to_owned(),
        }),
    });
    let quality = PropertyFilter {
        property: Property::Quality,
        minimum: Some(40.0),
        maximum: None,
        rules_out: crate::trade::query::RulesOut::OfTheLeague,
    };

    for (properties, counted) in [(vec![quality], 2), (Vec::new(), 3)] {
        let mut offers = Offers::default();
        let now = Instant::now();
        let _ = offers.search(searched(properties), now);
        offers.apply(answer(first_page.clone(), 3, false, now), SystemTime::now());
        let estimate = shown_estimate(
            &offers,
            &amulet,
            &Err(NoFixedPrice::NotFixedName),
            Some(WORTH),
        )
        .unwrap();
        assert_eq!(estimate.counted, counted);
    }
}
