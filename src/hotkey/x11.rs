//! The copy sent to the game under X11: Ctrl+C pressed through the XTest extension, as the
//! player would press it, in the window that has the focus; and the active window followed, so
//! that the hotkey is grabbed only while it is the game.

use std::collections::HashSet;
use std::time::{Duration, Instant};

use global_hotkey::HotKeyState;
use x11rb::connection::{Connection, RequestConnection as _};
use x11rb::protocol::Event;
use x11rb::protocol::xproto::{
    self, Atom, AtomEnum, ChangeWindowAttributesAux, ConnectionExt as _, EventMask, InputFocus,
    KeyButMask, Keycode, PropertyNotifyEvent, Window,
};
use x11rb::protocol::xtest::{self, ConnectionExt as _};
use x11rb::rust_connection::RustConnection;

use super::grab::{Active, GAME_TITLE};
use super::{Held, Key, Shortcut, copy_keys, modifiers_for_log, send_all};
use crate::logging;
use crate::overlay::{Point, ScreenRect, Target};
use crate::x11::{self as shared, Atoms, Result};

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

/// The class of the game run by Steam through Proton, its instance and class names alike in
/// `WM_CLASS`.
const GAME_CLASS: &[u8] = super::GAME_CLASS.as_bytes();
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
        match keyboard.send_copy(shortcut, prepare, tell) {
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

/// Follows the active window of the desktop on a thread of its own, and tells `game` what it is:
/// at the start, then at each change of the active window or of its class or title (a window may
/// be titled after it is shown), until `game` returns `false`. When the desktop doesn't tell its
/// active window (checked at the start), or the X server is lost, `game` is told once that it
/// can't be followed ([`Active::NotFollowed`]), and the thread ends.
///
/// # Errors
///
/// When the X server can't be reached: told at once, before the thread starts.
pub fn follow_game(mut game: impl FnMut(Active) -> bool + Send + 'static) -> Result<()> {
    let watch = ActiveWindow::connect()?;
    std::thread::Builder::new()
        .name("game-window".to_owned())
        .spawn(move || {
            if let Err(error) = watch.follow(&mut game) {
                log::warn!("The game window can't be followed: {error}");
                game(Active::NotFollowed);
            }
        })?;
    Ok(())
}

/// The atoms the active window is followed with.
#[derive(Debug, Clone, Copy)]
struct WatchAtoms {
    /// `_NET_ACTIVE_WINDOW`, on the root window.
    active_window: Atom,
    /// `_NET_SUPPORTED`, the hints the desktop keeps, on the root window.
    supported: Atom,
    /// `_NET_WM_NAME`, the title of a window in UTF-8.
    net_wm_name: Atom,
    /// `UTF8_STRING`, the type of `_NET_WM_NAME`.
    utf8_string: Atom,
}

/// The connection that follows the active window.
struct ActiveWindow {
    connection: RustConnection,
    root: Window,
    shared: Atoms,
    atoms: WatchAtoms,
}

impl ActiveWindow {
    fn connect() -> Result<Self> {
        let (connection, screen) = x11rb::connect(None)?;
        let root = connection.setup().roots[screen].root;
        let shared = Atoms::intern(&connection)?;
        let intern = |name: &[u8]| -> Result<Atom> {
            Ok(connection.intern_atom(false, name)?.reply()?.atom)
        };
        let atoms = WatchAtoms {
            active_window: shared.active_window,
            supported: intern(b"_NET_SUPPORTED")?,
            net_wm_name: intern(b"_NET_WM_NAME")?,
            utf8_string: intern(b"UTF8_STRING")?,
        };
        Ok(Self {
            connection,
            root,
            shared,
            atoms,
        })
    }

    /// Follows the active window until `game` returns `false`.
    ///
    /// # Errors
    ///
    /// When the desktop doesn't tell its active window, or the X server is lost.
    fn follow(&self, game: &mut impl FnMut(Active) -> bool) -> Result<()> {
        if !self.tells_active_window()? {
            return Err("the desktop doesn't tell its active window (_NET_SUPPORTED)".into());
        }
        // Listened to before the active window is read, so that no change is missed.
        self.listen(self.root, EventMask::PROPERTY_CHANGE)?
            .check()?;
        let mut followed = None;
        // The classes of the windows already written to the log.
        let mut classes_told = HashSet::new();
        loop {
            let active = known_active(
                shared::active_window(&self.connection, self.root, self.shared)?,
                self.root,
            );
            if active != followed {
                // The windows may be gone: their errors are dropped.
                if let Some(window) = followed {
                    self.listen(window, EventMask::NO_EVENT)?.ignore_error();
                }
                if let Some(window) = active {
                    self.listen(window, EventMask::PROPERTY_CHANGE)?
                        .ignore_error();
                }
                followed = active;
            }
            // None: the desktop tells no active window, which may be the window over the game
            // keeping the keyboard (`crate::overlay::keyboard`), or a window outside X11.
            let kind = active.map_or(Active::Unknown, |window| {
                match self.is_game(window, &mut classes_told) {
                    Ok(true) => Active::Game,
                    Ok(false) => Active::Other,
                    Err(error) => {
                        log::info!("The active window can't be read: {error}");
                        Active::Other
                    }
                }
            });
            if !game(kind) {
                return Ok(());
            }
            self.wait_for_change(followed)?;
        }
    }

    /// Whether the desktop keeps `_NET_ACTIVE_WINDOW` up to date, read once at the start: a
    /// desktop started after the application leaves the hotkey grabbed for every window.
    fn tells_active_window(&self) -> Result<bool> {
        // Read whole: the server sends the property up to its length.
        let supported = self
            .connection
            .get_property(
                false,
                self.root,
                self.atoms.supported,
                AtomEnum::ATOM,
                0,
                u32::MAX,
            )?
            .reply()?;
        Ok(supports(
            supported.value32().into_iter().flatten(),
            self.atoms.active_window,
        ))
    }

    /// Waits for a change that may change whether the active window is the game.
    fn wait_for_change(&self, followed: Option<Window>) -> Result<()> {
        loop {
            if let Event::PropertyNotify(event) = self.connection.wait_for_event()?
                && matters(&event, self.root, followed, self.atoms)
            {
                return Ok(());
            }
        }
    }

    /// Sets the events of `window` this connection gets.
    fn listen(
        &self,
        window: Window,
        events: EventMask,
    ) -> Result<x11rb::cookie::VoidCookie<'_, RustConnection>> {
        Ok(self.connection.change_window_attributes(
            window,
            &ChangeWindowAttributesAux::new().event_mask(events),
        )?)
    }

    /// Whether `window` is the game, by its class or its title; the class of a window told by its
    /// title, the game or not, is written to the log the first time it is active, never its
    /// title (the page of a browser), so that the class of a client of the game not known yet can
    /// be found there.
    fn is_game(&self, window: Window, classes_told: &mut HashSet<String>) -> Result<bool> {
        // The helper of the window over the game, active in its place on a single screen
        // (`crate::overlay`): the hotkey stays grabbed, and its copy gives the game back first.
        if shared::game_behind(&self.connection, self.shared, window)?.is_some() {
            return Ok(true);
        }
        let property = |name: Atom, kind: Atom| -> Result<Vec<u8>> {
            Ok(self
                .connection
                .get_property(false, window, name, kind, 0, 256)?
                .reply()?
                .value)
        };
        let class = property(AtomEnum::WM_CLASS.into(), AtomEnum::STRING.into())?;
        if is_game_class(&class) {
            return Ok(true);
        }
        // `WM_NAME` of any type: Wine or Xlib may write it in UTF-8 or as compound text, the
        // title of the game being in ASCII alike in each.
        let title = title(
            property(self.atoms.net_wm_name, self.atoms.utf8_string)?,
            || property(AtomEnum::WM_NAME.into(), AtomEnum::ANY.into()),
        )?;
        let is_game = is_game_title(&title);
        let class = window_class(&class);
        if !classes_told.contains(&class) {
            // The class of the game told by its title (a client run by Wine, whose class is
            // unknown): the bug report keeps it in the log as the game's.
            if is_game {
                log::info!("The active window, of class {class}, is the game");
            } else {
                log::info!("The active window, of class {class}, isn't the game");
            }
            classes_told.insert(class);
        }
        Ok(is_game)
    }
}

/// Whether the hints of `_NET_SUPPORTED`, `supported`, hold `hint`.
fn supports(mut supported: impl Iterator<Item = Atom>, hint: Atom) -> bool {
    supported.any(|atom| atom == hint)
}

/// The active window read, none for the root window, which a desktop may write there: listened to
/// for its active window, it must never be followed as one, whose events would be turned off once
/// it is no longer active.
fn known_active(active: Option<Window>, root: Window) -> Option<Window> {
    active.filter(|&window| window != root)
}

/// Whether `event` may change whether the active window is the game: the active window of `root`,
/// or the class or the title of the window `followed`.
fn matters(
    event: &PropertyNotifyEvent,
    root: Window,
    followed: Option<Window>,
    atoms: WatchAtoms,
) -> bool {
    if event.window == root {
        return event.atom == atoms.active_window;
    }
    Some(event.window) == followed
        && [
            AtomEnum::WM_CLASS.into(),
            AtomEnum::WM_NAME.into(),
            atoms.net_wm_name,
        ]
        .contains(&event.atom)
}

/// The title of a window: its `_NET_WM_NAME`, otherwise its old `WM_NAME`, read only then.
fn title(net_wm_name: Vec<u8>, wm_name: impl FnOnce() -> Result<Vec<u8>>) -> Result<Vec<u8>> {
    if net_wm_name.is_empty() {
        wm_name()
    } else {
        Ok(net_wm_name)
    }
}

/// Whether a window title is the title of the game, exactly.
fn is_game_title(title: &[u8]) -> bool {
    title == GAME_TITLE.as_bytes()
}

/// Whether a `WM_CLASS`, an instance name then a class name each ended by a null byte, is the
/// class of the game.
fn is_game_class(property: &[u8]) -> bool {
    property
        .split(|&byte| byte == 0)
        .any(|name| name == GAME_CLASS)
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

struct Keyboard {
    connection: RustConnection,
    root: Window,
    atoms: Atoms,
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
        let atoms = Atoms::intern(&connection)?;
        Ok(Self {
            connection,
            root,
            atoms,
        })
    }

    fn send_copy<P>(
        &self,
        shortcut: Shortcut,
        prepare: impl FnOnce() -> P,
        tell: impl FnOnce(Option<Target>, P),
    ) -> Result<Sent> {
        // The window the keys go to, read before them; for the log and the window over the
        // game, so that a failure doesn't stop the copy.
        // On a single screen, the helper of the window over the game may be active in place of
        // the game: the game is made active again, so that the copy and the window go to it.
        let (window, refused) = self.active_game();
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
        shared::drop_events(&self.connection);
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
        let keymap = self.connection.query_keymap()?.reply()?.keys;
        let modifiers_held = held_modifiers(
            &keymap,
            setup.min_keycode,
            mapping.keysyms_per_keycode,
            &mapping.keysyms,
        );
        let target = game.map(|game| Target {
            pointer: Point {
                x: i32::from(pointer.root_x),
                y: i32::from(pointer.root_y),
            },
            game,
        });
        tell(target, prepared);
        let mut first = true;
        let missed = missed_modifiers(shortcut, &modifiers_held, keycode);
        // Alt left held: Ctrl+Alt+C copies the same text.
        let released: Vec<_> = modifiers_held
            .into_iter()
            .filter(|&(_, modifier)| modifier == Modifier::Shift)
            .collect();
        let keycodes = |keys: &[(Keycode, Modifier)]| keys.iter().map(|&(key, _)| key).collect();
        let names = |keys: &[(Keycode, Modifier)]| {
            keys.iter().map(|&(_, modifier)| modifier.name()).collect()
        };
        let held = Held {
            control: control_held,
            others: keycodes(&released),
            missed: keycodes(&missed),
        };
        // Not twice when the desktop already refused.
        if !refused {
            self.game_in_front_again();
        }
        send_all(copy_keys(control, c, &held), |key| -> Result<()> {
            let (event, keycode) = match key {
                Key::Down(keycode) => (xproto::KEY_PRESS_EVENT, keycode),
                Key::Up(keycode) => (xproto::KEY_RELEASE_EVENT, keycode),
            };
            let delay = if std::mem::take(&mut first) {
                0
            } else {
                KEY_DELAY_MS
            };
            self.connection
                .xtest_fake_input(event, keycode, delay, self.root, 0, 0, 0)?
                .check()?;
            Ok(())
        })?;
        Ok(Sent {
            window_class,
            control_held,
            released: names(&released),
            missed: names(&missed),
        })
    }

    /// The game, when `window` is the helper of the window over the game, made the active window
    /// again; `window` otherwise. Whether the desktop refused to make the game active.
    fn game_in_front(&self, window: Window) -> (Window, bool) {
        match shared::game_behind(&self.connection, self.atoms, window) {
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
        match self.active_window() {
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
        if let Ok(Some(active)) = self.active_window() {
            let (game, _) = self.game_in_front(active);
            if let Err(error) = self.give_back_keyboard(game) {
                log::warn!("The keyboard can't be given back to the game: {error}");
            }
        }
    }

    /// Asks the desktop to make `game` the active window, and waits until it is, up to
    /// [`ACTIVATION_WAIT`].
    fn bring_back(&self, game: Window) -> Result<()> {
        shared::activate(&self.connection, self.root, self.atoms, game)?;
        let start = Instant::now();
        while start.elapsed() < ACTIVATION_WAIT {
            if self.active_window()? == Some(game) {
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
        let focused = self.connection.get_input_focus()?.reply()?.focus;
        let ours = || shared::is_ours(&self.connection, self.atoms, focused);
        if gives_keyboard_back(focused, game, self.root, ours)? {
            self.connection
                .set_input_focus(InputFocus::PARENT, game, x11rb::CURRENT_TIME)?
                .check()?;
            log::info!("The price-check hotkey gave the keyboard back to the game");
        }
        Ok(())
    }

    /// The active window, none when the desktop gives the focus to a window outside X11.
    fn active_window(&self) -> Result<Option<Window>> {
        shared::active_window(&self.connection, self.root, self.atoms)
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

/// The class of a window from its `WM_CLASS`, an instance name then a class name, each ended
/// by a null byte. Any program names its windows: the class is written to the log on one short
/// line.
fn window_class(property: &[u8]) -> String {
    let names: Vec<_> = property
        .split(|&byte| byte == 0)
        .filter(|name| !name.is_empty())
        .map(String::from_utf8_lossy)
        .collect();
    let class = match names.as_slice() {
        [] => "unknown".to_owned(),
        [instance, class, ..] if instance != class => format!("{class} ({instance})"),
        [name, ..] => name.to_string(),
    };
    logging::plain(&class)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOT: Window = 0x410;
    const GAME: Window = 0x660_0001;
    const OVERLAY: Window = 0x3e0_000a;
    const BROWSER: Window = 0x220_0004;

    /// Whether the hotkey gives the keyboard at `focused` back to the game, `focused` being a
    /// window of this application or not.
    fn gives_back(focused: Window, ours: bool) -> bool {
        gives_keyboard_back(focused, GAME, ROOT, || Ok(ours)).unwrap()
    }

    #[test]
    fn gives_the_keyboard_back_from_the_window_over_the_game() {
        assert!(gives_back(OVERLAY, true));
    }

    #[test]
    fn gives_the_keyboard_back_from_no_window() {
        // The window over the game hid at the same frame: the keyboard went to the root window.
        assert!(gives_back(ROOT, false));
        assert!(gives_back(x11rb::NONE, false));
    }

    #[test]
    fn leaves_the_keyboard_in_the_game_and_in_another_application() {
        assert!(!gives_back(GAME, true));
        assert!(!gives_back(BROWSER, false));
        // The window manager gives it to the window under the pointer: its choice stays.
        assert!(!gives_back(u32::from(InputFocus::POINTER_ROOT), false));
    }

    #[test]
    fn reads_the_owner_of_the_window_with_the_keyboard_only_when_needed() {
        let unread = || -> Result<bool> { panic!("read") };
        assert!(!gives_keyboard_back(GAME, GAME, ROOT, unread).unwrap());
        assert!(gives_keyboard_back(ROOT, GAME, ROOT, unread).unwrap());
    }

    /// Two keysyms per keycode from keycode 8: `a A`, `c C`, `Control_L`, `ccedilla c`.
    const MAPPING: [u32; 8] = [0x61, 0x41, 0x63, 0x43, XK_CONTROL_L, 0, 0xe7, 0x63];

    #[test]
    fn finds_the_keycode_of_a_keysym_without_modifier_first() {
        assert_eq!(keycode_of(XK_C, 8, 2, &MAPPING), Some(9));
        assert_eq!(keycode_of(XK_CONTROL_L, 8, 2, &MAPPING), Some(10));
    }

    /// A keyboard mapping from keycode 8: `a`, `Alt_L` (`Meta_L` with Shift), `Shift_R`,
    /// `Control_L`, then `Alt_R` on keycode 17.
    const MODIFIER_MAPPING: [u32; 20] = [
        0x61,
        0x41,
        XK_ALT_L,
        XK_META_L,
        XK_SHIFT_R,
        0,
        XK_CONTROL_L,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        XK_ALT_R,
        0,
    ];

    #[test]
    fn finds_the_alt_and_shift_keys_held_and_leaves_ctrl() {
        // Keycodes 8 to 11 held (the four lowest bits of the second byte), and 17 (the second
        // bit of the third byte).
        let mut keymap = [0; 32];
        keymap[1] = 0b0000_1111;
        keymap[2] = 0b0000_0010;
        assert_eq!(
            held_modifiers(&keymap, 8, 2, &MODIFIER_MAPPING),
            [
                (9, Modifier::Alt),
                (10, Modifier::Shift),
                (17, Modifier::Alt)
            ]
        );
        // `a` and Ctrl only.
        keymap[1] = 0b0000_1001;
        keymap[2] = 0;
        assert_eq!(held_modifiers(&keymap, 8, 2, &MODIFIER_MAPPING), []);
    }

    /// The keycodes of [`MODIFIER_MAPPING`].
    fn modifier_keycode(keysym: u32) -> Option<Keycode> {
        keycode_of(keysym, 8, 2, &MODIFIER_MAPPING)
    }

    #[test]
    fn the_modifiers_of_the_hotkey_no_longer_held_are_released_for_the_game() {
        let alt_d: Shortcut = "Alt+D".parse().unwrap();
        // Both Alt keys (Meta, on the left Alt too, once only); no Shift, not in the hotkey.
        assert_eq!(
            missed_modifiers(alt_d, &[], modifier_keycode),
            [(9, Modifier::Alt), (17, Modifier::Alt)]
        );
        // An Alt still held, left or right: none, the other one never pressed.
        assert_eq!(
            missed_modifiers(alt_d, &[(9, Modifier::Alt)], modifier_keycode),
            []
        );
        assert_eq!(
            missed_modifiers(alt_d, &[(17, Modifier::Alt)], modifier_keycode),
            []
        );
        // A Shift held tells nothing of Alt.
        assert_eq!(
            missed_modifiers(alt_d, &[(10, Modifier::Shift)], modifier_keycode),
            [(9, Modifier::Alt), (17, Modifier::Alt)]
        );
        // Ctrl+D: nothing.
        assert_eq!(
            missed_modifiers(Shortcut::default(), &[], modifier_keycode),
            []
        );
        let shift_d: Shortcut = "Ctrl+Shift+D".parse().unwrap();
        assert_eq!(
            missed_modifiers(shift_d, &[], modifier_keycode),
            [(10, Modifier::Shift)]
        );
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

    const FOLLOWED: Window = 0x0060_0001;
    const ATOMS: WatchAtoms = WatchAtoms {
        active_window: 300,
        supported: 301,
        net_wm_name: 302,
        utf8_string: 303,
    };

    /// A change of `atom` on `window`.
    fn changed(window: Window, atom: Atom) -> PropertyNotifyEvent {
        PropertyNotifyEvent {
            response_type: xproto::PROPERTY_NOTIFY_EVENT,
            sequence: 0,
            window,
            atom,
            time: 0,
            state: xproto::Property::NEW_VALUE,
        }
    }

    #[test]
    fn follows_the_active_window_of_the_root_window_only() {
        assert!(matters(
            &changed(ROOT, ATOMS.active_window),
            ROOT,
            None,
            ATOMS
        ));
        // `_NET_CLIENT_LIST` and the others on the root window.
        assert!(!matters(&changed(ROOT, 400), ROOT, None, ATOMS));
        // The title of the root window.
        assert!(!matters(
            &changed(ROOT, ATOMS.net_wm_name),
            ROOT,
            Some(ROOT),
            ATOMS
        ));
    }

    #[test]
    fn follows_the_class_and_the_titles_of_the_active_window() {
        for atom in [
            AtomEnum::WM_CLASS.into(),
            AtomEnum::WM_NAME.into(),
            ATOMS.net_wm_name,
        ] {
            assert!(matters(
                &changed(FOLLOWED, atom),
                ROOT,
                Some(FOLLOWED),
                ATOMS
            ));
        }
    }

    #[test]
    fn leaves_the_other_properties_and_the_windows_no_longer_active() {
        // `_NET_WM_USER_TIME`, written at each key.
        assert!(!matters(
            &changed(FOLLOWED, 401),
            ROOT,
            Some(FOLLOWED),
            ATOMS
        ));
        // The title of a window that was active before.
        assert!(!matters(
            &changed(FOLLOWED + 1, ATOMS.net_wm_name),
            ROOT,
            Some(FOLLOWED),
            ATOMS
        ));
        assert!(!matters(
            &changed(FOLLOWED, ATOMS.net_wm_name),
            ROOT,
            None,
            ATOMS
        ));
    }

    #[test]
    fn follows_the_game_only_on_a_desktop_that_tells_its_active_window() {
        let supported = [ATOMS.supported, 404, ATOMS.active_window];
        assert!(supports(supported.into_iter(), ATOMS.active_window));
        assert!(!supports([404, 405].into_iter(), ATOMS.active_window));
        // No `_NET_SUPPORTED` at all: a window manager not started yet, or none.
        assert!(!supports(std::iter::empty(), ATOMS.active_window));
    }

    #[test]
    fn never_follows_the_root_window_as_the_active_window() {
        assert_eq!(known_active(Some(ROOT), ROOT), None);
        assert_eq!(known_active(Some(FOLLOWED), ROOT), Some(FOLLOWED));
        assert_eq!(known_active(None, ROOT), None);
    }

    #[test]
    fn reads_the_old_title_only_without_net_wm_name() {
        let old = || -> Result<Vec<u8>> { Ok(b"Path of Exile 2".to_vec()) };
        assert_eq!(title(Vec::new(), old).unwrap(), b"Path of Exile 2");
        assert_eq!(
            title(b"Steam".to_vec(), || panic!("read without need")).unwrap(),
            b"Steam"
        );
    }

    #[test]
    fn tells_the_game_by_its_class_under_proton() {
        assert!(is_game_class(b"steam_app_2694490\0steam_app_2694490\0"));
        assert!(is_game_class(b"steam_app_2694490"));
    }

    #[test]
    fn tells_the_game_by_its_exact_title_only() {
        assert!(is_game_title(b"Path of Exile 2"));
        // Path of Exile 1, a page of a browser about the game.
        assert!(!is_game_title(b"Path of Exile"));
        assert!(!is_game_title(b"Path of Exile 2 - Mozilla Firefox"));
        assert!(!is_game_title(b""));
    }

    #[test]
    fn tells_another_window_by_its_class() {
        // Path of Exile 1, the Steam client, a class that only starts like the game's.
        assert!(!is_game_class(b"steam_app_238960\0steam_app_238960\0"));
        assert!(!is_game_class(b"steam\0steam\0"));
        assert!(!is_game_class(b"steam_app_26944900\0"));
        assert!(!is_game_class(b""));
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
        // A window named to write a line of its own in the log.
        assert_eq!(
            window_class(b"a\n12:00:00 [INFO] fake\0b\0"),
            "b (a 12:00:00 [INFO] fake)"
        );
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
