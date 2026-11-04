//! The bug report (`specs.md`, Bug report): what the application knows (the version, the system,
//! the languages, the league and the settings), the item shown and the last lines of the log of
//! this session or of the previous one, seen by the player in a preview, then opened in the issue
//! form of the repository through its address. Nothing is sent from the application: the player
//! signs in to GitHub and sends it.
//!
//! The report is written in English, the language of the issues, whatever the language of the
//! interface.

mod short_log;
mod system;

use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use directories::BaseDirs;

#[cfg(test)]
pub use self::short_log::ANOTHER_WINDOW;
pub use self::short_log::{Level, LogLine};
use crate::language::Language;
use crate::league::{League, LeagueMode};
use crate::settings::HotkeySetting;

/// The issue form of the repository, `.github/ISSUE_TEMPLATE/bug-report.yml`, whose fields
/// `report`, `item` and `log` the address fills.
const FORM: &str = concat!(
    env!("CARGO_PKG_REPOSITORY"),
    "/issues/new?template=bug-report.yml"
);
/// The longest address opened: GitHub answers an error from about 6,900 characters (measured
/// signed out on 2026-11-04).
const ADDRESS_BUDGET: usize = 6_000;
/// The openings of the bug report so far, which tell each one apart.
static OPENINGS: AtomicU64 = AtomicU64::new(0);
/// A line break, encoded, between two lines of the log.
const ENCODED_LINE_BREAK: usize = "%0A".len();

/// What the application knows, for the summary of the report.
#[derive(Debug, Clone)]
pub struct Facts<'a> {
    /// The language of the interface.
    pub interface: Language,
    /// The language of the last item copied; none before the first one.
    pub items: Option<Language>,
    /// None before the leagues arrive at the first session.
    pub league: Option<&'a League>,
    pub always_on_top: bool,
    /// None on a system without the hotkey (macOS).
    pub hotkey: Option<HotkeySetting>,
}

/// The item the window shows, which the report may join.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinedItem {
    /// Its text as copied from the game.
    pub text: String,
    /// Its name, as the window shows it; none for an item that couldn't be read.
    pub name: Option<String>,
}

/// The session whose log the report sends, one at a time: both would halve the lines of each in
/// the address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogSession {
    This,
    Previous,
}

/// The report, as the preview shows it and as its address sends it.
#[derive(Debug, Clone)]
pub struct Report {
    summary: String,
    item: Option<JoinedItem>,
    /// The log of this session; none when it can't be read.
    log: Option<Vec<LogLine>>,
    /// The log of the previous session; none in the first session, when it couldn't be kept at
    /// the start, when it is empty, or when it can't be read.
    previous_log: Option<Vec<LogLine>>,
    session: LogSession,
    join_item: bool,
    /// The session whose log the player copied last.
    copied: Option<LogSession>,
    sent: Sent,
    /// Which opening of the bug report it is, so that the preview starts again at each one:
    /// its log scrolled to its end.
    opening: u64,
}

/// What the address sends.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Sent {
    address: String,
    /// Whether it holds the item, or its first lines: none fit when the summary takes the whole
    /// budget.
    item: bool,
    /// The first line of the log sent, the lines after it sent too.
    first_line: usize,
}

impl Report {
    /// The report, with the system read now, the log of this session and the one of the previous
    /// session, chosen at once when that session ended without closing (a crash); the item joined
    /// by default.
    #[must_use]
    pub fn read(facts: &Facts<'_>, item: Option<JoinedItem>) -> Self {
        let home = BaseDirs::new().map(|directories| directories.home_dir().display().to_string());
        let short = |log: String| short_log::short_lines(&log, home.as_deref());
        let report = Self::new(
            summary(facts, &system::describe()),
            item,
            crate::logging::path()
                .and_then(|path| {
                    read_log(path)
                        .inspect_err(|error| log::warn!("The log can't be read: {error}"))
                        .ok()
                })
                .map(short),
        );
        let previous = crate::logging::previous_path().and_then(|path| match read_log(&path) {
            Ok(log) => Some(log),
            // The first session.
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => {
                log::warn!("The log of the previous session can't be read: {error}");
                None
            }
        });
        match previous {
            Some(log) => {
                let crashed =
                    short_log::ended_without_closing(&log, &crate::logging::closed_message());
                report.with_previous(short(log), crashed)
            }
            None => report,
        }
    }

    fn new(summary: String, item: Option<JoinedItem>, log: Option<Vec<LogLine>>) -> Self {
        let sent = sent(
            &summary,
            item.as_ref().map(|item| item.text.as_str()),
            log.as_deref().unwrap_or_default(),
            ADDRESS_BUDGET,
        );
        Self {
            summary,
            item,
            log,
            previous_log: None,
            session: LogSession::This,
            join_item: true,
            copied: None,
            sent,
            opening: OPENINGS.fetch_add(1, Ordering::Relaxed),
        }
    }

    /// The report with the log of the previous session, `lines`, chosen at once when that session
    /// `crashed`; a session stopped before its first line has nothing to show, nor to choose.
    fn with_previous(mut self, lines: Vec<LogLine>, crashed: bool) -> Self {
        if lines.iter().all(|line| line.text.trim().is_empty()) {
            return self;
        }
        self.previous_log = Some(lines);
        if crashed {
            self.choose(LogSession::Previous);
        }
        self
    }

    /// The version, the system, the languages, the league and the settings, on five lines.
    #[must_use]
    pub fn summary(&self) -> &str {
        &self.summary
    }

    /// The item the window shows, if any.
    #[must_use]
    pub fn item(&self) -> Option<&JoinedItem> {
        self.item.as_ref()
    }

    /// Whether the player joins the item shown to the report: its checkbox.
    #[must_use]
    pub fn joins_item(&self) -> bool {
        self.join_item && self.item.is_some()
    }

    /// Whether the address holds the item, or its first lines.
    #[must_use]
    pub fn sends_item(&self) -> bool {
        self.sent.item
    }

    /// Which opening of the bug report it is.
    #[must_use]
    pub fn opening(&self) -> u64 {
        self.opening
    }

    /// Whether the log of the previous session can be chosen: not in the first session, nor when
    /// that log couldn't be kept at the start, or is empty.
    #[must_use]
    pub fn has_previous(&self) -> bool {
        self.previous_log.is_some()
    }

    /// The session whose log the report sends.
    #[must_use]
    pub fn session(&self) -> LogSession {
        self.session
    }

    /// Joins the item shown to the report, or leaves it out; the lines of the log that fit
    /// change with it.
    pub fn join_item(&mut self, join: bool) {
        self.join_item = join;
        self.send();
    }

    /// Sends the log of `session`, when it has one; the lines that fit change with it.
    pub fn choose(&mut self, session: LogSession) {
        // Already chosen.
        if session == self.session {
            return;
        }
        // Nothing to choose in the first session.
        if session == LogSession::Previous && !self.has_previous() {
            return;
        }
        self.session = session;
        self.send();
    }

    /// The address and the lines of the log sent, again.
    fn send(&mut self) {
        self.sent = sent(
            &self.summary,
            self.item
                .as_ref()
                .filter(|_| self.join_item)
                .map(|item| item.text.as_str()),
            self.log().unwrap_or_default(),
            ADDRESS_BUDGET,
        );
    }

    /// The log of the session chosen, written short; none when it can't be read.
    fn log(&self) -> Option<&[LogLine]> {
        match self.session {
            LogSession::This => self.log.as_deref(),
            LogSession::Previous => self.previous_log.as_deref(),
        }
    }

    /// The lines of the log the address sends, its last ones; none when the log can't be read.
    #[must_use]
    pub fn log_sent(&self) -> Option<&[LogLine]> {
        self.log().map(|log| &log[self.sent.first_line..])
    }

    /// Whether the log of the session chosen can be read: "Copy the log" is offered. Asked at
    /// each frame, without building the log.
    #[must_use]
    pub fn has_log(&self) -> bool {
        self.log().is_some()
    }

    /// The whole log of the session chosen, written short as the preview shows it, for "Copy the
    /// log"; none when it can't be read.
    #[must_use]
    pub fn whole_log(&self) -> Option<String> {
        self.log().map(|log| {
            let mut whole = String::with_capacity(log.iter().map(|line| line.text.len() + 1).sum());
            for (index, line) in log.iter().enumerate() {
                if index > 0 {
                    whole.push('\n');
                }
                whole.push_str(&line.text);
            }
            whole
        })
    }

    /// Whether the player copied the log of the session chosen: "✓ Copied" until the preview
    /// closes; the other log, not copied yet, offers "Copy the log".
    #[must_use]
    pub fn copied(&self) -> bool {
        self.copied == Some(self.session)
    }

    /// Tells that the player copied the log of the session chosen.
    pub fn mark_copied(&mut self) {
        self.copied = Some(self.session);
    }

    /// The address of the issue form, its fields filled.
    #[must_use]
    pub fn address(&self) -> &str {
        &self.sent.address
    }
}

/// The text of the log file at `path`; a character cut in the middle (a log written while the
/// application stopped) stays readable.
fn read_log(path: &Path) -> io::Result<String> {
    fs::read(path).map(|log| String::from_utf8_lossy(&log).into_owned())
}

/// The summary of the report: the version, the system (`system`), the languages, the league and
/// the settings.
fn summary(facts: &Facts<'_>, system: &str) -> String {
    let languages = match facts.items {
        Some(items) => format!(
            "Interface: {} · Items: {}",
            language_name(facts.interface),
            language_name(items)
        ),
        None => format!("Interface: {}", language_name(facts.interface)),
    };
    let league = match facts.league {
        Some(league) => format!(
            "League: {} ({})",
            league.name(),
            match league.mode() {
                LeagueMode::Softcore => "softcore",
                LeagueMode::Hardcore => "hardcore",
            }
        ),
        None => "League: not known yet".to_owned(),
    };
    let on_off = |on| if on { "on" } else { "off" };
    let mut settings = format!("Always on top: {}", on_off(facts.always_on_top));
    if let Some(hotkey) = facts.hotkey {
        let _ = write!(
            settings,
            " · Hotkey: {}, {}",
            on_off(hotkey.on),
            hotkey.keys
        );
    }
    [
        format!("{} {}", crate::APP_NAME, crate::VERSION),
        system.to_owned(),
        languages,
        league,
        settings,
    ]
    .join("\n")
}

/// The name of `language` in English, the language of the report.
fn language_name(language: Language) -> &'static str {
    match language {
        Language::English => "English",
        Language::French => "French",
    }
}

/// The address that sends `summary`, `item` and the last lines of `log` that fit in `budget`
/// characters: the summary and the item first. An item too long for the address keeps its first
/// lines that fit.
fn sent(summary: &str, item: Option<&str>, log: &[LogLine], budget: usize) -> Sent {
    let mut address = format!("{FORM}&report={}", encode(summary));
    let mut item_sent = false;
    if let Some(item) = item {
        let room = budget.saturating_sub(address.len() + "&item=".len());
        let item = fitting_item(item, room);
        if !item.is_empty() {
            address.push_str("&item=");
            address.push_str(&encode(item));
            item_sent = true;
        }
    }
    let mut room = budget.saturating_sub(address.len() + "&log=".len());
    let mut first_line = log.len();
    for (index, line) in log.iter().enumerate().rev() {
        let cost = encoded_len(&line.text)
            + if first_line == log.len() {
                0
            } else {
                ENCODED_LINE_BREAK
            };
        if cost > room {
            break;
        }
        room -= cost;
        first_line = index;
    }
    if first_line < log.len() {
        let lines: Vec<&str> = log[first_line..]
            .iter()
            .map(|line| line.text.as_str())
            .collect();
        address.push_str("&log=");
        address.push_str(&encode(&lines.join("\n")));
    }
    Sent {
        address,
        item: item_sent,
        first_line,
    }
}

/// The first lines of `item` that fit in `room` encoded characters; all of it when it fits.
fn fitting_item(item: &str, room: usize) -> &str {
    if encoded_len(item) <= room {
        return item;
    }
    log::warn!("The item is too long for the address of the bug report: its first lines only");
    let mut end = 0;
    let mut length = 0;
    for line in item.split_inclusive('\n') {
        // Without its line break, which goes only between two lines.
        let text = line.strip_suffix('\n').unwrap_or(line);
        let cost = if end == 0 {
            encoded_len(text)
        } else {
            ENCODED_LINE_BREAK + encoded_len(text)
        };
        if length + cost > room {
            break;
        }
        length += cost;
        end = if end == 0 {
            text.len()
        } else {
            end + 1 + text.len()
        };
    }
    &item[..end]
}

/// How a byte is written in the query of an address.
enum Written {
    /// As it is: a letter, a digit, and the characters a query may hold that the fields of the
    /// form don't read (RFC 3986), so that the log keeps more lines.
    Kept,
    /// A space, written `+`, which the form reads as a space.
    Plus,
    /// `%XX`: `&`, `=`, `+`, `#`, `%`, the line breaks and the bytes of the other characters.
    Escaped,
}

fn written(byte: u8) -> Written {
    match byte {
        b' ' => Written::Plus,
        b'a'..=b'z'
        | b'A'..=b'Z'
        | b'0'..=b'9'
        | b'-'
        | b'.'
        | b'_'
        | b'~'
        | b':'
        | b'/'
        | b','
        | b'('
        | b')' => Written::Kept,
        _ => Written::Escaped,
    }
}

/// `text` encoded for the query of an address.
fn encode(text: &str) -> String {
    let mut encoded = String::with_capacity(encoded_len(text));
    for byte in text.bytes() {
        match written(byte) {
            Written::Kept => encoded.push(char::from(byte)),
            Written::Plus => encoded.push('+'),
            Written::Escaped => {
                let _ = write!(encoded, "%{byte:02X}");
            }
        }
    }
    encoded
}

/// The length of `text` once encoded.
fn encoded_len(text: &str) -> usize {
    text.bytes()
        .map(|byte| match written(byte) {
            Written::Kept | Written::Plus => 1,
            Written::Escaped => 3,
        })
        .sum()
}

#[cfg(test)]
mod tests;
