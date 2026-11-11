//! The text of an item copied in the game, before it is parsed.

use super::labels::Labels;
use crate::language::Language;

/// The text of a Path of Exile 2 item, as the game copies it with Ctrl+C or Ctrl+Alt+C.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopiedItem {
    text: String,
    language: Language,
}

/// A text longer than this isn't an item: the longest items of the game are under 4 KB and 60
/// lines. Everything that reads an item (the parser, the tables of the modifiers, the lines
/// drawn at each frame, the log) is then bounded too.
const MAX_BYTES: usize = 16 * 1024;
/// A text of more lines than this isn't an item either.
const MAX_LINES: usize = 300;

impl CopiedItem {
    /// The item in a copied text, if the text starts with the item class line, which also
    /// tells the language of the game client; `None` for any other text. The lines end with
    /// `\n` on every system. A text too long to be an item is refused as any other text.
    ///
    /// Every text copied by the player comes here, also the private ones: nothing that can
    /// panic may run on it before it is recognized, so that a panic message, which goes to the
    /// log, never carries a text copied outside the game.
    #[must_use]
    pub fn from_text(text: &str) -> Option<Self> {
        let text = text.trim();
        if is_too_long(text) {
            return None;
        }
        let language = language_of(text)?;
        Some(Self {
            text: text.replace("\r\n", "\n"),
            language,
        })
    }

    /// Whether `text` starts as an item but is too long to be one: worth a line in the log,
    /// without anything of the text.
    #[must_use]
    pub fn is_too_long_item(text: &str) -> bool {
        let text = text.trim();
        is_too_long(text) && language_of(text).is_some()
    }

    /// The text of the item, its lines ended with `\n`.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The language of the game client that copied the item.
    #[must_use]
    pub fn language(&self) -> Language {
        self.language
    }

    /// The item class line, which says what was copied without its details, for the log.
    #[must_use]
    pub fn first_line(&self) -> &str {
        self.text.lines().next().unwrap_or_default()
    }
}

/// The language of the game client whose item class line `text` starts with; none for a text
/// that doesn't start as an item.
fn language_of(text: &str) -> Option<Language> {
    Language::ALL
        .into_iter()
        .find(|&language| text.starts_with(Labels::of(language).item_class))
}

/// Whether `text` is too long to be an item.
fn is_too_long(text: &str) -> bool {
    text.len() > MAX_BYTES || text.lines().take(MAX_LINES + 1).count() > MAX_LINES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_an_item_in_english_and_in_french() {
        for text in [
            "Item Class: Stackable Currency\nRarity: Currency\nDivine Orb",
            "Classe d'objet: Objets monétaires empilables\nRareté: Objet monétaire\nOrbe divin",
        ] {
            assert_eq!(
                CopiedItem::from_text(text).map(|item| item.text),
                Some(text.to_owned())
            );
        }
    }

    #[test]
    fn tells_the_language_of_the_game_client() {
        for (text, language) in [
            ("Item Class: Rings", Language::English),
            ("Classe d'objet: Anneaux", Language::French),
        ] {
            assert_eq!(
                CopiedItem::from_text(text).map(|item| item.language()),
                Some(language)
            );
        }
    }

    #[test]
    fn ignores_the_other_texts() {
        for text in [
            "",
            "@seller Hi, I would like to buy your Divine Orb",
            "Rarity: Currency\nItem Class: Stackable Currency",
            "Item Class:Rings",
        ] {
            assert_eq!(CopiedItem::from_text(text), None, "{text:?}");
        }
    }

    #[test]
    fn refuses_a_text_too_long_to_be_an_item() {
        let lines = |count: usize| format!("Item Class: Rings{}", "\nx (rune)".repeat(count - 1));
        let bytes = |count: usize| format!("Item Class: {}", "x".repeat(count - 12));

        assert!(CopiedItem::from_text(&lines(MAX_LINES)).is_some());
        assert_eq!(CopiedItem::from_text(&lines(MAX_LINES + 1)), None);
        assert!(CopiedItem::from_text(&bytes(MAX_BYTES)).is_some());
        assert_eq!(CopiedItem::from_text(&bytes(MAX_BYTES + 1)), None);
        assert!(CopiedItem::is_too_long_item(&bytes(MAX_BYTES + 1)));
        assert!(!CopiedItem::is_too_long_item(&bytes(MAX_BYTES)));
        assert!(!CopiedItem::is_too_long_item(&"x".repeat(MAX_BYTES + 1)));
    }

    #[test]
    fn keeps_the_text_without_the_windows_line_ends_and_the_outer_spaces() {
        let item =
            CopiedItem::from_text("  \r\nItem Class: Rings\r\nRarity: Rare\r\n").expect("an item");

        assert_eq!(item.text(), "Item Class: Rings\nRarity: Rare");
    }
}
