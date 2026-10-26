//! The text of a number field, which keeps the characters of a number only, also when a text
//! is pasted: digits, one point, and a minus sign first when the number may be negative.

use std::ops::Range;

use egui::TextBuffer;
use egui::text::CharIndex;

pub struct NumberField<'a> {
    pub text: &'a mut String,
    /// The minus sign is allowed.
    pub negative: bool,
}

impl NumberField<'_> {
    /// The characters of `inserted` kept at `index` (in characters) in the text.
    fn accepted(&self, inserted: &str, index: usize) -> String {
        let text: &str = self.text;
        let mut has_point = text.contains('.');
        let mut kept = String::new();
        for c in inserted.chars() {
            let first = index == 0 && kept.is_empty();
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

    fn delete_char_range(&mut self, char_range: Range<CharIndex>) {
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
    fn a_number_field_keeps_the_characters_of_a_number_only() {
        assert_eq!(typed("4", "2", 1, false), "42");
        // No letter, not even the `e` of an exponent; a pasted text keeps its number.
        assert_eq!(typed("4", "e3", 1, false), "43");
        assert_eq!(typed("", "about 12.5%", 0, false), "12.5");
        // One point.
        assert_eq!(typed("1.5", ".", 3, false), "1.5");
        assert_eq!(typed("15", "1.2.3", 2, false), "151.23");
    }

    #[test]
    fn a_minus_sign_is_kept_first_for_a_negative_number_only() {
        assert_eq!(typed("15", "-", 0, false), "15");
        assert_eq!(typed("15", "-", 0, true), "-15");
        assert_eq!(typed("15", "-", 1, true), "15");
        assert_eq!(typed("-15", "-", 0, true), "-15");
        // Nothing before the sign.
        assert_eq!(typed("-15", "2", 0, true), "-15");
        assert_eq!(typed("", "-20", 0, true), "-20");
    }
}
