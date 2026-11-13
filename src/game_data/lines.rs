//! The stat of each line of a modifier. The numbers of the line become `#` (with their sign),
//! or stay written for the strings that write one (`retain 40% of Glory`), and the string is
//! looked up among the stats of the language of the item: those of Exiled Exchange 2, then
//! those of the trade site, for a line the first ones don't know or have no id for in its
//! group.

use std::iter;
use std::ops::Range;

use super::stats::{Stat, Stats, TradeGroup, normalize};
use super::trade_site::TradeStats;
use crate::item::Modifier;
use crate::language::Language;

/// Beyond this many numbers in a line, only the string with every number as `#` is looked
/// up: each number can be written three ways.
const MAX_NUMBERS: usize = 6;

/// Where the stat of a line was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatSource {
    /// The stats of Exiled Exchange 2, embedded.
    GameData,
    /// The templates of the trade site: a line the game data doesn't know yet.
    TradeSite,
}

/// The stat of a line.
#[derive(Debug, Clone, PartialEq)]
pub struct StatMatch<'a> {
    pub stat: &'a Stat,
    pub source: StatSource,
    /// The values of the line in the order of its `#`, with the sign of the stat: the line
    /// `20% reduced Charm Charges used` gives -20 (a stat of the trade site writes the side
    /// of its template: 20 for `#% reduced Charm Charges used`).
    pub values: Vec<f64>,
    /// For each number of the line, in its order (over all its lines for a stat written on
    /// several), whether it is a `#` of the stat; not a number the string writes (the 3 of
    /// `#% chance to gain Onslaught for 3 seconds`). Read by [`Self::counted`] and
    /// [`LineMatch::counted_numbers`] only.
    is_counted: Vec<bool>,
    /// The value given by the string of the line: the option of an option stat (the skill of
    /// `# to Level of all Arc Skills`), or else the value of a line without number.
    pub matcher_value: Option<f64>,
    /// The string of the line writes the opposite of the stat: `reduced` for an `increased`
    /// stat, whose values are given with the sign of the stat.
    pub negate: bool,
    /// The index of the string of the stat that matched the line.
    matcher_index: usize,
}

impl StatMatch<'_> {
    /// Of `numbers`, one for each number of the lines of the stat in their order, those the
    /// stat counts: its `#`, not a number its string writes. Numbers that don't line up with
    /// those the stat read stop at the fewer of both.
    pub fn counted<T>(&self, numbers: impl IntoIterator<Item = T>) -> impl Iterator<Item = T> {
        numbers
            .into_iter()
            .zip(&self.is_counted)
            .filter_map(|(number, &counted)| counted.then_some(number))
    }
}

/// The lines of a modifier written by one stat, or a line found nowhere.
#[derive(Debug, Clone, PartialEq)]
pub struct LineMatch<'a> {
    /// One line, or several for a stat written on several lines.
    pub lines: Range<usize>,
    pub stat: Option<StatMatch<'a>>,
}

impl LineMatch<'_> {
    /// For each of its lines, a line of `modifier`, the indexes of its numbers the stat counts;
    /// all of them when the numbers of the lines don't line up with those the stat read.
    #[must_use]
    pub fn counted_numbers(&self, modifier: &Modifier) -> Vec<Vec<usize>> {
        let counts: Vec<usize> = self
            .lines
            .clone()
            .map(|line| modifier.lines.get(line).map_or(0, |line| line.rolls.len()))
            .collect();
        let counted = self
            .stat
            .as_ref()
            .map(|stat| stat.is_counted.as_slice())
            .filter(|counted| counted.len() == counts.iter().sum::<usize>());
        let mut start = 0;
        counts
            .iter()
            .map(|&count| {
                let numbers = (0..count)
                    .filter(|number| {
                        counted.is_none_or(|counted| counted.get(start + number) == Some(&true))
                    })
                    .collect();
                start += count;
                numbers
            })
            .collect()
    }
}

/// Where the lines of a language are looked up.
struct Lookup<'a> {
    game: &'static Stats,
    /// None until the trade site gave its templates.
    trade: Option<&'a Stats>,
}

impl<'a> Lookup<'a> {
    fn new(language: Language, trade: Option<&'a TradeStats>) -> Self {
        Self {
            game: Stats::of(language),
            trade: trade.map(|trade| trade.of(language)),
        }
    }

    /// The most lines a string has.
    fn max_lines(&self) -> usize {
        let trade = self.trade.map_or(1, |trade| trade.max_lines);
        self.game.max_lines.max(trade)
    }

    /// The stat written `text`, preferably one with an id in the group of the line: a
    /// string with `#` can be a stat of another group (`#% of Armour also applies to
    /// Lightning Damage` is a rune) while the line is a stat that writes its number (`100% of
    /// Armour…`). The stats of the game data come first, then those of the trade site.
    fn find(&self, text: &str, group: TradeGroup) -> Option<StatMatch<'a>> {
        let numbers = numbers(text);
        let sources = iter::once((self.game, StatSource::GameData))
            .chain(self.trade.map(|trade| (trade, StatSource::TradeSite)));
        let found = sources.flat_map(|(stats, source)| {
            templates(text, &numbers).flat_map(move |template| {
                stats
                    .find(&template.string)
                    .filter_map(move |(stat, matcher_index)| {
                        stat_match(stat, source, matcher_index, &template)
                    })
            })
        });
        first_with_id(found, |stat| has_id(stat, group))
    }
}

/// The most lines a stat of the game data is written on in `language`: no run of lines longer
/// than this makes one stat.
#[must_use]
pub fn max_stat_lines(language: Language) -> usize {
    Stats::of(language).max_lines
}

/// The stats of the lines of `modifier`, in their order; a stat written on several lines is
/// preferred to the stats of these lines one by one. `trade` holds the templates of the trade
/// site, once it gave them.
#[must_use]
pub fn modifier_stats<'a>(
    language: Language,
    modifier: &Modifier,
    trade: Option<&'a TradeStats>,
) -> Vec<LineMatch<'a>> {
    let lookup = Lookup::new(language, trade);
    let group = TradeGroup::of(modifier);
    let texts: Vec<String> = modifier
        .lines
        .iter()
        .map(|line| normalize(&line.text))
        .collect();
    let mut matches = Vec::new();
    let mut first = 0;
    while first < texts.len() {
        let most = lookup.max_lines().min(texts.len() - first);
        let windows = (1..=most).rev().filter_map(|count| {
            let text = texts.get(first..first + count)?.join("\n");
            Some((count, lookup.find(&text, group)?))
        });
        let found = first_with_id(windows, |(_, stat)| has_id(stat, group));
        let (count, stat) = found.map_or((1, None), |(count, stat)| (count, Some(stat)));
        matches.push(LineMatch {
            lines: first..first + count,
            stat,
        });
        first += count;
    }
    matches
}

fn stat_match<'a>(
    stat: &'a Stat,
    source: StatSource,
    matcher_index: usize,
    template: &Template,
) -> Option<StatMatch<'a>> {
    let matcher = stat.matchers.get(matcher_index)?;
    let sign = if matcher.negate { -1.0 } else { 1.0 };
    Some(StatMatch {
        stat,
        source,
        values: template.values.iter().map(|value| value * sign).collect(),
        is_counted: template.counted.clone(),
        matcher_value: matcher.value,
        negate: matcher.negate,
        matcher_index,
    })
}

/// The English string of the stat of the line `text` written in `language`, at the same place
/// among the strings of the stat: `#% de Réduction de l'Armure` is `#% reduced Armour`; for a
/// stat of the trade site, the English template of its id (its own template when the English
/// trade site hasn't this id). None for an English line, a line found nowhere, or a stat whose
/// strings don't line up in English.
#[must_use]
pub fn english_string<'a>(
    language: Language,
    text: &str,
    group: TradeGroup,
    trade: Option<&'a TradeStats>,
) -> Option<&'a str> {
    if language == Language::English {
        return None;
    }
    let found = Lookup::new(language, trade).find(&normalize(text), group)?;
    match found.source {
        StatSource::GameData => {
            let twin = Stats::of(Language::English).twin(found.stat, found.matcher_index)?;
            Some(&twin.string)
        }
        StatSource::TradeSite => Some(&found.stat.reference),
    }
}

fn has_id(stat: &StatMatch, group: TradeGroup) -> bool {
    !stat.stat.trade_ids(group).is_empty()
}

/// The first item that has an id in its group, or else the first item.
fn first_with_id<T>(items: impl Iterator<Item = T>, has_id: impl Fn(&T) -> bool) -> Option<T> {
    let mut first = None;
    for item in items {
        if has_id(&item) {
            return Some(item);
        }
        first = first.or(Some(item));
    }
    first
}

/// A number of a line, with its sign when it has one.
#[derive(Debug, PartialEq)]
struct Number {
    /// Where it starts, at its sign or else at its first digit.
    start: usize,
    digits: usize,
    /// Where it ends, after its last digit.
    end: usize,
    magnitude: f64,
    negative: bool,
}

impl Number {
    /// Its value, with its sign.
    fn value(&self) -> f64 {
        if self.negative {
            -self.magnitude
        } else {
            self.magnitude
        }
    }
}

/// The numbers of `text`, as the parser reads those of a copied line (the test
/// `the_parser_and_the_strings_read_the_same_numbers`). A sign right before a number is its
/// own, unless it ends a word (`X-10`); a point is in a number only between two digits (`2.5`,
/// not `2 times.`).
fn numbers(text: &str) -> Vec<Number> {
    let bytes = text.as_bytes();
    let mut numbers = Vec::new();
    let mut index = 0;
    while let Some(&byte) = bytes.get(index) {
        if !byte.is_ascii_digit() {
            index += 1;
            continue;
        }
        let digits = index;
        let mut end = index;
        while let Some(&byte) = bytes.get(end) {
            let decimal_point = byte == b'.' && bytes.get(end + 1).is_some_and(u8::is_ascii_digit);
            if !byte.is_ascii_digit() && !decimal_point {
                break;
            }
            end += 1;
        }
        index = end;
        let sign = digits
            .checked_sub(1)
            .filter(|&sign| matches!(bytes.get(sign), Some(b'+' | b'-')))
            .filter(|&sign| {
                !text
                    .get(..sign)
                    .and_then(|before| before.chars().next_back())
                    .is_some_and(char::is_alphanumeric)
            });
        let Some(magnitude) = text.get(digits..end).and_then(|number| number.parse().ok()) else {
            continue;
        };
        numbers.push(Number {
            start: sign.unwrap_or(digits),
            digits,
            end,
            magnitude,
            negative: sign.is_some_and(|sign| bytes.get(sign) == Some(&b'-')),
        });
    }
    numbers
}

/// How a number is written in a string of the stats.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Written {
    /// `#`, its sign in the value: `+15% to Fire Resistance` is `#% to Fire Resistance`.
    Hash,
    /// Its sign, then `#`: `-5% to all…` is `-#% to all…`.
    SignedHash,
    /// As in the line.
    Number,
}

/// A string to look up, the values of its `#`, and whether each number of the line is one.
struct Template {
    string: String,
    values: Vec<f64>,
    counted: Vec<bool>,
}

/// `line` with `#` in place of each of its numbers, its sign with it (`+15% to Fire Resistance`
/// is `#% to Fire Resistance`), and the values of the `#`, in their order: the first string the
/// line is looked up by, and how a line of an offer is compared with the same line of the
/// copied item. None when a number lies outside the line, which `numbers` never gives.
#[must_use]
pub fn line_template(line: &str) -> Option<(String, Vec<f64>)> {
    let numbers = numbers(line);
    let template = write(line, &numbers, &vec![Written::Hash; numbers.len()])?;
    Some((template.string, template.values))
}

/// The strings `text` can match, those with the most `#` first, then those without the
/// signs.
fn templates<'a>(text: &'a str, numbers: &'a [Number]) -> impl Iterator<Item = Template> + 'a {
    let mut choices: Vec<Vec<Written>> = vec![Vec::new()];
    if numbers.len() > MAX_NUMBERS {
        choices = vec![vec![Written::Hash; numbers.len()]];
    } else {
        for number in numbers {
            let ways: &[Written] = if number.start == number.digits {
                &[Written::Hash, Written::Number]
            } else {
                &[Written::Hash, Written::SignedHash, Written::Number]
            };
            choices = choices
                .into_iter()
                .flat_map(|choice| {
                    ways.iter().map(move |&way| {
                        let mut choice = choice.clone();
                        choice.push(way);
                        choice
                    })
                })
                .collect();
        }
    }
    choices.sort_by_key(|choice| {
        let count = |written| choice.iter().filter(|&&way| way == written).count();
        (count(Written::Number), count(Written::SignedHash))
    });
    choices
        .into_iter()
        .filter_map(move |choice| write(text, numbers, &choice))
}

/// `text` with each of its `numbers` written the way `choice` says, in their order; none when a
/// number lies outside `text`.
fn write(text: &str, numbers: &[Number], choice: &[Written]) -> Option<Template> {
    let mut string = String::with_capacity(text.len());
    let mut values = Vec::new();
    let mut written = 0;
    for (number, &way) in numbers.iter().zip(choice) {
        match way {
            Written::Hash => {
                string.push_str(text.get(written..number.start)?);
                string.push('#');
                values.push(number.value());
            }
            Written::SignedHash => {
                string.push_str(text.get(written..number.digits)?);
                string.push('#');
                values.push(number.magnitude);
            }
            Written::Number => string.push_str(text.get(written..number.end)?),
        }
        written = number.end;
    }
    string.push_str(text.get(written..)?);
    let counted = choice.iter().map(|&way| way != Written::Number).collect();
    Some(Template {
        string,
        values,
        counted,
    })
}

/// Every string with `#` of the stats that can be on equipment in `language`, with its English
/// string when it has one in another language.
#[cfg(test)]
#[must_use]
pub fn equipment_strings(language: Language) -> Vec<(&'static str, Option<&'static str>)> {
    let english = Stats::of(Language::English);
    Stats::of(language)
        .equipment_strings()
        .filter_map(|(stat, index)| {
            let twin = (language != Language::English)
                .then(|| english.twin(stat, index))
                .flatten();
            Some((
                stat.matchers.get(index)?.string.as_str(),
                twin.map(|twin| twin.string.as_str()),
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests;
