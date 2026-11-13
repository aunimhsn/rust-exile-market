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
