//! The names of the items in each language, read on their own thread from the trade site.
//!
//! The thread gives the names of the cache at once, then downloads them when they are older
//! than a day, and every day after. When the trade site can't be reached, the names of the
//! cache stay, and it tries again every hour; every minute while no name is known, unless
//! the trade site refused the request. While no name is known and the trade site answers
//! with an error, or with something the application can't read, the next attempt waits longer
//! and longer, an hour at most: the six requests of a download fail the same way each time.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use super::Refusal;
use super::site::{NamesError, TradeSite};
use crate::files;
use crate::http::Backoff;
use crate::item_names::ItemNames;
use crate::logging;
use crate::worker::{OnDrop, Outbox, Worker};

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
            let listener = Listener { outbox, wake };
            watch(&mut open(), cache.as_deref(), intervals, &listener);
        });
        Self { worker }
    }

    /// Gives the names received since the last call to `handle`, without waiting.
    pub fn receive(&mut self, handle: impl FnMut(ItemNames)) {
        // A thread gone by a panic leaves the names already given.
        self.worker.receive(handle);
    }
}

/// The application, which the thread gives the names to.
struct Listener<'a> {
    outbox: &'a Outbox<ItemNames>,
    /// Wakes the application up, to take the names in.
    wake: &'a dyn Fn(),
}

impl Listener<'_> {
    /// Gives `names` to the application and wakes it up; `false` when it no longer listens.
    fn give(&self, names: ItemNames) -> bool {
        if !self.outbox.send(names) {
            return false;
        }
        (self.wake)();
        true
    }

    /// Waits `duration`; `false` when the application asked the thread to stop meanwhile.
    fn wait(&self, duration: Duration) -> bool {
        self.outbox.wait(duration)
    }
}

/// Reads the names until the application stops the thread or stops listening.
fn watch(
    source: &mut impl NameSource,
    cache: Option<&Path>,
    intervals: Intervals,
    listener: &Listener<'_>,
) {
    let mut due = Instant::now();
    let mut known = false;
    if let Some(names) = cache.and_then(load) {
        let age = names.age(SystemTime::now());
        log::info!("Item names from the cache, {} h old", age.as_secs() / 3600);
        due += intervals.refresh.saturating_sub(age);
        known = true;
        if !listener.give(names) {
            return;
        }
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
                    if !listener.give(names) {
                        return;
                    }
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
        if !listener.wait(due.saturating_duration_since(Instant::now())) {
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
mod tests;
