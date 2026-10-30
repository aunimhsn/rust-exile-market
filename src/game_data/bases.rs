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

    /// The id of the category on the trade site (`weapon.bow`), as Exiled Exchange 2 maps it
    /// (`CATEGORY_TO_TRADE_ID`); a flask by the `tags` of its base, since the trade site tells
    /// the life flasks from the mana flasks. Checked on the trade site on 2026-10-28.
    fn trade_id(self, tags: &[String]) -> &'static str {
        let tagged = |tag: &str| tags.iter().any(|candidate| candidate == tag);
        match self {
            Self::Amulet => "accessory.amulet",
            Self::Belt => "accessory.belt",
            Self::Ring => "accessory.ring",
            Self::BodyArmour => "armour.chest",
            Self::Boots => "armour.boots",
            Self::Buckler => "armour.buckler",
            Self::Focus => "armour.focus",
            Self::Gloves => "armour.gloves",
            Self::Helmet => "armour.helmet",
            Self::Quiver => "armour.quiver",
            Self::Shield => "armour.shield",
            Self::Charm => "flask.charm",
            Self::Flask if tagged("life_flask") => "flask.life",
            Self::Flask if tagged("mana_flask") => "flask.mana",
            Self::Flask => "flask",
            Self::Jewel => "jewel",
            Self::Bow => "weapon.bow",
            Self::Crossbow => "weapon.crossbow",
            Self::Dagger => "weapon.dagger",
            Self::Flail => "weapon.flail",
            Self::OneHandAxe => "weapon.oneaxe",
            Self::OneHandMace => "weapon.onemace",
            Self::OneHandSword => "weapon.onesword",
            Self::Sceptre => "weapon.sceptre",
            Self::Spear => "weapon.spear",
            Self::Staff => "weapon.staff",
            Self::Talisman => "weapon.talisman",
            Self::TwoHandAxe => "weapon.twoaxe",
            Self::TwoHandMace => "weapon.twomace",
            Self::TwoHandSword => "weapon.twosword",
            Self::Wand => "weapon.wand",
            Self::Warstaff => "weapon.warstaff",
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
    #[serde(flatten)]
    spawn: BaseSpawn,
}

/// What tells the modifiers that roll on a base.
#[derive(Deserialize, Debug, PartialEq)]
pub(super) struct BaseSpawn {
    #[serde(default)]
    pub domain: Domain,
    /// E.g. `crossbow`, `two_hand_weapon`: only those that tell where a modifier rolls, also
    /// the kind of a flask (`life_flask`, `mana_flask`), its category on the trade site.
    #[serde(default)]
    pub tags: Vec<String>,
}

/// The domain of the modifiers of a base in RePoE, and of a modifier table: a table rolls on
/// the bases of its domain only. A domain the application doesn't know leaves its line of the
/// data unread, and the tests that read every line fail.
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub(super) enum Domain {
    /// The equipment, when the generator writes none.
    #[default]
    Item,
    /// The flasks and the charms.
    Flask,
    /// The jewels.
    Misc,
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

/// The id on the trade site of the category of the base type `name` in `language`, without
/// "Exceptional" nor "Superior" (`weapon.bow` for `Gemini Bow`); none for a base the data
/// doesn't know.
#[must_use]
pub fn trade_category(language: Language, name: &str) -> Option<&'static str> {
    base(language, name).map(|base| base.category.trade_id(&base.spawn.tags))
}

/// The domain and the tags of the base type `name` in `language` that tell where a modifier
/// rolls; none for a base the data doesn't know, or doesn't know the tags of.
pub(super) fn spawn(language: Language, name: &str) -> Option<&'static BaseSpawn> {
    base(language, name)
        .map(|base| &base.spawn)
        .filter(|spawn| !spawn.tags.is_empty())
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
    fn a_base_has_the_category_of_the_trade_site_and_a_flask_its_kind() {
        assert_eq!(
            trade_category(Language::English, "Gemini Bow"),
            Some("weapon.bow")
        );
        assert_eq!(
            trade_category(Language::French, "Targe de mammouth"),
            Some("armour.shield")
        );
        assert_eq!(
            trade_category(Language::English, "Aegis Quarterstaff"),
            Some("weapon.warstaff")
        );
        assert_eq!(
            trade_category(Language::English, "Ultimate Life Flask"),
            Some("flask.life")
        );
        assert_eq!(
            trade_category(Language::English, "Ultimate Mana Flask"),
            Some("flask.mana")
        );
        assert_eq!(trade_category(Language::English, "Mystery Ring"), None);
        // Every flask of the data is a life or a mana flask.
        for bases in [&ENGLISH, &FRENCH] {
            for base in bases.values() {
                assert_ne!(base.category.trade_id(&base.spawn.tags), "flask");
            }
        }
    }

    #[test]
    fn a_base_has_the_domain_of_its_modifiers() {
        let domain = |language, name| spawn(language, name).map(|spawn| spawn.domain);

        assert_eq!(domain(Language::English, "Gemini Bow"), Some(Domain::Item));
        assert_eq!(
            domain(Language::English, "Ultimate Life Flask"),
            Some(Domain::Flask)
        );
        assert_eq!(
            domain(Language::French, "Charme en améthyste"),
            Some(Domain::Flask)
        );
        assert_eq!(domain(Language::English, "Ruby"), Some(Domain::Misc));
        assert_eq!(domain(Language::English, "Mystery Ring"), None);
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
