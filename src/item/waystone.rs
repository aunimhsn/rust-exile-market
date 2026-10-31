//! A waystone: its tier, written in its base type (`Waystone (Tier 12)`), and its properties,
//! the bonuses of the map it opens (`Item Rarity: +11% (augmented)`).

use super::Item;
use super::labels::Labels;
use crate::language::Language;

/// The highest tier of a waystone: a corrupted waystone of tier 15 raised one tier.
pub const MAX_WAYSTONE_TIER: u32 = 16;

/// A property of a waystone, a bonus of the map it opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WaystoneProperty {
    /// The deaths the player can afford in the map.
    Revives,
    ItemRarity,
    PackSize,
    MonsterRarity,
    MonsterEffectiveness,
    /// The chance that the monsters drop a waystone.
    DropChance,
}

impl WaystoneProperty {
    /// Written in percent with its sign (`+11%`); the revives are a number of their own.
    #[must_use]
    pub fn is_percent(self) -> bool {
        self != Self::Revives
    }
}

/// A property of a waystone and its value: in percent, but for the revives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaystoneValue {
    pub property: WaystoneProperty,
    pub value: i32,
}

impl Item {
    /// A waystone, of any rarity.
    #[must_use]
    pub fn is_waystone(&self) -> bool {
        self.class == Labels::of(self.language).waystones
    }

    /// The properties of a waystone, in the order of the item; none for another item, or for a
    /// value that can't be read.
    #[must_use]
    pub fn waystone_properties(&self) -> Vec<WaystoneValue> {
        if !self.is_waystone() {
            return Vec::new();
        }
        let labels = Labels::of(self.language);
        self.properties
            .iter()
            .filter_map(|written| {
                let &(_, property) = labels
                    .waystone_properties
                    .iter()
                    .find(|&&(name, _)| name == written.name)?;
                Some(WaystoneValue {
                    property,
                    value: signed_number(&written.value)?,
                })
            })
            .collect()
    }
}

/// `text` without the tier of a waystone, in `language`, and that tier: `Waystone (Tier 12)` is
/// `Waystone` of tier 12; the name of a magic waystone, `Burning Waystone (Tier 15)`, is
/// `Burning Waystone`, and in French, where the tier comes before the affix, `Pierre de
/// téléportation (Palier 15) incendiaire` is `Pierre de téléportation incendiaire`.
#[must_use]
pub fn without_tier(language: Language, text: &str) -> Option<(String, u32)> {
    let (before, rest) = text.split_once(Labels::of(language).waystone_tier)?;
    let (tier, after) = rest.split_once(')')?;
    let tier = tier.parse().ok().filter(|&tier| tier > 0)?;
    let name = [before.trim_end(), after.trim_start()]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    (!name.is_empty()).then_some((name, tier))
}

/// The base type of a waystone, in `language`, split into its name and its tier: `Waystone
/// (Tier 12)` is `Waystone` of tier 12.
#[must_use]
pub fn base_and_tier(language: Language, base_type: &str) -> Option<(&str, u32)> {
    let (name, rest) = base_type.split_once(Labels::of(language).waystone_tier)?;
    let tier = rest
        .strip_suffix(')')?
        .parse()
        .ok()
        .filter(|&tier| tier > 0)?;
    let name = name.trim_end();
    (!name.is_empty()).then_some((name, tier))
}

/// A whole number, with or without its sign and its percent: `3`, `+11%`, `-5%`.
fn signed_number(value: &str) -> Option<i32> {
    let value = value.strip_suffix('%').unwrap_or(value);
    value.strip_prefix('+').unwrap_or(value).parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn properties(text: &str) -> Vec<(WaystoneProperty, i32)> {
        let item =
            Item::parse(&crate::item::CopiedItem::from_text(text).expect("an item")).expect("read");
        item.waystone_properties()
            .into_iter()
            .map(|found| (found.property, found.value))
            .collect()
    }

    #[test]
    fn the_properties_of_the_owners_waystones_are_read_in_both_languages() {
        use WaystoneProperty::{
            DropChance, ItemRarity, MonsterEffectiveness, MonsterRarity, PackSize, Revives,
        };
        assert_eq!(
            properties(include_str!("test-items/en-owner-rare-waystone.txt")),
            [
                (Revives, 3),
                (ItemRarity, 11),
                (MonsterRarity, 25),
                (MonsterEffectiveness, 13),
                (DropChance, 50),
            ]
        );
        assert_eq!(
            properties(include_str!("test-items/fr-rare-waystone.txt")),
            [
                (Revives, 2),
                (ItemRarity, 25),
                (PackSize, 9),
                (MonsterRarity, 23),
                (DropChance, 65),
            ]
        );
        // `Efficacité des Monstres`, from Exiled Exchange 2.
        assert_eq!(
            properties(
                "Classe d'objet: Pierres de téléportation\nRareté: Normal\n\
                 Pierre de téléportation (Palier 3)\n--------\n\
                 Efficacité des Monstres: +5% (augmented)\n--------\nNiveau de l'objet: 66"
            ),
            [(MonsterEffectiveness, 5)]
        );
        // Another item has none, whatever its properties are named.
        assert_eq!(
            properties(
                "Item Class: Rings\nRarity: Normal\nGold Ring\n--------\n\
                 Item Rarity: +11%\n--------\nItem Level: 80"
            ),
            []
        );
    }

    #[test]
    fn the_tier_of_a_waystone_is_read_from_its_base_or_its_magic_name() {
        let english = |text| without_tier(Language::English, text);
        let french = |text| without_tier(Language::French, text);
        assert_eq!(
            english("Waystone (Tier 12)"),
            Some(("Waystone".to_owned(), 12))
        );
        assert_eq!(
            english("Burning Waystone (Tier 15)"),
            Some(("Burning Waystone".to_owned(), 15))
        );
        assert_eq!(
            french("Pierre de téléportation (Palier 15) incendiaire"),
            Some(("Pierre de téléportation incendiaire".to_owned(), 15))
        );
        assert_eq!(
            french("Pierre de téléportation (Palier 16)"),
            Some(("Pierre de téléportation".to_owned(), 16))
        );
        // Not a tier: another word, no number, nothing around it, another language.
        assert_eq!(english("Irradiated Tablet"), None);
        assert_eq!(english("Waystone (Tier X)"), None);
        assert_eq!(english("Waystone (Tier 0)"), None);
        assert_eq!(english("(Tier 3)"), None);
        assert_eq!(english("Pierre de téléportation (Palier 12)"), None);
    }

    #[test]
    fn a_base_type_of_waystone_is_split_into_its_name_and_its_tier() {
        assert_eq!(
            base_and_tier(Language::English, "Waystone (Tier 12)"),
            Some(("Waystone", 12))
        );
        assert_eq!(
            base_and_tier(Language::French, "Pierre de téléportation (Palier 16)"),
            Some(("Pierre de téléportation", 16))
        );
        // The tier at the end only, as in a base type.
        assert_eq!(
            base_and_tier(Language::English, "Burning Waystone (Tier 15) of Fire"),
            None
        );
        assert_eq!(base_and_tier(Language::English, "Waystone"), None);
        assert_eq!(base_and_tier(Language::English, "(Tier 3)"), None);
    }

    #[test]
    fn a_property_is_read_with_or_without_its_sign_and_percent() {
        assert_eq!(signed_number("3"), Some(3));
        assert_eq!(signed_number("+11%"), Some(11));
        assert_eq!(signed_number("-5%"), Some(-5));
        assert_eq!(signed_number("+1.5%"), None);
        assert_eq!(signed_number(""), None);
    }
}
