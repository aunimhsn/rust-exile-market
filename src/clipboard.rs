//! Clipboard watching: the items copied in the game, read while the game has the focus.
//!
//! A thread reads the clipboard every [`POLL_INTERVAL`] and sends what changed to the
//! application through a channel, so that the interface never waits for a slow reading
//! (under X11, the application that owns the clipboard has to answer) and only redraws
//! when something happened.

use std::hash::{BuildHasher, RandomState};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use crate::hotkey::COPY_WAIT;
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
const COPY_MARKER: &str = "Exile Market: copying an item from the game";

/// What the watcher tells the application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipboardEvent {
    /// An item was copied, or copied again after another text; by the hotkey, which prepared
    /// the clipboard at that instant ([`crate::hotkey::Press::copy`]), when it was read within
    /// [`COPY_WAIT`] of it.
    Copied {
        item: CopiedItem,
        hotkey: Option<Instant>,
    },
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
    /// When the hotkey prepared its copy: the item read within [`COPY_WAIT`] is a new copy, even
    /// the same.
    copy_expected: CopyExpected,
}

/// When the hotkey last prepared a copy, shared by the hotkey and the watcher.
type CopyExpected = Arc<Mutex<Option<Instant>>>;

impl<S: ClipboardSource> ClipboardWatcher<S> {
    /// The text in the clipboard when the watcher starts counts as new: an item copied
    /// before the application started is shown.
    fn new(source: S, copy_expected: CopyExpected) -> Self {
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
        // A copy prepared during the reading is for the next one: the text read may be from
        // before the marker (a reading under X11 waits for the owner of the clipboard).
        let started = Instant::now();
        match self.source.read() {
            Reading::Text(text) => {
                self.succeeded(&mut emit);
                // The marker of the hotkey is no copy of the player: the item before it and
                // after it is the same copy, unless the hotkey copied it.
                if text == COPY_MARKER {
                    return;
                }
                let text_hash = self.hasher.hash_one(&text);
                let item = CopiedItem::from_text(&text);
                let hotkey = self.copy_expected(started, Instant::now(), item.is_some());
                // Only a different text is a new copy: the same item read again after an
                // empty or late reading is not, unless the hotkey copied it again.
                if self.last_text_hash != Some(text_hash) || (hotkey.is_some() && item.is_some()) {
                    self.last_text_hash = Some(text_hash);
                    if let Some(item) = item {
                        log::info!("Item copied: {}", item.first_line());
                        emit(ClipboardEvent::Copied { item, hotkey });
                    }
                }
            }
            Reading::NoText => self.succeeded(&mut emit),
            Reading::Busy => self.failed("held by another application", &mut emit),
            Reading::Failed(reason) => self.failed(&reason, &mut emit),
        }
    }

    /// When the hotkey prepared a copy before the reading `started`, at most [`COPY_WAIT`]
    /// before `now`; forgotten once an item is `read`, or once too old. A copy prepared during
    /// the reading is kept for the next one.
    fn copy_expected(&self, started: Instant, now: Instant, read: bool) -> Option<Instant> {
        let mut prepared = self
            .copy_expected
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let at = (*prepared)?;
        if at > started {
            return None;
        }
        let expected = now.saturating_duration_since(at) <= COPY_WAIT;
        if read || !expected {
            *prepared = None;
        }
        expected.then_some(at)
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
    copy_expected: CopyExpected,
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
        let copy_expected = CopyExpected::default();
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
#[cfg_attr(
    not(target_os = "linux"),
    allow(dead_code, reason = "the clipboard is only prepared on Linux")
)]
pub struct CopyMarker {
    /// Opened at the first copy, and again until it opens.
    clipboard: Option<arboard::Clipboard>,
    copy_expected: CopyExpected,
}

#[cfg_attr(
    not(target_os = "linux"),
    allow(dead_code, reason = "the clipboard is only prepared on Linux")
)]
impl CopyMarker {
    /// Replaces an item in the clipboard with [`COPY_MARKER`] (any other text, that the player
    /// may need, is kept), then tells the watcher that a copy comes; when, which the watcher
    /// gives back with the item. None when the clipboard couldn't be prepared, or only after
    /// [`COPY_WAIT`] (arboard waits for an owner that doesn't answer): an item left in it would
    /// then be taken for the copy of an empty cell.
    pub fn before_copy(&mut self) -> Option<Instant> {
        let started = Instant::now();
        let marked = self.mark();
        let copy = copy_prepared(marked, started, Instant::now());
        // Replaces the copy a previous hotkey expected, even with none: its item would
        // otherwise be taken for the copy of this one.
        *self
            .copy_expected
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = copy;
        copy
    }

    fn mark(&mut self) -> Result<(), arboard::Error> {
        let clipboard = match &mut self.clipboard {
            Some(clipboard) => clipboard,
            None => self.clipboard.insert(arboard::Clipboard::new()?),
        };
        if needs_marker(clipboard.get_text())? {
            set_marker(clipboard)?;
        }
        Ok(())
    }
}

/// When the clipboard counts as prepared for the copy of the hotkey: `now`, once `marked` from
/// `started` on within [`COPY_WAIT`]; none, written to the log, when it failed or took longer
/// (arboard waits for an owner that doesn't answer), since an item left in it would then be
/// taken for the copy of an empty cell.
#[cfg_attr(
    not(target_os = "linux"),
    allow(dead_code, reason = "the clipboard is only prepared on Linux")
)]
fn copy_prepared(
    marked: Result<(), arboard::Error>,
    started: Instant,
    now: Instant,
) -> Option<Instant> {
    let took = now.saturating_duration_since(started);
    if let Err(error) = marked {
        log::warn!("The clipboard can't be prepared for the copy: {error}");
        None
    } else if took > COPY_WAIT {
        log::warn!(
            "The clipboard took {took:?} to be prepared for the copy: its item only shows in the main window"
        );
        None
    } else {
        Some(now)
    }
}

/// Whether the clipboard, which holds `text`, needs the marker: when it holds an item.
#[cfg_attr(
    not(target_os = "linux"),
    allow(dead_code, reason = "the clipboard is only prepared on Linux")
)]
fn needs_marker(text: Result<String, arboard::Error>) -> Result<bool, arboard::Error> {
    match text {
        Ok(text) => Ok(CopiedItem::from_text(&text).is_some()),
        Err(arboard::Error::ContentNotAvailable | arboard::Error::ConversionFailure) => Ok(false),
        Err(error) => Err(error),
    }
}

/// Writes the marker, out of the history of the clipboard manager (Klipper).
#[cfg(target_os = "linux")]
fn set_marker(clipboard: &mut arboard::Clipboard) -> Result<(), arboard::Error> {
    use arboard::SetExtLinux as _;
    clipboard.set().exclude_from_history().text(COPY_MARKER)
}

#[cfg(not(target_os = "linux"))]
#[allow(dead_code, reason = "the clipboard is only prepared on Linux")]
fn set_marker(clipboard: &mut arboard::Clipboard) -> Result<(), arboard::Error> {
    clipboard.set_text(COPY_MARKER)
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
        ClipboardEvent::Copied {
            item: CopiedItem::from_text(text).expect("an item"),
            hotkey: None,
        }
    }

    /// The events with, for each copy, whether the hotkey copied it, without its instant.
    fn by_hotkey(events: Vec<ClipboardEvent>) -> Vec<(ClipboardEvent, bool)> {
        events
            .into_iter()
            .map(|event| match event {
                ClipboardEvent::Copied { item, hotkey } => (
                    ClipboardEvent::Copied { item, hotkey: None },
                    hotkey.is_some(),
                ),
                event => (event, false),
            })
            .collect()
    }

    fn watcher(readings: impl IntoIterator<Item = Reading>) -> ClipboardWatcher<FakeClipboard> {
        ClipboardWatcher::new(
            FakeClipboard(readings.into_iter().collect()),
            CopyExpected::default(),
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
        expect_copy(&watcher, Instant::now());
        watcher.poll(|event| events.push(event));
        watcher.poll(|event| events.push(event));

        assert_eq!(
            by_hotkey(events),
            [(copied(RING), false), (copied(RING), true)]
        );
        assert_eq!(*watcher.copy_expected.lock().unwrap(), None);
    }

    #[test]
    fn the_copy_of_the_hotkey_carries_the_instant_it_was_prepared() {
        let mut watcher = watcher([text(RING), text(AMULET)]);
        let mut events = Vec::new();
        watcher.poll(|event| events.push(event));
        let prepared = Instant::now();
        expect_copy(&watcher, prepared);
        watcher.poll(|event| events.push(event));

        assert_eq!(
            events.last(),
            Some(&ClipboardEvent::Copied {
                item: CopiedItem::from_text(AMULET).unwrap(),
                hotkey: Some(prepared),
            })
        );
    }

    #[test]
    fn the_marker_then_the_same_item_is_a_new_copy() {
        let mut watcher = watcher([text(RING), text(COPY_MARKER), text(RING)]);
        let mut events = Vec::new();
        watcher.poll(|event| events.push(event));
        expect_copy(&watcher, Instant::now());
        watcher.poll(|event| events.push(event));
        watcher.poll(|event| events.push(event));

        assert_eq!(
            by_hotkey(events),
            [(copied(RING), false), (copied(RING), true)]
        );
    }

    /// The hotkey prepared a copy at `at`.
    fn expect_copy(watcher: &ClipboardWatcher<FakeClipboard>, at: Instant) {
        *watcher.copy_expected.lock().unwrap() = Some(at);
    }

    /// The events of one poll per reading, the hotkey preparing a copy at `at` before the
    /// reading of index `before`.
    fn events_with_copy(
        readings: impl IntoIterator<Item = Reading>,
        before: usize,
        at: Instant,
    ) -> Vec<ClipboardEvent> {
        let readings: Vec<_> = readings.into_iter().collect();
        let polls = readings.len();
        let mut watcher = watcher(readings);
        let mut events = Vec::new();
        for poll in 0..polls {
            if poll == before {
                expect_copy(&watcher, at);
            }
            watcher.poll(|event| events.push(event));
        }
        events
    }

    #[test]
    fn the_same_item_around_the_marker_is_no_new_copy() {
        // The hotkey on an empty cell, then Ctrl+C on the item shown: its filters stay.
        assert_eq!(
            events([text(RING), text(COPY_MARKER), text(RING)]),
            [copied(RING)]
        );
    }

    #[test]
    fn a_copy_expected_at_most_copy_wait_ago_is_taken() {
        let watcher = watcher([]);
        let prepared = Instant::now();
        expect_copy(&watcher, prepared);

        assert_eq!(
            watcher.copy_expected(prepared, prepared + COPY_WAIT, true),
            Some(prepared)
        );
        expect_copy(&watcher, prepared);
        assert_eq!(
            watcher.copy_expected(
                prepared,
                prepared + COPY_WAIT + Duration::from_millis(1),
                true
            ),
            None
        );
    }

    #[test]
    fn a_copy_expected_too_long_ago_is_forgotten() {
        let long_ago = Instant::now()
            .checked_sub(COPY_WAIT + Duration::from_secs(1))
            .expect("an earlier instant");

        assert_eq!(
            events_with_copy([text(RING), text(COPY_MARKER), text(RING)], 1, long_ago),
            [copied(RING)]
        );
    }

    #[test]
    fn the_hotkey_on_an_empty_cell_leaves_the_next_ctrl_c_alone() {
        let long_ago = Instant::now()
            .checked_sub(COPY_WAIT + Duration::from_secs(1))
            .expect("an earlier instant");
        let mut watcher = watcher([text(RING), text(COPY_MARKER), text(COPY_MARKER), text(RING)]);
        let mut events = Vec::new();
        watcher.poll(|event| events.push(event));
        expect_copy(&watcher, Instant::now());
        watcher.poll(|event| events.push(event));
        // Nothing copied in time: the expectation gets old.
        expect_copy(&watcher, long_ago);
        watcher.poll(|event| events.push(event));
        watcher.poll(|event| events.push(event));

        assert_eq!(events, [copied(RING)]);
        assert_eq!(*watcher.copy_expected.lock().unwrap(), None);
    }

    #[test]
    fn a_copy_expected_outlives_busy_and_empty_readings() {
        assert_eq!(
            by_hotkey(events_with_copy(
                [text(RING), Reading::Busy, Reading::NoText, text(RING)],
                1,
                Instant::now()
            )),
            [(copied(RING), false), (copied(RING), true)]
        );
    }

    /// A clipboard whose reading of index `during` lasts while the hotkey prepares a copy: the
    /// text it gives was in the clipboard before the marker.
    struct HotkeyDuringReading {
        readings: VecDeque<Reading>,
        during: usize,
        polls: usize,
        expected: CopyExpected,
    }

    impl ClipboardSource for HotkeyDuringReading {
        fn read(&mut self) -> Reading {
            if self.polls == self.during {
                // Strictly after the start of the reading.
                std::thread::sleep(Duration::from_millis(1));
                *self.expected.lock().unwrap() = Some(Instant::now());
            }
            self.polls += 1;
            self.readings.pop_front().unwrap_or(Reading::NoText)
        }
    }

    /// The events of one poll per reading, the hotkey preparing a copy during the reading of
    /// index `during`.
    fn events_with_copy_during(
        readings: impl IntoIterator<Item = Reading>,
        during: usize,
    ) -> Vec<ClipboardEvent> {
        let readings: VecDeque<_> = readings.into_iter().collect();
        let polls = readings.len();
        let expected = CopyExpected::default();
        let source = HotkeyDuringReading {
            readings,
            during,
            polls: 0,
            expected: Arc::clone(&expected),
        };
        let mut watcher = ClipboardWatcher::new(source, expected);
        let mut events = Vec::new();
        for _ in 0..polls {
            watcher.poll(|event| events.push(event));
        }
        events
    }

    #[test]
    fn the_item_read_while_the_hotkey_marks_an_empty_cell_is_no_copy() {
        assert_eq!(
            events_with_copy_during(
                [text(RING), text(RING), text(COPY_MARKER), text(COPY_MARKER)],
                1
            ),
            [copied(RING)]
        );
    }

    #[test]
    fn the_item_read_while_the_hotkey_marks_another_item_is_no_copy() {
        assert_eq!(
            by_hotkey(events_with_copy_during(
                [text(RING), text(RING), text(COPY_MARKER), text(AMULET)],
                1
            )),
            [(copied(RING), false), (copied(AMULET), true)]
        );
    }

    #[test]
    fn the_same_item_after_a_marker_written_during_a_reading_is_a_copy() {
        assert_eq!(
            by_hotkey(events_with_copy_during(
                [text(RING), text(RING), text(COPY_MARKER), text(RING)],
                1
            )),
            [(copied(RING), false), (copied(RING), true)]
        );
    }

    #[test]
    fn the_clipboard_counts_as_prepared_once_marked_in_time() {
        let started = Instant::now();
        let soon = started + Duration::from_millis(5);
        let late = started + COPY_WAIT + Duration::from_millis(1);

        assert_eq!(copy_prepared(Ok(()), started, soon), Some(soon));
        assert_eq!(copy_prepared(Ok(()), started, late), None);
        let at_the_limit = started + COPY_WAIT;
        assert_eq!(
            copy_prepared(Ok(()), started, at_the_limit),
            Some(at_the_limit)
        );
        assert_eq!(
            copy_prepared(Err(arboard::Error::ClipboardOccupied), started, soon),
            None
        );
    }

    #[test]
    fn a_text_that_isnt_an_item_keeps_the_copy_expected() {
        let prepared = Instant::now();
        assert_eq!(
            by_hotkey(events_with_copy(
                [text(RING), text("a whisper"), text(AMULET)],
                1,
                prepared
            )),
            [(copied(RING), false), (copied(AMULET), true)]
        );
    }

    #[test]
    fn another_item_after_a_copy_expected_too_long_ago_is_a_copy_of_the_player() {
        let long_ago = Instant::now()
            .checked_sub(COPY_WAIT + Duration::from_secs(1))
            .expect("an earlier instant");

        assert_eq!(
            by_hotkey(events_with_copy([text(RING), text(AMULET)], 1, long_ago)),
            [(copied(RING), false), (copied(AMULET), false)]
        );
    }

    #[test]
    fn the_marker_is_not_an_item() {
        assert!(CopiedItem::from_text(COPY_MARKER).is_none());
    }

    #[test]
    fn only_an_item_in_the_clipboard_gets_the_marker() {
        assert!(needs_marker(Ok(RING.to_owned())).unwrap());
        assert!(!needs_marker(Ok("a whisper to keep".to_owned())).unwrap());
        assert!(!needs_marker(Ok(COPY_MARKER.to_owned())).unwrap());
        assert!(!needs_marker(Err(arboard::Error::ContentNotAvailable)).unwrap());
        assert!(needs_marker(Err(arboard::Error::ClipboardOccupied)).is_err());
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
