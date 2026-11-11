//! The modifiers of the tables placed on the lines of an item: each tier of each modifier on
//! the lines it can sit on, without those whose values no sum can hold.

use super::{Line, Shown};
use crate::game_data::modifiers::base_tiers::ItemTables;
use crate::game_data::modifiers::copied::{Copied, CopiedTier, ROUNDING};
use crate::game_data::modifiers::tables::{Table, first_of_its_modifier};
use crate::item::{ModifierOrigin, Range};

/// A tier of a modifier of the tables on some lines of the item.
pub(super) struct Placement<'a> {
    /// Two modifiers of one type can't be on the same item.
    pub(super) kind: &'a str,
    pub(super) fractured: bool,
    pub(super) shown: Shown,
    /// Its lines, by index among the [`Line`]s, in their order.
    pub(super) lines: Vec<usize>,
    /// The range it adds to each value of its lines, by index among the values of every line.
    pub(super) ranges: Vec<(usize, Range)>,
}

/// Every tier of every modifier of `tables`, among those the level of the item allows, whose
/// lines are among `lines`, of one origin; without a tier that can't hold the `values`
/// whatever adds up with it.
pub(super) fn placements<'a>(
    tables: &ItemTables<'a>,
    lines: &[Line<'a>],
    values: &[f64],
) -> Vec<Placement<'a>> {
    let mut placements = Vec::new();
    for (index, table) in tables.all.iter().enumerate() {
        for mapped in mappings(table, lines) {
            // The lines of a mapping are of one origin.
            let origin = mapped
                .first()
                .and_then(|&line| lines.get(line))
                .and_then(|line| line.origin);
            if !table.may_hold(origin) {
                continue;
            }
            let copied = Copied {
                lines: mapped
                    .iter()
                    .filter_map(|&line| lines.get(line))
                    .map(|line| line.copied.clone())
                    .collect(),
                ranges: None,
                tier: CopiedTier::Unknown,
                origin,
            };
            let Some(order) = copied.order(&table.lines) else {
                continue;
            };
            if !first_of_its_modifier(tables.all, index) {
                continue;
            }
            // The value of a line without number is that of its stat: divided, it would differ
            // from a modifier to another (no such line rolls on a base, 2026-10-28; the best
            // reading isn't met then, and every line is in doubt).
            let divided = copied.lines.iter().zip(&order).any(|(line, &index)| {
                line.written.is_some()
                    && table
                        .lines
                        .get(index)
                        .is_some_and(|line| (line.divisor - 1.0).abs() > f64::EPSILON)
            });
            if divided {
                continue;
            }
            let Some(tiers) = tables.tiers_of(table, &order, &copied) else {
                continue;
            };
            let indexes: Vec<usize> = mapped
                .iter()
                .filter_map(|&line| lines.get(line))
                .flat_map(|line| line.values.clone())
                .collect();
            let mut modifiers: Vec<usize> = mapped
                .iter()
                .filter_map(|&line| lines.get(line))
                .map(|line| line.modifier)
                .collect();
            modifiers.dedup();
            let fractured = origin == Some(ModifierOrigin::Fractured);
            for tier in &tiers {
                if !tables.allows(tier) || tier.ranges.len() != indexes.len() {
                    continue;
                }
                placements.push(Placement {
                    kind: &table.kind,
                    fractured,
                    shown: Shown {
                        affix: table.affix.into(),
                        tier: tier.number,
                        modifiers: modifiers.clone(),
                        ranges: tier.ranges.clone(),
                    },
                    lines: mapped.clone(),
                    ranges: indexes
                        .iter()
                        .copied()
                        .zip(tier.ranges.iter().copied())
                        .collect(),
                });
            }
        }
    }
    // A value whose ranges only add up one way: a tier past it can't be among those of its line.
    let (rising, falling) = directions(&placements, values.len());
    placements.retain(|placement| {
        placement
            .ranges
            .iter()
            .all(|&(index, range)| !past(&rising, &falling, values, index, range))
    });
    placements
}

/// Whether the sum `sum` of the ranges on the value of `index` among `values` is past it,
/// whatever other ranges add up with it: only when they all rise (`rising`) or all fall.
pub(super) fn past(
    rising: &[bool],
    falling: &[bool],
    values: &[f64],
    index: usize,
    sum: Range,
) -> bool {
    let Some(&value) = values.get(index) else {
        return true;
    };
    let rises = rising.get(index).copied().unwrap_or(false);
    let falls = falling.get(index).copied().unwrap_or(false);
    rises && sum.min > value + ROUNDING || falls && sum.max < value - ROUNDING
}

/// For each value, whether every range a modifier adds to it is above zero (the sum only rises
/// as modifiers are added), and whether every one is below zero.
pub(super) fn directions(placements: &[Placement<'_>], count: usize) -> (Vec<bool>, Vec<bool>) {
    let mut rising = vec![true; count];
    let mut falling = vec![true; count];
    for &(index, range) in placements.iter().flat_map(|placement| &placement.ranges) {
        if let Some(rises) = rising.get_mut(index) {
            *rises &= range.min >= 0.0;
        }
        if let Some(falls) = falling.get_mut(index) {
            *falls &= range.max <= 0.0;
        }
    }
    (rising, falling)
}

/// Each way the lines of `table` are lines of the item among `lines`, each its own, of one
/// origin: for each, the indexes of the lines of the item, in their order.
fn mappings(table: &Table, lines: &[Line<'_>]) -> Vec<Vec<usize>> {
    let mut ways: Vec<Vec<usize>> = vec![Vec::new()];
    for table_line in &table.lines {
        ways = ways
            .into_iter()
            .flat_map(|way| {
                lines
                    .iter()
                    .enumerate()
                    .filter(|(index, line)| {
                        line.copied.is(&table_line.template)
                            && !way.contains(index)
                            && way
                                .first()
                                .and_then(|&first| lines.get(first))
                                .is_none_or(|first| first.origin == line.origin)
                    })
                    .map(|(index, _)| {
                        let mut way = way.clone();
                        way.push(index);
                        way
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
    }
    for way in &mut ways {
        way.sort_unstable();
    }
    ways.sort();
    ways.dedup();
    ways
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod game_tests;
