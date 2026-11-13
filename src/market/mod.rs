//! The price database, read on its own thread: the leagues, and the prices of the league
//! shown.
//!
//! The thread asks for the leagues at startup, then every hour. It gives the prices of the
//! league the application asks for: from the cache at once, then from the database when the
//! cache is older than an hour, and every hour after. Without a cache, the prices arrive
//! category by category. When the database can't be reached, it tries again every minute,
//! with one request: a download waits while the leagues can't be read, and stops at its first
//! failed request. When the database answers with an error, or with something the application
//! can't read, the next attempt waits longer and longer, an hour at most: a download fails
//! the same way each time then, and may have sent twenty requests before.

mod cache;
mod poe2scout;

use std::time::{Duration, Instant, SystemTime};

pub use poe2scout::FetchError;
use poe2scout::Poe2Scout;

use crate::http::Backoff;
use crate::league::{League, Leagues};
use crate::logging;
use crate::price::PriceList;
use crate::worker::{OnDrop, Outbox, Waited, Worker};
use cache::PriceCache;

/// When the thread reads the database: every `refresh`, and after a failure, `retry` when
/// nothing answered, then doubled at each failure the database answered, `longest_retry` at
/// most.
#[derive(Debug, Clone, Copy)]
struct Intervals {
    refresh: Duration,
    retry: Duration,
    longest_retry: Duration,
}

const INTERVALS: Intervals = Intervals {
    refresh: Duration::from_hours(1),
    retry: Duration::from_mins(1),
    longest_retry: Duration::from_hours(1),
};

impl Intervals {
    fn backoff(self) -> Backoff {
        Backoff::new(self.retry, self.longest_retry)
    }

    /// The wait before the next attempt after `error`.
    fn retry_after(self, error: &FetchError, backoff: &mut Backoff) -> Duration {
        if error.answered() {
            backoff.failed()
        } else {
            self.retry
        }
    }
}

/// The waits after the failures the database answered, for the leagues and for the prices of
/// the league shown, each on its own: the leagues read every hour must not start over the
/// wait of a download of prices that keeps failing.
#[derive(Debug, Clone, Copy)]
struct Retries {
    intervals: Intervals,
    leagues: Backoff,
    prices: Backoff,
}

impl Retries {
    fn new(intervals: Intervals) -> Self {
        Self {
            intervals,
            leagues: intervals.backoff(),
            prices: intervals.backoff(),
        }
    }

    fn leagues_read(&mut self) {
        self.leagues.succeeded();
    }

    /// The wait before the leagues are asked again.
    fn leagues_failed(&mut self, error: &FetchError) -> Duration {
        self.intervals.retry_after(error, &mut self.leagues)
    }

    fn prices_read(&mut self) {
        self.prices.succeeded();
    }

    /// Another league may not fail as the previous one did, and the player asked for it.
    fn league_changed(&mut self) {
        self.prices.succeeded();
    }

    /// The wait before the prices are downloaded again.
    fn prices_failed(&mut self, error: &FetchError) -> Duration {
        self.intervals.retry_after(error, &mut self.prices)
    }
}

/// What the price database tells the application.
#[derive(Debug, Clone)]
pub enum MarketEvent {
    Leagues(Leagues),
    /// The prices downloaded last, from the cache, until the database gives new ones.
    CachedPrices(PriceList),
    /// The prices downloaded so far of a league none of whose prices are known, `progress` of
    /// the download done (from 0 to 1).
    PartialPrices {
        prices: PriceList,
        progress: f32,
    },
    /// The prices the database just gave.
    Prices(PriceList),
    /// The database couldn't be read; the application keeps what it has.
    Unreachable(Source),
}

/// What was read from the database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    Leagues,
    /// The prices of this league: a failure says nothing of the other leagues.
    Prices(League),
}

/// What a download of prices tells the thread, between two requests.
trait DownloadObserver {
    /// The application asked for something else: the download stops.
    fn interrupted(&mut self) -> bool;

    /// The prices downloaded so far, `progress` of the download done (from 0 to 1).
    fn progress(&mut self, prices: &PriceList, progress: f32);
}

/// Where the thread reads the market: poe2scout, or a fake one in the tests.
trait PriceDatabase {
    /// The current leagues, in the order of the database.
    fn leagues(&mut self) -> Result<Vec<League>, FetchError>;

    /// The prices of `league`; `None` when `download` is interrupted.
    fn prices(
        &mut self,
        league: &League,
        download: &mut dyn DownloadObserver,
    ) -> Result<Option<PriceList>, FetchError>;
}

/// The price database read on its own thread, for as long as this value lives.
pub struct MarketWatch {
    /// Its orders are the league to price.
    worker: Worker<MarketEvent, League>,
    /// The league the thread was told to price.
    league: Option<League>,
}

impl MarketWatch {
    /// Starts reading poe2scout, with the prices of `league` if it is known; `wake` is called
    /// from the thread after each event.
    ///
    /// # Panics
    ///
    /// If the system can't start a thread.
    #[must_use]
    pub fn start(league: Option<League>, wake: impl Fn() + Send + 'static) -> Self {
        let open = || (Poe2Scout::new(), PriceCache::in_cache_directory());
        Self::spawn(open, INTERVALS, league, wake)
    }

    /// Starts reading the database and the cache that `open` gives, on the thread.
    fn spawn<D: PriceDatabase>(
        open: impl FnOnce() -> (D, PriceCache) + Send + 'static,
        intervals: Intervals,
        league: Option<League>,
        wake: impl Fn() + Send + 'static,
    ) -> Self {
        let first = league.clone();
        let worker = Worker::spawn("market", OnDrop::Detach, wake, move |outbox, wake| {
            let (database, cache) = open();
            let watch = Watch {
                database,
                cache,
                outbox,
                wake,
                intervals,
                leagues_due: Instant::now(),
                pricing: None,
                failing: false,
                retries: Retries::new(intervals),
            };
            watch.run(first);
        });
        Self { worker, league }
    }

    /// Gives the prices of `league` from now on, when it changed.
    pub fn price(&mut self, league: Option<&League>) {
        if let Some(league) = league
            && self.league.as_ref() != Some(league)
        {
            self.league = Some(league.clone());
            self.worker.order(league.clone());
        }
    }

    /// Gives each event received since the last call to `handle`, without waiting. If the
    /// thread is found gone without being asked to stop (a panic), the database counts as
    /// unreachable for good.
    pub fn receive(&mut self, mut handle: impl FnMut(MarketEvent)) {
        if self.worker.receive(&mut handle) {
            handle(MarketEvent::Unreachable(Source::Leagues));
        }
    }
}

/// The league whose prices the thread gives.
#[derive(Debug, Clone)]
struct Pricing {
    league: League,
    /// When to download its prices.
    due: Instant,
    /// Complete prices of this league were given, from the cache or the database: a new
    /// download replaces them at its end only.
    known: bool,
}

/// How a download of prices ended.
enum Downloaded {
    /// With the prices, or a failure.
    Done(MarketEvent),
    /// The application asked for something else meanwhile.
    Interrupted(Waited<League>),
}

/// The thread: what it reads, and when.
struct Watch<'a, D> {
    database: D,
    cache: PriceCache,
    outbox: &'a Outbox<MarketEvent, League>,
    wake: &'a dyn Fn(),
    intervals: Intervals,
    leagues_due: Instant,
    pricing: Option<Pricing>,
    /// The last request failed, which has been logged.
    failing: bool,
    retries: Retries,
}

impl<D: PriceDatabase> Watch<'_, D> {
    /// Reads the database until the application stops the thread or stops listening; with
    /// the prices of `league` first, if it is known.
    fn run(mut self, league: Option<League>) {
        let mut waited = league.map(Waited::Order);
        loop {
            if let Some(waited) = waited.take() {
                match self.latest(waited) {
                    Waited::Order(league) => {
                        if !self.switch(league) {
                            return;
                        }
                    }
                    Waited::Stop => return,
                    Waited::Elapsed => {}
                }
            }
            let mut reachable = true;
            if Instant::now() >= self.leagues_due {
                let event = self.read_leagues();
                reachable = !matches!(event, MarketEvent::Unreachable(_));
                if !self.send(event) {
                    return;
                }
            }
            if let Some(pricing) = &mut self.pricing
                && Instant::now() >= pricing.due
            {
                if reachable {
                    let due = pricing.clone();
                    match self.read_prices(due) {
                        Downloaded::Done(event) => {
                            if !self.send(event) {
                                return;
                            }
                        }
                        Downloaded::Interrupted(interruption) => {
                            waited = Some(interruption);
                            continue;
                        }
                    }
                } else {
                    // One request per minute while the database can't be reached: the
                    // prices wait for the leagues to be read again.
                    pricing.due = self.leagues_due;
                }
            }
            waited = Some(self.outbox.wait_for_order(self.until_due()));
        }
    }

    /// The last thing the application asked, when `waited` is an order: the leagues the
    /// player clicked meanwhile are skipped.
    fn latest(&self, waited: Waited<League>) -> Waited<League> {
        match waited {
            Waited::Order(_) => self.outbox.latest_order().unwrap_or(waited),
            Waited::Elapsed | Waited::Stop => waited,
        }
    }

    /// Prices `league` from now on: its prices from the cache at once, and from the database
    /// when they are older than an hour. `false` when the application no longer listens.
    fn switch(&mut self, league: League) -> bool {
        let pricing = match self.cache.load(&league) {
            Some(prices) => {
                let age = prices.age(SystemTime::now());
                log::info!(
                    "Prices of {} from the cache, {} min old: {} items",
                    logging::plain(league.id()),
                    age.as_secs() / 60,
                    prices.item_count()
                );
                if !self.send(MarketEvent::CachedPrices(prices)) {
                    return false;
                }
                Pricing {
                    league,
                    due: Instant::now() + self.intervals.refresh.saturating_sub(age),
                    known: true,
                }
            }
            None => Pricing {
                league,
                due: Instant::now(),
                known: false,
            },
        };
        self.retries.league_changed();
        self.pricing = Some(pricing);
        true
    }

    fn read_leagues(&mut self) -> MarketEvent {
        match self.database.leagues() {
            Ok(leagues) => {
                self.succeeded();
                self.retries.leagues_read();
                let ids: Vec<String> = leagues
                    .iter()
                    .map(|league| logging::plain(league.id()))
                    .collect();
                log::info!("Leagues: {}", ids.join(", "));
                self.leagues_due = Instant::now() + self.intervals.refresh;
                MarketEvent::Leagues(Leagues::new(leagues))
            }
            Err(error) => {
                self.failed(&error);
                let retry = self.retries.leagues_failed(&error);
                self.leagues_due = Instant::now() + retry;
                MarketEvent::Unreachable(Source::Leagues)
            }
        }
    }

    /// Downloads the prices of the league of `pricing`, and keeps them in the cache; with the
    /// prices given so far when none are known.
    fn read_prices(&mut self, pricing: Pricing) -> Downloaded {
        let Pricing { league, known, .. } = pricing;
        let mut download = Progress {
            outbox: self.outbox,
            wake: self.wake,
            league: league.clone(),
            partial: !known,
            interruption: None,
        };
        let result = self.database.prices(&league, &mut download);
        let (event, next) = match result {
            Ok(Some(prices)) => {
                self.succeeded();
                self.retries.prices_read();
                log::info!(
                    "Prices of {}: {} items",
                    logging::plain(league.id()),
                    prices.item_count()
                );
                self.cache.save(&prices);
                (MarketEvent::Prices(prices), self.intervals.refresh)
            }
            Ok(None) => {
                if let Some(interruption) = download.interruption {
                    return Downloaded::Interrupted(interruption);
                }
                // A download only stops when it is interrupted; tried again later, not in a
                // loop.
                let failure = MarketEvent::Unreachable(Source::Prices(league.clone()));
                (failure, self.intervals.retry)
            }
            Err(error) => {
                self.failed(&error);
                let retry = self.retries.prices_failed(&error);
                let failure = MarketEvent::Unreachable(Source::Prices(league.clone()));
                (failure, retry)
            }
        };
        let known = known || matches!(event, MarketEvent::Prices(_));
        self.pricing = Some(Pricing {
            league,
            due: Instant::now() + next,
            known,
        });
        Downloaded::Done(event)
    }

    fn succeeded(&mut self) {
        if self.failing {
            log::info!("The price database can be reached again");
        }
        self.failing = false;
    }

    fn failed(&mut self, error: &FetchError) {
        // Once, then quietly: offline for a night would fill the log.
        if self.failing {
            log::debug!("The price database still can't be reached: {error}");
        } else {
            log::warn!("The price database can't be reached: {error}");
        }
        self.failing = true;
    }

    /// The time until the next request.
    fn until_due(&self) -> Duration {
        let next = match &self.pricing {
            Some(pricing) => self.leagues_due.min(pricing.due),
            None => self.leagues_due,
        };
        next.saturating_duration_since(Instant::now())
    }

    /// Sends `event` and wakes the application up; `false` when it no longer listens.
    fn send(&self, event: MarketEvent) -> bool {
        if !self.outbox.send(event) {
            return false;
        }
        (self.wake)();
        true
    }
}

/// A download of prices seen from the thread: it stops when the application asks for
/// another league, and gives the prices downloaded so far when none of the league are known.
struct Progress<'a> {
    outbox: &'a Outbox<MarketEvent, League>,
    wake: &'a dyn Fn(),
    /// The league downloaded.
    league: League,
    partial: bool,
    /// What the application asked during the download.
    interruption: Option<Waited<League>>,
}

impl DownloadObserver for Progress<'_> {
    fn interrupted(&mut self) -> bool {
        if self.interruption != Some(Waited::Stop)
            && let Some(latest) = self.outbox.latest_order()
        {
            self.interruption = Some(latest);
        }
        // Back to the league downloaded after another one: the download goes on.
        if self.interruption.as_ref() == Some(&Waited::Order(self.league.clone())) {
            self.interruption = None;
        }
        self.interruption.is_some()
    }

    fn progress(&mut self, prices: &PriceList, progress: f32) {
        if !self.partial {
            return;
        }
        let event = MarketEvent::PartialPrices {
            prices: prices.clone(),
            progress,
        };
        if self.outbox.send(event) {
            (self.wake)();
        } else {
            self.interruption = Some(Waited::Stop);
        }
    }
}

#[cfg(test)]
mod tests;
