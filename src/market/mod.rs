//! The price database, read on its own thread: the leagues, and the prices of the league
//! shown.
//!
//! The thread asks for the leagues at startup, then every hour. It gives the prices of the
//! league the application asks for: from the cache at once, then from the database when the
//! cache is older than an hour, and every hour after. Without a cache, the prices arrive
//! category by category. When the database can't be reached, it tries again every minute,
//! with one request: a download waits while the leagues can't be read, and stops at its first
//! failed request.

mod cache;
mod poe2scout;

use std::time::{Duration, Instant, SystemTime};

pub use poe2scout::FetchError;
use poe2scout::Poe2Scout;

use crate::league::{League, Leagues};
use crate::prices::PriceList;
use crate::worker::{OnDrop, Outbox, Waited, Worker};
use cache::PriceCache;

/// When the thread reads the database: every `refresh`, and `retry` after a failure.
#[derive(Debug, Clone, Copy)]
struct Intervals {
    refresh: Duration,
    retry: Duration,
}

const INTERVALS: Intervals = Intervals {
    refresh: Duration::from_hours(1),
    retry: Duration::from_mins(1),
};

/// What the price database tells the application.
#[derive(Debug, Clone, PartialEq)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Leagues,
    Prices,
}

/// What a download of prices tells the thread, between two requests.
trait Download {
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
        download: &mut dyn Download,
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
        Self::spawn(
            Poe2Scout::new,
            PriceCache::in_cache_directory,
            INTERVALS,
            league,
            wake,
        )
    }

    fn spawn<D: PriceDatabase>(
        open: impl FnOnce() -> D + Send + 'static,
        open_cache: impl FnOnce() -> PriceCache + Send + 'static,
        intervals: Intervals,
        league: Option<League>,
        wake: impl Fn() + Send + 'static,
    ) -> Self {
        let first = league.clone();
        let worker = Worker::spawn("market", OnDrop::Detach, wake, move |outbox, wake| {
            let watch = Watch {
                database: open(),
                cache: open_cache(),
                outbox,
                wake,
                intervals,
                leagues_due: Instant::now(),
                pricing: None,
                failing: false,
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
    /// thread ended by itself (a panic), the database counts as unreachable for good.
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
                reachable = event != MarketEvent::Unreachable(Source::Leagues);
                if !self.send(event) {
                    return;
                }
            }
            let prices_due = self
                .pricing
                .as_ref()
                .is_some_and(|pricing| Instant::now() >= pricing.due);
            if prices_due && !reachable {
                // One request per minute while the database can't be reached: the prices
                // wait for the leagues to be read again.
                if let Some(pricing) = &mut self.pricing {
                    pricing.due = self.leagues_due;
                }
            } else if prices_due {
                match self.read_prices() {
                    Ok(event) => {
                        if !self.send(event) {
                            return;
                        }
                    }
                    // The application asked for something else during the download.
                    Err(interruption) => {
                        waited = Some(interruption);
                        continue;
                    }
                }
            }
            waited = Some(self.outbox.wait_for_order(self.until_due()));
        }
    }

    /// The last thing the application asked, when `waited` is an order: the leagues the
    /// player clicked meanwhile are skipped.
    fn latest(&self, mut waited: Waited<League>) -> Waited<League> {
        if !matches!(waited, Waited::Order(_)) {
            return waited;
        }
        while let Some(next) = self.outbox.pending_order() {
            waited = next;
            if waited == Waited::Stop {
                break;
            }
        }
        waited
    }

    /// Prices `league` from now on: its prices from the cache at once, and from the database
    /// when they are older than an hour. `false` when the application no longer listens.
    fn switch(&mut self, league: League) -> bool {
        let pricing = match self.cache.load(&league) {
            Some(prices) => {
                let age = prices.age(SystemTime::now());
                log::info!(
                    "Prices of {} from the cache, {} min old: {} items",
                    league.id(),
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
        self.pricing = Some(pricing);
        true
    }

    fn read_leagues(&mut self) -> MarketEvent {
        match self.database.leagues() {
            Ok(leagues) => {
                self.succeeded();
                log::info!("Leagues: {leagues:?}");
                self.leagues_due = Instant::now() + self.intervals.refresh;
                MarketEvent::Leagues(Leagues::new(leagues))
            }
            Err(error) => {
                self.failed(&error);
                self.leagues_due = Instant::now() + self.intervals.retry;
                MarketEvent::Unreachable(Source::Leagues)
            }
        }
    }

    /// Downloads the prices of the league priced, and keeps them in the cache; what the
    /// application asked for when it interrupted the download.
    fn read_prices(&mut self) -> Result<MarketEvent, Waited<League>> {
        let Some(Pricing { league, known, .. }) = self.pricing.clone() else {
            return Err(Waited::Elapsed);
        };
        let mut download = Progress {
            outbox: self.outbox,
            wake: self.wake,
            partial: !known,
            interruption: None,
        };
        let result = self.database.prices(&league, &mut download);
        let (event, next) = match result {
            Ok(Some(prices)) => {
                self.succeeded();
                log::info!("Prices of {}: {} items", league.id(), prices.item_count());
                self.cache.save(&prices);
                (MarketEvent::Prices(prices), self.intervals.refresh)
            }
            Ok(None) => {
                if let Some(interruption) = download.interruption {
                    return Err(interruption);
                }
                // A download only stops when it is interrupted; tried again later, not in a
                // loop.
                (
                    MarketEvent::Unreachable(Source::Prices),
                    self.intervals.retry,
                )
            }
            Err(error) => {
                self.failed(&error);
                (
                    MarketEvent::Unreachable(Source::Prices),
                    self.intervals.retry,
                )
            }
        };
        let known = known || matches!(event, MarketEvent::Prices(_));
        self.pricing = Some(Pricing {
            league,
            due: Instant::now() + next,
            known,
        });
        Ok(event)
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
/// something else, and gives the prices downloaded so far when none of the league are known.
struct Progress<'a> {
    outbox: &'a Outbox<MarketEvent, League>,
    wake: &'a dyn Fn(),
    partial: bool,
    /// What the application asked during the download.
    interruption: Option<Waited<League>>,
}

impl Download for Progress<'_> {
    fn interrupted(&mut self) -> bool {
        if self.interruption.is_none() {
            self.interruption = self.outbox.pending_order();
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
mod tests {
    use super::*;
    use crate::files::TestDirectory;
    use crate::prices::Rates;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc::{self, Receiver};
    use std::sync::{Arc, Mutex};
    use std::thread;

    /// Intervals short enough for the tests to see the retries.
    const FAST: Intervals = Intervals {
        refresh: Duration::from_hours(1),
        retry: Duration::from_millis(20),
    };

    /// A database that gives the same leagues and empty prices in two steps, or fails; it
    /// counts its requests.
    #[derive(Default)]
    struct FakeDatabase {
        /// Fails to give the leagues.
        leagues_unreachable: bool,
        /// Fails to give the prices.
        prices_unreachable: bool,
        /// The download of the prices of this league waits until it is interrupted.
        endless: Option<League>,
        /// The endless download looks for its interruption once this gives a message.
        release: Option<Receiver<()>>,
        requests: Arc<Requests>,
    }

    #[derive(Default)]
    struct Requests {
        leagues: AtomicUsize,
        prices: AtomicUsize,
        /// The downloads that saw their interruption, with what interrupted them.
        interrupted: Mutex<Vec<String>>,
    }

    impl PriceDatabase for FakeDatabase {
        fn leagues(&mut self) -> Result<Vec<League>, FetchError> {
            self.requests.leagues.fetch_add(1, Ordering::SeqCst);
            if self.leagues_unreachable {
                return Err(FetchError::Http(ureq::Error::StatusCode(503)));
            }
            Ok(vec![League::new("Forbidden Rites")])
        }

        fn prices(
            &mut self,
            league: &League,
            download: &mut dyn Download,
        ) -> Result<Option<PriceList>, FetchError> {
            self.requests.prices.fetch_add(1, Ordering::SeqCst);
            if self.endless.as_ref() == Some(league) {
                if let Some(release) = &self.release {
                    release
                        .recv_timeout(Duration::from_secs(5))
                        .expect("released");
                }
                let start = Instant::now();
                while !download.interrupted() {
                    assert!(start.elapsed() < Duration::from_secs(5), "interrupted");
                    thread::sleep(Duration::from_millis(1));
                }
                self.requests
                    .interrupted
                    .lock()
                    .unwrap()
                    .push(league.id().to_owned());
                return Ok(None);
            }
            if self.prices_unreachable {
                return Err(FetchError::Http(ureq::Error::StatusCode(503)));
            }
            let prices = price_list(league, SystemTime::now());
            download.progress(&prices, 0.5);
            Ok(Some(prices))
        }
    }

    fn price_list(league: &League, updated: SystemTime) -> PriceList {
        PriceList {
            league: league.clone(),
            updated,
            rates: Rates {
                chaos: 75.0,
                divine: 750.0,
            },
            uniques: Vec::new(),
            currencies: Vec::new(),
        }
    }

    /// A thread reading `database`, with a cache in `directory`, and its wake-ups.
    fn watch(
        database: FakeDatabase,
        directory: &TestDirectory,
        league: Option<&str>,
    ) -> (MarketWatch, Receiver<()>) {
        let (woken, wakes) = mpsc::channel();
        let cache = directory.0.clone();
        let watch = MarketWatch::spawn(
            move || database,
            move || PriceCache::in_directory(cache),
            FAST,
            league.map(League::new),
            move || {
                let _ = woken.send(());
            },
        );
        (watch, wakes)
    }

    /// The next `count` events of the thread.
    fn events(watch: &mut MarketWatch, wakes: &Receiver<()>, count: usize) -> Vec<String> {
        let mut received = Vec::new();
        while received.len() < count {
            wakes
                .recv_timeout(Duration::from_secs(5))
                .expect("the application is woken up");
            watch.receive(|event| received.push(kind(&event)));
        }
        received.truncate(count);
        received
    }

    /// The events received in `duration`.
    fn events_during(watch: &mut MarketWatch, duration: Duration) -> Vec<String> {
        thread::sleep(duration);
        let mut received = Vec::new();
        watch.receive(|event| received.push(kind(&event)));
        received
    }

    /// A short name of an event, to compare them.
    fn kind(event: &MarketEvent) -> String {
        match event {
            MarketEvent::Leagues(_) => "leagues".to_owned(),
            MarketEvent::CachedPrices(prices) => format!("cached {}", prices.league.id()),
            MarketEvent::PartialPrices { prices, progress } => {
                format!("partial {} {progress}", prices.league.id())
            }
            MarketEvent::Prices(prices) => format!("prices {}", prices.league.id()),
            MarketEvent::Unreachable(source) => format!("unreachable {source:?}"),
        }
    }

    #[test]
    fn sends_the_leagues_at_startup() {
        let directory = TestDirectory::new("market-leagues");
        let (mut watch, wakes) = watch(FakeDatabase::default(), &directory, None);

        assert_eq!(events(&mut watch, &wakes, 1), ["leagues"]);
    }

    #[test]
    fn an_outage_costs_one_request_per_attempt() {
        let directory = TestDirectory::new("market-outage");
        let database = FakeDatabase {
            leagues_unreachable: true,
            prices_unreachable: true,
            ..FakeDatabase::default()
        };
        let requests = Arc::clone(&database.requests);
        let (mut watch, wakes) = watch(database, &directory, Some("Standard"));

        assert_eq!(
            events(&mut watch, &wakes, 3),
            [
                "unreachable Leagues",
                "unreachable Leagues",
                "unreachable Leagues"
            ]
        );
        assert_eq!(requests.prices.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn a_failed_download_is_tried_again() {
        let directory = TestDirectory::new("market-retry");
        let database = FakeDatabase {
            prices_unreachable: true,
            ..FakeDatabase::default()
        };
        let requests = Arc::clone(&database.requests);
        let (mut watch, wakes) = watch(database, &directory, Some("Standard"));

        assert_eq!(
            events(&mut watch, &wakes, 3),
            ["leagues", "unreachable Prices", "unreachable Prices"]
        );
        assert_eq!(requests.leagues.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn downloads_the_prices_of_the_league_in_steps_and_keeps_them_in_the_cache() {
        let directory = TestDirectory::new("market-download");
        let (mut watch, wakes) = watch(FakeDatabase::default(), &directory, Some("Standard"));

        assert_eq!(
            events(&mut watch, &wakes, 3),
            ["leagues", "partial Standard 0.5", "prices Standard"]
        );
        let cache = PriceCache::in_directory(directory.0.clone());
        assert!(cache.load(&League::new("Standard")).is_some());
    }

    #[test]
    fn prices_younger_than_an_hour_in_the_cache_are_not_downloaded_again() {
        let directory = TestDirectory::new("market-fresh-cache");
        let standard = League::new("Standard");
        PriceCache::in_directory(directory.0.clone()).save(&price_list(
            &standard,
            SystemTime::now() - Duration::from_mins(10),
        ));
        let database = FakeDatabase::default();
        let requests = Arc::clone(&database.requests);
        let (mut watch, wakes) = watch(database, &directory, Some("Standard"));

        assert_eq!(
            events(&mut watch, &wakes, 2),
            ["cached Standard", "leagues"]
        );
        assert_eq!(
            events_during(&mut watch, Duration::from_millis(100)),
            Vec::<String>::new()
        );
        assert_eq!(requests.prices.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn older_prices_in_the_cache_are_replaced_at_the_end_of_the_download() {
        let directory = TestDirectory::new("market-old-cache");
        let standard = League::new("Standard");
        PriceCache::in_directory(directory.0.clone()).save(&price_list(
            &standard,
            SystemTime::now() - Duration::from_hours(3),
        ));
        let (mut watch, wakes) = watch(FakeDatabase::default(), &directory, Some("Standard"));

        // Without the partial prices, which would hide the complete ones of the cache.
        assert_eq!(
            events(&mut watch, &wakes, 3),
            ["cached Standard", "leagues", "prices Standard"]
        );
    }

    #[test]
    fn a_new_league_interrupts_the_download_of_the_previous_one() {
        let directory = TestDirectory::new("market-switch");
        let database = FakeDatabase {
            endless: Some(League::new("Standard")),
            ..FakeDatabase::default()
        };
        let requests = Arc::clone(&database.requests);
        let (mut watch, wakes) = watch(database, &directory, Some("Standard"));
        assert_eq!(events(&mut watch, &wakes, 1), ["leagues"]);

        watch.price(Some(&League::new("Forbidden Rites")));

        assert_eq!(
            events(&mut watch, &wakes, 2),
            ["partial Forbidden Rites 0.5", "prices Forbidden Rites"]
        );
        assert_eq!(*requests.interrupted.lock().unwrap(), ["Standard"]);
    }

    #[test]
    fn only_the_last_league_clicked_is_downloaded() {
        let directory = TestDirectory::new("market-clicks");
        let (release, released) = mpsc::channel();
        let database = FakeDatabase {
            endless: Some(League::new("Standard")),
            release: Some(released),
            ..FakeDatabase::default()
        };
        let requests = Arc::clone(&database.requests);
        let (mut watch, wakes) = watch(database, &directory, Some("Standard"));
        assert_eq!(events(&mut watch, &wakes, 1), ["leagues"]);

        for league in ["Runes of Aldur", "HC Runes of Aldur", "Forbidden Rites"] {
            watch.price(Some(&League::new(league)));
        }
        release.send(()).unwrap();

        assert_eq!(
            events(&mut watch, &wakes, 2),
            ["partial Forbidden Rites 0.5", "prices Forbidden Rites"]
        );
        // The endless download of Standard, then Forbidden Rites only.
        assert_eq!(requests.prices.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn closing_the_application_stops_a_download() {
        let directory = TestDirectory::new("market-stop");
        let database = FakeDatabase {
            endless: Some(League::new("Standard")),
            ..FakeDatabase::default()
        };
        let requests = Arc::clone(&database.requests);
        let (mut watch, wakes) = watch(database, &directory, Some("Standard"));
        assert_eq!(events(&mut watch, &wakes, 1), ["leagues"]);

        drop(watch);

        let start = Instant::now();
        while requests.interrupted.lock().unwrap().is_empty() {
            assert!(start.elapsed() < Duration::from_secs(5), "stopped");
            thread::sleep(Duration::from_millis(1));
        }
    }
}
