//! The price-check hotkey: Ctrl+D pressed in the game copies the item under the cursor, so
//! that the player no longer presses Ctrl+C.
//!
//! The application sends Ctrl+C to the game itself, and the item arrives through the clipboard
//! like any copy ([`crate::clipboard`]). On Linux through X11 (XWayland under Wayland), like the
//! game run by Proton, and on Windows. None on macOS, where Path of Exile 2 has no client.
//!
//! Each hotkey tells the application where it was pressed, so that the item it copied opens the
//! window over the game there ([`crate::overlay`]).

use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use crate::clipboard::CopyMarker;
use crate::overlay::Target;

/// How long the game may take to copy the item after the hotkey: past it, the hotkey copied
/// nothing (an empty cell of the inventory). The clipboard watching takes an item read within it
/// for the copy of the hotkey, and tells it with [`Press::copy`].
pub const COPY_WAIT: Duration = Duration::from_secs(1);

/// A press of the hotkey, told just before its keys, so that its item never comes first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Press {
    /// Where it was pressed, when the system could tell it.
    pub target: Option<Target>,
    /// When the clipboard was prepared for its copy, which the clipboard watching gives back
    /// with the item it copied ([`crate::clipboard::ClipboardEvent::Copied`]); none when it
    /// couldn't be prepared (then its item only shows in the main window), and always on
    /// Windows until step 14b3.
    pub copy: Option<Instant>,
}

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "linux")]
mod x11;

// The keys of each system: `CopySender` (`connect` once at the start, `send_copy` at each
// hotkey) and `SENT_AT`, the press or the release of D at which the copy is sent.
#[cfg(target_os = "windows")]
use self::windows as keys;
#[cfg(target_os = "linux")]
use self::x11 as keys;

/// The hotkey listened to for as long as this value lives.
pub struct PriceCheckHotkey {
    /// Listens until it is dropped.
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    _manager: Option<global_hotkey::GlobalHotKeyManager>,
    /// Each press of the hotkey.
    targets: Option<Receiver<Press>>,
}

impl PriceCheckHotkey {
    /// Starts listening to Ctrl+D. A failure (the key taken by another application, such as
    /// Exiled Exchange 2) only writes to the log: the player can still copy with Ctrl+C.
    /// `marker` prepares the clipboard before each copy (on Linux: on Windows, reading the
    /// clipboard first would delay the copy past a short press of Ctrl); `wake` is called after
    /// each hotkey.
    ///
    /// Called once: the handler of the hotkey events can only be set once. Called on the thread
    /// of the window: on Windows, the hotkey is heard by a hidden window of that thread, through
    /// its event loop.
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    #[must_use]
    pub fn start(marker: CopyMarker, wake: impl Fn() + Send + Sync + 'static) -> Self {
        use global_hotkey::hotkey::{Code, HotKey, Modifiers};
        use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager};
        use std::sync::{Mutex, PoisonError};

        let off = || Self {
            _manager: None,
            targets: None,
        };
        let sender = match keys::CopySender::connect() {
            Ok(sender) => Mutex::new((sender, marker)),
            Err(error) => {
                log::warn!("The price-check hotkey can't listen: {error}");
                return off();
            }
        };
        let manager = match GlobalHotKeyManager::new() {
            Ok(manager) => manager,
            Err(error) => {
                log::warn!("The price-check hotkey can't listen: {error}");
                return off();
            }
        };
        let (targets, receiver) = std::sync::mpsc::channel();
        let hotkey = HotKey::new(Some(Modifiers::CONTROL), Code::KeyD);
        let id = hotkey.id();
        GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
            // At the press or at the release of D, as each system lets the keys sent reach the
            // game.
            if event.id == id && event.state == keys::SENT_AT {
                let mut sender = sender.lock().unwrap_or_else(PoisonError::into_inner);
                let (keys, marker) = &mut *sender;
                keys.send_copy(|target| {
                    // Marked just before the keys, once they are sure to leave: a copy that
                    // doesn't leave would leave the marker in place of the item of the player.
                    #[cfg(target_os = "linux")]
                    let copy = marker.before_copy();
                    #[cfg(not(target_os = "linux"))]
                    let copy = {
                        let _ = &marker;
                        None
                    };
                    // The application stopped listening: nothing left to tell.
                    if targets.send(Press { target, copy }).is_ok() {
                        wake();
                    }
                });
            }
        }));
        match manager.register(hotkey) {
            Ok(()) => log::info!("Price-check hotkey Ctrl+D listened to"),
            Err(error) => log::warn!("The price-check hotkey Ctrl+D can't be listened to: {error}"),
        }
        Self {
            _manager: Some(manager),
            targets: Some(receiver),
        }
    }

    /// Nothing to listen to on this system.
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    #[must_use]
    pub fn start(_marker: CopyMarker, _wake: impl Fn() + Send + Sync + 'static) -> Self {
        log::info!("No price-check hotkey on this system");
        Self { targets: None }
    }
}

impl PriceCheckHotkey {
    /// Gives each press of the hotkey since the last call to `handle`, without waiting.
    pub fn receive(&mut self, mut handle: impl FnMut(Press)) {
        if let Some(targets) = &self.targets {
            while let Ok(target) = targets.try_recv() {
                handle(target);
            }
        }
    }
}
