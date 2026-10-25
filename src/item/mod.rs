//! A Path of Exile 2 item: the text copied in the game, and what the parser reads from it.
//!
//! The parser reads every copied item, in English or in French. It must never panic: a panic
//! message goes to the log with the line that caused it. The lints below, and the methods
//! forbidden by `clippy.toml`, rule out what can panic outside the tests; the parser returns
//! a [`ParseError`] or keeps a [`ParseWarning`] instead.
#![cfg_attr(
    not(test),
    deny(
        clippy::indexing_slicing,
        clippy::string_slice,
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable,
        clippy::todo,
        clippy::unimplemented
    )
)]

mod copied;
mod labels;
mod modifier;
mod parser;
mod weapon;

pub use copied::CopiedItem;
pub use modifier::{Affix, Modifier, ModifierKind, ModifierOrigin};
pub use parser::{ParseError, ParseWarning};
pub use weapon::{DamageKind, Weapon};

use crate::language::Language;

/// The rarity of an item, which also tells currencies and gems apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rarity {
    Normal,
    Magic,
    Rare,
    Unique,
    Currency,
    Gem,
}

impl Rarity {
    /// Whether the item can have explicit modifiers: prefixes, suffixes or unique modifiers.
    fn has_explicit_modifiers(self) -> bool {
        matches!(self, Self::Magic | Self::Rare | Self::Unique)
    }
}

/// A state written at the end of the item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemState {
    Corrupted,
    TwiceCorrupted,
    Sanctified,
    Fractured,
    Mirrored,
    Unidentified,
}

/// The size of a stack, and the largest stack of this item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stack {
    pub size: u32,
    pub max: u32,
}

/// What a character needs to use the item.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Requirements {
    pub level: Option<u32>,
    pub strength: Option<u32>,
    pub dexterity: Option<u32>,
    pub intelligence: Option<u32>,
}

/// A line of the properties section, e.g. `Physical Damage: 74-231 (augmented)`.
#[derive(Debug, Clone, PartialEq)]
pub struct Property {
    pub name: String,
    /// Empty for a line without value (`Name: value`).
    pub value: String,
    /// Changed by the quality or the modifiers of the item.
    pub augmented: bool,
}

/// What the parser reads from a copied item.
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    language: Language,
    class: String,
    rarity: Rarity,
    name: Option<String>,
    base_type: String,
    /// The base type as the game writes it, with "Exceptional" and "Superior".
    written_base_type: String,
    exceptional: bool,
    stack: Option<Stack>,
    #[expect(
        clippy::struct_field_names,
        reason = "the game calls it the item level"
    )]
    item_level: Option<u32>,
    quality: Option<u32>,
    sockets: Option<u32>,
    requirements: Requirements,
    granted_skills: Vec<String>,
    states: Vec<ItemState>,
    properties: Vec<Property>,
    /// The damage and the speed of a weapon.
    weapon: Option<Weapon>,
    modifiers: Vec<Modifier>,
    warnings: Vec<ParseWarning>,
}

impl Item {
    /// Reads a copied item.
    ///
    /// # Errors
    ///
    /// When the header of the item (rarity, base type) can't be read: the window then asks
    /// to copy it again. The other parts that can't be read are [`Item::warnings`].
    pub fn parse(copied: &CopiedItem) -> Result<Self, ParseError> {
        parser::parse(copied)
    }

    /// The parts of the item that couldn't be read, and that are missing from it.
    #[must_use]
    pub fn warnings(&self) -> &[ParseWarning] {
        &self.warnings
    }

    /// The name the game shows: the name of a rare or unique item, and the name line as
    /// written for the others, with the affixes of a magic item and the word "Exceptional"
    /// ("Exceptional Chiming Staff").
    #[must_use]
    pub fn title(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.written_base_type)
    }

    /// The base type as the game writes it, with "Exceptional" and "Superior".
    #[must_use]
    pub fn written_base_type(&self) -> &str {
        &self.written_base_type
    }

    /// The damage and the speed of a weapon.
    #[must_use]
    pub fn weapon(&self) -> Option<&Weapon> {
        self.weapon.as_ref()
    }

    /// The number of prefixes and suffixes at tier 1, the best one; only the advanced copy
    /// gives the tiers.
    #[must_use]
    pub fn tier_1_affixes(&self) -> usize {
        self.modifiers
            .iter()
            .filter(|modifier| {
                matches!(modifier.kind, ModifierKind::Explicit(Some(_))) && modifier.tier == Some(1)
            })
            .count()
    }
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "read by the price display and the filters from step 9"
    )
)]
impl Item {
    /// The language of the game client that copied the item.
    #[must_use]
    pub fn language(&self) -> Language {
        self.language
    }

    /// The item class, in the language of the item (`Bows`, `Bâtons`).
    #[must_use]
    pub fn class(&self) -> &str {
        &self.class
    }

    #[must_use]
    pub fn rarity(&self) -> Rarity {
        self.rarity
    }

    /// The name of a rare or unique item; `None` for the other items, named by their base.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// The base type, without "Exceptional" and "Superior". For a magic item, the whole name
    /// with its affixes: its base type is found among the names of the trade site
    /// ([`crate::item_names::ItemNames::magic_base_type`]).
    #[must_use]
    pub fn base_type(&self) -> &str {
        &self.base_type
    }

    /// An exceptional base, which can have more sockets.
    #[must_use]
    pub fn is_exceptional(&self) -> bool {
        self.exceptional
    }

    #[must_use]
    pub fn stack(&self) -> Option<Stack> {
        self.stack
    }

    #[must_use]
    pub fn item_level(&self) -> Option<u32> {
        self.item_level
    }

    /// The quality, in percent.
    #[must_use]
    pub fn quality(&self) -> Option<u32> {
        self.quality
    }

    /// The number of rune sockets.
    #[must_use]
    pub fn sockets(&self) -> Option<u32> {
        self.sockets
    }

    #[must_use]
    pub fn requirements(&self) -> Requirements {
        self.requirements
    }

    /// The skills the item grants, e.g. `Level 18 Cackling Companions`.
    #[must_use]
    pub fn granted_skills(&self) -> &[String] {
        &self.granted_skills
    }

    #[must_use]
    pub fn has_state(&self, state: ItemState) -> bool {
        self.states.contains(&state)
    }

    #[must_use]
    pub fn states(&self) -> &[ItemState] {
        &self.states
    }

    /// The properties of the base: damage, defences, attack speed…
    #[must_use]
    pub fn properties(&self) -> &[Property] {
        &self.properties
    }

    /// The modifiers, in the order of the item: enchantments, runes, implicit, then explicit.
    #[must_use]
    pub fn modifiers(&self) -> &[Modifier] {
        &self.modifiers
    }
}
