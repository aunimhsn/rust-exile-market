//! The offers of the trade site for the item shown: the searches the player asks, their
//! answers, and the waiting time before the next search, which applies to every item.
//!
//! One order at a time: no order is sent while the answer of the previous one hasn't come,
//! so that each answer is known to answer the last order, and to belong to the item shown or
//! to an item copied before it.

use std::time::{Duration, Instant, SystemTime};

use crate::trade::offers::{ExchangeOffer, ItemOffer};
use crate::trade::query::SearchKind;
use crate::trade::search::{Answer, NextSearch, Order, Outcome, Search};

/// The offers shown at once, and added by each click on the double chevron.
pub const PAGE: usize = 10;
/// A wait before the next search longer than this is told: the limits of the trade site
/// require it, beyond the usual 10 seconds.
pub const LONG_WAIT: Duration = Duration::from_mins(1);

/// What the offers block shows for the item shown.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum OffersState {
    /// No search for this item yet.
    #[default]
    Idle,
    Searching,
    /// The offers of a search of items, the cheapest first.
    Items {
        offers: Vec<ItemOffer>,
        total: u64,
        /// Some offers are left to fetch.
        more: bool,
        /// The next offers are asked.
        loading_more: bool,
        /// The next offers couldn't come: the trade site can't be reached. Refused by its
        /// limits, the waiting block tells it already.
        more_failed: bool,
        searched: SystemTime,
    },
    /// The offers of the currency exchange, all of them, the first `shown` shown.
    Exchange {
        offers: Vec<ExchangeOffer>,
        total: u64,
        shown: usize,
        searched: SystemTime,
    },
    /// The trade site can't be reached, or answered something unexpected.
    Unreachable,
    /// The trade site refused the search, or its limits didn't allow it.
    Limited,
}

/// The kinds of orders.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PendingOrder {
    Search(SearchKind),
    More,
}

/// The order waiting for its answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Pending {
    order: PendingOrder,
    /// Asked for the item shown: no item was copied since.
    shown: bool,
}

/// The offers of the item shown, and the searches.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Offers {
    state: OffersState,
    pending: Option<Pending>,
    /// When the next search of each kind may be sent; any time before the first answer.
    next_search: Option<NextSearch>,
    /// When the last search was answered with its offers.
    last_search: Option<SystemTime>,
    /// The thread of the searches is gone: no answer will come, and no order is sent.
    lost: bool,
}

impl Offers {
    /// What the offers block shows.
    #[must_use]
    pub fn state(&self) -> &OffersState {
        &self.state
    }

    /// When the last search was answered with its offers, for any item.
    #[must_use]
    pub fn last_search(&self) -> Option<SystemTime> {
        self.last_search
    }

    /// Another item is shown: no offer yet, and the answer of an order of the previous item
    /// will change nothing.
    pub fn clear(&mut self) {
        self.state = OffersState::Idle;
        if let Some(pending) = &mut self.pending {
            pending.shown = false;
        }
    }

    /// No order may be sent: the answer of the last one hasn't come, or the thread of the
    /// searches is gone.
    #[must_use]
    pub fn busy(&self) -> bool {
        self.pending.is_some() || self.lost
    }

    /// How long to wait at `now` before the limits allow a search of `kind`, once the
    /// offers are no longer [busy](Self::busy).
    #[must_use]
    pub fn wait(&self, kind: SearchKind, now: Instant) -> Duration {
        self.next_search.map_or(Duration::ZERO, |next| {
            next.of(kind).saturating_duration_since(now)
        })
    }

    /// Asks `search` at `now`: the order to send, or `None` when it isn't allowed yet.
    #[must_use]
    pub fn search(&mut self, search: Search, now: Instant) -> Option<Order> {
        let kind = search.query.kind();
        if self.busy() || !self.wait(kind, now).is_zero() {
            return None;
        }
        self.pending = Some(Pending {
            order: PendingOrder::Search(kind),
            shown: true,
        });
        self.state = OffersState::Searching;
        Some(Order::Search(search))
    }

    /// Shows 10 more offers: those of the currency exchange are all known, those of a search
    /// of items are fetched with the order returned.
    #[must_use]
    pub fn more(&mut self) -> Option<Order> {
        let busy = self.busy();
        match &mut self.state {
            OffersState::Exchange { offers, shown, .. } => {
                *shown = (*shown + PAGE).min(offers.len());
                None
            }
            OffersState::Items {
                more: true,
                loading_more,
                more_failed,
                ..
            } if !busy => {
                *loading_more = true;
                *more_failed = false;
                self.pending = Some(Pending {
                    order: PendingOrder::More,
                    shown: true,
                });
                Some(Order::More)
            }
            _ => None,
        }
    }

    /// The thread of the searches is gone: the order waiting for its answer won't get one,
    /// and no order is sent any more.
    pub fn thread_lost(&mut self) {
        self.lost = true;
        if self.pending.take().is_some_and(|pending| pending.shown) {
            self.state = OffersState::Unreachable;
        }
    }

    /// Takes in the answer of the last order, received at `now`.
    pub fn apply(&mut self, answer: Answer, now: SystemTime) {
        let pending = self.pending.take();
        // The waiting time first, whatever the order: it keeps the account of the player safe.
        let mut next_search = answer.next_search;
        if let Outcome::Limited { until, .. } = &answer.outcome {
            // Nothing is sent before the end of the wait, whatever the limits known: for the
            // kind of the search refused, for both when the order is unknown. The limits of the
            // fetch of more offers are already in those of the searches of items.
            let (items, exchange) = match pending.map(|pending| pending.order) {
                Some(PendingOrder::Search(SearchKind::Items)) => (true, false),
                Some(PendingOrder::Search(SearchKind::Exchange)) => (false, true),
                Some(PendingOrder::More) => (false, false),
                None => (true, true),
            };
            if items {
                next_search.items = next_search.items.max(*until);
            }
            if exchange {
                next_search.exchange = next_search.exchange.max(*until);
            }
        }
        // No answer shortens a wait told before.
        self.next_search = Some(match self.next_search {
            Some(known) => NextSearch {
                items: known.items.max(next_search.items),
                exchange: known.exchange.max(next_search.exchange),
            },
            None => next_search,
        });
        if matches!(
            answer.outcome,
            Outcome::Items { .. } | Outcome::Exchange { .. }
        ) {
            self.last_search = Some(now);
        }
        let Some(pending) = pending else {
            log::warn!("Trade site: an answer without an order");
            return;
        };
        if !pending.shown {
            return;
        }
        let state = std::mem::replace(&mut self.state, OffersState::Idle);
        self.state = match (answer.outcome, state) {
            (
                Outcome::Items {
                    offers,
                    total,
                    more,
                },
                _,
            ) => OffersState::Items {
                offers,
                total,
                more,
                loading_more: false,
                more_failed: false,
                searched: now,
            },
            (Outcome::Exchange { offers, total }, _) => OffersState::Exchange {
                shown: offers.len().min(PAGE),
                offers,
                total,
                searched: now,
            },
            (
                Outcome::More { offers: next, more },
                OffersState::Items {
                    mut offers,
                    total,
                    searched,
                    ..
                },
            ) => {
                offers.extend(next);
                OffersState::Items {
                    offers,
                    total,
                    more,
                    loading_more: false,
                    more_failed: false,
                    searched,
                }
            }
            // More offers that couldn't come: the offers shown stay, to ask them again.
            (
                failure @ (Outcome::Unreachable | Outcome::Limited { .. }),
                OffersState::Items {
                    offers,
                    total,
                    more,
                    searched,
                    ..
                },
            ) if pending.order == PendingOrder::More => OffersState::Items {
                offers,
                total,
                more,
                loading_more: false,
                more_failed: matches!(failure, Outcome::Unreachable),
                searched,
            },
            (Outcome::Unreachable, _) => OffersState::Unreachable,
            (Outcome::Limited { .. }, _) => OffersState::Limited,
            (Outcome::More { .. }, state) => state,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::language::Language;
    use crate::league::League;
    use crate::trade::offers::{OfferItem, Price, SellerStatus};
    use crate::trade::query::{ItemQuery, Query};

    fn item_search() -> Search {
        Search {
            language: Language::English,
            league: League::new("Standard"),
            query: Query::Items(ItemQuery {
                name: Some("Andvarius".to_owned()),
                base_type: "Gold Ring".to_owned(),
                rarity: None,
                corrupted: Some(false),
                stats: Vec::new(),
            }),
        }
    }

    fn exchange_search() -> Search {
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

    fn after(now: Instant, seconds: u64) -> NextSearch {
        let next = now + Duration::from_secs(seconds);
        NextSearch {
            items: next,
            exchange: next,
        }
    }

    fn answer(outcome: Outcome, next_search: NextSearch) -> Answer {
        Answer {
            outcome,
            next_search,
        }
    }

    fn items(ids: &[&str], more: bool) -> Outcome {
        Outcome::Items {
            offers: ids.iter().map(|id| offer(id)).collect(),
            total: 13,
            more,
        }
    }

    fn ids(offers: &Offers) -> Vec<String> {
        match offers.state() {
            OffersState::Items { offers, .. } => {
                offers.iter().map(|offer| offer.id.clone()).collect()
            }
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

        assert_eq!(offers.more(), Some(Order::More));
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
            more: false,
        };
        offers.apply(answer(outcome, after(now, 10)), SystemTime::now());

        assert_eq!(ids(&offers), ["a", "b", "c"]);
        assert!(matches!(
            offers.state(),
            OffersState::Items { more: false, loading_more: false, searched, .. }
                if *searched == SystemTime::UNIX_EPOCH
        ));
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
        assert_eq!(offers.more(), Some(Order::More));
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
        assert_eq!(offers.more(), Some(Order::More));

        offers.clear();
        let outcome = Outcome::More {
            offers: vec![offer("b")],
            more: false,
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
}
