//! The lines of a copied modifier as the tables are searched with them: their templates and
//! values, the ranges and the tier the advanced copy wrote, and how well a tier holds them.

use super::Tier;
use super::tables::{Table, TableLine};
use crate::game_data::lines::{LineMatch, StatSource};
use crate::game_data::stats::Stats;
use crate::item::{Modifier, ModifierKind, ModifierOrigin, Range};

/// How far a copied value can be from a tier: the game rounds the values it scales, e.g. a
/// regeneration stored per minute shown per second with one decimal.
pub(super) const ROUNDING: f64 = 0.051;

/// The lines of a copied modifier, and the ranges of its tier when the advanced copy gave them.
#[cfg_attr(test, derive(Debug, PartialEq))]
pub(super) struct Copied<'a> {
    pub(super) lines: Vec<CopiedLine<'a>>,
    /// The ranges the advanced copy gave for each value of the lines, in their order, as the
    /// game writes them: without their sign.
    pub(super) ranges: Option<Vec<Range>>,
    /// The tier the advanced copy wrote.
    pub(super) tier: CopiedTier,
    /// The origin the game marks (`Desecrated Prefix Modifier`, ` (desecrated)`, ` (crafted)`):
    /// only a desecrated modifier may be one of the desecrated tables, which the game always
    /// marks (a desecrated modifier can't be fractured, owner's answer on 2026-10-30), while a
    /// desecration also reveals ordinary modifiers ([`Table::may_hold`]).
    pub(super) origin: Option<ModifierOrigin>,
}

impl<'a> Copied<'a> {
    /// The lines of `modifier` as the tables are searched with them, each matched with its
    /// stat in `matches`; none when a line has no stat, or one the trade site alone knows,
    /// which has no template of the tables.
    pub(super) fn of(modifier: &Modifier, matches: &[LineMatch<'a>]) -> Option<Self> {
        let mut lines = Vec::new();
        let mut ranges = Vec::new();
        for line_match in matches {
            let stat = line_match.stat.as_ref()?;
            // A line the trade site only knows has no template of Exiled Exchange 2.
            if stat.source != StatSource::GameData {
                return None;
            }
            lines.push(CopiedLine {
                templates: Stats::english_references(stat.stat),
                values: stat.values.clone(),
                written: stat.matcher_value.filter(|_| stat.values.is_empty()),
            });
            let rolls = modifier
                .lines
                .get(line_match.lines.clone())?
                .iter()
                .flat_map(|line| &line.rolls);
            ranges.extend(stat.counted(rolls).filter_map(|roll| roll.range));
        }
        // A range for each value, or none: the modifier is then found by its values only. A
        // value without its range leaves fewer ranges than values, and so does a line without
        // number (`Loads an additional bolt`), for which the advanced copy writes none: it
        // counts the value its string gives.
        let count: usize = lines.iter().map(CopiedLine::count).sum();
        let ranges = (ranges.len() == count).then_some(ranges);
        Some(Self {
            lines,
            ranges,
            tier: match modifier.tier {
                Some(number) => CopiedTier::Written(number),
                None if matches!(modifier.kind, ModifierKind::Explicit(Some(_))) => {
                    CopiedTier::Unwritten
                }
                None => CopiedTier::Unknown,
            },
            origin: modifier.origin,
        })
    }

    /// How many values each line has, in their order.
    pub(super) fn counts(&self) -> Vec<usize> {
        self.lines.iter().map(CopiedLine::count).collect()
    }

    /// How many values the lines before the line `line` have: where its values start among
    /// those of every line.
    pub(super) fn values_before(&self, line: usize) -> usize {
        self.lines.iter().take(line).map(CopiedLine::count).sum()
    }

    /// Whether the modifier may be one of `table` ([`Table::may_hold`]).
    pub(super) fn may_be(&self, table: &Table) -> bool {
        table.may_hold(self.origin)
    }

    /// For each copied line, the index of the line of the same template in `lines`, the lines
    /// of a table: its lines in another order are the same modifier. None when the lines
    /// differ.
    pub(super) fn order(&self, lines: &[TableLine]) -> Option<Vec<usize>> {
        if lines.len() != self.lines.len() {
            return None;
        }
        let mut used = vec![false; lines.len()];
        self.lines
            .iter()
            .map(|copied| {
                let index = lines
                    .iter()
                    .zip(&used)
                    .position(|(line, &used)| !used && copied.is(&line.template))?;
                *used.get_mut(index)? = true;
                Some(index)
            })
            .collect()
    }

    /// The values of the copied lines, in their order, as the table of `lines` shows them: the
    /// value a line without number gives, divided as its table line says.
    pub(super) fn values(&self, lines: &[TableLine], order: &[usize]) -> Vec<f64> {
        let mut values = Vec::new();
        for (copied, &index) in self.lines.iter().zip(order) {
            values.extend(&copied.values);
            if let Some(written) = copied.written {
                let divisor = lines.get(index).map_or(1.0, |line| line.divisor);
                values.push(written / divisor);
            }
        }
        values
    }

    /// How well `tier` holds the copied `values`; none when its ranges aren't those the advanced
    /// copy gave, or when it doesn't hold them and the copy gave no range.
    pub(super) fn fit(&self, values: &[f64], tier: &Tier) -> Option<Fit> {
        if values.len() != tier.ranges.len() {
            return None;
        }
        if let Some(ranges) = &self.ranges {
            let same = ranges.iter().zip(&tier.ranges).all(|(copied, range)| {
                range.magnitude().is_some_and(|range| {
                    (copied.min - range.min).abs() < ROUNDING
                        && (copied.max - range.max).abs() < ROUNDING
                })
            });
            if !same {
                return None;
            }
        }
        holds(values, &tier.ranges).or_else(|| self.ranges.as_ref().map(|_| Fit::Written))
    }
}

/// The tier the advanced copy wrote in the header of a copied modifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CopiedTier {
    /// Its number (`(Tier: 2)`): of two tiers that hold the modifier as well, that one.
    Written(u32),
    /// None (`of the Essence`): of two tiers that hold it as well, one of a modifier whose tier
    /// the game doesn't write.
    Unwritten,
    /// No header: a line of the in-game trade, or several joined.
    Unknown,
}

/// A copied line of a modifier, the lines of a stat written on several lines counting as one.
#[derive(Clone)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub(super) struct CopiedLine<'a> {
    /// The English templates of its stat ([`Stats::english_references`]): one, but for a line
    /// of another language that is the line of two English stats.
    pub(super) templates: Vec<&'a str>,
    /// The values of its `#`, with the sign of the stat.
    pub(super) values: Vec<f64>,
    /// The value its string gives, for a line without `#` (`Loads an additional bolt` is 1),
    /// as the stat stores it.
    pub(super) written: Option<f64>,
}

impl CopiedLine<'_> {
    /// Whether the line is one of the template `template` of a table.
    pub(super) fn is(&self, template: &str) -> bool {
        self.templates.contains(&template)
    }

    /// How many values the line has.
    pub(super) fn count(&self) -> usize {
        if self.values.is_empty() {
            usize::from(self.written.is_some())
        } else {
            self.values.len()
        }
    }
}

/// How well a tier holds the copied modifier, the best first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Fit {
    /// Its values are within the ranges of the tier.
    Within,
    /// Its values are within the ranges of the tier, give or take the rounding of the game.
    Rounded,
    /// Its values are out of the ranges of the tier, which are those the advanced copy wrote:
    /// any value that left its tier, e.g. on a sanctified item (`+74(80-89) to maximum Mana`).
    Written,
}

/// How well `ranges` hold `values`: within them, or give or take the rounding of the game;
/// none otherwise.
pub(super) fn holds(values: &[f64], ranges: &[Range]) -> Option<Fit> {
    let within = |margin: f64| {
        values
            .iter()
            .zip(ranges)
            .all(|(&value, range)| range.min - margin <= value && value <= range.max + margin)
    };
    if within(0.0) {
        Some(Fit::Within)
    } else if within(ROUNDING) {
        Some(Fit::Rounded)
    } else {
        None
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod game_tests;
