//! The desktop of the X server behind [`Screen`]: the helper, the active window, the keyboard
//! focus, and the window Wine holds the pointer in.

use x11rb::connection::Connection as _;
use x11rb::properties::{WmSizeHints, WmSizeHintsSpecification};
use x11rb::protocol::xproto::{
    AtomEnum, ConfigureWindowAux, ConnectionExt as _, CreateWindowAux, EventMask,
    GetWindowAttributesReply, GrabMode, GrabStatus, InputFocus, KeyButMask, MapState, PropMode,
    Window, WindowClass,
};
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as _;

use crate::x11::{self as shared, Atoms, Result, unless_gone};

use super::Screen;

/// Whether a window at `position`, of `size`, covers the whole screen of X11, of the size
/// `screen`: the window Wine holds the pointer in does.
fn covers_screen(position: (i16, i16), size: (u16, u16), screen: (u16, u16)) -> bool {
    position.0 <= 0 && position.1 <= 0 && size.0 >= screen.0 && size.1 >= screen.1
}

/// The windows above `window` among `children`, the children of the root window from the
/// bottom of the stack to its top; none when it isn't one of them.
fn above<T>(children: &[(Window, T)], window: Window) -> &[(Window, T)] {
    children
        .iter()
        .position(|&(child, _)| child == window)
        .and_then(|at| children.get(at + 1..))
        .unwrap_or_default()
}

/// Whether a window with these attributes can be the one Wine holds the pointer in: never
/// drawn, which only takes the input; not managed by the window manager; and shown.
fn may_hold_the_pointer(class: WindowClass, override_redirect: bool, map_state: MapState) -> bool {
    class == WindowClass::INPUT_ONLY && override_redirect && map_state == MapState::VIEWABLE
}

/// The desktop on the X server.
pub struct XScreen {
    connection: RustConnection,
    root: Window,
    atoms: Atoms,
    /// The helper, created when it is first shown.
    helper: Option<Window>,
}

impl XScreen {
    pub(super) fn connect() -> Result<Self> {
        let (connection, screen) = x11rb::connect(None)?;
        let root = connection.setup().roots[screen].root;
        let atoms = Atoms::intern(&connection)?;
        Ok(Self {
            connection,
            root,
            atoms,
            helper: None,
        })
    }

    /// The helper: managed by the desktop, without decorations, invisible, out of the taskbar and
    /// the pager, of this process, and that the desktop can't close.
    fn create_helper(&mut self) -> Result<Window> {
        if let Some(helper) = self.helper {
            return Ok(helper);
        }
        let connection = &self.connection;
        let intern =
            |name: &[u8]| -> Result<_> { Ok(connection.intern_atom(false, name)?.reply()?.atom) };
        let helper = connection.generate_id()?;
        connection
            .create_window(
                x11rb::COPY_DEPTH_FROM_PARENT,
                helper,
                self.root,
                0,
                0,
                1,
                1,
                0,
                WindowClass::INPUT_OUTPUT,
                x11rb::COPY_FROM_PARENT,
                &CreateWindowAux::new().background_pixel(0),
            )?
            .check()?;
        connection.change_property8(
            PropMode::REPLACE,
            helper,
            AtomEnum::WM_CLASS,
            AtomEnum::STRING,
            shared::HELPER_CLASS,
        )?;
        connection.change_property8(
            PropMode::REPLACE,
            helper,
            AtomEnum::WM_NAME,
            AtomEnum::STRING,
            crate::APP_NAME.as_bytes(),
        )?;
        connection.change_property32(
            PropMode::REPLACE,
            helper,
            self.atoms.pid,
            AtomEnum::CARDINAL,
            &[std::process::id()],
        )?;
        // Neither functions nor decorations (the flags say both are given, then none): Alt+F4
        // while the helper is the active window can't close it, which `KWin` would do by killing
        // the client of a window that can't be asked to close.
        let motif = intern(b"_MOTIF_WM_HINTS")?;
        connection.change_property32(PropMode::REPLACE, helper, motif, motif, &[3, 0, 0, 0, 0])?;
        // Asked to close all the same, it is asked rather than killed (the event is dropped).
        connection.change_property32(
            PropMode::REPLACE,
            helper,
            intern(b"WM_PROTOCOLS")?,
            AtomEnum::ATOM,
            &[intern(b"WM_DELETE_WINDOW")?],
        )?;
        // Its pixel never shows, wherever the window over the game moves above it.
        connection.change_property32(
            PropMode::REPLACE,
            helper,
            intern(b"_NET_WM_WINDOW_OPACITY")?,
            AtomEnum::CARDINAL,
            &[0],
        )?;
        let skip = [
            intern(b"_NET_WM_STATE_SKIP_TASKBAR")?,
            intern(b"_NET_WM_STATE_SKIP_PAGER")?,
        ];
        connection.change_property32(
            PropMode::REPLACE,
            helper,
            intern(b"_NET_WM_STATE")?,
            AtomEnum::ATOM,
            &skip,
        )?;
        self.helper = Some(helper);
        Ok(helper)
    }

    /// The children of the root window from the bottom, with their attributes, asked for all at
    /// once; a window gone meanwhile is left out.
    fn children(&self) -> Result<Vec<(Window, GetWindowAttributesReply)>> {
        shared::drop_events(&self.connection);
        let children = self.connection.query_tree(self.root)?.reply()?.children;
        let cookies = children
            .into_iter()
            .map(|window| Ok((window, self.connection.get_window_attributes(window)?)))
            .collect::<Result<Vec<_>>>()?;
        let mut attributes = Vec::with_capacity(cookies.len());
        for (window, cookie) in cookies {
            if let Some(reply) = unless_gone(cookie.reply())? {
                attributes.push((window, reply));
            }
        }
        Ok(attributes)
    }

    /// The child of the root window that holds `window`, which is its frame under a window
    /// manager that frames the windows (an X11 session); none once it is gone.
    fn top_level(&self, mut window: Window) -> Result<Option<Window>> {
        loop {
            let Some(tree) = unless_gone(self.connection.query_tree(window)?.reply())? else {
                return Ok(None);
            };
            if tree.parent == self.root || tree.parent == x11rb::NONE {
                return Ok(Some(window));
            }
            window = tree.parent;
        }
    }
}

impl Screen for XScreen {
    fn pointer_grabbed(&mut self) -> Result<bool> {
        let buttons = KeyButMask::BUTTON1
            | KeyButMask::BUTTON2
            | KeyButMask::BUTTON3
            | KeyButMask::BUTTON4
            | KeyButMask::BUTTON5;
        let pointer = self.connection.query_pointer(self.root)?.reply()?;
        if u16::from(pointer.mask) & u16::from(buttons) != 0 {
            return Ok(false);
        }
        let status = self
            .connection
            .grab_pointer(
                false,
                self.root,
                EventMask::NO_EVENT,
                GrabMode::ASYNC,
                GrabMode::ASYNC,
                x11rb::NONE,
                x11rb::NONE,
                x11rb::CURRENT_TIME,
            )?
            .reply()?
            .status;
        if status == GrabStatus::SUCCESS {
            self.connection
                .ungrab_pointer(x11rb::CURRENT_TIME)?
                .check()?;
        }
        Ok(status == GrabStatus::ALREADY_GRABBED)
    }

    fn pointer_window_above(&mut self, game: Window) -> Result<bool> {
        let Some(game) = self.top_level(game)? else {
            return Ok(false);
        };
        let children = self.children()?;
        // The size of the screen of X11, which the window Wine holds the pointer in covers: read
        // each time, since the size kept from the connection was wrong after a change of
        // resolution or a screen unplugged, and the pointer held never seen again on a
        // smaller screen.
        let screen = self.connection.get_geometry(self.root)?.reply()?;
        let screen = (screen.width, screen.height);
        for &(window, ref attributes) in above(&children, game) {
            if !may_hold_the_pointer(
                attributes.class,
                attributes.override_redirect,
                attributes.map_state,
            ) {
                continue;
            }
            let Some(geometry) = unless_gone(self.connection.get_geometry(window)?.reply())? else {
                continue;
            };
            let covers = covers_screen(
                (geometry.x, geometry.y),
                (geometry.width, geometry.height),
                screen,
            );
            // Gone meanwhile, it isn't that window either.
            if covers
                && shared::ours_unless_gone(&self.connection, self.atoms, window)? == Some(false)
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn overlay(&mut self) -> Result<Option<Window>> {
        // From the top: the window over the game is above the others.
        for (window, attributes) in self.children()?.into_iter().rev() {
            if attributes.override_redirect
                && attributes.map_state == MapState::VIEWABLE
                && shared::is_ours(&self.connection, self.atoms, window)?
            {
                return Ok(Some(window));
            }
        }
        Ok(None)
    }

    fn active(&mut self) -> Result<Option<Window>> {
        shared::active_window(&self.connection, self.root, self.atoms)
    }

    fn game_behind(&mut self, window: Window) -> Result<Option<Window>> {
        shared::game_behind(&self.connection, self.atoms, window)
    }

    fn show_helper(&mut self, overlay: Window, game: Window) -> Result<Window> {
        let helper = self.create_helper()?;
        let geometry = self.connection.get_geometry(overlay)?.reply()?;
        // Its first pixel under the window over the game, where the player asked for it, so that
        // the desktop doesn't place it elsewhere.
        let mut hints = WmSizeHints::new();
        hints.position = Some((
            WmSizeHintsSpecification::UserSpecified,
            i32::from(geometry.x),
            i32::from(geometry.y),
        ));
        hints.set_normal_hints(&self.connection, helper)?;
        self.connection.configure_window(
            helper,
            &ConfigureWindowAux::new()
                .x(i32::from(geometry.x))
                .y(i32::from(geometry.y)),
        )?;
        self.connection.change_property32(
            PropMode::REPLACE,
            helper,
            self.atoms.game_behind,
            AtomEnum::WINDOW,
            &[game],
        )?;
        // A window of the game for the desktop, like its dialogs: `KWin` keeps a fullscreen
        // window above the taskbar while the active window is one of its own, which a fullscreen
        // game no longer active would fall behind.
        self.connection.change_property32(
            PropMode::REPLACE,
            helper,
            AtomEnum::WM_TRANSIENT_FOR,
            AtomEnum::WINDOW,
            &[game],
        )?;
        self.connection.map_window(helper)?.check()?;
        Ok(helper)
    }

    fn hide_helper(&mut self) -> Result<()> {
        let Some(helper) = self.helper else {
            return Ok(());
        };
        self.connection.unmap_window(helper)?.check()?;
        self.connection
            .delete_property(helper, self.atoms.game_behind)?
            .check()?;
        Ok(())
    }

    fn activate(&mut self, window: Window) -> Result<()> {
        shared::activate(&self.connection, self.root, self.atoms, window)
    }

    fn set_focus(&mut self, window: Window) -> Result<()> {
        // Back to the root window if it hides.
        self.connection
            .set_input_focus(InputFocus::PARENT, window, x11rb::CURRENT_TIME)?
            .check()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GAME: Window = 10;
    const OVERLAY: Window = 20;
    const HELPER: Window = 30;
    const MAIN: Window = 40;

    /// The desktop review of 2026-11-10: the size of the screen is read at each look, not kept
    /// from the connection.
    #[test]
    fn the_window_of_wine_covers_the_screen_as_it_is_now() {
        let window = ((0, 0), (1920, 1080));

        assert!(covers_screen(window.0, window.1, (1920, 1080)));
        // The screen got smaller meanwhile: still covered.
        assert!(covers_screen(window.0, window.1, (1280, 720)));
        assert!(covers_screen((-10, -10), (1940, 1100), (1920, 1080)));
        // A window of one screen of two, or one that starts further.
        assert!(!covers_screen(window.0, window.1, (3840, 1080)));
        assert!(!covers_screen((1, 0), window.1, (1920, 1080)));
    }

    /// The desktop review of 2026-11-10, assumption 6: nothing tested which windows are taken
    /// for the one Wine holds the pointer in. A window under the game isn't over it.
    #[test]
    fn only_the_windows_above_the_game_are_looked_at() {
        let children = [(MAIN, 'a'), (GAME, 'b'), (HELPER, 'c'), (OVERLAY, 'd')];

        assert_eq!(above(&children, GAME), [(HELPER, 'c'), (OVERLAY, 'd')]);
        assert_eq!(above(&children, OVERLAY), []);
        assert_eq!(above(&children, MAIN).len(), 3);
        // The game gone meanwhile, or framed by a window that isn't a child of the root.
        assert_eq!(above(&children, 99), []);
        assert_eq!(above::<char>(&[], GAME), []);
    }

    /// As a probe found it in the owner's game on 2026-11-07: `InputOnly`, override-redirect,
    /// viewable. A window that is drawn, that the window manager manages or
    /// that is hidden isn't it: the window over the game itself is above the game.
    #[test]
    fn the_window_of_wine_is_never_drawn_unmanaged_and_shown() {
        let input_only = WindowClass::INPUT_ONLY;
        let shown = MapState::VIEWABLE;

        assert!(may_hold_the_pointer(input_only, true, shown));
        assert!(!may_hold_the_pointer(
            WindowClass::INPUT_OUTPUT,
            true,
            shown
        ));
        assert!(!may_hold_the_pointer(
            WindowClass::COPY_FROM_PARENT,
            true,
            shown
        ));
        assert!(!may_hold_the_pointer(input_only, false, shown));
        assert!(!may_hold_the_pointer(input_only, true, MapState::UNMAPPED));
        assert!(!may_hold_the_pointer(
            input_only,
            true,
            MapState::UNVIEWABLE
        ));
    }
}
