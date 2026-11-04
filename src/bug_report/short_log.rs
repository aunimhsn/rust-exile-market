//! The log written short for the bug report (`specs.md`, Bug report): each line its time, its
//! level and its message, without the date; the names of the windows other than the game written
//! "another window", and the home folder `~`.

use std::collections::HashSet;
use std::ops::Range;

use crate::hotkey::GAME_CLASS;

/// What the log writes before the name of a window (`The active window, of class steam, isn't
/// the game`, `Copy sent … to the window of class steam_app_2694490 (Ctrl …)`).
const CLASS: &str = "of class ";
/// What may follow the name of a window, which may hold spaces itself (`steam
/// (steamwebhelper)`).
const AFTER_CLASS: [&str; 3] = [", isn't the game", ", is the game", " (Ctrl "];
/// What follows the name of the window taken for the game on Windows, told by its title.
const IS_THE_GAME: &str = ", is the game";
/// Written in place of the name of another window.
pub const ANOTHER_WINDOW: &str = "another window";

/// A level of the log, as written in its lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl Level {
    fn read(word: &str) -> Option<Self> {
        Some(match word {
            "ERROR" => Self::Error,
            "WARN" => Self::Warn,
            "INFO" => Self::Info,
            "DEBUG" => Self::Debug,
            "TRACE" => Self::Trace,
            _ => return None,
        })
    }

    fn word(self) -> &'static str {
        match self {
            Self::Error => "ERROR",
            Self::Warn => "WARN",
            Self::Info => "INFO",
            Self::Debug => "DEBUG",
            Self::Trace => "TRACE",
        }
    }
}

/// A line of the log written short.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogLine {
    /// `18:48:03 INFO Item copied: Item Class: Belts`; a line of a message on several lines as
    /// it is.
    pub text: String,
    /// The level of its message; a line of a message on several lines has the level of its
    /// first line.
    pub level: Option<Level>,
    /// Where "another window" stands in `text`, to draw it apart.
    pub hidden: Option<Range<usize>>,
}

/// The lines of `log`, the text of a log file, written short; `home`, the home folder, written
/// `~`.
#[must_use]
pub fn short_lines(log: &str, home: Option<&str>) -> Vec<LogLine> {
    let games = game_classes(log);
    let mut lines = Vec::new();
    // The level of the message under way, for its next lines.
    let mut level = None;
    for line in log.lines() {
        let line = without_home(line, home);
        let text = match stamped(&line) {
            Some((time, read, message)) => {
                level = read;
                match read {
                    Some(read) => format!("{time} {} {message}", read.word()),
                    None => format!("{time} {message}"),
                }
            }
            None => line,
        };
        let (text, hidden) = without_other_window(text, &games);
        lines.push(LogLine {
            text,
            level,
            hidden,
        });
    }
    lines
}

/// The classes of the windows the log tells as the game: the one of Proton, and on Windows those
/// told by the title of the game.
fn game_classes(log: &str) -> HashSet<&str> {
    let mut games: HashSet<&str> = log
        .lines()
        .filter_map(|line| {
            let (class, after) = class_in(line)?;
            line[after..].starts_with(IS_THE_GAME).then(|| &line[class])
        })
        .collect();
    games.insert(GAME_CLASS);
    games
}

/// Where the name of a window stands in `line`, and where what follows it starts; to the end of
/// the line when nothing known follows it.
fn class_in(line: &str) -> Option<(Range<usize>, usize)> {
    let start = line.find(CLASS)? + CLASS.len();
    let end = AFTER_CLASS
        .iter()
        .filter_map(|after| line[start..].find(after))
        .min()
        .map_or(line.len(), |end| start + end);
    Some((start..end, end))
}

/// `line` with "another window" in place of the name of a window that isn't the game, and where
/// it stands.
fn without_other_window(mut line: String, games: &HashSet<&str>) -> (String, Option<Range<usize>>) {
    let Some((class, _)) = class_in(&line) else {
        return (line, None);
    };
    if games.contains(&line[class.clone()]) {
        return (line, None);
    }
    let hidden = class.start..class.start + ANOTHER_WINDOW.len();
    line.replace_range(class, ANOTHER_WINDOW);
    (line, Some(hidden))
}

/// `line` with `~` in place of the home folder, which names the account of the player; the log
/// writes it so already, but an error of the system may write a path whole. Only the folder
/// itself, followed by a separator or by nothing: `/home/player2` isn't the folder of `player`.
fn without_home(line: &str, home: Option<&str>) -> String {
    let Some(home) = home.map(|home| home.trim_end_matches(['/', '\\'])) else {
        return line.to_owned();
    };
    // A home folder of a letter or two would replace words.
    if home.len() <= 3 {
        return line.to_owned();
    }
    let mut written = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(start) = rest.find(home) {
        let after = &rest[start + home.len()..];
        written.push_str(&rest[..start]);
        if after.is_empty() || after.starts_with(['/', '\\']) {
            written.push('~');
        } else {
            written.push_str(home);
        }
        rest = after;
    }
    written.push_str(rest);
    written
}

/// The time, the level and the message of a line that starts with a date and a time, as the log
/// writes them: `2026-11-04T18:48:03.116775967+01:00 [INFO] Item copied: …`.
fn stamped(line: &str) -> Option<(&str, Option<Level>, &str)> {
    let (stamp, rest) = line.split_once(' ')?;
    let bytes = stamp.as_bytes();
    let is_stamp = bytes.len() >= 19
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[10] == b'T'
        && bytes[13] == b':'
        && bytes[16] == b':';
    if !is_stamp {
        return None;
    }
    let time = stamp.get(11..19)?;
    let level = rest
        .strip_prefix('[')
        .and_then(|rest| rest.split_once("] "))
        .and_then(|(word, message)| Some((Level::read(word.trim())?, message)));
    Some(match level {
        Some((level, message)) => (time, Some(level), message),
        None => (time, None, rest),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(log: &str) -> Vec<String> {
        short_lines(log, Some("/home/player"))
            .into_iter()
            .map(|line| line.text)
            .collect()
    }

    #[test]
    fn a_line_keeps_its_time_its_level_and_its_message() {
        let lines = short_lines(
            "2026-11-04T18:48:03.116775967+01:00 [INFO] Item copied: Item Class: Belts\n\
             2026-11-04T18:48:04.1+01:00 [WARN] The clipboard can't be read: busy",
            None,
        );

        assert_eq!(
            lines[0].text,
            "18:48:03 INFO Item copied: Item Class: Belts"
        );
        assert_eq!(lines[0].level, Some(Level::Info));
        assert_eq!(
            lines[1].text,
            "18:48:04 WARN The clipboard can't be read: busy"
        );
        assert_eq!(lines[1].level, Some(Level::Warn));
    }

    #[test]
    fn a_message_on_several_lines_keeps_the_level_of_its_first_line() {
        let lines = short_lines(
            "2026-11-04T21:02:40.5+01:00 [ERROR] Panic in the main thread: index out of bounds\n\
             at src/game_data/modifiers.rs:412\n\
             2026-11-04T21:02:41.5+01:00 [INFO] Exile Market closed",
            None,
        );

        assert_eq!(lines[1].text, "at src/game_data/modifiers.rs:412");
        assert_eq!(lines[1].level, Some(Level::Error));
        assert_eq!(lines[2].level, Some(Level::Info));
    }

    #[test]
    fn the_class_of_another_window_is_written_another_window() {
        let lines = short_lines(
            "2026-11-04T18:51:09.7+01:00 [INFO] The active window, of class steam \
             (steamwebhelper), isn't the game",
            None,
        );

        assert_eq!(
            lines[0].text,
            "18:51:09 INFO The active window, of class another window, isn't the game"
        );
        let hidden = lines[0].hidden.clone().expect("hidden");
        assert_eq!(&lines[0].text[hidden], ANOTHER_WINDOW);
    }

    #[test]
    fn the_class_of_the_game_stays() {
        let log = "2026-11-04T18:48:03.0+01:00 [INFO] Copy sent by the price-check hotkey to the \
                   window of class steam_app_2694490 (Ctrl pressed by the application)";

        assert_eq!(
            texts(log),
            [
                "18:48:03 INFO Copy sent by the price-check hotkey to the window of class \
              steam_app_2694490 (Ctrl pressed by the application)"
            ]
        );
    }

    #[test]
    fn a_copy_sent_to_another_window_hides_its_class() {
        let log = "2026-11-04T18:48:03.0+01:00 [INFO] Copy sent by the price-check hotkey to the \
                   window of class firefox (Ctrl still held)";

        assert_eq!(
            texts(log),
            [
                "18:48:03 INFO Copy sent by the price-check hotkey to the window of class another \
              window (Ctrl still held)"
            ]
        );
    }

    #[test]
    fn the_class_told_as_the_game_on_windows_stays_in_every_line() {
        let log = "2026-11-04T18:48:01.0+01:00 [INFO] The foreground window, of class POEWindowClass, is the game\n\
                   2026-11-04T18:48:03.0+01:00 [INFO] Copy requested by the price-check hotkey from the window of class POEWindowClass (Ctrl still held)\n\
                   2026-11-04T18:48:09.0+01:00 [INFO] The foreground window, of class Chrome_WidgetWin_1, isn't the game";

        let texts = texts(log);
        assert!(texts[0].contains("of class POEWindowClass, is the game"));
        assert!(texts[1].contains("of class POEWindowClass (Ctrl"));
        assert!(texts[2].contains("of class another window, isn't the game"));
    }

    #[test]
    fn a_class_followed_by_nothing_known_is_hidden_to_the_end_of_the_line() {
        let log = "2026-11-04T18:48:03.0+01:00 [INFO] Something of class discord and more";

        assert_eq!(
            texts(log),
            ["18:48:03 INFO Something of class another window"]
        );
    }

    #[test]
    fn the_home_folder_is_written_with_a_tilde() {
        let log = "2026-11-04T18:48:03.0+01:00 [WARN] /home/player/.cache/exile-market/prices.json \
                   can't be read";

        assert_eq!(
            texts(log),
            ["18:48:03 WARN ~/.cache/exile-market/prices.json can't be read"]
        );
    }

    #[test]
    fn another_home_folder_that_starts_alike_stays() {
        let log = "2026-11-04T18:48:03.0+01:00 [WARN] /home/player2/x and /home/player/y";

        assert_eq!(texts(log), ["18:48:03 WARN /home/player2/x and ~/y"]);
    }

    #[test]
    fn a_line_without_a_date_stays_as_it_is() {
        assert_eq!(texts("Something else"), ["Something else"]);
    }
}
