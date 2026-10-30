//! The rolls of a unique copied without ranges (from the in-game trade), taken from the
//! modifiers the price database writes for it with their ranges, e.g. `+(40-60) to maximum
//! Energy Shield`. Each copied line is matched with a modifier of the database by its stat,
//! not by its text, so that a French copy gets them too.

use super::lines::{LineMatch, modifier_stats};
use super::stats::{Stat, TradeGroup};
use super::trade_site::TradeStats;
use crate::item::{Modifier, ModifierKind, ModifierOrigin, Range, StatLine};
use crate::language::Language;

/// The difference under which two values are one: both are read from decimal texts, so that
/// the same text gives the same value. A range of one value whose decimals the database left
/// after it (`Lose (5-5).3% of maximum Life per second`) is read without them, and doesn't
/// hold the copied `5.3`: the line stays without range.
const SAME_VALUE: f64 = 1e-6;

/// A stat of a modifier of the database that counts one number, with its range.
struct Rolled<'a> {
    stat: &'a Stat,
    /// The option of an option stat (the skill of `# to Level of all Arc Skills`).
    option: Option<f64>,
    /// Without its sign, as the game writes it.
    range: Range,
}

impl Rolled<'_> {
    fn holds(&self, value: f64) -> bool {
        self.range.min - SAME_VALUE <= value && value <= self.range.max + SAME_VALUE
    }

    fn fixed(&self) -> bool {
        self.range.max - self.range.min < SAME_VALUE
    }
}

/// The modifiers of a unique copied without ranges, in `language`, with the ranges of the
/// modifiers of the database on the lines it holds: `implicit` those of its base, `explicit`
/// its own, in English, as the database writes them. A range of one value (`(5-5)`), or a
/// number the database writes without range, is a line that never varies, when it is the
/// copied value: otherwise it is the value of one copy (`+3 to Level of all Cold Skills` of
/// Sunsplinter, which rolls). Only the stats that count one number get a range: the database
/// writes the range of the second number of `Adds # to # Cold Damage` on the first, and the
/// value of one copy on the second (`Adds (13-15) to 13` for `Adds 8(8-10) to 14(13-15)`,
/// checked on 2026-10-30). A line the database doesn't hold keeps its values without range,
/// as does the Vaal modifier, which the database never holds, and a line whose range crosses
/// zero (`+(-40-40)% to Fire Resistance`), which the game writes with a sign of its own.
#[must_use]
pub fn with_unique_rolls(
    language: Language,
    modifiers: &[Modifier],
    implicit: &[String],
    explicit: &[String],
    trade: Option<&TradeStats>,
) -> Vec<Modifier> {
    let mut implicit = rolled(implicit, ModifierKind::Implicit, trade);
    let mut explicit = rolled(explicit, ModifierKind::Explicit(None), trade);
    modifiers
        .iter()
        .map(|modifier| {
            let pool = match modifier.kind {
                ModifierKind::Implicit => &mut implicit,
                ModifierKind::Explicit(_) if modifier.origin != Some(ModifierOrigin::Vaal) => {
                    &mut explicit
                }
                _ => return modifier.clone(),
            };
            let mut modifier = modifier.clone();
            for line_match in modifier_stats(language, &modifier, trade) {
                write_range(&mut modifier, &line_match, pool);
            }
            modifier
        })
        .collect()
}

/// The stats of the modifiers `written` by the database, in English, of the `kind` of the
/// copied lines they are matched with.
fn rolled<'a>(
    written: &[String],
    kind: ModifierKind,
    trade: Option<&'a TradeStats>,
) -> Vec<Rolled<'a>> {
    written
        .iter()
        .flat_map(|text| stats_of(text, kind, trade))
        .collect()
}

/// The stats of the modifier `text` of the database that count one number with a range of one
/// sign; the other lines are left out.
fn stats_of<'a>(text: &str, kind: ModifierKind, trade: Option<&'a TradeStats>) -> Vec<Rolled<'a>> {
    let modifier = Modifier {
        kind,
        origin: None,
        name: None,
        tier: None,
        tags: Vec::new(),
        unrevealed: false,
        lines: text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(|line| StatLine::parse(&with_values(line)))
            .collect(),
        doubt: None,
    };
    modifier_stats(Language::English, &modifier, trade)
        .into_iter()
        .filter_map(|line_match| {
            let stat = line_match.stat?;
            let lines = modifier.lines.get(line_match.lines)?;
            let mut rolls = lines
                .iter()
                .flat_map(|line| &line.rolls)
                .zip(&stat.counted)
                .filter_map(|(roll, &counted)| counted.then_some(roll));
            let (Some(roll), None) = (rolls.next(), rolls.next()) else {
                return None;
            };
            // A number written without range never varies.
            let range = roll.range.unwrap_or(Range {
                min: roll.value,
                max: roll.value,
            });
            Some(Rolled {
                stat: stat.stat,
                option: stat.matcher_value.filter(|_| stat.stat.option),
                range: range.magnitude()?,
            })
        })
        .collect()
}

/// Whether two stats, of two languages, are one: the same English template, and the same game
/// id unless one of them is a stat of the trade site, which has none.
fn same_stat(first: &Stat, second: &Stat) -> bool {
    first.reference == second.reference
        && (first.id == second.id || first.id.is_none() || second.id.is_none())
}

/// Whether two stats share an id of the trade site in `group`, which searches them as one: two
/// stats a language writes the same, the French data holding only one of them, with the ids of
/// both. The French bleeding of a weapon (`#% de chances d'infliger le Saignement au Toucher`,
/// the global stat), the French rarity of an item (`… de la Rareté des Objets trouvés`, the
/// stat of a map), the mana recovery of the flasks, the Rage on hit; none in English (the list
/// is a test, `the_stats_matched_by_a_trade_id_are_known`).
fn same_trade_id(first: &Stat, second: &Stat, group: TradeGroup) -> bool {
    let ids = second.trade_ids(group);
    first.trade_ids(group).iter().any(|id| ids.contains(id))
}

/// Writes on the value of the lines of `line_match` the range of a stat of `pool` that is its
/// stat, taken out of the pool so that two lines of one stat get the ranges of two modifiers:
/// the first whose range holds the value, or else the first that varies, whose range the
/// value of a legacy unique left; none for a stat that counts several numbers, or when the
/// pool only has ranges of one other value.
fn write_range(modifier: &mut Modifier, line_match: &LineMatch, pool: &mut Vec<Rolled>) {
    let Some(stat) = &line_match.stat else {
        return;
    };
    let group = TradeGroup::of(modifier);
    let Some(lines) = modifier.lines.get_mut(line_match.lines.clone()) else {
        return;
    };
    let mut rolls = lines
        .iter_mut()
        .flat_map(|line| &mut line.rolls)
        .zip(&stat.counted)
        .filter_map(|(roll, &counted)| counted.then_some(roll));
    let (Some(roll), None) = (rolls.next(), rolls.next()) else {
        return;
    };
    let value = roll.value.abs();
    let option = stat.matcher_value.filter(|_| stat.stat.option);
    let same = |rolled: &Rolled| {
        let one = same_stat(rolled.stat, stat.stat) || same_trade_id(rolled.stat, stat.stat, group);
        one && rolled.option == option
    };
    let index = pool
        .iter()
        .position(|rolled| same(rolled) && rolled.holds(value))
        .or_else(|| {
            pool.iter()
                .position(|rolled| same(rolled) && !rolled.fixed())
        });
    if let Some(index) = index {
        roll.range = Some(pool.remove(index).range);
    }
}

/// The line `line` of the database with a value before each of its ranges, as the advanced
/// copy writes them, so that it is read like a copied line: `+(40-60) to maximum Life` is
/// `+40(40-60) to maximum Life`, `(-20--10) to Strength` is `-20(-20--10) to Strength`. The
/// database writes the decimals of the value of one copy after the range of a decimal one
/// (`gain (0.17-0.25).18 charges`, `Lose (5-5).3% of maximum Life`): they are left out. A
/// parenthesis that holds no range stays as it is.
fn with_values(line: &str) -> String {
    let mut written = String::with_capacity(line.len() * 2);
    let mut rest = line;
    let digit = |character: char| character.is_ascii_digit();
    while let Some((before, opened)) = rest.split_once('(') {
        written.push_str(before);
        let range = opened
            .split_once(')')
            .and_then(|(inside, after)| Some((first_bound(inside)?, inside, after)))
            // A range the line already writes a value before (`5(5-5)%`) keeps it.
            .filter(|_| !before.ends_with(digit));
        let Some((first, inside, after)) = range else {
            written.push('(');
            rest = opened;
            continue;
        };
        written.push_str(first);
        written.push('(');
        written.push_str(inside);
        written.push(')');
        rest = after
            .strip_prefix('.')
            .filter(|decimals| decimals.starts_with(digit))
            .map_or(after, |decimals| decimals.trim_start_matches(digit));
    }
    written.push_str(rest);
    written
}

/// The first bound of `inside`, the text of a range between its parentheses (`-40--20` gives
/// `-40`); none when it isn't a range of two numbers.
fn first_bound(inside: &str) -> Option<&str> {
    let separator = inside
        .char_indices()
        .skip(1)
        .find(|&(index, character)| {
            character == '-'
                && inside
                    .get(..index)
                    .and_then(|before| before.chars().next_back())
                    .is_some_and(|before| before.is_ascii_digit())
        })
        .map(|(index, _)| index)?;
    let (first, second) = (inside.get(..separator)?, inside.get(separator + 1..)?);
    // As the copied text writes a range: digits, points and a minus sign only.
    let is_number = |text: &str| {
        text.chars()
            .all(|character| character.is_ascii_digit() || matches!(character, '.' | '-'))
            && text.parse::<f64>().is_ok()
    };
    (is_number(first) && is_number(second)).then_some(first)
}

#[cfg(test)]
mod tests;
