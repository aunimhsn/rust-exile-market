use super::*;
use x11rb::protocol::xproto::InputFocus;

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

/// The keycodes of the fake keyboard: one keysym for each, from 8.
const CONTROL: Keycode = 8;
const C: Keycode = 9;
const SHIFT: Keycode = 10;

/// An X server that records what the copy asks of it, in order, and answers the readings
/// it was given, in their order; a reading it has no answer left for fails.
struct FakeServer {
    calls: std::cell::RefCell<Vec<String>>,
    active: Option<Window>,
    refused: bool,
    keysyms: Vec<u32>,
    pointers: std::cell::RefCell<std::collections::VecDeque<Pointer>>,
    keys_held: std::cell::RefCell<std::collections::VecDeque<Vec<u8>>>,
    /// The delay asked before each key.
    delays: std::cell::RefCell<Vec<u32>>,
}

/// The pointer on the item, Ctrl held or not.
fn pointer(control: bool) -> Pointer {
    Pointer {
        position: Point { x: 300, y: 200 },
        control,
    }
}

/// The keys held: Shift, or none.
fn keys(shift: bool) -> Vec<u8> {
    let mut keymap = vec![0; 32];
    if shift {
        keymap[usize::from(SHIFT / 8)] |= 1 << (SHIFT % 8);
    }
    keymap
}

impl FakeServer {
    /// The game active, each reading answered three times: before the marker, for the
    /// window over the game, and right before the keys.
    fn new(control: [bool; 3], shift: [bool; 2]) -> Self {
        Self {
            calls: std::cell::RefCell::default(),
            active: Some(GAME),
            refused: false,
            keysyms: vec![XK_CONTROL_L, XK_C, XK_SHIFT_L],
            pointers: std::cell::RefCell::new(control.map(pointer).into()),
            keys_held: std::cell::RefCell::new(shift.map(keys).into()),
            delays: std::cell::RefCell::default(),
        }
    }

    fn call(&self, call: impl Into<String>) {
        self.calls.borrow_mut().push(call.into());
    }

    /// Sends the copy of Ctrl+D; what was asked of the server, in order, with the
    /// preparation and the window over the game told among it.
    fn copy(&self) -> (Result<Sent>, Vec<String>) {
        let sent = send_copy(
            self,
            Shortcut::default(),
            || self.call("prepare"),
            |target, ()| self.call(format!("tell {}", target.is_some())),
        );
        (sent, self.calls.borrow().clone())
    }
}

impl Server for FakeServer {
    fn active_game(&self) -> (Result<Option<Window>>, bool) {
        self.call("active game");
        (Ok(self.active), self.refused)
    }

    fn window_class(&self, _: Window) -> Result<String> {
        Ok("steam_app_2694490".to_owned())
    }

    fn give_back_keyboard(&self, _: Window) -> Result<()> {
        self.call("keyboard back");
        Ok(())
    }

    fn window_rect(&self, _: Window) -> Result<Option<ScreenRect>> {
        Ok(Some(ScreenRect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        }))
    }

    fn drop_events(&self) {
        self.call("drop events");
    }

    fn layout(&self) -> Result<Layout> {
        self.call("layout");
        Ok(Layout {
            min_keycode: CONTROL,
            keysyms_per_keycode: 1,
            keysyms: self.keysyms.clone(),
        })
    }

    fn pointer(&self) -> Result<Pointer> {
        self.call("pointer");
        let pointer = self.pointers.borrow_mut().pop_front();
        pointer.ok_or_else(|| "no pointer".into())
    }

    fn keys_held(&self) -> Result<Vec<u8>> {
        self.call("keys held");
        let keys = self.keys_held.borrow_mut().pop_front();
        keys.ok_or_else(|| "no keymap".into())
    }

    fn game_in_front_again(&self) {
        self.call("game in front again");
    }

    fn fake_key(&self, key: Key<Keycode>, delay_ms: u32) -> Result<()> {
        self.call(format!("{key:?}"));
        self.delays.borrow_mut().push(delay_ms);
        Ok(())
    }
}

/// The keys sent among `calls`.
fn keys_sent(calls: &[String]) -> Vec<&str> {
    calls
        .iter()
        .map(String::as_str)
        .filter(|call| call.starts_with("Down") || call.starts_with("Up"))
        .collect()
}

/// The desktop review of 2026-11-10: the order of these steps is the rule, and no test ran
/// it.
#[test]
fn the_window_is_told_before_the_first_key_and_the_keys_held_are_read_last() {
    let server = FakeServer::new([true; 3], [false; 2]);

    let (sent, calls) = server.copy();

    assert_eq!(
        calls,
        [
            "active game",
            "keyboard back",
            "drop events",
            "layout",
            "pointer",
            "keys held",
            "prepare",
            "pointer",
            "tell true",
            "game in front again",
            "pointer",
            "keys held",
            &format!("{:?}", Key::Down(C)),
            &format!("{:?}", Key::Up(C)),
        ]
    );
    assert!(sent.expect("sent").control_held);
    // The first key at once, the next ones a moment apart, as a hand would press them.
    assert_eq!(*server.delays.borrow(), [0, KEY_DELAY_MS]);
}

/// Read before the wait for the game to be active again, a Ctrl released during it sent C
/// alone to the game: a key of the game pressed.
#[test]
fn a_ctrl_released_while_the_game_is_made_active_again_is_pressed_by_the_copy() {
    let server = FakeServer::new([true, true, false], [false; 2]);

    let (sent, calls) = server.copy();

    assert_eq!(keys_sent(&calls), ["Down(8)", "Down(9)", "Up(9)", "Up(8)"]);
    assert!(!sent.expect("sent").control_held);
}

/// And a Shift released during it was pressed again last, left held for the X server.
#[test]
fn a_shift_released_while_the_game_is_made_active_again_isnt_pressed_again() {
    let server = FakeServer::new([true; 3], [true, false]);

    let (sent, calls) = server.copy();

    assert_eq!(keys_sent(&calls), ["Down(9)", "Up(9)"]);
    assert_eq!(sent.expect("sent").released, [""; 0]);
}

#[test]
fn a_shift_still_held_is_released_around_the_copy() {
    let server = FakeServer::new([true; 3], [false, true]);

    let (sent, calls) = server.copy();

    assert_eq!(
        keys_sent(&calls),
        ["Up(10)", "Down(9)", "Up(9)", "Down(10)"]
    );
    assert_eq!(sent.expect("sent").released, ["Shift"]);
}

/// A marker without a copy would replace the item of the player in the clipboard.
#[test]
fn what_fails_before_the_marker_sends_no_key_and_leaves_the_clipboard_alone() {
    let without_c = FakeServer {
        keysyms: vec![XK_CONTROL_L],
        ..FakeServer::new([true; 3], [false; 2])
    };
    let without_pointer = FakeServer {
        pointers: std::cell::RefCell::default(),
        ..FakeServer::new([true; 3], [false; 2])
    };
    let without_keymap = FakeServer {
        keys_held: std::cell::RefCell::default(),
        ..FakeServer::new([true; 3], [false; 2])
    };

    for server in [without_c, without_pointer, without_keymap] {
        let (sent, calls) = server.copy();

        assert!(sent.is_err());
        assert!(!calls.iter().any(|call| call == "prepare"), "{calls:?}");
        assert!(
            !calls.iter().any(|call| call.starts_with("tell")),
            "{calls:?}"
        );
        assert!(keys_sent(&calls).is_empty(), "{calls:?}");
    }
}

#[test]
fn once_the_marker_is_written_a_failed_reading_keeps_the_first_one_and_the_keys_leave() {
    // One answer each: the readings after the marker fail.
    let server = FakeServer {
        pointers: std::cell::RefCell::new([pointer(false)].into()),
        keys_held: std::cell::RefCell::new([keys(true)].into()),
        ..FakeServer::new([true; 3], [false; 2])
    };

    let (sent, calls) = server.copy();

    assert!(calls.iter().any(|call| call == "tell true"), "{calls:?}");
    assert_eq!(
        keys_sent(&calls),
        ["Up(10)", "Down(8)", "Down(9)", "Up(9)", "Up(8)", "Down(10)"]
    );
    assert!(sent.is_ok());
}

#[test]
fn a_desktop_that_refused_the_game_isnt_asked_twice_and_no_window_gets_no_keyboard_back() {
    let refused = FakeServer {
        refused: true,
        ..FakeServer::new([true; 3], [false; 2])
    };
    let (_, calls) = refused.copy();
    assert!(!calls.iter().any(|call| call == "game in front again"));
    assert_eq!(keys_sent(&calls), ["Down(9)", "Up(9)"]);

    let nowhere = FakeServer {
        active: None,
        ..FakeServer::new([true; 3], [false; 2])
    };
    let (sent, calls) = nowhere.copy();
    assert!(!calls.iter().any(|call| call == "keyboard back"));
    // No window to open over, and the keys still leave, to whatever has the keyboard.
    assert!(calls.iter().any(|call| call == "tell false"), "{calls:?}");
    assert!(sent.expect("sent").window_class.starts_with("none"));
}
