//! The tiers of a modifier on a base: those of every table of the modifier that rolls on it,
//! numbered from the best.

use super::Tier;
use super::copied::Copied;
use super::tables::{Table, TableTier};
use crate::game_data::bases::BaseSpawn;
use crate::item::Range;

/// Every tier of the modifier of `table` that rolls on `base`, the best first, the
/// ranges of each in the order of the copied lines (`order`, [`Copied::order`]): a type can
/// roll other ranges on other bases (the added damage of a two-handed weapon). None when a
/// line of a tier hasn't as many values as the copied one.
pub(super) fn tiers_on_base(
    tables: &[Table],
    table: &Table,
    base: &BaseSpawn,
    order: &[usize],
    copied: &Copied<'_>,
) -> Option<Vec<Tier>> {
    let mut tiers: Vec<&TableTier> = tables
        .iter()
        .filter(|other| other.same_modifier(table) && other.rolls_on(base))
        .flat_map(|other| &other.tiers)
        .collect();
    // By level, then by ranges, as the generator sorts the tiers of a table: of two tiers of one
    // level, the higher ranges last, the better (`Upgrades Radius to Large` of a Time-Lost jewel,
    // T1, after `to Medium`), also from two tables (the cast speed of the Transcendent Alloy on a
    // wand, `39–47%` of the staves, T1, after the `26–31%` of the one-handed weapons).
    // The values of the game are never NaN: two ranges always compare.
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
