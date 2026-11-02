//! The window over the game on Windows: the pointer and its buttons read by the click watch
//! ([`super::ClickWatch`]), and the focus given back to the game after a click in the window,
//! kept while one of its fields is edited ([`super::game_focus`]), through the safe Win32 calls
//! of `winsafe`.

use std::error::Error;
use std::sync::{Mutex, MutexGuard, PoisonError};

use winsafe::{HWND, co};

use super::Point;
use super::clicks::PointerSource;
use super::game_focus::{Desktop, Frame, GameFocus};

/// The buttons that count as a click: left, middle and right.
const BUTTONS: [co::VK; 3] = [co::VK::LBUTTON, co::VK::MBUTTON, co::VK::RBUTTON];

/// The pointer of Windows: `GetCursorPos`, and the buttons held (`GetAsyncKeyState`), in the
/// physical pixels of the screen, the application being aware of the scale of each screen
/// (winit).
pub(super) struct SystemPointer {
    /// Where the pointer was last read.
    last: Point,
    /// The pointer couldn't be read at the last reading, which the log told.
    failing: bool,
}

impl SystemPointer {
    #[expect(
        clippy::unnecessary_wraps,
        reason = "the interface of the pointer of X11, whose connection may fail"
    )]
    pub(super) fn connect() -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            last: Point { x: 0, y: 0 },
            failing: false,
        })
    }
}

impl PointerSource for SystemPointer {
    /// A pointer that can't be read for a moment (the secure desktop of a password or of an
    /// administrator prompt, the session locked) reads as no button held where it was last, so
    /// that the window over the game isn't turned off for good.
    fn read(&mut self) -> Result<(Point, bool), Box<dyn Error>> {
        match winsafe::GetCursorPos() {
            Ok(point) => {
                if std::mem::take(&mut self.failing) {
                    log::info!("The pointer can be read again");
                }
                self.last = Point {
                    x: point.x,
                    y: point.y,
                };
                let down = BUTTONS.into_iter().any(winsafe::GetAsyncKeyState);
                Ok((self.last, down))
            }
            Err(error) => {
                if !std::mem::replace(&mut self.failing, true) {
                    log::warn!("The pointer can't be read for now: {error}");
                }
                Ok((self.last, false))
            }
        }
    }
}

/// The game, shared with the price-check hotkey, which gives it the focus back before its copy
/// while a field of the window over the game is edited ([`give_keyboard_back`]). A static, since
/// a handle of window can't be copied without `unsafe` (`raw_copy` of `winsafe`): the window
/// over the game and the hotkey, on the thread of the window, take it in turn.
static GAME: Mutex<Game> = Mutex::new(Game {
    window: None,
    keyboard_held: false,
});

struct Game {
    /// The foreground window while the window over the game showed without the focus: the
    /// game, where the hotkey was pressed.
    window: Option<HWND>,
    /// The window over the game has the focus while one of its fields is edited.
    keyboard_held: bool,
}

/// Never kept across a call that may send messages to the windows of this thread (activating a
/// window), whose handling could take it again.
fn game() -> MutexGuard<'static, Game> {
    GAME.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Whether `window` belongs to this application.
fn is_ours(window: &HWND) -> bool {
    let (_, process) = window.GetWindowThreadProcessId();
    process == std::process::id()
}

/// Brings the game remembered to the foreground, allowed while this application has it, as
/// after a click in its window; `false` when none was seen, it is gone, or Windows refused.
fn activate_game() -> bool {
    // Taken out of the lock while Windows activates it.
    let Some(window) = game().window.take() else {
        return false;
    };
    let given = window.IsWindow() && window.SetForegroundWindow();
    // Unless another game was seen meanwhile.
    game().window.get_or_insert(window);
    given
}

/// At the price-check hotkey, before the keys of its copy: while a field of the window over the
/// game is edited, the window has the focus, so that the keys would go to it; the game gets it
/// back first, and the field is left at the next frame ([`super::Overlay::take_leave_field`]).
/// `true` when the focus was given back, the keys then going to the game.
pub fn give_keyboard_back() -> bool {
    if !std::mem::take(&mut game().keyboard_held) {
        return false;
    }
    // The player went to another window meanwhile, which keeps the focus.
    if !HWND::GetForegroundWindow().is_some_and(|window| is_ours(&window)) {
        return false;
    }
    let given = activate_game();
    if given {
        log::info!("The price-check hotkey gave the keyboard back to the game");
    } else {
        log::warn!("The keyboard can't be given back to the game: the copy goes to this window");
    }
    given
}

/// The windows of Windows, the game remembered in [`GAME`].
struct WindowsDesktop;

impl Desktop for WindowsDesktop {
    fn remember_game(&mut self) {
        if let Some(window) = HWND::GetForegroundWindow().filter(|window| !is_ours(window)) {
            game().window = Some(window);
        }
    }

    fn activate_game(&mut self) -> bool {
        activate_game()
    }
}

/// The keyboard of the window over the game on Windows: the game keeps it, given back after each
/// click in the window, except while one of its fields is edited.
pub struct KeyboardFocus {
    game: GameFocus<WindowsDesktop>,
    /// The window has the focus while one of its fields is edited.
    held: bool,
}

impl Default for KeyboardFocus {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyboardFocus {
    #[must_use]
    pub fn new() -> Self {
        Self {
            game: GameFocus::new(WindowsDesktop),
            held: false,
        }
    }

    /// Whether the window has the keyboard, which [`Self::give_back`] followed during the frame:
    /// a click in a field took it already.
    pub fn follow(&mut self, _editing: bool) -> bool {
        self.held
    }

    /// The window over the game has the keyboard: it has the focus, and one of its fields is
    /// edited.
    #[must_use]
    pub fn held(&self) -> bool {
        self.held
    }

    /// At each frame of the window over the game, once its content is drawn: gives the focus
    /// back to the game once a click in the window took it and its buttons are released, unless
    /// the click opens a link, the browser then coming to the front, or one of its fields is
    /// edited, until it is left.
    pub fn give_back(&mut self, frame: Frame) {
        self.game.follow(frame);
        let held = frame.focused == Some(true) && frame.editing;
        game().keyboard_held = held;
        if held && !self.held {
            log::info!("The window over the game took the keyboard");
        }
        self.held = held;
    }
}
