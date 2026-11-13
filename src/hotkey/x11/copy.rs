//! The copy sent to the game under X11: Ctrl+C pressed through the XTest extension, as the
//! player would press it, in the window that has the focus.

use std::time::{Duration, Instant};

use global_hotkey::HotKeyState;
use x11rb::connection::{Connection, RequestConnection as _};
use x11rb::protocol::xproto::{self, AtomEnum, ConnectionExt as _, KeyButMask, Keycode, Window};
use x11rb::protocol::xtest::{self, ConnectionExt as _};

use crate::hotkey::Shortcut;
use crate::hotkey::sent_keys::{Held, Key, copy_keys, modifiers_for_log, send_all};
use crate::overlay::{Point, ScreenRect, Target};
use crate::x11::{self as shared, Result, Session};

use super::window_class;

/// How long the copy of the hotkey waits for the desktop to make the game the active window again,
/// in place of the helper of the window over the game.
const ACTIVATION_WAIT: Duration = Duration::from_millis(500);

/// The keysyms of the keys pressed or released, from `X11/keysymdef.h`.
const XK_C: u32 = 0x0063;
const XK_SHIFT_L: u32 = 0xffe1;
const XK_SHIFT_R: u32 = 0xffe2;
const XK_CONTROL_L: u32 = 0xffe3;
const XK_META_L: u32 = 0xffe7;
const XK_META_R: u32 = 0xffe8;
const XK_ALT_L: u32 = 0xffe9;
const XK_ALT_R: u32 = 0xffea;
/// The modifiers of a hotkey other than Ctrl, left and right, by their keysyms: Shift, Alt, and
/// Meta, which some layouts give to the Alt key.
const MODIFIERS: [(u32, Modifier); 6] = [
    (XK_SHIFT_L, Modifier::Shift),
    (XK_SHIFT_R, Modifier::Shift),
    (XK_ALT_L, Modifier::Alt),
    (XK_ALT_R, Modifier::Alt),
    (XK_META_L, Modifier::Alt),
    (XK_META_R, Modifier::Alt),
];

/// A modifier of a hotkey other than Ctrl.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Modifier {
    /// Released around the copy while the player holds it: Ctrl+Shift+C isn't the copy.
    Shift,
    /// Left held: Ctrl+Alt+C is the advanced copy of the game, which gives the same text.
    Alt,
}

impl Modifier {
    fn name(self) -> &'static str {
        match self {
            Self::Shift => "Shift",
            Self::Alt => "Alt",
        }
    }

    /// Whether `shortcut` holds it.
    fn in_shortcut(self, shortcut: Shortcut) -> bool {
        match self {
            Self::Shift => shortcut.shift(),
            Self::Alt => shortcut.alt(),
        }
    }
}
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
    ///
    /// The modifiers of `shortcut` the player released before its key are released once more
    /// for the game, which didn't see them ([`Held::missed`]).
    pub fn send_copy<P>(
        &mut self,
        shortcut: Shortcut,
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
        match send_copy(&keyboard, shortcut, prepare, tell) {
            Ok(sent) => {
                log::info!(
                    "Copy sent by the price-check hotkey to the window of class {} (Ctrl {}{})",
                    sent.window_class,
                    if sent.control_held {
                        "still held"
                    } else {
                        "pressed by the application"
                    },
                    modifiers_for_log(&sent.released, &sent.missed),
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
    /// The names of the modifiers released around the copy.
    released: Vec<&'static str>,
    /// The names of the modifiers of the hotkey released once more for the game.
    missed: Vec<&'static str>,
}

/// The keyboard of the X server, which the copy is sent to.
struct Keyboard {
    session: Session,
}

impl Keyboard {
    /// Connects to the X server, which must have the XTest extension.
    fn connect() -> Result<Self> {
        let session = Session::connect()?;
        if session
            .connection()
            .extension_information(xtest::X11_EXTENSION_NAME)?
            .is_none()
        {
            return Err("the X server has no XTest extension".into());
        }
        Ok(Self { session })
    }

    /// The keyboard mapping, read at each copy: the player may change the layout of the
    /// keyboard meanwhile.
    fn read_layout(&self) -> Result<Layout> {
        let connection = self.session.connection();
        let setup = connection.setup();
        let mapping = connection
            .get_keyboard_mapping(setup.min_keycode, setup.max_keycode - setup.min_keycode + 1)?
            .reply()?;
        Ok(Layout {
            min_keycode: setup.min_keycode,
            keysyms_per_keycode: mapping.keysyms_per_keycode,
            keysyms: mapping.keysyms,
        })
    }

    /// The game, when `window` is the helper of the window over the game, made the active window
    /// again; `window` otherwise. Whether the desktop refused to make the game active.
    fn game_in_front(&self, window: Window) -> (Window, bool) {
        match self.session.game_behind(window) {
            Ok(Some(game)) => {
                let refused = self.bring_back(game).map_err(|error| {
                    log::warn!("The game can't be made the active window again: {error}");
                });
                (game, refused.is_err())
            }
            Ok(None) => (window, false),
            Err(error) => {
                log::info!("The active window can't be read: {error}");
                (window, false)
            }
        }
    }

    /// The active window, the game in place of the helper of the window over the game; whether
    /// the desktop refused to make the game active again.
    fn active_game(&self) -> (Result<Option<Window>>, bool) {
        match self.session.active_window() {
            Ok(Some(window)) => {
                let (window, refused) = self.game_in_front(window);
                (Ok(Some(window)), refused)
            }
            other => (other, false),
        }
    }

    /// Just before the keys: the desktop may have activated the helper of the window over the
    /// game meanwhile (the window becoming the active one as the hotkey was pressed), and the
    /// window over the game taken the keyboard. The game is made active again and given the
    /// keyboard back, so that the keys reach it.
    fn game_in_front_again(&self) {
        if let Ok(Some(active)) = self.session.active_window() {
            let (game, _) = self.game_in_front(active);
            if let Err(error) = self.give_back_keyboard(game) {
                log::warn!("The keyboard can't be given back to the game: {error}");
            }
        }
    }

    /// Asks the desktop to make `game` the active window, and waits until it is, up to
    /// [`ACTIVATION_WAIT`].
    fn bring_back(&self, game: Window) -> Result<()> {
        self.session.activate(game)?;
        let start = Instant::now();
        while start.elapsed() < ACTIVATION_WAIT {
            if self.session.active_window()? == Some(game) {
                log::info!("The price-check hotkey made the game the active window again");
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        Err("the desktop didn't make it active in time".into())
    }

    /// Gives the keyboard back to `game`, the active window, when the window over the game has
    /// it, or no window has it (the window over the game just hid).
    fn give_back_keyboard(&self, game: Window) -> Result<()> {
        let focused = self.session.focused()?;
        let ours = || self.session.is_ours(focused);
        if gives_keyboard_back(focused, game, self.session.root(), ours)? {
            self.session.set_focus(game)?;
            log::info!("The price-check hotkey gave the keyboard back to the game");
        }
        Ok(())
    }

    /// Where `window` is on the screen, without its frame; none when it has no size.
    fn window_rect(&self, window: Window) -> Result<Option<ScreenRect>> {
        let connection = self.session.connection();
        let geometry = connection.get_geometry(window)?.reply()?;
        // The position of the geometry is in the frame of the desktop: the one on the screen
        // comes from the root window.
        let origin = connection
            .translate_coordinates(window, self.session.root(), 0, 0)?
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
            .session
            .connection()
            .get_property(false, window, AtomEnum::WM_CLASS, AtomEnum::STRING, 0, 256)?
            .reply()?
            .value;
        Ok(window_class(&class))
    }
}

/// What the copy reads from the X server and sends to it: the server itself ([`Keyboard`]),
/// or a fake one in the tests, the order of these calls being the rule (no test may send keys
/// to the game).
trait Server {
    /// The active window, the game in place of the helper of the window over the game; whether
    /// the desktop refused to make the game active again.
    fn active_game(&self) -> (Result<Option<Window>>, bool);
    fn window_class(&self, window: Window) -> Result<String>;
    fn give_back_keyboard(&self, game: Window) -> Result<()>;
    fn window_rect(&self, window: Window) -> Result<Option<ScreenRect>>;
    /// Forgets the events received since the last copy.
    fn drop_events(&self);
    fn layout(&self) -> Result<Layout>;
    fn pointer(&self) -> Result<Pointer>;
    /// The keys held, a bit for each keycode, from keycode 0.
    fn keys_held(&self) -> Result<Vec<u8>>;
    /// See [`Keyboard::game_in_front_again`]: it may wait up to [`ACTIVATION_WAIT`].
    fn game_in_front_again(&self);
    /// Presses or releases a key, `delay_ms` after the previous one.
    fn fake_key(&self, key: Key<Keycode>, delay_ms: u32) -> Result<()>;
}

/// The keyboard mapping of the X server: `keysyms_per_keycode` keysyms for each keycode from
/// `min_keycode`.
struct Layout {
    min_keycode: Keycode,
    keysyms_per_keycode: u8,
    keysyms: Vec<u32>,
}

impl Layout {
    fn keycode(&self, keysym: u32) -> Option<Keycode> {
        keycode_of(
            keysym,
            self.min_keycode,
            self.keysyms_per_keycode,
            &self.keysyms,
        )
    }

    /// The keys of [`MODIFIERS`] held in `keymap`.
    fn held(&self, keymap: &[u8]) -> Vec<(Keycode, Modifier)> {
        held_modifiers(
            keymap,
            self.min_keycode,
            self.keysyms_per_keycode,
            &self.keysyms,
        )
    }
}

/// Where the pointer is, and whether Ctrl is held.
#[derive(Debug, Clone, Copy)]
struct Pointer {
    position: Point,
    control: bool,
}

impl Server for Keyboard {
    fn active_game(&self) -> (Result<Option<Window>>, bool) {
        Keyboard::active_game(self)
    }

    fn window_class(&self, window: Window) -> Result<String> {
        Keyboard::window_class(self, window)
    }

    fn give_back_keyboard(&self, game: Window) -> Result<()> {
        Keyboard::give_back_keyboard(self, game)
    }

    fn window_rect(&self, window: Window) -> Result<Option<ScreenRect>> {
        Keyboard::window_rect(self, window)
    }

    fn drop_events(&self) {
        self.session.drop_events();
    }

    fn layout(&self) -> Result<Layout> {
        self.read_layout()
    }

    fn pointer(&self) -> Result<Pointer> {
        let pointer = self.session.pointer()?;
        Ok(Pointer {
            position: Point {
                x: i32::from(pointer.root_x),
                y: i32::from(pointer.root_y),
            },
            control: pointer.mask.contains(KeyButMask::CONTROL),
        })
    }

    fn keys_held(&self) -> Result<Vec<u8>> {
        Ok(self
            .session
            .connection()
            .query_keymap()?
            .reply()?
            .keys
            .to_vec())
    }

    fn game_in_front_again(&self) {
        Keyboard::game_in_front_again(self);
    }

    fn fake_key(&self, key: Key<Keycode>, delay_ms: u32) -> Result<()> {
        let (event, keycode) = match key {
            Key::Down(keycode) => (xproto::KEY_PRESS_EVENT, keycode),
            Key::Up(keycode) => (xproto::KEY_RELEASE_EVENT, keycode),
        };
        self.session
            .connection()
            .xtest_fake_input(event, keycode, delay_ms, self.session.root(), 0, 0, 0)?
            .check()?;
        Ok(())
    }
}

/// Sends Ctrl+C through `server` to the active window. The order is the rule:
///
/// 1. the window the keys go to, the keyboard given back to it, and everything that can fail
///    (the keyboard mapping, a first reading of the pointer and of the keys held): an error
///    there leaves the clipboard alone;
/// 2. `prepare`, which writes the marker to the clipboard and may take a moment: from there to
///    the keys, nothing gives up, since a marker without a copy would replace the item of the
///    player for nothing;
/// 3. the pointer read again, where the item is when the keys leave, and `tell`, which opens
///    the window over the game there, before the first key;
/// 4. the game made the active window again, which may wait up to [`ACTIVATION_WAIT`];
/// 5. Ctrl and Shift read last, as the player holds them right before the keys (as under
///    Windows): read before that wait, a Ctrl released during it sent C alone to the game, and
///    a Shift released during it was pressed again and left held (desktop review of
///    2026-11-10). A reading that fails then keeps the one of step 1;
/// 6. the keys.
fn send_copy<P>(
    server: &impl Server,
    shortcut: Shortcut,
    prepare: impl FnOnce() -> P,
    tell: impl FnOnce(Option<Target>, P),
) -> Result<Sent> {
    let window = aim_keys(server);
    server.drop_events();
    let layout = server.layout()?;
    let (Some(control), Some(c)) = (layout.keycode(XK_CONTROL_L), layout.keycode(XK_C)) else {
        return Err("the keyboard has no Control or C key".into());
    };
    let mut pointer = server.pointer()?;
    let mut keys_held = server.keys_held()?;
    let prepared = prepare();
    // Read after the preparation, which may take a moment: where the item is.
    if let Ok(read) = server.pointer() {
        pointer = read;
    }
    let target = window.game.map(|game| Target {
        pointer: pointer.position,
        game,
    });
    tell(target, prepared);
    // Not twice when the desktop already refused.
    if !window.refused {
        server.game_in_front_again();
    }
    let control_held = server
        .pointer()
        .map_or(pointer.control, |pointer| pointer.control);
    if let Ok(read) = server.keys_held() {
        keys_held = read;
    }
    let modifiers_held = layout.held(&keys_held);
    let missed = missed_modifiers(shortcut, &modifiers_held, |keysym| layout.keycode(keysym));
    // Alt left held: Ctrl+Alt+C copies the same text.
    let released: Vec<_> = modifiers_held
        .into_iter()
        .filter(|&(_, modifier)| modifier == Modifier::Shift)
        .collect();
    let keycodes = |keys: &[(Keycode, Modifier)]| keys.iter().map(|&(key, _)| key).collect();
    let names =
        |keys: &[(Keycode, Modifier)]| keys.iter().map(|&(_, modifier)| modifier.name()).collect();
    let held = Held {
        control: control_held,
        others: keycodes(&released),
        missed: keycodes(&missed),
    };
    press(server, copy_keys(control, c, &held))?;
    Ok(Sent {
        window_class: window.class,
        control_held,
        released: names(&released),
        missed: names(&missed),
    })
}

/// The window the keys of a copy go to, read before them (step 1 of [`send_copy`]).
struct KeysWindow {
    /// The class of the active window, for the log.
    class: String,
    /// Where the game is, for the window over the game: none when it isn't the active window,
    /// or can't be measured.
    game: Option<ScreenRect>,
    /// Whether the desktop refused to make the game the active window again.
    refused: bool,
}

/// Aims the keys of a copy: reads the window they go to, and gives it the keyboard back from a
/// field of the window over the game; a failure is told in the result or the log, so that it
/// doesn't stop the copy. On a single screen, the helper of the window over the game may be active in place of
/// the game: the game is made active again, so that the copy and the window go to it.
fn aim_keys(server: &impl Server) -> KeysWindow {
    let (window, refused) = server.active_game();
    let class = match &window {
        Ok(Some(window)) => server
            .window_class(*window)
            .unwrap_or_else(|error| format!("unknown ({error})")),
        Ok(None) => "none (an application outside X11?)".to_owned(),
        Err(error) => format!("unknown ({error})"),
    };
    let Ok(Some(window)) = window else {
        return KeysWindow {
            class,
            game: None,
            refused,
        };
    };
    // While a field of the window over the game is edited, the keys of the copy would go to
    // it: the game gets the keyboard back first.
    if let Err(error) = server.give_back_keyboard(window) {
        log::warn!("The keyboard can't be given back to the game: {error}");
    }
    let game = server.window_rect(window).unwrap_or_else(|error| {
        log::warn!("The window of the game can't be measured: {error}");
        None
    });
    KeysWindow {
        class,
        game,
        refused,
    }
}

/// Presses and releases `keys` in their order through `server`, each after the first
/// [`KEY_DELAY_MS`] after the previous one.
fn press(server: &impl Server, keys: Vec<Key<Keycode>>) -> Result<()> {
    let mut first = true;
    send_all(keys, |key| {
        let delay = if std::mem::take(&mut first) {
            0
        } else {
            KEY_DELAY_MS
        };
        server.fake_key(key, delay)
    })
}

/// Whether the hotkey gives the keyboard, at `focused`, back to `game`, the active window, before
/// its copy: from a window of this application that the desktop didn't activate (`ours`, read
/// only then: the window over the game, while one of its fields is edited), or from no window
/// (the window over the game just hid); never from the game, nor from another window.
fn gives_keyboard_back(
    focused: Window,
    game: Window,
    root: Window,
    ours: impl FnOnce() -> Result<bool>,
) -> Result<bool> {
    if focused == game {
        return Ok(false);
    }
    if shared::is_nowhere(focused, root) {
        return Ok(true);
    }
    ours()
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

/// The keys of [`MODIFIERS`] held in `keymap` (a bit for each keycode, from keycode 0),
/// with their modifier, read from the first column of the keyboard mapping (`keysyms_per_keycode`
/// keysyms for each keycode from `min_keycode`).
fn held_modifiers(
    keymap: &[u8],
    min_keycode: Keycode,
    keysyms_per_keycode: u8,
    keysyms: &[u32],
) -> Vec<(Keycode, Modifier)> {
    keysyms
        .chunks(usize::from(keysyms_per_keycode).max(1))
        .zip(min_keycode..=Keycode::MAX)
        .filter(|&(_, keycode)| {
            keymap
                .get(usize::from(keycode / 8))
                .is_some_and(|byte| byte & (1 << (keycode % 8)) != 0)
        })
        .filter_map(|(keysyms, keycode)| {
            let first = keysyms.first()?;
            MODIFIERS
                .iter()
                .find(|(keysym, _)| keysym == first)
                .map(|&(_, modifier)| (keycode, modifier))
        })
        .collect()
}

/// The keys of the modifiers of `shortcut` other than Ctrl that the player no longer holds, none
/// of their keys left or right among `held`: they were released before the key of the hotkey,
/// which the game didn't see ([`Held::missed`]). Each of their keys (found by `keycode`), since
/// which one the player pressed is unknown; none of a modifier one of whose keys is held.
fn missed_modifiers(
    shortcut: Shortcut,
    held: &[(Keycode, Modifier)],
    keycode: impl Fn(u32) -> Option<Keycode>,
) -> Vec<(Keycode, Modifier)> {
    let mut missed: Vec<(Keycode, Modifier)> = Vec::new();
    for (keysym, modifier) in MODIFIERS {
        if modifier.in_shortcut(shortcut)
            && !held.iter().any(|&(_, held)| held == modifier)
            && let Some(key) = keycode(keysym)
            && !missed.iter().any(|&(missed, _)| missed == key)
        {
            missed.push((key, modifier));
        }
    }
    missed
}

#[cfg(test)]
mod tests;
