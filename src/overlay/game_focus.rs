//! The focus given back to the game after a click in the window over the game, on Windows
//! (`specs.md`, Window over the game, Focus and Keyboard).
//!
//! winit can't make a window that is never activated (`WS_EX_NOACTIVATE`), and `unsafe` is
//! forbidden in the application: a click in the window over the game activates it, and the game
//! loses the keyboard. The window then gives the focus back to the game, as Exiled Exchange 2
//! does (`focusTarget` of electron-overlay-window), once the buttons are released, so that the
//! window still gets the release of its click (a checkbox, Search). A click in a min or max
//! field keeps it, so that the number typed reaches the field, until no field is edited any more
//! (Enter, Esc, a click elsewhere in the window). A click that opens a link (Check on trade)
//! leaves the foreground to the browser: Windows lets the browser come to the front only while
//! this application has the foreground.
//!
//! Without the system, so that the rules are tested on Linux too: the windows come from a
//! [`Desktop`].

/// The windows of the desktop, or fake ones in the tests.
pub trait Desktop {
    /// Whether the foreground window belongs to this application.
    fn foreground_is_ours(&mut self) -> bool;
    /// Remembers the foreground window as the game, unless it is a window of this application.
    fn remember_game(&mut self);
    /// Brings the game remembered to the foreground; `false` when none was seen, it is gone, or
    /// the system refused.
    fn activate_game(&mut self) -> bool;
}

/// What a frame of the window over the game did with the focus, for the tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gave {
    /// Nothing to give back: the window doesn't have the focus, or gave it back already, or a
    /// button is still held in it, or one of its fields is edited.
    Nothing,
    /// Given back to the game.
    Given,
    /// Left to the browser, which the click opened.
    Left,
    /// The system refused it, or the game is gone, or was never seen.
    Failed,
}

/// What a frame of the window over the game saw, once its content is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frame {
    /// Whether the window has the focus; none while it is hidden.
    pub focused: Option<bool>,
    /// A button of the mouse is held in it.
    pub pressed: bool,
    /// The frame opens a link in the browser.
    pub opens_link: bool,
    /// One of its fields is edited.
    pub editing: bool,
}

impl Frame {
    /// A frame of the window hidden.
    pub const HIDDEN: Self = Self {
        focused: None,
        pressed: false,
        opens_link: false,
        editing: false,
    };

    /// The window has the keyboard: it has the focus, and one of its fields is edited.
    #[must_use]
    pub fn holds_keyboard(self) -> bool {
        self.focused == Some(true) && self.editing
    }
}

/// What the price-check hotkey did with the focus before the keys of its copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyboardBack {
    /// No field of the window over the game was edited, or another window took the focus
    /// meanwhile: the keys go to the foreground window.
    NotHeld,
    /// The game is in front again, the keys go to it.
    Given,
    /// The system refused it, or the game is gone: the keys would go to the window over the
    /// game.
    Refused,
}

/// At the price-check hotkey, before the keys of its copy: while a field of the window over the
/// game is edited (`held`, as the last frame saw it), the window has the focus, so that the keys
/// would go to it; the game gets it back first, and the field is left at the next frame
/// ([`super::Overlay::take_leave_field`]).
pub fn give_keyboard_back(desktop: &mut impl Desktop, held: bool) -> KeyboardBack {
    // The player went to another window meanwhile, which keeps the focus.
    if !held || !desktop.foreground_is_ours() {
        return KeyboardBack::NotHeld;
    }
    if desktop.activate_game() {
        log::info!("The price-check hotkey gave the keyboard back to the game");
        KeyboardBack::Given
    } else {
        log::warn!("The keyboard can't be given back to the game: the hotkey copies nothing");
        KeyboardBack::Refused
    }
}

/// Gives the focus back to the game each time a click in the window over the game takes it.
pub struct GameFocus<D: Desktop> {
    desktop: D,
    /// The window over the game took the focus, and it was given back already (or failed).
    handled: bool,
}

impl<D: Desktop> GameFocus<D> {
    pub fn new(desktop: D) -> Self {
        Self {
            desktop,
            handled: false,
        }
    }

    /// At each frame of the window over the game, once its content is drawn.
    pub fn follow(&mut self, frame: Frame) -> Gave {
        let Frame {
            focused,
            pressed,
            opens_link,
            editing,
        } = frame;
        match focused {
            None => {
                self.handled = false;
                Gave::Nothing
            }
            Some(false) => {
                self.handled = false;
                // The game, where the hotkey was pressed, while the window shows without the
                // focus.
                self.desktop.remember_game();
                Gave::Nothing
            }
            Some(true) if self.handled || pressed || editing => Gave::Nothing,
            Some(true) if opens_link => {
                self.handled = true;
                Gave::Left
            }
            Some(true) => {
                self.handled = true;
                if self.desktop.activate_game() {
                    log::info!("The window over the game gave the focus back to the game");
                    Gave::Given
                } else {
                    log::warn!(
                        "The window over the game can't give the focus back to the game: a click \
                         in the game takes it back"
                    );
                    Gave::Failed
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
