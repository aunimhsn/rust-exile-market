//! A thread of the application that sends events to the interface: the clipboard watching, the
//! price database.
//!
//! The interface takes in the events without waiting, and only redraws when one arrives: the
//! thread wakes it up after each one, and when it ends. Dropping the worker stops the thread
//! and waits for it.

use std::convert::Infallible;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// What the thread gets: where to send its events, and how to wait while listening for the
/// order to stop.
pub struct Outbox<E> {
    events: Sender<E>,
    stop: Receiver<Infallible>,
}

impl<E> Outbox<E> {
    /// Sends an event; `false` when the application no longer listens.
    #[must_use]
    pub fn send(&self, event: E) -> bool {
        self.events.send(event).is_ok()
    }

    /// Waits `duration`; `false` when the application asked the thread to stop meanwhile.
    #[must_use]
    pub fn wait(&self, duration: Duration) -> bool {
        match self.stop.recv_timeout(duration) {
            Err(RecvTimeoutError::Timeout) => true,
            Err(RecvTimeoutError::Disconnected) => false,
        }
    }
}

/// A named thread, for as long as this value lives.
pub struct Worker<E> {
    name: &'static str,
    events: Receiver<E>,
    /// Carries nothing: dropping it tells the thread to stop.
    stop: Option<Sender<Infallible>>,
    thread: Option<JoinHandle<()>>,
    /// The thread ended without being asked to, which has been told once.
    lost: bool,
}

impl<E: Send + 'static> Worker<E> {
    /// Runs `body` on a thread named `name`. `body` gets the `wake` function, to call after
    /// sending events; it is also called when the thread ends, even by a panic.
    ///
    /// # Panics
    ///
    /// If the system can't start a thread.
    pub fn spawn(
        name: &'static str,
        wake: impl Fn() + Send + 'static,
        body: impl FnOnce(&Outbox<E>, &dyn Fn()) + Send + 'static,
    ) -> Self {
        let (events_sender, events) = mpsc::channel();
        let (stop, stop_receiver) = mpsc::channel();
        let thread = thread::Builder::new()
            .name(name.to_owned())
            .spawn(move || {
                let wake = WakeOnExit(wake);
                // Declared after the guard, so dropped before it, even during a panic: the
                // application, once woken up, sees that the thread is gone.
                let outbox = Outbox {
                    events: events_sender,
                    stop: stop_receiver,
                };
                body(&outbox, &wake.0);
            })
            .unwrap_or_else(|error| panic!("the {name} thread starts: {error}"));
        Self {
            name,
            events,
            stop: Some(stop),
            thread: Some(thread),
            lost: false,
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

impl<E> Drop for Worker<E> {
    /// Stops the thread and waits for it.
    fn drop(&mut self) {
        drop(self.stop.take());
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            log::error!("The {} thread panicked", self.name);
        }
        log::info!("The {} thread ended", self.name);
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
            events: receiver,
            stop: None,
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
    fn the_application_is_woken_up_when_the_thread_ends() {
        let (woken, wakes) = mpsc::channel();
        let mut worker = Worker::<()>::spawn(
            "test",
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
