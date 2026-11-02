//! The window over the game on Windows: the pointer and its buttons read by the click watch
//! ([`super::ClickWatch`]), and the focus given back to the game after a click in the window
//! ([`super::game_focus`]), through the safe Win32 calls of `winsafe`.

use std::error::Error;

use winsafe::{HWND, co};

use super::Point;
use super::clicks::PointerSource;
use super::game_focus::{Desktop, GameFocus};

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

/// The windows of Windows.
struct WindowsDesktop;

impl Desktop for WindowsDesktop {
    type Window = HWND;

    fn foreground_not_ours(&mut self) -> Option<HWND> {
        let window = HWND::GetForegroundWindow()?;
        let (_, process) = window.GetWindowThreadProcessId();
        (process != std::process::id()).then_some(window)
    }

    /// Allowed while this application has the foreground, as after a click in its window.
    fn activate(&mut self, window: &HWND) -> bool {
        window.IsWindow() && window.SetForegroundWindow()
    }
}

/// The keyboard of the window over the game on Windows: the game keeps it, given back after each
/// click in the window; its fields take it in step 14b3b.
pub struct KeyboardFocus {
    game: GameFocus<WindowsDesktop>,
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
        }
    }

    /// Never held (step 14b3b).
    #[expect(
        clippy::unused_self,
        reason = "the interface of the keyboard of Linux; the fields take it in step 14b3b"
    )]
    pub fn follow(&mut self, _editing: bool) -> bool {
        false
    }

    /// Never held (step 14b3b).
    #[expect(
        clippy::unused_self,
        reason = "the interface of the keyboard of Linux; the fields take it in step 14b3b"
    )]
    #[must_use]
    pub fn held(&self) -> bool {
        false
    }

    /// At each frame of the window over the game: gives the focus back to the game once a click
    /// in the window took it (`focused`, none while it is hidden) and its buttons are released
    /// (`pressed`).
    pub fn give_back(&mut self, focused: Option<bool>, pressed: bool) {
        self.game.follow(focused, pressed);
    }
}
