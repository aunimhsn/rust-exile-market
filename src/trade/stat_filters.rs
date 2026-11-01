//! The stat of each modifier line of the copied item on the trade site, read once when it is
//! copied: its ids, and how the value of its range field is written there; then the filters
//! of a search, from the filters the player checked, and the pseudo totals of the item.
//!
//! The value of a range field is the average of the numbers of its line the stat counts, its
//! `#` ([`LineStats::numbers`]), not those its string writes (the 3 of `#% chance to gain
//! Onslaught for 3 seconds`); the trade site reads the average of the numbers of the stat,
//! with the sign of the stat: a `reduced` line of an `increased` stat, or an inverted stat
//! (`#% less Damage`), has the other sign there, its minimum and its maximum swapped.

use super::pseudo::{self, CountedLine, PseudoTotal};
use super::query::{PropertyFilter, SearchFilters, StatFilter};
use crate::filters::{Filter, FilterKey, StatKey};
use crate::game_data::{self, AffixLimits, TradeGroup, TradeStats};
use crate::item::{Affix, Item, Modifier, Rarity, StatLine};
use crate::language::Language;

/// The game stats of the damage and the attack speed of a weapon, which its DPS counts.
const WEAPON_DAMAGE: [&str; 6] = [
    "local_physical_damage_+%",
    "local_minimum_added_physical_damage",
    "local_minimum_added_fire_damage",
    "local_minimum_added_cold_damage",
    "local_minimum_added_lightning_damage",
    "local_attack_speed_+%",
];

/// The stats of the trade site of the lines that add prefixes or suffixes to those an item
/// can have, or take some away: `+1 Suffix Modifier allowed` of a rune, `-1 Prefix Modifier
/// allowed` of an implicit (`local_maximum_prefixes_allowed_+` and
/// `local_maximum_suffixes_allowed_+` in the game data). By their trade stat, which a line
/// found by the templates of the trade site has too, not by their game id.
const AFFIXES_ALLOWED: [(&str, Affix); 2] = [
    ("stat_3182714256", Affix::Prefix),
    ("stat_718638445", Affix::Suffix),
];

/// How a line is searched on the trade site.
#[derive(Debug, Clone, PartialEq)]
pub enum LineStat {
    Searched(TradeStat),
    /// Another line of a stat written on several lines: the first line with a number of the
    /// stat searches it, or else the first line ([`LineStats::searched_line`]).
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
    /// `reduced` line of an `increased` stat; a share when the stat has numbers on other lines
    /// too (`Take #% more Damage from Hits` / `Take #% more Damage over time`).
    slope: f64,
}

impl TradeValue {
    /// The value on the trade site of `field`, a value of the range field.
    fn of(self, field: f64) -> f64 {
        self.trade + self.slope * (field - self.copied)
    }
}

impl TradeStat {
    /// The stat `found` of `line`, whose `numbers` it counts, in `group`; none when it has no
    /// id there, or is an option stat whose option isn't known.
    fn new(
        found: &game_data::StatMatch,
        group: TradeGroup,
        line: &StatLine,
        numbers: &[usize],
    ) -> Option<Self> {
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
            value: trade_value(found, line, numbers),
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

/// The value of `line` on the trade site, given its stat `found`, which counts its `numbers`;
/// none for a line or a stat without number.
fn trade_value(
    found: &game_data::StatMatch,
    line: &StatLine,
    numbers: &[usize],
) -> Option<TradeValue> {
    let rolls: Vec<f64> = numbers
        .iter()
        .filter_map(|&number| Some(line.rolls.get(number)?.value))
        .collect();
    let values = found.values.len();
    if rolls.is_empty() || values == 0 {
        return None;
    }
    let sign = |minus: bool| if minus { -1.0 } else { 1.0 };
    let inverted = sign(found.stat.inverted);
    // The values of the stat, written as the line writes them.
    let written_sign = sign(found.negate);
    #[expect(clippy::cast_precision_loss, reason = "a few numbers in a line")]
    let (numbers, values) = (rolls.len() as f64, values as f64);
    Some(TradeValue {
        copied: rolls.iter().sum::<f64>() / numbers,
        trade: inverted * found.values.iter().sum::<f64>() / values,
        // The field moves the numbers of the stat on its line by as much.
        slope: inverted * written_sign * numbers / values,
    })
}

/// For each line of `found`, a line of `modifier`, the indexes of its numbers the stat counts;
/// all of them when the numbers of the lines don't line up with those the stat read.
#[must_use]
pub fn counted_numbers(found: &game_data::LineMatch, modifier: &Modifier) -> Vec<Vec<usize>> {
    let counts: Vec<usize> = found
        .lines
        .clone()
        .map(|line| modifier.lines.get(line).map_or(0, |line| line.rolls.len()))
        .collect();
    let counted = found
        .stat
        .as_ref()
        .map(|stat| stat.counted.as_slice())
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

/// A number of affixes added up from the values of lines, rounded.
fn affixes(total: f64) -> i32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a few affixes; the cast saturates"
    )]
    let affixes = total.round() as i32;
    affixes
}

/// A limit as the search writes it, without the noise of the calculation (`37.500000001`).
fn written(value: f64) -> f64 {
    (value * 1e6).round() / 1e6
}

/// A modifier line of the copied item.
#[derive(Debug)]
struct Line {
    key: StatKey,
    /// The line of its modifier that searches its stat, which every line of the stat shares,
    /// also for a stat the trade site can't search; a line whose stat is unknown is its own:
    /// `game_data::modifier_stats` gives an unknown stat one line only.
    searched: usize,
    stat: LineStat,
    /// The indexes of its numbers its stat counts; none when its stat isn't known.
    numbers: Option<Vec<usize>>,
    /// The id of its stat in the files of the game (`base_maximum_life`), when known.
    game_id: Option<String>,
}

/// The stats of the modifier lines of the copied item, and its pseudo totals.
#[derive(Debug, Default)]
pub struct LineStats {
    lines: Vec<Line>,
    pseudo: Vec<PseudoTotal>,
    /// The prefixes the lines of [`AFFIXES_ALLOWED`] add, negative when they take some away.
    prefixes_allowed: i32,
    /// The suffixes they add.
    suffixes_allowed: i32,
}

impl LineStats {
    /// The stats of the lines of `modifiers`, written in `language`; `trade` holds the stats
    /// of the trade site, once it gave them.
    #[must_use]
    pub fn new(language: Language, modifiers: &[Modifier], trade: Option<&TradeStats>) -> Self {
        let mut lines = Vec::new();
        let (mut prefixes_allowed, mut suffixes_allowed) = (0.0, 0.0);
        for (index, modifier) in modifiers.iter().enumerate() {
            let group = TradeGroup::of(modifier);
            for found in game_data::modifier_stats(language, modifier, trade) {
                let numbers = counted_numbers(&found, modifier);
                // The first line with a number of the stat holds its filter: `Life that would
                // be lost… is instead Reserved` / `until you take no Damage to Life for #
                // seconds`.
                let offset = numbers
                    .iter()
                    .position(|line| !line.is_empty())
                    .unwrap_or(0);
                let searched = found.lines.start + offset;
                let stat = found
                    .stat
                    .as_ref()
                    .zip(modifier.lines.get(searched))
                    .and_then(|(stat, line)| {
                        let counted = numbers.get(offset).map_or(&[][..], Vec::as_slice);
                        TradeStat::new(stat, group, line, counted)
                    });
                let known = found.stat.is_some();
                let game_id = found.stat.as_ref().and_then(|found| found.stat.id.clone());
                if let Some(stat) = &found.stat
                    && let Some(&(_, affix)) = AFFIXES_ALLOWED
                        .iter()
                        .find(|(trade, _)| stat.stat.is_trade_stat(trade))
                    && let Some(&value) = stat.values.first()
                {
                    match affix {
                        Affix::Prefix => prefixes_allowed += value,
                        Affix::Suffix => suffixes_allowed += value,
                    }
                }
                for (line, numbers) in found.lines.zip(numbers) {
                    let key = StatKey {
                        modifier: index,
                        line,
                    };
                    let line_stat = match &stat {
                        Some(stat) if line == searched => LineStat::Searched(stat.clone()),
                        Some(_) => LineStat::Continued,
                        None => LineStat::Unsearchable,
                    };
                    lines.push(Line {
                        key,
                        searched,
                        stat: line_stat,
                        numbers: known.then_some(numbers),
                        game_id: game_id.clone(),
                    });
                }
            }
        }
        let counted: Vec<CountedLine<'_>> = lines
            .iter()
            .filter_map(|line| match &line.stat {
                LineStat::Searched(TradeStat {
                    value: Some(value), ..
                }) => Some(CountedLine {
                    key: line.key,
                    game_id: line.game_id.as_deref()?,
                    value: value.trade,
                }),
                _ => None,
            })
            .collect();
        let pseudo = pseudo::totals(&counted);
        Self {
            lines,
            pseudo,
            prefixes_allowed: affixes(prefixes_allowed),
            suffixes_allowed: affixes(suffixes_allowed),
        }
    }

    /// The stat of the line `key`; none for a line that isn't of the copied item.
    #[must_use]
    pub fn get(&self, key: StatKey) -> Option<&LineStat> {
        self.line(key).map(|line| &line.stat)
    }

    /// The indexes of the numbers of the line `key` its stat counts, also on a line it doesn't
    /// search; none when its stat isn't known, and all its numbers count.
    #[must_use]
    pub fn numbers(&self, key: StatKey) -> Option<&[usize]> {
        self.line(key)?.numbers.as_deref()
    }

    /// The stat of the line `key` is the damage or the attack speed of a weapon, which its DPS
    /// counts.
    #[must_use]
    pub fn weapon_damage(&self, key: StatKey) -> bool {
        self.line(key)
            .and_then(|line| line.game_id.as_deref())
            .is_some_and(|id| WEAPON_DAMAGE.contains(&id))
    }

    /// The pseudo totals of the item that are filters, for any item: the stats show them for
    /// a rare or magic one only.
    #[must_use]
    pub fn pseudo_totals(&self) -> &[PseudoTotal] {
        &self.pseudo
    }

    /// The line `key` is added up by a pseudo total, also on an item that doesn't show them.
    #[must_use]
    pub fn added_up(&self, key: StatKey) -> bool {
        self.pseudo.iter().any(|total| total.lines.contains(&key))
    }

    /// The prefixes or suffixes (`affix`) the lines of the item add to those it can have,
    /// negative when they take some away: a rune or an implicit, e.g. `+1 Suffix Modifier
    /// allowed`.
    #[must_use]
    pub fn affixes_allowed(&self, affix: Affix) -> i32 {
        match affix {
            Affix::Prefix => self.prefixes_allowed,
            Affix::Suffix => self.suffixes_allowed,
        }
    }

    /// How many prefixes and suffixes `item`, whose lines these are, can have: one of each
    /// for a magic item, two for a jewel or a rare tablet, three otherwise, with those its
    /// lines add or take away ([`Self::affixes_allowed`]). A corrupted rare waystone has no
    /// fixed limit of each: the corruption adds one of each (4 and 4) or makes them all of one
    /// affix (6 and none), 8 at most (40 offers of the trade site on 2026-10-31); six of
    /// each, its fill its count alone, nothing open, since nothing can change it any more.
    #[must_use]
    pub fn affix_limits(&self, item: &Item) -> AffixLimits {
        let category = game_data::category(item.language(), item.base_type());
        let corrupted_waystone =
            item.rarity() == Rarity::Rare && item.is_corrupted() && item.is_waystone();
        let usual = match item.rarity() {
            Rarity::Magic => 1,
            _ if item.is_jewel() || category == Some(game_data::Category::Tablet) => 2,
            _ if corrupted_waystone => 6,
            _ => 3,
        };
        let limit = |affix| u32::saturating_add_signed(usual, self.affixes_allowed(affix));
        AffixLimits {
            prefixes: limit(Affix::Prefix),
            suffixes: limit(Affix::Suffix),
            counted_only: corrupted_waystone,
        }
    }

    /// The line that searches the stat of the line `key`, the same for every line of a stat
    /// written on several lines, even when the trade site can't search it: the first line
    /// with a number of the stat, or else its first line; `key` itself when it is unknown.
    #[must_use]
    pub fn searched_line(&self, key: StatKey) -> StatKey {
        self.line(key).map_or(key, |line| StatKey {
            line: line.searched,
            ..key
        })
    }

    fn line(&self, key: StatKey) -> Option<&Line> {
        self.lines.iter().find(|line| line.key == key)
    }

    /// The filters of the search, from every filter of the item as the player set it: those
    /// checked, and any base type when the filter of the base type is unchecked.
    #[must_use]
    pub fn search_filters(&self, filters: &[(FilterKey, Filter)]) -> SearchFilters {
        let checked = || filters.iter().filter(|(_, filter)| filter.checked);
        let stats = checked().filter_map(|(key, filter)| match key {
            FilterKey::Stat(key) => Some((*key, filter)),
            FilterKey::BaseType | FilterKey::Property(_) | FilterKey::Pseudo(_) => None,
        });
        // A pseudo total is written as the trade site writes it.
        let pseudo = checked().filter_map(|(key, filter)| match key {
            FilterKey::Pseudo(pseudo) => {
                let (minimum, maximum) = filter.limits();
                Some(StatFilter {
                    ids: vec![pseudo.trade_id().to_owned()],
                    minimum: minimum.map(written),
                    maximum: maximum.map(written),
                })
            }
            FilterKey::Stat(_) | FilterKey::BaseType | FilterKey::Property(_) => None,
        });
        SearchFilters {
            any_base: filters
                .iter()
                .any(|(key, filter)| *key == FilterKey::BaseType && !filter.checked),
            properties: checked()
                .filter_map(|(key, filter)| match key {
                    FilterKey::Property(property) => {
                        let (minimum, maximum) = filter.limits();
                        Some(PropertyFilter {
                            property: *property,
                            minimum,
                            maximum,
                        })
                    }
                    FilterKey::Stat(_) | FilterKey::BaseType | FilterKey::Pseudo(_) => None,
                })
                .collect(),
            stats: pseudo.chain(self.stat_filters(stats)).collect(),
            // The state tags aren't lines: the item block lights them.
            states: Vec::new(),
        }
    }

    /// The stat filters of the search, from the filters the player `checked`: the lines the
    /// trade site can't search are left out.
    fn stat_filters<'f>(
        &self,
        checked: impl IntoIterator<Item = (StatKey, &'f Filter)>,
    ) -> Vec<StatFilter> {
        checked
            .into_iter()
            .filter_map(|(key, filter)| match self.get(key)? {
                LineStat::Searched(stat) => Some(stat.filter(filter)),
                LineStat::Continued | LineStat::Unsearchable => None,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests;
