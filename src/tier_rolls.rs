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
    /// What the lines of each modifier of the item roll over, in the order of the item.
    modifiers: Vec<ModifierRolls>,
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
        let modifiers = modifiers
            .iter()
            .map(|modifier| {
                let Some(written) = Written::of(modifier) else {
                    return ModifierRolls::default();
                };
                let in_tables = base_type.and_then(|base_type| {
                    ModifierInTables::of(language, base_type, item_level, modifier)
                });
                written
                    .rolls(modifier, in_tables.as_ref())
                    .unwrap_or_else(|| {
                        if let Some(base_type) = base_type {
                            warn_of_a_missing_table(language, base_type, modifier);
                        }
                        ModifierRolls::default()
                    })
            })
            .collect();
        Self { modifiers }
    }

    /// The rolls of the line `key` over every tier; none for a line without them.
    #[must_use]
    pub fn get(&self, key: StatKey) -> Option<&LineRolls> {
        self.modifiers
            .get(key.modifier)?
            .lines
            .iter()
            .find(|(line, _)| *line == key.line)
            .map(|(_, rolls)| rolls)
    }

    /// Whether the number of the tier of the modifier `index` isn't shown: one tier only on the
    /// base of the item, or a tier the game doesn't write.
    #[must_use]
    pub fn hides_number(&self, index: usize) -> bool {
        self.modifiers
            .get(index)
            .is_some_and(|modifier| modifier.hides_number)
    }

    /// The currency that adds the modifier `index`, an essence or an alloy, written without
    /// tier: none for another modifier.
    #[must_use]
    pub fn currency(&self, index: usize) -> Option<Currency> {
        self.modifiers.get(index)?.currency
    }
}

/// How the copied item writes a modifier, which tells what its lines roll over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Written {
    /// With its tier, from the header of the advanced copy or found from its values: its lines
    /// roll over every tier of its modifier on the base.
    Tier,
    /// A prefix or suffix with its ranges but without tier: the game writes none for a modifier
    /// of the Genesis tree (`Reinforcing`, `Minions have 21(19-22)% increased maximum Life`, the
    /// third of its six tiers), nor does the application for such a line copied from the in-game
    /// trade (`with_trade_tiers`). Its lines roll over the ranges the copy writes.
    RangesWithoutTier,
    /// A prefix or suffix without tier nor range: a fixed value of a modifier whose tier the game
    /// doesn't write (`of the Despot` of the Genesis tree on a belt, `+2 to Level of all Minion
    /// Skills`; `of the Essence` on a bow, `+2 to Level of all Attack Skills`). Its lines roll
    /// over that tier, which the tables find.
    FixedWithoutTier,
    /// A modifier found from its values whose `line` adds up two modifiers of its stat: its other
    /// lines roll over the tiers of the hybrid, that line, a sum, over none.
    Split { line: usize },
}

impl Written {
    /// How `modifier` is written; none for a modifier whose lines roll over no tier: an implicit,
    /// a rune, an enchantment, a modifier of a unique, one left in doubt between two affixes, or
    /// one not revealed yet.
    fn of(modifier: &Modifier) -> Option<Self> {
        if let Some(TierDoubt::Split { line, .. }) = &modifier.doubt {
            return Some(Self::Split { line: *line });
        }
        if modifier.tier.is_some() {
            return Some(Self::Tier);
        }
        let is_affix = matches!(modifier.kind, ModifierKind::Explicit(Some(_)))
            && modifier.doubt.is_none()
            && !modifier.unrevealed;
        if !is_affix {
            return None;
        }
        let has_ranges = modifier
            .lines
            .iter()
            .any(|line| line.rolls.iter().any(|roll| roll.range.is_some()));
        Some(if has_ranges {
            Self::RangesWithoutTier
        } else {
            Self::FixedWithoutTier
        })
    }

    /// What the lines of `modifier`, written so, roll over, from its line matches and its tiers
    /// `in_tables`; none when it needs them and the tables don't hold it.
    fn rolls(
        self,
        modifier: &Modifier,
        in_tables: Option<&ModifierInTables>,
    ) -> Option<ModifierRolls> {
        match self {
            // Its copied ranges, whatever its base; the currency that adds it, from the tables.
            Self::RangesWithoutTier => Some(ModifierRolls::over_copied_ranges(
                modifier,
                in_tables.and_then(|in_tables| in_tables.tiers.currency),
            )),
            Self::Tier => Some(ModifierRolls::over_every_tier(modifier, in_tables?)),
            Self::FixedWithoutTier => Some(ModifierRolls::over_fixed_tier(modifier, in_tables?)),
            Self::Split { line } => {
                Some(ModifierRolls::over_hybrid_tiers(modifier, in_tables?, line))
            }
        }
    }
}

/// What the lines of one modifier roll over; nothing for a modifier whose lines roll over no
/// tier.
#[derive(Debug, Default)]
#[cfg_attr(test, derive(PartialEq))]
struct ModifierRolls {
    /// The rolls of each line that has them, with the index of the line.
    lines: Vec<(usize, LineRolls)>,
    /// Whether the number of its tier isn't shown ([`TierRolls::hides_number`]).
    hides_number: bool,
    /// The currency that adds it, an essence or an alloy, written without tier.
    currency: Option<Currency>,
}

impl ModifierRolls {
    /// The lines of a modifier written without tier, over the ranges the copy writes, added by
    /// `currency` if any.
    fn over_copied_ranges(modifier: &Modifier, currency: Option<Currency>) -> Self {
        Self {
            lines: copied_rolls(modifier).collect(),
            hides_number: true,
            currency,
        }
    }

    /// The lines of a modifier written with its tier, over every tier of the base `in_tables`.
    fn over_every_tier(modifier: &Modifier, in_tables: &ModifierInTables) -> Self {
        let single = in_tables.tiers.single();
        if single && modifier.tier != Some(1) {
            // The game knows more tiers than the data, e.g. after a patch: the tier the copy
            // writes is kept.
            log::warn!(
                "Game data, one tier only for a modifier copied as tier {:?}: {}",
                modifier.tier,
                modifier.lines.first().map_or("", |line| line.text.as_str())
            );
        }
        Self {
            lines: in_tables.rolls_over(modifier, &in_tables.tiers.tiers, None),
            hides_number: single && modifier.tier == Some(1),
            currency: None,
        }
    }

    /// The lines of a fixed value written without tier, over its tier `in_tables`: only of a modifier
    /// whose tier the game doesn't write (the Genesis tree, an essence, an alloy), as a line of
    /// the in-game trade read in it, a tier of fixed values too (not a value of a modifier whose
    /// tier the game writes, nor one that falls in a tier of the tree that rolls, `19–22`);
    /// nothing otherwise.
    fn over_fixed_tier(modifier: &Modifier, in_tables: &ModifierInTables) -> Self {
        let unwritten = |tier: &&Tier| {
            in_tables.tiers.tier_unwritten && tier.ranges.iter().all(|range| range.is_one_value())
        };
        let Some(tier) = in_tables.tiers.tier().filter(unwritten) else {
            return Self::default();
        };
        Self {
            lines: in_tables.rolls_over(modifier, std::slice::from_ref(tier), None),
            hides_number: true,
            currency: in_tables.tiers.currency,
        }
    }

    /// The other lines of a hybrid whose part its `summed_line` adds up, over the tiers of the
    /// hybrid `in_tables`.
    fn over_hybrid_tiers(
        modifier: &Modifier,
        in_tables: &ModifierInTables,
        summed_line: usize,
    ) -> Self {
        Self {
            lines: in_tables.rolls_over(modifier, &in_tables.tiers.tiers, Some(summed_line)),
            hides_number: false,
            currency: None,
        }
    }
}

/// A modifier in the modifier tables, on the base of the item: its line matches and its tiers.
struct ModifierInTables {
    matches: Vec<LineMatch<'static>>,
    tiers: ModifierTiers,
}

impl ModifierInTables {
    /// `modifier`, written in `language`, in the tables of the base `base_type` of an item of
    /// `item_level`; none when no table holds it.
    fn of(
        language: Language,
        base_type: &str,
        item_level: Option<u32>,
        modifier: &Modifier,
    ) -> Option<Self> {
        let matches = game_data::modifier_stats(language, modifier, None);
        let tiers = game_data::modifier_tiers(language, base_type, item_level, modifier, &matches)?;
        Some(Self { matches, tiers })
    }

    /// The rolls of each line of `modifier` over `tiers`, but its `summed_line` that adds up two
    /// modifiers of its stat, a sum over no tiers, with the index of the line: the ranges of a
    /// tier follow the values of each line match, and the values of a match the numbers its
    /// stat counts on its lines, in their order.
    fn rolls_over(
        &self,
        modifier: &Modifier,
        tiers: &[Tier],
        summed_line: Option<usize>,
    ) -> Vec<(usize, LineRolls)> {
        let values_of_each_match = self.tiers.counts.iter().scan(0, |start, &count| {
            let first = *start;
            *start += count;
            Some((first, count))
        });
        self.matches
            .iter()
            .zip(values_of_each_match)
            .filter_map(|(line_match, (start, count))| {
                let values = share_values(start, count, &counted_numbers(line_match, modifier))?;
                Some(line_match.lines.clone().zip(values))
            })
            .flatten()
            .filter(|&(line, _)| summed_line != Some(line))
            .filter_map(|(line, values)| {
                let number_count = modifier.lines.get(line).map_or(0, |line| line.rolls.len());
                let mut rolls = line_rolls(tiers, &values, number_count)?;
                // Over one tier only, "T1–T1" would tell nothing.
                if tiers.len() < 2 {
                    rolls.tiers = None;
                }
                Some((line, rolls))
            })
            .collect()
    }
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

/// The rolls of each line of `modifier` over the ranges the advanced copy writes, without their
/// sign, with the index of the line.
fn copied_rolls(modifier: &Modifier) -> impl Iterator<Item = (usize, LineRolls)> {
    modifier
        .lines
        .iter()
        .enumerate()
        .filter_map(|(line, stat_line)| {
            let ranges: Vec<Option<Range>> = stat_line
                .rolls
                .iter()
                .map(|roll| roll.range.and_then(Range::magnitude))
                .collect();
            let has_ranges = ranges.iter().any(Option::is_some);
            has_ranges.then_some((
                line,
                LineRolls {
                    ranges,
                    tiers: None,
                },
            ))
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

/// The rolls over `tiers`, the best first, of a line of `number_count` numbers, whose `values` pair
/// the index of a number with the index of its value among the ranges of a tier; none for a
/// line without such a number, or when a tier hasn't the value, or its range crosses zero.
fn line_rolls(tiers: &[Tier], values: &[(usize, usize)], number_count: usize) -> Option<LineRolls> {
    if values.is_empty() {
        return None;
    }
    let mut ranges = vec![None; number_count];
    for &(number, value) in values {
        let span = tiers
            .iter()
            .map(|tier| tier.ranges.get(value)?.magnitude())
            .reduce(|span, range| {
                let (span, range) = (span?, range?);
                Some(span.spanning(range))
            })
            .flatten()?;
        *ranges.get_mut(number)? = Some(span);
    }
    let tiers = match values {
        &[(number, value)] => {
            let span = ranges.get(number).copied().flatten()?;
            let range_of =
                |tier: &&Tier| tier.ranges.get(value).and_then(|range| range.magnitude());
            // The worst tier that rolls the lowest value, the best that rolls the highest.
            let lowest = tiers
                .iter()
                .rev()
                .find(|tier| range_of(tier).is_some_and(|range| range.min <= span.min))?;
            let highest = tiers
                .iter()
                .find(|tier| range_of(tier).is_some_and(|range| range.max >= span.max))?;
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
