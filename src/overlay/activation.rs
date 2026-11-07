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
//! ([`crate::hotkey`]) and gives it back before its copy, and is a window of the game for the
//! desktop (`WM_TRANSIENT_FOR`), so that the game stays above the taskbar.
//!
//! Whether the game holds the pointer is asked when the window opens (`GrabPointer`, let go at
//! once). Wine may take it only a moment later, after the game became the active window again:
//! for a while after, the window looks for the window Wine holds the pointer in, without touching
//! the pointer (a try of the grab moves the pointer out of the window under it, and back).

use std::time::{Duration, Instant};

use x11rb::connection::Connection as _;
use x11rb::properties::{WmSizeHints, WmSizeHintsSpecification};
use x11rb::protocol::xproto::{
    AtomEnum, ConfigureWindowAux, ConnectionExt as _, CreateWindowAux, EventMask, GrabMode,
    GrabStatus, InputFocus, KeyButMask, MapState, PropMode, Window, WindowClass,
};
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as _;

use crate::x11::{self as shared, Atoms, Result};

/// How long the window over the game may take to be shown, then the desktop to activate the
/// helper, before it is given up: the window then behaves as on two screens.
const TIMEOUT: Duration = Duration::from_secs(1);
/// How often the next step is looked for while the window waits to show, or the desktop to
/// activate the helper.
const STEP_POLL: Duration = Duration::from_millis(30);
/// How long, after the window opens without the pointer held, or after another window became the
/// active one, the window looks for Wine holding the pointer: Wine takes it a second or two after
/// the game is the active window again.
const WATCH: Duration = Duration::from_secs(3);
/// How often the window looks for Wine holding the pointer meanwhile.
const WATCH_POLL: Duration = Duration::from_millis(250);

/// The desktop as the window over the game sees it, or a fake one in the tests.
pub trait Screen {
    /// Whether another application holds the pointer: it can't be taken, and is let go at once
    /// when it can. Not while a button is held, which gives the pointer to the application
    /// clicked for the time of the click.
    fn pointer_grabbed(&mut self) -> Result<bool>;
    /// Whether a window of another application, invisible, covering the screen, is above the
    /// `game`: the window Wine holds the pointer in. Read without touching the pointer.
    fn pointer_window_above(&mut self, game: Window) -> Result<bool>;
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
    /// The window over the game is closed, or it gave up.
    Idle,
    /// Open, the game not holding the pointer: looked for until `until`.
    Watching { until: Instant },
    /// The game holds the pointer: waiting for the window to show.
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

impl State {
    /// The game and the helper, from when the desktop is asked to activate the helper.
    fn helper(self) -> Option<(Window, Window)> {
        match self {
            Self::Activating { game, helper, .. } | Self::Active { game, helper } => {
                Some((game, helper))
            }
            Self::Idle | Self::Watching { .. } | Self::Waiting { .. } => None,
        }
    }

    /// The game the window gives the active window back to.
    fn game(self) -> Option<Window> {
        match self {
            Self::Waiting { game, .. }
            | Self::Activating { game, .. }
            | Self::Active { game, .. } => Some(game),
            Self::Idle | Self::Watching { .. } => None,
        }
    }
}

/// The connection to the X server: made when the window over the game first opens. A function
/// rather than a closure, so that it is copied out of the link when it is called.
enum Link<S> {
    Waiting(fn() -> Result<S>),
    Connected(S),
    /// It couldn't be made, which is written to the log once.
    Unavailable,
}

/// Makes the window over the game the active window while the game holds the pointer.
pub struct Activation<S: Screen = XScreen> {
    link: Link<S>,
    state: State,
    /// The opening of the window followed ([`super::Overlay::opened`]).
    opened: Option<u64>,
    /// The hotkey was pressed while the window was, or was becoming, the active one: the window
    /// that opens on its item becomes it again, without asking whether the game holds the
    /// pointer, which Wine only takes again a moment after the game is active once more; and
    /// when no item comes (an empty cell), the window becomes it again.
    carry: bool,
    /// The window closes with Esc: the game becomes the active window again. Never after a click,
    /// where the desktop activates the window clicked.
    escaped: bool,
    /// A field of the window was edited at the last frame.
    editing: bool,
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
            escaped: false,
            editing: false,
        }
    }

    /// The hotkey was pressed: the window that opens on its item makes itself the active window
    /// again when this one was.
    pub fn hotkey(&mut self) {
        self.carry = self.state.game().is_some();
    }

    /// The window is closed with Esc: the game becomes the active window again.
    pub fn escape(&mut self) {
        self.escaped = true;
    }

    /// Whether a field of the window is edited, at the end of each frame.
    pub fn editing(&mut self, editing: bool) {
        self.editing = editing;
    }

    /// Whether Esc closes the window: while it is the active window, unless a field was edited at
    /// the last frame, which the key leaves (egui takes the keyboard away at Esc before the
    /// frame reads it).
    #[must_use]
    pub fn escape_closes(&self) -> bool {
        self.holds() && !self.editing
    }

    /// The window over the game is the active window, or about to be: the keys go to it.
    #[must_use]
    pub fn holds(&self) -> bool {
        self.state.helper().is_some()
    }

    /// Follows the window over the game at each frame: `opened` while it is open (a new number
    /// each time it opens), `pending` while a hotkey waits for its item. When the next frame
    /// should come, while the window waits for something.
    pub fn follow(&mut self, opened: Option<u64>, pending: bool, now: Instant) -> Option<Duration> {
        if opened != std::mem::replace(&mut self.opened, opened) {
            let carry = std::mem::take(&mut self.carry);
            if opened.is_some() {
                self.open(carry, now);
            } else {
                let escaped = std::mem::take(&mut self.escaped);
                let state = std::mem::replace(&mut self.state, State::Idle);
                self.release(state, escaped);
            }
        }
        self.escaped = false;
        self.advance(pending, now);
        match self.state {
            State::Waiting { .. } | State::Activating { .. } => Some(STEP_POLL),
            State::Watching { .. } => Some(WATCH_POLL),
            State::Idle | State::Active { .. } => None,
        }
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
    /// when the game holds the pointer, or when it was the active window (`carry`); otherwise it
    /// looks for the game holding it a moment later.
    fn open(&mut self, carry: bool, now: Instant) {
        let state = self.state;
        let Some(screen) = self.screen() else { return };
        match open(screen, state, carry) {
            Ok(Some(game)) => self.state = State::Waiting { game, since: now },
            Ok(None) => {
                self.release(state, false);
                self.state = State::Watching { until: now + WATCH };
            }
            Err(error) => {
                log::warn!("The window over the game can't become the active window: {error}");
                self.release(state, true);
                self.state = State::Idle;
            }
        }
    }

    /// Moves on: when the window shows, when the desktop activates the helper, when the game
    /// holds the pointer while it is looked for, or when another window became the active one.
    fn advance(&mut self, pending: bool, now: Instant) {
        let state = self.state;
        if state == State::Idle {
            return;
        }
        let carry = self.carry;
        let Some(screen) = self.screen() else { return };
        match advance(
            screen,
            state,
            Moment {
                carry,
                pending,
                now,
            },
        ) {
            Ok(next) => {
                // An empty cell: the hotkey made the game active, and no item came.
                if matches!(state, State::Active { .. }) && matches!(next, State::Waiting { .. }) {
                    self.carry = false;
                }
                // Another window became the active one: the helper hides, the window stays open.
                if state.helper().is_some() && next.helper().is_none() {
                    self.release(state, false);
                }
                self.state = next;
            }
            Err(error) => {
                log::warn!("The window over the game can't become the active window: {error}");
                self.release(state, true);
                self.state = State::Idle;
            }
        }
    }

    /// Hides the helper of `state`; first makes the game the active window again when
    /// `give_back` and the helper still is the active window.
    fn release(&mut self, state: State, give_back: bool) {
        let Some((game, helper)) = state.helper() else {
            return;
        };
        let Some(screen) = self.screen() else { return };
        if give_back && let Err(error) = self::give_back(screen, game, helper) {
            log::warn!("The game can't be made the active window again: {error}");
        }
        if let Err(error) = screen.hide_helper() {
            log::warn!("The helper of the window over the game can't be hidden: {error}");
        }
    }
}

/// The application closing while the window over the game is the active window: the game becomes
/// it again, rather than whatever window the desktop would choose.
impl<S: Screen> Drop for Activation<S> {
    fn drop(&mut self) {
        let state = std::mem::replace(&mut self.state, State::Idle);
        self.release(state, true);
    }
}

/// What the state moves on with, besides the desktop.
#[derive(Debug, Clone, Copy)]
struct Moment {
    /// See [`Activation::carry`].
    carry: bool,
    /// A hotkey waits for its item.
    pending: bool,
    now: Instant,
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
    Ok(Some(if carry {
        state.game().unwrap_or(game)
    } else {
        game
    }))
}

/// The state after `state` at `moment`.
fn advance(screen: &mut impl Screen, state: State, moment: Moment) -> Result<State> {
    let now = moment.now;
    match state {
        State::Idle => Ok(state),
        State::Watching { until } => {
            if now > until {
                return Ok(State::Idle);
            }
            let Some(game) = screen.active()? else {
                return Ok(state);
            };
            // Confirmed by a try of the grab, once the window is seen.
            if screen.pointer_window_above(game)? && screen.pointer_grabbed()? {
                return Ok(State::Waiting { game, since: now });
            }
            Ok(state)
        }
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
        State::Active { game, helper } => {
            if screen.active()? == Some(helper) {
                return Ok(state);
            }
            // The hotkey made the game active for its copy: its item opens the window again
            // (`Activation::open`); when none comes (an empty cell), the window becomes the active
            // one again.
            if moment.carry {
                return Ok(if moment.pending {
                    state
                } else {
                    State::Waiting { game, since: now }
                });
            }
            // The player went to another window (Alt+Tab, a click in the game or the main
            // window): the window stays open, and becomes the active one again if the game holds
            // the pointer once more.
            log::info!(
                "Another window became the active window: the window over the game lets it be"
            );
            Ok(State::Watching { until: now + WATCH })
        }
    }
}

/// Makes the `game` the active window again, while the `helper` still is: not when the player
/// went to another window.
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
    /// The size of the screen of X11, which the window Wine holds the pointer in covers.
    size: (u16, u16),
    atoms: Atoms,
    /// The helper, created when it is first shown.
    helper: Option<Window>,
}

impl XScreen {
    fn connect() -> Result<Self> {
        let (connection, screen) = x11rb::connect(None)?;
        let setup = &connection.setup().roots[screen];
        let (root, size) = (setup.root, (setup.width_in_pixels, setup.height_in_pixels));
        let atoms = Atoms::intern(&connection)?;
        Ok(Self {
            connection,
            root,
            size,
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

    /// The children of the root window from the bottom, with their attributes, asked for all at
    /// once.
    fn children(&self) -> Result<Vec<(Window, x11rb::protocol::xproto::GetWindowAttributesReply)>> {
        shared::drop_events(&self.connection);
        let children = self.connection.query_tree(self.root)?.reply()?.children;
        let cookies = children
            .into_iter()
            .map(|window| Ok((window, self.connection.get_window_attributes(window)?)))
            .collect::<Result<Vec<_>>>()?;
        // A window gone meanwhile is left out.
        Ok(cookies
            .into_iter()
            .filter_map(|(window, cookie)| Some((window, cookie.reply().ok()?)))
            .collect())
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
        let children = self.children()?;
        let Some(at) = children.iter().position(|&(window, _)| window == game) else {
            return Ok(false);
        };
        for &(window, ref attributes) in &children[at + 1..] {
            if attributes.class != WindowClass::INPUT_ONLY
                || !attributes.override_redirect
                || attributes.map_state != MapState::VIEWABLE
            {
                continue;
            }
            let geometry = self.connection.get_geometry(window)?.reply()?;
            let covers = geometry.x <= 0
                && geometry.y <= 0
                && geometry.width >= self.size.0
                && geometry.height >= self.size.1;
            if covers && !shared::is_ours(&self.connection, self.atoms, window)? {
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
mod tests;
