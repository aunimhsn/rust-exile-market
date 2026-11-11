//! The rolls of each line of a prefix or suffix of the copied item over every tier of its
//! modifier on the base of the item, from the modifier tables: its bar, the range its field
//! stays in and the rolls its tooltip tells. Read once when the item is copied, for a modifier
//! whose header tells its affix and its tier, and for the other lines of a hybrid whose part a
//! line adds up, over the tiers of the hybrid; not for that line, a sum over no tiers. A
//! modifier of one tier only on the base has no tier to tell: its lines roll over that tier. Nor
//! has a modifier whose tier the game doesn't write, of the Genesis tree: its lines roll over the
//! ranges of the advanced copy (`Reinforcing`, `Minions have 21(19-22)% increased maximum
//! Life`), or over the tier the tables find for a fixed value, which the copy writes without
//! range (`of the Despot` of a belt, `+2 to Level of all Minion Skills`). The tables also tell
//! the currency that adds such a modifier, an essence or an alloy, named after its ranges.
//!
//! The ranges of a tier follow the values of each line match of the modifier, and the values
//! of a match the numbers its stat counts on its lines, in their order: the numbers are read
//! from the same matches as the tiers, so that each range lands on its number.

use std::collections::BTreeSet;
use std::sync::{Mutex, PoisonError};

use crate::filters::StatKey;
use crate::game_data::{self, LineMatch, ModifierTiers, Tier};
use crate::item::{Currency, Modifier, ModifierKind, Range, TierDoubt};
use crate::language::Language;
use crate::logging;
use crate::trade::stat_filters::counted_numbers;

/// The tiers of the lowest and of the highest roll of a line over every tier: "T13–T1".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TierSpan {
    pub lowest: u32,
    pub highest: u32,
}

/// The rolls of a line over every tier of its modifier.
#[derive(Debug, Clone, PartialEq)]
pub struct LineRolls {
    /// For each number of the line, in its order, from the lowest roll of its worst tier to
    /// the highest of tier 1, as the game writes it: without its sign; none for a number its
    /// stat doesn't count (the 3 of `#% chance to gain Onslaught for 3 seconds`), except for a
    /// modifier written without tier, the range the copy writes of every number, those its
    /// stat doesn't count left out by the reader (`Reading::pick` in `src/ui/item_stats.rs`).
    pub ranges: Vec<Option<Range>>,
    /// The tiers of its lowest and highest rolls, for a line of one number its stat counts.
    pub tiers: Option<TierSpan>,
}

/// The rolls over every tier of the lines of the copied item that have them.
#[derive(Debug, Default)]
pub struct TierRolls {
    lines: Vec<(StatKey, LineRolls)>,
    /// The index of each modifier without tier to tell: of one tier only on the base of the
    /// item, whose T1 would read as "the best" while it is "the only one", or whose tier the game
    /// doesn't write (the Genesis tree, an essence, an alloy).
    single: Vec<usize>,
    /// The index of each modifier a currency adds, written without tier, and that currency.
    currencies: Vec<(usize, Currency)>,
}

impl TierRolls {
    /// The rolls of the lines of `modifiers`, written in `language`, on the base `base_type`
    /// (the name its search looks for; none for a magic item before the names of the trade
    /// site) of an item of `item_level` (none when the copy doesn't write it), which the tiers
    /// of a split line are among.
    #[must_use]
    pub fn new(
        language: Language,
        modifiers: &[Modifier],
        base_type: Option<&str>,
        item_level: Option<u32>,
    ) -> Self {
        let mut lines = Vec::new();
        let mut single = Vec::new();
        let mut currencies = Vec::new();
        for (index, modifier) in modifiers.iter().enumerate() {
            let without_tier = written_without_tier(modifier);
            let fixed = fixed_without_tier(modifier);
            let summed = match &modifier.doubt {
                Some(TierDoubt::Split { line, .. }) => Some(*line),
                _ if without_tier || modifier.tier.is_some() || fixed => None,
                _ => continue,
            };
            let found = base_type
                .and_then(|base_type| tables_of(language, base_type, item_level, modifier));
            // Its copied ranges, whatever its base; the currency that adds it, from the tables.
            if without_tier {
                single.push(index);
                lines.extend(copied_rolls(index, modifier));
                let currency = found.and_then(|(_, tiers)| tiers.currency);
                currencies.extend(currency.map(|currency| (index, currency)));
                continue;
            }
            let Some((matches, tiers)) = found else {
                if let Some(base_type) = base_type {
                    warn_of_a_missing_table(language, base_type, modifier);
                }
                continue;
            };
            // A fixed value without tier: only of a modifier whose tier the game doesn't write
            // (the Genesis tree, an essence, an alloy), over that tier, as a line of the in-game
            // trade read in it, a tier of fixed values too (not a value of a modifier whose tier
            // the game writes, nor one that falls in a tier of the tree that rolls, `19–22`).
            let rolled = if fixed {
                let unwritten = |tier: &&Tier| {
                    tiers.tier_unwritten && tier.ranges.iter().all(|range| range.is_one_value())
                };
                match tiers.tier().filter(unwritten) {
                    Some(tier) => std::slice::from_ref(tier),
                    None => continue,
                }
            } else {
                tiers.tiers.as_slice()
            };
            // The hybrid of a split line of one tier only too: its other lines tell no tiers.
            let one_tier = tiers.single() || fixed;
            if fixed {
                single.push(index);
                currencies.extend(tiers.currency.map(|currency| (index, currency)));
            } else if one_tier && summed.is_none() {
                match modifier.tier {
                    Some(1) => single.push(index),
                    // The game knows more tiers than the data, e.g. after a patch: the tier
                    // the copy writes is kept.
                    tier => log::warn!(
                        "Game data, one tier only for a modifier copied as tier {tier:?}: {}",
                        modifier.lines.first().map_or("", |line| line.text.as_str())
                    ),
                }
            }
            let mut start = 0;
            for (found, &count) in matches.iter().zip(&tiers.counts) {
                let numbers = counted_numbers(found, modifier);
                if let Some(values) = share_values(start, count, &numbers) {
                    for (line, values) in found.lines.clone().zip(values) {
                        if summed == Some(line) {
                            continue;
                        }
                        let rolls = modifier.lines.get(line).map_or(0, |line| line.rolls.len());
                        if let Some(mut rolls) = line_rolls(rolled, &values, rolls) {
                            if one_tier {
                                // "T1–T1" would tell nothing.
                                rolls.tiers = None;
                            }
                            let key = StatKey {
                                modifier: index,
                                line,
                            };
                            lines.push((key, rolls));
                        }
                    }
                }
                start += count;
            }
        }
        Self {
            lines,
            single,
            currencies,
        }
    }

    /// The rolls of the line `key` over every tier; none for a line without them.
    #[must_use]
    pub fn get(&self, key: StatKey) -> Option<&LineRolls> {
        self.lines
            .iter()
            .find(|(line, _)| *line == key)
            .map(|(_, rolls)| rolls)
    }

    /// Whether the modifier `index` has no tier to tell: one tier only on the base of the item,
    /// or a tier the game doesn't write.
    #[must_use]
    pub fn single(&self, index: usize) -> bool {
        self.single.contains(&index)
    }

    /// The currency that adds the modifier `index`, an essence or an alloy, written without
    /// tier: none for another modifier.
    #[must_use]
    pub fn currency(&self, index: usize) -> Option<Currency> {
        self.currencies
            .iter()
            .find(|&&(modifier, _)| modifier == index)
            .map(|&(_, currency)| currency)
    }
}

/// The line matches of `modifier` and its tiers among the modifier tables, on the base
/// `base_type` of an item of `item_level`; none when no table holds it.
fn tables_of(
    language: Language,
    base_type: &str,
    item_level: Option<u32>,
    modifier: &Modifier,
) -> Option<(Vec<LineMatch<'static>>, ModifierTiers)> {
    let matches = game_data::modifier_stats(language, modifier, None);
    let tiers = game_data::modifier_tiers(language, base_type, item_level, modifier, &matches)?;
    Some((matches, tiers))
}

/// Writes to the log that no table holds `modifier`, when one should
/// ([`game_data::lacks_table`]): the first sign that the game changed its modifiers, and the
/// list of those to generate again, in the log a bug report carries.
fn warn_of_a_missing_table(language: Language, base_type: &str, modifier: &Modifier) {
    /// What was already told: an item is read again at each copy, and when the names of the
    /// trade site arrive.
    static TOLD: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());

    let matches = game_data::modifier_stats(language, modifier, None);
    if !game_data::lacks_table(language, base_type, modifier, &matches) {
        return;
    }
    // Its name, not its line: the same modifier rolled otherwise is the same missing table.
    let key = format!(
        "\"{}\" as tier {:?} on {}",
        logging::plain(modifier.name.as_deref().unwrap_or_default()),
        modifier.tier,
        logging::plain(base_type),
    );
    let mut told = TOLD.lock().unwrap_or_else(PoisonError::into_inner);
    if first_told(&mut told, &key) {
        log::warn!(
            "Game data, no table for the modifier {key} (data behind the game, or an item of \
             an older patch): {}",
            modifier
                .lines
                .first()
                .map_or_else(String::new, |line| logging::plain(&line.text))
        );
    }
}

/// Whether `key` is told for the first time, kept in `told` then; never past [`MOST_TOLD`]
/// keys, so that the log and the memory stay small whatever is copied.
fn first_told(told: &mut BTreeSet<String>, key: &str) -> bool {
    told.len() < MOST_TOLD && told.insert(key.to_owned())
}

/// The most missing tables told in a session.
const MOST_TOLD: usize = 500;

/// Whether the advanced copy writes `modifier`, a prefix or suffix, with its ranges but without
/// tier: the game writes none for a modifier of the Genesis tree (`Reinforcing`, `Minions have
/// 21(19-22)% increased maximum Life`, the third of its six tiers), nor does the application for
/// such a line copied from the in-game trade (`with_trade_tiers`).
fn written_without_tier(modifier: &Modifier) -> bool {
    matches!(modifier.kind, ModifierKind::Explicit(Some(_)))
        && modifier.tier.is_none()
        && modifier.doubt.is_none()
        && !modifier.unrevealed
        && modifier
            .lines
            .iter()
            .any(|line| line.rolls.iter().any(|roll| roll.range.is_some()))
}

/// Whether the advanced copy writes `modifier`, a prefix or suffix, without tier nor range: a
/// fixed value of a modifier whose tier the game doesn't write (`of the Despot` of the Genesis
/// tree on a belt, `+2 to Level of all Minion Skills`; `of the Essence` on a bow, `+2 to Level of
/// all Attack Skills`).
fn fixed_without_tier(modifier: &Modifier) -> bool {
    matches!(modifier.kind, ModifierKind::Explicit(Some(_)))
        && modifier.tier.is_none()
        && modifier.doubt.is_none()
        && !modifier.unrevealed
        && modifier
            .lines
            .iter()
            .all(|line| line.rolls.iter().all(|roll| roll.range.is_none()))
}

/// The rolls of each line of the modifier `index`, `modifier`, over the ranges the advanced copy
/// writes, without their sign.
fn copied_rolls(index: usize, modifier: &Modifier) -> impl Iterator<Item = (StatKey, LineRolls)> {
    modifier
        .lines
        .iter()
        .enumerate()
        .filter_map(move |(line, text)| {
            let ranges: Vec<Option<Range>> = text
                .rolls
                .iter()
                .map(|roll| roll.range.and_then(Range::magnitude))
                .collect();
            ranges.iter().any(Option::is_some).then(|| {
                let key = StatKey {
                    modifier: index,
                    line,
                };
                (
                    key,
                    LineRolls {
                        ranges,
                        tiers: None,
                    },
                )
            })
        })
}

/// The values of a line match, `count` of them from `start` among the ranges of a tier,
/// shared among its lines in their order: for each line, the pairs of the index of a number
/// its stat counts (`numbers`, for each line) and the index of its value. None when the
/// numbers aren't as many as the values: a line without number whose string gives the value
/// (`Loads an additional bolt`) has no roll.
fn share_values(
    start: usize,
    count: usize,
    numbers: &[Vec<usize>],
) -> Option<Vec<Vec<(usize, usize)>>> {
    let total: usize = numbers.iter().map(Vec::len).sum();
    if total != count {
        return None;
    }
    let mut values = start..;
    Some(
        numbers
            .iter()
            .map(|line| line.iter().copied().zip(&mut values).collect())
            .collect(),
    )
}

/// The rolls over `tiers`, the best first, of a line of `rolls` numbers, whose `values` pair
/// the index of a number with the index of its value among the ranges of a tier; none for a
/// line without such a number, or when a tier hasn't the value, or its range crosses zero.
fn line_rolls(tiers: &[Tier], values: &[(usize, usize)], rolls: usize) -> Option<LineRolls> {
    if values.is_empty() {
        return None;
    }
    let mut ranges = vec![None; rolls];
    for &(number, value) in values {
        let over = tiers
            .iter()
            .map(|tier| tier.ranges.get(value)?.magnitude())
            .reduce(|over, range| {
                let (over, range) = (over?, range?);
                Some(over.spanning(range))
            })
            .flatten()?;
        *ranges.get_mut(number)? = Some(over);
    }
    let tiers = match values {
        &[(number, value)] => {
            let over = ranges.get(number).copied().flatten()?;
            let rolls = |tier: &&Tier| tier.ranges.get(value).and_then(|range| range.magnitude());
            // The worst tier that rolls the lowest value, the best that rolls the highest.
            let lowest = tiers
                .iter()
                .rev()
                .find(|tier| rolls(tier).is_some_and(|rolls| rolls.min <= over.min))?;
            let highest = tiers
                .iter()
                .find(|tier| rolls(tier).is_some_and(|rolls| rolls.max >= over.max))?;
            Some(TierSpan {
                lowest: lowest.number,
                highest: highest.number,
            })
        }
        _ => None,
    };
    Some(LineRolls { ranges, tiers })
}

#[cfg(test)]
mod tests;
