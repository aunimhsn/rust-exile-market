//! What the stats of the modifier lines of a unique and of an affixed item share: how a line is
//! read beyond its text, whether it starts checked, the filter of a line added by a rune or an
//! enchantment and the limit that finds comparable items, and the stats gathered by kind of
//! modifier, the ones that vary first.

use super::model::{Limit, Stat, StatFilter, StatText};
use super::rolls::{comparable_limit, decimals, searched_value};
use crate::filters::{FilterKey, Half, StatKey};
use crate::item::{Item, ModifierKind, Range, StatLine};
use crate::roll_difference::CopiedLines;
use crate::tier_rolls::TierRolls;
use crate::trade::stat_filters::LineStats;

/// How the stats read the modifier lines of the item, beyond their text.
pub(super) struct LineContext<'r> {
    /// They tell the lines better lower.
    pub(super) copied_lines: &'r CopiedLines,
    /// They tell the numbers of each line its stat counts, and the lines a DPS or a pseudo
    /// total counts.
    pub(super) line_stats: &'r LineStats,
    /// They tell the rolls of every tier of a line of a prefix or suffix.
    pub(super) tier_rolls: &'r TierRolls,
    /// The DPS and the pseudo totals checked at first, which count lines of the item.
    pub(super) checked_totals: Vec<FilterKey>,
    /// Every line starts unchecked: the item has an unrevealed modifier, which can change it,
    /// or is a rare, whose value lies in the few modifiers its buyer needs, or a waystone of any
    /// rarity, whose buyer pays for its properties, not for its monsters (both like Exiled
    /// Exchange 2).
    pub(super) starts_unchecked: bool,
}

impl LineContext<'_> {
    /// The line `key` starts checked: unchecked when a DPS or a pseudo total checked at first
    /// counts it already (the damage and the attack speed of a weapon, a line a total adds
    /// up), or on an item whose lines all start unchecked (an unrevealed modifier, a rare, a
    /// waystone).
    pub(super) fn starts_checked(&self, key: StatKey) -> bool {
        let counted = self
            .checked_totals
            .iter()
            .any(|&total| self.line_stats.counts_in(key, total));
        !counted && !self.starts_unchecked
    }

    /// The half of the range field the stat `key` fills: the maximum for a line better lower.
    pub(super) fn half(&self, key: StatKey) -> Half {
        if self.copied_lines.is_better_lower(key) {
            Half::Maximum
        } else {
            Half::Minimum
        }
    }

    /// `line`, the line `key`, with the numbers its stat counts only: not those its string
    /// writes (the 3 of `20% chance to gain Onslaught for 3 seconds`), which never vary.
    pub(super) fn counted(&self, key: StatKey, line: &StatLine) -> StatLine {
        StatLine {
            text: line.text.clone(),
            rolls: self.counted_numbers(key, &line.rolls),
        }
    }

    /// Among `items`, one for each number of the line `key`, those of the numbers its stat
    /// counts; all of them when its stat isn't known.
    pub(super) fn counted_numbers<T: Clone>(&self, key: StatKey, items: &[T]) -> Vec<T> {
        let picked = self.line_stats.numbers(key).and_then(|numbers| {
            numbers
                .iter()
                .map(|&number| items.get(number).cloned())
                .collect::<Option<Vec<T>>>()
        });
        picked.unwrap_or_else(|| items.to_vec())
    }
}

/// A line of an enchantment or a rune: a filter, since the trade site searches them, without
/// a roll; unchecked, a rune being easy to change.
pub(super) fn added_stat<'a>(
    key: StatKey,
    line: &'a StatLine,
    context: &LineContext<'_>,
) -> Stat<'a> {
    let half = context.half(key);
    let counted = context.counted(key, line);
    Stat {
        key: FilterKey::Stat(key),
        text: StatText::line(&line.text),
        filter: Some(StatFilter {
            checked: false,
            limit: searched_value(&counted).map(|value| Limit::new(value, half, None, &counted)),
        }),
        is_unsearchable: false,
        roll: None,
    }
}

/// The limit of a filter of a line of a rare, magic or normal item that varies, `line`: a
/// minimum at 90 % of the copied value, or a maximum at 110 % when `half` is the maximum,
/// within its `bounds`; none for a line without a number the trade site searches.
pub(super) fn limit_of_line(line: &StatLine, bounds: Option<Range>, half: Half) -> Option<Limit> {
    let value = searched_value(line)?;
    let written = line.rolls.iter().map(|roll| decimals(roll.value)).max();
    let limit = comparable_limit(value, written.unwrap_or(0), half);
    Some(Limit::new(value, half, bounds, line).at(limit))
}

/// The stats of the modifiers of `item` whose kind passes `keep`, made by `stat`: the
/// modifiers with a stat that `varies` first, each with its lines together.
pub(super) fn stats_of_kind<'a>(
    item: &'a Item,
    keep: impl Fn(ModifierKind) -> bool,
    stat: impl Fn(StatKey, &'a StatLine) -> Stat<'a>,
    varies: impl Fn(&StatLine, &Stat<'_>) -> bool,
) -> Vec<Stat<'a>> {
    let mut modifiers: Vec<(bool, Vec<Stat<'a>>)> = item
        .modifiers()
        .iter()
        .enumerate()
        .filter(|(_, modifier)| keep(modifier.kind))
        .map(|(index, modifier)| {
            let stats: Vec<Stat<'a>> = modifier
                .lines
                .iter()
                .enumerate()
                .map(|(line, text)| {
                    let key = StatKey {
                        modifier: index,
                        line,
                    };
                    stat(key, text)
                })
                .collect();
            let fixed = !modifier
                .lines
                .iter()
                .zip(&stats)
                .any(|(line, stat)| varies(line, stat));
            (fixed, stats)
        })
        .collect();
    modifiers.sort_by_key(|&(fixed, _)| fixed);
    modifiers.into_iter().flat_map(|(_, stats)| stats).collect()
}

/// A line with a number whose range isn't known, or is wider than one value: not `Loads
/// an additional bolt`, nor `5(5-5)% increased Movement Speed`.
pub(super) fn varies(line: &StatLine) -> bool {
    line.rolls
        .iter()
        .any(|roll| roll.range.is_none_or(|range| range.max > range.min))
}

pub(super) fn is_enchant(kind: ModifierKind) -> bool {
    matches!(kind, ModifierKind::Enchant | ModifierKind::Corruption)
}

pub(super) fn is_rune(kind: ModifierKind) -> bool {
    kind == ModifierKind::Rune
}

pub(super) fn is_implicit(kind: ModifierKind) -> bool {
    kind == ModifierKind::Implicit
}

#[cfg(test)]
mod tests;
