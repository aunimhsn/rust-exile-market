//! The stand-ins of what a system lacks of the window over the game, with the interface of the
//! real ones and no state: the activation of Linux elsewhere, and everything on macOS, where
//! Path of Exile 2 has no client.

use std::time::Duration;

#[cfg(not(target_os = "windows"))]
use super::Point;

/// The window over the game never becomes the active window on this system: only Wine holds
/// the pointer of the game, on Linux.
#[derive(Default)]
pub struct Activation;

#[expect(
    clippy::unused_self,
    reason = "the interface of the activation of Linux, without its state"
)]
impl Activation {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    pub fn hotkey(&mut self) {}

    /// Never the active window.
    #[must_use]
    pub fn holds(&self) -> bool {
        false
    }

    pub fn escape(&mut self) {}

    pub fn editing(&mut self, _editing: bool) {}

    /// Esc is left to the game.
    #[must_use]
    pub fn escape_closes(&self) -> bool {
        false
    }

    /// Never the active window, never lost.
    #[must_use]
    pub fn lost(&self) -> bool {
        false
    }

    /// Nothing to wait for.
    pub fn follow(
        &mut self,
        _opened: Option<u64>,
        _pending: impl Fn() -> bool,
        _now: std::time::Instant,
    ) -> Option<Duration> {
        None
    }
}

/// No window over the game on this system (macOS, where Path of Exile 2 has no client): no
/// click to watch.
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub struct ClickWatch;

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
#[expect(
    clippy::unused_self,
    reason = "the interface of the click watch of Linux and Windows, without its state"
)]
impl ClickWatch {
    #[must_use]
    pub fn start(_wake: impl Fn() + Send + 'static) -> Self {
        Self
    }

    pub fn watch(&mut self, _watching: bool) {}

    /// No click, and nothing that stopped.
    pub fn receive(&mut self, _handle: impl FnMut(Point)) -> bool {
        true
    }
}

/// No window over the game on this system (macOS): no keyboard to give.
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
#[derive(Default)]
pub struct KeyboardFocus;

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
#[expect(
    clippy::unused_self,
    reason = "the interface of the keyboard of Linux, without its state"
)]
impl KeyboardFocus {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Never held.
    pub fn follow(&mut self, _editing: bool) -> bool {
        false
    }

    /// Never held.
    #[must_use]
    pub fn held(&self) -> bool {
        false
    }
}
