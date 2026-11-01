//! What the hotkey ([`crate::hotkey`]) and the window over the game ([`crate::overlay`]) read
//! alike on the X server: the active window of the desktop, the windows of this application, and
//! where the keyboard is.

use std::error::Error;

use x11rb::connection::Connection;
use x11rb::protocol::xproto::{Atom, AtomEnum, ConnectionExt as _, Window};
use x11rb::rust_connection::RustConnection;

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// The atoms read on the windows.
#[derive(Debug, Clone, Copy)]
pub struct Atoms {
    /// `_NET_ACTIVE_WINDOW`, the window the desktop gives the focus to.
    pub active_window: Atom,
    /// `_NET_WM_PID`, the process of a window, which winit writes on each of its windows.
    pub pid: Atom,
}

impl Atoms {
    pub fn intern(connection: &RustConnection) -> Result<Self> {
        let active_window = connection
            .intern_atom(false, b"_NET_ACTIVE_WINDOW")?
            .reply()?
            .atom;
        let pid = connection.intern_atom(false, b"_NET_WM_PID")?.reply()?.atom;
        Ok(Self { active_window, pid })
    }
}

/// The active window of the desktop, none when it gives the focus to a window outside X11.
pub fn active_window(
    connection: &RustConnection,
    root: Window,
    atoms: Atoms,
) -> Result<Option<Window>> {
    let window = connection
        .get_property(false, root, atoms.active_window, AtomEnum::WINDOW, 0, 1)?
        .reply()?
        .value32()
        .and_then(|mut windows| windows.next())
        .unwrap_or(x11rb::NONE);
    Ok((window != x11rb::NONE).then_some(window))
}

/// Whether `window` belongs to this application: its `_NET_WM_PID` is the id of this process.
pub fn is_ours(connection: &RustConnection, atoms: Atoms, window: Window) -> Result<bool> {
    let owner = connection
        .get_property(false, window, atoms.pid, AtomEnum::CARDINAL, 0, 1)?
        .reply()?
        .value32()
        .and_then(|mut values| values.next());
    Ok(owner == Some(std::process::id()))
}

/// Whether the keyboard, at `focused`, is in no window: the root window (which gets it when the
/// window that had it hides), or no window. Not the window under the pointer (`PointerRoot`), a
/// mode the window manager chooses, which the keyboard of this application never leaves.
#[must_use]
pub fn is_nowhere(focused: Window, root: Window) -> bool {
    focused == root || focused == x11rb::NONE
}

/// Drops the events the X server sends to every client, such as a change of the keyboard
/// mapping, which are never needed, so that they don't pile up.
pub fn drop_events(connection: &RustConnection) {
    while let Ok(Some(_)) = connection.poll_for_event() {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use x11rb::protocol::xproto::InputFocus;

    const ROOT: Window = 0x410;

    #[test]
    fn the_root_and_no_window_are_nowhere_the_pointer_root_isnt() {
        assert!(is_nowhere(ROOT, ROOT));
        assert!(is_nowhere(x11rb::NONE, ROOT));
        assert!(!is_nowhere(u32::from(InputFocus::POINTER_ROOT), ROOT));
        assert!(!is_nowhere(0x660_0001, ROOT));
    }
}
