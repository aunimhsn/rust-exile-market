//! The copy sent to the game under X11: Ctrl+C pressed through the XTest extension, as the
//! player would press it, in the window that has the focus.

use std::error::Error;

use global_hotkey::HotKeyState;
use x11rb::connection::{Connection, RequestConnection as _};
use x11rb::protocol::xproto::{
    self, Atom, AtomEnum, ConnectionExt as _, InputFocus, KeyButMask, Keycode, Window,
};
use x11rb::protocol::xtest::{self, ConnectionExt as _};
use x11rb::rust_connection::RustConnection;

use crate::overlay::{Point, ScreenRect, Target, is_ours};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// The keysyms of the keys pressed, from `X11/keysymdef.h`.
const XK_C: u32 = 0x0063;
const XK_CONTROL_L: u32 = 0xffe3;
/// The delay the X server waits before each key after the first, in milliseconds, so that a
/// game that reads the keys at each frame sees each of them (the delay of the `enigo` crate).
const KEY_DELAY_MS: u32 = 12;

/// At the release of D: while it is held, the X server gives every key to the application that
/// listens to the hotkey, so that a copy sent then wouldn't reach the game.
pub const SENT_AT: HotKeyState = HotKeyState::Released;

/// The connection to the X server, opened at the start, and again at the next copy after a
/// failure.
pub struct CopySender {
    keyboard: Option<Keyboard>,
}

impl CopySender {
    /// Opens the connection at once, so that an X server out of reach is told at the start:
    /// `global-hotkey` only says so on its own thread, without the log.
    ///
    /// # Errors
    ///
    /// When the X server can't be reached or has no XTest extension.
    pub fn connect() -> Result<Self> {
        let keyboard = Keyboard::connect().map_err(|error| format!("no X11 keyboard ({error})"))?;
        Ok(Self {
            keyboard: Some(keyboard),
        })
    }

    /// Sends Ctrl+C to the window that has the focus, and writes to the log where it went.
    /// Once the window and the keyboard are read, `prepare` runs (it may take a moment), then
    /// the pointer is read, as the game sees it when the keys leave, and `tell` gets where the
    /// window over the game opens, when the pointer and the window could be read, with what
    /// `prepare` gave, just before the keys.
    pub fn send_copy<P>(
        &mut self,
        prepare: impl FnOnce() -> P,
        tell: impl FnOnce(Option<Target>, P),
    ) {
        let keyboard = match self.keyboard.take() {
            Some(keyboard) => keyboard,
            None => match Keyboard::connect() {
                Ok(keyboard) => keyboard,
                Err(error) => {
                    log::warn!("The copy can't be sent: no X11 keyboard ({error})");
                    return;
                }
            },
        };
        match keyboard.send_copy(prepare, tell) {
            Ok(sent) => {
                log::info!(
                    "Copy sent by the price-check hotkey to the window of class {} (Ctrl {})",
                    sent.window_class,
                    if sent.control_held {
                        "still held"
                    } else {
                        "pressed by the application"
                    }
                );
                self.keyboard = Some(keyboard);
            }
            // The connection is opened again at the next copy.
            Err(error) => log::warn!("The copy can't be sent: {error}"),
        }
    }
}

/// What a copy sent did, for the log.
struct Sent {
    /// The class of the active window, which tells the game from another application.
    window_class: String,
    control_held: bool,
}

struct Keyboard {
    connection: RustConnection,
    root: Window,
    /// `_NET_ACTIVE_WINDOW`, the window the desktop gives the focus to.
    active_window: Atom,
    /// `_NET_WM_PID`, the process of a window.
    pid: Atom,
}

impl Keyboard {
    fn connect() -> Result<Self> {
        let (connection, screen) = x11rb::connect(None)?;
        if connection
            .extension_information(xtest::X11_EXTENSION_NAME)?
            .is_none()
        {
            return Err("the X server has no XTest extension".into());
        }
        let root = connection.setup().roots[screen].root;
        let active_window = connection
            .intern_atom(false, b"_NET_ACTIVE_WINDOW")?
            .reply()?
            .atom;
        let pid = connection.intern_atom(false, b"_NET_WM_PID")?.reply()?.atom;
        Ok(Self {
            connection,
            root,
            active_window,
            pid,
        })
    }

    fn send_copy<P>(
        &self,
        prepare: impl FnOnce() -> P,
        tell: impl FnOnce(Option<Target>, P),
    ) -> Result<Sent> {
        // The window the keys go to, read before them; for the log and the window over the
        // game, so that a failure doesn't stop the copy.
        let window = self.active_window();
        let window_class = match &window {
            Ok(Some(window)) => self
                .window_class(*window)
                .unwrap_or_else(|error| format!("unknown ({error})")),
            Ok(None) => "none (an application outside X11?)".to_owned(),
            Err(error) => format!("unknown ({error})"),
        };
        // While a field of the window over the game is edited, the keys of the copy would go to
        // it: the game gets the keyboard back first.
        if let Ok(Some(game)) = window
            && let Err(error) = self.give_back_keyboard(game)
        {
            log::warn!("The keyboard can't be given back to the game: {error}");
        }
        let game = match window {
            Ok(Some(window)) => self.window_rect(window).unwrap_or_else(|error| {
                log::warn!("The window of the game can't be measured: {error}");
                None
            }),
            _ => None,
        };
        // The events the X server sends to every client, such as a change of the keyboard
        // mapping, are never needed: dropped so that they don't pile up.
        while let Ok(Some(_)) = self.connection.poll_for_event() {}
        // Read at each copy: the player may change the layout of the keyboard meanwhile.
        let setup = self.connection.setup();
        let mapping = self
            .connection
            .get_keyboard_mapping(setup.min_keycode, setup.max_keycode - setup.min_keycode + 1)?
            .reply()?;
        let keycode = |keysym| {
            keycode_of(
                keysym,
                setup.min_keycode,
                mapping.keysyms_per_keycode,
                &mapping.keysyms,
            )
        };
        let (Some(control), Some(c)) = (keycode(XK_CONTROL_L), keycode(XK_C)) else {
            return Err("the keyboard has no Control or C key".into());
        };
        let prepared = prepare();
        // Read after the preparation, which may take a moment: where the item is, and Ctrl as
        // the player holds it, when the keys leave.
        let pointer = self.connection.query_pointer(self.root)?.reply()?;
        let control_held = pointer.mask.contains(KeyButMask::CONTROL);
        let target = game.map(|game| Target {
            pointer: Point {
                x: i32::from(pointer.root_x),
                y: i32::from(pointer.root_y),
            },
            game,
        });
        tell(target, prepared);
        for (index, key) in copy_keys(control, c, control_held).into_iter().enumerate() {
            let (event, keycode) = match key {
                Key::Down(keycode) => (xproto::KEY_PRESS_EVENT, keycode),
                Key::Up(keycode) => (xproto::KEY_RELEASE_EVENT, keycode),
            };
            let delay = if index == 0 { 0 } else { KEY_DELAY_MS };
            self.connection
                .xtest_fake_input(event, keycode, delay, self.root, 0, 0, 0)?
                .check()?;
        }
        Ok(Sent {
            window_class,
            control_held,
        })
    }

    /// Gives the keyboard back to `game`, the active window, when the window over the game has
    /// it: the window of this application with the keyboard, which the desktop didn't activate.
    fn give_back_keyboard(&self, game: Window) -> Result<()> {
        let focused = self.connection.get_input_focus()?.reply()?.focus;
        if focused != game
            && focused != self.root
            && focused > u32::from(InputFocus::POINTER_ROOT)
            && is_ours(&self.connection, self.pid, focused)?
        {
            self.connection
                .set_input_focus(InputFocus::PARENT, game, x11rb::CURRENT_TIME)?
                .check()?;
            log::info!("The price-check hotkey gave the keyboard back to the game");
        }
        Ok(())
    }

    /// The active window, none when the desktop gives the focus to a window outside X11.
    fn active_window(&self) -> Result<Option<Window>> {
        let window = self
            .connection
            .get_property(false, self.root, self.active_window, AtomEnum::WINDOW, 0, 1)?
            .reply()?
            .value32()
            .and_then(|mut windows| windows.next())
            .unwrap_or(x11rb::NONE);
        Ok((window != x11rb::NONE).then_some(window))
    }

    /// Where `window` is on the screen, without its frame; none when it has no size.
    fn window_rect(&self, window: Window) -> Result<Option<ScreenRect>> {
        let geometry = self.connection.get_geometry(window)?.reply()?;
        // The position of the geometry is in the frame of the desktop: the one on the screen
        // comes from the root window.
        let origin = self
            .connection
            .translate_coordinates(window, self.root, 0, 0)?
            .reply()?;
        let rect = ScreenRect {
            x: i32::from(origin.dst_x),
            y: i32::from(origin.dst_y),
            width: i32::from(geometry.width),
            height: i32::from(geometry.height),
        };
        Ok((rect.width > 0 && rect.height > 0).then_some(rect))
    }

    /// The class of `window` (`WM_CLASS`).
    fn window_class(&self, window: Window) -> Result<String> {
        let class = self
            .connection
            .get_property(false, window, AtomEnum::WM_CLASS, AtomEnum::STRING, 0, 256)?
            .reply()?
            .value;
        Ok(window_class(&class))
    }
}

/// A key pressed or released.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Key {
    Down(Keycode),
    Up(Keycode),
}

/// The keys that copy the item: C alone while the player still holds Ctrl, otherwise Ctrl
/// pressed around it, then released, so that the keyboard is left as the player holds it.
/// Releasing a Ctrl the player holds would make the X server take it for released, and the
/// next D of the player would reach the game alone.
fn copy_keys(control: Keycode, c: Keycode, control_held: bool) -> Vec<Key> {
    if control_held {
        vec![Key::Down(c), Key::Up(c)]
    } else {
        vec![
            Key::Down(control),
            Key::Down(c),
            Key::Up(c),
            Key::Up(control),
        ]
    }
}

/// The keycode that gives `keysym` in the first column of the keyboard mapping of the X server
/// (the first group, without Shift), otherwise in another column (another level or group):
/// `keysyms_per_keycode` keysyms for each keycode from `min_keycode`.
fn keycode_of(
    keysym: u32,
    min_keycode: Keycode,
    keysyms_per_keycode: u8,
    keysyms: &[u32],
) -> Option<Keycode> {
    let mut keycodes = keysyms.chunks(usize::from(keysyms_per_keycode).max(1));
    let offset = keycodes
        .clone()
        .position(|keysyms| keysyms.first() == Some(&keysym))
        .or_else(|| keycodes.position(|keysyms| keysyms.contains(&keysym)))?;
    min_keycode.checked_add(u8::try_from(offset).ok()?)
}

/// The class of a window from its `WM_CLASS`, an instance name then a class name, each ended
/// by a null byte.
fn window_class(property: &[u8]) -> String {
    let names: Vec<_> = property
        .split(|&byte| byte == 0)
        .filter(|name| !name.is_empty())
        .map(String::from_utf8_lossy)
        .collect();
    match names.as_slice() {
        [] => "unknown".to_owned(),
        [instance, class, ..] if instance != class => format!("{class} ({instance})"),
        [name, ..] => name.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONTROL: Keycode = 37;
    const C: Keycode = 54;

    #[test]
    fn taps_c_alone_while_the_player_holds_ctrl() {
        assert_eq!(copy_keys(CONTROL, C, true), [Key::Down(C), Key::Up(C)]);
    }

    #[test]
    fn presses_ctrl_around_c_once_the_player_released_it() {
        assert_eq!(
            copy_keys(CONTROL, C, false),
            [
                Key::Down(CONTROL),
                Key::Down(C),
                Key::Up(C),
                Key::Up(CONTROL)
            ]
        );
    }

    /// Two keysyms per keycode from keycode 8: `a A`, `c C`, `Control_L`, `ccedilla c`.
    const MAPPING: [u32; 8] = [0x61, 0x41, 0x63, 0x43, XK_CONTROL_L, 0, 0xe7, 0x63];

    #[test]
    fn finds_the_keycode_of_a_keysym_without_modifier_first() {
        assert_eq!(keycode_of(XK_C, 8, 2, &MAPPING), Some(9));
        assert_eq!(keycode_of(XK_CONTROL_L, 8, 2, &MAPPING), Some(10));
    }

    #[test]
    fn finds_a_keysym_given_with_a_modifier_only() {
        assert_eq!(keycode_of(0x41, 8, 2, &MAPPING), Some(8));
    }

    #[test]
    fn a_keysym_missing_from_the_mapping_has_no_keycode() {
        assert_eq!(keycode_of(0x64, 8, 2, &MAPPING), None);
        assert_eq!(keycode_of(XK_C, 8, 2, &[]), None);
    }

    #[test]
    fn a_keycode_past_the_last_one_is_none() {
        let mut mapping = vec![0; 300];
        mapping[299] = XK_C;
        assert_eq!(keycode_of(XK_C, 8, 1, &mapping), None);
    }

    #[test]
    fn names_the_class_of_a_window_and_its_instance_when_it_differs() {
        assert_eq!(
            window_class(b"steam_app_2694490\0steam_app_2694490\0"),
            "steam_app_2694490"
        );
        assert_eq!(
            window_class(b"pathofexile2.exe\0PathOfExile2\0"),
            "PathOfExile2 (pathofexile2.exe)"
        );
        assert_eq!(window_class(b""), "unknown");
    }

    #[test]
    fn reads_a_class_of_one_name_or_without_its_last_null_byte() {
        assert_eq!(window_class(b"steam\0"), "steam");
        assert_eq!(
            window_class(b"pathofexile2.exe\0PathOfExile2"),
            "PathOfExile2 (pathofexile2.exe)"
        );
    }
}
