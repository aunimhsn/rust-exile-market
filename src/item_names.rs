//! The names of the items in each language of the game client, from the trade site: the
//! items of the currency exchange by their id, the base types of the equipment, and the
//! uniques side by side in English and French; and the templates of its stats.
//!
//! The price database only knows the English names: these names find the price of an item
//! copied in French. The templates find the trade id of a modifier line the data of the game
//! doesn't know yet ([`crate::game_data::TradeStats`]).

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
    /// The templates of the stats of the modifier lines on the trade site.
    pub stats: Vec<StatTemplate>,
}

/// The template of a stat on the trade site, e.g. `explicit.stat_3299347043`, `# to maximum
/// Life`; the id of a stat with an option ends with it, e.g. `enchant.stat_2954116742|46224`,
/// `Allocates Arcane Alchemy`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatTemplate {
    pub id: String,
    pub text: String,
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
    /// Its name and base type in `language`.
    #[must_use]
    pub fn of(&self, language: Language) -> &UniqueName {
        match language {
            Language::English => &self.english,
            Language::French => &self.french,
        }
    }
}

/// The number of names of each kind, for the log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NameCounts {
    /// The items of the currency exchange, in English.
    pub exchange: usize,
    /// The base types of the equipment, in English.
    pub base_types: usize,
    /// The uniques in both languages.
    pub uniques: usize,
    /// The templates of the stats, in English.
    pub stats: usize,
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
    pub fn counts(&self) -> NameCounts {
        NameCounts {
            exchange: self.english.exchange.len(),
            base_types: self.english.base_types.len(),
            uniques: self.uniques.len(),
            stats: self.english.stats.len(),
        }
    }

    /// The templates of the stats of the trade site in `language`.
    #[cfg(test)]
    #[must_use]
    pub fn stat_templates(&self, language: Language) -> &[StatTemplate] {
        &self.names(language).stats
    }

    /// Takes the templates of the stats out of the names, in English and in French: once
    /// indexed ([`crate::game_data::TradeStats`]), they would be kept twice.
    pub fn take_stat_templates(&mut self) -> (Vec<StatTemplate>, Vec<StatTemplate>) {
        (
            std::mem::take(&mut self.english.stats),
            std::mem::take(&mut self.french.stats),
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

    /// The uniques whose base type is `base_type` in `language`, each name once: a unique
    /// can have a base in two versions (e.g. runemastered).
    #[must_use]
    pub fn uniques_of_base(&self, language: Language, base_type: &str) -> Vec<&UniqueNames> {
        let mut uniques: Vec<&UniqueNames> = Vec::new();
        for unique in &self.uniques {
            let seen = uniques
                .iter()
                .any(|other| other.english.name == unique.english.name);
            if unique.of(language).base_type == base_type && !seen {
                uniques.push(unique);
            }
        }
        uniques
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
    use crate::item::{CopiedItem, Item};

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
            stats: Vec::new(),
        };
        ItemNames::new(
            SystemTime::UNIX_EPOCH,
            names(
                &[
                    ("Divine Orb", "divine"),
                    ("Orb of Transmutation", "transmute"),
                    ("Uncut Skill Gem (Level 2)", "uncut-skill-gem-2"),
                ],
                &["Maul", "Temple Maul", "Ruby Ring", "Emerald"],
            ),
            names(
                &[
                    ("Orbe divin", "divine"),
                    ("Orbe de transmutation", "transmute"),
                    ("Gemme d'aptitude brute (Niveau 4)", "uncut-skill-gem-4"),
                ],
                &["Massue", "Massue du temple", "Bague de rubis", "Émeraude"],
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

        let names_of = |language, base_type| -> Vec<(String, String)> {
            names
                .uniques_of_base(language, base_type)
                .into_iter()
                .map(|unique| (unique.english.name.clone(), unique.french.name.clone()))
                .collect()
        };
        let pair = |english: &str, french: &str| (english.to_owned(), french.to_owned());

        assert_eq!(
            names_of(Language::French, "Émeraude"),
            [
                pair("Grand Spectrum", "Grand spectre"),
                pair("Prism Guardian", "Gardien prismatique")
            ]
        );
        assert_eq!(
            names_of(Language::English, "Makeshift Crossbow"),
            [pair("Mist Whisper", "Murmure des brumes")]
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
            names.magic_base_type(Language::French, "Massue du temple crépitante de la Brute"),
            Some("Massue du temple")
        );
        assert_eq!(
            names.magic_base_type(Language::English, "Mauling Ring of the Bear"),
            None,
            "a base type inside a longer word doesn't count"
        );
    }

    #[test]
    fn the_base_type_of_a_magic_tablet_or_waystone_is_found() {
        let names = |base_types: &[&str]| Names {
            exchange: HashMap::new(),
            base_types: base_types.iter().map(|&base| base.to_owned()).collect(),
            stats: Vec::new(),
        };
        let names = ItemNames::new(
            SystemTime::UNIX_EPOCH,
            names(&[
                "Irradiated Tablet",
                "Waystone (Tier 1)",
                "Waystone (Tier 15)",
            ]),
            names(&["Tablette irradiée", "Pierre de téléportation (Palier 15)"]),
            Vec::new(),
        );
        let base_type = |text: &str| {
            let item = Item::parse(&CopiedItem::from_text(text).expect("an item")).expect("read");
            names
                .magic_base_type(item.language(), item.base_type())
                .map(str::to_owned)
        };

        // The owner's tablets, and a waystone whose tier holds another one (`Tier 1`).
        assert_eq!(
            base_type(include_str!("item/test-items/en-owner-magic-tablet.txt")).as_deref(),
            Some("Irradiated Tablet")
        );
        assert_eq!(
            base_type(include_str!("item/test-items/fr-magic-tablet.txt")).as_deref(),
            Some("Tablette irradiée")
        );
        assert_eq!(
            base_type(
                "Item Class: Waystones\nRarity: Magic\nBurning Waystone (Tier 15)\n--------\n\
                 Item Level: 80"
            )
            .as_deref(),
            Some("Waystone (Tier 15)")
        );
        // Written by hand: the French affix after the base.
        assert_eq!(
            base_type(
                "Classe d'objet: Pierres de téléportation\nRareté: Magique\n\
                 Pierre de téléportation (Palier 15) incendiaire\n--------\n\
                 Niveau de l'objet: 80"
            )
            .as_deref(),
            Some("Pierre de téléportation (Palier 15)")
        );
    }

    #[test]
    fn the_base_type_of_a_copied_magic_item_is_found() {
        let text = include_str!("item/test-items/en-magic-mace.txt");
        let copied = CopiedItem::from_text(text).expect("an item");
        let item = Item::parse(&copied).expect("read");

        assert_eq!(
            item_names().magic_base_type(item.language(), item.base_type()),
            Some("Temple Maul")
        );
    }

    #[test]
    fn whole_words_are_found_at_both_ends_and_after_a_part_of_a_word() {
        assert!(holds_words("Temple Maul", "Temple Maul"), "the whole text");
        assert!(
            holds_words("Temple Maul of the Brute", "Temple Maul"),
            "at the start"
        );
        assert!(
            holds_words("Crackling Temple Maul", "Temple Maul"),
            "at the end"
        );
        // The first "Maul" is inside "Mauling", the second one is a word.
        assert!(holds_words("Mauling Maul", "Maul"));
        assert!(!holds_words("Mauling", "Maul"));
        assert!(!holds_words("Shammaul", "Maul"));
        assert!(!holds_words("Temple Maul", ""), "nothing isn't a base type");
    }
}
