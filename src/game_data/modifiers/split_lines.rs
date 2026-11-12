//! A line that adds up two modifiers of its stat, which the game writes as one: the ways a
//! line copied from the in-game trade splits, and the tiers of the modifier of a line read so.

use super::copied::{Copied, CopiedTier, Fit, holds};
use super::item_tables::{Held, ItemTables};
use super::tables::TableAffix;
use super::{ModifierTiers, Tier};
use crate::game_data::lines::LineMatch;
use crate::item::{Affix, Modifier, PartTier, Range};

/// The other modifier of a line copied from the in-game trade that adds up two modifiers of
/// its stat, a pure one, found from the value of that line.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct AddedTiers {
    /// The index among the modifiers of the item of the line.
    pub(super) line: usize,
    pub(super) affix: Affix,
    /// Its tiers on the base, the copied one that holds its part of the value, the ranges of
    /// a tier following the values of the line.
    pub(super) tiers: ModifierTiers,
}

/// A way a copied line adds up two modifiers of its stat: the modifier of every copied line,
/// with the summed one, pure or a hybrid, of `affix` with its `tiers`, and a pure modifier of
/// another type of the summed line only.
#[derive(Clone)]
pub(super) struct Split {
    /// The indexes among the tables of both modifiers, the summed one first.
    tables: (usize, usize),
    pub(super) affix: Affix,
    pub(super) tiers: ModifierTiers,
    pub(super) added: AddedTiers,
    fit: Fit,
}

/// Every way the line `summed` of `copied` (the lines of consecutive modifiers), the line of
/// the modifier `owner` of the item, adds up two modifiers of its stat among `tables`:
/// the modifier of every copied line, and a pure modifier of another type of the summed line
/// only, whose values add up on that line, their tiers among those the level of the item
/// allows. Only the ways that fit best, those of two neighbouring tiers of one modifier made
/// one ([`neighbours_joined`]); none for a line without number. Of two pure modifiers, the
/// prefix is the summed one, as [`crate::item::TierDoubt::EitherAffix`] reads a prefix first.
pub(super) fn splits(
    tables: &ItemTables<'_>,
    copied: &Copied<'_>,
    (summed, owner): (usize, usize),
) -> Vec<Split> {
    let Some(line) = copied
        .lines
        .get(summed)
        .filter(|line| !line.values.is_empty())
    else {
        return Vec::new();
    };
    let alone = Copied {
        lines: vec![line.clone()],
        ranges: None,
        tier: CopiedTier::Unknown,
        origin: copied.origin,
    };
    // The values of the summed line among those of every line, in their order.
    let offset = copied.values_before(summed);
    let summed_values = offset..offset + line.count();
    // The pure modifiers of the summed line.
    let pure: Vec<Held<'_>> = tables.holding(&alone).collect();
    // Of two pure modifiers, the prefix first.
    let rank = |held: &Held<'_>| (held.table.affix, held.index);
    let mut ways = Vec::new();
    for whole in tables.holding(copied) {
        for added in &pure {
            // Each pair of pure modifiers once, the prefix first.
            if added.table.kind == whole.table.kind
                || (whole.table.lines.len() == 1 && rank(added) <= rank(&whole))
            {
                continue;
            }
            for (found, whole_tier) in whole.tiers.iter().enumerate() {
                for (added_found, added_tier) in added.tiers.iter().enumerate() {
                    if !tables.allows(whole_tier) || !tables.allows(added_tier) {
                        continue;
                    }
                    let fit = split_fit(&whole.values, whole_tier, added_tier, &summed_values);
                    let Some(fit) = fit else {
                        continue;
                    };
                    ways.push(Split {
                        tables: (whole.index, added.index),
                        affix: whole.table.affix.into(),
                        tiers: whole.clone().into_tiers(found, copied),
                        added: AddedTiers {
                            line: owner,
                            affix: added.table.affix.into(),
                            tiers: added.clone().into_tiers(added_found, &alone),
                        },
                        fit,
                    });
                }
            }
        }
    }
    let best = ways.iter().map(|way| way.fit).min();
    ways.retain(|way| Some(way.fit) == best);
    neighbours_joined(&ways)
}

/// The `ways` a line adds up two modifiers, those of the same two modifiers following each
/// other, with the two ways of a pair of modifiers that differ only by the tier of one of them,
/// two neighbouring tiers, made one (the pure T1 or T2 of the `248% increased Physical Damage`
/// of a bow, with the hybrid T1 on `+195 to Accuracy Rating`). Two ways that differ by both
/// tiers (the `30% increased Rarity of Items found` of a tiara, a prefix T1 and a suffix T2, or a
/// T2 and a T1), or more ways of a pair (its `33%`, also both T1), stay apart: a modifier in three
/// tiers leaves its line without tier.
fn neighbours_joined(ways: &[Split]) -> Vec<Split> {
    let mut joined = Vec::new();
    for pair in ways.chunk_by(|one, other| one.tables == other.tables) {
        match pair {
            [one, other] => match one.with_neighbour(other) {
                Some(way) => joined.push(way),
                None => joined.extend([one.clone(), other.clone()]),
            },
            _ => joined.extend(pair.iter().cloned()),
        }
    }
    joined
}

impl Split {
    /// The way this one and `other`, of the same modifiers, make when they differ only by the
    /// tier of one modifier, two neighbouring tiers.
    fn with_neighbour(&self, other: &Self) -> Option<Self> {
        let (whole, added) = (&self.tiers, &self.added.tiers);
        let (other_whole, other_added) = (&other.tiers, &other.added.tiers);
        if whole.found == other_whole.found && added.found.abs_diff(other_added.found) == 1 {
            Some(Self {
                added: AddedTiers {
                    tiers: added.or_tier(other_added.found),
                    ..self.added.clone()
                },
                ..self.clone()
            })
        } else if added.found == other_added.found && whole.found.abs_diff(other_whole.found) == 1 {
            Some(Self {
                tiers: whole.or_tier(other_whole.found),
                ..self.clone()
            })
        } else {
            None
        }
    }
}

/// How well the tier `whole` of the modifier of every copied line, with the tier `added` of
/// another on the summed line, hold the copied `values`: those in `summed`, of the summed
/// line, within the sum of the ranges of both.
fn split_fit(
    values: &[f64],
    whole: &Tier,
    added: &Tier,
    summed: &std::ops::Range<usize>,
) -> Option<Fit> {
    if values.len() != whole.ranges.len() || added.ranges.len() != summed.len() {
        return None;
    }
    let ranges: Vec<Range> = whole
        .ranges
        .iter()
        .enumerate()
        .map(|(index, range)| {
            let added = index
                .checked_sub(summed.start)
                .filter(|_| summed.contains(&index))
                .and_then(|index| added.ranges.get(index));
            added.map_or(*range, |added| Range {
                min: range.min + added.min,
                max: range.max + added.max,
            })
        })
        .collect();
    holds(values, &ranges)
}

/// The tiers among `tables` of the modifier of every line of `modifier`, whose line `line`
/// adds up its part `whole` and a pure modifier ([`hybrid_tiers`]).
pub(super) fn split_tiers(
    tables: &ItemTables<'_>,
    modifier: &Modifier,
    matches: &[LineMatch<'_>],
    (line, whole): (usize, &PartTier),
) -> Option<ModifierTiers> {
    let copied = Copied::of(modifier, matches)?;
    let summed = matches
        .iter()
        .position(|found| found.lines == (line..line + 1))?;
    hybrid_tiers(tables, copied, (summed, whole))
}

/// The tiers among `tables` of the modifier of every line of `copied`, whose line `summed`
/// adds up its part `whole` and a pure modifier: those of the table whose tiers of the numbers
/// of `whole`, among those the level of the item allows, give that line the ranges of `whole`
/// and hold best the values of its other lines, that line taken at the lowest roll of its
/// part, the sum being in no tier; the worse found and the better its neighbour. As [`splits`]
/// takes the ways that fit best: another table holding them as well would have been a second
/// way to split the line, which leaves it without tier; the first of the file on a tie, as
/// `find` takes.
fn hybrid_tiers(
    tables: &ItemTables<'_>,
    mut copied: Copied<'_>,
    (summed, whole): (usize, &PartTier),
) -> Option<ModifierTiers> {
    let affix = match whole.affix {
        Affix::Prefix => TableAffix::Prefix,
        Affix::Suffix => TableAffix::Suffix,
    };
    // The values of that line among those of every line, in their order.
    let offset = copied.values_before(summed);
    let values = &mut copied.lines.get_mut(summed)?.values;
    if values.len() != whole.ranges.len() {
        return None;
    }
    for (value, range) in values.iter_mut().zip(&whole.ranges) {
        // The ranges as the game writes them, the values with the sign of the stat.
        *value = range.min.copysign(*value);
    }
    let mut best: Option<(Fit, ModifierTiers)> = None;
    for held in tables.holding_as(&copied, affix) {
        let Some((worse, better)) = part_in(tables, &held.tiers, whole, offset) else {
            continue;
        };
        let fit = [Some(worse), better]
            .into_iter()
            .flatten()
            .filter_map(|tier| holds(&held.values, &held.tiers.get(tier)?.ranges))
            .min();
        if let Some(fit) = fit
            && best.as_ref().is_none_or(|(best, _)| fit < *best)
        {
            let tiers = ModifierTiers {
                neighbour: better,
                ..held.into_tiers(worse, &copied)
            };
            best = Some((fit, tiers));
        }
    }
    best.map(|(_, tiers)| tiers)
}

/// The indexes in `tiers` (the best first) of the tiers of `whole`, the part of a line from
/// the value `offset` among those of a tier, the worse and the better one: none when one of them
/// isn't among `tiers`, the level of the item of `tables` doesn't allow it, or the tiers don't
/// give that line the ranges of `whole`.
fn part_in(
    tables: &ItemTables<'_>,
    tiers: &[Tier],
    whole: &PartTier,
    offset: usize,
) -> Option<(usize, Option<usize>)> {
    let index = |number: &u32| {
        let (index, tier) = tiers
            .iter()
            .enumerate()
            .find(|(_, tier)| tier.number == *number)?;
        tables.allows(tier).then_some(index)
    };
    let worse = index(whole.tiers.last()?)?;
    let better = match whole.tiers.as_slice() {
        [better, _] => Some(index(better)?),
        _ => None,
    };
    for (position, expected) in whole.ranges.iter().enumerate() {
        let mut spanned: Option<Range> = None;
        for tier in [Some(worse), better].into_iter().flatten() {
            let range = tiers
                .get(tier)?
                .ranges
                .get(offset + position)?
                .magnitude()?;
            spanned = Some(spanned.map_or(range, |spanned| spanned.spanning(range)));
        }
        if spanned != Some(*expected) {
            return None;
        }
    }
    Some((worse, better))
}

#[cfg(test)]
mod tests;
