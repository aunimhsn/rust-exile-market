//! The keyboard of the window over the game under X11: given to the window while one of its
//! fields is edited, and given back to the game afterwards (`specs.md`, Window over the game).
//!
//! The window manager doesn't manage the window over the game, so that it never gives it the
//! keyboard: the application sets the input focus of the X server itself (`SetInputFocus`), as
//! Exiled Exchange 2 does (`xcb_set_input_focus`). The desktop still takes the game for its active
//! window meanwhile (`_NET_ACTIVE_WINDOW`), and doesn't take the keyboard back, which a probe in
//! the owner's game checked on 2026-11-01 (Bazzite KDE, windowed fullscreen).

use std::error::Error;

use x11rb::connection::Connection as _;
use x11rb::protocol::xproto::{AtomEnum, ConnectionExt as _, InputFocus, Window};
use x11rb::rust_connection::RustConnection;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// The input focus of the X server, or a fake one in the tests.
pub trait Focus {
    /// The window of this application under the pointer that the window manager doesn't manage:
    /// the window over the game, when a click just went to one of its fields.
    fn ours_under_pointer(&mut self) -> Result<Option<Window>>;
    /// The active window of the desktop: the game, which keeps it while the window over the
    /// game has the keyboard.
    fn active(&mut self) -> Result<Option<Window>>;
    /// The window that has the keyboard.
    fn focused(&mut self) -> Result<Window>;
    /// The root window, which gets the keyboard when the window that had it hides.
    fn root(&self) -> Window;
    fn set_focus(&mut self, window: Window) -> Result<()>;
}

/// Who has the keyboard while the window over the game has it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Held {
    overlay: Window,
    game: Window,
}

/// The connection to the X server: made at the first field edited.
enum Link<F> {
    Waiting(fn() -> Result<F>),
    Connected(F),
    /// It couldn't be made, which is written to the log once.
    Unavailable,
}

/// Gives the keyboard to the window over the game while one of its fields is edited.
pub struct KeyboardFocus<F = XFocus> {
    link: Link<F>,
    editing: bool,
    held: Option<Held>,
}

impl KeyboardFocus {
    #[must_use]
    pub fn new() -> Self {
        Self::with(XFocus::connect)
    }
}

impl Default for KeyboardFocus {
    fn default() -> Self {
        Self::new()
    }
}

impl<F: Focus> KeyboardFocus<F> {
    fn with(connect: fn() -> Result<F>) -> Self {
        Self {
            link: Link::Waiting(connect),
            editing: false,
            held: None,
        }
    }

    /// Follows whether a field of the window over the game is `editing`: when one starts, the
    /// window takes the keyboard; when none is edited any more (Enter, Esc, a click elsewhere,
    /// the window closed), the game gets it back. `true` while the window has it.
    pub fn follow(&mut self, editing: bool) -> bool {
        if editing != std::mem::replace(&mut self.editing, editing) {
            if editing {
                self.take();
            } else {
                self.give_back();
            }
        }
        self.held.is_some()
    }

    /// The window over the game has the keyboard.
    #[must_use]
    pub fn held(&self) -> bool {
        self.held.is_some()
    }

    fn focus(&mut self) -> Option<&mut F> {
        if let Link::Waiting(connect) = self.link {
            self.link = match connect() {
                Ok(focus) => Link::Connected(focus),
                Err(error) => {
                    log::warn!(
                        "The window over the game can't take the keyboard: no X server ({error})"
                    );
                    Link::Unavailable
                }
            };
        }
        match &mut self.link {
            Link::Connected(focus) => Some(focus),
            Link::Waiting(_) | Link::Unavailable => None,
        }
    }

    fn take(&mut self) {
        let Some(focus) = self.focus() else { return };
        let held: Result<Held> = (|| {
            let Some(overlay) = focus.ours_under_pointer()? else {
                return Err("the window over the game isn't under the pointer".into());
            };
            let Some(game) = focus.active()? else {
                return Err("no active window to give the keyboard back to".into());
            };
            focus.set_focus(overlay)?;
            Ok(Held { overlay, game })
        })();
        self.held = held
            .map_err(|error| {
                log::warn!("The window over the game can't take the keyboard: {error}");
            })
            .ok();
    }

    fn give_back(&mut self) {
        let Some(held) = self.held.take() else { return };
        let Some(focus) = self.focus() else { return };
        let given: Result<()> = (|| {
            // Given meanwhile to another window (a click in the main window, Ctrl+D in the
            // game), the keyboard stays there.
            if gives_back(focus.focused()?, held, focus.root()) {
                focus.set_focus(held.game)?;
            }
            Ok(())
        })();
        if let Err(error) = given {
            log::warn!("The keyboard can't be given back to the game: {error}");
        }
    }
}

/// Whether the keyboard, at `focused`, goes back to the game: while the window over the game
/// has it, or the root window got it when the window hid; not when it went to another window.
fn gives_back(focused: Window, held: Held, root: Window) -> bool {
    focused == held.overlay
        || focused == root
        || focused == x11rb::NONE
        || focused == u32::from(InputFocus::POINTER_ROOT)
}

/// The input focus of the X server.
pub struct XFocus {
    connection: RustConnection,
    root: Window,
    active_window: u32,
    pid: u32,
}

impl XFocus {
    fn connect() -> Result<Self> {
        let (connection, screen) = x11rb::connect(None)?;
        let root = connection.setup().roots[screen].root;
        let active_window = connection
            .intern_atom(false, b"_NET_ACTIVE_WINDOW")?
            .reply()?
            .atom;
        let pid = connection.intern_atom(false, b"_NET_WM_PID")?.reply()?.atom;
        Ok(Self {
            connection,
            root,
            active_window,
            pid,
        })
    }
}

impl Focus for XFocus {
    fn ours_under_pointer(&mut self) -> Result<Option<Window>> {
        // The window over the game is a child of the root window, without the frame of the
        // window manager.
        let window = self.connection.query_pointer(self.root)?.reply()?.child;
        if window == x11rb::NONE
            || !self
                .connection
                .get_window_attributes(window)?
                .reply()?
                .override_redirect
        {
            return Ok(None);
        }
        Ok(is_ours(&self.connection, self.pid, window)?.then_some(window))
    }

    fn active(&mut self) -> Result<Option<Window>> {
        let window = self
            .connection
            .get_property(false, self.root, self.active_window, AtomEnum::WINDOW, 0, 1)?
            .reply()?
            .value32()
            .and_then(|mut windows| windows.next())
            .unwrap_or(x11rb::NONE);
        Ok((window != x11rb::NONE).then_some(window))
    }

    fn focused(&mut self) -> Result<Window> {
        Ok(self.connection.get_input_focus()?.reply()?.focus)
    }

    fn root(&self) -> Window {
        self.root
    }

    fn set_focus(&mut self, window: Window) -> Result<()> {
        // Back to the root window if it hides, which `gives_back` recognizes.
        self.connection
            .set_input_focus(InputFocus::PARENT, window, x11rb::CURRENT_TIME)?
            .check()?;
        Ok(())
    }
}

/// Whether `window` belongs to this application: its `_NET_WM_PID` (the atom `pid`), which winit
/// writes on each window, is the id of this process.
pub fn is_ours(connection: &RustConnection, pid: u32, window: Window) -> Result<bool> {
    let owner = connection
        .get_property(false, window, pid, AtomEnum::CARDINAL, 0, 1)?
        .reply()?
        .value32()
        .and_then(|mut values| values.next());
    Ok(owner == Some(std::process::id()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    const ROOT: Window = 0x410;
    const OVERLAY: Window = 0x3e0_000a;
    const GAME: Window = 0x660_0001;
    const MAIN: Window = 0x3e0_0003;

    /// The X server as the tests set it: what the pointer is over, the active window, and the
    /// window with the keyboard, which `set_focus` changes.
    #[derive(Debug)]
    struct Server {
        under_pointer: Option<Window>,
        active: Option<Window>,
        focused: Window,
        fails: bool,
        set: Vec<Window>,
    }

    impl Default for Server {
        fn default() -> Self {
            Self {
                under_pointer: Some(OVERLAY),
                active: Some(GAME),
                focused: GAME,
                fails: false,
                set: Vec::new(),
            }
        }
    }

    struct FakeFocus(Rc<RefCell<Server>>);

    impl Focus for FakeFocus {
        fn ours_under_pointer(&mut self) -> Result<Option<Window>> {
            Ok(self.0.borrow().under_pointer)
        }
        fn active(&mut self) -> Result<Option<Window>> {
            Ok(self.0.borrow().active)
        }
        fn focused(&mut self) -> Result<Window> {
            Ok(self.0.borrow().focused)
        }
        fn root(&self) -> Window {
            ROOT
        }
        fn set_focus(&mut self, window: Window) -> Result<()> {
            let mut server = self.0.borrow_mut();
            if server.fails {
                return Err("refused".into());
            }
            server.focused = window;
            server.set.push(window);
            Ok(())
        }
    }

    /// The keyboard of the window over the game on `server`, already connected.
    fn keyboard(server: Server) -> (KeyboardFocus<FakeFocus>, Rc<RefCell<Server>>) {
        let server = Rc::new(RefCell::new(server));
        let mut keyboard = KeyboardFocus::with(|| Err("never connected".into()));
        keyboard.link = Link::Connected(FakeFocus(Rc::clone(&server)));
        (keyboard, server)
    }

    #[test]
    fn a_field_edited_takes_the_keyboard_and_leaving_it_gives_it_back() {
        let (mut keyboard, server) = keyboard(Server::default());
        assert!(!keyboard.follow(false));
        assert!(keyboard.follow(true));
        assert_eq!(server.borrow().focused, OVERLAY);
        assert!(keyboard.follow(true));
        assert!(!keyboard.follow(false));
        assert_eq!(server.borrow().focused, GAME);
        assert_eq!(server.borrow().set, [OVERLAY, GAME]);
    }

    #[test]
    fn the_window_hidden_gives_the_keyboard_back_from_the_root_window() {
        let (mut keyboard, server) = keyboard(Server::default());
        keyboard.follow(true);
        server.borrow_mut().focused = ROOT;
        keyboard.follow(false);
        assert_eq!(server.borrow().focused, GAME);
    }

    #[test]
    fn a_keyboard_given_to_another_window_meanwhile_stays_there() {
        let (mut keyboard, server) = keyboard(Server::default());
        keyboard.follow(true);
        // A click in the main window, which the window manager activates.
        server.borrow_mut().focused = MAIN;
        keyboard.follow(false);
        assert_eq!(server.borrow().focused, MAIN);
        assert_eq!(server.borrow().set, [OVERLAY]);
    }

    #[test]
    fn without_the_window_under_the_pointer_nothing_is_taken() {
        let (mut keyboard, server) = keyboard(Server {
            under_pointer: None,
            ..Server::default()
        });
        assert!(!keyboard.follow(true));
        assert!(!keyboard.follow(false));
        assert_eq!(server.borrow().set, Vec::<Window>::new());
    }

    #[test]
    fn without_an_active_window_nothing_is_taken() {
        let (mut keyboard, server) = keyboard(Server {
            active: None,
            ..Server::default()
        });
        assert!(!keyboard.follow(true));
        assert_eq!(server.borrow().set, Vec::<Window>::new());
    }

    #[test]
    fn a_focus_refused_holds_nothing() {
        let (mut keyboard, server) = keyboard(Server {
            fails: true,
            ..Server::default()
        });
        assert!(!keyboard.follow(true));
        assert_eq!(server.borrow().focused, GAME);
    }

    #[test]
    fn without_an_x_server_nothing_is_held() {
        let mut keyboard = KeyboardFocus::<FakeFocus>::with(|| Err("no display".into()));
        assert!(!keyboard.follow(true));
        assert!(!keyboard.follow(false));
    }

    #[test]
    fn the_keyboard_goes_back_from_the_window_the_root_or_no_window_only() {
        let held = Held {
            overlay: OVERLAY,
            game: GAME,
        };
        assert!(gives_back(OVERLAY, held, ROOT));
        assert!(gives_back(ROOT, held, ROOT));
        assert!(gives_back(x11rb::NONE, held, ROOT));
        assert!(gives_back(u32::from(InputFocus::POINTER_ROOT), held, ROOT));
        assert!(!gives_back(MAIN, held, ROOT));
        assert!(!gives_back(GAME, held, ROOT));
    }
}
