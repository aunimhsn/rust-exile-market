//! The bases of the game, from Exiled Exchange 2 (`assets/game-data/bases-<language>.ndjson`):
//! the category of each base of equipment, jewel, flask or charm, by its name in each language,
//! and its tags from RePoE, which tell the modifiers that roll on it.

use std::collections::HashMap;
use std::sync::LazyLock;

use serde::Deserialize;

use crate::language::Language;

/// The category of a base, the same in every language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Category {
    Amulet,
    Belt,
    #[serde(rename = "Body Armour")]
    BodyArmour,
    Boots,
    Bow,
    Buckler,
    Charm,
    Crossbow,
    Dagger,
    Flail,
    Flask,
    Focus,
    Gloves,
    Helmet,
    Jewel,
    #[serde(rename = "One Hand Axe")]
    OneHandAxe,
    #[serde(rename = "One Hand Mace")]
    OneHandMace,
    #[serde(rename = "One Hand Sword")]
    OneHandSword,
    Quiver,
    Ring,
    Sceptre,
    Shield,
    Spear,
    Staff,
    Talisman,
    #[serde(rename = "Two Hand Axe")]
    TwoHandAxe,
    #[serde(rename = "Two Hand Mace")]
    TwoHandMace,
    #[serde(rename = "Two Hand Sword")]
    TwoHandSword,
    Wand,
    /// The quarterstaves.
    Warstaff,
}

impl Category {
    /// The augmentable sockets a base of the category has at most, as Exiled Exchange 2 counts
    /// them (`getMaxSockets`): an exceptional base, or a corruption, can give it more.
    #[must_use]
    pub fn usual_sockets(self) -> u32 {
        match self {
            Self::BodyArmour
            | Self::TwoHandAxe
            | Self::TwoHandMace
            | Self::TwoHandSword
            | Self::Crossbow
            | Self::Bow
            | Self::Warstaff
            | Self::Staff
            | Self::Talisman => 2,
            Self::Helmet
            | Self::Shield
            | Self::Gloves
            | Self::Boots
            | Self::OneHandAxe
            | Self::OneHandMace
            | Self::OneHandSword
            | Self::Dagger
            | Self::Focus
            | Self::Spear
            | Self::Flail
            | Self::Wand
            | Self::Buckler
            | Self::Sceptre => 1,
            Self::Amulet
            | Self::Belt
            | Self::Charm
            | Self::Flask
            | Self::Jewel
            | Self::Quiver
            | Self::Ring => 0,
        }
    }
}

/// A base, as the generator writes it.
#[derive(Deserialize)]
struct Base {
    name: String,
    #[serde(flatten)]
    data: BaseData,
}

#[derive(Deserialize)]
struct BaseData {
    category: Category,
    /// E.g. `crossbow`, `two_hand_weapon`: only those that tell where a modifier rolls.
    #[serde(default)]
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "read by the roll bar of step 13j2")
    )]
    tags: Vec<String>,
}

static ENGLISH: LazyLock<HashMap<String, BaseData>> =
    LazyLock::new(|| read(include_str!("../../assets/game-data/bases-en.ndjson")));
static FRENCH: LazyLock<HashMap<String, BaseData>> =
    LazyLock::new(|| read(include_str!("../../assets/game-data/bases-fr.ndjson")));

/// The base type `name` in `language`, without "Exceptional" nor "Superior".
fn base(language: Language, name: &str) -> Option<&'static BaseData> {
    let bases = match language {
        Language::English => &ENGLISH,
        Language::French => &FRENCH,
    };
    bases.get(name)
}

/// The category of the base type `name` in `language` (`Mammoth Targe`, `Targe de mammouth`),
/// without "Exceptional" nor "Superior"; none for a base the data doesn't know.
#[must_use]
pub fn category(language: Language, name: &str) -> Option<Category> {
    base(language, name).map(|base| base.category)
}

/// The tags of the base type `name` in `language` that tell where a modifier rolls; none for a
/// base the data doesn't know, or doesn't know the tags of.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "read by the roll bar of step 13j2")
)]
pub(super) fn tags(language: Language, name: &str) -> Option<&'static [String]> {
    base(language, name)
        .map(|base| base.tags.as_slice())
        .filter(|tags| !tags.is_empty())
}

/// Reads one base per line; a line that can't be read is logged and left out (the tests read
/// every line of the embedded files).
fn read(text: &str) -> HashMap<String, BaseData> {
    let mut bases = HashMap::new();
    for (index, line) in text.lines().enumerate() {
        match serde_json::from_str::<Base>(line) {
            Ok(base) => {
                bases.insert(base.name, base.data);
            }
            Err(error) => log::error!("Game data, base {} unreadable: {error}", index + 1),
        }
    }
    bases
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::{CopiedItem, Item};

    #[test]
    fn every_base_of_the_embedded_files_is_read() {
        for (text, read) in [
            (
                include_str!("../../assets/game-data/bases-en.ndjson"),
                &ENGLISH,
            ),
            (
                include_str!("../../assets/game-data/bases-fr.ndjson"),
                &FRENCH,
            ),
        ] {
            // The generator writes each name once.
            assert_eq!(read.len(), text.lines().count());
        }
    }

    #[test]
    fn the_base_of_a_copied_item_is_found_in_its_language() {
        let category_of = |text: &str| {
            let item = Item::parse(&CopiedItem::from_text(text).expect("an item")).expect("read");
            category(item.language(), item.base_type())
        };

        assert_eq!(
            category_of(include_str!(
                "../item/test-items/fr-normal-exceptional-shield.txt"
            )),
            Some(Category::Shield)
        );
        assert_eq!(
            category_of(include_str!("../item/test-items/en-normal-helmet.txt")),
            Some(Category::Helmet)
        );
        assert_eq!(
            category_of(include_str!("../item/test-items/fr-rare-body-armour.txt")),
            Some(Category::BodyArmour)
        );
        assert_eq!(
            category_of(include_str!(
                "../item/test-items/en-rare-crossbow-all-modifiers.txt"
            )),
            Some(Category::Crossbow)
        );
        assert_eq!(
            category(Language::English, "Exceptional Mammoth Targe"),
            None
        );
    }

    #[test]
    fn a_base_has_the_usual_sockets_of_its_category() {
        assert_eq!(
            category(Language::English, "Waxed Jacket").map(Category::usual_sockets),
            Some(2)
        );
        assert_eq!(Category::Shield.usual_sockets(), 1);
        assert_eq!(Category::Ring.usual_sockets(), 0);
    }
}
