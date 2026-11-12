//! The rows of a rare, magic or normal item: its enchantments, runes and implicits, then its
//! prefixes and suffixes with a bar over every tier of their modifier on the base, their tier
//! and the ranges of that tier.

use super::model::{Group, ModifierTier, Origin, Row, Stat, StatRoll, StatText, TierRow};
use super::modifier_stats::{
    LineContext, added_stat, comparable_filter, is_enchant, is_implicit, is_rune, stats_of_kind,
    varies,
};
use super::push_group;
use super::rolls::{bar, copied_ranges, magnitudes, value_bounds, varying_share};
use super::split_lines::{hybrid_tier_row, other_lines, split_tier_rows};
use crate::filters::{FilterKey, StatKey};
use crate::item::{Affix, Item, Modifier, ModifierKind, Range, StatLine, TierDoubt};
use crate::tier_rolls::LineRolls;

/// The rows of a rare, magic or normal item: its enchantments, its runes, its implicit
/// modifiers, then its prefixes and suffixes.
pub(super) fn affix_rows<'a>(item: &'a Item, context: &LineContext<'_>) -> Vec<Row<'a>> {
    // A rune or an implicit can add affixes or take some away (`+1 Suffix Modifier allowed`).
    let limits = context.line_stats.affix_limits(item);
    let max = |limit: u32| (!limits.counted_only).then_some(limit);
    let line_varies = |line: &StatLine, _: &Stat<'_>| varies(line);
    let prefixes = affix_stats(item, Some(Affix::Prefix), context);
    let suffixes = affix_stats(item, Some(Affix::Suffix), context);
    let filled = |affix| {
        item.modifiers()
            .iter()
            .map(|modifier| affix_count(modifier, affix))
            .sum()
    };
    let mut rows = Vec::new();
    push_group(
        &mut rows,
        Group::Enchant,
        stats_of_kind(
            item,
            is_enchant,
            |key, line| added_stat(key, line, context),
            line_varies,
        ),
    );
    push_group(
        &mut rows,
        Group::Runes,
        stats_of_kind(
            item,
            is_rune,
            |key, line| added_stat(key, line, context),
            line_varies,
        ),
    );
    push_group(
        &mut rows,
        Group::Implicit,
        stats_of_kind(
            item,
            is_implicit,
            |key, line| implicit_stat(key, line, context),
            line_varies,
        ),
    );
    push_group(
        &mut rows,
        Group::Prefixes {
            filled: filled(Affix::Prefix),
            max: max(limits.prefixes),
        },
        prefixes,
    );
    push_group(
        &mut rows,
        Group::Suffixes {
            filled: filled(Affix::Suffix),
            max: max(limits.suffixes),
        },
        suffixes,
    );
    push_group(&mut rows, Group::Explicit, affix_stats(item, None, context));
    rows
}

/// The prefixes or suffixes of `affix` that `modifier` counts in the room of its item: one of
/// its affix, two for a line that adds up two modifiers of that affix, one for a line that adds
/// up a prefix and a suffix; none for a modifier whose affix is unknown.
fn affix_count(modifier: &Modifier, affix: Affix) -> u32 {
    match (&modifier.doubt, modifier.kind) {
        (Some(TierDoubt::Split { whole, added, .. }), _) => {
            u32::from(whole.affix == affix) + u32::from(added.affix == affix)
        }
        (_, ModifierKind::Explicit(Some(kind))) => u32::from(kind == affix),
        _ => 0,
    }
}

/// The explicit modifiers of `affix` (`None`: a header that doesn't tell it), those with a
/// line that varies first and those not revealed yet last: each line with its bar over every
/// tier of its modifier, and its tier and the ranges of that tier under its last line; the
/// tier of each affix for a modifier copied from the in-game trade that can be either, and
/// the tier of each part under a line that adds up two modifiers.
fn affix_stats<'a>(
    item: &'a Item,
    affix: Option<Affix>,
    context: &LineContext<'_>,
) -> Vec<Stat<'a>> {
    sorted_modifiers(item, affix, context)
        .into_iter()
        .flat_map(|(index, modifier)| modifier_stats(index, modifier, context))
        .collect()
}

/// The explicit modifiers of `affix` with their indexes, those with a line that varies first
/// and those not revealed yet last.
fn sorted_modifiers<'a>(
    item: &'a Item,
    affix: Option<Affix>,
    context: &LineContext<'_>,
) -> Vec<(usize, &'a Modifier)> {
    let mut modifiers: Vec<(usize, &Modifier)> = item
        .modifiers()
        .iter()
        .enumerate()
        .filter(|(_, modifier)| modifier.kind == ModifierKind::Explicit(affix))
        .collect();
    let line_varies = |modifier: usize, (line, text): (usize, &StatLine)| {
        let key = StatKey { modifier, line };
        let counted = context.counted(key, text);
        affix_varies(&counted, rolls_over_tiers(key, context).as_ref())
    };
    modifiers.sort_by_key(|&(index, modifier)| {
        let fixed = !modifier
            .lines
            .iter()
            .enumerate()
            .any(|line| line_varies(index, line));
        (modifier.is_unrevealed, fixed)
    });
    modifiers
}

/// The stats of the lines of the prefix or suffix `index`, `modifier`: its tier under its last
/// line, or the tier of the hybrid under the last of its other lines for a line that adds up
/// two modifiers, whose origin goes under the tiers of that line; the tiers of each affix or
/// of each part in place of the roll of their line; one stat for a modifier not revealed yet.
fn modifier_stats<'a>(
    index: usize,
    modifier: &'a Modifier,
    context: &LineContext<'_>,
) -> Vec<Stat<'a>> {
    if modifier.is_unrevealed
        && let ModifierKind::Explicit(Some(affix)) = modifier.kind
    {
        return vec![unrevealed_stat(index, affix)];
    }
    let last = modifier.lines.len().saturating_sub(1);
    let origin = Origin::of_modifier(modifier, context.tier_rolls.currency(index));
    let (origin_line, tier_line) = match &modifier.doubt {
        Some(TierDoubt::Split { line, .. }) => (
            *line,
            other_lines(modifier, *line)
                .next_back()
                .map(|(index, _)| index),
        ),
        _ => (last, Some(last)),
    };
    modifier
        .lines
        .iter()
        .enumerate()
        .map(|(line_index, line)| {
            let key = StatKey {
                modifier: index,
                line: line_index,
            };
            let tier = (tier_line == Some(line_index))
                .then(|| modifier_tier_row(index, modifier, context))
                .flatten();
            let origin = origin.filter(|_| line_index == origin_line);
            let mut stat = affix_stat(key, line, tier, origin, context);
            if let Some(rows) = doubt_tier_rows(modifier, line_index) {
                stat.roll = Some(StatRoll::Tiers { rows, origin });
            }
            stat
        })
        .collect()
}

/// A modifier of `affix` not revealed yet, the modifier `index`: no line yet, so the key of its
/// first one, which no filter has.
fn unrevealed_stat<'a>(index: usize, affix: Affix) -> Stat<'a> {
    Stat {
        key: FilterKey::Stat(StatKey {
            modifier: index,
            line: 0,
        }),
        text: StatText::Unrevealed(affix),
        filter: None,
        is_unsearchable: false,
        roll: None,
    }
}

/// The tier under the line of the prefix or suffix `index`, `modifier`, that holds it: the tier
/// of the hybrid of a line that adds up two modifiers; else its tier and the ranges of that
/// tier, or of a modifier of one tier only the ranges without its number, also when the copy
/// writes no tier; nothing when its value is fixed.
fn modifier_tier_row<'a>(
    index: usize,
    modifier: &'a Modifier,
    context: &LineContext<'_>,
) -> Option<TierRow<'a>> {
    if let Some(TierDoubt::Split {
        line, whole, added, ..
    }) = &modifier.doubt
    {
        return hybrid_tier_row(modifier, *line, whole, added);
    }
    let hides_number = context.tier_rolls.hides_number(index);
    let number = modifier.tier.filter(|_| !hides_number);
    let ranges = tier_ranges(modifier);
    (number.is_some() || (hides_number && !ranges.is_empty()))
        .then(|| TierRow::plain(ModifierTier { number, ranges }))
}

/// The tiers in place of the roll of the line `line_index` of `modifier`, when its tier is in
/// doubt: the tier of each affix under the last line of a modifier copied from the in-game
/// trade that can be either, the tier of each part under a line that adds up two modifiers.
fn doubt_tier_rows(modifier: &Modifier, line_index: usize) -> Option<Vec<TierRow<'_>>> {
    match &modifier.doubt {
        Some(TierDoubt::EitherAffix { prefix, suffix })
            if line_index + 1 == modifier.lines.len() =>
        {
            Some(vec![TierRow::affix(prefix), TierRow::affix(suffix)])
        }
        Some(TierDoubt::Split {
            line, whole, added, ..
        }) if line_index == *line => {
            let others: Vec<&str> = other_lines(modifier, *line)
                .map(|(_, other)| other.text.as_str())
                .collect();
            Some(split_tier_rows(whole, added, &others))
        }
        _ => None,
    }
}

/// The line `key` of a prefix or suffix, `line`: a filter that stays within the rolls of every
/// tier of its modifier, its bar over them, and under its last line the tier of the modifier
/// and the ranges of that tier, then its origin.
fn affix_stat<'a>(
    key: StatKey,
    line: &'a StatLine,
    tier: Option<TierRow<'a>>,
    origin: Option<Origin>,
    context: &LineContext<'_>,
) -> Stat<'a> {
    let counted = context.counted(key, line);
    let over = rolls_over_tiers(key, context);
    let mut filter = comparable_filter(
        &counted,
        over.as_ref()
            .and_then(|over| value_bounds(&counted, &over.ranges)),
        affix_varies(&counted, over.as_ref()),
        context.half(key),
        context.starts_checked(key),
    );
    if let Some(limit) = &mut filter.limit {
        limit.tiers = over
            .as_ref()
            .and_then(|over| over.tiers)
            .filter(|_| limit.rolls.is_some());
    }
    let share = over
        .as_ref()
        .and_then(|over| varying_share(&magnitudes(&counted, &over.ranges)));
    Stat {
        key: FilterKey::Stat(key),
        text: StatText::line(&line.text),
        filter: Some(filter),
        is_unsearchable: false,
        roll: (share.is_some() || tier.is_some() || origin.is_some()).then_some(StatRoll::Affix {
            share,
            tier: tier.map(Box::new),
            origin,
        }),
    }
}

/// A line of an implicit modifier of a rare, magic or normal item: a filter, rolled in the
/// range its base gives it.
fn implicit_stat<'a>(key: StatKey, line: &'a StatLine, context: &LineContext<'_>) -> Stat<'a> {
    let counted = context.counted(key, line);
    let ranges = copied_ranges(&counted);
    Stat {
        key: FilterKey::Stat(key),
        text: StatText::line(&line.text),
        filter: Some(comparable_filter(
            &counted,
            value_bounds(&counted, &ranges),
            varies(&counted),
            context.half(key),
            context.starts_checked(key),
        )),
        is_unsearchable: false,
        roll: bar(magnitudes(&counted, &ranges), None),
    }
}

/// The rolls over every tier of the line `key`, for the numbers its stat counts; none when
/// the modifier tables don't know them.
fn rolls_over_tiers(key: StatKey, context: &LineContext<'_>) -> Option<LineRolls> {
    let rolls = context.tier_rolls.get(key)?;
    let ranges = context.counted_numbers(key, &rolls.ranges);
    ranges.iter().any(Option::is_some).then_some(LineRolls {
        ranges,
        tiers: rolls.tiers,
    })
}

/// A line of a prefix or suffix varies over the tiers of its modifier when they are known (a
/// line fixed in its tier, `+3 to Level of all Projectile Skills` of tier 2, varies when the
/// other tiers roll other values), or else in its range.
fn affix_varies(counted: &StatLine, over: Option<&LineRolls>) -> bool {
    over.map_or_else(
        || varies(counted),
        |over| {
            over.ranges
                .iter()
                .flatten()
                .any(|range| range.max > range.min)
        },
    )
}

/// The ranges written under the last line of a prefix or suffix, those the copy writes, in their
/// order; none for a fixed value ([`Range::unless_fixed`]: `50(50)% reduced Amount Recovered`, or
/// the range the tables put on a line of the in-game trade, `+2 to Level of all Minion Skills`).
fn tier_ranges(modifier: &Modifier) -> Vec<Range> {
    Range::unless_fixed(
        modifier
            .lines
            .iter()
            .flat_map(|line| magnitudes(line, &copied_ranges(line))),
    )
}

#[cfg(test)]
mod tests;
