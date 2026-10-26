//! The words of the copied items in each language of the game client.
//!
//! The parser only knows the structure of a copied item; every word it looks for comes from
//! here. The markers at the end of the lines (`(augmented)`, `(rune)`, `(implicit)`…) are in
//! English in every language.

use super::{Affix, ItemState, ModifierKind, ModifierOrigin, Rarity};
use crate::language::Language;

/// The words of one language. The labels of the `Label: value` lines end with their
/// separator, as the game writes it (`"Requires: "`, `"Prérequis : "`).
pub(super) struct Labels {
    pub item_class: &'static str,
    /// The item class of the jewels, which have two prefixes and two suffixes at most.
    pub jewels: &'static str,
    pub rarity: &'static str,
    pub rarities: &'static [(&'static str, Rarity)],
    pub item_level: &'static str,
    pub stack_size: &'static str,
    pub quality: &'static str,
    pub sockets: &'static str,
    pub requires: &'static str,
    pub level: &'static str,
    /// The short and long names of the attributes in the requirements.
    pub strength: &'static [&'static str],
    pub dexterity: &'static [&'static str],
    pub intelligence: &'static [&'static str],
    pub grants_skill: &'static str,
    /// The names of the properties of a weapon read for its DPS; the elemental and chaos
    /// damage are told by their markers, the same in every language.
    pub physical_damage: &'static str,
    pub attacks_per_second: &'static str,
    /// The names of the defences of an armour piece or a shield.
    pub armour: &'static str,
    pub evasion: &'static str,
    pub energy_shield: &'static str,
    pub block: &'static str,
    /// The note of a priced item; unknown in some languages.
    pub note: Option<&'static str>,
    /// The state lines at the end of the item; "Unidentified" may be followed by a tier.
    pub states: &'static [(&'static str, ItemState)],
    /// The words around the base type of an exceptional or superior item, in each gender.
    pub exceptional: &'static [Wording],
    pub superior: &'static [Wording],
    pub tier: &'static str,
    /// The marker of the advanced copy after a line whose value never changes, e.g.
    /// ` — Unscalable Value`.
    pub fixed_value: &'static str,
    /// The kinds of the modifier headers. A label never ends with an earlier one, so that
    /// "Corruption Enhancement" is not taken for "Enhancement" (checked by the tests).
    pub modifier_kinds: &'static [(&'static str, ModifierKind)],
    /// The words before the kind in a modifier header.
    pub modifier_origins: &'static [(&'static str, ModifierOrigin)],
}

/// A word written before or after a name, e.g. "Superior" in "Superior Divine Crown".
#[derive(Debug, Clone, Copy)]
pub(super) enum Wording {
    Before(&'static str),
    After(&'static str),
}

impl Wording {
    /// The name without the word, if it has it.
    pub fn strip(self, name: &str) -> Option<&str> {
        match self {
            Self::Before(word) => name.strip_prefix(word),
            Self::After(word) => name.strip_suffix(word),
        }
    }
}

impl Labels {
    pub fn of(language: Language) -> &'static Self {
        match language {
            Language::English => &ENGLISH,
            Language::French => &FRENCH,
        }
    }
}

/// The markers at the end of a property or a requirement, in English in every language; the
/// markers of the modifiers are [`super::modifier::Marker`].
pub(super) mod marker {
    pub const AUGMENTED: &str = " (augmented)";
    pub const UNMET: &str = " (unmet)";
}

const PREFIX: ModifierKind = ModifierKind::Explicit(Some(Affix::Prefix));
const SUFFIX: ModifierKind = ModifierKind::Explicit(Some(Affix::Suffix));
const UNIQUE: ModifierKind = ModifierKind::Explicit(None);

static ENGLISH: Labels = Labels {
    item_class: "Item Class: ",
    jewels: "Jewels",
    rarity: "Rarity: ",
    rarities: &[
        ("Normal", Rarity::Normal),
        ("Magic", Rarity::Magic),
        ("Rare", Rarity::Rare),
        ("Unique", Rarity::Unique),
        ("Currency", Rarity::Currency),
        ("Gem", Rarity::Gem),
    ],
    item_level: "Item Level: ",
    stack_size: "Stack Size: ",
    quality: "Quality: ",
    sockets: "Sockets: ",
    requires: "Requires: ",
    level: "Level ",
    strength: &["Str", "Strength"],
    dexterity: &["Dex", "Dexterity"],
    intelligence: &["Int", "Intelligence"],
    grants_skill: "Grants Skill: ",
    physical_damage: "Physical Damage",
    attacks_per_second: "Attacks per Second",
    armour: "Armour",
    evasion: "Evasion Rating",
    energy_shield: "Energy Shield",
    // To check on a copied shield.
    block: "Block chance",
    note: Some("Note: "),
    states: &[
        ("Corrupted", ItemState::Corrupted),
        ("Twice Corrupted", ItemState::TwiceCorrupted),
        ("Sanctified", ItemState::Sanctified),
        ("Fractured Item", ItemState::Fractured),
        ("Mirrored", ItemState::Mirrored),
        ("Unidentified", ItemState::Unidentified),
    ],
    exceptional: &[Wording::Before("Exceptional ")],
    superior: &[Wording::Before("Superior ")],
    tier: "Tier: ",
    // From the French marker: to check on an item copied in English.
    fixed_value: " — Unscalable Value",
    modifier_kinds: &[
        ("Prefix Modifier", PREFIX),
        ("Suffix Modifier", SUFFIX),
        ("Vaal Unique Modifier", UNIQUE),
        ("Unique Modifier", UNIQUE),
        ("Implicit Modifier", ModifierKind::Implicit),
        ("Corruption Enhancement", ModifierKind::Corruption),
        ("Enhancement", ModifierKind::Enchant),
    ],
    modifier_origins: &[
        ("Crafted", ModifierOrigin::Crafted),
        ("Desecrated", ModifierOrigin::Desecrated),
        ("Fractured", ModifierOrigin::Fractured),
    ],
};

static FRENCH: Labels = Labels {
    item_class: "Classe d'objet: ",
    jewels: "Joyaux",
    rarity: "Rareté: ",
    rarities: &[
        ("Normal", Rarity::Normal),
        ("Magique", Rarity::Magic),
        ("Rare", Rarity::Rare),
        ("Unique", Rarity::Unique),
        ("Objet monétaire", Rarity::Currency),
        ("Gemme", Rarity::Gem),
    ],
    item_level: "Niveau de l'objet: ",
    stack_size: "Taille de l'ensemble: ",
    quality: "Qualité: ",
    sockets: "Châsses: ",
    requires: "Prérequis : ",
    level: "Niveau ",
    strength: &["For", "Force"],
    dexterity: &["Dex", "Dextérité"],
    intelligence: &["Int", "Intelligence"],
    grants_skill: "Octroie l'Aptitude: ",
    physical_damage: "Dégâts Physiques",
    attacks_per_second: "Attaques par seconde",
    armour: "Armure",
    evasion: "Score d'Évasion",
    energy_shield: "Bouclier d'énergie",
    block: "Chances de Blocage",
    note: None,
    states: &[
        ("Corrompu", ItemState::Corrupted),
        ("Double Corruption", ItemState::TwiceCorrupted),
        ("Sanctifié", ItemState::Sanctified),
        ("Objet fissuré", ItemState::Fractured),
        ("Reflété", ItemState::Mirrored),
        ("Non identifié", ItemState::Unidentified),
    ],
    exceptional: &[
        Wording::Before("Exceptionnel "),
        Wording::Before("Exceptionnelle "),
    ],
    // From the data of Exiled Exchange 2: to check on a copied item.
    superior: &[Wording::After(" supérieur"), Wording::After(" supérieure")],
    tier: "Palier : ",
    fixed_value: " — Valeur fixe",
    modifier_kinds: &[
        ("Préfixe", PREFIX),
        ("Suffixe", SUFFIX),
        ("Modificateur vaal unique", UNIQUE),
        ("Modificateur d'Objet unique", UNIQUE),
        ("Modificateur Implicite", ModifierKind::Implicit),
        ("Amélioration de Corruption", ModifierKind::Corruption),
        ("Amélioration", ModifierKind::Enchant),
    ],
    modifier_origins: &[
        ("Artisanat", ModifierOrigin::Crafted),
        ("Profané", ModifierOrigin::Desecrated),
        ("Fissuré", ModifierOrigin::Fractured),
    ],
};

#[cfg(test)]
mod tests {
    use super::*;

    /// Every variant, with a `match` that fails to compile when one is added.
    fn every_rarity() -> [Rarity; 6] {
        use Rarity::{Currency, Gem, Magic, Normal, Rare, Unique};
        let all = [Normal, Magic, Rare, Unique, Currency, Gem];
        for rarity in all {
            match rarity {
                Normal | Magic | Rare | Unique | Currency | Gem => {}
            }
        }
        all
    }

    fn every_state() -> [ItemState; 6] {
        use ItemState::{Corrupted, Fractured, Mirrored, Sanctified, TwiceCorrupted, Unidentified};
        let all = [
            Corrupted,
            TwiceCorrupted,
            Sanctified,
            Fractured,
            Mirrored,
            Unidentified,
        ];
        for state in all {
            match state {
                Corrupted | TwiceCorrupted | Sanctified | Fractured | Mirrored | Unidentified => {}
            }
        }
        all
    }

    fn every_origin() -> [ModifierOrigin; 3] {
        use ModifierOrigin::{Crafted, Desecrated, Fractured};
        let all = [Crafted, Desecrated, Fractured];
        for origin in all {
            match origin {
                Crafted | Desecrated | Fractured => {}
            }
        }
        all
    }

    #[test]
    fn every_language_names_every_rarity_state_and_origin() {
        for language in Language::ALL {
            let labels = Labels::of(language);
            for rarity in every_rarity() {
                assert!(
                    labels.rarities.iter().any(|&(_, r)| r == rarity),
                    "{language:?} {rarity:?}"
                );
            }
            for state in every_state() {
                assert!(
                    labels.states.iter().any(|&(_, s)| s == state),
                    "{language:?} {state:?}"
                );
            }
            for origin in every_origin() {
                assert!(
                    labels.modifier_origins.iter().any(|&(_, o)| o == origin),
                    "{language:?} {origin:?}"
                );
            }
        }
    }

    #[test]
    fn no_modifier_kind_is_hidden_by_an_earlier_one() {
        for language in Language::ALL {
            let kinds = Labels::of(language).modifier_kinds;
            for (index, (earlier, _)) in kinds.iter().enumerate() {
                for (later, _) in kinds.iter().skip(index + 1) {
                    assert!(
                        !later.ends_with(earlier),
                        "{language:?}: {later} after {earlier}"
                    );
                }
            }
        }
    }
}
