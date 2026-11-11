//! The stat of each modifier line of the copied item on the trade site, read once when it is
//! copied: its ids, and how the value of its range field is written there; then the filters
//! of a search, from the filters the player checked, and the pseudo totals of the item.
//!
//! The value of a range field is the average of the numbers of its line the stat counts, its
//! `#` ([`LineStats::numbers`]), not those its string writes (the 3 of `#% chance to gain
//! Onslaught for 3 seconds`); the trade site reads the average of the numbers of the stat,
//! with the sign of the stat: a `reduced` line of an `increased` stat, or an inverted stat
//! (`#% less Damage`), has the other sign there, its minimum and its maximum swapped.

use super::pseudo::{self, CountedLine, Pseudo, PseudoTotal};
use super::query::{PropertyFilter, RulesOut, SearchFilters, StatFilter};
use crate::filters::{Filter, FilterKey, Property, StatKey};
use crate::game_data::{self, AffixLimits, TradeGroup, TradeStats};
use crate::item::{
    Affix, BaseDps, Item, Modifier, ModifierKind, Range, Rarity, StatLine, USUAL_QUALITY,
    WeaponLines,
};

/// The stats of the game of the lines of a weapon its DPS counts: its increased physical
/// damage, its added physical damage, its attack speed; its added elemental damage.
const INCREASED_PHYSICAL: &str = "local_physical_damage_+%";
const ADDED_PHYSICAL: &str = "local_minimum_added_physical_damage";
const ATTACK_SPEED: &str = "local_attack_speed_+%";
const ADDED_ELEMENTAL: [&str; 3] = [
    "local_minimum_added_fire_damage",
    "local_minimum_added_cold_damage",
    "local_minimum_added_lightning_damage",
];

/// The lines each DPS counts.
const PHYSICAL_DPS: [&str; 3] = [INCREASED_PHYSICAL, ADDED_PHYSICAL, ATTACK_SPEED];
const ELEMENTAL_DPS: [&str; 4] = [
    ADDED_ELEMENTAL[0],
    ADDED_ELEMENTAL[1],
    ADDED_ELEMENTAL[2],
    ATTACK_SPEED,
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
    fn filter(&self, filter: &Filter, rules_out: RulesOut) -> StatFilter {
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
            rules_out,
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

/// The DPS the base of `item`, a weapon, reaches with the `lines` of the item that any item
/// of the base may have (its runes), the physical one at the usual quality
/// ([`Weapon::base_dps`](crate::item::Weapon::base_dps)); none for another item.
fn dps_reach(item: &Item, lines: &[Line]) -> Option<BaseDps> {
    let weapon = item.weapon()?;
    // What the lines from one of `sources` add; never those of an implicit, which the
    // properties count as the base's.
    let sum = |sources: &[Source]| {
        let of = |ids: &[&str]| -> f64 {
            lines
                .iter()
                .filter(|line| sources.contains(&line.source) && line.is_one_of(ids))
                .filter_map(|line| Some(line.trade_value()?.trade))
                .sum()
        };
        WeaponLines {
            increased_physical: of(&[INCREASED_PHYSICAL]),
            added_physical: of(&[ADDED_PHYSICAL]),
            added_elemental: of(&ADDED_ELEMENTAL),
            attack_speed: of(&[ATTACK_SPEED]),
        }
    };
    let quality = f64::from(item.quality().unwrap_or(0));
    Some(weapon.base_dps(
        quality,
        sum(&[Source::Modifier, Source::Added]),
        sum(&[Source::Added]),
    ))
}

/// What the quality of a catalyst may multiply an implicit of `item` by, on any item of its
/// base: the usual quality, or its own when higher, on a ring, an amulet or a jewel, whose
/// quality is a catalyst's, which raises the modifiers of its kind, an implicit too (the
/// advanced copy writes the range of the implicit without it: `+10(7-10)%` at 20 %); as much
/// on a base the game data doesn't know, which may be one, a magic item among them, whose
/// name doesn't tell its base alone; nothing on another item.
fn catalyst_factor(item: &Item) -> f64 {
    let category = game_data::category(item.language(), item.base_type());
    if category.is_none_or(game_data::Category::takes_catalysts) {
        1.0 + f64::from(item.quality().unwrap_or(0).max(USUAL_QUALITY)) / 100.0
    } else {
        1.0
    }
}

/// What a line from `source` gives on the trade site to any item of the base
/// ([`Line::reach`]): `value` is its value there, read from the `numbers` its `line` counts;
/// `headed` and `catalyst` tell the best roll of an implicit ([`best_roll`],
/// [`catalyst_factor`]).
fn reach(
    source: Source,
    value: TradeValue,
    line: &StatLine,
    numbers: &[usize],
    headed: bool,
    catalyst: f64,
) -> f64 {
    match source {
        Source::Modifier => 0.0,
        Source::Added => value.trade,
        // A catalyst raises it away from zero: no higher when it counts against a total.
        Source::Implicit => best_roll(value, line, numbers, headed)
            .map_or(f64::INFINITY, |best| best.max(best * catalyst)),
    }
}

/// The value on the trade site of `line`, of the trade `value` read from the `numbers` it
/// counts, at its best roll: the end of their ranges that gives the most. A number written
/// without range is a fixed value in the advanced copy (`headed`); in a copy from the in-game
/// trade, which writes no range, its best roll is unknown: none.
fn best_roll(value: TradeValue, line: &StatLine, numbers: &[usize], headed: bool) -> Option<f64> {
    let field = |end: fn(&Range) -> f64| -> Option<f64> {
        let ends: Vec<f64> = numbers
            .iter()
            .map(|&number| {
                let roll = line.rolls.get(number)?;
                match &roll.range {
                    Some(range) => Some(end(range)),
                    None => headed.then_some(roll.value),
                }
            })
            .collect::<Option<_>>()?;
        #[expect(clippy::cast_precision_loss, reason = "a few numbers in a line")]
        let count = ends.len() as f64;
        Some(ends.iter().sum::<f64>() / count)
    };
    let (highest, lowest) = (field(|range| range.max)?, field(|range| range.min)?);
    Some(value.of(highest).max(value.of(lowest)))
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
    source: Source,
    /// What the line gives on the trade site to any item of the base, which a filter must go
    /// beyond to compare the item: nothing for a line of a modifier; for an implicit, its best
    /// roll, raised by a catalyst on a jewel, infinite when unknown (a copy from the in-game
    /// trade); for a rune or an enchantment, its value.
    reach: f64,
}

impl Line {
    /// Its value on the trade site, on the line that searches its stat.
    fn trade_value(&self) -> Option<TradeValue> {
        match &self.stat {
            LineStat::Searched(TradeStat { value, .. }) => *value,
            LineStat::Continued | LineStat::Unsearchable => None,
        }
    }

    /// Its stat of the game is one of `ids`.
    fn is_one_of(&self, ids: &[&str]) -> bool {
        self.game_id.as_deref().is_some_and(|id| ids.contains(&id))
    }
}

/// Where a line comes from, which tells whether it compares the item with the other items of
/// its base.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Source {
    /// A modifier rolled on this item, which no other item of its base has: a prefix, a
    /// suffix (fractured, desecrated, crafted too), or what its corruption added.
    Modifier,
    /// An implicit modifier, which comes with the base.
    Implicit,
    /// A rune or an enchantment, which goes on any item of the base.
    Added,
}

impl Source {
    /// The source of the lines of a modifier of `kind`: by its kind on the item, not by its
    /// group on the trade site, which lists what a corruption added with the enchantments.
    fn of(kind: ModifierKind) -> Self {
        match kind {
            ModifierKind::Explicit(_) | ModifierKind::Corruption => Self::Modifier,
            ModifierKind::Implicit => Self::Implicit,
            ModifierKind::Rune | ModifierKind::Enchant => Self::Added,
        }
    }
}

/// The stats of the modifier lines of the copied item, and its pseudo totals.
#[derive(Debug, Default)]
pub struct LineStats {
    lines: Vec<Line>,
    pseudo: Vec<PseudoTotal>,
    /// What each pseudo total reaches with the lines of the item that any item of its base may
    /// have ([`Line::reach`]), the others left out.
    pseudo_reach: Vec<(Pseudo, f64)>,
    /// The DPS the base of a weapon reaches with them, the physical one at the usual quality.
    dps_reach: Option<BaseDps>,
    /// The prefixes the lines of [`AFFIXES_ALLOWED`] add, negative when they take some away.
    prefixes_allowed: i32,
    /// The suffixes they add.
    suffixes_allowed: i32,
}

impl LineStats {
    /// The stats of the lines of `modifiers`, those of `item` or another reading of them, in
    /// the language of `item`; `trade` holds the stats of the trade site, once it gave them.
    #[must_use]
    pub fn new(item: &Item, modifiers: &[Modifier], trade: Option<&TradeStats>) -> Self {
        let language = item.language();
        // What tells the best roll of an implicit on any item of the base.
        let (headed, catalyst) = (item.is_headed(), catalyst_factor(item));
        let mut lines = Vec::new();
        let (mut prefixes_allowed, mut suffixes_allowed) = (0.0, 0.0);
        for (index, modifier) in modifiers.iter().enumerate() {
            let group = TradeGroup::of(modifier);
            let source = Source::of(modifier.kind);
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
                let searched_line = modifier.lines.get(searched);
                let counted = numbers.get(offset).map_or(&[][..], Vec::as_slice);
                let stat = found
                    .stat
                    .as_ref()
                    .zip(searched_line)
                    .and_then(|(stat, line)| TradeStat::new(stat, group, line, counted));
                // A line without number gives a total nothing.
                let reach = stat
                    .as_ref()
                    .and_then(|stat| stat.value)
                    .zip(searched_line)
                    .map_or(0.0, |(value, line)| {
                        reach(source, value, line, counted, headed, catalyst)
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
                        source,
                        reach,
                    });
                }
            }
        }
        // The lines a pseudo total adds up, each with the value `value` gives it.
        let counted = |value: fn(&Line, TradeValue) -> f64| -> Vec<CountedLine<'_>> {
            lines
                .iter()
                .filter_map(|line| {
                    let trade = line.trade_value()?;
                    Some(CountedLine {
                        key: line.key,
                        game_id: line.game_id.as_deref()?,
                        value: value(line, trade),
                    })
                })
                .collect()
        };
        let pseudo = pseudo::totals(&counted(|_, trade| trade.trade));
        let pseudo_reach = pseudo::totals(&counted(|line, _| line.reach))
            .into_iter()
            .map(|total| (total.pseudo, total.value))
            .collect();
        let dps_reach = dps_reach(item, &lines);
        Self {
            lines,
            pseudo,
            pseudo_reach,
            dps_reach,
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

    /// The filter `sum` counts the line `key`: a DPS of a weapon, its damage and its attack
    /// speed; a pseudo total, the lines it adds up, also on an item that doesn't show it.
    #[must_use]
    pub fn counts_in(&self, key: StatKey, sum: FilterKey) -> bool {
        let has = |ids: &[&str]| self.line(key).is_some_and(|line| line.is_one_of(ids));
        match sum {
            FilterKey::Property(Property::PhysicalDps) => has(&PHYSICAL_DPS),
            FilterKey::Property(Property::ElementalDps) => has(&ELEMENTAL_DPS),
            FilterKey::Pseudo(pseudo) => self
                .pseudo
                .iter()
                .any(|total| total.pseudo == pseudo && total.lines.contains(&key)),
            FilterKey::Property(
                Property::ItemLevel
                | Property::Sockets
                | Property::Quality
                | Property::RequiredLevel
                | Property::Waystone(_)
                | Property::GemLevel
                | Property::GemSockets,
            )
            | FilterKey::Stat(_)
            | FilterKey::BaseType => false,
        }
    }

    /// The pseudo totals of the item that are filters, for any item: the stats show them for
    /// a rare or magic one only.
    #[must_use]
    pub fn pseudo_totals(&self) -> &[PseudoTotal] {
        &self.pseudo
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

    /// What the filter `key` with the `minimum` rules out of the offers: items of the base of
    /// the item when it compares a stat of the item, the rule of the estimate of a rare or
    /// magic item, which needs one, and of the filters checked by default. A line of a
    /// modifier, not of the base (an implicit, a rune, an enchantment); a pseudo total that
    /// adds up such a line, with a minimum beyond what the lines of the base reach; a DPS
    /// with a minimum beyond what the base reaches with them, the physical one at the usual
    /// quality; a property of a waystone or of a gem. The others rule out items of the league
    /// only (an implicit, the item level…), and the count of the unrevealed modifiers no item
    /// of the league it alone would: any base may have one. The base is that of the item,
    /// also when its base type is unchecked and the search spans its category, whose other
    /// bases may reach more: the damage of the bases isn't in the game data.
    #[must_use]
    pub fn rules_out(&self, key: FilterKey, minimum: Option<f64>) -> RulesOut {
        let of_a_modifier = |key: &StatKey| {
            self.line(*key)
                .is_some_and(|line| line.source == Source::Modifier)
        };
        let of_the_base_if = |compares: bool| {
            if compares {
                RulesOut::OfTheBase
            } else {
                RulesOut::OfTheLeague
            }
        };
        // Beyond what any item of the base reaches; with an empty field, only what it never
        // has.
        let beyond = |reach: f64| minimum.map_or(reach <= 0.0, |minimum| minimum > reach);
        match key {
            FilterKey::Stat(key) => of_the_base_if(of_a_modifier(&key)),
            FilterKey::BaseType => RulesOut::OfTheLeague,
            FilterKey::Pseudo(pseudo) if !pseudo.is_total() => RulesOut::Nothing,
            FilterKey::Pseudo(pseudo) => {
                let reach = self
                    .pseudo_reach
                    .iter()
                    .find(|(total, _)| *total == pseudo)
                    .map_or(0.0, |&(_, reach)| reach);
                let adds_a_modifier = self
                    .pseudo
                    .iter()
                    .find(|total| total.pseudo == pseudo)
                    .is_some_and(|total| total.lines.iter().any(of_a_modifier));
                of_the_base_if(adds_a_modifier && beyond(reach))
            }
            FilterKey::Property(property) => {
                // A DPS beyond what the base reaches; without a minimum, any weapon passes.
                let beyond_the_base = |reach: fn(BaseDps) -> f64| {
                    self.dps_reach
                        .map(reach)
                        .zip(minimum)
                        .is_some_and(|(reach, minimum)| minimum > reach)
                };
                match property {
                    Property::PhysicalDps => of_the_base_if(beyond_the_base(|base| base.physical)),
                    Property::ElementalDps => {
                        of_the_base_if(beyond_the_base(|base| base.elemental))
                    }
                    Property::Waystone(_) | Property::GemLevel | Property::GemSockets => {
                        RulesOut::OfTheBase
                    }
                    Property::ItemLevel
                    | Property::Sockets
                    | Property::Quality
                    | Property::RequiredLevel => RulesOut::OfTheLeague,
                }
            }
        }
    }

    /// The filter `key` with the `minimum` compares a stat of the item: it rules out items of
    /// its base ([`Self::rules_out`]).
    #[must_use]
    pub fn compares(&self, key: FilterKey, minimum: f64) -> bool {
        self.rules_out(key, Some(minimum)) == RulesOut::OfTheBase
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
                    rules_out: self.rules_out(*key, minimum),
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
                            rules_out: self.rules_out(*key, minimum),
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
                LineStat::Searched(stat) => {
                    Some(stat.filter(filter, self.rules_out(FilterKey::Stat(key), None)))
                }
                LineStat::Continued | LineStat::Unsearchable => None,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests;
