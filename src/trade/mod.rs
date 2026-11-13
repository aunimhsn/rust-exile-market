//! The official trade site: the links to its searches ([`link`]), its offers, searched on
//! their own thread when the player asks ([`search`]), and the names of the items in each
//! language, read on their own thread, which find the price of an item copied in French, with
//! the templates of its stats.
//!
//! The thread gives the names of the cache at once, then downloads them when they are older
//! than a day, and every day after. When the trade site can't be reached, the names of the
//! cache stay, and it tries again every hour; every minute while no name is known, unless
//! the trade site refused the request. While no name is known and the trade site answers
//! with an error, or with something the application can't read, the next attempt waits longer
//! and longer, an hour at most: the six requests of a download fail the same way each time.

mod client;
mod limits;
pub mod link;
pub mod offers;
pub mod pseudo;
pub mod query;
mod refusal;
pub mod search;
mod site;
pub mod stat_filters;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use crate::files;
use crate::http::Backoff;
use crate::item_names::ItemNames;
use crate::language::{Language, PerLanguage};
use crate::logging;
use crate::worker::{OnDrop, Outbox, Worker};
pub use refusal::Refusal;
use site::{NamesError, TradeSite};

/// The trade site of each language.
const HOSTS: PerLanguage<&str> = PerLanguage {
    english: "https://www.pathofexile.com",
    french: "https://fr.pathofexile.com",
};

/// The trade site of `language`.
fn host(language: Language) -> &'static str {
    HOSTS.of(language)
}

/// The file of the names in the cache directory.
const CACHE_FILE: &str = "item-names.json";
/// Changed with the shape of [`ItemNames`]: a file of another version is downloaded again.
const CACHE_VERSION: u32 = 2;

/// When the thread downloads the names: every `refresh`, and after a failure, `retry` while
/// names are known, `retry_without_names` before, doubled up to `retry` at each failure the
/// trade site answered.
#[derive(Debug, Clone, Copy)]
struct Intervals {
    refresh: Duration,
    retry: Duration,
    retry_without_names: Duration,
}

const INTERVALS: Intervals = Intervals {
    refresh: Duration::from_hours(24),
    retry: Duration::from_hours(1),
    retry_without_names: Duration::from_mins(1),
};

impl Intervals {
    fn backoff(self) -> Backoff {
        Backoff::new(self.retry_without_names, self.retry)
    }

    /// The wait before the next attempt after `error`, with names `known` or not.
    fn retry_after(self, error: &NamesError, known: bool, backoff: &mut Backoff) -> Duration {
        if known {
            self.retry
        } else if error.answered() {
            backoff.failed()
        } else {
            self.retry_without_names
        }
    }
}

/// Where the thread reads the names: the trade site, or a fake one in the tests.
trait NameSource {
    fn item_names(&mut self) -> Result<ItemNames, NamesError>;
}

impl NameSource for TradeSite {
    fn item_names(&mut self) -> Result<ItemNames, NamesError> {
        TradeSite::item_names(self)
    }
}

/// The names of the items read on their own thread, for as long as this value lives.
pub struct ItemNamesWatch {
    worker: Worker<ItemNames>,
}

impl ItemNamesWatch {
    /// Starts reading the names; `wake` is called from the thread after each event. No
    /// request is sent during the wait of `refusal`, shared with the searches of offers.
    ///
    /// # Panics
    ///
    /// If the system can't start a thread.
    #[must_use]
    pub fn start(refusal: Refusal, wake: impl Fn() + Send + 'static) -> Self {
        let cache = files::cache_directory();
        if cache.is_none() {
            log::warn!("No cache directory: the item names will be downloaded at each launch");
        }
        Self::spawn(move || TradeSite::new(refusal), cache, INTERVALS, wake)
    }

    fn spawn<S: NameSource>(
        open: impl FnOnce() -> S + Send + 'static,
        cache: Option<PathBuf>,
        intervals: Intervals,
        wake: impl Fn() + Send + 'static,
    ) -> Self {
        let worker = Worker::spawn("item-names", OnDrop::Detach, wake, move |outbox, wake| {
            watch(&mut open(), cache.as_deref(), intervals, outbox, wake);
        });
        Self { worker }
    }

    /// Gives the names received since the last call to `handle`, without waiting.
    pub fn receive(&mut self, handle: impl FnMut(ItemNames)) {
        // A thread gone by a panic leaves the names already given.
        self.worker.receive(handle);
    }
}

/// Reads the names until the application stops the thread or stops listening.
fn watch(
    source: &mut impl NameSource,
    cache: Option<&Path>,
    intervals: Intervals,
    outbox: &Outbox<ItemNames>,
    wake: &dyn Fn(),
) {
    let mut due = Instant::now();
    let mut known = false;
    if let Some(names) = cache.and_then(load) {
        let age = names.age(SystemTime::now());
        log::info!("Item names from the cache, {} h old", age.as_secs() / 3600);
        due += intervals.refresh.saturating_sub(age);
        known = true;
        if !outbox.send(names) {
            return;
        }
        wake();
    }
    let mut failing = false;
    let mut backoff = intervals.backoff();
    loop {
        if Instant::now() >= due {
            match source.item_names() {
                Ok(names) => {
                    let counts = names.counts();
                    log::info!(
                        "Item names: {} of the currency exchange, {} base types, {} uniques, \
                         {} stat templates",
                        counts.exchange,
                        counts.base_types,
                        counts.uniques,
                        counts.stats
                    );
                    failing = false;
                    backoff.succeeded();
                    known = true;
                    if let Some(directory) = cache {
                        save(directory, &names);
                    }
                    if !outbox.send(names) {
                        return;
                    }
                    wake();
                    due = Instant::now() + intervals.refresh;
                }
                // Not sent: the trade site refused a search, or the names, a moment ago.
                Err(NamesError::Wait(until)) => {
                    log::info!(
                        "The item names wait for the end of a refusal of the trade site, {} s",
                        until.saturating_duration_since(Instant::now()).as_secs()
                    );
                    due = until;
                }
                Err(error) => {
                    // Once, then quietly.
                    if failing {
                        log::debug!("The item names still can't be downloaded: {error}");
                    } else {
                        log::warn!("The item names can't be downloaded: {error}");
                    }
                    failing = true;
                    // A refusal waits as long as a failure with names, and for the delay it
                    // gives when longer: the trade site must not be asked again soon.
                    let now = Instant::now();
                    due = match error.refused_until() {
                        Some(until) => until.max(now + intervals.retry),
                        None => now + intervals.retry_after(&error, known, &mut backoff),
                    };
                }
            }
        }
        if !outbox.wait(due.saturating_duration_since(Instant::now())) {
            return;
        }
    }
}

/// The names of the cache in `directory`, if it has them.
fn load(directory: &Path) -> Option<ItemNames> {
    let path = directory.join(CACHE_FILE);
    files::read_versioned_json(&path, CACHE_VERSION).unwrap_or_else(|error| {
        log::info!(
            "{} can't be read, the item names will be downloaded again: {error}",
            logging::without_home(&path).display()
        );
        None
    })
}

/// Keeps `names` for the next launches; a failure is logged.
fn save(directory: &Path, names: &ItemNames) {
    if let Err(error) = files::write_versioned_json(directory, CACHE_FILE, CACHE_VERSION, names) {
        log::warn!(
            "The item names couldn't be saved in {}: {error}",
            logging::without_home(directory).display()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files::TestDirectory;
    use crate::http::HttpError;
    use crate::item_names::Names;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc::{self, Receiver};
    use std::thread;

    /// Only the retry without names is short enough for the tests to see it.
    const FAST: Intervals = Intervals {
        refresh: Duration::from_hours(24),
        retry: Duration::from_hours(1),
        retry_without_names: Duration::from_millis(20),
    };

    /// A trade site that gives empty names, fails with an error status, doesn't answer, or
    /// refuses with a 429 and the wait it asks for; it counts its requests.
    #[derive(Default)]
    struct FakeSite {
        failure: Option<u16>,
        unreachable: bool,
        refused_for: Option<Duration>,
        requests: Arc<AtomicUsize>,
    }

    impl NameSource for FakeSite {
        fn item_names(&mut self) -> Result<ItemNames, NamesError> {
            self.requests.fetch_add(1, Ordering::SeqCst);
            if let Some(wait) = self.refused_for {
                return Err(NamesError::Refused(429, Instant::now() + wait));
            }
            if let Some(status) = self.failure {
                return Err(status_error(status));
            }
            if self.unreachable {
                return Err(no_answer());
            }
            Ok(names(SystemTime::now()))
        }
    }

    fn status_error(status: u16) -> NamesError {
        NamesError::Http {
            path: "/data/items".to_owned(),
            error: HttpError::Request(ureq::Error::StatusCode(status)),
            after_answers: false,
        }
    }

    fn no_answer() -> NamesError {
        NamesError::Http {
            path: "/data/items".to_owned(),
            error: HttpError::Request(ureq::Error::ConnectionFailed),
            after_answers: false,
        }
    }

    fn names(updated: SystemTime) -> ItemNames {
        ItemNames::new(updated, Names::default(), Names::default(), Vec::new())
    }

    /// A thread reading `site`, with a cache in `directory`, and its wake-ups.
    fn start_watch(site: FakeSite, directory: &TestDirectory) -> (ItemNamesWatch, Receiver<()>) {
        let (woken, wakes) = mpsc::channel();
        let watch = ItemNamesWatch::spawn(
            move || site,
            Some(directory.0.clone()),
            FAST,
            move || {
                let _ = woken.send(());
            },
        );
        (watch, wakes)
    }

    /// The names received once the thread woke the application up.
    fn received(watch: &mut ItemNamesWatch, wakes: &Receiver<()>) -> Vec<ItemNames> {
        wakes
            .recv_timeout(Duration::from_secs(5))
            .expect("the application is woken up");
        let mut received = Vec::new();
        watch.receive(|names| received.push(names));
        received
    }

    /// The requests made by the site after `duration`.
    fn requests_after(requests: &AtomicUsize, duration: Duration) -> usize {
        thread::sleep(duration);
        requests.load(Ordering::SeqCst)
    }

    /// Waits for the first request of the thread, however busy the machine: "no other
    /// request for a while" is then checked from there, not from the start of the thread.
    fn wait_for_a_request(requests: &AtomicUsize) {
        let start = Instant::now();
        while requests.load(Ordering::SeqCst) == 0 {
            assert!(start.elapsed() < Duration::from_secs(5), "a first request");
            thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn downloads_the_names_once_and_keeps_them_in_the_cache() {
        let directory = TestDirectory::new("item-names-download");
        let site = FakeSite::default();
        let requests = Arc::clone(&site.requests);
        let (mut watch, wakes) = start_watch(site, &directory);

        assert_eq!(received(&mut watch, &wakes).len(), 1);
        assert!(load(&directory.0).is_some());
        assert_eq!(requests_after(&requests, Duration::from_millis(100)), 1);
    }

    #[test]
    fn names_younger_than_a_day_in_the_cache_are_not_downloaded_again() {
        let directory = TestDirectory::new("item-names-fresh");
        save(
            &directory.0,
            &names(SystemTime::now() - Duration::from_hours(2)),
        );
        let site = FakeSite::default();
        let requests = Arc::clone(&site.requests);
        let (mut watch, wakes) = start_watch(site, &directory);

        assert_eq!(received(&mut watch, &wakes).len(), 1);
        assert_eq!(requests_after(&requests, Duration::from_millis(100)), 0);
    }

    #[test]
    fn the_names_of_the_cache_stay_while_the_trade_site_cant_be_reached() {
        let directory = TestDirectory::new("item-names-offline");
        save(
            &directory.0,
            &names(SystemTime::now() - Duration::from_hours(48)),
        );
        let site = FakeSite {
            failure: Some(503),
            ..FakeSite::default()
        };
        let requests = Arc::clone(&site.requests);
        let (mut watch, wakes) = start_watch(site, &directory);

        assert_eq!(received(&mut watch, &wakes).len(), 1);
        // One attempt, the next one in an hour.
        assert_eq!(requests_after(&requests, Duration::from_millis(100)), 1);
        assert!(load(&directory.0).is_some(), "the cache is kept");
    }

    /// The network review of 2026-11-10: without names, a download that fails at its sixth
    /// request sent six requests a minute to the trade site.
    #[test]
    fn without_names_a_failure_the_trade_site_answered_waits_longer_and_longer() {
        let mut backoff = INTERVALS.backoff();
        let mut minutes = |error: &NamesError, known: bool| {
            INTERVALS.retry_after(error, known, &mut backoff).as_secs() / 60
        };
        let missing = NamesError::Missing("base types", crate::language::Language::French);

        assert_eq!(minutes(&status_error(500), false), 1);
        assert_eq!(minutes(&missing, false), 2);
        assert_eq!(minutes(&NamesError::NoUniquePairs, false), 4);
        // Nothing reached the trade site: a minute.
        assert_eq!(minutes(&no_answer(), false), 1);
        assert_eq!(minutes(&status_error(500), false), 8);
        // With names, an hour whatever the failure.
        assert_eq!(minutes(&status_error(500), true), 60);
        // A request that hangs after the first ones were answered: the trade site is there.
        let hung = NamesError::Http {
            path: "/data/stats".to_owned(),
            error: HttpError::Request(ureq::Error::Timeout(ureq::Timeout::Global)),
            after_answers: true,
        };
        assert!(hung.answered());
        assert_eq!(minutes(&no_answer(), true), 60);
        let waits: Vec<u64> = (0..4).map(|_| minutes(&status_error(500), false)).collect();
        assert_eq!(waits, [16, 32, 60, 60]);
    }

    #[test]
    fn without_names_a_failed_download_is_tried_again_sooner() {
        let directory = TestDirectory::new("item-names-none");
        let site = FakeSite {
            unreachable: true,
            ..FakeSite::default()
        };
        let requests = Arc::clone(&site.requests);
        let _watch = start_watch(site, &directory);

        let start = Instant::now();
        while requests.load(Ordering::SeqCst) < 3 {
            assert!(start.elapsed() < Duration::from_secs(5), "tried again");
            thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn a_refusal_of_the_trade_site_isnt_tried_again_soon() {
        let directory = TestDirectory::new("item-names-refused");
        // Its wait is over at once: the hour of a failure with names still holds.
        let site = FakeSite {
            refused_for: Some(Duration::ZERO),
            ..FakeSite::default()
        };
        let requests = Arc::clone(&site.requests);
        let _watch = start_watch(site, &directory);

        wait_for_a_request(&requests);
        assert_eq!(requests_after(&requests, Duration::from_millis(100)), 1);
    }

    #[test]
    fn a_refusal_waits_for_its_delay_when_longer_than_the_retry() {
        let directory = TestDirectory::new("item-names-long-refusal");
        // Every retry far shorter than the delay asked by the refusal.
        let hasty = Intervals {
            refresh: Duration::from_hours(24),
            retry: Duration::from_millis(20),
            retry_without_names: Duration::from_millis(20),
        };
        let site = FakeSite {
            refused_for: Some(Duration::from_secs(60)),
            ..FakeSite::default()
        };
        let requests = Arc::clone(&site.requests);
        let _watch = ItemNamesWatch::spawn(move || site, Some(directory.0.clone()), hasty, || {});

        wait_for_a_request(&requests);
        assert_eq!(requests_after(&requests, Duration::from_millis(200)), 1);
    }

    /// A trade site that waits for the end of a refusal, as during a refused search: `wait`
    /// from its first request, whenever the thread starts.
    struct WaitingSite {
        wait: Duration,
        until: Option<Instant>,
        requests: Arc<AtomicUsize>,
    }

    impl NameSource for WaitingSite {
        fn item_names(&mut self) -> Result<ItemNames, NamesError> {
            self.requests.fetch_add(1, Ordering::SeqCst);
            let now = Instant::now();
            let until = *self.until.get_or_insert(now + self.wait);
            if now < until {
                return Err(NamesError::Wait(until));
            }
            Ok(names(SystemTime::now()))
        }
    }

    #[test]
    fn the_names_are_asked_again_at_the_end_of_the_wait_of_a_refused_search() {
        let directory = TestDirectory::new("item-names-wait");
        let requests = Arc::new(AtomicUsize::new(0));
        let site = WaitingSite {
            wait: Duration::from_millis(600),
            until: None,
            requests: Arc::clone(&requests),
        };
        let (woken, wakes) = mpsc::channel();
        let mut watch = ItemNamesWatch::spawn(
            move || site,
            Some(directory.0.clone()),
            FAST,
            move || {
                let _ = woken.send(());
            },
        );

        // Asked once during the wait, not every 20 ms as after a failure without names.
        wait_for_a_request(&requests);
        assert_eq!(requests_after(&requests, Duration::from_millis(200)), 1);
        assert_eq!(received(&mut watch, &wakes).len(), 1);
        assert_eq!(requests.load(Ordering::SeqCst), 2);
    }
}
