//! The bug report (`specs.md`, Bug report): what the application knows (the version, the system,
//! the languages, the league and the settings), the item shown and the last lines of the log,
//! seen by the player in a preview, then opened in the issue form of the repository through its
//! address. Nothing is sent from the application: the player signs in to GitHub and sends it.
//!
//! The report is written in English, the language of the issues, whatever the language of the
//! interface.

mod log;
mod system;

use std::fmt::Write as _;
use std::fs;

use directories::BaseDirs;

#[cfg(test)]
pub use self::log::ANOTHER_WINDOW;
pub use self::log::{Level, LogLine};
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

/// The report, as the preview shows it and as its address sends it.
#[derive(Debug, Clone)]
pub struct Report {
    summary: String,
    item: Option<JoinedItem>,
    /// None when the log can't be read.
    log: Option<Vec<LogLine>>,
    join_item: bool,
    sent: Sent,
}

/// What the address sends.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Sent {
    address: String,
    /// The first line of the log sent, the lines after it sent too.
    first_line: usize,
}

impl Report {
    /// The report of this session, with the system read now and the log of this session; the
    /// item joined by default.
    #[must_use]
    pub fn of_this_session(facts: &Facts<'_>, item: Option<JoinedItem>) -> Self {
        let home = BaseDirs::new().map(|directories| directories.home_dir().display().to_string());
        let log = crate::logging::path().and_then(|path| {
            fs::read_to_string(path)
                .inspect_err(|error| ::log::warn!("The log can't be read: {error}"))
                .ok()
        });
        Self::new(
            summary(facts, &system::describe()),
            item,
            log.map(|log| self::log::short_lines(&log, home.as_deref())),
        )
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
            join_item: true,
            sent,
        }
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

    /// Whether the item shown goes with the report.
    #[must_use]
    pub fn joins_item(&self) -> bool {
        self.join_item && self.item.is_some()
    }

    /// Joins the item shown to the report, or leaves it out; the lines of the log that fit
    /// change with it.
    pub fn join_item(&mut self, join: bool) {
        self.join_item = join;
        self.sent = sent(
            &self.summary,
            self.item
                .as_ref()
                .filter(|_| join)
                .map(|item| item.text.as_str()),
            self.log.as_deref().unwrap_or_default(),
            ADDRESS_BUDGET,
        );
    }

    /// The lines of the log the address sends, its last ones; none when the log can't be read.
    #[must_use]
    pub fn log_sent(&self) -> Option<&[LogLine]> {
        self.log.as_deref().map(|log| &log[self.sent.first_line..])
    }

    /// The address of the issue form, its fields filled.
    #[must_use]
    pub fn address(&self) -> &str {
        &self.sent.address
    }
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
    if let Some(item) = item {
        let room = budget.saturating_sub(address.len() + "&item=".len());
        let item = fitting_item(item, room);
        if !item.is_empty() {
            address.push_str("&item=");
            address.push_str(&encode(item));
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
        first_line,
    }
}

/// The first lines of `item` that fit in `room` encoded characters; all of it when it fits.
fn fitting_item(item: &str, room: usize) -> &str {
    if encoded_len(item) <= room {
        return item;
    }
    ::log::warn!("The item is too long for the address of the bug report: its first lines only");
    let mut end = 0;
    for (start, _) in item.match_indices('\n') {
        if encoded_len(&item[..start]) > room {
            break;
        }
        end = start;
    }
    &item[..end]
}

/// Whether `byte` stays as it is in an address: the unreserved characters of RFC 3986.
fn unreserved(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~')
}

/// `text` encoded for the query of an address: every byte but the unreserved ones as `%XX`.
fn encode(text: &str) -> String {
    let mut encoded = String::with_capacity(encoded_len(text));
    for byte in text.bytes() {
        if unreserved(byte) {
            encoded.push(char::from(byte));
        } else {
            let _ = write!(encoded, "%{byte:02X}");
        }
    }
    encoded
}

/// The length of `text` once encoded.
fn encoded_len(text: &str) -> usize {
    text.bytes()
        .map(|byte| if unreserved(byte) { 1 } else { 3 })
        .sum()
}

#[cfg(test)]
mod tests;
