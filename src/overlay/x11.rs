//! The clicks in the game under X11, while the window over the game is open: the buttons of the
//! mouse read on the X server a few dozen times per second.
//!
//! The game keeps the focus, so that its clicks never reach the application: the pointer of the
//! X server is read instead (`QueryPointer`), which needs no grab and leaves every click to the
//! game, only while the window is open.

use std::time::Duration;

use x11rb::connection::Connection as _;
use x11rb::protocol::xproto::{ConnectionExt as _, KeyButMask, Window};
use x11rb::rust_connection::RustConnection;

use super::Point;
use crate::worker::{OnDrop, Outbox, Waited, Worker};

/// Shorter than a click: a press and its release are about 100 ms apart.
const POLL_INTERVAL: Duration = Duration::from_millis(20);
/// The wait of the thread while nothing is watched: it only ends with an order.
const IDLE_WAIT: Duration = Duration::from_secs(3600);
/// The buttons that count as a click: left, middle and right.
const BUTTONS: [KeyButMask; 3] = [
    KeyButMask::BUTTON1,
    KeyButMask::BUTTON2,
    KeyButMask::BUTTON3,
];

/// The thread that tells the clicks, each where it was pressed, for as long as this value lives.
pub struct ClickWatch {
    worker: Worker<Point, bool>,
    watching: bool,
}

impl ClickWatch {
    /// Starts the thread, idle until [`ClickWatch::watch`]; `wake` is called after each click.
    #[must_use]
    pub fn start(wake: impl Fn() + Send + 'static) -> Self {
        let worker = Worker::spawn("clicks", OnDrop::Detach, wake, |outbox, wake| {
            match Pointer::connect() {
                Ok(pointer) => watch(&pointer, outbox, wake),
                Err(error) => log::warn!(
                    "The clicks in the game can't be read: the window over the game will close with the next item ({error})"
                ),
            }
        });
        Self {
            worker,
            watching: false,
        }
    }

    /// Reads the clicks while `watching`, the window open.
    pub fn watch(&mut self, watching: bool) {
        if self.watching != watching {
            self.watching = watching;
            self.worker.order(watching);
        }
    }

    /// Gives each click since the last call to `handle`, without waiting.
    pub fn receive(&mut self, handle: impl FnMut(Point)) {
        if self.worker.receive(handle) {
            log::warn!("The thread of the clicks in the game stopped");
        }
    }
}

struct Pointer {
    connection: RustConnection,
    root: Window,
}

impl Pointer {
    fn connect() -> Result<Self, Box<dyn std::error::Error>> {
        let (connection, screen) = x11rb::connect(None)?;
        let root = connection.setup().roots[screen].root;
        Ok(Self { connection, root })
    }

    /// Where the pointer is, and whether a button is held.
    fn read(&self) -> Result<(Point, bool), Box<dyn std::error::Error>> {
        let reply = self.connection.query_pointer(self.root)?.reply()?;
        let point = Point {
            x: i32::from(reply.root_x),
            y: i32::from(reply.root_y),
        };
        let down = BUTTONS
            .into_iter()
            .any(|button| reply.mask.contains(button));
        Ok((point, down))
    }
}

/// Sends each press of a button while the application watches, until it stops the thread.
fn watch(pointer: &Pointer, outbox: &Outbox<Point, bool>, wake: &dyn Fn()) {
    let mut watching = false;
    // A button already held when the watch starts is no new click.
    let mut held = true;
    loop {
        match outbox.wait_for_order(if watching { POLL_INTERVAL } else { IDLE_WAIT }) {
            Waited::Stop => return,
            Waited::Order(order) => {
                watching = order;
                held = true;
            }
            Waited::Elapsed => {}
        }
        if !watching {
            continue;
        }
        let (point, down) = match pointer.read() {
            Ok(read) => read,
            Err(error) => {
                log::warn!("The clicks in the game can't be read: {error}");
                return;
            }
        };
        if is_click(held, down) {
            if !outbox.send(point) {
                return;
            }
            wake();
        }
        held = down;
    }
}

/// A button pressed since the last reading, where none was.
fn is_click(held: bool, down: bool) -> bool {
    down && !held
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_click_is_a_button_pressed_where_none_was() {
        assert!(is_click(false, true));
        assert!(!is_click(true, true));
        assert!(!is_click(false, false));
        assert!(!is_click(true, false));
    }
}
