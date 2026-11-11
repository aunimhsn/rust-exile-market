//! The marker at the end of a modifier line written without header, which tells its kind or its
//! origin, e.g. ` (rune)` or ` (fractured)`.

use super::{ModifierKind, ModifierOrigin};

/// The marker at the end of a modifier line, in English in every language, e.g. ` (rune)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Marker {
    Implicit,
    Enchant,
    Rune,
    Crafted,
    Fractured,
    Desecrated,
    /// The Vaal modifier of a corrupted unique copied from the trade of the game.
    Mutated,
}

impl Marker {
    const ALL: [Self; 7] = [
        Self::Implicit,
        Self::Enchant,
        Self::Rune,
        Self::Crafted,
        Self::Fractured,
        Self::Desecrated,
        Self::Mutated,
    ];

    pub fn suffix(self) -> &'static str {
        match self {
            Self::Implicit => " (implicit)",
            Self::Enchant => " (enchant)",
            Self::Rune => " (rune)",
            Self::Crafted => " (crafted)",
            Self::Fractured => " (fractured)",
            Self::Desecrated => " (desecrated)",
            Self::Mutated => " (mutated)",
        }
    }

    /// A line without its marker, and the marker found.
    pub fn take(line: &str) -> (&str, Option<Self>) {
        Self::ALL
            .into_iter()
            .find_map(|marker| Some((line.strip_suffix(marker.suffix())?, Some(marker))))
            .unwrap_or((line, None))
    }

    /// The kind of the modifiers that the marker alone tells, e.g. a rune.
    pub fn kind(self) -> Option<ModifierKind> {
        match self {
            Self::Implicit => Some(ModifierKind::Implicit),
            Self::Enchant => Some(ModifierKind::Enchant),
            Self::Rune => Some(ModifierKind::Rune),
            Self::Crafted | Self::Fractured | Self::Desecrated | Self::Mutated => None,
        }
    }

    pub fn origin(self) -> Option<ModifierOrigin> {
        match self {
            Self::Crafted => Some(ModifierOrigin::Crafted),
            Self::Fractured => Some(ModifierOrigin::Fractured),
            Self::Desecrated => Some(ModifierOrigin::Desecrated),
            Self::Mutated => Some(ModifierOrigin::Vaal),
            Self::Implicit | Self::Enchant | Self::Rune => None,
        }
    }
}
