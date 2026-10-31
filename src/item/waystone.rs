//! A waystone: its tier, written in its base type (`Waystone (Tier 12)`), and its properties,
//! the bonuses of the map it opens (`Item Rarity: +11% (augmented)`).

use super::Item;
use super::labels::Labels;
use super::parser::{Field, ParseWarning};
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
            .filter_map(|written| read_property(labels, &written.name, &written.value).ok())
            .collect()
    }
}

/// The property of a waystone named `name`, of the value `value`, in the words of `labels`;
/// the warning of a name not in the labels (a property added or renamed by a patch), or of a
/// value that can't be read.
pub(super) fn read_property(
    labels: &Labels,
    name: &str,
    value: &str,
) -> Result<WaystoneValue, ParseWarning> {
    let &(_, property) = labels
        .waystone_properties
        .iter()
        .find(|&&(label, _)| label == name)
        .ok_or(ParseWarning::UnknownWaystoneProperty)?;
    let value = signed_number(value).ok_or(ParseWarning::InvalidValue(Field::WaystoneProperty))?;
    Ok(WaystoneValue { property, value })
}

/// `text` without the tier of a waystone, in `language`, and that tier: `Waystone (Tier 12)` is
/// `Waystone` of tier 12; the name of a magic waystone, `Burning Waystone (Tier 15)`, is
/// `Burning Waystone`, and in French, where the tier comes before the affix, `Pierre de
/// téléportation (Palier 15) incendiaire` is `Pierre de téléportation incendiaire`.
#[must_use]
pub fn without_tier(language: Language, text: &str) -> Option<(String, u32)> {
    let (before, tier, after) = split_tier(language, text)?;
    let name = match (before, after) {
        ("", "") => return None,
        (name, "") | ("", name) => name.to_owned(),
        (before, after) => format!("{before} {after}"),
    };
    Some((name, tier))
}

/// The base type of a waystone, in `language`, split into its name and its tier: `Waystone
/// (Tier 12)` is `Waystone` of tier 12; none when anything follows the tier.
#[must_use]
pub fn base_and_tier(language: Language, base_type: &str) -> Option<(&str, u32)> {
    match split_tier(language, base_type)? {
        (name, tier, "") if !name.is_empty() => Some((name, tier)),
        _ => None,
    }
}

/// `text` around the tier of a waystone, in `language`: the words before it, the tier, the
/// words after it, without the spaces next to the tier.
fn split_tier(language: Language, text: &str) -> Option<(&str, u32, &str)> {
    let (before, rest) = text.split_once(Labels::of(language).waystone_tier)?;
    let (tier, after) = rest.split_once(')')?;
    let tier = tier.parse().ok().filter(|&tier| tier > 0)?;
    Some((before.trim_end(), tier, after.trim_start()))
}

/// A whole number, with or without its sign and its percent: `3`, `+11%`, `-5%`.
fn signed_number(value: &str) -> Option<i32> {
    value.strip_suffix('%').unwrap_or(value).parse().ok()
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
    fn a_property_of_a_waystone_that_cant_be_read_is_logged() {
        let warnings = |line: &str| {
            let text = format!(
                "Item Class: Waystones\nRarity: Normal\nWaystone (Tier 3)\n--------\n\
                 {line}\n--------\nItem Level: 66"
            );
            let item = Item::parse(&crate::item::CopiedItem::from_text(&text).expect("an item"))
                .expect("read");
            (item.waystone_properties().len(), item.warnings().to_vec())
        };

        // A name not in the labels, maybe renamed by a patch, so that it can be added.
        assert_eq!(
            warnings("Monster Quantity: +5% (augmented)"),
            (0, vec![ParseWarning::UnknownWaystoneProperty])
        );
        assert_eq!(
            warnings("Item Rarity: +1.5% (augmented)"),
            (0, vec![ParseWarning::InvalidValue(Field::WaystoneProperty)])
        );
        // A negative value is read: it isn't a filter.
        assert_eq!(warnings("Item Rarity: -5% (augmented)"), (1, vec![]));
        // The owner's waystones, read without warning.
        for text in [
            include_str!("test-items/en-owner-rare-waystone.txt"),
            include_str!("test-items/fr-rare-waystone.txt"),
        ] {
            let item = Item::parse(&crate::item::CopiedItem::from_text(text).expect("an item"))
                .expect("read");
            assert_eq!(item.warnings(), [], "{text}");
        }
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
        // An English magic waystone with a prefix and a suffix, the suffix after the tier.
        assert_eq!(
            english("Burning Waystone (Tier 15) of Ruin"),
            Some(("Burning Waystone of Ruin".to_owned(), 15))
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
        assert_eq!(signed_number("++11%"), None);
        assert_eq!(signed_number(""), None);
    }
}
