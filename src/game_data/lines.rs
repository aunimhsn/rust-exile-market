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
    /// The value given by the string of the line: the option of an option stat (the skill of
    /// `# to Level of all Arc Skills`), or else the value of a line without number.
    pub matcher_value: Option<f64>,
    /// The index of the string of the stat that matched the line.
    matcher_index: usize,
}

/// The lines of a modifier written by one stat, or a line found nowhere.
#[derive(Debug, Clone, PartialEq)]
pub struct LineMatch<'a> {
    /// One line, or several for a stat written on several lines.
    pub lines: Range<usize>,
    pub stat: Option<StatMatch<'a>>,
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
                        stat_match(stat, source, matcher_index, &template.values)
                    })
            })
        });
        first_with_id(found, |stat| has_id(stat, group))
    }
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
    values: &[f64],
) -> Option<StatMatch<'a>> {
    let matcher = stat.matchers.get(matcher_index)?;
    let sign = if matcher.negate { -1.0 } else { 1.0 };
    Some(StatMatch {
        stat,
        source,
        values: values.iter().map(|value| value * sign).collect(),
        matcher_value: matcher.value,
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
    end: usize,
    magnitude: f64,
    negative: bool,
}

impl Number {
    fn value(&self) -> f64 {
        if self.negative {
            -self.magnitude
        } else {
            self.magnitude
        }
    }
}

/// The numbers of `text`. A sign right before a number is its own, unless it ends a word
/// (`X-10`); a point is in a number only between two digits (`2.5`, not `2 times.`).
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

/// A string to look up, and the values of its `#`.
struct Template {
    string: String,
    values: Vec<f64>,
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
    Some(Template { string, values })
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
mod tests {
    use super::*;
    use crate::game_data::trade_stats;
    use crate::item::{CopiedItem, Item, ModifierKind, ModifierOrigin, StatLine};

    fn modifier(kind: ModifierKind, origin: Option<ModifierOrigin>, lines: &[&str]) -> Modifier {
        Modifier {
            kind,
            origin,
            name: None,
            tier: None,
            tags: Vec::new(),
            lines: lines.iter().map(|line| StatLine::parse(line)).collect(),
        }
    }

    fn explicit(lines: &[&str]) -> Modifier {
        modifier(ModifierKind::Explicit(None), None, lines)
    }

    /// The stat of a modifier of one line.
    fn stat(language: Language, modifier: &Modifier) -> StatMatch<'static> {
        stat_with(language, modifier, None)
    }

    /// The stat of a modifier of one line, with the stats of the trade site `trade`.
    fn stat_with<'a>(
        language: Language,
        modifier: &Modifier,
        trade: Option<&'a TradeStats>,
    ) -> StatMatch<'a> {
        match modifier_stats(language, modifier, trade).as_slice() {
            [
                LineMatch {
                    stat: Some(stat), ..
                },
            ] => stat.clone(),
            other => panic!("{modifier:?}: {other:?}"),
        }
    }

    #[test]
    fn the_numbers_and_their_signs_are_read() {
        let numbers = numbers("+15% to -2.5 X-10 ricochets 2 times.");

        let read: Vec<f64> = numbers.iter().map(Number::value).collect();
        assert_eq!(read, [15.0, -2.5, 10.0, 2.0]);
        assert_eq!((numbers[0].start, numbers[0].digits), (0, 1));
        assert_eq!(numbers[2].start, numbers[2].digits);
    }

    #[test]
    fn a_line_finds_its_stat_and_its_values() {
        let found = stat(
            Language::English,
            &explicit(&["Adds 10(1-16) to 273(239-300) Lightning Damage"]),
        );

        assert_eq!(found.stat.reference, "Adds # to # Lightning Damage");
        assert_eq!(found.values, [10.0, 273.0]);
        assert_eq!(found.matcher_value, None);
    }

    #[test]
    fn the_sign_goes_into_the_value() {
        let found = stat(Language::English, &explicit(&["+42 to maximum Life"]));
        assert_eq!(
            (found.stat.reference.as_str(), found.values.as_slice()),
            ("# to maximum Life", [42.0].as_slice())
        );

        let found = stat(Language::English, &explicit(&["-10% to Cold Resistance"]));
        assert_eq!(found.values, [-10.0]);

        let found = stat(
            Language::English,
            &explicit(&["-5% to all Elemental Resistances per Socket filled"]),
        );
        assert_eq!(found.values, [-5.0]);
    }

    #[test]
    fn a_reduced_line_is_the_opposite_of_its_stat() {
        let found = stat(
            Language::English,
            &explicit(&["20% reduced Charm Charges used"]),
        );
        assert_eq!(found.stat.reference, "#% reduced Charm Charges used");
        assert_eq!(found.values, [-20.0]);

        let found = stat(
            Language::French,
            &explicit(&["Réduit la durée des saignements vous affectant de 41(45-41) %."]),
        );
        assert_eq!(found.values, [-41.0]);
    }

    #[test]
    fn a_number_written_in_the_string_stays_written() {
        let found = stat(
            Language::English,
            &explicit(&["25% chance for Skills to retain 40% of Glory on use"]),
        );
        assert!(found.stat.reference.contains("40% of Glory"));
        assert_eq!(found.values, [25.0]);

        let found = stat(
            Language::English,
            &explicit(&["Skills to retain 40% of Glory on use"]),
        );
        assert_eq!(found.values, Vec::<f64>::new());
        assert_eq!(found.matcher_value, Some(100.0));
    }

    #[test]
    fn a_stat_with_an_id_in_the_group_is_preferred_to_a_string_with_more_hashes() {
        for (language, line) in [
            (
                Language::English,
                "100% of Armour also applies to Lightning Damage",
            ),
            (
                Language::French,
                "100% de l'Armure s'applique également aux Dégâts de Foudre",
            ),
        ] {
            let found = stat(language, &explicit(&[line]));

            assert_eq!(
                found.stat.trade_ids(TradeGroup::Explicit),
                ["explicit.stat_2134207902"],
                "{language:?}"
            );
        }
    }

    #[test]
    fn beyond_six_numbers_only_the_string_with_every_hash_is_looked_up() {
        let text = "1 2 3 4 5 6 7";
        let numbers = numbers(text);

        let strings: Vec<String> = templates(text, &numbers)
            .map(|template| template.string)
            .collect();

        assert_eq!(strings, ["# # # # # # #"]);
        assert_eq!(templates("1 2", &super::numbers("1 2")).count(), 4);
    }

    #[test]
    fn a_string_without_number_gives_its_value() {
        let found = stat(Language::English, &explicit(&["Blind Enemies on Hit"]));

        assert_eq!(found.values, Vec::<f64>::new());
        assert_eq!(found.matcher_value, Some(100.0));
    }

    #[test]
    fn an_option_stat_gives_its_option() {
        let found = stat(
            Language::English,
            &explicit(&["+3 to Level of all Spark Skills"]),
        );

        assert!(found.stat.option);
        assert_eq!(found.values, [3.0]);
        assert_eq!(found.matcher_value, Some(154.0));
    }

    #[test]
    fn a_stat_on_several_lines_takes_them_together() {
        let modifier = explicit(&[
            "+30 to maximum Life",
            "Spells fire 2 additional Projectiles",
            "Spells fire Projectiles in a circle",
        ]);

        let matches = modifier_stats(Language::English, &modifier, None);

        let lines: Vec<_> = matches.iter().map(|found| found.lines.clone()).collect();
        assert_eq!(lines, [0..1, 1..3]);
        let projectiles = matches[1].stat.as_ref().unwrap();
        assert!(projectiles.stat.reference.contains("in a circle"));
        assert_eq!(projectiles.values, [2.0]);
    }

    #[test]
    fn a_line_found_nowhere_has_no_stat() {
        let matches = modifier_stats(
            Language::English,
            &explicit(&["Grants a strange power", "+30 to maximum Life"]),
            None,
        );

        assert_eq!(matches[0].stat, None);
        assert!(matches[1].stat.is_some());
    }

    #[test]
    fn the_group_of_the_line_picks_its_trade_id() {
        let fractured = modifier(
            ModifierKind::Explicit(None),
            Some(ModifierOrigin::Fractured),
            &["+42 to maximum Life"],
        );
        let found = stat(Language::English, &fractured);
        assert_eq!(
            found.stat.trade_ids(TradeGroup::of(&fractured)),
            ["fractured.stat_3299347043"]
        );

        let rune = modifier(ModifierKind::Rune, None, &["+12% to Fire Resistance"]);
        assert_eq!(TradeGroup::of(&rune), TradeGroup::Rune);
        assert_eq!(
            stat(Language::English, &rune)
                .stat
                .trade_ids(TradeGroup::Rune)
                .len(),
            1
        );
        let corruption = modifier(ModifierKind::Corruption, None, &[]);
        assert_eq!(TradeGroup::of(&corruption), TradeGroup::Enchant);
    }

    #[test]
    fn a_local_and_a_global_stat_keep_both_ids() {
        let found = stat(Language::English, &explicit(&["+120 to Accuracy Rating"]));

        assert_eq!(found.stat.trade_ids(TradeGroup::Explicit).len(), 2);
    }

    #[test]
    fn a_line_the_game_data_doesnt_know_is_found_on_the_trade_site() {
        let trade = trade_stats();
        let line = "Sacrifice up to 10 Chaos Orbs to receive double on Trial completion";

        let without = modifier_stats(Language::English, &explicit(&[line]), None);
        assert_eq!(without[0].stat, None, "not in the game data");

        let found = stat_with(Language::English, &explicit(&[line]), Some(&trade));
        assert_eq!(found.source, StatSource::TradeSite);
        assert_eq!(
            found.stat.trade_ids(TradeGroup::Explicit),
            ["explicit.stat_3076483222"]
        );
        assert_eq!(found.values, Vec::<f64>::new());
        assert_eq!(found.matcher_value, Some(49977.0), "its option");
    }

    #[test]
    fn a_line_without_id_in_its_group_takes_the_one_of_the_trade_site() {
        let trade = trade_stats();
        let rune = modifier(
            ModifierKind::Rune,
            None,
            &["+2 to Level of all Mark Skills"],
        );

        let known = stat(Language::English, &rune);
        assert_eq!(known.source, StatSource::GameData);
        assert_eq!(known.stat.trade_ids(TradeGroup::Rune), [] as [String; 0]);

        let found = stat_with(Language::English, &rune, Some(&trade));
        assert_eq!(found.source, StatSource::TradeSite);
        assert_eq!(
            found.stat.trade_ids(TradeGroup::Rune),
            ["rune.stat_1992191903"]
        );
        assert_eq!(found.values, [2.0]);
    }

    #[test]
    fn a_line_of_the_game_data_is_taken_from_it_first() {
        let trade = trade_stats();

        let found = stat_with(
            Language::English,
            &explicit(&["+42 to maximum Life"]),
            Some(&trade),
        );

        assert_eq!(found.source, StatSource::GameData);
        assert_eq!(found.stat.id.as_deref(), Some("base_maximum_life"));
    }

    #[test]
    fn a_stat_of_the_trade_site_on_several_lines_takes_them_together() {
        let trade = trade_stats();
        let modifier = explicit(&["+30 to maximum Life", "Gain 3 Patches", "while it Patches"]);

        let matches = modifier_stats(Language::English, &modifier, Some(&trade));

        let lines: Vec<_> = matches.iter().map(|found| found.lines.clone()).collect();
        assert_eq!(lines, [0..1, 1..3]);
        let patches = matches[1].stat.as_ref().unwrap();
        assert_eq!(patches.source, StatSource::TradeSite);
        assert_eq!(patches.values, [3.0]);
    }

    #[test]
    fn a_french_line_of_the_trade_site_has_the_english_template_of_its_id() {
        let trade = trade_stats();
        let english =
            |text| english_string(Language::French, text, TradeGroup::Explicit, Some(&trade));

        assert_eq!(
            english("12% de Réduction des Dégâts par patch"),
            Some("#% reduced Damage per Patch")
        );
        assert_eq!(
            english("12% de Réduction de quelque chose"),
            Some("#% de Réduction de quelque chose"),
            "no English template: its own"
        );
        assert_eq!(
            english_string(
                Language::French,
                "12% de Réduction des Dégâts par patch",
                TradeGroup::Explicit,
                None
            ),
            None
        );
    }

    macro_rules! test_item {
        ($file:literal) => {
            ($file, include_str!(concat!("../item/test-items/", $file)))
        };
    }

    /// The copied items of the parser tests (`src/item/test-items/`).
    const TEST_ITEMS: [(&str, &str); 19] = [
        test_item!("en-currency-uncut-gem.txt"),
        test_item!("en-magic-mace.txt"),
        test_item!("en-normal-helmet.txt"),
        test_item!("en-owner-unique-mace-fixed-values.txt"),
        test_item!("en-plain-currency-large-stack.txt"),
        test_item!("en-plain-magic-jewel-unidentified.txt"),
        test_item!("en-plain-rare-ring-unidentified.txt"),
        test_item!("en-plain-rare-ring.txt"),
        test_item!("en-plain-unique-jewel.txt"),
        test_item!("en-rare-bow-fractured.txt"),
        test_item!("en-rare-crossbow-all-modifiers.txt"),
        test_item!("en-rare-wand-unidentified.txt"),
        test_item!("en-unique-focus.txt"),
        test_item!("fr-currency-stack.txt"),
        test_item!("fr-normal-exceptional-shield.txt"),
        test_item!("fr-rare-body-armour.txt"),
        test_item!("fr-rare-staff.txt"),
        test_item!("fr-unique-crossbow.txt"),
        test_item!("fr-unique-mace-fixed-values.txt"),
    ];

    /// Every modifier line of the copied items of the parser tests finds its stat, and an
    /// id in its group.
    #[test]
    fn every_line_of_the_copied_items_finds_its_stat() {
        let mut missing = Vec::new();
        let mut found = 0;
        for (name, text) in TEST_ITEMS {
            let item = Item::parse(&CopiedItem::from_text(text).unwrap()).unwrap();
            for modifier in item.modifiers() {
                let group = TradeGroup::of(modifier);
                for line in modifier_stats(item.language(), modifier, None) {
                    let texts: Vec<&str> = modifier.lines[line.lines.clone()]
                        .iter()
                        .map(|line| line.text.as_str())
                        .collect();
                    match line.stat {
                        Some(stat) if !stat.stat.trade_ids(group).is_empty() => found += 1,
                        Some(_) => missing.push(format!("{name}: no {group:?} id: {texts:?}")),
                        None => missing.push(format!("{name}: {texts:?}")),
                    }
                }
            }
        }

        assert!(missing.is_empty(), "{missing:#?}");
        assert!(found >= 65, "{found} lines");
    }
}
