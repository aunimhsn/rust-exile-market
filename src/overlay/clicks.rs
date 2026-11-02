//! The clicks in the game under X11, while the window over the game is open: the buttons of the
//! mouse read on the X server a few dozen times per second.
//!
//! The game keeps the focus, so that its clicks never reach the application: the pointer of the
//! X server is read instead (`QueryPointer`), which needs no grab and leaves every click to the
//! game, only while the window is open.

use std::error::Error;
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

/// Where the thread reads the pointer: the X server, or a fake one in the tests.
trait PointerSource {
    /// Where the pointer is, and whether a button is held.
    fn read(&mut self) -> Result<(Point, bool), Box<dyn Error>>;
}

/// What the thread tells the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Read {
    /// A button pressed, where.
    Click(Point),
    /// The pointer can no longer be read (the X server out of reach), which the thread wrote to
    /// the log; the thread then waits to be stopped.
    Unreadable,
}

/// The thread that tells the clicks, each where it was pressed, for as long as this value lives.
pub struct ClickWatch {
    worker: Worker<Read, bool>,
    watching: bool,
    /// The thread stopped: the clicks can no longer be read.
    stopped: bool,
}

impl ClickWatch {
    /// Starts the thread, idle until [`ClickWatch::watch`]; `wake` is called after each click,
    /// and when the thread stops.
    #[must_use]
    pub fn start(wake: impl Fn() + Send + 'static) -> Self {
        Self::spawn(XPointer::connect, wake)
    }

    /// Reads the pointer that `open` gives, on the thread, where it is opened.
    fn spawn<P: PointerSource>(
        open: impl FnOnce() -> Result<P, Box<dyn Error>> + Send + 'static,
        wake: impl Fn() + Send + 'static,
    ) -> Self {
        let worker = Worker::spawn("clicks", OnDrop::Detach, wake, |outbox, wake| {
            let error = match open() {
                Ok(mut pointer) => match watch(&mut pointer, outbox, wake) {
                    Some(error) => error,
                    None => return,
                },
                Err(error) => error,
            };
            log::warn!("The clicks in the game can't be read: {error}");
            if outbox.send(Read::Unreadable) {
                wake();
                // Ends when the application stops it, so that the end isn't taken for a crash.
                while outbox.wait_for_order(IDLE_WAIT) != Waited::Stop {}
            }
        });
        Self {
            worker,
            watching: false,
            stopped: false,
        }
    }

    /// Reads the clicks while `watching`, the window open.
    pub fn watch(&mut self, watching: bool) {
        if self.watching != watching {
            self.watching = watching;
            self.worker.order(watching);
        }
    }

    /// Gives each click since the last call to `handle`, without waiting; `false` once the
    /// clicks can no longer be read (the X server out of reach, or the thread gone), which the
    /// thread or the worker wrote to the log.
    pub fn receive(&mut self, mut handle: impl FnMut(Point)) -> bool {
        let mut unreadable = false;
        let lost = self.worker.receive(|read| match read {
            Read::Click(point) => handle(point),
            Read::Unreadable => unreadable = true,
        });
        if lost || unreadable {
            self.stopped = true;
        }
        !self.stopped
    }
}

struct XPointer {
    connection: RustConnection,
    root: Window,
}

impl XPointer {
    fn connect() -> Result<Self, Box<dyn Error>> {
        let (connection, screen) = x11rb::connect(None)?;
        let root = connection.setup().roots[screen].root;
        Ok(Self { connection, root })
    }
}

impl PointerSource for XPointer {
    fn read(&mut self) -> Result<(Point, bool), Box<dyn Error>> {
        crate::x11::drop_events(&self.connection);
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

/// Sends each press of a button while the application watches, until it stops the thread (or
/// stops listening), or the pointer can't be read: why, then.
fn watch(
    pointer: &mut impl PointerSource,
    outbox: &Outbox<Read, bool>,
    wake: &dyn Fn(),
) -> Option<Box<dyn Error>> {
    let mut watching = false;
    // A button already held when the watch starts is no new click.
    let mut held = true;
    loop {
        match outbox.wait_for_order(if watching { POLL_INTERVAL } else { IDLE_WAIT }) {
            Waited::Stop => return None,
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
            Err(error) => return Some(error),
        };
        if is_click(held, down) {
            if !outbox.send(Read::Click(point)) {
                return None;
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
    use std::collections::VecDeque;
    use std::sync::mpsc;

    /// A pointer that gives prepared readings, then fails.
    struct FakePointer(VecDeque<(Point, bool)>);

    impl PointerSource for FakePointer {
        fn read(&mut self) -> Result<(Point, bool), Box<dyn Error>> {
            self.0.pop_front().ok_or_else(|| "no more readings".into())
        }
    }

    const AT: Point = Point { x: 4000, y: 500 };

    /// The clicks the thread sends for `readings` while watched, until it stops.
    fn clicks(readings: &[bool]) -> Vec<Point> {
        let readings = readings.iter().map(|&down| (AT, down)).collect();
        let (woken, wakes) = mpsc::channel();
        let mut watch = ClickWatch::spawn(
            move || Ok(FakePointer(readings)),
            move || {
                let _ = woken.send(());
            },
        );
        watch.watch(true);
        let mut clicks = Vec::new();
        // Woken after each click, and once more when the readings run out.
        while wakes.recv_timeout(Duration::from_secs(5)).is_ok() {
            if !watch.receive(|point| clicks.push(point)) {
                break;
            }
        }
        clicks
    }

    #[test]
    fn a_click_is_a_button_pressed_where_none_was() {
        assert!(is_click(false, true));
        assert!(!is_click(true, true));
        assert!(!is_click(false, false));
        assert!(!is_click(true, false));
    }

    #[test]
    fn sends_each_press_once_where_it_was() {
        assert_eq!(clicks(&[false, true, true, false, true, false]), [AT, AT]);
    }

    #[test]
    fn a_button_held_when_the_watch_starts_is_no_click() {
        assert_eq!(clicks(&[true, true, false, true]), [AT]);
    }

    /// A pointer read from the test, one reading at a time.
    struct ChannelPointer(mpsc::Receiver<bool>);

    impl PointerSource for ChannelPointer {
        fn read(&mut self) -> Result<(Point, bool), Box<dyn Error>> {
            let down = self.0.recv()?;
            Ok((AT, down))
        }
    }

    #[test]
    fn a_button_held_when_the_watch_starts_again_is_no_click() {
        let (woken, wakes) = mpsc::channel();
        let (readings, pointer) = mpsc::channel::<bool>();
        let mut watch = ClickWatch::spawn(
            move || Ok(ChannelPointer(pointer)),
            move || {
                let _ = woken.send(());
            },
        );
        watch.watch(true);
        readings.send(false).unwrap();
        readings.send(true).unwrap();
        wakes
            .recv_timeout(Duration::from_secs(5))
            .expect("woken up by the click");
        let mut clicks = Vec::new();
        assert!(watch.receive(|point| clicks.push(point)));
        assert_eq!(clicks, [AT]);

        // The window closed by that click, then opened again while the button is still held:
        // the thread may take the first of these readings before or after the orders, and in
        // both orders the button is still held when the watch starts again.
        watch.watch(false);
        watch.watch(true);
        for down in [true, true, false, true] {
            readings.send(down).unwrap();
        }
        // No more readings: the thread then tells that the pointer can't be read, after every
        // click of these readings.
        drop(readings);
        while watch.receive(|point| clicks.push(point)) {
            wakes
                .recv_timeout(Duration::from_secs(10))
                .expect("woken up by a click or by the end of the readings");
        }
        assert_eq!(clicks, [AT, AT]);
    }

    #[test]
    fn tells_once_the_pointer_can_no_longer_be_read() {
        let (woken, wakes) = mpsc::channel();
        let mut watch = ClickWatch::spawn(
            || Ok(FakePointer(VecDeque::new())),
            move || {
                let _ = woken.send(());
            },
        );
        assert!(watch.receive(|_| {}));
        watch.watch(true);
        wakes
            .recv_timeout(Duration::from_secs(5))
            .expect("woken up when the thread stops");

        assert!(!watch.receive(|_| {}));
        assert!(!watch.receive(|_| {}));
    }
}
