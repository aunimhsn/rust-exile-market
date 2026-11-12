//! The modifier tables as an item sees them, on its base and at its level: the modifiers that
//! have the lines of a copied one (`holding`), and the tiers of a modifier on that base, those
//! of every table of the modifier that rolls on it, numbered from the best.

use super::copied::Copied;
use super::tables::{TABLES, Table, TableAffix, TableTier, first_of_its_modifier};
use super::{ModifierTiers, Tier};
use crate::game_data::bases::{self, BaseSpawn};
use crate::item::{Lord, Range};
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

/// A modifier of the tables whose lines are those of a copied modifier, on the base of an
/// item.
#[derive(Clone)]
pub(super) struct Held<'a> {
    /// The index among the tables of its first table.
    pub(super) index: usize,
    /// Its first table.
    pub(super) table: &'a Table,
    /// Its tiers on the base ([`ItemTables::tiers_of`]), one at least.
    pub(super) tiers: Vec<Tier>,
    /// The copied values, in their order, as its table shows them ([`Copied::values`]).
    pub(super) values: Vec<f64>,
}

impl Held<'_> {
    /// Its tiers as those of `copied`, found in the tier of index `found`. Consumed: a lookup
    /// moves the tiers of the modifier it found, which a borrow would clone at each lookup (the
    /// draw of `random_items.rs` takes 9.96 s in place of 9.77, 2026-11-12); `splits`, which
    /// reads a modifier in several ways, clones it for each.
    pub(super) fn into_tiers(self, found: usize, copied: &Copied<'_>) -> ModifierTiers {
        ModifierTiers {
            tiers: self.tiers,
            found,
            neighbour: None,
            counts: copied.counts(),
            tier_unwritten: self.table.tier_unwritten(),
            currency: self.table.currency(),
            lord: self.table.lord.map(Lord::from),
        }
    }
}

impl<'a> ItemTables<'a> {
    /// Each modifier of the tables that has the lines of `copied`, in the order of the file:
    /// of its origin ([`Copied::may_be`]), its lines in any order, looked at once, at its first
    /// table, with its tiers on the base whatever the level of the item; a modifier that doesn't
    /// roll on the base isn't given. Whether one of these tiers holds the copied values is left
    /// to the caller.
    pub(super) fn holding(&self, copied: &Copied<'_>) -> impl Iterator<Item = Held<'a>> {
        let tables = self.all.iter().enumerate();
        tables.filter_map(move |(index, table)| self.held(index, table, copied))
    }

    /// The modifiers of `affix` among those that have the lines of `copied`
    /// ([`Self::holding`]).
    pub(super) fn holding_as(
        &self,
        copied: &Copied<'_>,
        affix: TableAffix,
    ) -> impl Iterator<Item = Held<'a>> {
        // The affix first, the cheapest to tell: a table of the other one is looked at no
        // further.
        let tables = self.all.iter().enumerate();
        tables
            .filter(move |(_, table)| table.affix == affix)
            .filter_map(move |(index, table)| self.held(index, table, copied))
    }

    /// The modifier of `table`, the table of `index`; none unless `table` is of the origin of
    /// `copied`, has its lines, is the first table of its modifier, and the modifier has tiers
    /// on the base, each with a range for each copied value ([`Self::tiers_of`]).
    #[expect(
        clippy::inline_always,
        reason = "run for each table at each lookup: left to the compiler, which doesn't inline \
                  it, the draw of `random_items.rs` takes 11.0 s in place of 9.8 (2026-11-11), \
                  and 11.1 s with `#[inline]` alone (2026-11-12)"
    )]
    #[inline(always)]
    fn held(&self, index: usize, table: &'a Table, copied: &Copied<'_>) -> Option<Held<'a>> {
        if !copied.may_be(table) {
            return None;
        }
        let order = copied.order(&table.lines)?;
        if !first_of_its_modifier(self.all, index) {
            return None;
        }
        // No table of the modifier rolls on the base: `tiers_of` gives an empty list, not none.
        let tiers = self.tiers_of(table, &order, copied)?;
        if tiers.is_empty() {
            return None;
        }
        Some(Held {
            index,
            table,
            tiers,
            values: copied.values(&table.lines, &order),
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
mod tests;

#[cfg(test)]
mod game_tests;
