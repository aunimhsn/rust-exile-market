//! The text of an item copied in the game, before it is parsed.

/// The first line of an item copied in the game, in each language of the game client.
const ITEM_CLASS_PREFIXES: [&str; 2] = ["Item Class: ", "Classe d'objet: "];

/// The text of a Path of Exile 2 item, as the game copies it with Ctrl+C or Ctrl+Alt+C.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopiedItem {
    text: String,
}

impl CopiedItem {
    /// The item in a copied text, if the text starts with the item class line; `None` for
    /// any other text. The lines end with `\n` on every system.
    #[must_use]
    pub fn from_text(text: &str) -> Option<Self> {
        let text = text.trim();
        ITEM_CLASS_PREFIXES
            .iter()
            .any(|prefix| text.starts_with(prefix))
            .then(|| Self {
                text: text.replace("\r\n", "\n"),
            })
    }

    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
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
