//! The price-check hotkey: Ctrl+D pressed in the game copies the item under the cursor, so
//! that the player no longer presses Ctrl+C.
//!
//! The application sends Ctrl+C to the game itself, and the item arrives through the clipboard
//! like any copy ([`crate::clipboard`]). On Linux through X11 (XWayland under Wayland), like the
//! game run by Proton, and on Windows; macOS comes in step 14a2b.

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "linux")]
mod x11;

#[cfg(target_os = "windows")]
use windows as keys;
#[cfg(target_os = "linux")]
use x11 as keys;

/// The hotkey listened to for as long as this value lives.
pub struct PriceCheckHotkey {
    /// Listens until it is dropped.
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    _manager: Option<global_hotkey::GlobalHotKeyManager>,
}

impl PriceCheckHotkey {
    /// Starts listening to Ctrl+D. A failure (the key taken by another application, such as
    /// Exiled Exchange 2) only writes to the log: the player can still copy with Ctrl+C.
    ///
    /// Called once: the handler of the hotkey events can only be set once.
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    #[must_use]
    pub fn start() -> Self {
        use global_hotkey::hotkey::{Code, HotKey, Modifiers};
        use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager};
        use std::sync::{Mutex, PoisonError};

        let sender = match keys::CopySender::connect() {
            Ok(sender) => Mutex::new(sender),
            Err(error) => {
                log::warn!("The price-check hotkey can't listen: {error}");
                return Self { _manager: None };
            }
        };
        let manager = match GlobalHotKeyManager::new() {
            Ok(manager) => manager,
            Err(error) => {
                log::warn!("The price-check hotkey can't listen: {error}");
                return Self { _manager: None };
            }
        };
        let hotkey = HotKey::new(Some(Modifiers::CONTROL), Code::KeyD);
        let id = hotkey.id();
        GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
            // At the press or at the release of D, as each system lets the keys sent reach the
            // game.
            if event.id == id && event.state == keys::SENT_AT {
                sender
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .send_copy();
            }
        }));
        match manager.register(hotkey) {
            Ok(()) => log::info!("Price-check hotkey Ctrl+D listened to"),
            Err(error) => log::warn!("The price-check hotkey Ctrl+D can't be listened to: {error}"),
        }
        Self {
            _manager: Some(manager),
        }
    }

    /// Nothing to listen to yet on this system (step 14a2b).
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    #[must_use]
    pub fn start() -> Self {
        log::info!("No price-check hotkey on this system yet");
        Self {}
    }
}
