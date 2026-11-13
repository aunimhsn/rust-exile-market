use super::*;
use crate::files::TestDirectory;
use crate::http::HttpError;
use crate::price::Rates;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;

/// Intervals short enough for the tests to see the retries.
const FAST: Intervals = Intervals {
    refresh: Duration::from_hours(1),
    retry: Duration::from_millis(20),
    longest_retry: Duration::from_secs(5),
};

/// A database that gives the same leagues and empty prices in two steps, or fails; it
/// counts its requests.
#[derive(Default)]
struct FakeDatabase {
    /// Fails to give the leagues.
    leagues_unreachable: bool,
    /// Fails to give the prices.
    prices_unreachable: bool,
    /// Answers the prices with an error status.
    prices_refused: bool,
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
            return Err(unreachable("/Leagues"));
        }
        Ok(vec![League::new("Forbidden Rites")])
    }

    fn prices(
        &mut self,
        league: &League,
        download: &mut dyn DownloadObserver,
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
            return Err(unreachable("/Items"));
        }
        if self.prices_refused {
            return Err(answered_error());
        }
        let prices = price_list(league, SystemTime::now());
        download.progress(&prices, 0.5);
        Ok(Some(prices))
    }
}

/// No answer of the database.
fn unreachable(path: &str) -> FetchError {
    FetchError::Http {
        path: path.to_owned(),
        error: HttpError::Request(ureq::Error::ConnectionFailed),
        after_answers: false,
    }
}

/// An error the database answered.
fn answered_error() -> FetchError {
    FetchError::Http {
        path: "/Items".to_owned(),
        error: HttpError::Request(ureq::Error::StatusCode(500)),
        after_answers: false,
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
        MarketEvent::Unreachable(Source::Leagues) => "unreachable leagues".to_owned(),
        MarketEvent::Unreachable(Source::Prices(league)) => {
            format!("unreachable prices {}", league.id())
        }
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
            "unreachable leagues",
            "unreachable leagues",
            "unreachable leagues"
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
        [
            "leagues",
            "unreachable prices Standard",
            "unreachable prices Standard"
        ]
    );
    assert_eq!(requests.leagues.load(Ordering::SeqCst), 1);
}

/// The network review of 2026-11-10: a download that fails at its twentieth request
/// sent twenty requests a minute, for as long as the application ran.
#[test]
fn a_failure_the_database_answered_waits_longer_and_longer() {
    let intervals = Intervals {
        refresh: Duration::from_hours(1),
        retry: Duration::from_mins(1),
        longest_retry: Duration::from_hours(1),
    };
    let mut backoff = intervals.backoff();
    let mut minutes =
        |error: &FetchError| intervals.retry_after(error, &mut backoff).as_secs() / 60;

    assert_eq!(minutes(&answered_error()), 1);
    assert_eq!(minutes(&FetchError::MissingRate("divine")), 2);
    assert_eq!(minutes(&FetchError::NoCurrentLeague), 4);
    // Nothing reached the database: a minute, and the longer waits aren't forgotten.
    assert_eq!(minutes(&unreachable("/Leagues")), 1);
    assert_eq!(minutes(&answered_error()), 8);
    let waits: Vec<u64> = (0..4).map(|_| minutes(&answered_error())).collect();
    assert_eq!(waits, [16, 32, 60, 60]);
}

/// The code review of the quality fixes: nothing pinned when each wait starts over, and
/// the leagues, read with success every hour, would have made the doubling useless.
#[test]
fn the_leagues_read_dont_start_over_the_wait_of_the_prices() {
    let mut retries = Retries::new(INTERVALS);
    let minutes = |wait: Duration| wait.as_secs() / 60;
    let refused = answered_error();

    assert_eq!(minutes(retries.prices_failed(&refused)), 1);
    assert_eq!(minutes(retries.prices_failed(&refused)), 2);
    retries.leagues_read();
    assert_eq!(minutes(retries.prices_failed(&refused)), 4);
    // Nor do the leagues that fail lengthen it, or the prices that of the leagues.
    assert_eq!(minutes(retries.leagues_failed(&refused)), 1);
    assert_eq!(minutes(retries.leagues_failed(&refused)), 2);
    assert_eq!(minutes(retries.prices_failed(&refused)), 8);
    retries.prices_read();
    assert_eq!(minutes(retries.leagues_failed(&refused)), 4);
    assert_eq!(minutes(retries.prices_failed(&refused)), 1);
}

#[test]
fn another_league_starts_the_wait_of_the_prices_over() {
    let mut retries = Retries::new(INTERVALS);
    let refused = answered_error();
    retries.prices_failed(&refused);
    retries.prices_failed(&refused);
    retries.leagues_failed(&refused);

    retries.league_changed();

    assert_eq!(retries.prices_failed(&refused), Duration::from_mins(1));
    assert_eq!(retries.leagues_failed(&refused), Duration::from_mins(2));
}

/// The same in the thread: the leagues are read with success every 30 ms, and the prices
/// keep their longer and longer wait.
#[test]
fn the_leagues_read_often_dont_hasten_a_download_the_database_refuses() {
    let directory = TestDirectory::new("market-backoff-leagues");
    let database = FakeDatabase {
        prices_refused: true,
        ..FakeDatabase::default()
    };
    let requests = Arc::clone(&database.requests);
    let cache = directory.0.clone();
    let hourly = Intervals {
        refresh: Duration::from_millis(30),
        ..FAST
    };
    let _watch = MarketWatch::spawn(
        move || database,
        move || PriceCache::in_directory(cache),
        hourly,
        Some(League::new("Standard")),
        || {},
    );

    // Twenty readings of the leagues, 30 ms apart, however busy the machine.
    let start = Instant::now();
    while requests.leagues.load(Ordering::SeqCst) < 20 {
        assert!(
            start.elapsed() < Duration::from_secs(10),
            "the leagues are read"
        );
        thread::sleep(Duration::from_millis(5));
    }

    // Every 20 ms, about thirty in these 600 ms; doubled, at 0, 20, 60, 140 and 300 ms,
    // five or six.
    let downloads = requests.prices.load(Ordering::SeqCst);
    assert!(downloads < 10, "{downloads} downloads");
}

#[test]
fn a_download_the_database_refuses_is_tried_again_less_and_less_often() {
    let directory = TestDirectory::new("market-backoff");
    let database = FakeDatabase {
        prices_refused: true,
        ..FakeDatabase::default()
    };
    let requests = Arc::clone(&database.requests);
    let (mut watch, wakes) = watch(database, &directory, Some("Standard"));
    // The leagues, then the downloads at 0, 20 and 60 ms.
    events(&mut watch, &wakes, 4);

    thread::sleep(Duration::from_millis(300));

    // Every 20 ms, about fifteen more; doubled, 140, 300 and 620 ms after the first.
    let downloads = requests.prices.load(Ordering::SeqCst);
    assert!((3..=6).contains(&downloads), "{downloads} downloads");
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
fn coming_back_to_the_league_downloaded_doesnt_restart_its_download() {
    let directory = TestDirectory::new("market-back");
    let (release, released) = mpsc::channel();
    let database = FakeDatabase {
        endless: Some(League::new("Standard")),
        release: Some(released),
        ..FakeDatabase::default()
    };
    let requests = Arc::clone(&database.requests);
    let (mut watch, wakes) = watch(database, &directory, Some("Standard"));
    assert_eq!(events(&mut watch, &wakes, 1), ["leagues"]);

    watch.price(Some(&League::new("Runes of Aldur")));
    watch.price(Some(&League::new("Standard")));
    release.send(()).unwrap();

    // Still the first download of Standard, which only an order of another league ends.
    thread::sleep(Duration::from_millis(50));
    watch.price(Some(&League::new("Forbidden Rites")));
    assert_eq!(
        events(&mut watch, &wakes, 2),
        ["partial Forbidden Rites 0.5", "prices Forbidden Rites"]
    );
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
