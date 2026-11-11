//! The modifiers of an item copied from the in-game trade written again as the advanced copy
//! writes them, with the tiers found from their values.

use super::split_lines::AddedTiers;
use super::trade_tiers::{FoundTiers, TradeItem, trade_tiers};
use super::{ModifierTiers, Tier};
use crate::game_data::lines::modifier_stats;
use crate::item::{Affix, AffixTier, Modifier, ModifierKind, PartTier, Range, TierDoubt};
use crate::language::Language;

/// The `modifiers` of `item`, rare or magic, copied from the in-game trade, read again with
/// the tiers found from their values ([`trade_tiers`]), as the advanced copy writes them: the
/// lines of a hybrid made one modifier, a prefix or a suffix, its tier,
/// and on each value its stat counts the range of that tier. A modifier that can be a prefix
/// or a suffix stays explicit, without affix nor tier, with the tier of each
/// ([`TierDoubt::EitherAffix`]); a line that adds up two modifiers is one modifier with the
/// other lines of its hybrid, without tier, with the tiers of each part
/// ([`TierDoubt::Split`]); one that no tier holds stays as it was.
#[must_use]
pub fn with_trade_tiers(item: &TradeItem<'_>, modifiers: &[Modifier]) -> Vec<Modifier> {
    let found = trade_tiers(item, modifiers);
    let (language, base_type) = (item.base.language, item.base.base_type);
    let mut read = Vec::with_capacity(modifiers.len());
    let mut next = 0;
    while let Some(modifier) = modifiers.get(next) {
        let joined = found
            .iter()
            .filter(|found| found.modifiers.start == next)
            .find_map(|found| {
                let parts = modifiers.get(found.modifiers.clone())?;
                Some((
                    found.modifiers.end,
                    read_found(language, base_type, parts, found)?,
                ))
            });
        if let Some((end, joined)) = joined {
            read.push(joined);
            next = end;
        } else {
            read.push(modifier.clone());
            next += 1;
        }
    }
    read
}

/// The modifiers `parts`, read as one modifier in `found`: of its affix with its tier; or, when
/// `found` has both affixes, explicit with the tier of each; or, for a line that adds up two
/// modifiers, with the tiers of each part, logged when they can't be written on the item
/// (`base_type`).
fn read_found(
    language: Language,
    base_type: &str,
    parts: &[Modifier],
    found: &FoundTiers,
) -> Option<Modifier> {
    if let Some(added) = &found.added {
        let [(affix, tiers)] = found.affixes.as_slice() else {
            return None;
        };
        let split = read_split(
            language,
            parts,
            found.modifiers.start,
            (*affix, tiers),
            added,
        );
        if split.is_none() {
            // Counted as two affixes by the reading, but left without tier.
            let lines: Vec<&str> = parts
                .iter()
                .flat_map(|part| &part.lines)
                .map(|line| line.text.as_str())
                .collect();
            log::warn!(
                "Game data, {base_type} copied from the in-game trade: the split of {lines:?} can't be written, its lines left without tier"
            );
        }
        return split;
    }
    match found.affixes.as_slice() {
        [(affix, tiers)] => with_tier(language, parts, *affix, tiers),
        [(Affix::Prefix, prefix), (Affix::Suffix, suffix)] => Some(Modifier {
            doubt: Some(TierDoubt::EitherAffix {
                prefix: affix_tier(language, parts, Affix::Prefix, prefix)?,
                suffix: affix_tier(language, parts, Affix::Suffix, suffix)?,
            }),
            ..joined(parts)?
        }),
        _ => None,
    }
}

/// The modifiers `parts`, the first the modifier `first` of the item, made one modifier whose
/// line `added.line` adds up the modifier `whole` (of every line of `parts`) and `added`
/// (of that line only): of their affix, or explicit without affix for a prefix and a suffix,
/// without tier, with the tiers of each part; the other lines of a hybrid `whole` with its
/// ranges over its tiers, that line without range.
fn read_split(
    language: Language,
    parts: &[Modifier],
    first: usize,
    (affix, whole): (Affix, &ModifierTiers),
    added: &AddedTiers,
) -> Option<Modifier> {
    let summed = added.line.checked_sub(first)?;
    // The index of that line among the lines of `parts`: each has one line, as the in-game
    // trade writes them, but the lines are counted.
    let line: usize = parts.iter().take(summed).map(|part| part.lines.len()).sum();
    let alone = parts.get(summed..=summed)?;
    let mut joined = joined(parts)?;
    for (index, other) in joined.lines.iter_mut().enumerate() {
        if index == line {
            continue;
        }
        // When the tiers of the hybrid don't give a range to the same values of that line,
        // it keeps its values without range, as the summed line has none.
        if let Some(ranges) = spanned_ranges(language, parts, whole, index) {
            for (roll, range) in other.rolls.iter_mut().zip(ranges) {
                roll.range = range;
            }
        }
    }
    Some(Modifier {
        kind: ModifierKind::Explicit((affix == added.affix).then_some(affix)),
        tier: None,
        doubt: Some(TierDoubt::Split {
            line,
            whole: part_tier(language, parts, affix, whole, line)?,
            added: part_tier(language, alone, added.affix, &added.tiers, 0)?,
        }),
        ..joined
    })
}

/// The tiers `tiers` may be (one, or two neighbouring ones) of the modifiers `parts` read as
/// one modifier of `affix`, with the range over these tiers of each value of their line
/// `line` its stat counts ([`spanned_ranges`]); none when the tiers don't give a range to the
/// same values of that line, whose ranges then can't be told apart.
fn part_tier(
    language: Language,
    parts: &[Modifier],
    affix: Affix,
    tiers: &ModifierTiers,
    line: usize,
) -> Option<PartTier> {
    Some(PartTier {
        affix,
        tiers: tiers.possible().map(|tier| tier.number).collect(),
        hides_number: tiers.hides_number(),
        ranges: spanned_ranges(language, parts, tiers, line)?
            .into_iter()
            .flatten()
            .collect(),
        currency: tiers.currency,
    })
}

/// For each value of the line `line` of the modifiers `parts` read as one, the range over the
/// tiers `tiers` may be ([`with_ranges`]), none for a value its stat doesn't count; none when
/// the tiers don't give a range to the same values.
fn spanned_ranges(
    language: Language,
    parts: &[Modifier],
    tiers: &ModifierTiers,
    line: usize,
) -> Option<Vec<Option<Range>>> {
    let mut ranges: Option<Vec<Option<Range>>> = None;
    for tier in tiers.possible() {
        let read = with_ranges(language, parts, tier, &tiers.counts)?;
        // The range of each value, none for a value its stat doesn't count.
        let line_ranges: Vec<Option<Range>> = read
            .lines
            .get(line)?
            .rolls
            .iter()
            .map(|roll| roll.range)
            .collect();
        ranges = Some(match ranges {
            None => line_ranges,
            Some(spanned) => spanned
                .into_iter()
                .zip(line_ranges)
                .map(|pair| match pair {
                    (Some(spanned), Some(range)) => Some(Some(spanned.spanning(range))),
                    (None, None) => Some(None),
                    _ => None,
                })
                .collect::<Option<_>>()?,
        });
    }
    ranges
}

/// The modifiers `parts` made one, with the lines of each in their order.
fn joined(parts: &[Modifier]) -> Option<Modifier> {
    Some(Modifier {
        lines: parts
            .iter()
            .flat_map(|part| part.lines.iter().cloned())
            .collect(),
        ..parts.first()?.clone()
    })
}

/// The modifiers `parts` made one modifier of `affix`, with the tier of `tiers` and the ranges
/// of that tier on its values ([`with_ranges`]); without tier when the game writes none, as
/// the advanced copy writes it.
fn with_tier(
    language: Language,
    parts: &[Modifier],
    affix: Affix,
    tiers: &ModifierTiers,
) -> Option<Modifier> {
    let tier = tiers.tier()?;
    Some(Modifier {
        kind: ModifierKind::Explicit(Some(affix)),
        tier: (!tiers.tier_unwritten).then_some(tier.number),
        ..with_ranges(language, parts, tier, &tiers.counts)?
    })
}

/// The tier of `tiers` that holds the values of the modifiers `parts`, as `affix`, with the
/// ranges [`with_ranges`] puts on their values, in their order, as the game writes them.
fn affix_tier(
    language: Language,
    parts: &[Modifier],
    affix: Affix,
    tiers: &ModifierTiers,
) -> Option<AffixTier> {
    let ranged = with_ranges(language, parts, tiers.tier()?, &tiers.counts)?;
    Some(AffixTier {
        affix,
        tier: tiers.tier()?.number,
        hides_number: tiers.hides_number(),
        // None for a fixed value, as under any modifier: "T2 Suffix", not "T2 1 Suffix".
        ranges: Range::unless_fixed(
            ranged
                .lines
                .iter()
                .flat_map(|line| &line.rolls)
                .filter_map(|roll| Some((roll.value, roll.range?))),
        ),
        lord: tiers.lord,
    })
}

/// The modifiers `parts` made one, with the ranges of `tier` on the values their stats count,
/// in their order, without their sign, `counts` giving the number of values of each line match
/// ([`ModifierTiers::counts`]). A line whose counted values aren't as many as those of its stat
/// in the tables (none for `Loads an additional bolt`, whose string gives the value) keeps them
/// without range, as does a range across zero, which needs its sign.
fn with_ranges(
    language: Language,
    parts: &[Modifier],
    tier: &Tier,
    counts: &[usize],
) -> Option<Modifier> {
    let mut joined = joined(parts)?;
    // The ranges and `counts` follow the line matches of each part, in their order.
    let mut ranges = tier.ranges.iter();
    let mut counts = counts.iter();
    let mut offset = 0;
    for part in parts {
        for line_match in modifier_stats(language, part, None) {
            let count = *counts.next()?;
            let taken: Vec<Range> = ranges.by_ref().take(count).copied().collect();
            let stat = line_match.stat.as_ref()?;
            let lines = offset + line_match.lines.start..offset + line_match.lines.end;
            let numbers = joined.lines.get_mut(lines)?.iter_mut();
            let rolls: Vec<_> = stat
                .counted(numbers.flat_map(|line| &mut line.rolls))
                .collect();
            if rolls.len() == taken.len() {
                for (roll, range) in rolls.into_iter().zip(taken) {
                    // As the game writes it: without its sign.
                    roll.range = range.magnitude();
                }
            }
        }
        offset += part.lines.len();
    }
    Some(joined)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod game_tests;
