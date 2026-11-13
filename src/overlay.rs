//! The window over the game: the price beside the panel of the game the item is in, opened by
//! the price-check hotkey and closed by a click in the game (`specs.md`, Window over the game).
//!
//! This module decides where the window goes, and when it opens and closes, without the system:
//! the pointer and the game window come with the hotkey ([`crate::hotkey`]), the clicks from a
//! thread that reads the pointer of each system ([`ClickWatch`]), and the application draws the
//! window. The keyboard goes to the window while one of its fields is edited ([`KeyboardFocus`]);
//! on Windows, the game gets it back after each other click in the window. On Linux, while the
//! game holds the pointer (a single screen), the window becomes the active window ([`Activation`]).

#[cfg(target_os = "linux")]
mod activation;
#[cfg(any(target_os = "linux", target_os = "windows"))]
mod clicks;
// The rules of the focus given back on Windows, tested on Linux too.
#[cfg(any(target_os = "windows", test))]
mod game_focus;
#[cfg(target_os = "linux")]
mod keyboard;
#[cfg(not(target_os = "linux"))]
mod unsupported;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "linux")]
mod x11;

// The pointer of each system, read by the click watch.
#[cfg(target_os = "windows")]
use self::windows as system;
#[cfg(target_os = "linux")]
use self::x11 as system;

#[cfg(target_os = "linux")]
pub use self::activation::Activation;
#[cfg(any(target_os = "linux", target_os = "windows"))]
pub use self::clicks::ClickWatch;
#[cfg(target_os = "windows")]
pub use self::game_focus::{Frame, KeyboardBack};
#[cfg(target_os = "linux")]
pub use self::keyboard::KeyboardFocus;
#[cfg(not(target_os = "linux"))]
pub use self::unsupported::Activation;
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub use self::unsupported::{ClickWatch, KeyboardFocus};
#[cfg(target_os = "windows")]
pub use self::windows::{KeyboardFocus, give_keyboard_back, is_ours};

use std::time::{Duration, Instant};

use crate::hotkey::COPY_WAIT;

/// How long a hotkey waits for its item before it is forgotten: the clipboard watching takes an
/// item for its copy within [`COPY_WAIT`], then the item reaches the window at the next frame, up
/// to 200 ms later while the main window is minimized. Only for the log: the item of a hotkey
/// is known by the instant it carries.
const FORGOTTEN_AFTER: Duration = COPY_WAIT.saturating_add(Duration::from_secs(1));

/// The width of the window, in points: the one of the narrow layout.
pub const WIDTH: f32 = 420.0;
/// The width of a panel of the game (inventory, stash) for its height: 370 for 600, the ratio
/// Exiled Exchange 2 uses (`uiSidebarWidth` in `main/src/windowing/GameWindow.ts`).
const PANEL_WIDTH_PER_HEIGHT: (i32, i32) = (370, 600);

/// A point of the screen, in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

/// A rectangle of the screen, in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl ScreenRect {
    fn right(&self) -> i32 {
        self.x + self.width
    }

    fn bottom(&self) -> i32 {
        self.y + self.height
    }

    #[must_use]
    pub fn contains(&self, point: Point) -> bool {
        (self.x..self.right()).contains(&point.x) && (self.y..self.bottom()).contains(&point.y)
    }
}

/// Where the hotkey was pressed: the pointer, and the window of the game, which has the focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target {
    pub pointer: Point,
    pub game: ScreenRect,
}

impl Target {
    /// The place of the window, `width` wide, with its content `content_height` high (physical
    /// pixels): beside the panel of the game the item is in, which it never covers (left of the
    /// inventory when the pointer is on the right half of the game, right of the stash
    /// otherwise), its top level with the pointer, moved up as much as the game needs; at most
    /// the height of the game, its content then scrolls.
    #[must_use]
    pub fn place(&self, width: i32, content_height: i32) -> ScreenRect {
        let game = self.game;
        let (panel_width, per_height) = PANEL_WIDTH_PER_HEIGHT;
        let panel = game.height * panel_width / per_height;
        let height = content_height.min(game.height).max(1);
        let x = if self.pointer.x - game.x > game.width / 2 {
            game.right() - panel - width
        } else {
            game.x + panel
        };
        let y = self.pointer.y.min(game.bottom() - height).max(game.y);
        ScreenRect {
            x,
            y,
            width,
            height,
        }
    }
}

/// A hotkey whose item hasn't arrived yet.
#[derive(Debug, Clone, Copy)]
struct Pending {
    target: Option<Target>,
    /// When it prepared the clipboard, which its item carries.
    copy: Instant,
    told: Instant,
}

/// When the window shows: from the item the hotkey copied until a click in the game.
#[derive(Debug, Default)]
pub struct Overlay {
    /// Where the window shows, while it is open.
    shown: Option<Target>,
    /// The last hotkey that prepared the clipboard, until its item arrives.
    pending: Option<Pending>,
    /// Turned off for good: it would no longer close.
    off: bool,
    /// The hotkey was pressed since the last frame of the window: it gave the keyboard back to
    /// the game, so that the field edited in the window is left.
    leave_field: bool,
    /// How many times the window opened, or opened again on another item.
    openings: u64,
}

impl Overlay {
    /// The hotkey, told at `now`, prepared the clipboard for its copy at `copy`: the window
    /// opens at `target` when its item arrives. Without target (the pointer or the game couldn't
    /// be read) or without copy (the clipboard couldn't be prepared), its item only shows in
    /// the main window; it replaces the hotkey before it all the same, whose item would
    /// otherwise open the window at the wrong place.
    pub fn hotkey(&mut self, target: Option<Target>, copy: Option<Instant>, now: Instant) {
        self.leave_field = true;
        self.pending = copy.map(|copy| Pending {
            target,
            copy,
            told: now,
        });
    }

    /// An item arrived, which the hotkey that prepared the clipboard at `hotkey` copied, if
    /// any (the clipboard watching tells it). The item of the last hotkey opens the window
    /// where it was pressed, or moves it there (`true`); any other item (copied with Ctrl+C)
    /// only shows in the main window, and closes the window over the game, which would show it
    /// beside an item it isn't. So does the item of an older hotkey, arriving after a newer one
    /// was told (two quick hotkeys while the main window is minimized): the window then opens
    /// again with the item of the newer one, without ever showing an item at the wrong place.
    pub fn copied(&mut self, hotkey: Option<Instant>) -> bool {
        match self.pending {
            Some(Pending { target, copy, .. }) if hotkey == Some(copy) => {
                self.pending = None;
                // Without target, the window would show the item at the place of another: it
                // closes. A hotkey pressed just before the window was turned off opens nothing.
                if !self.off {
                    self.shown = target;
                    self.openings += 1;
                }
                true
            }
            _ => {
                self.shown = None;
                false
            }
        }
    }

    /// Whether the field edited in the window is left, once per press of the hotkey, which gave
    /// the keyboard back to the game.
    pub fn take_leave_field(&mut self) -> bool {
        std::mem::take(&mut self.leave_field)
    }

    /// Closes the window and never opens it again, when the clicks that close it can no longer
    /// be read; `true` the first time.
    pub fn turn_off(&mut self) -> bool {
        self.shown = None;
        !std::mem::replace(&mut self.off, true)
    }

    /// Forgets a hotkey that copied nothing; while one is pending, the time left before it is
    /// forgotten.
    pub fn expire(&mut self, now: Instant) -> Option<Duration> {
        let pending = self.pending?;
        let left = FORGOTTEN_AFTER.checked_sub(now.saturating_duration_since(pending.told));
        if left.is_none() {
            self.pending = None;
            log::info!("No item came from the price-check hotkey (an empty cell?)");
        }
        left
    }

    /// A button of the mouse pressed at `point`: outside the `window` (in the game), it closes
    /// it; in it, the window takes the click.
    pub fn clicked(&mut self, point: Point, window: Option<ScreenRect>) {
        if self.shown.is_some() && !window.is_some_and(|window| window.contains(point)) {
            self.shown = None;
        }
    }

    /// A hotkey waits for its item.
    #[must_use]
    pub fn waiting(&self) -> bool {
        self.pending.is_some()
    }

    /// Where the window shows, while it is open.
    #[must_use]
    pub fn shown(&self) -> Option<Target> {
        self.shown
    }

    /// While the window is open, a number that changes each time it opens, or opens again on the
    /// item of another hotkey.
    #[must_use]
    pub fn opened(&self) -> Option<u64> {
        self.shown.map(|_| self.openings)
    }

    /// Closes the window (Esc, while it is the active window on a single screen).
    pub fn close(&mut self) {
        self.shown = None;
    }
}

#[cfg(test)]
mod tests;
