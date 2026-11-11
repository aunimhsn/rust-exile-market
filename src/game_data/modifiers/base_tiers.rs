//! The modifier tables as an item sees them, on its base and at its level; and the tiers of
//! a modifier on that base: those of every table of the modifier that rolls on it, numbered
//! from the best.

use super::Tier;
use super::copied::Copied;
use super::tables::{TABLES, Table, TableTier};
use crate::game_data::bases::{self, BaseSpawn};
use crate::item::Range;
use crate::language::Language;

/// The item whose modifiers are looked up in the tables.
#[derive(Debug, Clone, Copy)]
pub struct ItemBase<'a> {
    /// The language of the item, which its base type is written in.
    pub language: Language,
    /// Its base type, the name its search looks for.
    pub base_type: &'a str,
    /// Its level; none when the copy doesn't write it.
    pub item_level: Option<u32>,
}

/// The modifier tables as an item sees them: on its base, among the tiers its level allows.
#[derive(Clone, Copy)]
pub(super) struct ItemTables<'a> {
    /// Every table, whatever its bases: those of the game, or those of a test.
    pub(super) all: &'a [Table],
    pub(super) base: &'a BaseSpawn,
    /// None when the copy doesn't write it: an item of any level.
    pub(super) item_level: Option<u32>,
}

impl ItemTables<'static> {
    /// The tables of the game as the item of `base` sees them; none for a base type the data
    /// don't know, or don't know the tags of.
    pub(super) fn of_game(base: ItemBase<'_>) -> Option<Self> {
        Some(Self {
            all: &TABLES,
            base: bases::spawn(base.language, base.base_type)?,
            item_level: base.item_level,
        })
    }
}

impl ItemTables<'_> {
    /// Whether an item of that level can roll `tier`.
    pub(super) fn allows(&self, tier: &Tier) -> bool {
        self.item_level.is_none_or(|level| tier.level <= level)
    }

    /// Every tier of the modifier of `table` that rolls on the base, whatever the level of the
    /// item, the best first, the ranges of each in the order of the copied lines (`order`,
    /// [`Copied::order`]): a type can roll other ranges on other bases (the added damage of a
    /// two-handed weapon). None when a line of a tier hasn't as many values as the copied one.
    pub(super) fn tiers_of(
        &self,
        table: &Table,
        order: &[usize],
        copied: &Copied<'_>,
    ) -> Option<Vec<Tier>> {
        let mut tiers: Vec<&TableTier> = self
            .all
            .iter()
            .filter(|other| other.same_modifier(table) && other.rolls_on(self.base))
            .flat_map(|other| &other.tiers)
            .collect();
        // By level, then by ranges, as the generator sorts the tiers of a table: of two tiers
        // of one level, the higher ranges last, the better (`Upgrades Radius to Large` of a
        // Time-Lost jewel, T1, after `to Medium`), also from two tables (the cast speed of the
        // Transcendent Alloy on a wand, `39–47%` of the staves, T1, after the `26–31%` of the
        // one-handed weapons). The values of the game are never NaN: two ranges always compare.
        tiers.sort_by(|tier, other| {
            tier.level.cmp(&other.level).then_with(|| {
                range_values(tier)
                    .partial_cmp(range_values(other))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        });
        tiers
            .iter()
            .rev()
            .zip(1..)
            .map(|(tier, number)| {
                Some(Tier {
                    number,
                    level: tier.level,
                    ranges: ranges_in_order(tier, order, copied)?,
                })
            })
            .collect()
    }
}

/// The bounds of every range of `tier`, in the order of its lines.
pub(super) fn range_values(tier: &TableTier) -> impl Iterator<Item = f64> + '_ {
    tier.ranges.iter().flatten().flatten().copied()
}

/// The ranges of `tier` in the order of the copied lines.
fn ranges_in_order(tier: &TableTier, order: &[usize], copied: &Copied<'_>) -> Option<Vec<Range>> {
    let mut ranges = Vec::new();
    for (line, &index) in copied.lines.iter().zip(order) {
        let line_ranges = tier.ranges.get(index)?;
        if line_ranges.len() != line.count() {
            return None;
        }
        ranges.extend(line_ranges.iter().map(|&[min, max]| Range {
            min: min.min(max),
            max: min.max(max),
        }));
    }
    Some(ranges)
}

#[cfg(test)]
mod game_tests;
