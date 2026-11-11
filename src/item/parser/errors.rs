//! Why a copied item, or a part of it, couldn't be read.

use std::fmt;

/// Why a copied item couldn't be read at all. It never holds the text of the item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    MissingRarity,
    UnknownRarity,
    MissingBaseType,
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::MissingRarity => "no rarity line",
            Self::UnknownRarity => "unknown rarity",
            Self::MissingBaseType => "no base type",
        })
    }
}

impl std::error::Error for ParseError {}

/// A part of the item that couldn't be read: the item is read without it. It never holds the
/// text of the item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseWarning {
    /// A line whose value can't be read: not a number where one is expected, or an
    /// attribute of the requirements that isn't in the labels.
    InvalidValue(Field),
    /// A modifier header whose kind isn't in the labels: the modifier is kept without its
    /// kind, name nor tier.
    UnknownModifierHeader,
    /// Modifier lines before the first header of their section: they are kept as a
    /// modifier without kind, name nor tier.
    ModifierWithoutHeader,
    /// The quality of a catalyst of a kind not in the labels: it is kept with the words of
    /// the game.
    UnknownQualityKind,
    /// A section without header whose lines mix a marker of a kind (` (rune)`) with another
    /// marker (` (enchant)`, ` (crafted)`), a format not seen: its lines aren't read.
    MixedMarkers,
    /// A property of a waystone whose name isn't in the labels, maybe added or renamed by a
    /// patch: it is neither shown nor searched.
    UnknownWaystoneProperty,
}

/// A line of the item with a value to read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    ItemLevel,
    Stack,
    GemLevel,
    Quality,
    Requirements,
    WaystoneProperty,
}

impl fmt::Display for Field {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ItemLevel => "item level",
            Self::Stack => "stack size",
            Self::GemLevel => "level of a gem",
            Self::Quality => "quality",
            Self::Requirements => "requirements",
            Self::WaystoneProperty => "property of a waystone",
        })
    }
}

impl fmt::Display for ParseWarning {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidValue(field) => write!(formatter, "invalid {field}"),
            Self::UnknownModifierHeader => formatter.write_str("unknown modifier header"),
            Self::ModifierWithoutHeader => formatter.write_str("modifier lines without header"),
            Self::UnknownQualityKind => formatter.write_str("unknown kind of quality"),
            Self::MixedMarkers => formatter.write_str("modifier lines of mixed markers"),
            Self::UnknownWaystoneProperty => formatter.write_str("unknown property of a waystone"),
        }
    }
}
