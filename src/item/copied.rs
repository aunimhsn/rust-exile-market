//! The text of an item copied in the game, before it is parsed.

use super::labels::Labels;
use crate::language::Language;

/// The text of a Path of Exile 2 item, as the game copies it with Ctrl+C or Ctrl+Alt+C.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopiedItem {
    text: String,
    language: Language,
}

impl CopiedItem {
    /// The item in a copied text, if the text starts with the item class line, which also
    /// tells the language of the game client; `None` for any other text. The lines end with
    /// `\n` on every system.
    ///
    /// Every text copied by the player comes here, also the private ones: nothing that can
    /// panic may run on it before it is recognized, so that a panic message, which goes to the
    /// log, never carries a text copied outside the game.
    #[must_use]
    pub fn from_text(text: &str) -> Option<Self> {
        let text = text.trim();
        let language = Language::ALL
            .into_iter()
            .find(|&language| text.starts_with(Labels::of(language).item_class))?;
        Some(Self {
            text: text.replace("\r\n", "\n"),
            language,
        })
    }

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
    fn keeps_the_text_without_the_windows_line_ends_and_the_outer_spaces() {
        let item =
            CopiedItem::from_text("  \r\nItem Class: Rings\r\nRarity: Rare\r\n").expect("an item");

        assert_eq!(item.text(), "Item Class: Rings\nRarity: Rare");
    }
}
