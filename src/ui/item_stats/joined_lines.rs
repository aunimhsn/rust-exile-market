//! The rows of a stat the game writes on several lines, joined into one row.

use super::model::{Roll, Row, Stat, StatText};
use crate::filters::{FilterKey, StatKey};
use crate::item::Item;
use crate::trade::stat_filters::LineStats;

/// The rows of a stat the game writes on several lines joined into one, after
/// [`searchable`](super::searchable): its text the lines joined, with the filter of its
/// searched line, and under it the roll of its lines, so that its bar never comes between them
/// (`Lorsque vous collectez une Infusion élémentaire, vous avez 23% de chances de gagner une`
/// then `Infusion élémentaire supplémentaire du même type` of Kurgal). Rows that can't be
/// joined stay apart: a stat with a bar on two of its lines (`Alternating every 5 seconds:`),
/// or a line of a Time-Lost jewel.
pub(super) fn join_stat_lines<'a>(
    item: &'a Item,
    stats: &LineStats,
    rows: Vec<Row<'a>>,
) -> Vec<Row<'a>> {
    // The line of the modifier that searches the stat of `key`, which its rows share.
    let anchor = |key: StatKey| stats.searched_line(key);
    let mut joined = Vec::with_capacity(rows.len());
    let mut group: Vec<(StatKey, Stat<'a>)> = Vec::new();
    for row in rows {
        match row {
            Row::Stat(
                stat @ Stat {
                    key: FilterKey::Stat(key),
                    ..
                },
            ) => {
                let follows = group.last().is_some_and(|(last, _)| {
                    last.modifier == key.modifier
                        && last.line + 1 == key.line
                        && anchor(*last) == anchor(key)
                });
                if !follows {
                    flush_stat_lines(item, &mut group, &mut joined, anchor);
                }
                group.push((key, stat));
            }
            row => {
                flush_stat_lines(item, &mut group, &mut joined, anchor);
                joined.push(row);
            }
        }
    }
    flush_stat_lines(item, &mut group, &mut joined, anchor);
    joined
}

/// Pushes the rows of `group`, lines of one stat that follow each other, to `rows`: one row
/// when they can be joined, otherwise each as it is.
fn flush_stat_lines<'a>(
    item: &'a Item,
    group: &mut Vec<(StatKey, Stat<'a>)>,
    rows: &mut Vec<Row<'a>>,
    anchor: impl Fn(StatKey) -> StatKey,
) {
    let stats = std::mem::take(group);
    match joined_stat(item, &stats, anchor) {
        Some(stat) => rows.push(Row::Stat(stat)),
        None => rows.extend(stats.into_iter().map(|(_, stat)| Row::Stat(stat))),
    }
}

/// The row of the lines of one stat, `stats`, in their order; none for a single line, or for
/// lines that can't be joined.
fn joined_stat<'a>(
    item: &'a Item,
    stats: &[(StatKey, Stat<'a>)],
    anchor: impl Fn(StatKey) -> StatKey,
) -> Option<Stat<'a>> {
    let [(first, _), .., (last, _)] = stats else {
        return None;
    };
    if !stats
        .iter()
        .all(|(_, stat)| matches!(stat.text, StatText::Line(_)))
    {
        return None;
    }
    let lines = item
        .modifiers()
        .get(first.modifier)?
        .lines
        .get(first.line..=last.line)?;
    let searched = anchor(*first);
    let (_, searched_stat) = stats.iter().find(|(key, _)| *key == searched)?;
    let mut rolls = stats.iter().filter_map(|(_, stat)| stat.roll.clone());
    let roll = match rolls.next() {
        Some(first) => Some(rolls.try_fold(first, joined_roll)?),
        None => None,
    };
    Some(Stat {
        key: FilterKey::Stat(searched),
        text: StatText::Lines(lines),
        filter: searched_stat.filter,
        unsearchable: searched_stat.unsearchable,
        roll,
    })
}

/// The roll under the joined lines of a stat, from the roll of a line, `roll`, and of the
/// next one: the bar of the line with a number, and the tier and the origin written under its
/// last line; none when both can't be told under one text (two bars, two tiers).
fn joined_roll<'a>(roll: Roll<'a>, next: Roll<'a>) -> Option<Roll<'a>> {
    match (roll, next) {
        (
            Roll::Affix {
                share,
                tier,
                origin,
            },
            Roll::Affix {
                share: next_share,
                tier: next_tier,
                origin: next_origin,
            },
        ) if (share.is_none() || next_share.is_none())
            && (tier.is_none() || next_tier.is_none()) =>
        {
            Some(Roll::Affix {
                share: share.or(next_share),
                tier: tier.or(next_tier),
                origin: next_origin.or(origin),
            })
        }
        // The bar of a unique and the origin under its last line.
        (
            Roll::Bar {
                share,
                ranges,
                origin: bar_origin,
            },
            Roll::Affix {
                share: None,
                tier: None,
                origin,
            },
        )
        | (
            Roll::Affix {
                share: None,
                tier: None,
                origin,
            },
            Roll::Bar {
                share,
                ranges,
                origin: bar_origin,
            },
        ) => Some(Roll::Bar {
            share,
            ranges,
            origin: origin.or(bar_origin),
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
