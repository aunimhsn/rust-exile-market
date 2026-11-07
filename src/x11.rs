//! What the hotkey ([`crate::hotkey`]) and the window over the game ([`crate::overlay`]) read
//! alike on the X server: the active window of the desktop, the windows of this application, and
//! where the keyboard is.

use std::error::Error;

use x11rb::connection::Connection;
use x11rb::protocol::xproto::{
    Atom, AtomEnum, ClientMessageEvent, ConnectionExt as _, EventMask, Window,
};
use x11rb::rust_connection::RustConnection;

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// The atoms read on the windows.
#[derive(Debug, Clone, Copy)]
pub struct Atoms {
    /// `_NET_ACTIVE_WINDOW`, the window the desktop gives the focus to.
    pub active_window: Atom,
    /// `_NET_WM_PID`, the process of a window, which winit writes on each of its windows.
    pub pid: Atom,
    /// `_EXILE_MARKET_GAME`, the game the helper of the window over the game carries
    /// ([`crate::overlay`]), while it is shown.
    pub game_behind: Atom,
}

impl Atoms {
    pub fn intern(connection: &RustConnection) -> Result<Self> {
        let active_window = connection
            .intern_atom(false, b"_NET_ACTIVE_WINDOW")?
            .reply()?
            .atom;
        let pid = connection.intern_atom(false, b"_NET_WM_PID")?.reply()?.atom;
        let game_behind = connection
            .intern_atom(false, b"_EXILE_MARKET_GAME")?
            .reply()?
            .atom;
        Ok(Self {
            active_window,
            pid,
            game_behind,
        })
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

/// The class of the helper of the window over the game (`WM_CLASS`: its instance, then its class).
pub const HELPER_CLASS: &[u8] = b"exile-market-helper\0exile-market\0";

/// The game `window` carries, when it is the helper of the window over the game, shown on a single
/// screen ([`crate::overlay`]): a window of this application with the game in
/// `_EXILE_MARKET_GAME`. The hotkey takes it for the game.
pub fn game_behind(
    connection: &RustConnection,
    atoms: Atoms,
    window: Window,
) -> Result<Option<Window>> {
    let game = connection
        .get_property(false, window, atoms.game_behind, AtomEnum::WINDOW, 0, 1)?
        .reply()?
        .value32()
        .and_then(|mut windows| windows.next())
        .filter(|&game| game != x11rb::NONE);
    if game.is_none() || !is_ours(connection, atoms, window)? {
        return Ok(None);
    }
    Ok(game)
}

/// Asks the desktop to make `window` the active window (`_NET_ACTIVE_WINDOW`), as a pager does
/// (source 2), which the focus stealing prevention of `KWin` lets through.
pub fn activate(
    connection: &RustConnection,
    root: Window,
    atoms: Atoms,
    window: Window,
) -> Result<()> {
    let event = ClientMessageEvent::new(32, window, atoms.active_window, [2, 0, 0, 0, 0]);
    connection
        .send_event(
            false,
            root,
            EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
            event,
        )?
        .check()?;
    Ok(())
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
