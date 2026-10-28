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
use crate::game_data::{self, TradeGroup, TradeStats};
use crate::item::{Modifier, StatLine};
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

/// How a line is searched on the trade site.
#[derive(Debug, Clone, PartialEq)]
pub enum LineStat {
    Searched(TradeStat),
    /// Another line of a stat written on several lines: the first line with a number of the
    /// stat searches it, or else the first line.
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

/// A limit as the search writes it, without the noise of the calculation (`37.500000001`).
fn written(value: f64) -> f64 {
    (value * 1e6).round() / 1e6
}

/// A modifier line of the copied item.
#[derive(Debug)]
struct Line {
    key: StatKey,
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
}

impl LineStats {
    /// The stats of the lines of `modifiers`, written in `language`; `trade` holds the stats
    /// of the trade site, once it gave them.
    #[must_use]
    pub fn new(language: Language, modifiers: &[Modifier], trade: Option<&TradeStats>) -> Self {
        let mut lines = Vec::new();
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
        Self { lines, pseudo }
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
mod tests {
    use super::*;
    use crate::filters::Property;
    use crate::item::{CopiedItem, Item, ModifierOrigin, Range};
    use crate::trade::pseudo::Pseudo;

    fn parse(text: &str) -> Item {
        Item::parse(&CopiedItem::from_text(text).expect("an item")).expect("read")
    }

    /// `lines`, each under the `header` of the advanced copy: a modifier of one line each.
    fn headed(header: &str, lines: &str) -> String {
        lines
            .lines()
            .map(|line| format!("{{ {header} }}\n{line}"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// A unique with the explicit modifier `lines`: one modifier each, or as their own
    /// `{ … }` headers group them.
    fn unique(lines: &str) -> Item {
        let modifiers = if lines.starts_with('{') {
            lines.to_owned()
        } else {
            headed("Unique Modifier", lines)
        };
        parse(&format!(
            "Item Class: Rings\nRarity: Unique\nTest Loop\nGold Ring\n--------\n\
             Item Level: 80\n--------\n{modifiers}"
        ))
    }

    fn line_stats(item: &Item) -> LineStats {
        LineStats::new(item.language(), item.modifiers(), None)
    }

    fn key(modifier: usize, line: usize) -> StatKey {
        StatKey { modifier, line }
    }

    /// The filters `filters`, as `LineStats::stat_filters` takes them.
    fn refs(filters: &[(StatKey, Filter)]) -> impl Iterator<Item = (StatKey, &Filter)> {
        filters.iter().map(|(key, filter)| (*key, filter))
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
        let filters =
            line_stats(&item).stat_filters(refs(&[(key(0, 0), filter(minimum, maximum))]));
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
    fn a_desecrated_or_fractured_line_is_searched_by_its_explicit_id() {
        let ids_of = |text: &str| -> Vec<Vec<String>> {
            let item = parse(text);
            let stats = line_stats(&item);
            item.modifiers()
                .iter()
                .enumerate()
                .filter(|(_, modifier)| {
                    matches!(
                        modifier.origin,
                        Some(ModifierOrigin::Desecrated | ModifierOrigin::Fractured)
                    )
                })
                .map(|(index, _)| match stats.get(key(index, 0)) {
                    Some(LineStat::Searched(stat)) => stat.ids.clone(),
                    other => panic!("searched: {other:?}"),
                })
                .collect()
        };

        assert_eq!(
            ids_of(include_str!(
                "../item/test-items/en-rare-crossbow-all-modifiers.txt"
            )),
            [["explicit.stat_3336890334"], ["explicit.stat_1368271171"]],
            "a fractured, then a desecrated line"
        );
        assert_eq!(
            ids_of(include_str!("../item/test-items/fr-rare-body-armour.txt")),
            [["explicit.stat_378817135"]],
            "a desecrated line written in French"
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
        // Its minimum and its maximum swap.
        assert_eq!(
            searched("20% increased Charm Charges used", "15", "25"),
            stat("explicit.stat_1570770415", Some(-25.0), Some(-15.0))
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
             Item Level: 80\n--------\n{ Unique Modifier }\nAdds 24 to 51 Physical Damage",
        );
        let stats = line_stats(&item);

        assert_eq!(
            stats.stat_filters(refs(&[(key(0, 0), filter("37.5", ""))])),
            [stat("explicit.stat_1940865751", Some(37.5), None)]
        );
    }

    #[test]
    fn a_number_written_in_the_string_of_a_stat_isnt_searched() {
        // The field holds the 5 only, the number the stat counts.
        assert_eq!(
            searched("1% more Unarmed Damage per 5 Strength", "5", "7"),
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
    fn a_stat_of_the_trade_site_is_searched_as_its_template_writes_it() {
        let trade = game_data::trade_stats();
        let item = unique(
            "20% reduced Damage per Patch\n\
             Sacrifice up to 10 Chaos Orbs to receive double on Trial completion",
        );
        let stats = LineStats::new(item.language(), item.modifiers(), Some(&trade));

        assert_eq!(
            stats.stat_filters(refs(&[
                (key(0, 0), filter("", "20")),
                (key(1, 0), filter("10", "")),
            ])),
            [
                // Its value is written as its template: 20, not -20.
                stat("explicit.stat_1000000001", None, Some(20.0)),
                // An option of the trade site, its number written in its template.
                stat("explicit.stat_3076483222|49977", None, None),
            ]
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
            line_stats(&item).stat_filters(refs(&[(key(0, 0), life)])),
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
        assert_eq!(stats.numbers(key(0, 0)), Some(&[0][..]));
        // The field holds the 20 only, not the 3 its string writes.
        let searched = stats.stat_filters(refs(&[
            (key(0, 0), filter("15", "25")),
            (key(0, 1), filter("", "")),
        ]));
        assert_eq!(searched.len(), 1, "{searched:?}");
        assert_eq!(searched[0].minimum, Some(15.0));
        assert_eq!(searched[0].maximum, Some(25.0));
    }

    #[test]
    fn a_stat_whose_numbers_are_on_its_second_line_is_searched_by_it() {
        let item = unique(
            "{ Unique Modifier — Life }\n\
             Life that would be lost by taking Damage is instead Reserved\n\
             until you take no Damage to Life for 5(4-6) seconds",
        );
        let stats = line_stats(&item);

        assert_eq!(stats.get(key(0, 0)), Some(&LineStat::Continued));
        assert_eq!(
            stats.stat_filters(refs(&[(key(0, 1), filter("", "5"))])),
            [stat("explicit.stat_1777740627", None, Some(5.0))]
        );
    }

    #[test]
    fn a_stat_is_searched_by_its_first_line_with_one_of_its_numbers() {
        let item = unique(
            "{ Unique Modifier — Damage }\n\
             Alternating every 5 seconds:\n\
             Take 20% more Damage from Hits\n\
             Take 20% more Damage over time",
        );
        let stats = line_stats(&item);

        // The 5 is written by the string of the stat.
        assert_eq!(stats.get(key(0, 0)), Some(&LineStat::Continued));
        assert_eq!(stats.numbers(key(0, 0)), Some(&[][..]));
        assert_eq!(stats.numbers(key(0, 1)), Some(&[0][..]));
        assert_eq!(stats.get(key(0, 2)), Some(&LineStat::Continued));
    }

    #[test]
    fn a_line_the_trade_site_cant_search_isnt_a_stat_filter() {
        let item = unique("+52 to maximum Life\nGain 3 Unknown Charges when you Dance");
        let stats = line_stats(&item);

        assert!(matches!(stats.get(key(0, 0)), Some(LineStat::Searched(_))));
        assert_eq!(stats.get(key(1, 0)), Some(&LineStat::Unsearchable));
        // Found nowhere: all its numbers count.
        assert_eq!(stats.numbers(key(1, 0)), None);
        assert_eq!(
            stats.stat_filters(refs(&[(key(1, 0), filter("3", ""))])),
            Vec::new()
        );
    }

    /// The modifier `lines` of a rare weapon, one prefix each (the stats don't depend on the
    /// affix), in `language`.
    fn weapon(language: Language, lines: &str) -> Item {
        let (item, prefix) = match language {
            Language::English => (
                "Item Class: Bows\nRarity: Rare\nDoom Song\nGemini Bow\n--------\nItem Level: 80",
                "Prefix Modifier",
            ),
            Language::French => (
                "Classe d'objet: Arcs\nRareté: Rare\nChant funeste\nArc jumelé\n--------\n\
                 Niveau de l'objet: 80",
                "Préfixe",
            ),
        };
        parse(&format!("{item}\n--------\n{}", headed(prefix, lines)))
    }

    #[test]
    fn the_damage_and_the_attack_speed_of_a_weapon_are_known_in_every_language() {
        let english = weapon(
            Language::English,
            "152% increased Physical Damage\nAdds 3 to 71 Lightning Damage\n\
             14% increased Attack Speed\n109% increased Elemental Damage with Attacks\n\
             Adds 5 to 9 Chaos Damage",
        );
        let french = weapon(
            Language::French,
            "152% d'Augmentation des Dégâts Physiques\nAjoute 21 à 38 Dégâts Physiques",
        );

        let damage = |item: &Item| {
            let stats = line_stats(item);
            (0..item.modifiers().len())
                .map(|modifier| stats.weapon_damage(key(modifier, 0)))
                .collect::<Vec<_>>()
        };
        // Not the elemental damage of its attacks, which any item gives; nor its chaos
        // damage, which neither DPS filter counts.
        assert_eq!(damage(&english), [true, true, true, false, false]);
        assert_eq!(damage(&french), [true, true]);
    }

    /// A rare ring with resistances, life and strength in several groups, in `language`.
    fn rare_ring(language: Language) -> Item {
        parse(match language {
            Language::English => {
                "Item Class: Rings\nRarity: Rare\nStorm Spiral\nSapphire Ring\n--------\n\
                 Item Level: 77\n--------\n+10% to Lightning Resistance (rune)\n--------\n\
                 { Implicit Modifier }\n+22% to Cold Resistance\n--------\n\
                 { Prefix Modifier }\n+65 to maximum Life\n\
                 { Suffix Modifier }\n+38% to Fire Resistance\n\
                 { Suffix Modifier }\n+10% to all Elemental Resistances\n\
                 { Suffix Modifier }\n+12 to Strength\n\
                 { Suffix Modifier }\n+23% to Chaos Resistance"
            }
            Language::French => {
                "Classe d'objet: Anneaux\nRareté: Rare\nSpirale\nAnneau de saphir\n--------\n\
                 Niveau de l'objet: 77\n--------\n+10% à la Résistance à la foudre (rune)\n\
                 --------\n{ Modificateur Implicite }\n+22% à la Résistance au froid\n\
                 --------\n{ Préfixe }\n+65 de Vie maximale\n\
                 { Suffixe }\n+38% à la Résistance au feu\n\
                 { Suffixe }\n+10% à toutes les Résistances Élémentaires\n\
                 { Suffixe }\n+12 de Force\n\
                 { Suffixe }\n+23% à la Résistance au chaos"
            }
        })
    }

    #[test]
    fn the_pseudo_totals_add_up_the_lines_of_every_group_in_every_language() {
        for language in [Language::English, Language::French] {
            let item = rare_ring(language);
            let stats = line_stats(&item);
            let totals: Vec<(Pseudo, f64)> = stats
                .pseudo_totals()
                .iter()
                .map(|total| (total.pseudo, total.value))
                .collect();

            assert_eq!(
                totals,
                [
                    // The rune, the implicit, and each element of all of them.
                    (Pseudo::ElementalResistance, 100.0),
                    (Pseudo::ChaosResistance, 23.0),
                    // Each point of strength twice.
                    (Pseudo::Life, 89.0),
                    (Pseudo::Strength, 12.0),
                ],
                "{language:?}"
            );
            for modifier in 0..item.modifiers().len() {
                assert!(stats.added_up(key(modifier, 0)), "{language:?} {modifier}");
            }
        }
    }

    #[test]
    fn a_negative_line_counts_against_its_total() {
        let item = parse(
            "Item Class: Rings\nRarity: Rare\nVile Loop\nAmethyst Ring\n--------\n\
             Item Level: 77\n--------\n{ Implicit Modifier }\n-13% to Chaos Resistance\n\
             --------\n{ Suffix Modifier }\n+17% to Fire and Chaos Resistances",
        );
        let totals: Vec<(Pseudo, f64)> = line_stats(&item)
            .pseudo_totals()
            .iter()
            .map(|total| (total.pseudo, total.value))
            .collect();

        assert_eq!(
            totals,
            [
                (Pseudo::ElementalResistance, 17.0),
                (Pseudo::ChaosResistance, 4.0),
            ]
        );
    }

    #[test]
    fn a_pseudo_total_is_searched_by_its_id_before_the_lines() {
        let item = rare_ring(Language::English);
        let stats = line_stats(&item);
        let filters = [
            (FilterKey::Stat(key(2, 0)), filter("60", "")),
            (
                FilterKey::Pseudo(Pseudo::ElementalResistance),
                filter("90", "120"),
            ),
            (FilterKey::Pseudo(Pseudo::Life), filter("", "")),
            (
                FilterKey::Pseudo(Pseudo::Strength),
                Filter {
                    checked: false,
                    ..filter("11", "")
                },
            ),
        ];

        assert_eq!(
            stats.search_filters(&filters).stats,
            [
                stat(
                    "pseudo.pseudo_total_elemental_resistance",
                    Some(90.0),
                    Some(120.0)
                ),
                stat("pseudo.pseudo_total_life", None, None),
                stat("explicit.stat_3299347043", Some(60.0), None),
            ]
        );
    }

    #[test]
    fn the_filters_of_a_search_are_those_checked_and_any_base_when_it_is_unchecked() {
        let item = unique("+52 to maximum Life\n+12 to Strength");
        let stats = line_stats(&item);
        let unchecked = |minimum| Filter {
            checked: false,
            ..filter(minimum, "")
        };
        let mut filters = vec![
            (FilterKey::BaseType, filter("", "")),
            (FilterKey::Property(Property::ItemLevel), unchecked("80")),
            (
                FilterKey::Property(Property::PhysicalDps),
                filter("220", ""),
            ),
            (FilterKey::Stat(key(0, 0)), filter("47", "")),
            (FilterKey::Stat(key(1, 0)), unchecked("11")),
        ];

        let searched = stats.search_filters(&filters);
        assert!(!searched.any_base);
        assert_eq!(
            searched.properties,
            [PropertyFilter {
                property: Property::PhysicalDps,
                minimum: Some(220.0),
                maximum: None,
            }]
        );
        assert_eq!(
            searched.stats,
            [stat("explicit.stat_3299347043", Some(47.0), None)]
        );
        filters[0].1.checked = false;
        assert!(stats.search_filters(&filters).any_base);
        // An item without filter of its base type, a unique, is searched by it.
        assert!(!stats.search_filters(&filters[1..]).any_base);
    }
}
