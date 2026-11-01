//! The copy sent to the game under Windows: C pressed through `SendInput`, as the player would
//! press it, in the window that has the focus.

use std::error::Error;

use enigo::{Direction, Enigo, Key, Keyboard as _, Settings};
use global_hotkey::HotKeyState;

/// At the press of D: `RegisterHotKey` takes D only, so that the keys sent then reach the game,
/// and Ctrl is held, since the hotkey needs it. The press reaches the application as a message of
/// the thread of its window, handled within a few milliseconds: a Ctrl released sooner (the
/// window busy, the key barely tapped) would let C reach the game alone.
pub const SENT_AT: HotKeyState = HotKeyState::Pressed;

/// Sends the keys of the copy.
pub struct CopySender {
    keyboard: Enigo,
}

impl CopySender {
    /// # Errors
    ///
    /// When the keys can't be sent on this system.
    pub fn connect() -> Result<Self, Box<dyn Error>> {
        let settings = Settings {
            // The keys it holds when dropped: none, since C is pressed and released at once.
            release_keys_when_dropped: false,
            ..Settings::default()
        };
        Ok(Self {
            keyboard: Enigo::new(&settings)?,
        })
    }

    /// Sends C to the window that has the focus: with the Ctrl the player holds, the game reads
    /// Ctrl+C. Ctrl is neither pressed nor released, so that the keyboard is left as the player
    /// holds it, as Exiled Exchange 2 does on Windows: a Ctrl released while the player holds it
    /// would make their next D reach the game alone.
    pub fn send_copy(&mut self) {
        // The virtual key of C, which Windows maps to the key that writes C in the layout of the
        // window that has the focus.
        match self.keyboard.key(Key::C, Direction::Click) {
            // Windows doesn't tell a key it drops: a game run as administrator never gets it.
            Ok(()) => {
                log::info!("Copy requested by the price-check hotkey (C, Ctrl held by the player)")
            }
            Err(error) => log::warn!("The copy can't be sent: {error}"),
        }
    }
}
