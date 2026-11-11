//! The modifier tables, from RePoE (`assets/game-data/modifiers.ndjson`): every tier of a
//! prefix or suffix on the base of an item, found from the stats of its lines, or from their
//! values alone for an item copied from the in-game trade.
//!
//! One file per rule, each depending only on those named before it. The `tables` as the
//! generator writes them, the lines of a `copied` modifier, the tables as an item sees them
//! and the tiers of a modifier on its base (`base_tiers`), a line that adds up two modifiers
//! (`split_lines`), and the `lookup` of a copied modifier in the tables. Then the reading of
//! an item copied from the in-game trade: its `candidates`, its `best_reading` and its
//! `doubts`, put together by `trade_tiers`, and its modifiers written again as the advanced
//! copy writes them (`trade_modifiers`).

use crate::item::{Currency, Lord, Range};

mod base_tiers;
mod best_reading;
mod candidates;
mod copied;
mod doubts;
#[cfg(test)]
mod fixtures;
mod lookup;
mod split_lines;
mod tables;
mod trade_modifiers;
mod trade_tiers;

pub use base_tiers::ItemBase;
pub use lookup::{lacks_table, modifier_tiers};
pub use trade_modifiers::with_trade_tiers;
pub use trade_tiers::{AffixLimits, TradeItem};

/// A tier of a modifier on a base.
#[derive(Debug, Clone, PartialEq)]
pub struct Tier {
    /// 1 is the best.
    pub number: u32,
    /// The level an item needs for the tier to roll.
    pub level: u32,
    /// The range of each value of the copied lines, in their order, with the sign of the stat
    /// (`#% reduced` rolls negative values).
    pub ranges: Vec<Range>,
}

/// Every tier of a modifier on the base of its item.
#[derive(Debug, Clone, PartialEq)]
pub struct ModifierTiers {
    /// The best first: tier 1, tier 2…
    pub tiers: Vec<Tier>,
    /// The index in `tiers` of the tier of the copied modifier; for a modifier in two
    /// neighbouring tiers, the worse of both: [`Self::possible`] gives both.
    pub found: usize,
    /// The index in `tiers` of the tier above `found` (always `found - 1`) the copied modifier
    /// may be as well: a part of a line that adds up two modifiers, in either of two
    /// neighbouring tiers while the other part is sure (the pure T1 or T2 of the `248%
    /// increased Physical Damage` of a bow, with the hybrid T1 on `+195 to Accuracy Rating`);
    /// none otherwise.
    pub neighbour: Option<usize>,
    /// The number of values of each line match given, in their order: the ranges of a tier
    /// follow them (two for `Adds # to # Physical Damage`; for a line without number, one
    /// when its string gives the value, `Loads an additional bolt`, none otherwise).
    pub counts: Vec<usize>,
    /// Whether the game writes the modifier without tier, one of the Genesis tree, of an
    /// essence or of an alloy: it is read as the advanced copy writes it, the ranges of its tier
    /// without number.
    pub tier_unwritten: bool,
    /// The currency that adds the modifier, an essence or an alloy: none for another
    /// modifier, or when tables of another source hold the copied one as well (`find`).
    pub currency: Option<Currency>,
    /// The lord of the Abyss whose modifier it is: none for another modifier, or when the
    /// modifiers of two lords hold the copied one as well (`find`; a split, `splits`, takes the
    /// lord of its table, never shown).
    pub lord: Option<Lord>,
}

impl ModifierTiers {
    /// The tier of the copied modifier.
    #[must_use]
    pub fn tier(&self) -> Option<&Tier> {
        self.tiers.get(self.found)
    }

    /// Whether the modifier has one tier only on the base: its T1 would read as "the best"
    /// while it is "the only one".
    #[must_use]
    pub fn single(&self) -> bool {
        self.tiers.len() == 1
    }

    /// Whether the number of its tier isn't shown: one tier only on the base, or a tier the
    /// game doesn't write (the Genesis tree, an essence, an alloy).
    #[must_use]
    pub fn hides_number(&self) -> bool {
        self.single() || self.tier_unwritten
    }

    /// The tiers the copied modifier may be, the best first: the neighbouring one, then its
    /// tier.
    pub fn possible(&self) -> impl Iterator<Item = &Tier> {
        let neighbour = self.neighbour.and_then(|index| self.tiers.get(index));
        neighbour.into_iter().chain(self.tier())
    }

    /// These tiers, the copied modifier in its tier or in `other`, a neighbouring one: the
    /// worse of both found, the better its neighbour.
    fn or_tier(&self, other: usize) -> Self {
        Self {
            found: self.found.max(other),
            neighbour: Some(self.found.min(other)),
            ..self.clone()
        }
    }
}
