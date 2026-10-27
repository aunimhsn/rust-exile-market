//! The stat of each modifier line of the copied item on the trade site, read once when it is
//! copied: its ids, and how the value of its range field is written there; then the stat
//! filters of a search, from the filters the player checked.
//!
//! The value of a range field is the average of the numbers of its line (see
//! `ui::item_stats`); the trade site reads the average of the numbers of the stat, with the
//! sign of the stat: a `reduced` line of an `increased` stat, or an inverted stat (`#% less
//! Damage`), has the other sign there, its minimum and its maximum swapped.

use super::query::StatFilter;
use crate::filters::{Filter, StatKey};
use crate::game_data::{self, TradeGroup, TradeStats};
use crate::item::{Modifier, StatLine};
use crate::language::Language;

/// How a line is searched on the trade site.
#[derive(Debug, Clone, PartialEq)]
pub enum LineStat {
    Searched(TradeStat),
    /// A line after the first of a stat written on several lines: the first one searches it.
    Continued,
    /// A line without trade id in its group: a modifier added by a patch the game data
    /// doesn't know yet, or a line the parser read wrong.
    Unsearchable,
}

/// The stat of a line on the trade site.
#[derive(Debug, Clone, PartialEq)]
pub struct TradeStat {
    /// Its id, with the option of an option stat (`explicit.stat_448592698|98`); or a local
    /// and a global one written the same (`# to Accuracy Rating`).
    ids: Vec<String>,
    /// None for a line without number: its stat alone is searched.
    value: Option<TradeValue>,
}

/// The value of a line on the trade site, from the value of its range field.
#[derive(Debug, Clone, Copy, PartialEq)]
struct TradeValue {
    /// The value of the range field when copied.
    copied: f64,
    /// The value of the copied line on the trade site.
    trade: f64,
    /// How much the value on the trade site changes with the value of the field: -1 for a
    /// `reduced` line of an `increased` stat; another number when the line writes a number
    /// its stat doesn't count (`1% more Unarmed Damage per # Strength`).
    slope: f64,
}

impl TradeValue {
    /// The value on the trade site of `field`, a value of the range field.
    fn of(self, field: f64) -> f64 {
        self.trade + self.slope * (field - self.copied)
    }
}

impl TradeStat {
    /// The stat `found` of `line`, the first line of its stat, in `group`; none when it has no
    /// id there, or is an option stat whose option isn't known.
    fn new(found: &game_data::StatMatch, group: TradeGroup, line: &StatLine) -> Option<Self> {
        let ids = found.stat.trade_ids(group);
        if ids.is_empty() {
            return None;
        }
        let ids = if found.stat.option {
            let option = found.matcher_value?;
            ids.iter().map(|id| format!("{id}|{option}")).collect()
        } else {
            ids.to_vec()
        };
        Some(Self {
            ids,
            value: trade_value(found, line),
        })
    }

    /// The filter of the search with the limits of `filter`, the minimum never above the
    /// maximum.
    fn filter(&self, filter: &Filter) -> StatFilter {
        let (minimum, maximum) = match (self.value, filter.limits()) {
            (Some(value), (minimum, maximum)) => {
                let minimum = minimum.map(|field| written(value.of(field)));
                let maximum = maximum.map(|field| written(value.of(field)));
                if value.slope < 0.0 {
                    (maximum, minimum)
                } else {
                    (minimum, maximum)
                }
            }
            (None, _) => (None, None),
        };
        StatFilter {
            ids: self.ids.clone(),
            minimum,
            maximum,
        }
    }
}

/// The value of `line` on the trade site, given its stat `found`; none for a line or a stat
/// without number.
fn trade_value(found: &game_data::StatMatch, line: &StatLine) -> Option<TradeValue> {
    let numbers = line.rolls.len();
    let values = found.values.len();
    if numbers == 0 || values == 0 {
        return None;
    }
    let sign = |minus: bool| if minus { -1.0 } else { 1.0 };
    let inverted = sign(found.stat.inverted);
    // The values of the stat, written as the line writes them.
    let written_sign = sign(found.negate);
    #[expect(clippy::cast_precision_loss, reason = "a few numbers in a line")]
    let (numbers, values) = (numbers as f64, values as f64);
    Some(TradeValue {
        copied: line.rolls.iter().map(|roll| roll.value).sum::<f64>() / numbers,
        trade: inverted * found.values.iter().sum::<f64>() / values,
        // The field moves the numbers of the line, those of the stat among them, by as much.
        slope: inverted * written_sign * numbers / values,
    })
}

/// A limit as the search writes it, without the noise of the calculation (`37.500000001`).
fn written(value: f64) -> f64 {
    (value * 1e6).round() / 1e6
}

/// The stats of the modifier lines of the copied item.
#[derive(Debug, Default)]
pub struct LineStats(Vec<(StatKey, LineStat)>);

impl LineStats {
    /// The stats of the lines of `modifiers`, written in `language`; `trade` holds the stats
    /// of the trade site, once it gave them.
    #[must_use]
    pub fn new(language: Language, modifiers: &[Modifier], trade: Option<&TradeStats>) -> Self {
        let mut lines = Vec::new();
        for (index, modifier) in modifiers.iter().enumerate() {
            let group = TradeGroup::of(modifier);
            for found in game_data::modifier_stats(language, modifier, trade) {
                let first = found.lines.start;
                let stat = found
                    .stat
                    .as_ref()
                    .zip(modifier.lines.get(first))
                    .and_then(|(stat, line)| TradeStat::new(stat, group, line));
                for line in found.lines {
                    let key = StatKey {
                        modifier: index,
                        line,
                    };
                    let line_stat = match &stat {
                        Some(stat) if line == first => LineStat::Searched(stat.clone()),
                        Some(_) => LineStat::Continued,
                        None => LineStat::Unsearchable,
                    };
                    lines.push((key, line_stat));
                }
            }
        }
        Self(lines)
    }

    /// The stat of the line `key`; none for a line that isn't of the copied item.
    #[must_use]
    pub fn get(&self, key: StatKey) -> Option<&LineStat> {
        self.0
            .iter()
            .find(|(line, _)| *line == key)
            .map(|(_, stat)| stat)
    }

    /// The stat filters of the search, from the filters the player `checked`: the lines the
    /// trade site can't search are left out.
    #[must_use]
    pub fn stat_filters(&self, checked: &[(StatKey, Filter)]) -> Vec<StatFilter> {
        checked
            .iter()
            .filter_map(|(key, filter)| match self.get(*key)? {
                LineStat::Searched(stat) => Some(stat.filter(filter)),
                LineStat::Continued | LineStat::Unsearchable => None,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::{CopiedItem, Item, Range};

    fn parse(text: &str) -> Item {
        Item::parse(&CopiedItem::from_text(text).expect("an item")).expect("read")
    }

    /// A unique with the explicit modifier `lines`.
    fn unique(lines: &str) -> Item {
        parse(&format!(
            "Item Class: Rings\nRarity: Unique\nTest Loop\nGold Ring\n--------\n\
             Item Level: 80\n--------\n{lines}"
        ))
    }

    fn line_stats(item: &Item) -> LineStats {
        LineStats::new(item.language(), item.modifiers(), None)
    }

    fn key(modifier: usize, line: usize) -> StatKey {
        StatKey { modifier, line }
    }

    fn filter(minimum: &str, maximum: &str) -> Filter {
        Filter {
            checked: true,
            minimum: minimum.to_owned(),
            maximum: maximum.to_owned(),
            bounds: None,
        }
    }

    /// The stat filter of the first line of `lines`, with the limits `minimum` and `maximum`.
    fn searched(lines: &str, minimum: &str, maximum: &str) -> StatFilter {
        let item = unique(lines);
        let filters = line_stats(&item).stat_filters(&[(key(0, 0), filter(minimum, maximum))]);
        filters.into_iter().next().expect("a stat filter")
    }

    fn stat(id: &str, minimum: Option<f64>, maximum: Option<f64>) -> StatFilter {
        StatFilter {
            ids: vec![id.to_owned()],
            minimum,
            maximum,
        }
    }

    #[test]
    fn a_line_is_searched_by_the_id_of_its_stat_with_its_limits() {
        assert_eq!(
            searched("+52 to maximum Life", "45", "60"),
            stat("explicit.stat_3299347043", Some(45.0), Some(60.0))
        );
        assert_eq!(
            searched("+52 to maximum Life", "", ""),
            stat("explicit.stat_3299347043", None, None)
        );
    }

    #[test]
    fn a_reduced_line_of_an_increased_stat_swaps_its_limits() {
        // At most 18% reduced: at least -18% increased.
        assert_eq!(
            searched("18% reduced Movement Speed", "", "18"),
            stat("explicit.stat_2250533757", Some(-18.0), None)
        );
        assert_eq!(
            searched("18% reduced Movement Speed", "15", "25"),
            stat("explicit.stat_2250533757", Some(-25.0), Some(-15.0))
        );
        assert_eq!(
            searched("10% increased Movement Speed", "8", ""),
            stat("explicit.stat_2250533757", Some(8.0), None)
        );
    }

    #[test]
    fn an_inverted_stat_has_the_other_sign_on_the_trade_site() {
        // The trade site counts the reduced charges used: an increase is a negative value.
        assert_eq!(
            searched("20% increased Charm Charges used", "", "20"),
            stat("explicit.stat_1570770415", Some(-20.0), None)
        );
        // A reduced line of an inverted stat keeps its sign and its limits.
        assert_eq!(
            searched("20% reduced Charm Charges used", "15", ""),
            stat("explicit.stat_1570770415", Some(15.0), None)
        );
    }

    #[test]
    fn a_line_of_several_numbers_is_searched_at_their_average() {
        let item = parse(
            "Item Class: One Hand Maces\nRarity: Unique\nTest Mace\nWooden Club\n--------\n\
             Item Level: 80\n--------\nAdds 24 to 51 Physical Damage",
        );
        let stats = line_stats(&item);

        assert_eq!(
            stats.stat_filters(&[(key(0, 0), filter("37.5", ""))]),
            [stat("explicit.stat_1940865751", Some(37.5), None)]
        );
    }

    #[test]
    fn a_number_written_in_the_string_of_a_stat_isnt_searched() {
        // The field holds the average of 1 and 5, the trade site counts the 5 only.
        assert_eq!(
            searched("1% more Unarmed Damage per 5 Strength", "3", "4"),
            stat("explicit.stat_3452816629", Some(5.0), Some(7.0))
        );
    }

    #[test]
    fn an_option_stat_is_searched_by_its_option() {
        assert_eq!(
            searched("+2 to Level of all Arc Skills", "2", ""),
            stat("explicit.stat_448592698|98", Some(2.0), None)
        );
    }

    #[test]
    fn a_local_and_a_global_stat_are_both_searched() {
        assert_eq!(
            searched("+150 to Accuracy Rating", "150", "").ids,
            ["explicit.stat_691932474", "explicit.stat_803737631"]
        );
    }

    #[test]
    fn a_minimum_above_the_maximum_is_never_searched() {
        assert_eq!(
            searched("+52 to maximum Life", "55", "50"),
            stat("explicit.stat_3299347043", Some(55.0), Some(55.0))
        );
        assert_eq!(
            searched("18% reduced Movement Speed", "20", "16"),
            stat("explicit.stat_2250533757", Some(-20.0), Some(-20.0))
        );
    }

    #[test]
    fn the_limits_are_searched_within_the_range_of_the_stat() {
        let item = unique("+52 to maximum Life");
        let mut life = filter("10", "90");
        life.bounds = Some(Range {
            min: 40.0,
            max: 60.0,
        });

        assert_eq!(
            line_stats(&item).stat_filters(&[(key(0, 0), life)]),
            [stat("explicit.stat_3299347043", Some(40.0), Some(60.0))]
        );
    }

    #[test]
    fn a_stat_written_on_two_lines_is_searched_by_its_first_line() {
        let item = unique(
            "{ Unique Modifier — Speed }\n\
             20(15-25)% chance to gain Onslaught for 3 seconds when you kill an\n\
             enemy affected by Abyssal Wasting",
        );
        let stats = line_stats(&item);

        assert_eq!(stats.get(key(0, 1)), Some(&LineStat::Continued));
        // The field holds the average of 20 and 3, the trade site counts the 20 only: the
        // field from 9 to 14 is a roll from 15 to 25.
        let searched =
            stats.stat_filters(&[(key(0, 0), filter("9", "14")), (key(0, 1), filter("", ""))]);
        assert_eq!(searched.len(), 1, "{searched:?}");
        assert_eq!(searched[0].minimum, Some(15.0));
        assert_eq!(searched[0].maximum, Some(25.0));
    }

    #[test]
    fn a_line_the_trade_site_cant_search_isnt_a_stat_filter() {
        let item = unique("+52 to maximum Life\nGain 3 Unknown Charges when you Dance");
        let stats = line_stats(&item);

        assert!(matches!(stats.get(key(0, 0)), Some(LineStat::Searched(_))));
        assert_eq!(stats.get(key(1, 0)), Some(&LineStat::Unsearchable));
        assert_eq!(
            stats.stat_filters(&[(key(1, 0), filter("3", ""))]),
            Vec::new()
        );
    }
}
