//! The search of whole words in a text; what ends a word is the rule of each use.

/// Whether `text` holds `words` with a boundary before and after them: a character for which
/// `is_boundary` holds, or an end of `text`.
pub fn holds_whole(text: &str, words: &str, is_boundary: impl Fn(char) -> bool) -> bool {
    text.match_indices(words).any(|(start, _)| {
        let before = text[..start].chars().next_back();
        let after = text[start + words.len()..].chars().next();
        before.is_none_or(&is_boundary) && after.is_none_or(&is_boundary)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn holds(text: &str, words: &str) -> bool {
        holds_whole(text, words, |character| character == ' ')
    }

    #[test]
    fn whole_words_are_found_at_both_ends_and_after_a_part_of_a_word() {
        assert!(holds("Temple Maul", "Temple Maul"), "the whole text");
        assert!(
            holds("Temple Maul of the Brute", "Temple Maul"),
            "at the start"
        );
        assert!(holds("Crackling Temple Maul", "Temple Maul"), "at the end");
        // The first "Maul" is inside "Mauling", the second one is a word.
        assert!(holds("Mauling Maul", "Maul"));
        assert!(!holds("Mauling", "Maul"));
        assert!(!holds("Shammaul", "Maul"));
    }

    #[test]
    fn the_boundary_is_the_one_given() {
        let between_spaces = |text| holds_whole(text, "less", |character| character == ' ');
        let between_symbols =
            |text| holds_whole(text, "less", |character| !character.is_alphanumeric());

        assert_eq!(
            [between_spaces("less, more"), between_symbols("less, more")],
            [false, true]
        );
        assert_eq!(
            [between_spaces("unless"), between_symbols("unless")],
            [false, false]
        );
    }
}
