//! The official trade site, read on its own thread: for now, the names of the items in each
//! language, which find the price of an item copied in French.
//!
//! The thread gives the names of the cache at once, then downloads them when they are older
//! than a day, and every day after. When the trade site can't be reached, the names of the
//! cache stay, and it tries again every hour.

mod site;

use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

use crate::files;
use crate::http::HttpError;
use crate::item_names::ItemNames;
use crate::logging;
use crate::worker::{OnDrop, Outbox, Worker};
use site::TradeSite;

/// The file of the names in the cache directory.
const CACHE_FILE: &str = "item-names.json";
/// Changed with the shape of [`ItemNames`]: a file of another version is downloaded again.
const CACHE_VERSION: u32 = 1;

/// When the thread downloads the names: every `refresh`, and `retry` after a failure.
#[derive(Debug, Clone, Copy)]
struct Intervals {
    refresh: Duration,
    retry: Duration,
}

const INTERVALS: Intervals = Intervals {
    refresh: Duration::from_hours(24),
    retry: Duration::from_hours(1),
};

/// Where the thread reads the names: the trade site, or a fake one in the tests.
trait NameSource {
    fn item_names(&mut self) -> Result<ItemNames, HttpError>;
}

impl NameSource for TradeSite {
    fn item_names(&mut self) -> Result<ItemNames, HttpError> {
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
        Self::spawn(TradeSite::new, files::cache_directory(), INTERVALS, wake)
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
    cache: Option<&std::path::Path>,
    intervals: Intervals,
    outbox: &Outbox<ItemNames>,
    wake: &dyn Fn(),
) {
    let mut due = Instant::now();
    if let Some(names) = cache.and_then(load) {
        let age = names.age(SystemTime::now());
        log::info!("Item names from the cache, {} h old", age.as_secs() / 3600);
        due += intervals.refresh.saturating_sub(age);
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
                    let (exchange, base_types, uniques) = names.counts();
                    log::info!(
                        "Item names: {exchange} of the currency exchange, {base_types} base \
                         types, {uniques} uniques"
                    );
                    failing = false;
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
                    due = Instant::now() + intervals.retry;
                }
            }
        }
        if !outbox.wait(due.saturating_duration_since(Instant::now())) {
            return;
        }
    }
}

/// The names of the cache in `directory`, if it has them.
fn load(directory: &std::path::Path) -> Option<ItemNames> {
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
fn save(directory: &std::path::Path, names: &ItemNames) {
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
    use crate::item_names::Names;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc::{self, Receiver};
    use std::thread;

    const FAST: Intervals = Intervals {
        refresh: Duration::from_hours(24),
        retry: Duration::from_millis(20),
    };

    /// A trade site that gives empty names, or fails; it counts its requests.
    #[derive(Default)]
    struct FakeSite {
        unreachable: bool,
        requests: Arc<AtomicUsize>,
    }

    impl NameSource for FakeSite {
        fn item_names(&mut self) -> Result<ItemNames, HttpError> {
            self.requests.fetch_add(1, Ordering::SeqCst);
            if self.unreachable {
                return Err(HttpError::Request(ureq::Error::StatusCode(503)));
            }
            Ok(names(SystemTime::now()))
        }
    }

    fn names(updated: SystemTime) -> ItemNames {
        ItemNames::new(updated, Names::default(), Names::default(), Vec::new())
    }

    fn watch(site: FakeSite, directory: &TestDirectory) -> (ItemNamesWatch, Receiver<()>) {
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

    #[test]
    fn downloads_the_names_and_keeps_them_in_the_cache() {
        let directory = TestDirectory::new("item-names-download");
        let (mut watch, wakes) = watch(FakeSite::default(), &directory);

        assert_eq!(received(&mut watch, &wakes).len(), 1);
        assert!(load(&directory.0).is_some());
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
        let (mut watch, wakes) = watch(site, &directory);

        assert_eq!(received(&mut watch, &wakes).len(), 1);
        thread::sleep(Duration::from_millis(100));
        assert_eq!(requests.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn the_names_of_the_cache_stay_while_the_trade_site_cant_be_reached() {
        let directory = TestDirectory::new("item-names-offline");
        save(
            &directory.0,
            &names(SystemTime::now() - Duration::from_hours(48)),
        );
        let site = FakeSite {
            unreachable: true,
            ..FakeSite::default()
        };
        let requests = Arc::clone(&site.requests);
        let (mut watch, wakes) = watch(site, &directory);

        assert_eq!(received(&mut watch, &wakes).len(), 1);
        // Tried again, without insisting.
        let start = Instant::now();
        while requests.load(Ordering::SeqCst) < 2 {
            assert!(start.elapsed() < Duration::from_secs(5), "tried again");
            thread::sleep(Duration::from_millis(5));
        }
        assert!(load(&directory.0).is_some(), "the cache is kept");
    }
}
