//! Clipboard watching: the items copied in the game, read while the game has the focus.
//!
//! A thread reads the clipboard every [`POLL_INTERVAL`] and sends what changed to the
//! application through a channel, so that the interface never waits for a slow reading
//! (under X11, the application that owns the clipboard has to answer) and only redraws
//! when something happened.

use std::hash::{BuildHasher, RandomState};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::item::CopiedItem;
use crate::worker::{OnDrop, Outbox, Worker};

/// Fast enough to react before the player looks at the window.
const POLL_INTERVAL: Duration = Duration::from_millis(250);
/// Failed or busy readings in a row before the clipboard counts as unreadable: about one
/// second, so that a single failure doesn't make the warning blink.
const FAILURES_BEFORE_UNREADABLE: u32 = 4;
/// Written in place of an item just before the hotkey copies one, so that the copy of the same
/// item changes the clipboard, and a copy of nothing (an empty cell) leaves no item in it. Not
/// empty: KDE may put the previous text back in an emptied clipboard.
#[cfg_attr(target_os = "macos", allow(dead_code, reason = "no hotkey on macOS"))]
const COPY_MARKER: &str = "Exile Market: copying an item from the game";

/// What the watcher tells the application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipboardEvent {
    /// An item was copied, or copied again after another text.
    Copied(CopiedItem),
    Unreadable,
    /// Readable again, after [`ClipboardEvent::Unreadable`].
    Readable,
}

/// One reading of the clipboard.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Reading {
    Text(String),
    /// Empty, something else than text (an image…), or no answer in time.
    NoText,
    /// Another application holds the clipboard for a moment.
    Busy,
    /// The reason, for the log.
    Failed(String),
}

/// Where the watcher reads the copied texts: the system clipboard, or a fake one in the tests.
trait ClipboardSource {
    fn read(&mut self) -> Reading;
}

/// The clipboard of the system, opened at the first reading, and again until it opens.
#[derive(Default)]
struct SystemClipboard {
    clipboard: Option<arboard::Clipboard>,
}

impl ClipboardSource for SystemClipboard {
    fn read(&mut self) -> Reading {
        let clipboard = match &mut self.clipboard {
            Some(clipboard) => clipboard,
            None => match arboard::Clipboard::new() {
                Ok(clipboard) => self.clipboard.insert(clipboard),
                Err(error) => return Reading::Failed(error.to_string()),
            },
        };
        match clipboard.get_text() {
            Ok(text) => Reading::Text(text),
            Err(arboard::Error::ContentNotAvailable | arboard::Error::ConversionFailure) => {
                Reading::NoText
            }
            Err(arboard::Error::ClipboardOccupied) => Reading::Busy,
            Err(error) => Reading::Failed(error.to_string()),
        }
    }
}

/// Turns the readings of a clipboard into events: a new item, or a change of readability.
struct ClipboardWatcher<S> {
    source: S,
    /// Hashes the texts with a key of its own, so that the watcher can see when the text
    /// changes without keeping a copy of what the player copied outside the game.
    hasher: RandomState,
    last_text_hash: Option<u64>,
    failures_in_a_row: u32,
    readable: bool,
    /// Set by the hotkey before its copy: the next item read is a new copy, even the same.
    copy_expected: Arc<AtomicBool>,
}

impl<S: ClipboardSource> ClipboardWatcher<S> {
    /// The text in the clipboard when the watcher starts counts as new: an item copied
    /// before the application started is shown.
    fn new(source: S, copy_expected: Arc<AtomicBool>) -> Self {
        Self {
            source,
            hasher: RandomState::new(),
            last_text_hash: None,
            failures_in_a_row: 0,
            readable: true,
            copy_expected,
        }
    }

    /// Reads the clipboard once, and gives each event to `emit`.
    fn poll(&mut self, mut emit: impl FnMut(ClipboardEvent)) {
        // Taken before the reading: a copy expected after it is seen at the next one.
        if self.copy_expected.swap(false, Ordering::AcqRel) {
            self.last_text_hash = None;
        }
        match self.source.read() {
            Reading::Text(text) => {
                self.succeeded(&mut emit);
                let text_hash = self.hasher.hash_one(&text);
                // Only a different text is a new copy: the same item read again after an
                // empty or late reading is not.
                if self.last_text_hash != Some(text_hash) {
                    self.last_text_hash = Some(text_hash);
                    if let Some(item) = CopiedItem::from_text(&text) {
                        log::info!("Item copied: {}", item.first_line());
                        emit(ClipboardEvent::Copied(item));
                    }
                }
            }
            Reading::NoText => self.succeeded(&mut emit),
            Reading::Busy => self.failed("held by another application", &mut emit),
            Reading::Failed(reason) => self.failed(&reason, &mut emit),
        }
    }

    fn succeeded(&mut self, emit: &mut impl FnMut(ClipboardEvent)) {
        self.failures_in_a_row = 0;
        if !self.readable {
            self.readable = true;
            log::info!("The clipboard can be read again");
            emit(ClipboardEvent::Readable);
        }
    }

    fn failed(&mut self, reason: &str, emit: &mut impl FnMut(ClipboardEvent)) {
        self.failures_in_a_row += 1;
        if self.readable && self.failures_in_a_row >= FAILURES_BEFORE_UNREADABLE {
            self.readable = false;
            log::warn!("The clipboard can't be read: {reason}");
            emit(ClipboardEvent::Unreadable);
        }
    }
}

/// The clipboard watched on its own thread, for as long as this value lives.
pub struct ClipboardWatch {
    worker: Worker<ClipboardEvent>,
    copy_expected: Arc<AtomicBool>,
}

impl ClipboardWatch {
    /// Starts watching the system clipboard; `wake` is called from the thread after each
    /// event, and when the thread ends, so that the application handles it at once.
    ///
    /// # Panics
    ///
    /// If the system can't start a thread.
    #[must_use]
    pub fn start(wake: impl Fn() + Send + 'static) -> Self {
        Self::spawn(SystemClipboard::default, wake)
    }

    /// Watches the clipboard that `open` gives, on the thread, where it is opened.
    fn spawn<S: ClipboardSource>(
        open: impl FnOnce() -> S + Send + 'static,
        wake: impl Fn() + Send + 'static,
    ) -> Self {
        let copy_expected = Arc::new(AtomicBool::new(false));
        let expected = Arc::clone(&copy_expected);
        let worker = Worker::spawn("clipboard", OnDrop::Join, wake, move |outbox, wake| {
            watch(ClipboardWatcher::new(open(), expected), outbox, wake);
        });
        log::info!("Clipboard watching started");
        Self {
            worker,
            copy_expected,
        }
    }

    /// What prepares the clipboard before each copy of the hotkey.
    #[must_use]
    pub fn copy_marker(&self) -> CopyMarker {
        CopyMarker {
            clipboard: None,
            copy_expected: Arc::clone(&self.copy_expected),
        }
    }

    /// Gives each event received since the last call to `handle`, without waiting. If the
    /// thread ended by itself (a panic), the clipboard counts as unreadable.
    pub fn receive(&mut self, mut handle: impl FnMut(ClipboardEvent)) {
        if self.worker.receive(&mut handle) {
            handle(ClipboardEvent::Unreadable);
        }
    }
}

/// Prepares the clipboard just before the hotkey copies an item, as Exiled Exchange 2 does
/// (`readItemText` in `main/src/shortcuts/HostClipboard.ts`): every copy of the hotkey then
/// reaches the application, even of the item already shown, and the hotkey on an empty cell
/// shows nothing.
#[cfg_attr(target_os = "macos", allow(dead_code, reason = "no hotkey on macOS"))]
pub struct CopyMarker {
    /// Opened at the first copy, and again until it opens.
    clipboard: Option<arboard::Clipboard>,
    copy_expected: Arc<AtomicBool>,
}

#[cfg_attr(target_os = "macos", allow(dead_code, reason = "no hotkey on macOS"))]
impl CopyMarker {
    /// Replaces an item in the clipboard with [`COPY_MARKER`] (any other text, that the player
    /// may need, is kept), then tells the watcher that a copy comes.
    pub fn before_copy(&mut self) {
        if let Err(error) = self.mark() {
            log::warn!("The clipboard can't be prepared for the copy: {error}");
        }
        self.copy_expected.store(true, Ordering::Release);
    }

    fn mark(&mut self) -> Result<(), arboard::Error> {
        let clipboard = match &mut self.clipboard {
            Some(clipboard) => clipboard,
            None => self.clipboard.insert(arboard::Clipboard::new()?),
        };
        match clipboard.get_text() {
            Ok(text) if CopiedItem::from_text(&text).is_some() => clipboard.set_text(COPY_MARKER),
            Ok(_)
            | Err(arboard::Error::ContentNotAvailable | arboard::Error::ConversionFailure) => {
                Ok(())
            }
            Err(error) => Err(error),
        }
    }
}

/// Reads the clipboard until the application stops the thread or stops listening.
fn watch<S: ClipboardSource>(
    mut watcher: ClipboardWatcher<S>,
    outbox: &Outbox<ClipboardEvent>,
    wake: &dyn Fn(),
) {
    loop {
        let mut sent = false;
        let mut listened = true;
        watcher.poll(|event| {
            listened &= outbox.send(event);
            sent = true;
        });
        if !listened {
            return;
        }
        if sent {
            wake();
        }
        if !outbox.wait(POLL_INTERVAL) {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    /// A clipboard that gives prepared readings, one per poll.
    struct FakeClipboard(VecDeque<Reading>);

    impl ClipboardSource for FakeClipboard {
        fn read(&mut self) -> Reading {
            self.0.pop_front().unwrap_or(Reading::NoText)
        }
    }

    const RING: &str = "Item Class: Rings\nRarity: Rare";
    const AMULET: &str = "Item Class: Amulets\nRarity: Unique";
    const STAFF: &str = "Classe d'objet: Bâtons\nRareté: Rare";

    fn text(text: &str) -> Reading {
        Reading::Text(text.to_owned())
    }

    fn failed() -> Reading {
        Reading::Failed("test".to_owned())
    }

    fn copied(text: &str) -> ClipboardEvent {
        ClipboardEvent::Copied(CopiedItem::from_text(text).expect("an item"))
    }

    fn watcher(readings: impl IntoIterator<Item = Reading>) -> ClipboardWatcher<FakeClipboard> {
        ClipboardWatcher::new(
            FakeClipboard(readings.into_iter().collect()),
            Arc::new(AtomicBool::new(false)),
        )
    }

    /// The events of one poll per reading.
    fn events(readings: impl IntoIterator<Item = Reading>) -> Vec<ClipboardEvent> {
        let readings: Vec<_> = readings.into_iter().collect();
        let polls = readings.len();
        let mut watcher = watcher(readings);
        let mut events = Vec::new();
        for _ in 0..polls {
            watcher.poll(|event| events.push(event));
        }
        events
    }

    #[test]
    fn sends_each_copied_item_once() {
        assert_eq!(
            events([text(RING), text(RING), text(AMULET)]),
            [copied(RING), copied(AMULET)]
        );
    }

    #[test]
    fn reads_the_french_items_and_the_windows_line_ends() {
        assert_eq!(
            events([text(&STAFF.replace('\n', "\r\n"))]),
            [copied(STAFF)]
        );
    }

    #[test]
    fn ignores_the_texts_that_are_not_items() {
        assert_eq!(events([text("hello"), Reading::NoText]), []);
    }

    #[test]
    fn sends_an_item_copied_again_after_another_text() {
        assert_eq!(
            events([text(RING), text("@seller Hi"), text(RING)]),
            [copied(RING), copied(RING)]
        );
    }

    #[test]
    fn an_empty_or_late_reading_is_not_a_new_copy() {
        assert_eq!(
            events([text(RING), Reading::NoText, text(RING)]),
            [copied(RING)]
        );
    }

    #[test]
    fn tells_when_the_clipboard_stays_unreadable_then_becomes_readable() {
        let mut readings = vec![failed(); 5];
        readings.push(text(RING));
        assert_eq!(
            events(readings),
            [
                ClipboardEvent::Unreadable,
                ClipboardEvent::Readable,
                copied(RING)
            ]
        );
    }

    #[test]
    fn a_few_failures_or_busy_readings_change_nothing() {
        assert_eq!(
            events([
                text(RING),
                failed(),
                Reading::Busy,
                failed(),
                text(RING),
                failed()
            ]),
            [copied(RING)]
        );
    }

    #[test]
    fn the_clipboard_is_unreadable_from_the_fourth_failure_in_a_row() {
        assert_eq!(events(vec![failed(); 3]), []);
        assert_eq!(events(vec![failed(); 4]), [ClipboardEvent::Unreadable]);
    }

    #[test]
    fn a_clipboard_busy_for_too_long_is_unreadable() {
        assert_eq!(events(vec![Reading::Busy; 4]), [ClipboardEvent::Unreadable]);
    }

    #[test]
    fn a_copy_of_the_hotkey_sends_the_same_item_again() {
        let mut watcher = watcher([text(RING), text(RING), text(RING), text(RING)]);
        let mut events = Vec::new();
        watcher.poll(|event| events.push(event));
        watcher.poll(|event| events.push(event));
        watcher.copy_expected.store(true, Ordering::Release);
        watcher.poll(|event| events.push(event));
        watcher.poll(|event| events.push(event));

        assert_eq!(events, [copied(RING), copied(RING)]);
        assert!(!watcher.copy_expected.load(Ordering::Acquire));
    }

    #[test]
    fn the_marker_then_the_same_item_is_a_new_copy() {
        let mut watcher = watcher([text(RING), text(COPY_MARKER), text(RING)]);
        let mut events = Vec::new();
        watcher.poll(|event| events.push(event));
        watcher.copy_expected.store(true, Ordering::Release);
        watcher.poll(|event| events.push(event));
        watcher.poll(|event| events.push(event));

        assert_eq!(events, [copied(RING), copied(RING)]);
    }

    #[test]
    fn the_marker_is_not_an_item() {
        assert!(CopiedItem::from_text(COPY_MARKER).is_none());
    }

    #[test]
    fn the_thread_sends_the_events_and_wakes_up_the_application() {
        let (woken, wakes) = std::sync::mpsc::channel();
        let mut watch = ClipboardWatch::spawn(
            || FakeClipboard([text(RING)].into()),
            move || {
                let _ = woken.send(());
            },
        );
        wakes
            .recv_timeout(Duration::from_secs(5))
            .expect("the application is woken up");

        let mut received = Vec::new();
        watch.receive(|event| received.push(event));

        assert_eq!(received, [copied(RING)]);
    }

    #[test]
    fn a_panic_of_the_thread_makes_the_clipboard_unreadable_at_once() {
        struct PanickingClipboard;

        impl ClipboardSource for PanickingClipboard {
            fn read(&mut self) -> Reading {
                panic!("a test panic of the clipboard thread");
            }
        }

        let (woken, wakes) = std::sync::mpsc::channel();
        let (handled, handled_receiver) = std::sync::mpsc::channel::<()>();
        let mut watch = ClipboardWatch::spawn(
            || PanickingClipboard,
            move || {
                let _ = woken.send(());
                // Holds the thread in its exit guard until the events are received: the test
                // then sees what the application sees when it is woken up at the earliest.
                let _ = handled_receiver.recv_timeout(Duration::from_secs(5));
            },
        );
        wakes
            .recv_timeout(Duration::from_secs(5))
            .expect("the application is woken up");

        let mut received = Vec::new();
        watch.receive(|event| received.push(event));
        // Before the assertion: if it fails, the join of `Drop` doesn't wait for the time-out.
        drop(handled);
        assert_eq!(received, [ClipboardEvent::Unreadable]);
    }
}
