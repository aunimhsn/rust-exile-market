//! A thread of the application that sends events to the interface: the clipboard watching, the
//! price database.
//!
//! The interface takes in the events without waiting, and only redraws when one arrives: the
//! thread wakes it up after each one, and when it ends. The interface can also give the thread
//! orders, which end its wait. Dropping the worker stops the thread, and waits for it when it
//! has something to close.

use std::convert::Infallible;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// What the thread gets: where to send its events, and how to wait while listening for the
/// orders of the application, `C`; a thread without orders has `Infallible` ones.
pub struct Outbox<E, C = Infallible> {
    events: Sender<E>,
    /// Closed when the application asks the thread to stop.
    orders: Receiver<C>,
}

/// What ended a wait of the thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Waited<C> {
    /// The whole duration passed.
    Elapsed,
    Order(C),
    /// The application asked the thread to stop.
    Stop,
}

impl<E, C> Outbox<E, C> {
    /// Sends an event; `false` when the application no longer listens.
    #[must_use]
    pub fn send(&self, event: E) -> bool {
        self.events.send(event).is_ok()
    }

    /// Waits `duration`, or less when the application gives an order or asks the thread to
    /// stop.
    #[must_use]
    pub fn wait_for_order(&self, duration: Duration) -> Waited<C> {
        match self.orders.recv_timeout(duration) {
            Ok(order) => Waited::Order(order),
            Err(RecvTimeoutError::Timeout) => Waited::Elapsed,
            Err(RecvTimeoutError::Disconnected) => Waited::Stop,
        }
    }

    /// What the application asked since the last wait, without waiting: the oldest order, or
    /// the stop once every order was taken.
    #[must_use]
    pub fn pending_order(&self) -> Option<Waited<C>> {
        match self.wait_for_order(Duration::ZERO) {
            Waited::Elapsed => None,
            waited => Some(waited),
        }
    }

    /// The last thing the application asked since the last wait, without waiting: its last
    /// order, or the stop.
    #[must_use]
    pub fn latest_order(&self) -> Option<Waited<C>> {
        let mut latest = None;
        while let Some(waited) = self.pending_order() {
            let stop = matches!(waited, Waited::Stop);
            latest = Some(waited);
            if stop {
                break;
            }
        }
        latest
    }
}

impl<E> Outbox<E> {
    /// Waits `duration`; `false` when the application asked the thread to stop meanwhile.
    #[must_use]
    pub fn wait(&self, duration: Duration) -> bool {
        match self.wait_for_order(duration) {
            Waited::Elapsed => true,
            Waited::Stop => false,
        }
    }
}

/// What dropping a worker does with its thread, once told to stop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnDrop {
    /// Waits for it: it has something to close (the clipboard).
    Join,
    /// Lets it end by itself: it may be blocked in a network request it can't leave, and
    /// the application doesn't wait for it to close its window.
    Detach,
}

/// A named thread, for as long as this value lives, which takes orders of type `C`.
#[must_use = "dropping the worker stops its thread"]
pub struct Worker<E, C = Infallible> {
    name: &'static str,
    on_drop: OnDrop,
    events: Receiver<E>,
    /// Dropping it tells the thread to stop.
    orders: Option<Sender<C>>,
    thread: Option<JoinHandle<()>>,
    /// The thread ended without being asked to, which has been told once.
    lost: bool,
}

impl<E: Send + 'static, C: Send + 'static> Worker<E, C> {
    /// Runs `body` on a thread named `name`. `body` gets the `wake` function, to call after
    /// sending events; it is also called when the thread ends, even by a panic.
    ///
    /// # Panics
    ///
    /// If the system can't start a thread.
    pub fn spawn(
        name: &'static str,
        on_drop: OnDrop,
        wake: impl Fn() + Send + 'static,
        body: impl FnOnce(&Outbox<E, C>, &dyn Fn()) + Send + 'static,
    ) -> Self {
        let (events_sender, events) = mpsc::channel();
        let (orders, orders_receiver) = mpsc::channel();
        let thread = thread::Builder::new()
            .name(name.to_owned())
            .spawn(move || {
                let wake = WakeOnExit(wake);
                // Declared after the guard, so dropped before it, even during a panic: the
                // application, once woken up, sees that the thread is gone.
                let outbox = Outbox {
                    events: events_sender,
                    orders: orders_receiver,
                };
                body(&outbox, &wake.0);
            })
            .unwrap_or_else(|error| panic!("the {name} thread starts: {error}"));
        Self {
            name,
            on_drop,
            events,
            orders: Some(orders),
            thread: Some(thread),
            lost: false,
        }
    }

    /// Gives `order` to the thread, which takes it at its next wait; a thread that ended
    /// ignores it.
    pub fn order(&self, order: C) {
        if let Some(orders) = &self.orders {
            let _ = orders.send(order);
        }
    }

    /// Gives each event received since the last call to `handle`, without waiting; `true` the
    /// first time the thread is found gone without being asked to (a panic), after its last
    /// events.
    pub fn receive(&mut self, mut handle: impl FnMut(E)) -> bool {
        loop {
            match self.events.try_recv() {
                Ok(event) => handle(event),
                Err(TryRecvError::Empty) => return false,
                Err(TryRecvError::Disconnected) => {
                    if self.lost {
                        return false;
                    }
                    self.lost = true;
                    log::error!("The {} thread stopped", self.name);
                    return true;
                }
            }
        }
    }
}

impl<E, C> Drop for Worker<E, C> {
    /// Stops the thread, and waits for it unless it is detached.
    fn drop(&mut self) {
        drop(self.orders.take());
        let Some(thread) = self.thread.take() else {
            return;
        };
        match self.on_drop {
            OnDrop::Join => {
                if thread.join().is_err() {
                    log::error!("The {} thread panicked", self.name);
                }
                log::info!("The {} thread ended", self.name);
            }
            OnDrop::Detach => log::info!("The {} thread was told to stop", self.name),
        }
    }
}

/// Calls the function when dropped, even during a panic.
struct WakeOnExit<F: Fn()>(F);

impl<F: Fn()> Drop for WakeOnExit<F> {
    fn drop(&mut self) {
        (self.0)();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A worker whose thread already sent `events` then ended, as after a panic.
    fn ended<E: Send + 'static>(events: impl IntoIterator<Item = E>) -> Worker<E> {
        let (events_sender, receiver) = mpsc::channel();
        for event in events {
            events_sender.send(event).expect("sent");
        }
        Worker {
            name: "test",
            on_drop: OnDrop::Join,
            events: receiver,
            orders: None,
            thread: None,
            lost: false,
        }
    }

    #[test]
    fn the_events_sent_before_the_thread_ended_are_received_first() {
        let mut worker = ended([1, 2]);

        let mut received = Vec::new();
        let lost = worker.receive(|event| received.push(event));

        assert_eq!(received, [1, 2]);
        assert!(lost);
        assert!(!worker.receive(|event| received.push(event)), "told once");
    }

    #[test]
    fn the_thread_stops_when_the_worker_is_dropped() {
        let (stopped, stopped_receiver) = mpsc::channel();
        let worker = Worker::<()>::spawn(
            "test",
            OnDrop::Join,
            || {},
            move |outbox, _| {
                while outbox.wait(Duration::from_millis(10)) {}
                stopped.send(()).expect("sent");
            },
        );

        drop(worker);

        assert_eq!(stopped_receiver.try_recv(), Ok(()));
    }

    #[test]
    fn an_order_ends_the_wait_of_the_thread() {
        let (woken, wakes) = mpsc::channel();
        let mut worker = Worker::<&str, &str>::spawn(
            "test",
            OnDrop::Join,
            move || {
                let _ = woken.send(());
            },
            |outbox, wake| {
                if let Waited::Order(order) = outbox.wait_for_order(Duration::from_secs(60)) {
                    let _ = outbox.send(order);
                    wake();
                }
            },
        );

        worker.order("refresh");

        wakes
            .recv_timeout(Duration::from_secs(5))
            .expect("woken up");
        let mut received = Vec::new();
        worker.receive(|event| received.push(event));
        assert_eq!(received, ["refresh"]);
    }

    #[test]
    fn a_detached_thread_isnt_waited_for() {
        let (release, released) = mpsc::channel::<()>();
        let worker = Worker::<()>::spawn(
            "test",
            OnDrop::Detach,
            || {},
            move |_, _| {
                // Blocked as in a request that doesn't answer.
                let _ = released.recv();
            },
        );

        drop(worker);

        // The drop returned while the thread is still blocked.
        drop(release);
    }

    #[test]
    fn the_application_is_woken_up_when_the_thread_ends() {
        let (woken, wakes) = mpsc::channel();
        let mut worker = Worker::<()>::spawn(
            "test",
            OnDrop::Join,
            move || {
                let _ = woken.send(());
            },
            |_, _| {},
        );

        wakes
            .recv_timeout(Duration::from_secs(5))
            .expect("woken up");
        // The thread ended by itself.
        assert!(worker.receive(|()| {}));
    }
}
