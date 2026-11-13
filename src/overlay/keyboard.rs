//! The keyboard of the window over the game under X11: given to the window while one of its
//! fields is edited, and given back to the game afterwards (`specs.md`, Window over the game).
//!
//! The window manager doesn't manage the window over the game, so that it never gives it the
//! keyboard: the application sets the input focus of the X server itself (`SetInputFocus`), as
//! Exiled Exchange 2 does (`xcb_set_input_focus`). The desktop still takes the game for its active
//! window meanwhile (`_NET_ACTIVE_WINDOW`), and doesn't take the keyboard back, which a probe in
//! the owner's game checked on 2026-11-01 (Bazzite KDE, windowed fullscreen).

use x11rb::protocol::xproto::{ConnectionExt as _, Window};

use crate::x11::{self as shared, Lazy, Result, Session};

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

/// Gives the keyboard to the window over the game while one of its fields is edited.
pub struct KeyboardFocus<F = XFocus> {
    /// The connection to the X server: made at the first field edited, and again at the next
    /// field edited or left once it failed.
    focus: Lazy<F>,
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
            focus: Lazy::new(connect, "The window over the game can't take the keyboard"),
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

    /// Gives the keyboard to the window over the game; on a failure, the connection is made
    /// again at the next field edited or left.
    fn take(&mut self) {
        let Some(focus) = self.focus.get_or_connect() else {
            return;
        };
        match take(focus) {
            Ok(held) => self.held = Some(held),
            Err(error) => {
                log::warn!("The window over the game can't take the keyboard: {error}");
                self.focus.forget();
            }
        }
    }

    /// Gives the keyboard back to the game; on a failure, the connection is made again at the
    /// next field edited.
    fn give_back(&mut self) {
        let Some(held) = self.held.take() else { return };
        let Some(focus) = self.focus.get_or_connect() else {
            return;
        };
        if let Err(error) = give_back(focus, held) {
            log::warn!("The keyboard can't be given back to the game: {error}");
            self.focus.forget();
        }
    }
}

/// Gives the keyboard to the window over the game, under the pointer; who has it then.
fn take(focus: &mut impl Focus) -> Result<Held> {
    let Some(overlay) = focus.ours_under_pointer()? else {
        return Err("the window over the game isn't under the pointer".into());
    };
    let Some(game) = focus.active()? else {
        return Err("no active window to give the keyboard back to".into());
    };
    focus.set_focus(overlay)?;
    Ok(Held { overlay, game })
}

/// Gives the keyboard back to the game, unless it went to another window meanwhile.
fn give_back(focus: &mut impl Focus, held: Held) -> Result<()> {
    let active = focus.active()?;
    let focused = focus.focused()?;
    if gives_back(focused, active, held, focus.root()) {
        focus.set_focus(held.game)?;
    } else {
        log::info!(
            "The keyboard stays where it went meanwhile (window {focused:#x}, active window {active:?})"
        );
    }
    Ok(())
}

/// Whether the keyboard, at `focused`, goes back to the game: while the window over the game
/// has it, or no window has it (the window hid), and the game is still the `active` window. Not
/// when the keyboard went to another window, nor when the desktop activated another window
/// (a click in the main window, Alt+Tab), which gets the keyboard from it. A desktop may tell
/// no active window while the window over the game has the keyboard (`KWin` keeps the game, other
/// window managers are unchecked): the keyboard then goes back from the window only.
fn gives_back(focused: Window, active: Option<Window>, held: Held, root: Window) -> bool {
    match active {
        Some(active) => {
            active == held.game && (focused == held.overlay || shared::is_nowhere(focused, root))
        }
        None => focused == held.overlay,
    }
}

/// The input focus of the X server.
pub struct XFocus {
    session: Session,
}

impl XFocus {
    fn connect() -> Result<Self> {
        Ok(Self {
            session: Session::connect()?,
        })
    }
}

impl Focus for XFocus {
    fn ours_under_pointer(&mut self) -> Result<Option<Window>> {
        self.session.drop_events();
        // The window over the game is a child of the root window, without the frame of the
        // window manager.
        let window = self.session.pointer()?.child;
        if window == x11rb::NONE
            || !self
                .session
                .connection()
                .get_window_attributes(window)?
                .reply()?
                .override_redirect
        {
            return Ok(None);
        }
        Ok(self.session.is_ours(window)?.then_some(window))
    }

    fn active(&mut self) -> Result<Option<Window>> {
        self.session.active_window()
    }

    fn focused(&mut self) -> Result<Window> {
        self.session.focused()
    }

    fn root(&self) -> Window {
        self.session.root()
    }

    fn set_focus(&mut self, window: Window) -> Result<()> {
        self.session.set_focus(window)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use x11rb::protocol::xproto::InputFocus;

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
        /// How many connections were made.
        connections: usize,
    }

    impl Default for Server {
        fn default() -> Self {
            Self {
                under_pointer: Some(OVERLAY),
                active: Some(GAME),
                focused: GAME,
                fails: false,
                set: Vec::new(),
                connections: 0,
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

    thread_local! {
        /// The fake X server of the test, for the connection, a function that can't capture it.
        static SERVER: RefCell<Option<Rc<RefCell<Server>>>> = const { RefCell::new(None) };
    }

    /// The keyboard of the window over the game on `server`, connected at the first field edited.
    fn keyboard(server: Server) -> (KeyboardFocus<FakeFocus>, Rc<RefCell<Server>>) {
        let server = Rc::new(RefCell::new(server));
        SERVER.with(|cell| *cell.borrow_mut() = Some(Rc::clone(&server)));
        let keyboard = KeyboardFocus::with(|| {
            let server = SERVER.with(|cell| cell.borrow().clone().expect("a fake X server"));
            server.borrow_mut().connections += 1;
            Ok(FakeFocus(server))
        });
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
    fn the_desktop_activating_another_window_keeps_the_keyboard_there() {
        let (mut keyboard, server) = keyboard(Server::default());
        keyboard.follow(true);
        // Alt+Tab: the window manager activates the main window, the window over the game hides.
        {
            let mut server = server.borrow_mut();
            server.active = Some(MAIN);
            server.focused = ROOT;
        }
        keyboard.follow(false);
        assert_eq!(server.borrow().set, [OVERLAY]);
    }

    #[test]
    fn a_keyboard_that_couldnt_be_given_back_is_no_longer_held() {
        let (mut keyboard, server) = keyboard(Server::default());
        assert!(keyboard.follow(true));
        server.borrow_mut().fails = true;
        assert!(!keyboard.follow(false));
        assert!(!keyboard.held());
        // The next field edited tries again, on a new connection.
        server.borrow_mut().fails = false;
        assert!(keyboard.follow(true));
        assert_eq!(server.borrow().connections, 2);
    }

    #[test]
    fn after_a_focus_refused_the_next_field_edited_connects_again() {
        let (mut keyboard, server) = keyboard(Server {
            fails: true,
            ..Server::default()
        });
        assert!(!keyboard.follow(true));
        keyboard.follow(false);
        server.borrow_mut().fails = false;

        assert!(keyboard.follow(true));

        assert_eq!(server.borrow().focused, OVERLAY);
        assert_eq!(server.borrow().connections, 2);
    }

    #[test]
    fn the_connection_is_kept_while_nothing_fails() {
        let (mut keyboard, server) = keyboard(Server::default());
        for editing in [true, false, true, false] {
            keyboard.follow(editing);
        }
        assert_eq!(server.borrow().connections, 1);
    }

    #[test]
    fn without_an_x_server_the_connection_is_tried_once() {
        static ATTEMPTS: AtomicUsize = AtomicUsize::new(0);
        let mut keyboard = KeyboardFocus::<FakeFocus>::with(|| {
            ATTEMPTS.fetch_add(1, Ordering::Relaxed);
            Err("no display".into())
        });
        for editing in [true, false, true, false] {
            assert!(!keyboard.follow(editing));
        }
        assert_eq!(ATTEMPTS.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn the_keyboard_goes_back_from_the_window_or_no_window_only() {
        let held = Held {
            overlay: OVERLAY,
            game: GAME,
        };
        let back = |focused| gives_back(focused, Some(GAME), held, ROOT);
        assert!(back(OVERLAY));
        assert!(back(ROOT));
        assert!(back(x11rb::NONE));
        assert!(!back(u32::from(InputFocus::POINTER_ROOT)));
        assert!(!back(MAIN));
        assert!(!back(GAME));
    }

    #[test]
    fn the_keyboard_stays_with_a_window_the_desktop_activated_meanwhile() {
        let held = Held {
            overlay: OVERLAY,
            game: GAME,
        };
        // Its activation may reach the X server after the window over the game hid.
        assert!(!gives_back(ROOT, Some(MAIN), held, ROOT));
    }

    #[test]
    fn without_an_active_window_the_keyboard_goes_back_from_the_window_only() {
        let held = Held {
            overlay: OVERLAY,
            game: GAME,
        };
        // A window manager that tells no active window while an unmanaged window has the keyboard.
        assert!(gives_back(OVERLAY, None, held, ROOT));
        assert!(!gives_back(ROOT, None, held, ROOT));
        assert!(!gives_back(MAIN, None, held, ROOT));
    }
}
