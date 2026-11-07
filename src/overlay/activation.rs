//! The window over the game made the active window of the desktop on a single screen
//! (`specs.md`, Window over the game, On a single screen).
//!
//! With a single screen and the game in windowed fullscreen, Wine holds the pointer of the X
//! server (a grab, so that the mouse stays in the game) a second or two after the game becomes
//! the active window: every move and click then goes to the game, and the window over the game
//! shows the cursor of the game and gets neither the hover nor the clicks. Wine lets the pointer
//! go as soon as another window is the active window, and holds it again once the game is
//! (probes in the owner's game on 2026-11-07, Bazzite KDE). The window over the game is out of
//! the hands of the window manager, which can't activate it: a window of this application that
//! the desktop manages, 1 × 1 under it and out of the taskbar (the helper), is activated instead,
//! and the window over the game takes the keyboard from it (`SetInputFocus`). The helper carries
//! the game (`_EXILE_MARKET_GAME`), so that the hotkey takes it for the game
//! ([`crate::hotkey`]) and gives it back before its copy.

use std::time::{Duration, Instant};

use x11rb::connection::Connection as _;
use x11rb::properties::WmSizeHints;
use x11rb::protocol::xproto::{
    AtomEnum, ConnectionExt as _, CreateWindowAux, EventMask, GrabMode, GrabStatus, InputFocus,
    MapState, PropMode, Window, WindowClass,
};
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as _;

use crate::x11::{self as shared, Atoms, Result};

/// How long the window over the game may take to be shown, then the desktop to activate the
/// helper, before it is given up: the window then behaves as on two screens.
const TIMEOUT: Duration = Duration::from_secs(1);

/// The desktop as the window over the game sees it, or a fake one in the tests.
pub trait Screen {
    /// Whether another application holds the pointer: it can't be taken, and is let go at once
    /// when it can.
    fn pointer_grabbed(&mut self) -> Result<bool>;
    /// The window over the game, once it is shown: the window of this application that the
    /// window manager doesn't manage.
    fn overlay(&mut self) -> Result<Option<Window>>;
    /// The active window of the desktop.
    fn active(&mut self) -> Result<Option<Window>>;
    /// The game the `window` carries, when it is the helper.
    fn game_behind(&mut self, window: Window) -> Result<Option<Window>>;
    /// Shows the helper under the `overlay`, carrying the `game`; the helper.
    fn show_helper(&mut self, overlay: Window, game: Window) -> Result<Window>;
    fn hide_helper(&mut self) -> Result<()>;
    /// Asks the desktop to make `window` the active window.
    fn activate(&mut self, window: Window) -> Result<()>;
    fn set_focus(&mut self, window: Window) -> Result<()>;
}

/// Where the window over the game is in making itself the active window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    /// The window over the game is closed, or the game doesn't hold the pointer.
    Idle,
    /// The game held the pointer when the window opened: waiting for the window to show.
    Waiting { game: Window, since: Instant },
    /// The desktop was asked to activate the helper.
    Activating {
        overlay: Window,
        game: Window,
        helper: Window,
        since: Instant,
    },
    /// The helper is the active window and the window over the game has the keyboard.
    Active { game: Window, helper: Window },
}

/// The connection to the X server: made when the window over the game first opens.
enum Link<S> {
    Waiting(fn() -> Result<S>),
    Connected(S),
    /// It couldn't be made, which is written to the log once.
    Unavailable,
}

/// Makes the window over the game the active window while the game holds the pointer.
pub struct Activation<S = XScreen> {
    link: Link<S>,
    state: State,
    /// The opening of the window followed ([`super::Overlay::opened`]).
    opened: Option<u64>,
    /// The hotkey was pressed while the window was the active one: the window that opens on its
    /// item becomes it again, without asking whether the game holds the pointer, which Wine only
    /// takes again a moment after the game is active once more.
    carry: bool,
}

impl Activation {
    #[must_use]
    pub fn new() -> Self {
        Self::with(XScreen::connect)
    }
}

impl Default for Activation {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Screen> Activation<S> {
    fn with(connect: fn() -> Result<S>) -> Self {
        Self {
            link: Link::Waiting(connect),
            state: State::Idle,
            opened: None,
            carry: false,
        }
    }

    /// The hotkey was pressed: the window that opens on its item makes itself the active window
    /// again when this one was.
    pub fn hotkey(&mut self) {
        self.carry = self.state != State::Idle;
    }

    /// The window over the game is the active window, or about to be: the keys go to it.
    #[must_use]
    pub fn holds(&self) -> bool {
        matches!(self.state, State::Activating { .. } | State::Active { .. })
    }

    /// Follows the window over the game, `opened` while it is open (a new number each time it
    /// opens), at each frame; `true` while it waits for the window or the desktop, so that the
    /// next frame comes soon.
    pub fn follow(&mut self, opened: Option<u64>, now: Instant) -> bool {
        if opened != std::mem::replace(&mut self.opened, opened) {
            let carry = std::mem::take(&mut self.carry);
            if opened.is_some() {
                self.open(carry, now);
            } else {
                self.close();
            }
        }
        self.advance(now);
        matches!(self.state, State::Waiting { .. } | State::Activating { .. })
    }

    fn screen(&mut self) -> Option<&mut S> {
        if let Link::Waiting(connect) = self.link {
            self.link = match connect() {
                Ok(screen) => Link::Connected(screen),
                Err(error) => {
                    log::warn!(
                        "The window over the game can't tell whether the game holds the pointer: no X server ({error})"
                    );
                    Link::Unavailable
                }
            };
        }
        match &mut self.link {
            Link::Connected(screen) => Some(screen),
            Link::Waiting(_) | Link::Unavailable => None,
        }
    }

    /// The window opens, or opens again on another item: it waits to become the active window
    /// when the game holds the pointer, or when it was the active window (`carry`).
    fn open(&mut self, carry: bool, now: Instant) {
        let state = self.state;
        let Some(screen) = self.screen() else { return };
        let game = match open(screen, state, carry) {
            Ok(game) => game,
            Err(error) => {
                log::warn!("The window over the game can't become the active window: {error}");
                None
            }
        };
        if let Some(game) = game {
            self.state = State::Waiting { game, since: now };
        } else {
            self.state = State::Idle;
            self.hide();
        }
    }

    /// The window closed: the game is the active window again, unless the player went to another
    /// window meanwhile.
    fn close(&mut self) {
        let state = std::mem::replace(&mut self.state, State::Idle);
        let (game, helper) = match state {
            State::Idle => return,
            State::Waiting { .. } => (None, None),
            State::Activating { game, helper, .. } | State::Active { game, helper } => {
                (Some(game), Some(helper))
            }
        };
        let Some(screen) = self.screen() else { return };
        if let (Some(game), Some(helper)) = (game, helper)
            && let Err(error) = give_back(screen, game, helper)
        {
            log::warn!("The game can't be made the active window again: {error}");
        }
        self.hide();
    }

    /// Moves on when the window shows, then when the desktop activates the helper; gives up
    /// after [`TIMEOUT`].
    fn advance(&mut self, now: Instant) {
        let state = self.state;
        if matches!(state, State::Idle | State::Active { .. }) {
            return;
        }
        let Some(screen) = self.screen() else { return };
        match advance(screen, state, now) {
            Ok(next) => self.state = next,
            Err(error) => {
                log::warn!("The window over the game can't become the active window: {error}");
                self.state = State::Idle;
                self.hide();
            }
        }
    }

    fn hide(&mut self) {
        let Some(screen) = self.screen() else { return };
        if let Err(error) = screen.hide_helper() {
            log::warn!("The helper of the window over the game can't be hidden: {error}");
        }
    }
}

/// The game the window over the game, opening, will give the active window back to, when it is
/// to become the active window: when the game holds the pointer, or when it was the active window
/// (`carry`) at the hotkey, whose copy gave it back to the game.
fn open(screen: &mut impl Screen, state: State, carry: bool) -> Result<Option<Window>> {
    if !carry && !screen.pointer_grabbed()? {
        return Ok(None);
    }
    let Some(active) = screen.active()? else {
        return Ok(None);
    };
    // The copy of the hotkey may not have given the active window back yet.
    let game = screen.game_behind(active)?.unwrap_or(active);
    let before = match state {
        State::Waiting { game, .. }
        | State::Activating { game, .. }
        | State::Active { game, .. } => Some(game),
        State::Idle => None,
    };
    Ok(Some(if carry { before.unwrap_or(game) } else { game }))
}

/// The state after `state` at `now`, once the window shows or the desktop activates the helper.
fn advance(screen: &mut impl Screen, state: State, now: Instant) -> Result<State> {
    match state {
        State::Waiting { game, since } => {
            let Some(overlay) = screen.overlay()? else {
                return Ok(if now.saturating_duration_since(since) > TIMEOUT {
                    log::warn!("The window over the game didn't show: it stays as on two screens");
                    State::Idle
                } else {
                    state
                });
            };
            let helper = screen.show_helper(overlay, game)?;
            screen.activate(helper)?;
            Ok(State::Activating {
                overlay,
                game,
                helper,
                since: now,
            })
        }
        State::Activating {
            overlay,
            game,
            helper,
            since,
        } => {
            if screen.active()? == Some(helper) {
                screen.set_focus(overlay)?;
                log::info!(
                    "The game holds the pointer: the window over the game is the active window"
                );
                return Ok(State::Active { game, helper });
            }
            if now.saturating_duration_since(since) > TIMEOUT {
                return Err("the desktop didn't activate its helper".into());
            }
            Ok(state)
        }
        State::Idle | State::Active { .. } => Ok(state),
    }
}

/// Makes the `game` the active window again, while the `helper` still is: not when the player
/// went to another window (a click in the game or the main window, Alt+Tab).
fn give_back(screen: &mut impl Screen, game: Window, helper: Window) -> Result<()> {
    if screen.active()? == Some(helper) {
        screen.activate(game)?;
    }
    Ok(())
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
    fn connect() -> Result<Self> {
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

    /// The helper: managed by the desktop, without decorations, out of the taskbar and the
    /// pager, of this process.
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
        // No decorations: the flags say that the decorations are given, then none.
        let motif = intern(b"_MOTIF_WM_HINTS")?;
        connection.change_property32(PropMode::REPLACE, helper, motif, motif, &[2, 0, 0, 0, 0])?;
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
}

impl Screen for XScreen {
    fn pointer_grabbed(&mut self) -> Result<bool> {
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

    fn overlay(&mut self) -> Result<Option<Window>> {
        shared::drop_events(&self.connection);
        let children = self.connection.query_tree(self.root)?.reply()?.children;
        // From the top: the window over the game is above the others.
        for window in children.into_iter().rev() {
            let attributes = self.connection.get_window_attributes(window)?.reply()?;
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
            x11rb::properties::WmSizeHintsSpecification::UserSpecified,
            i32::from(geometry.x),
            i32::from(geometry.y),
        ));
        hints.set_normal_hints(&self.connection, helper)?;
        self.connection.configure_window(
            helper,
            &x11rb::protocol::xproto::ConfigureWindowAux::new()
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
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::*;

    const GAME: Window = 10;
    const OVERLAY: Window = 20;
    const HELPER: Window = 30;
    const MAIN: Window = 40;

    /// What the fake desktop did, and how it answers.
    #[derive(Debug, Default)]
    struct Desk {
        grabbed: bool,
        shown: bool,
        active: Option<Window>,
        /// Whether the desktop activates the helper when asked.
        refuses: bool,
        /// The helper, while it is shown.
        helper: Option<Window>,
        focus: Option<Window>,
        activated: Vec<Window>,
    }

    #[derive(Clone)]
    struct FakeScreen(Rc<RefCell<Desk>>);

    impl Screen for FakeScreen {
        fn pointer_grabbed(&mut self) -> Result<bool> {
            Ok(self.0.borrow().grabbed)
        }
        fn overlay(&mut self) -> Result<Option<Window>> {
            Ok(self.0.borrow().shown.then_some(OVERLAY))
        }
        fn active(&mut self) -> Result<Option<Window>> {
            Ok(self.0.borrow().active)
        }
        fn game_behind(&mut self, window: Window) -> Result<Option<Window>> {
            let desk = self.0.borrow();
            Ok((window == HELPER && desk.helper == Some(HELPER)).then_some(GAME))
        }
        fn show_helper(&mut self, overlay: Window, game: Window) -> Result<Window> {
            assert_eq!((overlay, game), (OVERLAY, GAME));
            self.0.borrow_mut().helper = Some(HELPER);
            Ok(HELPER)
        }
        fn hide_helper(&mut self) -> Result<()> {
            self.0.borrow_mut().helper = None;
            Ok(())
        }
        fn activate(&mut self, window: Window) -> Result<()> {
            let mut desk = self.0.borrow_mut();
            desk.activated.push(window);
            if !(window == HELPER && desk.refuses) {
                desk.active = Some(window);
            }
            Ok(())
        }
        fn set_focus(&mut self, window: Window) -> Result<()> {
            self.0.borrow_mut().focus = Some(window);
            Ok(())
        }
    }

    thread_local! {
        static DESK: RefCell<Option<FakeScreen>> = const { RefCell::new(None) };
    }

    /// An activation on a fake desktop where the game is active, holding the pointer or not.
    fn activation(grabbed: bool) -> (Activation<FakeScreen>, Rc<RefCell<Desk>>) {
        let desk = Rc::new(RefCell::new(Desk {
            grabbed,
            active: Some(GAME),
            ..Desk::default()
        }));
        DESK.with(|cell| *cell.borrow_mut() = Some(FakeScreen(Rc::clone(&desk))));
        let activation = Activation::with(|| {
            Ok(DESK.with(|cell| cell.borrow().clone().expect("a fake desktop")))
        });
        (activation, desk)
    }

    #[test]
    fn on_two_screens_the_window_stays_out_of_the_way() {
        let (mut activation, desk) = activation(false);
        let now = Instant::now();
        desk.borrow_mut().shown = true;

        assert!(!activation.follow(Some(1), now));

        assert!(!activation.holds());
        let desk = desk.borrow();
        assert_eq!(desk.activated, Vec::<Window>::new());
        assert_eq!(desk.focus, None);
        assert_eq!(desk.active, Some(GAME));
    }

    #[test]
    fn while_the_game_holds_the_pointer_the_window_becomes_the_active_one() {
        let (mut activation, desk) = activation(true);
        let now = Instant::now();

        // Not shown yet: it waits for the window, and wants the next frame soon.
        assert!(activation.follow(Some(1), now));
        assert!(!activation.holds());
        desk.borrow_mut().shown = true;
        // Shown: the helper is asked for, then the window takes the keyboard from it.
        assert!(activation.follow(Some(1), now));
        assert!(!activation.follow(Some(1), now));

        assert!(activation.holds());
        let desk = desk.borrow();
        assert_eq!(desk.helper, Some(HELPER));
        assert_eq!(desk.active, Some(HELPER));
        assert_eq!(desk.focus, Some(OVERLAY));
    }

    /// Opened while the game holds the pointer, and followed until it holds the keyboard.
    fn opened_active() -> (Activation<FakeScreen>, Rc<RefCell<Desk>>) {
        let (mut activation, desk) = activation(true);
        desk.borrow_mut().shown = true;
        let now = Instant::now();
        activation.follow(Some(1), now);
        activation.follow(Some(1), now);
        assert!(activation.holds());
        (activation, desk)
    }

    #[test]
    fn closed_the_game_is_the_active_window_again() {
        let (mut activation, desk) = opened_active();

        activation.follow(None, Instant::now());

        assert!(!activation.holds());
        let desk = desk.borrow();
        assert_eq!(desk.active, Some(GAME));
        assert_eq!(desk.helper, None);
    }

    #[test]
    fn closed_by_a_click_in_another_window_the_active_window_stays_there() {
        let (mut activation, desk) = opened_active();
        // A click in the main window: the desktop made it active.
        desk.borrow_mut().active = Some(MAIN);

        activation.follow(None, Instant::now());

        let desk = desk.borrow();
        assert_eq!(desk.active, Some(MAIN));
        assert_eq!(desk.activated, [HELPER]);
        assert_eq!(desk.helper, None);
    }

    #[test]
    fn the_hotkey_opens_it_again_as_the_active_window_without_the_pointer_held() {
        let (mut activation, desk) = opened_active();
        activation.hotkey();
        // The copy of the hotkey made the game active again: Wine doesn't hold the pointer yet.
        {
            let mut desk = desk.borrow_mut();
            desk.active = Some(GAME);
            desk.grabbed = false;
        }
        let now = Instant::now();

        activation.follow(Some(2), now);
        activation.follow(Some(2), now);

        assert!(activation.holds());
        assert_eq!(desk.borrow().active, Some(HELPER));
    }

    #[test]
    fn the_hotkey_on_two_screens_asks_again() {
        let (mut activation, desk) = activation(false);
        desk.borrow_mut().shown = true;
        let now = Instant::now();
        activation.follow(Some(1), now);
        activation.hotkey();

        activation.follow(Some(2), now);

        assert!(!activation.holds());
        assert_eq!(desk.borrow().activated, Vec::<Window>::new());
    }

    #[test]
    fn a_desktop_that_refuses_leaves_the_window_as_on_two_screens() {
        let (mut activation, desk) = activation(true);
        {
            let mut desk = desk.borrow_mut();
            desk.shown = true;
            desk.refuses = true;
        }
        let now = Instant::now();
        activation.follow(Some(1), now);
        assert!(activation.follow(Some(1), now));

        assert!(!activation.follow(Some(1), now + TIMEOUT + Duration::from_millis(1)));

        assert!(!activation.holds());
        let desk = desk.borrow();
        assert_eq!(desk.helper, None);
        assert_eq!(desk.focus, None);
    }

    #[test]
    fn a_window_that_never_shows_is_given_up() {
        let (mut activation, desk) = activation(true);
        let now = Instant::now();
        assert!(activation.follow(Some(1), now));

        assert!(!activation.follow(Some(1), now + TIMEOUT + Duration::from_millis(1)));

        assert_eq!(desk.borrow().activated, Vec::<Window>::new());
    }

    #[test]
    fn closed_before_it_shows_nothing_is_activated() {
        let (mut activation, desk) = activation(true);
        let now = Instant::now();
        activation.follow(Some(1), now);

        activation.follow(None, now);
        desk.borrow_mut().shown = true;
        activation.follow(None, now);

        assert_eq!(desk.borrow().activated, Vec::<Window>::new());
    }
}
