//! The keys of the price-check hotkey, as the settings keep them (`Alt+D`), as the window draws
//! them and as each system grabs them.

use std::fmt;
use std::str::FromStr;

#[cfg(any(target_os = "linux", target_os = "windows"))]
use global_hotkey::hotkey::Code;

/// A combination of keys: Ctrl, Shift or Alt held, then one key pressed.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Shortcut {
    control: bool,
    shift: bool,
    alt: bool,
    key: KeyName,
}

/// The key pressed while the modifiers are held.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum KeyName {
    /// From `A` to `Z`, its index from `A`.
    Letter(u8),
    /// From `0` to `9`.
    Digit(u8),
    /// From `F1` to `F12`, its number.
    Function(u8),
}

const LETTERS: [&str; 26] = [
    "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R", "S",
    "T", "U", "V", "W", "X", "Y", "Z",
];
const DIGITS: [&str; 10] = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"];
const FUNCTIONS: [&str; 12] = [
    "F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12",
];

/// The codes of [`LETTERS`] for `global-hotkey`, in the same order.
#[cfg(any(target_os = "linux", target_os = "windows"))]
const LETTER_CODES: [Code; 26] = [
    Code::KeyA,
    Code::KeyB,
    Code::KeyC,
    Code::KeyD,
    Code::KeyE,
    Code::KeyF,
    Code::KeyG,
    Code::KeyH,
    Code::KeyI,
    Code::KeyJ,
    Code::KeyK,
    Code::KeyL,
    Code::KeyM,
    Code::KeyN,
    Code::KeyO,
    Code::KeyP,
    Code::KeyQ,
    Code::KeyR,
    Code::KeyS,
    Code::KeyT,
    Code::KeyU,
    Code::KeyV,
    Code::KeyW,
    Code::KeyX,
    Code::KeyY,
    Code::KeyZ,
];
/// The codes of [`DIGITS`] for `global-hotkey`, in the same order.
#[cfg(any(target_os = "linux", target_os = "windows"))]
const DIGIT_CODES: [Code; 10] = [
    Code::Digit0,
    Code::Digit1,
    Code::Digit2,
    Code::Digit3,
    Code::Digit4,
    Code::Digit5,
    Code::Digit6,
    Code::Digit7,
    Code::Digit8,
    Code::Digit9,
];
/// The codes of [`FUNCTIONS`] for `global-hotkey`, in the same order.
#[cfg(any(target_os = "linux", target_os = "windows"))]
const FUNCTION_CODES: [Code; 12] = [
    Code::F1,
    Code::F2,
    Code::F3,
    Code::F4,
    Code::F5,
    Code::F6,
    Code::F7,
    Code::F8,
    Code::F9,
    Code::F10,
    Code::F11,
    Code::F12,
];

const CONTROL: &str = "Ctrl";
const SHIFT: &str = "Shift";
const ALT: &str = "Alt";

impl KeyName {
    fn name(self) -> &'static str {
        match self {
            Self::Letter(index) => LETTERS[usize::from(index)],
            Self::Digit(digit) => DIGITS[usize::from(digit)],
            Self::Function(number) => FUNCTIONS[usize::from(number - 1)],
        }
    }

    /// Its code for `global-hotkey`, which grabs it.
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    fn code(self) -> Code {
        match self {
            Self::Letter(index) => LETTER_CODES[usize::from(index)],
            Self::Digit(digit) => DIGIT_CODES[usize::from(digit)],
            Self::Function(number) => FUNCTION_CODES[usize::from(number - 1)],
        }
    }

    /// The key named `name`, whatever its case (`d`, `f5`).
    fn named(name: &str) -> Option<Self> {
        let position = |names: &[&str]| {
            names
                .iter()
                .position(|known| known.eq_ignore_ascii_case(name))
                .and_then(|index| u8::try_from(index).ok())
        };
        position(&LETTERS)
            .map(Self::Letter)
            .or_else(|| position(&DIGITS).map(Self::Digit))
            .or_else(|| position(&FUNCTIONS).map(|index| Self::Function(index + 1)))
    }
}

/// Why keys can't be the hotkey.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortcutError {
    /// Not a combination of Ctrl, Shift or Alt and one letter, number or function key.
    Unreadable,
    /// A letter or a number without Ctrl nor Alt, which would type in the chat of the game.
    WithoutControlOrAlt,
    /// Ctrl+C, the copy the hotkey sends itself, or Ctrl+Alt+C, which it sends while the player
    /// holds Alt: it would press itself again.
    Copy,
}

impl fmt::Display for ShortcutError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Unreadable => {
                "not Ctrl, Shift or Alt with one letter, number or function key (F1 to F12)"
            }
            Self::WithoutControlOrAlt => "a letter or a number needs Ctrl or Alt",
            Self::Copy => "Ctrl+C and Ctrl+Alt+C are the copies the hotkey sends",
        })
    }
}

impl Shortcut {
    /// Ctrl+D, the default, the one of Exiled Exchange 2.
    pub const PRICE_CHECK: Self = Self::control(KeyName::Letter(3));

    /// The keys the settings offer, the default first: two keys at most, none the game, the chat,
    /// Exiled Exchange 2 or the systems use by default (`specs.md`, Settings, checked on
    /// 2026-11-03): G sits at the same place on QWERTY, AZERTY and QWERTZ, and the game leaves F3
    /// and F4 free.
    pub const CHOICES: [Self; 5] = [
        Self::PRICE_CHECK,
        Self::control(KeyName::Letter(6)),
        Self {
            alt: true,
            ..Self::alone(KeyName::Letter(6))
        },
        Self::alone(KeyName::Function(3)),
        Self::alone(KeyName::Function(4)),
    ];

    /// `key` without modifier.
    const fn alone(key: KeyName) -> Self {
        Self {
            control: false,
            shift: false,
            alt: false,
            key,
        }
    }

    /// Ctrl and `key`.
    const fn control(key: KeyName) -> Self {
        Self {
            control: true,
            ..Self::alone(key)
        }
    }

    /// Its keys in order, the modifiers first, as key caps or joined by "+": `Ctrl`, `D`.
    #[must_use]
    pub fn keys(self) -> Vec<&'static str> {
        [
            self.control.then_some(CONTROL),
            self.shift.then_some(SHIFT),
            self.alt.then_some(ALT),
            Some(self.key.name()),
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    /// Whether Shift is held with its key.
    #[cfg(target_os = "linux")]
    #[must_use]
    pub fn shift(self) -> bool {
        self.shift
    }

    /// Whether Alt is held with its key.
    #[cfg(target_os = "linux")]
    #[must_use]
    pub fn alt(self) -> bool {
        self.alt
    }

    /// The same combination for `global-hotkey`, which grabs it.
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    #[must_use]
    pub(super) fn hotkey(self) -> global_hotkey::hotkey::HotKey {
        use global_hotkey::hotkey::{HotKey, Modifiers};

        let mut modifiers = Modifiers::empty();
        modifiers.set(Modifiers::CONTROL, self.control);
        modifiers.set(Modifiers::SHIFT, self.shift);
        modifiers.set(Modifiers::ALT, self.alt);
        HotKey::new(
            (!modifiers.is_empty()).then_some(modifiers),
            self.key.code(),
        )
    }
}

impl Default for Shortcut {
    fn default() -> Self {
        Self::PRICE_CHECK
    }
}

impl fmt::Debug for Shortcut {
    /// Its keys joined, as the log writes the settings: `Shortcut(Alt+D)`.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Shortcut({self})")
    }
}

impl fmt::Display for Shortcut {
    /// Its keys joined, as in a sentence and in the settings file: `Ctrl+Shift+D`.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.keys().join("+"))
    }
}

impl FromStr for Shortcut {
    type Err = ShortcutError;

    /// Reads keys joined by "+", whatever their case and order, the spaces around them left out
    /// (`alt + d`); `Control` is read as `Ctrl`.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (mut control, mut shift, mut alt, mut key) = (false, false, false, None);
        for name in text.split('+').map(str::trim) {
            let modifier =
                if name.eq_ignore_ascii_case(CONTROL) || name.eq_ignore_ascii_case("Control") {
                    &mut control
                } else if name.eq_ignore_ascii_case(SHIFT) {
                    &mut shift
                } else if name.eq_ignore_ascii_case(ALT) {
                    &mut alt
                } else {
                    // One key only.
                    match (key, KeyName::named(name)) {
                        (None, Some(named)) => {
                            key = Some(named);
                            continue;
                        }
                        _ => return Err(ShortcutError::Unreadable),
                    }
                };
            if std::mem::replace(modifier, true) {
                return Err(ShortcutError::Unreadable);
            }
        }
        let key = key.ok_or(ShortcutError::Unreadable)?;
        if !matches!(key, KeyName::Function(_)) && !control && !alt {
            return Err(ShortcutError::WithoutControlOrAlt);
        }
        if control && !shift && key == KeyName::Letter(2) {
            return Err(ShortcutError::Copy);
        }
        Ok(Self {
            control,
            shift,
            alt,
            key,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(text: &str) -> Result<String, ShortcutError> {
        text.parse::<Shortcut>()
            .map(|shortcut| shortcut.to_string())
    }

    #[test]
    fn the_log_writes_its_keys() {
        assert_eq!(format!("{:?}", Shortcut::default()), "Shortcut(Ctrl+D)");
    }

    #[test]
    fn the_default_is_ctrl_d() {
        assert_eq!(Shortcut::default().keys(), ["Ctrl", "D"]);
        assert_eq!(Shortcut::default().to_string(), "Ctrl+D");
        assert_eq!(read("Ctrl+D"), Ok("Ctrl+D".to_owned()));
    }

    #[test]
    fn the_choices_are_the_default_then_ctrl_g_alt_g_f3_and_f4() {
        let choices = Shortcut::CHOICES.map(|shortcut| shortcut.to_string());
        assert_eq!(choices, ["Ctrl+D", "Ctrl+G", "Alt+G", "F3", "F4"]);
        assert_eq!(Shortcut::CHOICES[0], Shortcut::default());
    }

    #[test]
    fn each_choice_is_two_keys_at_most_and_reads_back_from_the_file() {
        for choice in Shortcut::CHOICES {
            assert!(choice.keys().len() <= 2, "{choice}");
            assert_eq!(choice.to_string().parse(), Ok(choice));
        }
    }

    #[test]
    fn reads_the_keys_whatever_their_case_order_and_spaces() {
        assert_eq!(read("alt + d"), Ok("Alt+D".to_owned()));
        assert_eq!(read("D+Shift+Control"), Ok("Ctrl+Shift+D".to_owned()));
        assert_eq!(read("Ctrl+Alt+Shift+7"), Ok("Ctrl+Shift+Alt+7".to_owned()));
        assert_eq!(read("f5"), Ok("F5".to_owned()));
        assert_eq!(read("Shift+F12"), Ok("Shift+F12".to_owned()));
    }

    #[test]
    fn a_shortcut_written_reads_back_the_same() {
        for text in ["Ctrl+D", "Alt+D", "Ctrl+Shift+D", "Alt+0", "F11", "Ctrl+F1"] {
            let shortcut: Shortcut = text.parse().expect(text);
            assert_eq!(shortcut.to_string().parse(), Ok(shortcut));
        }
    }

    #[test]
    fn refuses_what_is_not_one_key_with_modifiers() {
        for text in [
            "",
            "Ctrl",
            "Ctrl+",
            "Ctrl+D+E",
            "Ctrl+Ctrl+D",
            "Ctrl+Space",
            "Super+D",
            "F13",
            "Ctrl+DD",
        ] {
            assert_eq!(read(text), Err(ShortcutError::Unreadable), "{text}");
        }
    }

    #[test]
    fn a_letter_or_a_number_needs_ctrl_or_alt() {
        assert_eq!(read("D"), Err(ShortcutError::WithoutControlOrAlt));
        assert_eq!(read("Shift+D"), Err(ShortcutError::WithoutControlOrAlt));
        assert_eq!(read("5"), Err(ShortcutError::WithoutControlOrAlt));
    }

    #[test]
    fn refuses_the_copy_it_sends() {
        assert_eq!(read("Ctrl+C"), Err(ShortcutError::Copy));
        assert_eq!(read("ctrl + c"), Err(ShortcutError::Copy));
        // Sent while the player holds Alt.
        assert_eq!(read("Ctrl+Alt+C"), Err(ShortcutError::Copy));
        // Other combinations: the copy sent never presses them (Shift released, Ctrl pressed).
        assert_eq!(read("Ctrl+Shift+C"), Ok("Ctrl+Shift+C".to_owned()));
        assert_eq!(read("Ctrl+Shift+Alt+C"), Ok("Ctrl+Shift+Alt+C".to_owned()));
        assert_eq!(read("Alt+C"), Ok("Alt+C".to_owned()));
    }

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    #[test]
    fn grabs_the_same_keys() {
        use global_hotkey::hotkey::{Code, HotKey, Modifiers};

        assert_eq!(
            Shortcut::default().hotkey(),
            HotKey::new(Some(Modifiers::CONTROL), Code::KeyD)
        );
        assert_eq!(
            "Ctrl+Shift+Alt+Z".parse::<Shortcut>().unwrap().hotkey(),
            HotKey::new(
                Some(Modifiers::CONTROL | Modifiers::SHIFT | Modifiers::ALT),
                Code::KeyZ
            )
        );
        assert_eq!(
            "Alt+0".parse::<Shortcut>().unwrap().hotkey(),
            HotKey::new(Some(Modifiers::ALT), Code::Digit0)
        );
        assert_eq!(
            "F12".parse::<Shortcut>().unwrap().hotkey(),
            HotKey::new(None, Code::F12)
        );
    }
}
