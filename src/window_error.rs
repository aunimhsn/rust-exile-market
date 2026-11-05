//! The message of a window that can't open (specifications, Error handling; mock-up section
//! "Window can't open (17a)"). The application can't draw anything then: the message goes to a
//! dialog of the system (`MessageBox` on Windows, kdialog or zenity on Linux, which open under
//! Wayland too, an alert on macOS), to the terminal and to the log.

use std::path::Path;
#[cfg(not(target_os = "windows"))]
use std::process::Command;

use crate::i18n::Translator;

/// Why the window couldn't open, as far as the player can act on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cause {
    /// No X11 display: neither X11 nor XWayland (Linux only, where the window always goes
    /// through X11).
    Display,
    /// No OpenGL 2.1, the least the renderer of egui draws with.
    OpenGl,
    /// Anything else, to report.
    Other,
}

impl Cause {
    /// The cause of `error`, returned by `eframe::run_native`.
    #[must_use]
    pub fn of(error: &eframe::Error) -> Self {
        match error {
            // The event loop of winit, forced to X11, fails without its connection to the
            // display (`XNotSupported`): the other systems always have their display.
            eframe::Error::WinitEventLoop(_) if cfg!(target_os = "linux") => Self::Display,
            eframe::Error::Glutin(_)
            | eframe::Error::NoGlutinConfigs(..)
            | eframe::Error::OpenGL(_) => Self::OpenGl,
            _ => Self::Other,
        }
    }
}

/// The text of the message: what failed and what to do, then the log for a cause to report,
/// then the error itself, in paragraphs.
#[must_use]
pub fn message(translator: &Translator, cause: Cause, error: &str, log: Option<&Path>) -> String {
    let mut paragraphs = translator.window_error(cause).to_vec();
    if let (Cause::Other, Some(log)) = (cause, log) {
        paragraphs.push(translator.window_error_log(&log.display().to_string()));
    }
    paragraphs.push(translator.window_error_details(&without_source(error)));
    paragraphs.join("\n\n")
}

/// `error` without the place in the source of winit that a winit error writes (`os error at
/// /home/…/.cargo/…/mod.rs:788: Failed to open connection to X server`), of no use to the player:
/// the log keeps the whole error.
fn without_source(error: &str) -> String {
    let Some(at) = error.find(" at ") else {
        return error.to_owned();
    };
    let place = &error[at + " at ".len()..];
    let Some(rs) = place.find(".rs:") else {
        return error.to_owned();
    };
    let after_line = place[rs + ".rs:".len()..].trim_start_matches(|c: char| c.is_ascii_digit());
    match after_line.strip_prefix(": ") {
        Some(rest) => format!("{}: {rest}", &error[..at]),
        None => error.to_owned(),
    }
}

/// Shows `message` in the terminal, then in a dialog of the system, which waits for the player.
pub fn show(message: &str) {
    eprintln!("{message}");
    if !dialog(message) {
        log::warn!("No dialog of the system could show the message of the window");
    }
}

/// The dialog of Windows; true once shown.
#[cfg(target_os = "windows")]
fn dialog(message: &str) -> bool {
    use winsafe::{HWND, co, prelude::Handle};
    HWND::NULL
        .MessageBox(
            message,
            crate::APP_NAME,
            co::MB::OK | co::MB::ICONERROR | co::MB::SETFOREGROUND,
        )
        .is_ok()
}

/// An alert of macOS, through AppleScript, its texts given as arguments so that nothing in them
/// needs escaping; true once shown.
#[cfg(target_os = "macos")]
fn dialog(message: &str) -> bool {
    Command::new("osascript")
        .args([
            "-e",
            "on run argv",
            "-e",
            "display alert (item 1 of argv) message (item 2 of argv) as critical",
            "-e",
            "end run",
            crate::APP_NAME,
            message,
        ])
        .status()
        .is_ok_and(|status| status.success())
}

/// The first dialog tool of the desktop that runs; true once one has shown it.
#[cfg(target_os = "linux")]
fn dialog(message: &str) -> bool {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    linux_tools(&desktop).into_iter().any(|tool| {
        Command::new(tool.program())
            .args(tool.arguments(message))
            // Closed by the player with OK or the cross alike: any exit tells it was shown.
            .status()
            .is_ok()
    })
}

/// A dialog tool of Linux.
#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tool {
    /// Of KDE (Qt).
    Kdialog,
    /// Of GNOME (GTK), found on most other desktops too.
    Zenity,
}

#[cfg(target_os = "linux")]
impl Tool {
    fn program(self) -> &'static str {
        match self {
            Self::Kdialog => "kdialog",
            Self::Zenity => "zenity",
        }
    }

    fn arguments(self, message: &str) -> Vec<&str> {
        match self {
            Self::Kdialog => vec!["--title", crate::APP_NAME, "--error", message],
            // Without markup: a `<` or a `&` of the error would otherwise break the text.
            Self::Zenity => vec![
                "--error",
                "--no-markup",
                "--title",
                crate::APP_NAME,
                "--text",
                message,
            ],
        }
    }
}

/// The tools tried in order: the one of the desktop first (`XDG_CURRENT_DESKTOP`, e.g. `KDE` or
/// `ubuntu:GNOME`), so that the dialog looks like the others there.
#[cfg(target_os = "linux")]
fn linux_tools(desktop: &str) -> [Tool; 2] {
    if desktop
        .split(':')
        .any(|name| name.eq_ignore_ascii_case("KDE"))
    {
        [Tool::Kdialog, Tool::Zenity]
    } else {
        [Tool::Zenity, Tool::Kdialog]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::language::Language;

    #[test]
    fn a_failed_creation_of_the_application_is_a_cause_to_report() {
        let error = eframe::Error::AppCreation("no state".into());
        assert_eq!(Cause::of(&error), Cause::Other);
    }

    #[test]
    fn the_message_of_a_missing_display_says_what_to_install_then_the_error() {
        let translator = Translator::new(Language::English);
        let text = message(
            &translator,
            Cause::Display,
            "X11 not supported",
            Some(Path::new(
                "/home/player/.local/state/exile-market/exile-market.log",
            )),
        );
        assert_eq!(
            text,
            "Exile Market can't open its window: it needs X11, or XWayland on a Wayland \
             desktop.\n\n\
             Install XWayland (the package xwayland on most distributions), then launch Exile \
             Market again.\n\n\
             Details: X11 not supported"
        );
    }

    #[test]
    fn the_message_of_a_missing_opengl_asks_to_update_the_driver() {
        let translator = Translator::new(Language::French);
        let text = message(&translator, Cause::OpenGl, "no configs", None);
        assert_eq!(
            text,
            "Exile Market ne peut pas ouvrir sa fenêtre\u{a0}: il lui faut OpenGL 2.1 ou plus \
             récent, que le pilote graphique ne fournit pas.\n\n\
             Mettez à jour le pilote graphique, puis relancez Exile Market.\n\n\
             Détails\u{a0}: no configs"
        );
    }

    #[test]
    fn another_cause_asks_to_report_it_with_the_log() {
        let translator = Translator::new(Language::English);
        let text = message(
            &translator,
            Cause::Other,
            "failed",
            Some(Path::new("/logs/exile-market.log")),
        );
        assert_eq!(
            text,
            "Exile Market can't open its window.\n\n\
             Report it on GitHub with its log, so that it can be fixed: \
             https://github.com/aunimhsn/rust-exile-market/issues\n\n\
             Log: /logs/exile-market.log\n\n\
             Details: failed"
        );
    }

    #[test]
    fn the_details_leave_out_the_place_in_the_source_of_winit() {
        assert_eq!(
            without_source(
                "winit EventLoopError: os error at /home/runner/.cargo/registry/src/\
                 winit-0.30.13/src/platform_impl/linux/mod.rs:788: Failed to open connection \
                 to X server"
            ),
            "winit EventLoopError: os error: Failed to open connection to X server"
        );
        assert_eq!(without_source("failed at the start"), "failed at the start");
    }

    #[test]
    fn another_cause_without_a_log_file_still_gives_the_error() {
        let translator = Translator::new(Language::English);
        let text = message(&translator, Cause::Other, "failed", None);
        assert!(!text.contains("Log:"), "{text}");
        assert!(text.ends_with("Details: failed"), "{text}");
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn the_dialog_tool_of_the_desktop_comes_first() {
        assert_eq!(linux_tools("KDE"), [Tool::Kdialog, Tool::Zenity]);
        assert_eq!(linux_tools("ubuntu:GNOME"), [Tool::Zenity, Tool::Kdialog]);
        assert_eq!(linux_tools(""), [Tool::Zenity, Tool::Kdialog]);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn zenity_shows_the_message_without_markup() {
        let arguments = Tool::Zenity.arguments("a < b & c");
        assert!(arguments.contains(&"--no-markup"));
        assert_eq!(arguments.last(), Some(&"a < b & c"));
        assert_eq!(
            Tool::Kdialog.arguments("text"),
            ["--title", "Exile Market", "--error", "text"]
        );
    }
}
