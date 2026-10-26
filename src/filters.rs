//! The stat filters of the offers search, as the player sets them: whether each stat of the
//! item is searched, and its minimum. They are kept for the item shown, until another item
//! is copied; a stat the player didn't touch keeps the choice made for it by default.
//!
//! The minimum field takes a number only: digits, one point, and a minus sign first for a
//! negative stat ([`NumberField`]); it is brought back within the range of the stat when the
//! player leaves it ([`within`]).

use std::collections::HashMap;
use std::ops::Range as Span;

use egui::TextBuffer;
use egui::text::CharIndex;

use crate::format;
use crate::item::Range;

/// A line of a modifier of the item: the index of its modifier among the modifiers of the
/// item, and of the line in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StatKey {
    pub modifier: usize,
    pub line: usize,
}

/// A filter as the player set it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Filter {
    pub checked: bool,
    /// The text of the minimum field: a number, or empty for no minimum.
    pub minimum: String,
}

/// The filters the player changed, for the item shown.
#[derive(Debug, Default)]
pub struct Filters {
    changed: HashMap<StatKey, Filter>,
}

impl Filters {
    /// The filter of `key` as the player set it, or else `default`.
    #[must_use]
    pub fn get(&self, key: StatKey, default: impl FnOnce() -> Filter) -> Filter {
        self.changed.get(&key).cloned().unwrap_or_else(default)
    }

    pub fn set(&mut self, key: StatKey, filter: Filter) {
        self.changed.insert(key, filter);
    }

    /// Forgets the choices of the player: another item is copied.
    pub fn clear(&mut self) {
        self.changed.clear();
    }
}

/// The text of a minimum, e.g. `40`, `7.5`, `-15`.
#[must_use]
pub fn minimum_text(value: f64) -> String {
    format::number(value)
}

/// The minimum written in `text`, brought back within `bounds` when they are known; `None`
/// when `text` isn't a number (empty, `-` or `.` alone).
#[must_use]
pub fn within(text: &str, bounds: Option<Range>) -> Option<f64> {
    let value: f64 = text.parse().ok().filter(|value: &f64| value.is_finite())?;
    Some(bounds.map_or(value, |bounds| value.clamp(bounds.min, bounds.max)))
}

/// The text of a minimum field, which keeps the characters of a number only, also when a
/// text is pasted: digits, one point, and a minus sign first when the stat is `negative`.
pub struct NumberField<'a> {
    pub text: &'a mut String,
    pub negative: bool,
}

impl NumberField<'_> {
    /// The characters of `inserted` kept at `index` (in characters) in the text.
    fn accepted(&self, inserted: &str, index: usize) -> String {
        let text: &str = self.text;
        let before: String = text.chars().take(index).collect();
        let mut has_point = text.contains('.');
        let mut kept = String::new();
        for c in inserted.chars() {
            let first = before.is_empty() && kept.is_empty();
            // Nothing goes before the minus sign.
            let before_sign = first && text.starts_with('-');
            let ok = match c {
                '0'..='9' => !before_sign,
                '.' => !before_sign && !has_point,
                '-' => self.negative && first && !before_sign,
                _ => false,
            };
            if ok {
                has_point |= c == '.';
                kept.push(c);
            }
        }
        kept
    }
}

impl TextBuffer for NumberField<'_> {
    fn is_mutable(&self) -> bool {
        true
    }

    fn as_str(&self) -> &str {
        self.text
    }

    fn insert_text(&mut self, text: &str, char_index: CharIndex) -> usize {
        let kept = self.accepted(text, char_index.0);
        self.text.insert_text(&kept, char_index)
    }

    fn delete_char_range(&mut self, char_range: Span<CharIndex>) {
        self.text.delete_char_range(char_range);
    }

    fn type_id(&self) -> std::any::TypeId {
        std::any::TypeId::of::<NumberField<'static>>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The text of a field holding `text` once `inserted` is typed or pasted at `index`.
    fn typed(text: &str, inserted: &str, index: usize, negative: bool) -> String {
        let mut text = text.to_owned();
        let mut field = NumberField {
            text: &mut text,
            negative,
        };
        field.insert_text(inserted, CharIndex(index));
        text
    }

    #[test]
    fn a_minimum_field_keeps_the_characters_of_a_number_only() {
        assert_eq!(typed("4", "2", 1, false), "42");
        // No letter, not even the `e` of an exponent; a pasted text keeps its number.
        assert_eq!(typed("4", "e3", 1, false), "43");
        assert_eq!(typed("", "about 12.5%", 0, false), "12.5");
        // One point.
        assert_eq!(typed("1.5", ".", 3, false), "1.5");
        assert_eq!(typed("15", "1.2.3", 2, false), "151.23");
    }

    #[test]
    fn a_minus_sign_is_kept_first_for_a_negative_stat_only() {
        assert_eq!(typed("15", "-", 0, false), "15");
        assert_eq!(typed("15", "-", 0, true), "-15");
        assert_eq!(typed("15", "-", 1, true), "15");
        assert_eq!(typed("-15", "-", 0, true), "-15");
        // Nothing before the sign.
        assert_eq!(typed("-15", "2", 0, true), "-15");
        assert_eq!(typed("", "-20", 0, true), "-20");
    }

    #[test]
    fn a_minimum_is_brought_back_within_the_range_of_its_stat() {
        let range = Some(Range {
            min: 40.0,
            max: 60.0,
        });

        assert_eq!(within("55", range), Some(55.0));
        assert_eq!(within("5", range), Some(40.0));
        assert_eq!(within("600", range), Some(60.0));
        assert_eq!(within("600", None), Some(600.0));
        // Not a number: no minimum.
        for text in ["", "-", "."] {
            assert_eq!(within(text, range), None, "{text:?}");
        }
    }
}
