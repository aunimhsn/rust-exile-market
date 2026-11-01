//! The offers of the trade site for the item shown: the searches the player asks, their
//! answers, and the waiting time before the next search, which applies to every item.
//!
//! One order at a time: no order is sent while the answer of the previous one hasn't come,
//! so that each answer is known to answer the last order, and to belong to the item shown or
//! to an item copied before it.

use std::time::{Duration, Instant, SystemTime};

use crate::filters::Property;
use crate::item::{Catalyst, Item};
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
        /// How many offers of `offers`, the first ones, came in the answer to the search:
        /// fewer than a page when some were gone since the search.
        first_page: usize,
        /// Some offers are left to fetch.
        more: bool,
        /// The next offers are asked.
        loading_more: bool,
        /// The next offers couldn't come: the trade site can't be reached. Refused by its
        /// limits, the waiting block tells it already.
        more_failed: bool,
        /// The search filtered the quality, which the trade site searches of any kind: the
        /// offers of a catalyst of another kind than the item's aren't counted.
        quality_searched: bool,
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
    Search {
        kind: SearchKind,
        /// The search filters the quality.
        quality_searched: bool,
    },
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

    /// The kind of the quality of a catalyst of `item`, the item shown, when its offers shown
    /// were searched with its quality: the offers of another kind aren't counted.
    #[must_use]
    pub fn searched_catalyst(&self, item: &Item) -> Option<Catalyst> {
        match self.state {
            OffersState::Items {
                quality_searched: true,
                ..
            } => item.quality_kind()?.catalyst,
            _ => None,
        }
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
            order: PendingOrder::Search {
                kind,
                quality_searched: search.query.searches(Property::Quality),
            },
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

    /// Takes in the waiting time told by `answer` to the `pending` order: no answer shortens
    /// a wait told before.
    fn wait_after(&mut self, answer: &Answer, pending: Option<Pending>) {
        let mut next_search = answer.next_search;
        if let Outcome::Limited { until, .. } = &answer.outcome {
            // Nothing is sent before the end of the wait, whatever the limits known: for the
            // kind of the search refused, for both when the order is unknown. The limits of the
            // fetch of more offers are already in those of the searches of items.
            let (items, exchange) = match pending.map(|pending| pending.order) {
                Some(PendingOrder::Search {
                    kind: SearchKind::Items,
                    ..
                }) => (true, false),
                Some(PendingOrder::Search {
                    kind: SearchKind::Exchange,
                    ..
                }) => (false, true),
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
    }

    /// Takes in the answer of the last order, received at `now`.
    pub fn apply(&mut self, answer: Answer, now: SystemTime) {
        let pending = self.pending.take();
        // The waiting time first, whatever the order: it keeps the account of the player safe.
        self.wait_after(&answer, pending);
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
                first_page: offers.len(),
                offers,
                total,
                more,
                loading_more: false,
                more_failed: false,
                quality_searched: matches!(
                    pending.order,
                    PendingOrder::Search {
                        quality_searched: true,
                        ..
                    }
                ),
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
                    first_page,
                    quality_searched,
                    searched,
                    ..
                },
            ) => {
                offers.extend(next);
                OffersState::Items {
                    offers,
                    total,
                    first_page,
                    more,
                    loading_more: false,
                    more_failed: false,
                    quality_searched,
                    searched,
                }
            }
            // More offers that couldn't come: the offers shown stay, to ask them again.
            (
                failure @ (Outcome::Unreachable | Outcome::Limited { .. }),
                OffersState::Items {
                    offers,
                    total,
                    first_page,
                    more,
                    quality_searched,
                    searched,
                    ..
                },
            ) if pending.order == PendingOrder::More => OffersState::Items {
                offers,
                total,
                first_page,
                more,
                loading_more: false,
                more_failed: matches!(failure, Outcome::Unreachable),
                quality_searched,
                searched,
            },
            (Outcome::Unreachable, _) => OffersState::Unreachable,
            (Outcome::Limited { .. }, _) => OffersState::Limited,
            (Outcome::More { .. }, state) => state,
        };
    }
}

#[cfg(test)]
mod tests;
