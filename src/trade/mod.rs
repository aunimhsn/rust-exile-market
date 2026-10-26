//! The official trade site: the links to its searches ([`link`]), its offers, searched on
//! their own thread when the player asks ([`search`]), and the names of the items in each
//! language, read on their own thread, which find the price of an item copied in French.
//!
//! The thread gives the names of the cache at once, then downloads them when they are older
//! than a day, and every day after. When the trade site can't be reached, the names of the
//! cache stay, and it tries again every hour; every minute while no name is known, unless
//! the trade site refused the request.

mod client;
mod limits;
pub mod link;
pub mod offers;
pub mod query;
pub mod search;
mod site;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use crate::files;
use crate::item_names::ItemNames;
use crate::language::Language;
use crate::logging;
use crate::worker::{OnDrop, Outbox, Worker};
use site::{NamesError, TradeSite};

/// The trade site of `language`.
fn host(language: Language) -> &'static str {
    match language {
        Language::English => "https://www.pathofexile.com",
        Language::French => "https://fr.pathofexile.com",
    }
}

/// The file of the names in the cache directory.
const CACHE_FILE: &str = "item-names.json";
/// Changed with the shape of [`ItemNames`]: a file of another version is downloaded again.
const CACHE_VERSION: u32 = 1;

/// When the thread downloads the names: every `refresh`, and after a failure, `retry` while
/// names are known, `retry_without_names` before.
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
    /// Starts reading the names; `wake` is called from the thread after each event.
    ///
    /// # Panics
    ///
    /// If the system can't start a thread.
    #[must_use]
    pub fn start(wake: impl Fn() + Send + 'static) -> Self {
        let cache = files::cache_directory();
        if cache.is_none() {
            log::warn!("No cache directory: the item names will be downloaded at each launch");
        }
        Self::spawn(TradeSite::new, cache, INTERVALS, wake)
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
    loop {
        if Instant::now() >= due {
            match source.item_names() {
                Ok(names) => {
                    let counts = names.counts();
                    log::info!(
                        "Item names: {} of the currency exchange, {} base types, {} uniques",
                        counts.exchange,
                        counts.base_types,
                        counts.uniques
                    );
                    failing = false;
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
                Err(error) => {
                    // Once, then quietly.
                    if failing {
                        log::debug!("The item names still can't be downloaded: {error}");
                    } else {
                        log::warn!("The item names can't be downloaded: {error}");
                    }
                    failing = true;
                    // A refusal waits as long as a failure with names: the trade site must
                    // not be asked again soon.
                    let retry = if known || error.is_refusal() {
                        intervals.retry
                    } else {
                        intervals.retry_without_names
                    };
                    due = Instant::now() + retry;
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

    /// A trade site that gives empty names, or fails with an error status; it counts its
    /// requests.
    #[derive(Default)]
    struct FakeSite {
        failure: Option<u16>,
        requests: Arc<AtomicUsize>,
    }

    impl NameSource for FakeSite {
        fn item_names(&mut self) -> Result<ItemNames, NamesError> {
            self.requests.fetch_add(1, Ordering::SeqCst);
            if let Some(status) = self.failure {
                let error = HttpError::Request(ureq::Error::StatusCode(status));
                return Err(NamesError::Http(error));
            }
            Ok(names(SystemTime::now()))
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

    #[test]
    fn without_names_a_failed_download_is_tried_again_sooner() {
        let directory = TestDirectory::new("item-names-none");
        let site = FakeSite {
            failure: Some(503),
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
        let site = FakeSite {
            failure: Some(429),
            ..FakeSite::default()
        };
        let requests = Arc::clone(&site.requests);
        let _watch = start_watch(site, &directory);

        assert_eq!(requests_after(&requests, Duration::from_millis(100)), 1);
    }
}
