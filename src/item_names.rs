//! The names of the items in each language of the game client, from the trade site: the
//! items of the currency exchange by their id, the base types of the equipment, and the
//! uniques side by side in English and French.
//!
//! The price database only knows the English names: these names find the price of an item
//! copied in French.

use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

use crate::language::Language;

/// The names of one language.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Names {
    /// The id of each item of the currency exchange on the trade site, by its name, e.g.
    /// "Orbe divin" → `divine`; the ids are the same in every language.
    pub exchange: HashMap<String, String>,
    /// The base types of the equipment, to find in the name of a magic item.
    pub base_types: Vec<String>,
}

/// A unique item: its name and its base type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UniqueName {
    pub name: String,
    pub base_type: String,
}

/// A unique item in English and in French.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UniqueNames {
    pub english: UniqueName,
    pub french: UniqueName,
}

impl UniqueNames {
    fn of(&self, language: Language) -> &UniqueName {
        match language {
            Language::English => &self.english,
            Language::French => &self.french,
        }
    }
}

/// The names of the items, as the trade site gave them at one time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemNames {
    pub updated: SystemTime,
    english: Names,
    french: Names,
    uniques: Vec<UniqueNames>,
}

impl ItemNames {
    /// The names given by the trade site at `updated`.
    #[must_use]
    pub fn new(
        updated: SystemTime,
        english: Names,
        french: Names,
        uniques: Vec<UniqueNames>,
    ) -> Self {
        Self {
            updated,
            english,
            french,
            uniques,
        }
    }

    /// How long ago the trade site gave them; none for names dated in the future.
    #[must_use]
    pub fn age(&self, now: SystemTime) -> Duration {
        now.duration_since(self.updated).unwrap_or_default()
    }

    /// The number of names of each kind, for the log.
    #[must_use]
    pub fn counts(&self) -> (usize, usize, usize) {
        (
            self.english.exchange.len(),
            self.english.base_types.len(),
            self.uniques.len(),
        )
    }

    fn names(&self, language: Language) -> &Names {
        match language {
            Language::English => &self.english,
            Language::French => &self.french,
        }
    }

    /// The id on the currency exchange of the item named `name` in `language`.
    #[must_use]
    pub fn exchange_id(&self, language: Language, name: &str) -> Option<&str> {
        self.names(language).exchange.get(name).map(String::as_str)
    }

    /// The English name of the unique named `name` in `language`.
    #[must_use]
    pub fn english_unique(&self, language: Language, name: &str) -> Option<&str> {
        self.uniques
            .iter()
            .find(|unique| unique.of(language).name == name)
            .map(|unique| unique.english.name.as_str())
    }

    /// The English names of the uniques whose base type is `base_type` in `language`, each
    /// once.
    #[must_use]
    pub fn english_uniques_of_base(&self, language: Language, base_type: &str) -> Vec<&str> {
        let mut names: Vec<&str> = Vec::new();
        for unique in &self.uniques {
            let name = unique.english.name.as_str();
            if unique.of(language).base_type == base_type && !names.contains(&name) {
                names.push(name);
            }
        }
        names
    }

    /// The base type of the magic item named `name` in `language`: the longest base type
    /// its name holds as whole words, e.g. "Temple Maul" in "Crackling Temple Maul of the
    /// Brute".
    #[must_use]
    pub fn magic_base_type(&self, language: Language, name: &str) -> Option<&str> {
        self.names(language)
            .base_types
            .iter()
            .filter(|base_type| holds_words(name, base_type))
            .max_by_key(|base_type| base_type.len())
            .map(String::as_str)
    }
}

/// `text` holds `words` between spaces or at its ends, not inside a longer word.
fn holds_words(text: &str, words: &str) -> bool {
    !words.is_empty()
        && text.match_indices(words).any(|(start, _)| {
            let before = text[..start].chars().next_back();
            let after = text[start + words.len()..].chars().next();
            before.is_none_or(|character| character == ' ')
                && after.is_none_or(|character| character == ' ')
        })
}

#[cfg(test)]
pub mod tests {
    use super::*;

    fn unique(name: &str, base_type: &str) -> UniqueName {
        UniqueName {
            name: name.to_owned(),
            base_type: base_type.to_owned(),
        }
    }

    /// Names of a few items, as the trade site gives them on 2026-10-25.
    pub fn item_names() -> ItemNames {
        let names = |exchange: &[(&str, &str)], base_types: &[&str]| Names {
            exchange: exchange
                .iter()
                .map(|&(name, id)| (name.to_owned(), id.to_owned()))
                .collect(),
            base_types: base_types.iter().map(|&base| base.to_owned()).collect(),
        };
        ItemNames::new(
            SystemTime::UNIX_EPOCH,
            names(
                &[
                    ("Divine Orb", "divine"),
                    ("Orb of Transmutation", "transmute"),
                ],
                &["Maul", "Temple Maul", "Ruby Ring", "Emerald"],
            ),
            names(
                &[
                    ("Orbe divin", "divine"),
                    ("Orbe de transmutation", "transmute"),
                ],
                &["Maillet", "Maillet du temple", "Bague de rubis", "Émeraude"],
            ),
            vec![
                UniqueNames {
                    english: unique("Grand Spectrum", "Emerald"),
                    french: unique("Grand spectre", "Émeraude"),
                },
                UniqueNames {
                    english: unique("Prism Guardian", "Emerald"),
                    french: unique("Gardien prismatique", "Émeraude"),
                },
                UniqueNames {
                    english: unique("Mist Whisper", "Makeshift Crossbow"),
                    french: unique("Murmure des brumes", "Arbalète de fortune"),
                },
            ],
        )
    }

    #[test]
    fn an_item_of_the_currency_exchange_has_the_same_id_in_every_language() {
        let names = item_names();

        assert_eq!(
            names.exchange_id(Language::French, "Orbe divin"),
            Some("divine")
        );
        assert_eq!(
            names.exchange_id(Language::English, "Divine Orb"),
            Some("divine")
        );
        assert_eq!(names.exchange_id(Language::French, "Divine Orb"), None);
    }

    #[test]
    fn a_french_unique_has_its_english_name() {
        let names = item_names();

        assert_eq!(
            names.english_unique(Language::French, "Murmure des brumes"),
            Some("Mist Whisper")
        );
        assert_eq!(names.english_unique(Language::French, "Inconnu"), None);
    }

    #[test]
    fn the_uniques_of_a_base_type_are_found_in_its_language() {
        let names = item_names();

        assert_eq!(
            names.english_uniques_of_base(Language::French, "Émeraude"),
            ["Grand Spectrum", "Prism Guardian"]
        );
        assert_eq!(
            names.english_uniques_of_base(Language::English, "Makeshift Crossbow"),
            ["Mist Whisper"]
        );
    }

    #[test]
    fn the_base_type_of_a_magic_item_is_the_longest_one_in_its_name() {
        let names = item_names();

        assert_eq!(
            names.magic_base_type(Language::English, "Crackling Temple Maul of the Brute"),
            Some("Temple Maul")
        );
        assert_eq!(
            names.magic_base_type(Language::French, "Maillet du temple crépitant de la Brute"),
            Some("Maillet du temple")
        );
        assert_eq!(
            names.magic_base_type(Language::English, "Mauling Ring of the Bear"),
            None,
            "a base type inside a longer word doesn't count"
        );
    }
}
