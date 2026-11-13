//! What the hotkey ([`crate::hotkey`]) and the window over the game ([`crate::overlay`]) read
//! alike on the X server: the active window of the desktop, the windows of this application, and
//! where the keyboard is.

use std::error::Error;

use x11rb::connection::Connection;
use x11rb::errors::ReplyError;
use x11rb::protocol::ErrorKind;
use x11rb::protocol::xproto::{
    Atom, AtomEnum, ClientMessageEvent, ConnectionExt as _, EventMask, InputFocus,
    QueryPointerReply, Window,
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

/// A connection to the X server, with its root window and the atoms of the application. Each
/// user keeps its own: the threads of the hotkey and the window over the game never wait for one
/// another.
pub struct Session {
    connection: RustConnection,
    root: Window,
    atoms: Atoms,
}

impl Session {
    /// Connects to the X server of `DISPLAY`, on its default screen.
    ///
    /// # Errors
    ///
    /// When the X server can't be reached, or the atoms can't be read.
    pub fn connect() -> Result<Self> {
        let (connection, screen) = x11rb::connect(None)?;
        let root = connection.setup().roots[screen].root;
        let intern = |name: &[u8]| intern(&connection, name);
        let atoms = Atoms {
            active_window: intern(b"_NET_ACTIVE_WINDOW")?,
            pid: intern(b"_NET_WM_PID")?,
            game_behind: intern(b"_EXILE_MARKET_GAME")?,
        };
        Ok(Self {
            connection,
            root,
            atoms,
        })
    }

    /// The connection, for the requests only one user makes.
    #[must_use]
    pub fn connection(&self) -> &RustConnection {
        &self.connection
    }

    /// The root window of the screen.
    #[must_use]
    pub fn root(&self) -> Window {
        self.root
    }

    /// The atoms of the application.
    #[must_use]
    pub fn atoms(&self) -> Atoms {
        self.atoms
    }

    /// The atom of `name`, created when the X server doesn't know it yet.
    pub fn intern(&self, name: &[u8]) -> Result<Atom> {
        intern(&self.connection, name)
    }

    /// The active window of the desktop, none when it gives the focus to a window outside X11.
    pub fn active_window(&self) -> Result<Option<Window>> {
        let window = self
            .connection
            .get_property(
                false,
                self.root,
                self.atoms.active_window,
                AtomEnum::WINDOW,
                0,
                1,
            )?
            .reply()?
            .value32()
            .and_then(|mut windows| windows.next())
            .unwrap_or(x11rb::NONE);
        Ok((window != x11rb::NONE).then_some(window))
    }

    /// The pointer, on the root window: where it is, its child window there and the buttons and
    /// modifiers held. It needs no grab.
    pub fn pointer(&self) -> Result<QueryPointerReply> {
        Ok(self.connection.query_pointer(self.root)?.reply()?)
    }

    /// The window that has the keyboard.
    pub fn focused(&self) -> Result<Window> {
        Ok(self.connection.get_input_focus()?.reply()?.focus)
    }

    /// Whether `window` belongs to this application: its `_NET_WM_PID` is the id of this
    /// process. A window gone isn't.
    pub fn is_ours(&self, window: Window) -> Result<bool> {
        Ok(self.ours_unless_gone(window)?.unwrap_or(false))
    }

    /// Whether `window` belongs to this application, none when it is gone meanwhile.
    pub fn ours_unless_gone(&self, window: Window) -> Result<Option<bool>> {
        let reply = unless_gone(
            self.connection
                .get_property(false, window, self.atoms.pid, AtomEnum::CARDINAL, 0, 1)?
                .reply(),
        )?;
        Ok(reply.map(|reply| {
            reply.value32().and_then(|mut values| values.next()) == Some(std::process::id())
        }))
    }

    /// The game `window` carries, when it is the helper of the window over the game, shown on a
    /// single screen ([`crate::overlay`]): a window of this application with the game in
    /// `_EXILE_MARKET_GAME`. The hotkey takes it for the game.
    pub fn game_behind(&self, window: Window) -> Result<Option<Window>> {
        let Some(reply) = unless_gone(
            self.connection
                .get_property(
                    false,
                    window,
                    self.atoms.game_behind,
                    AtomEnum::WINDOW,
                    0,
                    1,
                )?
                .reply(),
        )?
        else {
            return Ok(None);
        };
        let game = reply
            .value32()
            .and_then(|mut windows| windows.next())
            .filter(|&game| game != x11rb::NONE);
        if game.is_none() || !self.is_ours(window)? {
            return Ok(None);
        }
        Ok(game)
    }

    /// Asks the desktop to make `window` the active window (`_NET_ACTIVE_WINDOW`), as a pager
    /// does (source 2), which the focus stealing prevention of `KWin` lets through.
    pub fn activate(&self, window: Window) -> Result<()> {
        let event = ClientMessageEvent::new(32, window, self.atoms.active_window, [2, 0, 0, 0, 0]);
        self.connection
            .send_event(
                false,
                self.root,
                EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
                event,
            )?
            .check()?;
        Ok(())
    }

    /// Gives the keyboard to `window`; it goes back to the root window if `window` hides, which
    /// [`is_nowhere`] recognizes.
    pub fn set_focus(&self, window: Window) -> Result<()> {
        self.connection
            .set_input_focus(InputFocus::PARENT, window, x11rb::CURRENT_TIME)?
            .check()?;
        Ok(())
    }

    /// Drops the events the X server sends to every client, such as a change of the keyboard
    /// mapping, which are never needed, so that they don't pile up.
    pub fn drop_events(&self) {
        while let Ok(Some(_)) = self.connection.poll_for_event() {}
    }
}

/// The atom of `name` on `connection`, created when the X server doesn't know it yet.
fn intern(connection: &RustConnection, name: &[u8]) -> Result<Atom> {
    Ok(connection.intern_atom(false, name)?.reply()?.atom)
}

/// The reply of a request on a window, none when the window is gone meanwhile (the tree of the
/// windows changes between two requests); a lost connection stays an error.
pub fn unless_gone<T>(reply: std::result::Result<T, ReplyError>) -> Result<Option<T>> {
    match reply {
        Ok(reply) => Ok(Some(reply)),
        Err(ReplyError::X11Error(error))
            if matches!(error.error_kind, ErrorKind::Window | ErrorKind::Drawable) =>
        {
            Ok(None)
        }
        Err(error) => Err(error.into()),
    }
}

/// The class of the helper of the window over the game (`WM_CLASS`: its instance, then its class).
pub const HELPER_CLASS: &[u8] = b"exile-market-helper\0exile-market\0";

/// Whether the keyboard, at `focused`, is in no window: the root window (which gets it when the
/// window that had it hides), or no window. Not the window under the pointer (`PointerRoot`), a
/// mode the window manager chooses, which the keyboard of this application never leaves.
#[must_use]
pub fn is_nowhere(focused: Window, root: Window) -> bool {
    focused == root || focused == x11rb::NONE
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOT: Window = 0x410;

    #[test]
    fn the_root_and_no_window_are_nowhere_the_pointer_root_isnt() {
        assert!(is_nowhere(ROOT, ROOT));
        assert!(is_nowhere(x11rb::NONE, ROOT));
        assert!(!is_nowhere(u32::from(InputFocus::POINTER_ROOT), ROOT));
        assert!(!is_nowhere(0x660_0001, ROOT));
    }
}
