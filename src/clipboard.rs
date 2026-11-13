//! Clipboard watching: the items copied in the game.
//!
//! A thread reads the clipboard every [`POLL_INTERVAL`], whatever the active window (of a text
//! that isn't an item, only a hash is kept, see below), and sends what changed to the
//! application through a channel, so that the interface never waits for a slow reading
//! (under X11, the application that owns the clipboard has to answer) and only redraws
//! when something happened.

use std::hash::{BuildHasher, RandomState};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use crate::hotkey::COPY_WAIT;
use crate::item::CopiedItem;
use crate::logging;
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
                        log::info!("Item copied: {}", logging::plain(item.first_line()));
                        emit(ClipboardEvent::Copied { item, hotkey });
                    } else if CopiedItem::is_too_long_item(&text) {
                        log::warn!(
                            "A copied text of {} bytes starts as an item but is too long to \
                             be one: ignored",
                            text.len()
                        );
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
    /// Never read: under X11, it keeps alive what answers the other applications for the texts
    /// of [`write_text`], as long as the application runs. Closed after the thread of the
    /// watching (the order of the fields). None in the tests, or when it couldn't open.
    _keeps_written_text: Option<arboard::Clipboard>,
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
        let keeps_written_text = arboard::Clipboard::new()
            .inspect_err(|error| log::warn!("The clipboard couldn't be opened: {error}"))
            .ok();
        Self {
            _keeps_written_text: keeps_written_text,
            ..Self::spawn(SystemClipboard::default, wake)
        }
    }

    /// Watches the clipboard that `open` gives, on the thread, where it is opened.
    fn spawn<S: ClipboardSource>(
        open: impl FnOnce() -> S + Send + 'static,
        wake: impl Fn() + Send + 'static,
    ) -> Self {
        let copy_expected = CopyExpected::default();
        let expected = Arc::clone(&copy_expected);
        // Waited for, so that the handle of the thread closes before the one that keeps the
        // written texts: under X11 the last handle hands a text the player copied from the
        // window over to the clipboard manager, which a thread cut by the end of the program
        // wouldn't. The closing waits at most for a reading under way: up to 4 s under X11 when
        // another application holds the clipboard and doesn't answer (`LONG_TIMEOUT_DUR` of
        // arboard 3.7.0), the window still shown; nothing of ours is to hand over then.
        let worker = Worker::spawn("clipboard", OnDrop::Join, wake, move |outbox, wake| {
            watch(ClipboardWatcher::new(open(), expected), outbox, wake);
        });
        log::info!("Clipboard watching started");
        Self {
            worker,
            copy_expected,
            _keeps_written_text: None,
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
    /// thread is found gone without being asked to stop (a panic), the clipboard counts as
    /// unreadable.
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
    not(any(target_os = "linux", target_os = "windows")),
    expect(dead_code, reason = "no hotkey prepares the clipboard on this system")
)]
pub struct CopyMarker {
    /// Opened at the first copy, and again until it opens.
    clipboard: Option<arboard::Clipboard>,
    copy_expected: CopyExpected,
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
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
    all(not(any(target_os = "linux", target_os = "windows")), not(test)),
    expect(dead_code, reason = "no hotkey prepares the clipboard on this system")
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
    all(not(any(target_os = "linux", target_os = "windows")), not(test)),
    expect(dead_code, reason = "no hotkey prepares the clipboard on this system")
)]
fn needs_marker(text: Result<String, arboard::Error>) -> Result<bool, arboard::Error> {
    match text {
        Ok(text) => Ok(CopiedItem::from_text(&text).is_some()),
        Err(arboard::Error::ContentNotAvailable | arboard::Error::ConversionFailure) => Ok(false),
        Err(error) => Err(error),
    }
}

/// Writes `text` to the clipboard of the system, for the player to paste it elsewhere; whether
/// the system took it (the reason of a failure goes to the log, never the text). Rather than
/// `egui::Context::copy_text`, which keeps its result to itself.
///
/// Opened for each copy: under X11 every handle of the application shares one connection and
/// one thread that answers the other applications, kept alive by [`ClipboardWatch`], so that
/// this one closing doesn't end the ownership of the text. Were it the last (the handle of
/// the watch couldn't open), closing it would wait up to 100 ms for the clipboard manager, and
/// without one the text would be lost, though this returned true. Under X11 a success only
/// means that the request was sent. On Windows, arboard tries for about 25 ms when another
/// application holds the clipboard.
#[must_use = "the pill tells the player whether the text was copied"]
pub fn write_text(text: &str) -> bool {
    let written = arboard::Clipboard::new().and_then(|mut clipboard| clipboard.set_text(text));
    if let Err(error) = &written {
        log::warn!("The clipboard couldn't be written: {error}");
    }
    written.is_ok()
}

/// Writes the marker, out of the history of the clipboard manager (Klipper).
#[cfg(target_os = "linux")]
fn set_marker(clipboard: &mut arboard::Clipboard) -> Result<(), arboard::Error> {
    use arboard::SetExtLinux as _;
    clipboard.set().exclude_from_history().text(COPY_MARKER)
}

/// Writes the marker, out of the history of the clipboard (Windows+V), of its copy to the other
/// devices of the player and of the clipboard managers that honour the format of Windows for it
/// (`ExcludeClipboardContentFromMonitorProcessing`): `exclude_from_monitoring`, which arboard
/// advises against combining with the other two, covers them all.
#[cfg(target_os = "windows")]
fn set_marker(clipboard: &mut arboard::Clipboard) -> Result<(), arboard::Error> {
    use arboard::SetExtWindows as _;
    clipboard.set().exclude_from_monitoring().text(COPY_MARKER)
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
mod tests;
