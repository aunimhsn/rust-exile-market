//! A line that adds up two modifiers of its stat, which the game writes as one: the ways a
//! line copied from the in-game trade splits, and the tiers of the modifier of a line read so.

use super::base_tiers::tiers_on_base;
use super::copied::{Copied, CopiedLine, CopiedTier, Fit, holds};
use super::tables::{TABLES, Table, TableAffix, first_of_its_modifier};
use super::{ModifierTiers, Tier};
use crate::game_data::bases::{self, BaseSpawn};
use crate::game_data::lines::LineMatch;
use crate::item::{Affix, Lord, Modifier, PartTier, Range};
use crate::language::Language;

/// The other modifier of a line copied from the in-game trade that adds up two modifiers of
/// its stat, a pure one, found from the value of that line.
#[derive(Debug, Clone, PartialEq)]
pub struct AddedTiers {
    /// The index among the modifiers of the item of the line.
    pub line: usize,
    pub affix: Affix,
    /// Its tiers on the base, the copied one that holds its part of the value, the ranges of
    /// a tier following the values of the line.
    pub tiers: ModifierTiers,
}

/// A way a copied line adds up two modifiers of its stat: the modifier of every copied line,
/// with the summed one, pure or a hybrid, of `affix` with its `tiers`, and a pure modifier of
/// another type of the summed line only.
#[derive(Clone)]
pub(super) struct Split {
    /// The indexes among the tables of both modifiers, the summed one first.
    pub(super) tables: (usize, usize),
    pub(super) affix: Affix,
    pub(super) tiers: ModifierTiers,
    pub(super) added: AddedTiers,
    pub(super) fit: Fit,
}

/// Every way the line `summed` of `copied` (the lines of consecutive modifiers), the line of
/// the modifier `owner` of the item, adds up two modifiers of its stat on `base`:
/// the modifier of every copied line, and a pure modifier of another type of the summed line
/// only, whose values add up on that line, their tiers among those `item_level` allows. Only
/// the ways that fit best, those of two neighbouring tiers of one modifier made one
/// ([`neighbours_joined`]); none for a line without number. Of two pure modifiers, the prefix
/// is the summed one, as [`crate::item::TierDoubt::EitherAffix`] reads a prefix first.
pub(super) fn splits(
    tables: &[Table],
    base: &BaseSpawn,
    copied: &Copied<'_>,
    (summed, owner): (usize, usize),
    item_level: Option<u32>,
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
    let offset: usize = copied
        .lines
        .iter()
        .take(summed)
        .map(CopiedLine::count)
        .sum();
    let summed_values = offset..offset + line.count();
    let allowed = |tier: &Tier| item_level.is_none_or(|level| tier.level <= level);
    let counts: Vec<usize> = copied.lines.iter().map(CopiedLine::count).collect();
    // The pure modifiers of the summed line.
    let pure: Vec<_> = tables
        .iter()
        .enumerate()
        .filter(|(_, table)| alone.may_be(table))
        .filter_map(|(index, table)| {
            let order = alone.order(&table.lines)?;
            let tiers = tiers_on_base(tables, table, base, &order, &alone)?;
            first_of_its_modifier(tables, index).then_some((index, table, tiers))
        })
        .collect();
    // Of two pure modifiers, the prefix first.
    let rank = |table: &Table, index: usize| (table.affix, index);
    let mut ways = Vec::new();
    for (whole_index, whole) in tables.iter().enumerate() {
        if !copied.may_be(whole) {
            continue;
        }
        let Some(order) = copied.order(&whole.lines) else {
            continue;
        };
        if !first_of_its_modifier(tables, whole_index) {
            continue;
        }
        let Some(whole_tiers) = tiers_on_base(tables, whole, base, &order, copied) else {
            continue;
        };
        let values = copied.values(&whole.lines, &order);
        for (added_index, added, added_tiers) in &pure {
            // Each pair of pure modifiers once, the prefix first.
            if added.kind == whole.kind
                || (whole.lines.len() == 1 && rank(added, *added_index) <= rank(whole, whole_index))
            {
                continue;
            }
            for (found, whole_tier) in whole_tiers.iter().enumerate() {
                for (added_found, added_tier) in added_tiers.iter().enumerate() {
                    if !allowed(whole_tier) || !allowed(added_tier) {
                        continue;
                    }
                    let Some(fit) = split_fit(&values, whole_tier, added_tier, &summed_values)
                    else {
                        continue;
                    };
                    ways.push(Split {
                        tables: (whole_index, *added_index),
                        affix: whole.affix.into(),
                        tiers: ModifierTiers {
                            tiers: whole_tiers.clone(),
                            found,
                            neighbour: None,
                            counts: counts.clone(),
                            tier_unwritten: whole.tier_unwritten(),
                            currency: whole.currency(),
                            lord: whole.lord.map(Lord::from),
                        },
                        added: AddedTiers {
                            line: owner,
                            affix: added.affix.into(),
                            tiers: ModifierTiers {
                                tiers: added_tiers.clone(),
                                found: added_found,
                                neighbour: None,
                                counts: vec![line.count()],
                                tier_unwritten: added.tier_unwritten(),
                                currency: added.currency(),
                                lord: added.lord.map(Lord::from),
                            },
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

/// The tiers of the modifier of every line of `modifier`, whose line `line` adds up its part
/// `whole` and a pure modifier, among those `item_level` allows ([`hybrid_tiers`]).
pub(super) fn split_tiers(
    language: Language,
    base_type: &str,
    item_level: Option<u32>,
    modifier: &Modifier,
    matches: &[LineMatch<'_>],
    (line, whole): (usize, &PartTier),
) -> Option<ModifierTiers> {
    let base = bases::spawn(language, base_type)?;
    let copied = Copied::of(modifier, matches)?;
    let summed = matches
        .iter()
        .position(|found| found.lines == (line..line + 1))?;
    hybrid_tiers(&TABLES, base, copied, (summed, whole), item_level)
}

/// The tiers among `tables`, on `base`, of the modifier of every line of `copied`,
/// whose line `summed` adds up its part `whole` and a pure modifier: those of the table whose
/// tiers of the numbers of `whole`, among those `item_level` allows, give that line the ranges
/// of `whole` and hold best the values of its other lines, that line taken at the lowest roll
/// of its part, the sum being in no tier; the worse found and the better its neighbour. As
/// [`splits`] takes the ways that fit best: another table holding them as well would have
/// been a second way to split the line, which leaves it without tier; the first of the file
/// on a tie, as `find` takes.
fn hybrid_tiers(
    tables: &[Table],
    base: &BaseSpawn,
    mut copied: Copied<'_>,
    (summed, whole): (usize, &PartTier),
    item_level: Option<u32>,
) -> Option<ModifierTiers> {
    let affix = match whole.affix {
        Affix::Prefix => TableAffix::Prefix,
        Affix::Suffix => TableAffix::Suffix,
    };
    // The values of that line among those of every line, in their order.
    let offset: usize = copied
        .lines
        .iter()
        .take(summed)
        .map(CopiedLine::count)
        .sum();
    let values = &mut copied.lines.get_mut(summed)?.values;
    if values.len() != whole.ranges.len() {
        return None;
    }
    for (value, range) in values.iter_mut().zip(&whole.ranges) {
        // The ranges as the game writes them, the values with the sign of the stat.
        *value = range.min.copysign(*value);
    }
    let counts: Vec<usize> = copied.lines.iter().map(CopiedLine::count).collect();
    let mut best: Option<(Fit, ModifierTiers)> = None;
    for (index, table) in tables.iter().enumerate() {
        if table.affix != affix || !copied.may_be(table) {
            continue;
        }
        let Some(order) = copied.order(&table.lines) else {
            continue;
        };
        if !first_of_its_modifier(tables, index) {
            continue;
        }
        let Some(tiers) = tiers_on_base(tables, table, base, &order, &copied) else {
            continue;
        };
        let Some((worse, better)) = part_in(&tiers, whole, offset, item_level) else {
            continue;
        };
        let values = copied.values(&table.lines, &order);
        let fit = [Some(worse), better]
            .into_iter()
            .flatten()
            .filter_map(|tier| holds(&values, &tiers.get(tier)?.ranges))
            .min();
        if let Some(fit) = fit
            && best.as_ref().is_none_or(|(best, _)| fit < *best)
        {
            let tiers = ModifierTiers {
                tiers,
                found: worse,
                neighbour: better,
                counts: counts.clone(),
                tier_unwritten: table.tier_unwritten(),
                currency: table.currency(),
                lord: table.lord.map(Lord::from),
            };
            best = Some((fit, tiers));
        }
    }
    best.map(|(_, tiers)| tiers)
}

/// The indexes in `tiers` (the best first) of the tiers of `whole`, the part of a line from
/// the value `offset` among those of a tier, the worse and the better one: none when one of them
/// isn't among `tiers`, an item of `item_level` can't roll it, or the tiers don't give that
/// line the ranges of `whole`.
fn part_in(
    tiers: &[Tier],
    whole: &PartTier,
    offset: usize,
    item_level: Option<u32>,
) -> Option<(usize, Option<usize>)> {
    let index = |number: &u32| {
        tiers
            .iter()
            .position(|tier| tier.number == *number)
            .filter(|&index| {
                tiers
                    .get(index)
                    .is_some_and(|tier| item_level.is_none_or(|level| tier.level <= level))
            })
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
