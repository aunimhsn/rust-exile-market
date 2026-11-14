use super::*;
use crate::hotkey::Shortcut;

fn facts(league: Option<&League>) -> Facts<'_> {
    Facts {
        interface: Language::French,
        items: Some(Language::French),
        league,
        always_on_top: false,
        hotkey: Some(HotkeySetting {
            on: true,
            keys: "F4".parse::<Shortcut>().expect("keys"),
        }),
    }
}

/// The line of the log written short as `text`, `18:48:03 INFO …`.
fn line(text: &str) -> LogLine {
    let (time, rest) = text.split_once(' ').expect("a time");
    let (level, message) = rest.split_once(' ').expect("a level");
    let stamped = format!("2026-11-04T{time}.0+01:00 [{level}] {message}");
    let [line] = &short_lines(&stamped, None)[..] else {
        panic!("one line: {text}");
    };
    line.clone()
}

/// `count` lines of the log, numbered, of about 60 characters each once encoded.
fn log(count: usize) -> Vec<LogLine> {
    (0..count)
        .map(|index| {
            line(&format!(
                "18:48:03 INFO Item copied: Item Class: Belts {index:04}"
            ))
        })
        .collect()
}

/// The value of `field` in `address`, decoded.
fn field(address: &str, field: &str) -> Option<String> {
    let query = address.split_once('?')?.1;
    let value = query
        .split('&')
        .find_map(|pair| pair.strip_prefix(&format!("{field}=")))?;
    let bytes = value.as_bytes();
    let mut decoded = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let hex = std::str::from_utf8(&bytes[index + 1..index + 3]).ok()?;
            decoded.push(u8::from_str_radix(hex, 16).ok()?);
            index += 3;
        } else {
            decoded.push(if bytes[index] == b'+' {
                b' '
            } else {
                bytes[index]
            });
            index += 1;
        }
    }
    String::from_utf8(decoded).ok()
}

#[test]
fn the_summary_tells_the_version_the_system_the_languages_the_league_and_the_settings() {
    let league = League::new("Forbidden Rites");

    assert_eq!(
        summary(&facts(Some(&league)), "Linux · KDE Plasma · X11"),
        format!(
            "Exile Market {}\nLinux · KDE Plasma · X11\nInterface: French · Items: French\n\
             League: Forbidden Rites (softcore)\nAlways on top: off · Hotkey: on, F4",
            crate::VERSION
        )
    );
}

#[test]
fn the_summary_before_the_first_item_and_the_leagues() {
    let facts = Facts {
        items: None,
        hotkey: None,
        always_on_top: true,
        ..facts(None)
    };
    let summary = summary(&facts, "macOS 15.1");

    assert!(summary.contains("\nInterface: French\n"), "{summary}");
    assert!(summary.contains("\nLeague: not known yet\n"), "{summary}");
    assert!(summary.ends_with("\nAlways on top: on"), "{summary}");
}

#[test]
fn the_hardcore_league_is_told() {
    let league = League::new("HC Forbidden Rites");

    assert!(summary(&facts(Some(&league)), "Linux").contains("(hardcore)"));
}

#[test]
fn the_address_opens_the_form_with_its_fields_filled() {
    let item = JoinedItem {
        text: "Item Class: Belts\nRarity: Unique\nHeadhunter\nHeavy Belt".to_owned(),
        name: Some("Headhunter".to_owned()),
    };
    let report = Report::new(
        "Exile Market 0.1.0".to_owned(),
        Some(item.clone()),
        Some(log(3)),
    );
    let address = report.address();

    assert!(
        address.starts_with(
            "https://github.com/sylent-dev/rust-exile-market/issues/new?template=bug-report.yml&"
        ),
        "{address}"
    );
    assert_eq!(
        field(address, "report").as_deref(),
        Some("Exile Market 0.1.0")
    );
    assert_eq!(field(address, "item"), Some(item.text));
    let sent = field(address, "log").expect("log");
    assert_eq!(sent.lines().count(), 3);
    assert_eq!(report.log_sent().map(<[LogLine]>::len), Some(3));
}

#[test]
fn the_address_keeps_the_last_lines_of_the_log_that_fit() {
    let report = Report::new("Exile Market 0.1.0".to_owned(), None, Some(log(500)));
    let sent = report.log_sent().expect("log");

    assert!(report.address().len() <= ADDRESS_BUDGET);
    assert!(sent.len() > 50 && sent.len() < 500, "{}", sent.len());
    assert_eq!(sent.last(), log(500).last());
    let decoded = field(report.address(), "log").expect("log");
    assert_eq!(
        decoded,
        sent.iter()
            .map(LogLine::text)
            .collect::<Vec<_>>()
            .join("\n"),
        "the preview shows exactly the lines sent"
    );
}

#[test]
fn leaving_the_item_out_leaves_room_for_more_lines_of_the_log() {
    let item = JoinedItem {
        text: "Item Class: Belts\n".repeat(60),
        name: Some("Headhunter".to_owned()),
    };
    let mut report = Report::new("Exile Market 0.1.0".to_owned(), Some(item), Some(log(500)));
    let with_item = report.log_sent().expect("log").len();
    report.join_item(false);

    assert!(!report.joins_item());
    assert_eq!(field(report.address(), "item"), None);
    assert!(report.log_sent().expect("log").len() > with_item);
}

#[test]
fn an_item_too_long_for_the_address_keeps_its_first_lines() {
    let text = (0..400)
        .map(|index| format!("+{index} to maximum Life"))
        .collect::<Vec<_>>()
        .join("\n");
    let item = JoinedItem {
        text: text.clone(),
        name: None,
    };
    let report = Report::new("Exile Market 0.1.0".to_owned(), Some(item), Some(log(10)));
    let sent = field(report.address(), "item").expect("item");

    assert!(report.address().len() <= ADDRESS_BUDGET);
    assert!(
        text.starts_with(&sent) && sent.ends_with("maximum Life"),
        "{sent}"
    );
    assert_eq!(report.log_sent().map(<[LogLine]>::len), Some(0));
    assert_eq!(field(report.address(), "log"), None);
}

#[test]
fn an_item_that_starts_with_an_empty_line_keeps_whole_lines() {
    let item = "\nRarity: Unique\nHeadhunter";
    let kept = "\nRarity: Unique";

    assert_eq!(fitting_item(item, encoded_len(kept) + 1), kept);
    assert_eq!(fitting_item(item, encoded_len(kept)), kept, "just fitting");
    assert_eq!(
        fitting_item("\n\nHeadhunter\nX", encoded_len("\n\nHeadhunter")),
        "\n\nHeadhunter"
    );
    assert_eq!(fitting_item(item, 0), "");
}

#[test]
fn the_last_lines_of_the_log_that_fit_are_sent() {
    let log = [line("18:48:03 INFO a"), line("18:48:04 INFO b")];
    let last = encoded_len(log[1].text());
    let both = encoded_len(log[0].text()) + ENCODED_LINE_BREAK + last;

    assert_eq!(first_line_that_fits(&log, both), 0, "just fitting");
    assert_eq!(first_line_that_fits(&log, both - 1), 1);
    assert_eq!(
        first_line_that_fits(&log, last - 1),
        2,
        "not even the last one"
    );
    assert_eq!(first_line_that_fits(&[], 100), 0);
}

#[test]
fn without_a_log_the_report_sends_the_summary_and_the_item() {
    let report = Report::new("Exile Market 0.1.0".to_owned(), None, None);

    assert_eq!(report.log_sent(), None);
    assert_eq!(field(report.address(), "log"), None);
    assert!(!report.joins_item(), "no item shown");
}

#[test]
fn the_address_keeps_the_characters_a_query_may_hold() {
    assert_eq!(encode("a-z_0.9~:/,()"), "a-z_0.9~:/,()");
    assert_eq!(encode("a b&c=d+e#f%\n"), "a+b%26c%3Dd%2Be%23f%25%0A");
    assert_eq!(encode("é·"), "%C3%A9%C2%B7");
    assert_eq!(encoded_len("é· a:"), encode("é· a:").len());
}

#[test]
fn an_item_that_doesnt_fit_at_all_isnt_sent() {
    let item = JoinedItem {
        text: "x".repeat(ADDRESS_BUDGET),
        name: Some("Headhunter".to_owned()),
    };
    let report = Report::new("Exile Market 0.1.0".to_owned(), Some(item), Some(log(3)));

    assert!(report.joins_item(), "still checked");
    assert!(!report.sends_item());
    assert_eq!(field(report.address(), "item"), None);
}

/// `count` lines of the log of the previous session, told apart from those of [`log`].
fn previous_log(count: usize) -> Vec<LogLine> {
    (0..count)
        .map(|index| line(&format!("21:02:11 INFO Price of Headhunter: {index} ex")))
        .collect()
}

#[test]
fn the_first_session_offers_no_choice_of_the_log() {
    let mut report = Report::new("Exile Market 0.1.0".to_owned(), None, Some(log(3)));
    report.choose(LogSession::Previous);

    assert!(!report.has_previous());
    assert_eq!(report.session(), LogSession::This, "nothing to choose");
}

#[test]
fn the_previous_log_is_chosen_at_once_after_a_crash() {
    let crashed = Report::new("Exile Market 0.1.0".to_owned(), None, Some(log(3)))
        .with_previous(previous_log(2), true);
    let closed = Report::new("Exile Market 0.1.0".to_owned(), None, Some(log(3)))
        .with_previous(previous_log(2), false);

    assert!(crashed.has_previous());
    assert_eq!(crashed.session(), LogSession::Previous);
    assert_eq!(crashed.log_sent(), Some(&previous_log(2)[..]));
    assert_eq!(
        field(crashed.address(), "log").as_deref(),
        Some("21:02:11 INFO Price of Headhunter: 0 ex\n21:02:11 INFO Price of Headhunter: 1 ex")
    );
    assert!(closed.has_previous());
    assert_eq!(closed.session(), LogSession::This);
    assert_eq!(closed.log_sent(), Some(&log(3)[..]));
}

#[test]
fn one_log_is_sent_at_a_time() {
    let mut report = Report::new("Exile Market 0.1.0".to_owned(), None, Some(log(500)))
        .with_previous(previous_log(3), false);
    report.choose(LogSession::Previous);

    assert_eq!(report.log_sent(), Some(&previous_log(3)[..]));
    let sent = field(report.address(), "log").expect("log");
    assert!(!sent.contains("Item copied"), "{sent}");

    report.choose(LogSession::This);
    let this = report.log_sent().expect("log");
    assert!(
        this.len() > 3 && this.len() < 500,
        "its last lines that fit"
    );
    assert_eq!(this.last(), log(500).last());
}

#[test]
fn copy_the_log_copies_the_whole_log_of_the_session_chosen() {
    let mut report = Report::new("Exile Market 0.1.0".to_owned(), None, Some(log(500)))
        .with_previous(previous_log(2), false);
    let whole = report.whole_log().expect("log");

    assert!(report.has_log());
    assert_eq!(whole.lines().count(), 500, "beyond the lines sent");
    assert!(whole.starts_with("18:48:03 INFO Item copied: Item Class: Belts 0000\n"));
    assert!(whole.ends_with("Belts 0499"));

    report.choose(LogSession::Previous);
    assert_eq!(
        report.whole_log().as_deref(),
        Some("21:02:11 INFO Price of Headhunter: 0 ex\n21:02:11 INFO Price of Headhunter: 1 ex")
    );
}

#[test]
fn a_refused_log_never_reads_copied_and_can_be_tried_again() {
    let mut report = Report::new("Exile Market 0.1.0".to_owned(), None, Some(log(3)))
        .with_previous(previous_log(2), false);
    report.copy_result(false);

    assert!(report.copy_failed());
    assert!(!report.copied());

    report.choose(LogSession::Previous);
    assert!(!report.copy_failed(), "another log, not tried yet");

    report.choose(LogSession::This);
    assert!(report.copy_failed(), "back to the log refused");

    report.copy_result(true);
    assert!(report.copied());
    assert!(!report.copy_failed());
}

#[test]
fn a_log_refused_leaves_the_other_log_copied() {
    let mut report = Report::new("Exile Market 0.1.0".to_owned(), None, Some(log(3)))
        .with_previous(previous_log(2), false);
    report.copy_result(true);

    report.choose(LogSession::Previous);
    report.copy_result(false);
    assert!(report.copy_failed());
    assert!(!report.copied());

    report.choose(LogSession::This);
    assert!(report.copied(), "still in the clipboard");
    assert!(!report.copy_failed());
}

#[test]
fn copied_while_the_log_copied_is_chosen() {
    let mut report = Report::new("Exile Market 0.1.0".to_owned(), None, Some(log(3)))
        .with_previous(previous_log(2), false);
    report.copy_result(true);
    report.join_item(false);

    assert!(report.copied(), "the same log");

    report.choose(LogSession::Previous);
    assert!(!report.copied(), "another log, not copied yet");

    report.choose(LogSession::This);
    assert!(report.copied(), "back to the log copied");
}

#[test]
fn without_a_log_nothing_is_copied() {
    let report = Report::new("Exile Market 0.1.0".to_owned(), None, None);

    assert!(!report.has_log());
    assert_eq!(report.whole_log(), None);
}

#[test]
fn an_empty_previous_log_offers_no_choice() {
    let mut report = Report::new("Exile Market 0.1.0".to_owned(), None, Some(log(3)))
        .with_previous(short_lines("\n  ", None), true);
    report.choose(LogSession::Previous);

    assert!(!report.has_previous());
    assert_eq!(report.session(), LogSession::This);
}

#[test]
fn each_opening_is_told_apart() {
    let first = Report::new(String::new(), None, None);
    let second = Report::new(String::new(), None, None);

    assert_ne!(first.opening(), second.opening());
}

/// The sentences of the log that write the name of a window, as the bug report reads them
/// (`short_log`): a sentence written otherwise would show the name of another window.
#[test]
fn the_log_writes_the_windows_as_the_bug_report_reads_them() {
    let x11_active_window = include_str!("../hotkey/x11/active_window.rs");
    let x11_copy = include_str!("../hotkey/x11/copy.rs");
    let windows = include_str!("../hotkey/windows.rs");
    for (source, sentence) in [
        (x11_active_window, "of class {class}, isn't the game"),
        (x11_active_window, "of class {class}, is the game"),
        (x11_copy, "to the window of class {} (Ctrl {}"),
        (windows, "of class {class}, isn't the game"),
        (windows, "of class {class}, is the game"),
        (windows, "from the window of class {class} (Ctrl {}"),
    ] {
        assert!(source.contains(sentence), "{sentence}");
    }
    for source in [x11_active_window, x11_copy, windows] {
        assert_eq!(
            source.matches("of class ").count(),
            source.matches("of class {").count(),
            "every class written is read"
        );
    }
}

#[test]
fn a_log_that_fits_its_budget_is_read_whole() {
    let log = "line 1\nline 2\n";

    assert_eq!(
        read_end(io::Cursor::new(log), 14).expect("read"),
        "line 1\nline 2\n"
    );
    assert_eq!(read_end(io::Cursor::new(""), 14).expect("read"), "");
}

#[test]
fn a_longer_log_keeps_its_last_whole_lines() {
    let log = "aaa\nbbb\nccc\n";

    // From a line break: the line after it is whole.
    assert_eq!(
        read_end(io::Cursor::new(log), 8).expect("read"),
        "bbb\nccc\n"
    );
    // Inside a line: the line cut is left out.
    assert_eq!(read_end(io::Cursor::new(log), 7).expect("read"), "ccc\n");
    assert_eq!(read_end(io::Cursor::new(log), 2).expect("read"), "");
}

#[test]
fn a_character_cut_by_the_budget_is_left_out_with_its_line() {
    let log = "été\nok";

    // Its last byte alone is read: the line break and what follows it fit.
    assert_eq!(read_end(io::Cursor::new(log), 3).expect("read"), "ok");
}
