//! The searches of offers on the trade site, made on their own thread when the player asks:
//! a search of items and the fetch of its first 10 offers, or a search of the currency
//! exchange; then 10 more offers of the last search of items, when asked. Nothing runs in
//! the background, and nothing is tried again by itself.

use std::thread;
use std::time::{Duration, Instant};

use super::client::{FETCH_SIZE, TradeClient, TradeError, Transport};
use super::offers::{ExchangeOffer, ItemOffer};
use super::query::{Query, SearchKind};
use crate::language::Language;
use crate::league::League;
use crate::worker::{OnDrop, Outbox, Waited, Worker};

/// The longest wait for the limits of a fetch asked by the player: a fetch that must wait
/// longer fails, rather than leaving the player without an answer.
const LONGEST_FETCH_WAIT: Duration = Duration::from_secs(5);

/// What the player asks.
#[derive(Debug, Clone, PartialEq)]
pub enum Order {
    /// A search, in `league`, on the trade site of `language`.
    Search {
        language: Language,
        league: League,
        query: Query,
    },
    /// 10 more offers of the last search of items.
    More,
}

/// The answer to an order.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// The first offers of a search of items, the cheapest first.
    Items {
        offers: Vec<ItemOffer>,
        total: u64,
        /// Some offers are left to fetch.
        more: bool,
    },
    /// The offers of the currency exchange, the cheapest first, all of them.
    Exchange {
        offers: Vec<ExchangeOffer>,
        total: u64,
    },
    /// The next offers of the last search of items.
    More { offers: Vec<ItemOffer>, more: bool },
    /// The trade site can't be reached, or answered something unexpected.
    Unreachable,
    /// The trade site refused the request (`refused`), or its limits didn't allow it: nothing
    /// is sent before `until`.
    Limited { until: Instant, refused: bool },
}

/// When the next search may be sent, for each kind of search: their limits differ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NextSearch {
    pub items: Instant,
    pub exchange: Instant,
}

impl NextSearch {
    /// When a search of `kind` may be sent.
    #[must_use]
    pub fn of(&self, kind: SearchKind) -> Instant {
        match kind {
            SearchKind::Items => self.items,
            SearchKind::Exchange => self.exchange,
        }
    }
}

/// An outcome, and the waiting time it leaves.
#[derive(Debug, Clone, PartialEq)]
pub struct Answer {
    pub outcome: Outcome,
    pub next_search: NextSearch,
}

/// The searches of offers, on their own thread for as long as this value lives.
pub struct OfferSearch {
    worker: Worker<Answer, Order>,
}

impl OfferSearch {
    /// Starts the thread; `wake` is called from it after each answer.
    ///
    /// # Panics
    ///
    /// If the system can't start a thread.
    #[must_use]
    pub fn start(wake: impl Fn() + Send + 'static) -> Self {
        Self::spawn(TradeClient::new, wake)
    }

    fn spawn<T: Transport + 'static>(
        open: impl FnOnce() -> TradeClient<T> + Send + 'static,
        wake: impl Fn() + Send + 'static,
    ) -> Self {
        let worker = Worker::spawn("trade-search", OnDrop::Detach, wake, move |outbox, wake| {
            let mut searches = Searches::new(open());
            loop {
                let order = match outbox.wait_for_order(Duration::MAX) {
                    Waited::Order(order) => order,
                    Waited::Elapsed => continue,
                    Waited::Stop => return,
                };
                let Some(order) = latest(order, outbox) else {
                    return;
                };
                if !outbox.send(searches.answer(order)) {
                    return;
                }
                wake();
            }
        });
        Self { worker }
    }

    /// Asks `order`; its answer comes through [`Self::receive`]. The orders given while the
    /// thread is busy are answered once, by the last search among them (see [`latest`]).
    pub fn order(&self, order: Order) {
        self.worker.order(order);
    }

    /// Gives the answers received since the last call to `handle`, without waiting.
    pub fn receive(&mut self, handle: impl FnMut(Answer)) {
        self.worker.receive(handle);
    }
}

/// The order to answer among `first` and the orders given meanwhile: the last search, since
/// the player only sees its offers, or else one request of more offers; `None` when the
/// application asks the thread to stop.
fn latest(first: Order, outbox: &Outbox<Answer, Order>) -> Option<Order> {
    let mut latest = first;
    while let Some(waited) = outbox.pending_order() {
        match waited {
            Waited::Order(order @ Order::Search { .. }) => latest = order,
            // More offers of a search answered before, or asked twice.
            Waited::Order(Order::More) | Waited::Elapsed => {}
            Waited::Stop => return None,
        }
    }
    Some(latest)
}

/// The offers of the last search of items left to fetch.
struct Remaining {
    language: Language,
    search: String,
    offers: Vec<String>,
}

/// The client and the last search of items.
struct Searches<T> {
    client: TradeClient<T>,
    remaining: Option<Remaining>,
}

impl<T: Transport> Searches<T> {
    fn new(client: TradeClient<T>) -> Self {
        Self {
            client,
            remaining: None,
        }
    }

    fn answer(&mut self, order: Order) -> Answer {
        let outcome = self.outcome(order).unwrap_or_else(|error| {
            log::warn!("Trade site: {error}");
            match error {
                TradeError::Wait(until) => Outcome::Limited {
                    until,
                    refused: false,
                },
                TradeError::Refused(until) => Outcome::Limited {
                    until,
                    refused: true,
                },
                TradeError::Http(_) => Outcome::Unreachable,
            }
        });
        let now = Instant::now();
        let next_search = NextSearch {
            items: self.client.next_search(SearchKind::Items, now),
            exchange: self.client.next_search(SearchKind::Exchange, now),
        };
        Answer {
            outcome,
            next_search,
        }
    }

    fn outcome(&mut self, order: Order) -> Result<Outcome, TradeError> {
        match order {
            Order::Search {
                language,
                league,
                query: Query::Items(query),
            } => {
                let result = self.client.search_items(language, &league, &query);
                let search = self.forget_unless_unsent(result)?;
                log::info!("Trade site: {} offers found", search.total);
                let total = search.total;
                self.remaining = Some(Remaining {
                    language,
                    search: search.id,
                    offers: search.offers,
                });
                // The player never sees a search whose first offers can't be fetched: its next
                // offers aren't more of the offers shown.
                let (offers, more) = self.fetch_next().inspect_err(|_| self.remaining = None)?;
                Ok(Outcome::Items {
                    offers,
                    total,
                    more,
                })
            }
            Order::Search {
                language,
                league,
                query: Query::Exchange(want),
            } => {
                let result = self.client.search_exchange(language, &league, &want);
                let search = self.forget_unless_unsent(result)?;
                log::info!("Trade site: {} exchange offers of {want}", search.total);
                Ok(Outcome::Exchange {
                    offers: search.offers,
                    total: search.total,
                })
            }
            Order::More => {
                let (offers, more) = self.fetch_next()?;
                Ok(Outcome::More { offers, more })
            }
        }
    }

    /// Forgets the last search of items once a new search was sent, whatever its answer: the
    /// player no longer sees its offers. A search that wasn't sent keeps it.
    fn forget_unless_unsent<S>(&mut self, result: Result<S, TradeError>) -> Result<S, TradeError> {
        if !matches!(result, Err(TradeError::Wait(_))) {
            self.remaining = None;
        }
        result
    }

    /// The next 10 offers of the last search of items, and whether some are left; they
    /// stay to fetch when the fetch fails.
    fn fetch_next(&mut self) -> Result<(Vec<ItemOffer>, bool), TradeError> {
        let Some(remaining) = &mut self.remaining else {
            return Ok((Vec::new(), false));
        };
        let wait = self
            .client
            .next_fetch(Instant::now())
            .saturating_duration_since(Instant::now());
        if wait > LONGEST_FETCH_WAIT {
            return Err(TradeError::Wait(Instant::now() + wait));
        }
        thread::sleep(wait);
        let count = remaining.offers.len().min(FETCH_SIZE);
        let ids = remaining.offers.get(..count).unwrap_or_default();
        let offers = self
            .client
            .fetch(remaining.language, &remaining.search, ids)?;
        remaining.offers.drain(..count);
        Ok((offers, !remaining.offers.is_empty()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trade::client::tests::{FakeSite, SEARCH, limits, ring};
    use crate::trade::limits::LimitHeaders;
    use std::sync::mpsc::{self, Receiver};

    /// One offer of `/fetch`, shortened.
    fn listed(id: &str) -> String {
        format!(
            r#"{{"id": "{id}", "listing": {{"price": {{"amount": 1, "currency": "divine"}},
            "account": {{"online": {{}}}}, "whisper": "@seller Hi"}},
            "item": {{"name": "Andvarius", "typeLine": "Gold Ring"}}}}"#
        )
    }

    fn fetch(ids: &[&str]) -> String {
        let listed: Vec<String> = ids.iter().map(|id| listed(id)).collect();
        format!(r#"{{"result": [{}]}}"#, listed.join(","))
    }

    fn start(site: &FakeSite) -> (OfferSearch, Receiver<()>) {
        let (woken, wakes) = mpsc::channel();
        let site = site.clone();
        let search = OfferSearch::spawn(
            move || TradeClient::with_transport(site),
            move || {
                let _ = woken.send(());
            },
        );
        (search, wakes)
    }

    fn answer(search: &mut OfferSearch, wakes: &Receiver<()>) -> Answer {
        wakes
            .recv_timeout(Duration::from_secs(5))
            .expect("the application is woken up");
        let mut answers = Vec::new();
        search.receive(|answer| answers.push(answer));
        answers.pop().expect("an answer")
    }

    fn search_ring() -> Order {
        Order::Search {
            language: Language::English,
            league: League::new("Standard"),
            query: Query::Items(ring()),
        }
    }

    fn ids(offers: &[ItemOffer]) -> Vec<&str> {
        offers.iter().map(|offer| offer.id.as_str()).collect()
    }

    #[test]
    fn a_search_gives_its_first_ten_offers_then_ten_more_when_asked() {
        let site = FakeSite::default();
        site.answer(200, LimitHeaders::default(), SEARCH);
        site.answer(
            200,
            LimitHeaders::default(),
            &fetch(&["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"]),
        );
        site.answer(200, LimitHeaders::default(), &fetch(&["k", "l", "m"]));
        let (mut search, wakes) = start(&site);

        search.order(search_ring());
        let first = answer(&mut search, &wakes);
        search.order(Order::More);
        let second = answer(&mut search, &wakes);

        let Outcome::Items {
            offers,
            total,
            more,
        } = first.outcome
        else {
            panic!("offers: {:?}", first.outcome);
        };
        assert_eq!(ids(&offers).len(), 10);
        assert_eq!(total, 13);
        assert!(more);
        assert!(first.next_search.items > Instant::now());
        assert_eq!(
            second.outcome,
            Outcome::More {
                offers: second_offers(&second),
                more: false
            }
        );
        assert_eq!(ids(&second_offers(&second)), ["k", "l", "m"]);
        assert!(site.requests()[2].url.contains("/fetch/k,l,m?query=Q1"));
    }

    fn second_offers(answer: &Answer) -> Vec<ItemOffer> {
        match &answer.outcome {
            Outcome::More { offers, .. } => offers.clone(),
            other => panic!("more offers: {other:?}"),
        }
    }

    #[test]
    fn a_trade_site_that_cant_be_reached_is_told() {
        let site = FakeSite::default();
        let (mut search, wakes) = start(&site);

        search.order(search_ring());

        assert_eq!(answer(&mut search, &wakes).outcome, Outcome::Unreachable);
    }

    #[test]
    fn a_refused_search_tells_when_to_search_again() {
        let site = FakeSite::default();
        let mut headers = limits("trade-search-request-limit", "5:10:60", "6:10:60");
        headers.retry_after = Some("60".to_owned());
        site.answer(429, headers, "");
        let (mut search, wakes) = start(&site);

        search.order(search_ring());
        let refused = answer(&mut search, &wakes);

        let Outcome::Limited {
            until,
            refused: true,
        } = refused.outcome
        else {
            panic!("refused: {:?}", refused.outcome);
        };
        assert!(until > Instant::now() + Duration::from_secs(55));
        assert!(refused.next_search.items >= until);
        // The exchange has its own limits: only the 10 s between two searches.
        assert!(refused.next_search.exchange < Instant::now() + Duration::from_secs(11));
    }

    #[test]
    fn more_offers_without_a_search_of_items_are_none() {
        let site = FakeSite::default();
        let (mut search, wakes) = start(&site);

        search.order(Order::More);

        assert_eq!(
            answer(&mut search, &wakes).outcome,
            Outcome::More {
                offers: Vec::new(),
                more: false
            }
        );
        assert_eq!(site.requests(), Vec::new());
    }

    #[test]
    fn the_orders_given_during_a_search_leave_only_the_last_search() {
        let site = FakeSite::default();
        site.answer(200, LimitHeaders::default(), SEARCH);
        site.answer(200, LimitHeaders::default(), &fetch(&["a"]));
        let (mut search, wakes) = start(&site);
        // The first search is blocked in its request while the player clicks.
        let blocked = site.answers.lock().unwrap();
        search.order(search_ring());
        let start = Instant::now();
        while site.requests().is_empty() {
            assert!(start.elapsed() < Duration::from_secs(5), "sent");
            thread::sleep(Duration::from_millis(5));
        }
        search.order(Order::More);
        search.order(Order::Search {
            language: Language::English,
            league: League::new("Standard"),
            query: Query::Exchange("divine".to_owned()),
        });
        search.order(Order::More);
        drop(blocked);

        let mut answers = Vec::new();
        while answers.len() < 2 {
            wakes
                .recv_timeout(Duration::from_secs(5))
                .expect("the application is woken up");
            search.receive(|answer| answers.push(answer.outcome));
        }

        assert!(matches!(answers[0], Outcome::Items { .. }), "{answers:?}");
        // Only the last search is answered, without the more offers asked around it: not
        // sent, 10 s after the first one.
        assert!(
            matches!(answers[1], Outcome::Limited { refused: false, .. }),
            "{answers:?}"
        );
        assert_eq!(answers.len(), 2);
        assert_eq!(site.requests().len(), 2, "a search and its fetch");
    }

    #[test]
    fn a_search_that_wasnt_sent_keeps_the_offers_left_of_the_last_one() {
        let site = FakeSite::default();
        site.answer(200, LimitHeaders::default(), SEARCH);
        site.answer(200, LimitHeaders::default(), &fetch(&["a"]));
        site.answer(200, LimitHeaders::default(), &fetch(&["k"]));
        let (mut search, wakes) = start(&site);
        search.order(search_ring());
        answer(&mut search, &wakes);

        // Within the 10 s: not sent.
        search.order(search_ring());
        answer(&mut search, &wakes);
        search.order(Order::More);

        assert_eq!(ids(&second_offers(&answer(&mut search, &wakes))), ["k"]);
    }

    #[test]
    fn a_search_whose_first_offers_cant_be_fetched_is_forgotten() {
        let site = FakeSite::default();
        site.answer(200, LimitHeaders::default(), SEARCH);
        site.answer(503, LimitHeaders::default(), "");
        let (mut search, wakes) = start(&site);
        search.order(search_ring());
        assert_eq!(answer(&mut search, &wakes).outcome, Outcome::Unreachable);

        search.order(Order::More);

        assert_eq!(
            answer(&mut search, &wakes).outcome,
            Outcome::More {
                offers: Vec::new(),
                more: false
            }
        );
        assert_eq!(site.requests().len(), 2, "a search and its fetch");
    }
}
