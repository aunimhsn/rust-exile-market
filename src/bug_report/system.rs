//! The system of the player, for the bug report: Linux with its desktop and its session (X11 or
//! Wayland), Windows or macOS with its version.

/// The system the application runs on, as the bug report writes it: `Linux · KDE Plasma ·
/// Wayland`, `Windows 11 23H2 (build 22631)`, `macOS 15.1`.
#[must_use]
pub fn describe() -> String {
    #[cfg(target_os = "linux")]
    {
        let variable = |name| std::env::var(name).ok();
        linux(
            variable("XDG_CURRENT_DESKTOP").as_deref(),
            variable("XDG_SESSION_TYPE").as_deref(),
            variable("WAYLAND_DISPLAY").is_some(),
        )
    }
    #[cfg(target_os = "windows")]
    {
        windows()
    }
    #[cfg(target_os = "macos")]
    {
        macos()
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    {
        std::env::consts::OS.to_owned()
    }
}

/// Linux with its desktop, `desktop` (`XDG_CURRENT_DESKTOP`, a list joined by `:` such as
/// `ubuntu:GNOME`, the desktop last), and its session, `session` (`XDG_SESSION_TYPE`), or
/// Wayland when `WAYLAND_DISPLAY` is set, `wayland_display`. The window of the application is an
/// X11 window in both, through XWayland under Wayland.
#[cfg(any(target_os = "linux", test))]
fn linux(desktop: Option<&str>, session: Option<&str>, wayland_display: bool) -> String {
    let desktop = desktop
        .and_then(|list| list.rsplit(':').find(|name| !name.is_empty()))
        .map(|name| match name.strip_prefix("X-").unwrap_or(name) {
            "KDE" => "KDE Plasma",
            other => other,
        });
    let session = match session {
        Some(session) if session.eq_ignore_ascii_case("wayland") => "Wayland",
        Some(session) if session.eq_ignore_ascii_case("x11") => "X11",
        _ if wayland_display => "Wayland",
        _ => "X11",
    };
    let mut parts = vec!["Linux"];
    parts.extend(desktop);
    parts.push(session);
    parts.join(" · ")
}

/// Windows with its version, read from the registry: Windows 11 from its build 22000, which its
/// registry still names "Windows 10"; its feature update (`23H2`) when told.
#[cfg(target_os = "windows")]
fn windows() -> String {
    use winsafe::{HKEY, RegistryValue, co};

    let read = |name| match HKEY::LOCAL_MACHINE.RegGetValue(
        Some(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion"),
        Some(name),
        co::RRF::RT_REG_SZ,
    ) {
        Ok(RegistryValue::Sz(value)) => Some(value),
        _ => None,
    };
    windows_named(
        read("CurrentBuild").as_deref(),
        read("DisplayVersion").as_deref(),
    )
}

/// Windows of `build`, with its feature update, `update`.
#[cfg(any(target_os = "windows", test))]
fn windows_named(build: Option<&str>, update: Option<&str>) -> String {
    let Some(build) = build else {
        return "Windows".to_owned();
    };
    let name = match build.parse::<u32>() {
        Ok(number) if number >= 22_000 => "Windows 11",
        Ok(_) => "Windows 10",
        Err(_) => "Windows",
    };
    match update {
        Some(update) => format!("{name} {update} (build {build})"),
        None => format!("{name} (build {build})"),
    }
}

/// macOS with its version, read from the file of the system that tells it.
#[cfg(target_os = "macos")]
fn macos() -> String {
    std::fs::read_to_string("/System/Library/CoreServices/SystemVersion.plist")
        .ok()
        .and_then(|plist| macos_version(&plist).map(str::to_owned))
        .map_or_else(|| "macOS".to_owned(), |version| format!("macOS {version}"))
}

/// The value of `ProductVersion` in the property list `plist`.
#[cfg(any(target_os = "macos", test))]
fn macos_version(plist: &str) -> Option<&str> {
    let after = plist.split_once("<key>ProductVersion</key>")?.1;
    let value = after.split_once("<string>")?.1;
    Some(value.split_once("</string>")?.0.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linux_tells_its_desktop_and_its_session() {
        assert_eq!(
            linux(Some("KDE"), Some("wayland"), true),
            "Linux · KDE Plasma · Wayland"
        );
        assert_eq!(
            linux(Some("ubuntu:GNOME"), Some("x11"), false),
            "Linux · GNOME · X11"
        );
        assert_eq!(
            linux(Some("X-Cinnamon"), None, false),
            "Linux · Cinnamon · X11"
        );
    }

    #[test]
    fn linux_without_a_desktop_tells_its_session_from_the_display() {
        assert_eq!(linux(None, None, true), "Linux · Wayland");
        assert_eq!(linux(Some(""), Some("tty"), false), "Linux · X11");
    }

    #[test]
    fn windows_11_is_told_by_its_build() {
        assert_eq!(
            windows_named(Some("22631"), Some("23H2")),
            "Windows 11 23H2 (build 22631)"
        );
        assert_eq!(
            windows_named(Some("19045"), None),
            "Windows 10 (build 19045)"
        );
        assert_eq!(windows_named(None, Some("23H2")), "Windows");
    }

    #[test]
    fn macos_reads_its_version() {
        let plist = "<dict>\n\t<key>ProductName</key>\n\t<string>macOS</string>\n\
                     \t<key>ProductVersion</key>\n\t<string>15.1</string>\n</dict>";

        assert_eq!(macos_version(plist), Some("15.1"));
        assert_eq!(macos_version("<dict></dict>"), None);
    }
}
