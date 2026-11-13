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
