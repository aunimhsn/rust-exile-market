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
//! the desktop manages, 1 × 1 under it, invisible and out of the taskbar (the helper), is
//! activated instead, and the window over the game takes the keyboard from it (`SetInputFocus`).
//! The helper carries the game (`_EXILE_MARKET_GAME`), so that the hotkey takes it for the game
//! ([`crate::hotkey`]) and gives it back before its copy, and is a window of the game for the
//! desktop (`WM_TRANSIENT_FOR`), so that the game stays above the taskbar.
//!
//! Wine holds the pointer in a window of its own, invisible, covering the screen, above the game:
//! the window looks for it without touching the pointer, then makes sure with a try of the grab
//! (`GrabPointer`, let go at once), which moves the pointer out of the window under it and back.
//! Wine may take the pointer only a moment after the game became the active window again: for a
//! while after the window opens, or after another window became the active one, the window looks
//! for it again.
//!
//! The hotkey makes the game the active window again for its copy: while a press is on its way,
//! the window neither takes the game made active for a window the player went to, nor becomes
//! the active window, which would take the keys of the copy.

use std::time::{Duration, Instant};

use x11rb::protocol::xproto::Window;

use crate::x11::Result;

mod x_screen;

use x_screen::XScreen;

/// How long the window over the game may take to be shown, then the desktop to activate the
/// helper, before it is given up: the window then behaves as on two screens.
const TIMEOUT: Duration = Duration::from_secs(1);
/// How often the next step is looked for while the window waits to show, or the desktop to
/// activate the helper.
const STEP_POLL: Duration = Duration::from_millis(30);
/// How long, after the window opens without the pointer held, or after the game became the
/// active window again, the window looks for Wine holding the pointer: Wine takes it a second or
/// two after the game is the active window again.
const WATCH: Duration = Duration::from_secs(3);
/// How often the window looks for Wine holding the pointer meanwhile, whatever the frames.
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
    /// Open, the game not holding the pointer: looked for at `next`, until `until`, while the
    /// `game` is the active window (any window when it isn't known).
    Watching {
        until: Instant,
        next: Instant,
        game: Option<Window>,
    },
    /// The game holds the pointer: waiting for the window to show, and for no press of the
    /// hotkey to be on its way. The helper still `shown` when the window opens again on another
    /// item, or becomes the active window again.
    Waiting {
        game: Window,
        since: Instant,
        shown: Option<Window>,
    },
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
    /// Looks for the game holding the pointer from `next`, for a while from `now`.
    fn watching(now: Instant, next: Instant, game: Option<Window>) -> Self {
        Self::Watching {
            until: now + WATCH,
            next,
            game,
        }
    }

    /// The game and the helper, while the window is or is about to be the active window: the keys
    /// go to it.
    fn helper(self) -> Option<(Window, Window)> {
        match self {
            Self::Activating { game, helper, .. } | Self::Active { game, helper } => {
                Some((game, helper))
            }
            Self::Idle | Self::Watching { .. } | Self::Waiting { .. } => None,
        }
    }

    /// The game and the helper, while the helper is shown: hidden when the state is left for one
    /// without it.
    fn shown(self) -> Option<(Window, Window)> {
        match self {
            Self::Waiting {
                game,
                shown: Some(helper),
                ..
            } => Some((game, helper)),
            _ => self.helper(),
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

/// The connection to the X server: made when the window over the game first opens, and again at
/// the next opening once it failed.
enum Link<S> {
    Waiting,
    Connected(S),
    /// It couldn't be made, which is written to the log once.
    Unavailable,
}

/// Makes the window over the game the active window while the game holds the pointer.
pub struct Activation<S: Screen = XScreen> {
    /// A function rather than a closure, so that it is copied out when it is called.
    connect: fn() -> Result<S>,
    link: Link<S>,
    state: State,
    /// The state at the start of the last [`Self::follow`], which [`Self::lost`] compares.
    before: State,
    /// The opening of the window followed ([`super::Overlay::opened`]).
    opened: Option<u64>,
    /// The hotkey was pressed while the window was, or was becoming, the active one: the window
    /// that opens on its item becomes it again, without asking whether the game holds the
    /// pointer, which Wine only takes again a moment after the game is active once more; and
    /// when no item comes (an empty cell), the window becomes it again. Forgotten once no press
    /// waits for its item.
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
            connect,
            link: Link::Waiting,
            state: State::Idle,
            before: State::Idle,
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

    /// Whether the keys are the game's while they go to the window: while it is the active
    /// window, unless a field was edited at the last frame (egui takes the keyboard away at Esc
    /// before the frame reads it). Then only Esc does something, and closes the window.
    #[must_use]
    pub fn escape_closes(&self) -> bool {
        self.holds() && !self.editing
    }

    /// The window over the game is the active window, or about to be: the keys go to it.
    #[must_use]
    pub fn holds(&self) -> bool {
        self.state.helper().is_some()
    }

    /// The window over the game stopped being the active window at this frame: it lost the
    /// keyboard, which winit may only tell at a later frame.
    #[must_use]
    pub fn lost(&self) -> bool {
        self.before.helper().is_some() && !self.holds()
    }

    /// Follows the window over the game at each frame: `opened` while it is open (a new number
    /// each time it opens), `pending` while a press of the hotkey is on its way or waits for its
    /// item, read after the active window. When the next frame should come, while the window
    /// waits for something.
    pub fn follow(
        &mut self,
        opened: Option<u64>,
        pending: impl Fn() -> bool,
        now: Instant,
    ) -> Option<Duration> {
        self.before = self.state;
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
        self.advance(&pending, now);
        // Its item came, or never will (an empty cell).
        if !pending() {
            self.carry = false;
        }
        match self.state {
            State::Waiting { .. } | State::Activating { .. } => Some(STEP_POLL),
            State::Watching { next, .. } => {
                Some(next.saturating_duration_since(now).max(STEP_POLL))
            }
            State::Idle | State::Active { .. } => None,
        }
    }

    fn screen(&mut self) -> Option<&mut S> {
        if let Link::Waiting = self.link {
            self.link = match (self.connect)() {
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
            Link::Waiting | Link::Unavailable => None,
        }
    }

    /// The window opens, or opens again on another item: it waits to become the active window
    /// when the game holds the pointer, or when it was, or was becoming, the active window;
    /// otherwise it looks for the game holding it a moment later.
    fn open(&mut self, carry: bool, now: Instant) {
        let state = self.state;
        // The hotkey made the game the active window for its copy, so the window was the active
        // one when it was pressed.
        let carry = carry || state.shown().is_some();
        let Some(screen) = self.screen() else { return };
        let next = open(screen, state, carry, now);
        self.step(state, next);
    }

    /// Moves on: when the window shows, when the desktop activates the helper, when the game
    /// holds the pointer while it is looked for, or when another window became the active one.
    fn advance(&mut self, pending: &dyn Fn() -> bool, now: Instant) {
        let state = self.state;
        if state == State::Idle {
            return;
        }
        let carry = self.carry;
        let Some(screen) = self.screen() else { return };
        let next = advance(
            screen,
            state,
            Moment {
                carry,
                pending,
                now,
            },
        );
        self.step(state, next);
    }

    /// Goes from `state` to `next`: the helper hidden when `next` doesn't show it; on a failure,
    /// the game given back, and the connection made again at the next opening.
    fn step(&mut self, state: State, next: Result<State>) {
        match next {
            Ok(next) => {
                if state.shown().is_some() && next.shown().is_none() {
                    self.release(state, true);
                }
                self.state = next;
            }
            Err(error) => {
                log::warn!("The window over the game can't become the active window: {error}");
                self.release(state, true);
                self.state = State::Idle;
                self.link = Link::Waiting;
            }
        }
    }

    /// Hides the helper of `state`; first makes the game the active window again when
    /// `give_back` and the helper still is the active window. Never connects.
    fn release(&mut self, state: State, give_back: bool) {
        let Some((game, helper)) = state.shown() else {
            return;
        };
        let Link::Connected(screen) = &mut self.link else {
            return;
        };
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
#[derive(Clone, Copy)]
struct Moment<'a> {
    /// See [`Activation::carry`].
    carry: bool,
    /// A press of the hotkey is on its way or waits for its item. Read after the active window
    /// where a change of it is taken for the player's (`Active`): the press is counted before
    /// its copy makes the game active.
    pending: &'a dyn Fn() -> bool,
    now: Instant,
}

/// The state of the window over the game opening over the active window, after `state`: waiting
/// to become the active window when it was, or was becoming, the active window (`carry`) at the
/// hotkey, whose copy gave the active window back to the game, or when the game holds the
/// pointer; otherwise looking for the game holding it a moment later.
fn open(screen: &mut impl Screen, state: State, carry: bool, now: Instant) -> Result<State> {
    let shown = state.shown().map(|(_, helper)| helper);
    if carry && let Some(game) = state.game() {
        return Ok(State::Waiting {
            game,
            since: now,
            shown,
        });
    }
    let Some(active) = screen.active()? else {
        return Ok(State::watching(now, now, None));
    };
    // The copy of the hotkey may not have given the active window back yet.
    let game = screen.game_behind(active)?.unwrap_or(active);
    if screen.pointer_window_above(game)? && screen.pointer_grabbed()? {
        return Ok(State::Waiting {
            game,
            since: now,
            shown,
        });
    }
    Ok(State::watching(now, now + WATCH_POLL, Some(game)))
}

/// The state after `state` at `moment`.
fn advance(screen: &mut impl Screen, state: State, moment: Moment<'_>) -> Result<State> {
    let now = moment.now;
    match state {
        State::Idle => Ok(state),
        State::Watching { until, next, game } => {
            if now > until {
                return Ok(State::Idle);
            }
            // The copy of a press may make the game active, and must not lose its keys.
            if now < next || (moment.pending)() {
                return Ok(state);
            }
            let watching = State::Watching {
                until,
                next: now + WATCH_POLL,
                game,
            };
            let Some(active) = screen.active()? else {
                return Ok(watching);
            };
            if game.is_some_and(|game| game != active) {
                // Away from the game: the wait counts from the return to it, where Wine
                // takes the pointer again. Counted from the leave, the window was left
                // without hover nor click after more than `WATCH` elsewhere.
                return Ok(State::watching(now, now + WATCH_POLL, game));
            }
            // Confirmed by a try of the grab, once the window is seen.
            if screen.pointer_window_above(active)? && screen.pointer_grabbed()? {
                return Ok(State::Waiting {
                    game: active,
                    since: now,
                    shown: None,
                });
            }
            Ok(watching)
        }
        State::Waiting { game, since, shown } => show(screen, game, since, shown, moment),
        State::Activating {
            game,
            helper,
            overlay,
            since,
        } => {
            // A press while the desktop activates the helper: activated again once its copy is
            // sent, which may give the active window back to the game meanwhile.
            if (moment.pending)() {
                return Ok(State::Waiting {
                    game,
                    since: now,
                    shown: Some(helper),
                });
            }
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
            // (`Activation::open`).
            if (moment.pending)() {
                return Ok(state);
            }
            // No item came (an empty cell): the window becomes the active one again.
            if moment.carry {
                return Ok(State::Waiting {
                    game,
                    since: now,
                    shown: Some(helper),
                });
            }
            // The player went to another window (Alt+Tab, a click in the game or the main
            // window): the window stays open, and becomes the active one again if the game holds
            // the pointer once more.
            log::info!(
                "Another window became the active window: the window over the game lets it be"
            );
            Ok(State::watching(now, now + WATCH_POLL, Some(game)))
        }
    }
}

/// The state after waiting for the window over the game since `since`, the helper still `shown`
/// or not: the helper shown and activated once the window shows.
fn show(
    screen: &mut impl Screen,
    game: Window,
    since: Instant,
    shown: Option<Window>,
    moment: Moment<'_>,
) -> Result<State> {
    let now = moment.now;
    let state = State::Waiting { game, since, shown };
    // The keys of the copy of a press would go to the window: once it is sent.
    if (moment.pending)() {
        return Ok(State::Waiting {
            game,
            since: now,
            shown,
        });
    }
    // Only from the game, or from its helper still active: never over a window the player went
    // to meanwhile (Alt+Tab while a hotkey on an empty cell waited for its item), which the game,
    // brought to the front with its helper, would cover.
    let active = screen.active()?;
    let from_ours = active.is_some_and(|active| active == game || shown == Some(active));
    if !from_ours {
        log::info!("Another window became the active window: the window over the game lets it be");
        return Ok(State::watching(now, now + WATCH_POLL, Some(game)));
    }
    let Some(overlay) = screen.overlay()? else {
        return Ok(if now.saturating_duration_since(since) > TIMEOUT {
            log::warn!("The window over the game didn't show: it stays as on two screens");
            State::Idle
        } else {
            state
        });
    };
    let helper = screen.show_helper(overlay, game)?;
    // A press since `pending` was read: its keys first.
    if (moment.pending)() {
        return Ok(State::Waiting {
            game,
            since: now,
            shown: Some(helper),
        });
    }
    screen.activate(helper)?;
    Ok(State::Activating {
        overlay,
        game,
        helper,
        since: now,
    })
}

/// Makes the `game` the active window again, while the `helper` still is: not when the player
/// went to another window.
fn give_back(screen: &mut impl Screen, game: Window, helper: Window) -> Result<()> {
    if screen.active()? == Some(helper) {
        screen.activate(game)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
