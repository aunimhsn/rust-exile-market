use super::*;
use crate::language::Language;
use crate::league::League;
use crate::trade::offers::{OfferItem, Price, SellerStatus};
use crate::trade::query::{ItemQuery, Query, StateFilters};

pub(crate) fn item_search() -> Search {
    Search {
        language: Language::English,
        league: League::new("Standard"),
        query: Query::Items(ItemQuery {
            name: Some("Andvarius".to_owned()),
            base_type: Some("Gold Ring".to_owned()),
            category: None,
            properties: Vec::new(),
            rarity: None,
            states: StateFilters {
                corrupted: Some(false),
                ..StateFilters::default()
            },
            stats: Vec::new(),
        }),
    }
}

pub(crate) fn exchange_search() -> Search {
    Search {
        language: Language::English,
        league: League::new("Standard"),
        query: Query::Exchange("divine".to_owned()),
    }
}

fn offer(id: &str) -> ItemOffer {
    ItemOffer {
        id: id.to_owned(),
        price: Some(Price {
            amount: 1.0,
            currency: "divine".to_owned(),
        }),
        seller: SellerStatus::Online,
        listed: None,
        whisper: String::new(),
        item: OfferItem {
            rarity: None,
            name: None,
            base_type: "Gold Ring".to_owned(),
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

fn exchange_offer() -> ExchangeOffer {
    ExchangeOffer {
        seller: SellerStatus::Online,
        listed: None,
        pay: Price {
            amount: 750.0,
            currency: "exalted".to_owned(),
        },
        get: Price {
            amount: 1.0,
            currency: "divine".to_owned(),
        },
        stock: 3,
        whisper: String::new(),
    }
}

pub(crate) fn after(now: Instant, seconds: u64) -> NextSearch {
    let next = now + Duration::from_secs(seconds);
    NextSearch {
        items: next,
        exchange: next,
    }
}

pub(crate) fn answer(outcome: Outcome, next_search: NextSearch) -> Answer {
    Answer {
        outcome,
        next_search,
    }
}

fn items(ids: &[&str], more: bool) -> Outcome {
    Outcome::Items {
        offers: ids.iter().map(|id| offer(id)).collect(),
        total: 13,
        rest: rest(more),
    }
}

/// The offers left of the search: one, or none.
fn rest(more: bool) -> Rest {
    let left: &[&str] = if more { &["k"] } else { &[] };
    Rest::new(Language::English, "Q1", left)
}

/// The owner's amulet, its quality of a catalyst of the kind `words`.
fn amulet(words: &str) -> Item {
    let text = include_str!("../item/test-items/en-owner-rare-amulet-catalyst.txt")
        .replace("Mana Modifiers", words);
    Item::parse(&crate::item::CopiedItem::from_text(&text).unwrap()).unwrap()
}

/// The search of the item with its quality, from `minimum` to `maximum`.
fn quality_between(minimum: Option<f64>, maximum: Option<f64>) -> Search {
    let mut search = item_search();
    if let Query::Items(query) = &mut search.query {
        query.properties.push(crate::trade::query::PropertyFilter {
            property: Property::Quality,
            minimum,
            maximum,
        });
    }
    search
}

/// The search of the item with its quality, 40 % at least.
fn quality_search() -> Search {
    quality_between(Some(40.0), None)
}

#[test]
fn a_quality_without_limits_isnt_searched() {
    let mana = amulet("Mana Modifiers");
    let now = Instant::now();
    let searched = |search| {
        let mut offers = Offers::default();
        let _ = offers.search(search, now);
        offers.apply(
            answer(items(&["a"], false), after(now, 0)),
            SystemTime::UNIX_EPOCH,
        );
        offers.searched_catalyst(&mana)
    };

    // The trade site gets no filter: it finds the offers without quality too.
    assert_eq!(searched(quality_between(None, None)), None);
    assert_eq!(
        searched(quality_between(None, Some(30.0))),
        Some(Catalyst::Mana)
    );
    assert_eq!(searched(quality_search()), Some(Catalyst::Mana));
}

#[test]
fn the_quality_searched_lasts_with_its_offers_until_the_next_search() {
    let mana = amulet("Mana Modifiers");
    let now = Instant::now();
    let at = SystemTime::UNIX_EPOCH;
    let mut offers = Offers::default();

    let _ = offers.search(quality_search(), now);
    assert_eq!(offers.searched_catalyst(&mana), None, "while searching");
    offers.apply(answer(items(&["a"], true), after(now, 0)), at);
    assert_eq!(offers.searched_catalyst(&mana), Some(Catalyst::Mana));
    assert_eq!(
        offers.searched_catalyst(&amulet("Strange Modifiers")),
        None,
        "a kind the application doesn't know"
    );
    // The next offers, those that couldn't come, and those refused by the limits.
    for outcome in [
        Outcome::More {
            offers: vec![offer("b")],
            rest: rest(true),
        },
        Outcome::Unreachable,
        Outcome::Limited {
            until: now,
            refused: true,
        },
    ] {
        let _ = offers.more();
        offers.apply(answer(outcome.clone(), after(now, 0)), at);
        assert_eq!(
            offers.searched_catalyst(&mana),
            Some(Catalyst::Mana),
            "{outcome:?}"
        );
    }

    let _ = offers.search(item_search(), now);
    offers.apply(answer(items(&["c"], false), after(now, 0)), at);
    assert_eq!(
        offers.searched_catalyst(&mana),
        None,
        "a search without the quality"
    );
    let _ = offers.search(quality_search(), now);
    offers.apply(answer(items(&["d"], false), after(now, 0)), at);
    offers.clear();
    assert_eq!(offers.searched_catalyst(&mana), None, "another item copied");
}

fn ids(offers: &Offers) -> Vec<String> {
    match offers.state() {
        OffersState::Items { offers, .. } => offers.iter().map(|offer| offer.id.clone()).collect(),
        state => panic!("no offers of items: {state:?}"),
    }
}

#[test]
fn a_search_shows_its_offers_then_waits_before_the_next_one() {
    let now = Instant::now();
    let mut offers = Offers::default();

    assert_eq!(offers.wait(SearchKind::Exchange, now), Duration::ZERO);
    assert_eq!(
        offers.search(exchange_search(), now),
        Some(Order::Search(exchange_search()))
    );
    assert_eq!(offers.state(), &OffersState::Searching);
    assert!(offers.busy(), "an answer is awaited");
    assert_eq!(offers.search(exchange_search(), now), None);

    let outcome = Outcome::Exchange {
        offers: vec![exchange_offer(); 14],
        total: 14,
    };
    offers.apply(answer(outcome, after(now, 10)), SystemTime::UNIX_EPOCH);

    assert!(matches!(
        offers.state(),
        OffersState::Exchange {
            shown: 10,
            total: 14,
            ..
        }
    ));
    assert_eq!(offers.last_search(), Some(SystemTime::UNIX_EPOCH));
    assert_eq!(
        offers.wait(SearchKind::Exchange, now),
        Duration::from_secs(10)
    );
    assert_eq!(offers.search(exchange_search(), now), None, "too soon");
    let later = now + Duration::from_secs(10);
    assert_eq!(
        offers.search(exchange_search(), later),
        Some(Order::Search(exchange_search()))
    );
}

#[test]
fn the_offers_of_the_exchange_are_all_known_and_shown_ten_more_at_a_time() {
    let now = Instant::now();
    let mut offers = Offers::default();
    let _ = offers.search(exchange_search(), now);
    let outcome = Outcome::Exchange {
        offers: vec![exchange_offer(); 14],
        total: 48,
    };
    offers.apply(answer(outcome, after(now, 10)), SystemTime::UNIX_EPOCH);

    assert_eq!(offers.more(), None, "no request");
    assert!(matches!(
        offers.state(),
        OffersState::Exchange {
            shown: 14,
            total: 48,
            ..
        }
    ));
}

#[test]
fn more_offers_of_items_are_fetched_and_added_to_the_list() {
    let now = Instant::now();
    let mut offers = Offers::default();
    let _ = offers.search(item_search(), now);
    offers.apply(
        answer(items(&["a", "b"], true), after(now, 10)),
        SystemTime::UNIX_EPOCH,
    );

    assert_eq!(offers.more(), Some(Order::More(rest(true))));
    assert_eq!(offers.more(), None, "once at a time");
    assert!(matches!(
        offers.state(),
        OffersState::Items {
            loading_more: true,
            ..
        }
    ));
    let outcome = Outcome::More {
        offers: vec![offer("c")],
        rest: rest(false),
    };
    offers.apply(answer(outcome, after(now, 10)), SystemTime::now());

    assert_eq!(ids(&offers), ["a", "b", "c"]);
    assert!(matches!(
        offers.state(),
        OffersState::Items { rest, loading_more: false, searched, .. }
            if rest.is_empty() && *searched == SystemTime::UNIX_EPOCH
    ));
    assert!(
        matches!(offers.state(), OffersState::Items { first_page: 2, .. }),
        "the next offers aren't in the answer to the search"
    );
    assert_eq!(offers.more(), None, "none left");
}

#[test]
fn more_offers_that_cant_come_keep_the_list_to_ask_them_again() {
    let now = Instant::now();
    let mut offers = Offers::default();
    let _ = offers.search(item_search(), now);
    offers.apply(
        answer(items(&["a"], true), after(now, 10)),
        SystemTime::UNIX_EPOCH,
    );
    let _ = offers.more();

    offers.apply(
        answer(Outcome::Unreachable, after(now, 10)),
        SystemTime::now(),
    );

    assert_eq!(ids(&offers), ["a"]);
    assert!(matches!(
        offers.state(),
        OffersState::Items {
            more_failed: true,
            ..
        }
    ));
    assert_eq!(offers.more(), Some(Order::More(rest(true))));
    assert!(
        matches!(
            offers.state(),
            OffersState::Items {
                more_failed: false,
                loading_more: true,
                ..
            }
        ),
        "asked again"
    );
}

#[test]
fn the_answer_to_a_previous_item_changes_only_the_waiting_time() {
    let now = Instant::now();
    let mut offers = Offers::default();
    let _ = offers.search(item_search(), now);

    offers.clear();
    assert_eq!(
        offers.search(item_search(), now),
        None,
        "an answer is awaited"
    );
    offers.apply(
        answer(items(&["a"], false), after(now, 10)),
        SystemTime::UNIX_EPOCH,
    );

    assert_eq!(offers.state(), &OffersState::Idle);
    assert_eq!(offers.last_search(), Some(SystemTime::UNIX_EPOCH));
    assert_eq!(
        offers.wait(SearchKind::Items, now),
        Duration::from_secs(10),
        "the waiting time applies to every item"
    );
}

#[test]
fn a_refusal_waits_for_its_delay_and_shows_no_offer() {
    let now = Instant::now();
    let mut offers = Offers::default();
    let _ = offers.search(item_search(), now);
    let until = now + Duration::from_secs(240);
    let just_before = now + Duration::from_secs(239);
    let outcome = Outcome::Limited {
        until,
        refused: true,
    };

    // Limits that would allow a search sooner don't shorten the delay.
    offers.apply(answer(outcome, after(now, 10)), SystemTime::UNIX_EPOCH);

    assert_eq!(offers.state(), &OffersState::Limited);
    assert_eq!(offers.last_search(), None);
    assert_eq!(
        offers.wait(SearchKind::Items, now),
        Duration::from_secs(240)
    );
    assert_eq!(offers.search(item_search(), just_before), None);
}

#[test]
fn an_unreachable_trade_site_is_shown() {
    let now = Instant::now();
    let mut offers = Offers::default();
    let _ = offers.search(item_search(), now);

    offers.apply(
        answer(Outcome::Unreachable, after(now, 10)),
        SystemTime::UNIX_EPOCH,
    );

    assert_eq!(offers.state(), &OffersState::Unreachable);
    assert_eq!(offers.last_search(), None);
}

#[test]
fn more_offers_for_a_previous_item_change_only_the_waiting_time() {
    let now = Instant::now();
    let mut offers = Offers::default();
    let _ = offers.search(item_search(), now);
    offers.apply(
        answer(items(&["a"], true), after(now, 10)),
        SystemTime::UNIX_EPOCH,
    );
    assert_eq!(offers.more(), Some(Order::More(rest(true))));

    offers.clear();
    let outcome = Outcome::More {
        offers: vec![offer("b")],
        rest: rest(false),
    };
    offers.apply(answer(outcome, after(now, 20)), SystemTime::now());

    assert_eq!(offers.state(), &OffersState::Idle);
    assert!(!offers.busy());
    assert_eq!(offers.wait(SearchKind::Items, now), Duration::from_secs(20));
}

#[test]
fn a_refused_search_of_the_exchange_waits_only_before_the_next_one() {
    let now = Instant::now();
    let mut offers = Offers::default();
    let _ = offers.search(exchange_search(), now);
    let outcome = Outcome::Limited {
        until: now + Duration::from_secs(240),
        refused: true,
    };

    offers.apply(answer(outcome, after(now, 10)), SystemTime::UNIX_EPOCH);

    assert_eq!(
        offers.wait(SearchKind::Exchange, now),
        Duration::from_secs(240)
    );
    assert_eq!(offers.wait(SearchKind::Items, now), Duration::from_secs(10));
}

#[test]
fn refused_more_offers_wait_as_the_limits_of_the_fetch_tell() {
    let now = Instant::now();
    let mut offers = Offers::default();
    let _ = offers.search(item_search(), now);
    offers.apply(
        answer(items(&["a"], true), after(now, 10)),
        SystemTime::UNIX_EPOCH,
    );
    let _ = offers.more();
    let outcome = Outcome::Limited {
        until: now + Duration::from_secs(240),
        refused: true,
    };

    // The limits of the fetch are already in the waiting time of the searches of items.
    offers.apply(answer(outcome, after(now, 60)), SystemTime::now());

    assert_eq!(ids(&offers), ["a"]);
    assert!(
        matches!(
            offers.state(),
            OffersState::Items {
                more_failed: false,
                ..
            }
        ),
        "the waiting block tells the refusal"
    );
    assert_eq!(offers.wait(SearchKind::Items, now), Duration::from_secs(60));
}

#[test]
fn no_answer_shortens_a_wait_told_before() {
    let now = Instant::now();
    let mut offers = Offers::default();
    let _ = offers.search(item_search(), now);
    let outcome = Outcome::Limited {
        until: now + Duration::from_secs(240),
        refused: true,
    };
    offers.apply(answer(outcome, after(now, 10)), SystemTime::UNIX_EPOCH);

    // An answer without an order, and limits that would allow a search sooner.
    offers.apply(
        answer(items(&["a"], false), after(now, 10)),
        SystemTime::now(),
    );

    assert_eq!(
        offers.wait(SearchKind::Items, now),
        Duration::from_secs(240)
    );
}

#[test]
fn a_limit_answered_without_an_order_applies_to_both_kinds_of_searches() {
    let now = Instant::now();
    let mut offers = Offers::default();
    let outcome = Outcome::Limited {
        until: now + Duration::from_secs(240),
        refused: true,
    };

    offers.apply(answer(outcome, after(now, 10)), SystemTime::UNIX_EPOCH);

    assert_eq!(offers.state(), &OffersState::Idle);
    assert_eq!(
        offers.wait(SearchKind::Items, now),
        Duration::from_secs(240)
    );
    assert_eq!(
        offers.wait(SearchKind::Exchange, now),
        Duration::from_secs(240)
    );
}

#[test]
fn a_lost_thread_of_the_searches_shows_the_trade_site_unreachable_for_good() {
    let now = Instant::now();
    let mut offers = Offers::default();
    let _ = offers.search(item_search(), now);

    offers.thread_lost();

    assert_eq!(offers.state(), &OffersState::Unreachable);
    assert!(offers.busy());
    let later = now + Duration::from_mins(10);
    assert_eq!(offers.search(item_search(), later), None);
}

#[test]
fn a_lost_thread_asks_no_more_offers() {
    let now = Instant::now();
    let mut offers = Offers::default();
    let _ = offers.search(item_search(), now);
    offers.apply(
        answer(items(&["a"], true), after(now, 10)),
        SystemTime::UNIX_EPOCH,
    );

    offers.thread_lost();

    assert_eq!(offers.more(), None);
}
