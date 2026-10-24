//! The price database, read on its own thread: the leagues now, the prices in step 7b.
//!
//! The thread asks for the leagues at startup, then every hour; when the database can't be
//! reached, it tries again every minute, without insisting.

mod poe2scout;

use std::time::Duration;

pub use poe2scout::FetchError;
use poe2scout::Poe2Scout;

use crate::league::{League, Leagues};
use crate::worker::{Outbox, Worker};

const REFRESH_INTERVAL: Duration = Duration::from_hours(1);
const RETRY_INTERVAL: Duration = Duration::from_mins(1);

/// What the price database tells the application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarketEvent {
    Leagues(Leagues),
    /// The database couldn't be read; the application keeps what it has.
    Unreachable,
}

/// Where the thread reads the market: poe2scout, or a fake one in the tests.
trait PriceDatabase {
    /// The current leagues, in the order of the database.
    fn leagues(&mut self) -> Result<Vec<League>, FetchError>;
}

/// The price database read on its own thread, for as long as this value lives.
pub struct MarketWatch {
    worker: Worker<MarketEvent>,
}

impl MarketWatch {
    /// Starts reading poe2scout; `wake` is called from the thread after each event.
    ///
    /// # Panics
    ///
    /// If the system can't start a thread.
    #[must_use]
    pub fn start(wake: impl Fn() + Send + 'static) -> Self {
        Self::spawn(Poe2Scout::new, wake)
    }

    fn spawn<D: PriceDatabase>(
        open: impl FnOnce() -> D + Send + 'static,
        wake: impl Fn() + Send + 'static,
    ) -> Self {
        let worker = Worker::spawn("market", wake, move |outbox, wake| {
            watch(&mut open(), outbox, wake);
        });
        Self { worker }
    }

    /// Gives each event received since the last call to `handle`, without waiting. If the
    /// thread ended by itself (a panic), the database counts as unreachable.
    pub fn receive(&mut self, mut handle: impl FnMut(MarketEvent)) {
        if self.worker.receive(&mut handle) {
            handle(MarketEvent::Unreachable);
        }
    }
}

/// Reads the database until the application stops the thread or stops listening.
fn watch(database: &mut impl PriceDatabase, outbox: &Outbox<MarketEvent>, wake: &dyn Fn()) {
    loop {
        let (event, next) = match database.leagues() {
            Ok(leagues) => {
                log::info!("Leagues: {leagues:?}");
                (
                    MarketEvent::Leagues(Leagues::new(leagues)),
                    REFRESH_INTERVAL,
                )
            }
            Err(error) => {
                log::warn!("The price database can't be reached: {error}");
                (MarketEvent::Unreachable, RETRY_INTERVAL)
            }
        };
        if !outbox.send(event) {
            return;
        }
        wake();
        if !outbox.wait(next) {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    /// A database that gives the same leagues, or fails.
    struct FakeDatabase(Option<Vec<League>>);

    impl PriceDatabase for FakeDatabase {
        fn leagues(&mut self) -> Result<Vec<League>, FetchError> {
            self.0.clone().ok_or_else(|| {
                FetchError::Json(serde_json::from_str::<()>("not json").expect_err("invalid"))
            })
        }
    }

    /// The first event of a thread reading `database`.
    fn first_event(database: FakeDatabase) -> MarketEvent {
        let (woken, wakes) = mpsc::channel();
        let mut watch = MarketWatch::spawn(
            move || database,
            move || {
                let _ = woken.send(());
            },
        );
        wakes
            .recv_timeout(Duration::from_secs(5))
            .expect("the application is woken up");
        let mut received = Vec::new();
        watch.receive(|event| received.push(event));
        received.remove(0)
    }

    #[test]
    fn sends_the_leagues_at_startup() {
        let event = first_event(FakeDatabase(Some(vec![League::new("Forbidden Rites")])));

        assert_eq!(
            event,
            MarketEvent::Leagues(Leagues::new([League::new("Forbidden Rites")]))
        );
    }

    #[test]
    fn tells_when_the_database_cant_be_reached() {
        assert_eq!(first_event(FakeDatabase(None)), MarketEvent::Unreachable);
    }
}
