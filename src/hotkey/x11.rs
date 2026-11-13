//! The copy sent to the game under X11: Ctrl+C pressed through the XTest extension, as the
//! player would press it, in the window that has the focus; and the active window followed, so
//! that the hotkey is grabbed only while it is the game.

mod active_window;
mod copy;

pub use active_window::follow_game;
pub use copy::{CopySender, SENT_AT};

use crate::logging;

/// The class of a window from its `WM_CLASS`, an instance name then a class name, each ended
/// by a null byte. Any program names its windows: the class is written to the log on one short
/// line.
fn window_class(property: &[u8]) -> String {
    let names: Vec<_> = property
        .split(|&byte| byte == 0)
        .filter(|name| !name.is_empty())
        .map(String::from_utf8_lossy)
        .collect();
    let class = match names.as_slice() {
        [] => "unknown".to_owned(),
        [instance, class, ..] if instance != class => format!("{class} ({instance})"),
        [name, ..] => name.to_string(),
    };
    logging::plain(&class)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_the_class_of_a_window_and_its_instance_when_it_differs() {
        assert_eq!(
            window_class(b"steam_app_2694490\0steam_app_2694490\0"),
            "steam_app_2694490"
        );
        assert_eq!(
            window_class(b"pathofexile2.exe\0PathOfExile2\0"),
            "PathOfExile2 (pathofexile2.exe)"
        );
        assert_eq!(window_class(b""), "unknown");
        // A window named to write a line of its own in the log.
        assert_eq!(
            window_class(b"a\n12:00:00 [INFO] fake\0b\0"),
            "b (a 12:00:00 [INFO] fake)"
        );
    }

    #[test]
    fn reads_a_class_of_one_name_or_without_its_last_null_byte() {
        assert_eq!(window_class(b"steam\0"), "steam");
        assert_eq!(
            window_class(b"pathofexile2.exe\0PathOfExile2"),
            "PathOfExile2 (pathofexile2.exe)"
        );
    }
}
