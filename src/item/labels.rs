//! The words of the copied items in each language of the game client.
//!
//! The parser only knows the structure of a copied item; every word it looks for comes from
//! here. The markers at the end of the lines (`(augmented)`, `(rune)`, `(implicit)`…) are in
//! English in every language.

use super::{Affix, Catalyst, ItemState, ModifierKind, ModifierOrigin, Rarity};
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
    /// The words around the kind of the quality of a catalyst, `Quality (` and `): ` in
    /// `Quality (Mana Modifiers): +40%`.
    pub catalyst_quality: (&'static str, &'static str),
    /// The kinds of the quality of a catalyst, as the game writes them between the
    /// parentheses.
    pub catalysts: &'static [(&'static str, Catalyst)],
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
    /// The defence of a runeforged base, without attribute.
    pub runic_ward: &'static str,
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
    pub modifier_kinds: &'static [HeaderKind],
    /// The words before the kind in a modifier header.
    pub modifier_origins: &'static [(&'static str, ModifierOrigin)],
    /// The only line of a desecrated modifier not revealed yet, for each affix, under its
    /// header without tier (`{ Prefix Modifier "Veiled" }`).
    pub unrevealed: &'static [(&'static str, Affix)],
}

/// The kind of a modifier header, e.g. `Prefix Modifier`.
#[derive(Debug, Clone, Copy)]
pub(super) struct HeaderKind {
    pub label: &'static str,
    pub kind: ModifierKind,
    /// The origin the kind itself tells: the Vaal modifier of a unique.
    pub origin: Option<ModifierOrigin>,
}

impl HeaderKind {
    const fn new(label: &'static str, kind: ModifierKind) -> Self {
        Self {
            label,
            kind,
            origin: None,
        }
    }

    const fn vaal(label: &'static str) -> Self {
        Self {
            label,
            kind: UNIQUE,
            origin: Some(ModifierOrigin::Vaal),
        }
    }
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
    catalyst_quality: ("Quality (", "): "),
    // Seen on the trade site on 2026-10-31: Life, Mana, Defence, Fire, Cold, Lightning, Attack,
    // Caster, Attribute and Minion; Physical, Chaos and Speed from the tags of Exiled
    // Exchange 2 (`CATALYST_TO_TAG`).
    catalysts: &[
        ("Life Modifiers", Catalyst::Life),
        ("Mana Modifiers", Catalyst::Mana),
        ("Defence Modifiers", Catalyst::Defence),
        ("Physical Modifiers", Catalyst::Physical),
        ("Fire Modifiers", Catalyst::Fire),
        ("Cold Modifiers", Catalyst::Cold),
        ("Lightning Modifiers", Catalyst::Lightning),
        ("Chaos Modifiers", Catalyst::Chaos),
        ("Attack Modifiers", Catalyst::Attack),
        ("Caster Modifiers", Catalyst::Caster),
        ("Speed Modifiers", Catalyst::Speed),
        ("Attribute Modifiers", Catalyst::Attribute),
        ("Minion Modifiers", Catalyst::Minion),
    ],
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
    block: "Block chance",
    runic_ward: "Runic Ward",
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
    fixed_value: " — Unscalable Value",
    modifier_kinds: &[
        HeaderKind::new("Prefix Modifier", PREFIX),
        HeaderKind::new("Suffix Modifier", SUFFIX),
        HeaderKind::vaal("Vaal Unique Modifier"),
        HeaderKind::new("Unique Modifier", UNIQUE),
        HeaderKind::new("Implicit Modifier", ModifierKind::Implicit),
        HeaderKind::new("Corruption Enhancement", ModifierKind::Corruption),
        HeaderKind::new("Enhancement", ModifierKind::Enchant),
    ],
    modifier_origins: &[
        ("Crafted", ModifierOrigin::Crafted),
        ("Desecrated", ModifierOrigin::Desecrated),
        ("Fractured", ModifierOrigin::Fractured),
    ],
    unrevealed: &[
        ("Desecrated Prefix", Affix::Prefix),
        ("Desecrated Suffix", Affix::Suffix),
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
    catalyst_quality: ("Qualité (", "): "),
    // Seen in the owner's copies: Mana and Froid; the others from the French tags of Exiled
    // Exchange 2, unchecked (a wrong one keeps the words of the game); the defences unknown.
    catalysts: &[
        ("Modificateurs de Vie", Catalyst::Life),
        ("Modificateurs de Mana", Catalyst::Mana),
        ("Modificateurs de Physique", Catalyst::Physical),
        ("Modificateurs de Feu", Catalyst::Fire),
        ("Modificateurs de Froid", Catalyst::Cold),
        ("Modificateurs de Foudre", Catalyst::Lightning),
        ("Modificateurs de Chaos", Catalyst::Chaos),
        ("Modificateurs d'Attaque", Catalyst::Attack),
        ("Modificateurs de Lanceur de sorts", Catalyst::Caster),
        ("Modificateurs de Vitesse", Catalyst::Speed),
        ("Modificateurs d'Attribut", Catalyst::Attribute),
        ("Modificateurs de Créature", Catalyst::Minion),
    ],
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
    runic_ward: "Sauvegarde runique",
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
        HeaderKind::new("Préfixe", PREFIX),
        HeaderKind::new("Suffixe", SUFFIX),
        HeaderKind::vaal("Modificateur vaal unique"),
        HeaderKind::new("Modificateur d'Objet unique", UNIQUE),
        HeaderKind::new("Modificateur Implicite", ModifierKind::Implicit),
        HeaderKind::new("Amélioration de Corruption", ModifierKind::Corruption),
        HeaderKind::new("Amélioration", ModifierKind::Enchant),
    ],
    modifier_origins: &[
        ("Artisanat", ModifierOrigin::Crafted),
        ("Profané", ModifierOrigin::Desecrated),
        ("Fissuré", ModifierOrigin::Fractured),
    ],
    unrevealed: &[
        ("Préfixe profané", Affix::Prefix),
        ("Suffixe profané", Affix::Suffix),
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

    fn every_origin() -> [ModifierOrigin; 4] {
        use ModifierOrigin::{Crafted, Desecrated, Fractured, Vaal};
        let all = [Crafted, Desecrated, Fractured, Vaal];
        for origin in all {
            match origin {
                Crafted | Desecrated | Fractured | Vaal => {}
            }
        }
        all
    }

    #[test]
    fn every_language_names_every_rarity_state_origin_and_unrevealed_affix() {
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
            // By the words before the kind of the header, or by the kind itself.
            for origin in every_origin() {
                assert!(
                    labels.modifier_origins.iter().any(|&(_, o)| o == origin)
                        || labels
                            .modifier_kinds
                            .iter()
                            .any(|kind| kind.origin == Some(origin)),
                    "{language:?} {origin:?}"
                );
            }
            for affix in [Affix::Prefix, Affix::Suffix] {
                assert!(
                    labels.unrevealed.iter().any(|&(_, a)| a == affix),
                    "{language:?} {affix:?}"
                );
            }
        }
    }

    #[test]
    fn no_modifier_kind_is_hidden_by_an_earlier_one() {
        for language in Language::ALL {
            let kinds = Labels::of(language).modifier_kinds;
            for (index, earlier) in kinds.iter().enumerate() {
                for later in kinds.iter().skip(index + 1) {
                    assert!(
                        !later.label.ends_with(earlier.label),
                        "{language:?}: {} after {}",
                        later.label,
                        earlier.label
                    );
                }
            }
        }
    }
}
