//! The arithmetic of the rolls: where a value falls in its ranges, the bounds of a field, the
//! limit of a filter that finds comparable items, and the ranges written as text.

use super::model::{Origin, StatRoll};
use crate::filters::Half;
use crate::format;
use crate::item::{Range, StatLine};
use crate::ui::widgets;

/// The share of the copied value the limit of a rare, magic or normal item leaves below it
/// for a minimum, above it for the maximum of a line better lower, to find comparable items
/// and not only identical ones.
pub(super) const COMPARABLE_MARGIN: f64 = 0.1;

/// The limit 10 % of `value` below it for a minimum, above it for a maximum, also for a
/// negative value (`-15` gets a minimum of `-17`, not `-14`, which the item itself wouldn't
/// pass): rounded to `decimals`, those of the copied numbers, so that it is written as short
/// as them (`12.2` gets `11`, not `10.98`). Never beyond `value`, so that the item always
/// passes its own filter, also when the rounding would cross it: the average of three rolls
/// (`1.333` written whole) or a value with more decimals than the numbers are written with.
pub(super) fn comparable_limit(value: f64, decimals: u8, half: Half) -> f64 {
    let margin = value.abs() * COMPARABLE_MARGIN;
    match half {
        Half::Minimum => format::rounded(value - margin, decimals).min(value),
        Half::Maximum => format::rounded(value + margin, decimals).max(value),
    }
}

/// The decimals `value` is written with, [`format::NUMBER_DECIMALS`] at most like the numbers
/// of the stats: `0` for `12`, `1` for `12.2`, `2` for `1.55`.
pub(super) fn decimals(value: f64) -> u8 {
    (0..format::NUMBER_DECIMALS)
        .find(|&decimals| (format::rounded(value, decimals) - value).abs() < 1e-6)
        .unwrap_or(format::NUMBER_DECIMALS)
}

/// The range of each number of a line, from the advanced copy.
pub(super) fn copied_ranges(line: &StatLine) -> Vec<Option<Range>> {
    line.rolls.iter().map(|roll| roll.range).collect()
}

/// The values of a line with a range, without their sign, given the range of each value.
pub(super) fn magnitudes(line: &StatLine, ranges: &[Option<Range>]) -> Vec<(f64, Range)> {
    line.rolls
        .iter()
        .zip(ranges)
        .filter_map(|(roll, range)| Some((roll.value, range.as_ref()?.magnitude()?)))
        .collect()
}

/// The value a line is searched by: the average of its numbers, as the trade site does for
/// `Adds # to # Physical Damage`; none for a line without number.
pub(super) fn searched_value(line: &StatLine) -> Option<f64> {
    average(line.rolls.iter().map(|roll| roll.value))
}

/// The range of the value of a line ([`searched_value`]), given the range of each of its
/// numbers: a number without range is fixed. None when no number has a range.
pub(super) fn value_bounds(line: &StatLine, ranges: &[Option<Range>]) -> Option<Range> {
    if ranges.iter().all(Option::is_none) {
        return None;
    }
    let signed: Vec<Range> = line
        .rolls
        .iter()
        .zip(ranges)
        .map(|(roll, range)| {
            range.map_or(
                Range {
                    min: roll.value,
                    max: roll.value,
                },
                |range| signed(range, roll.value),
            )
        })
        .collect();
    Some(Range {
        min: average(signed.iter().map(|range| range.min))?,
        max: average(signed.iter().map(|range| range.max))?,
    })
}

/// `range`, from its lowest to its highest value, with the sign of `value`: the game writes
/// the range of a negative number without its sign.
fn signed(range: Range, value: f64) -> Range {
    let (low, high) = (range.min.min(range.max), range.min.max(range.max));
    if value < 0.0 && low >= 0.0 {
        Range {
            min: -high,
            max: -low,
        }
    } else {
        Range {
            min: low,
            max: high,
        }
    }
}

fn average(values: impl Iterator<Item = f64>) -> Option<f64> {
    let (sum, count) = values.fold((0.0, 0u32), |(sum, count), value| (sum + value, count + 1));
    (count > 0).then(|| sum / f64::from(count))
}

/// The bar of `rolls`, those whose range is wider than one value ([`varying_share`]), their
/// ranges and the `origin` of their modifier; none when no roll varies.
pub(super) fn bar<'a>(rolls: Vec<(f64, Range)>, origin: Option<Origin>) -> Option<StatRoll<'a>> {
    let share = varying_share(&rolls)?;
    Some(StatRoll::Bar {
        share,
        ranges: rolls
            .into_iter()
            .map(|(_, range)| range)
            .filter(|range| range.max > range.min)
            .collect(),
        origin,
    })
}

/// Where `rolls` fall, from 0 to 1: the average of where each value falls in its range, of
/// those whose range is wider than one value; none when no roll varies.
pub(super) fn varying_share(rolls: &[(f64, Range)]) -> Option<f32> {
    let average = average(
        rolls
            .iter()
            .filter(|(_, range)| range.max > range.min)
            .map(|&(value, range)| share(value.abs(), range)),
    )?;
    #[expect(clippy::cast_possible_truncation, reason = "a share from 0 to 1")]
    Some(average as f32)
}

/// Where `value` falls in `range`, from 0 to 1; 1 for a range of one value.
fn share(value: f64, range: Range) -> f64 {
    if range.max <= range.min {
        return 1.0;
    }
    ((value - range.min) / (range.max - range.min)).clamp(0.0, 1.0)
}

/// The ranges of a stat, e.g. `40–60`, or `8–12 · 16–22` for several rolls; a range of one
/// value as one number, as the game writes it (`29(30)%` of a sanctified item).
pub(super) fn ranges_text(ranges: &[Range]) -> String {
    ranges
        .iter()
        .map(|range| {
            let (min, max) = (format::number(range.min), format::number(range.max));
            if min == max {
                min
            } else {
                format!("{min}–{max}")
            }
        })
        .collect::<Vec<_>>()
        .join(widgets::SEPARATOR)
}

#[cfg(test)]
mod tests;
