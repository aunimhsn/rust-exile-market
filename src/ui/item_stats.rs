//! The stats of the item, unfolded under the item block: for a rare, magic or normal item,
//! first the filters of its base type, item level and DPS, then its modifiers in the color of
//! the game, grouped, the stats that vary first. A unique, or an implicit modifier of a rare,
//! shows a bar of where its value falls in its range; a prefix or a suffix, its tier and the
//! range of that tier (its bar over every tier comes with the modifier tables, step 13).
//!
//! A stat that is a filter of the search starts with its checkbox and keeps the column of its
//! range field on the right, also when it has no limit; a stat of a unique that never varies
//! isn't a filter and starts at the left edge; a line the trade site can't search shows the
//! "not allowed" sign in place of its checkbox. A line better lower is searched with a
//! maximum, any other with a minimum. The damage and attack speed modifiers of a weapon start
//! unchecked: its DPS counts them. A corrupted item ends with a note: the search compares
//! it with corrupted offers only.
//!
//! The rows are decided apart from the drawing ([`rows`]), and tested.

use std::borrow::Cow;

use egui::text::{LayoutJob, TextFormat};
use egui::{
    Align, Align2, Area, Color32, CornerRadius, Frame, Id, Label, Layout, Margin, Order, Pos2,
    Rect, RichText, Sense, Stroke, TextEdit, Ui, Vec2, pos2, vec2,
};

use super::number_field::NumberField;
use super::theme::{self, color, size, text};
use super::widgets;
use crate::filters::{self, Filter, FilterKey, Filters, Half, Property, StatKey};
use crate::i18n::{Message, Translator};
use crate::item::{Affix, Item, Modifier, ModifierKind, Range, Rarity, StatLine};
use crate::prices::UniqueModifiers;
use crate::roll_difference::CopiedLines;
use crate::trade::stat_filters::{LineStat, LineStats};
use crate::{format, rolls};

/// The space between two rows.
const ROW_GAP: f32 = 6.0;
/// The space added above a small title, after the first.
const GROUP_GAP: f32 = 6.0;
/// The column on the left of a filter, for its checkbox.
const GUTTER: f32 = 20.0;
/// The space between the columns of a stat, and between a bar and its range.
const GAP: f32 = 8.0;
/// The column on the right of a filter, for its range field: the text never runs under it.
const RANGE_COLUMN: f32 = 62.0;
/// The column of a range field of four characters a side.
const WIDE_RANGE_COLUMN: f32 = 76.0;
/// A half of a range field, for three characters, and for four.
const HALF_WIDTH: f32 = 24.0;
const WIDE_HALF_WIDTH: f32 = 31.0;
/// The characters a number of a stat reaches for its range field to hold four a side.
const WIDE_CHARACTERS: usize = 4;
/// The space around the halves of a range field.
const RANGE_PADDING: Margin = Margin::symmetric(2, 1);
/// The space between a range field and its tooltip, under it.
const TOOLTIP_GAP: f32 = 8.0;
/// The space around the text of the tooltip of a range field, and between its lines.
const TOOLTIP_PADDING: Margin = Margin::symmetric(8, 5);
const TOOLTIP_LINE_GAP: f32 = 3.0;
/// How far the tooltip of the "not allowed" sign starts left of the sign.
const UNSEARCHABLE_TOOLTIP_SHIFT: f32 = 4.0;
/// The lines of the text of a stat, at most; the whole text is in its tooltip.
const MAX_LINES: usize = 2;
/// The bar of a roll.
const BAR_SIZE: Vec2 = vec2(96.0, 4.0);
/// The space above the roll of a stat.
const ROLL_GAP: f32 = 3.0;
/// The separator of the ranges of a stat with several rolls.
const RANGE_SEPARATOR: &str = " · ";
/// The share of the copied value the limit of a rare, magic or normal item leaves below it
/// for a minimum, above it for the maximum of a line better lower, to find comparable items
/// and not only identical ones.
const COMPARABLE_MARGIN: f64 = 0.1;

/// A small title of the stats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Group {
    /// The base type and the item level of a rare, magic or normal item.
    Item,
    /// The DPS of a weapon.
    Weapon,
    /// The enchantments, and the modifiers added by the corruption.
    Enchant,
    Runes,
    Implicit,
    Prefixes {
        filled: u32,
        max: u32,
    },
    Suffixes {
        filled: u32,
        max: u32,
    },
    /// The explicit modifiers of a magic or rare item copied without their affixes.
    Explicit,
    /// The modifiers of a unique, no prefix nor suffix.
    Unique,
}

/// The roll of a stat.
#[derive(Debug, Clone, PartialEq)]
enum Roll {
    /// Where its value falls in its ranges, from 0 to 1, and the ranges.
    Bar { share: f32, ranges: Vec<Range> },
    /// A prefix or a suffix: its tier, and the ranges of that tier; under the last line of a
    /// hybrid modifier, the ranges of all its lines, so that it reads as one modifier.
    Tier { tier: u32, ranges: Vec<Range> },
}

/// How the bar of a roll is drawn: a setting from step 18, which adds the zone searched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RollBar {
    /// Filled up to where the roll falls in its range, also on a line better lower, so that
    /// it reads with the range written next to it.
    Position,
}

/// A filter of the search, as it starts.
#[derive(Debug, Clone, Copy, PartialEq)]
struct StatFilter {
    checked: bool,
    /// None for a line that never varies, e.g. a fixed implicit of a rare.
    limit: Option<Limit>,
}

/// The limit of a filter at first: its value, the half of the range field it fills, and the
/// range it stays in, when known.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Limit {
    value: f64,
    /// The maximum for a line better lower, the minimum otherwise.
    half: Half,
    bounds: Option<Range>,
    /// The rolls the tooltip of the field tells: the range of today of a line with one
    /// number; the field of a line with several holds their average, not a roll.
    rolls: Option<Range>,
}

impl Limit {
    /// A minimum of `value`, with no range to stay in.
    fn minimum(value: f64) -> Self {
        Self {
            value,
            half: Half::Minimum,
            bounds: None,
            rolls: None,
        }
    }

    /// A limit of `value` filling `half`, for `line` copied with the value `copied`, which
    /// rolls in `range`: its bounds are `range`, widened to hold `copied` for a legacy unique,
    /// rolled in a range that changed since.
    fn new(value: f64, half: Half, range: Option<Range>, copied: f64, line: &StatLine) -> Self {
        let bounds = range.map(|range| range.holding(copied));
        Self {
            value: bounds.map_or(value, |bounds| bounds.clamp(value)),
            half,
            bounds,
            rolls: range.filter(|_| line.rolls.len() == 1),
        }
    }

    /// The minus sign is allowed in the field.
    fn negative(self) -> bool {
        self.value < 0.0 || self.bounds.is_some_and(|bounds| bounds.min < 0.0)
    }

    /// The halves of the field hold four characters: a number of its stat reaches them.
    fn wide(self) -> bool {
        let reaches = |value: f64| filters::limit_text(value).chars().count() >= WIDE_CHARACTERS;
        reaches(self.value)
            || self
                .bounds
                .is_some_and(|bounds| reaches(bounds.min) || reaches(bounds.max))
    }
}

impl StatFilter {
    /// The filter as the player finds it.
    fn default_choice(self) -> Filter {
        let mut filter = Filter {
            checked: self.checked,
            minimum: String::new(),
            maximum: String::new(),
            bounds: self.limit.and_then(|limit| limit.bounds),
        };
        if let Some(limit) = self.limit {
            *filter.text_mut(limit.half) = filters::limit_text(limit.value);
        }
        filter
    }
}

/// The text of a stat.
#[derive(Debug, Clone, Copy, PartialEq)]
enum StatText<'a> {
    /// A line of a modifier.
    Line(&'a str),
    BaseType(&'a str),
    /// A property of the item, with its value.
    Property(Property, f64),
}

impl<'a> StatText<'a> {
    fn text(self, i18n: &Translator) -> Cow<'a, str> {
        match self {
            Self::Line(text) => Cow::Borrowed(text),
            Self::BaseType(base_type) => Cow::Owned(i18n.filter_base_type(base_type)),
            Self::Property(property, value) => {
                Cow::Owned(i18n.filter_property(property, &format::number(value)))
            }
        }
    }
}

/// A stat: a line of a modifier, or a property of the item.
#[derive(Debug, Clone, PartialEq)]
struct Stat<'a> {
    key: FilterKey,
    text: StatText<'a>,
    /// A filter of the search, with its checkbox and its range field.
    filter: Option<StatFilter>,
    /// A line the trade site can't search, which would be a filter: the "not allowed" sign in
    /// place of its checkbox.
    unsearchable: bool,
    roll: Option<Roll>,
}

/// A row of the stats.
#[derive(Debug, Clone, PartialEq)]
enum Row<'a> {
    Group(Group),
    Stat(Stat<'a>),
    /// Under the stats of a corrupted item.
    CorruptedOnly,
}

/// Where the ranges of the rolls of a unique come from.
#[derive(Debug, Clone, Copy)]
enum RangeSource<'a> {
    /// The advanced copy: a line without a range never varies.
    Copied,
    /// The modifiers of the unique in the price database, of the kind of the line.
    Database(&'a [String]),
    /// Unknown, e.g. an item copied in French with Ctrl+C: a line with a number may vary.
    Unknown,
}

/// The number of stats of `item`, for the pill that unfolds them.
#[must_use]
pub fn count(item: &Item) -> u32 {
    len(item.modifiers())
}

/// The stats and filters of an item, decided once for a frame.
pub struct Stats<'a> {
    rows: Vec<Row<'a>>,
}

impl<'a> Stats<'a> {
    /// The stats of `item`, whose modifier `lines` tell which are better lower, and their
    /// `stats` which the trade site can search; `unique_modifiers` are the modifiers of its
    /// unique in the price database, the ranges of a unique copied without them; `base_type`
    /// is the base type its search looks for, a filter.
    #[must_use]
    pub fn new(
        item: &'a Item,
        lines: &CopiedLines,
        stats: &LineStats,
        unique_modifiers: Option<&UniqueModifiers>,
        base_type: Option<&'a str>,
    ) -> Self {
        Self {
            rows: rows(item, lines, stats, unique_modifiers, base_type),
        }
    }

    /// Nothing to unfold: no stat, no filter.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// The number of filters checked for the search, beside the pill of the stats.
    #[must_use]
    pub fn active_filters(&self, filters: &Filters) -> u32 {
        len(&self.checked(filters))
    }

    /// The filters of the search, checked or not, as the player set them or else by default.
    #[must_use]
    pub fn filters(&self, filters: &Filters) -> Vec<(FilterKey, Filter)> {
        self.rows
            .iter()
            .filter_map(|row| match row {
                Row::Stat(Stat {
                    key,
                    filter: Some(filter),
                    ..
                }) => Some((*key, filters.get(*key, || filter.default_choice()))),
                _ => None,
            })
            .collect()
    }

    /// The filters checked for the search, as the player set them or else by default.
    #[must_use]
    pub fn checked(&self, filters: &Filters) -> Vec<(FilterKey, Filter)> {
        let mut checked = self.filters(filters);
        checked.retain(|(_, filter)| filter.checked);
        checked
    }
}

fn len<T>(list: &[T]) -> u32 {
    u32::try_from(list.len()).unwrap_or(u32::MAX)
}

/// The rows of the stats of `item`, whose modifier `lines` tell which are better lower, and
/// their `stats` which the trade site can search; `unique_modifiers` are the modifiers of its
/// unique in the price database, the ranges of a unique copied without them; `base_type` is
/// the base type its search looks for, a filter.
fn rows<'a>(
    item: &'a Item,
    lines: &CopiedLines,
    stats: &LineStats,
    unique_modifiers: Option<&UniqueModifiers>,
    base_type: Option<&'a str>,
) -> Vec<Row<'a>> {
    let mut rows = Vec::new();
    if matches!(item.rarity(), Rarity::Normal | Rarity::Magic | Rarity::Rare) {
        push_group(&mut rows, Group::Item, item_filters(item, base_type));
        push_group(&mut rows, Group::Weapon, dps_filters(item));
    }
    let reading = Reading {
        lines,
        stats,
        dps_filters: rows.contains(&Row::Group(Group::Weapon)),
    };
    if item.rarity() == Rarity::Unique {
        rows.extend(unique_rows(item, unique_modifiers, &reading));
    } else {
        rows.extend(affix_rows(item, &reading));
    }
    for row in &mut rows {
        if let Row::Stat(stat) = row
            && let FilterKey::Stat(key) = stat.key
        {
            searchable(stat, stats.get(key));
        }
    }
    // The corrupted state is always a filter; a currency is never corrupted.
    if item.is_corrupted() && item.rarity() != Rarity::Currency {
        rows.push(Row::CorruptedOnly);
    }
    rows
}

/// Takes the filter away from `stat` when the trade site can't search it as its `line`:
/// a line it can't search shows the "not allowed" sign instead, and another line of a stat
/// written on several lines is searched by its first line with a number of the stat.
fn searchable(stat: &mut Stat<'_>, line: Option<&LineStat>) {
    match line {
        Some(LineStat::Searched(_)) => {}
        Some(LineStat::Continued) => stat.filter = None,
        Some(LineStat::Unsearchable) | None => stat.unsearchable = stat.filter.take().is_some(),
    }
}

/// The filters of the base type of a rare, magic or normal item, checked, and of its item
/// level, unchecked, with the level of the item as its minimum.
fn item_filters<'a>(item: &Item, base_type: Option<&'a str>) -> Vec<Stat<'a>> {
    let base_type = base_type.map(|base_type| Stat {
        key: FilterKey::BaseType,
        text: StatText::BaseType(base_type),
        filter: Some(StatFilter {
            checked: true,
            limit: None,
        }),
        unsearchable: false,
        roll: None,
    });
    let item_level = item.item_level().map(|level| {
        property_filter(
            Property::ItemLevel,
            f64::from(level),
            f64::from(level),
            false,
        )
    });
    base_type.into_iter().chain(item_level).collect()
}

/// The filters of the DPS of a weapon, each kind it deals, with a minimum at 90 % of its
/// DPS: the physical one checked, the elemental one unchecked.
fn dps_filters(item: &Item) -> Vec<Stat<'static>> {
    let Some(weapon) = item.weapon() else {
        return Vec::new();
    };
    let dps = weapon.dps();
    [
        (Property::PhysicalDps, dps.physical, true),
        (Property::ElementalDps, dps.elemental, false),
    ]
    .into_iter()
    .filter_map(|(property, dps, checked)| {
        // As the item block shows it.
        let dps = dps.round();
        (dps >= 1.0).then(|| {
            let minimum = (dps * (1.0 - COMPARABLE_MARGIN)).round();
            property_filter(property, dps, minimum, checked)
        })
    })
    .collect()
}

/// The filter of `property`, of the value `value`, with a minimum of `minimum`.
fn property_filter(property: Property, value: f64, minimum: f64, checked: bool) -> Stat<'static> {
    Stat {
        key: FilterKey::Property(property),
        text: StatText::Property(property, value),
        filter: Some(StatFilter {
            checked,
            limit: Some(Limit::minimum(minimum)),
        }),
        unsearchable: false,
        roll: None,
    }
}

/// How the stats read the modifier lines of the item, beyond their text.
struct Reading<'r> {
    /// They tell the lines better lower.
    lines: &'r CopiedLines,
    /// They tell the numbers of each line its stat counts, and the damage of a weapon.
    stats: &'r LineStats,
    /// The item has the filters of the DPS of a weapon.
    dps_filters: bool,
}

impl Reading<'_> {
    /// The line `key` starts checked: unchecked when the DPS of the weapon counts it, its
    /// damage or its attack speed.
    fn checked(&self, key: StatKey) -> bool {
        !(self.dps_filters && self.stats.weapon_damage(key))
    }

    /// The half of the range field the stat `key` fills: the maximum for a line better lower.
    fn half(&self, key: StatKey) -> Half {
        if self.lines.better_lower(key) {
            Half::Maximum
        } else {
            Half::Minimum
        }
    }

    /// `line`, the line `key`, with the numbers its stat counts only: not those its string
    /// writes (the 3 of `20% chance to gain Onslaught for 3 seconds`), which never vary.
    fn counted(&self, key: StatKey, line: &StatLine) -> StatLine {
        StatLine {
            text: line.text.clone(),
            rolls: self.pick(key, &line.rolls),
        }
    }

    /// Among `items`, one for each number of the line `key`, those of the numbers its stat
    /// counts; all of them when its stat isn't known.
    fn pick<T: Clone>(&self, key: StatKey, items: &[T]) -> Vec<T> {
        let picked = self.stats.numbers(key).and_then(|numbers| {
            numbers
                .iter()
                .map(|&number| items.get(number).cloned())
                .collect::<Option<Vec<T>>>()
        });
        picked.unwrap_or_else(|| items.to_vec())
    }
}

/// The rows of a unique: its enchantments, its runes, the implicit modifiers of its base,
/// then its own modifiers.
fn unique_rows<'a>(
    item: &'a Item,
    unique_modifiers: Option<&UniqueModifiers>,
    reading: &Reading<'_>,
) -> Vec<Row<'a>> {
    let copied = item
        .modifiers()
        .iter()
        .flat_map(|modifier| &modifier.lines)
        .any(|line| line.rolls.iter().any(|roll| roll.range.is_some()));
    let (implicit, explicit) = if copied {
        (RangeSource::Copied, RangeSource::Copied)
    } else if let Some(modifiers) = unique_modifiers {
        (
            RangeSource::Database(&modifiers.implicit),
            RangeSource::Database(&modifiers.explicit),
        )
    } else {
        (RangeSource::Unknown, RangeSource::Unknown)
    };
    // A stat of a unique varies when it is a filter.
    let varies = |_: &StatLine, stat: &Stat<'_>| stat.filter.is_some();
    let mut rows = Vec::new();
    let added_stat = |key, line| added(key, line, reading);
    push_group(
        &mut rows,
        Group::Enchant,
        stats(item, is_enchant, added_stat, varies),
    );
    push_group(
        &mut rows,
        Group::Runes,
        stats(item, is_rune, added_stat, varies),
    );
    push_group(
        &mut rows,
        Group::Implicit,
        stats(
            item,
            is_implicit,
            |key, line| unique_stat(key, line, implicit, reading),
            varies,
        ),
    );
    push_group(
        &mut rows,
        Group::Unique,
        stats(
            item,
            |kind| matches!(kind, ModifierKind::Explicit(_)),
            |key, line| unique_stat(key, line, explicit, reading),
            varies,
        ),
    );
    rows
}

/// The rows of a rare, magic or normal item: its enchantments, its runes, its implicit
/// modifiers, then its prefixes and suffixes.
fn affix_rows<'a>(item: &'a Item, reading: &Reading<'_>) -> Vec<Row<'a>> {
    let max = match item.rarity() {
        Rarity::Magic => 1,
        _ if item.is_jewel() => 2,
        _ => 3,
    };
    let line_varies = |line: &StatLine, _: &Stat<'_>| varies(line);
    let prefixes = affix_stats(item, Some(Affix::Prefix), reading);
    let suffixes = affix_stats(item, Some(Affix::Suffix), reading);
    let added_stat = |key, line| added(key, line, reading);
    let mut rows = Vec::new();
    push_group(
        &mut rows,
        Group::Enchant,
        stats(item, is_enchant, added_stat, line_varies),
    );
    push_group(
        &mut rows,
        Group::Runes,
        stats(item, is_rune, added_stat, line_varies),
    );
    push_group(
        &mut rows,
        Group::Implicit,
        stats(
            item,
            is_implicit,
            |key, line| implicit_stat(key, line, reading),
            line_varies,
        ),
    );
    push_group(
        &mut rows,
        Group::Prefixes {
            filled: prefixes.count,
            max,
        },
        prefixes.stats,
    );
    push_group(
        &mut rows,
        Group::Suffixes {
            filled: suffixes.count,
            max,
        },
        suffixes.stats,
    );
    push_group(
        &mut rows,
        Group::Explicit,
        affix_stats(item, None, reading).stats,
    );
    rows
}

/// A line of an enchantment or a rune: a filter, since the trade site searches them, without
/// a roll; unchecked, a rune being easy to change.
fn added<'a>(key: StatKey, line: &'a StatLine, reading: &Reading<'_>) -> Stat<'a> {
    let half = reading.half(key);
    let counted = reading.counted(key, line);
    Stat {
        key: FilterKey::Stat(key),
        text: StatText::Line(&line.text),
        filter: Some(StatFilter {
            checked: false,
            limit: value(&counted).map(|value| Limit::new(value, half, None, value, &counted)),
        }),
        unsearchable: false,
        roll: None,
    }
}

/// A line of an implicit modifier of a rare, magic or normal item: a filter, rolled in the
/// range its base gives it.
fn implicit_stat<'a>(key: StatKey, line: &'a StatLine, reading: &Reading<'_>) -> Stat<'a> {
    let counted = reading.counted(key, line);
    let ranges = copied_ranges(&counted);
    Stat {
        key: FilterKey::Stat(key),
        text: StatText::Line(&line.text),
        filter: Some(comparable_filter(
            &counted,
            bounds(&counted, &ranges),
            reading.half(key),
            reading.checked(key),
        )),
        unsearchable: false,
        roll: bar(magnitudes(&counted, &ranges)),
    }
}

/// A filter of a rare, magic or normal item, when the line varies with a minimum at 90 % of
/// the copied value, or a maximum at 110 % when `half` is the maximum.
fn comparable_filter(
    line: &StatLine,
    bounds: Option<Range>,
    half: Half,
    checked: bool,
) -> StatFilter {
    StatFilter {
        checked,
        limit: varies(line)
            .then(|| value(line))
            .flatten()
            .map(|value| Limit::new(comparable(value, line, half), half, bounds, value, line)),
    }
}

/// The limit 10 % of `value`, a value of `line`, below it for a minimum, above it for a
/// maximum, also for a negative value (`-15` gets a minimum of `-17`, not `-14`, which the
/// item itself wouldn't pass): rounded when the line has whole numbers only.
fn comparable(value: f64, line: &StatLine, half: Half) -> f64 {
    let margin = value.abs() * COMPARABLE_MARGIN;
    let limit = match half {
        Half::Minimum => value - margin,
        Half::Maximum => value + margin,
    };
    if line.rolls.iter().all(|roll| roll.value.fract() == 0.0) {
        limit.round()
    } else {
        limit
    }
}

/// The number of modifiers of an affix, and their stats.
struct AffixStats<'a> {
    count: u32,
    stats: Vec<Stat<'a>>,
}

/// The explicit modifiers of `affix` (`None`: copied without their affixes), those with a
/// line that varies first, each with its tier and its ranges under its last line.
fn affix_stats<'a>(item: &'a Item, affix: Option<Affix>, reading: &Reading<'_>) -> AffixStats<'a> {
    let mut modifiers: Vec<(usize, &Modifier)> = item
        .modifiers()
        .iter()
        .enumerate()
        .filter(|(_, modifier)| modifier.kind == ModifierKind::Explicit(affix))
        .collect();
    modifiers.sort_by_key(|(_, modifier)| !modifier.lines.iter().any(varies));
    let stats = modifiers
        .iter()
        .flat_map(|&(index, modifier)| {
            let last = modifier.lines.len().saturating_sub(1);
            modifier
                .lines
                .iter()
                .enumerate()
                .map(move |(line_index, line)| {
                    let key = StatKey {
                        modifier: index,
                        line: line_index,
                    };
                    Stat {
                        key: FilterKey::Stat(key),
                        text: StatText::Line(&line.text),
                        // The range of every tier comes with the modifier tables (step 13).
                        filter: Some(comparable_filter(
                            &reading.counted(key, line),
                            None,
                            reading.half(key),
                            reading.checked(key),
                        )),
                        unsearchable: false,
                        roll: modifier
                            .tier
                            .filter(|_| line_index == last)
                            .map(|tier| Roll::Tier {
                                tier,
                                ranges: modifier
                                    .lines
                                    .iter()
                                    .flat_map(|line| magnitudes(line, &copied_ranges(line)))
                                    .map(|(_, range)| range)
                                    .collect(),
                            }),
                    }
                })
        })
        .collect();
    AffixStats {
        count: len(&modifiers),
        stats,
    }
}

/// Adds the title of `group` and its stats, when it has some.
fn push_group<'a>(rows: &mut Vec<Row<'a>>, group: Group, stats: Vec<Stat<'a>>) {
    if !stats.is_empty() {
        rows.push(Row::Group(group));
        rows.extend(stats.into_iter().map(Row::Stat));
    }
}

/// The stats of the modifiers of `item` whose kind passes `keep`, made by `stat`: the
/// modifiers with a stat that `varies` first, each with its lines together.
fn stats<'a>(
    item: &'a Item,
    keep: impl Fn(ModifierKind) -> bool,
    stat: impl Fn(StatKey, &'a StatLine) -> Stat<'a>,
    varies: impl Fn(&StatLine, &Stat<'_>) -> bool,
) -> Vec<Stat<'a>> {
    let mut modifiers: Vec<(bool, Vec<Stat<'a>>)> = item
        .modifiers()
        .iter()
        .enumerate()
        .filter(|(_, modifier)| keep(modifier.kind))
        .map(|(index, modifier)| {
            let stats: Vec<Stat<'a>> = modifier
                .lines
                .iter()
                .enumerate()
                .map(|(line, text)| {
                    let key = StatKey {
                        modifier: index,
                        line,
                    };
                    stat(key, text)
                })
                .collect();
            let fixed = !modifier
                .lines
                .iter()
                .zip(&stats)
                .any(|(line, stat)| varies(line, stat));
            (fixed, stats)
        })
        .collect();
    modifiers.sort_by_key(|&(fixed, _)| fixed);
    modifiers.into_iter().flat_map(|(_, stats)| stats).collect()
}

/// A line with a number whose range isn't known, or is wider than one value: not `Loads
/// an additional bolt`, nor `5(5-5)% increased Movement Speed`.
fn varies(line: &StatLine) -> bool {
    line.rolls
        .iter()
        .any(|roll| roll.range.is_none_or(|range| range.max > range.min))
}

fn is_enchant(kind: ModifierKind) -> bool {
    matches!(kind, ModifierKind::Enchant | ModifierKind::Corruption)
}

fn is_rune(kind: ModifierKind) -> bool {
    kind == ModifierKind::Rune
}

fn is_implicit(kind: ModifierKind) -> bool {
    kind == ModifierKind::Implicit
}

/// A line of a unique: a filter checked with its value filling its half and its bar when it
/// varies, at the left edge otherwise.
fn unique_stat<'a>(
    key: StatKey,
    whole: &'a StatLine,
    source: RangeSource<'_>,
    reading: &Reading<'_>,
) -> Stat<'a> {
    let line = &reading.counted(key, whole);
    let ranges = match source {
        RangeSource::Copied => Some(copied_ranges(line)),
        RangeSource::Database(modifiers) => rolls::unique_ranges(&whole.text, modifiers)
            // One range or none for each number of the line.
            .filter(|ranges| ranges.len() == whole.rolls.len())
            .map(|ranges| reading.pick(key, &ranges)),
        RangeSource::Unknown => None,
    };
    let fixed = Stat {
        key: FilterKey::Stat(key),
        text: StatText::Line(&whole.text),
        filter: None,
        unsearchable: false,
        roll: None,
    };
    let (roll, bounds) = match &ranges {
        // Its ranges are known: a filter when one of them is wider than one value.
        Some(ranges) => match bar(magnitudes(line, ranges)) {
            Some(roll) => (Some(roll), bounds(line, ranges)),
            None => return fixed,
        },
        // Unknown: a line with a number may vary.
        None => (None, None),
    };
    let Some(value) = value(line) else {
        return fixed;
    };
    Stat {
        filter: Some(StatFilter {
            checked: true,
            // A legacy unique keeps its value, out of the range of today.
            limit: Some(Limit::new(value, reading.half(key), bounds, value, line)),
        }),
        roll,
        ..fixed
    }
}

/// The range of each number of a line, from the advanced copy.
fn copied_ranges(line: &StatLine) -> Vec<Option<Range>> {
    line.rolls.iter().map(|roll| roll.range).collect()
}

/// The values of a line with a range, without their sign, given the range of each value.
fn magnitudes(line: &StatLine, ranges: &[Option<Range>]) -> Vec<(f64, Range)> {
    line.rolls
        .iter()
        .zip(ranges)
        .filter_map(|(roll, range)| Some((roll.value, range.as_ref()?.magnitude()?)))
        .collect()
}

/// The value a line is searched by: the average of its numbers, as the trade site does for
/// `Adds # to # Physical Damage`; none for a line without number.
fn value(line: &StatLine) -> Option<f64> {
    average(line.rolls.iter().map(|roll| roll.value))
}

/// The range of the value of a line ([`value`]), given the range of each of its numbers: a
/// number without range is fixed. None when no number has a range.
fn bounds(line: &StatLine, ranges: &[Option<Range>]) -> Option<Range> {
    if ranges.iter().all(Option::is_none) {
        return None;
    }
    let signed: Vec<Range> = line
        .rolls
        .iter()
        .zip(ranges)
        .map(|(roll, range)| {
            range.map_or(
                Range {
                    min: roll.value,
                    max: roll.value,
                },
                |range| signed(range, roll.value),
            )
        })
        .collect();
    Some(Range {
        min: average(signed.iter().map(|range| range.min))?,
        max: average(signed.iter().map(|range| range.max))?,
    })
}

/// `range`, from its lowest to its highest value, with the sign of `value`: the price
/// database may write the range of a negative number without its sign.
fn signed(range: Range, value: f64) -> Range {
    let (low, high) = (range.min.min(range.max), range.min.max(range.max));
    if value < 0.0 && low >= 0.0 {
        Range {
            min: -high,
            max: -low,
        }
    } else {
        Range {
            min: low,
            max: high,
        }
    }
}

fn average(values: impl Iterator<Item = f64>) -> Option<f64> {
    let (sum, count) = values.fold((0.0, 0u32), |(sum, count), value| (sum + value, count + 1));
    (count > 0).then(|| sum / f64::from(count))
}

/// The bar of `rolls`, those whose range is wider than one value: the average of where each
/// value falls in its range; none when no roll varies.
fn bar(rolls: Vec<(f64, Range)>) -> Option<Roll> {
    let rolls: Vec<(f64, Range)> = rolls
        .into_iter()
        .filter(|(_, range)| range.max > range.min)
        .collect();
    let average = average(
        rolls
            .iter()
            .map(|&(value, range)| share(value.abs(), range)),
    )?;
    #[expect(clippy::cast_possible_truncation, reason = "a share from 0 to 1")]
    let share = average as f32;
    Some(Roll::Bar {
        share,
        ranges: rolls.into_iter().map(|(_, range)| range).collect(),
    })
}

/// Where `value` falls in `range`, from 0 to 1; 1 for a range of one value.
fn share(value: f64, range: Range) -> f64 {
    if range.max <= range.min {
        return 1.0;
    }
    ((value - range.min) / (range.max - range.min)).clamp(0.0, 1.0)
}

/// The ranges of a stat, e.g. `40–60`, or `8–12 · 16–22` for several rolls.
fn ranges_text(ranges: &[Range]) -> String {
    ranges
        .iter()
        .map(|range| {
            format!(
                "{}–{}",
                format::number(range.min),
                format::number(range.max)
            )
        })
        .collect::<Vec<_>>()
        .join(RANGE_SEPARATOR)
}

impl Stats<'_> {
    /// Draws the stats, in the item block, with the filters the player set.
    pub fn show(self, ui: &mut Ui, filters: &mut Filters, i18n: &Translator) {
        show(ui, self.rows, filters, i18n, RollBar::Position);
    }
}

fn show(ui: &mut Ui, rows: Vec<Row<'_>>, filters: &mut Filters, i18n: &Translator, bar: RollBar) {
    // "min – max" on the right of the first title, above the range fields.
    let mut range_title = rows.iter().any(|row| {
        matches!(
            row,
            Row::Stat(Stat {
                filter: Some(StatFilter { limit: Some(_), .. }),
                ..
            })
        )
    });
    for (index, row) in rows.into_iter().enumerate() {
        if index > 0 {
            let group_gap = if matches!(row, Row::Group(_)) {
                GROUP_GAP
            } else {
                0.0
            };
            ui.add_space(ROW_GAP + group_gap);
        }
        match row {
            Row::Group(group) => {
                group_title(ui, group, std::mem::take(&mut range_title), i18n);
            }
            Row::Stat(stat) => stat_row(ui, &stat, filters, i18n, bar),
            Row::CorruptedOnly => {
                let galley =
                    widgets::text_line(ui, i18n.text(Message::CorruptedOffersOnly), text::note());
                let (rect, _) = ui.allocate_exact_size(galley.size(), Sense::hover());
                ui.painter().galley(rect.min, galley, color::FAINT);
            }
        }
    }
}

/// A small title, with the fill of the prefixes and suffixes: lighter while one is open; and
/// "min – max" on its right, above the range fields of the filters.
fn group_title(ui: &mut Ui, group: Group, range: bool, i18n: &Translator) {
    let (message, fill) = match group {
        Group::Item => (Message::GroupItem, None),
        Group::Weapon => (Message::GroupWeapon, None),
        Group::Enchant => (Message::GroupEnchant, None),
        Group::Runes => (Message::GroupRunes, None),
        Group::Implicit => (Message::GroupImplicit, None),
        Group::Prefixes { filled, max } => (Message::GroupPrefixes, Some((filled, max))),
        Group::Suffixes { filled, max } => (Message::GroupSuffixes, Some((filled, max))),
        Group::Explicit => (Message::GroupExplicit, None),
        Group::Unique => (Message::GroupUnique, None),
    };
    let small_title = |ui: &mut Ui, message: Message| {
        let title = widgets::spaced_line(
            ui,
            i18n.text(message).to_uppercase(),
            text::label(),
            text::LABEL_LETTER_SPACING,
        );
        let (rect, _) = ui.allocate_exact_size(title.size(), Sense::hover());
        ui.painter().galley(rect.min, title, color::FAINT);
    };
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        small_title(ui, message);
        if let Some((filled, max)) = fill {
            let tint = if filled < max {
                color::ACCENT
            } else {
                color::WEAK
            };
            ui.label(
                RichText::new(i18n.affix_fill(filled, max))
                    .font(text::label())
                    .color(tint),
            );
        }
        if range {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                small_title(ui, Message::FilterRange);
            });
        }
    });
}

/// A stat in the color of the modifiers, on two lines at most, and its roll under it; a
/// filter between its checkbox and the column of its range field.
fn stat_row(ui: &mut Ui, stat: &Stat<'_>, filters: &mut Filters, i18n: &Translator, bar: RollBar) {
    let text = stat.text.text(i18n);
    ui.horizontal_top(|ui| {
        let width = ui.available_width();
        let line_height = ui.fonts_mut(|fonts| fonts.row_height(&text::body()));
        if stat.unsearchable {
            let sign = widgets::not_allowed_sign(ui, vec2(GUTTER, line_height));
            if sign.hovered() {
                unsearchable_tooltip(ui, stat.key, sign.rect, i18n);
            }
            ui.add_space(GAP);
            stat_text(
                ui,
                &text,
                stat.roll.as_ref(),
                (width - GUTTER - GAP - GAP - RANGE_COLUMN).max(0.0),
                bar,
            );
            return;
        }
        let Some(filter) = stat.filter else {
            stat_text(ui, &text, stat.roll.as_ref(), width, bar);
            return;
        };
        let mut choice = filters.get(stat.key, || filter.default_choice());
        let before = choice.clone();
        // The checkbox on the first line of the text.
        if widgets::check_box(ui, choice.checked, vec2(GUTTER, line_height), &text).clicked() {
            choice.checked = !choice.checked;
        }
        ui.add_space(GAP);
        let column = if filter.limit.is_some_and(Limit::wide) {
            WIDE_RANGE_COLUMN
        } else {
            RANGE_COLUMN
        };
        stat_text(
            ui,
            &text,
            stat.roll.as_ref(),
            (width - GUTTER - GAP - GAP - column).max(0.0),
            bar,
        );
        ui.add_space(GAP);
        if let Some(limit) = filter.limit {
            range_field(ui, stat.key, &mut choice, limit, i18n);
        }
        if choice != before {
            filters.set(stat.key, choice);
        }
    });
}

/// The text of a stat and its roll, `width` across.
fn stat_text(ui: &mut Ui, text: &str, roll: Option<&Roll>, width: f32, bar: RollBar) {
    ui.allocate_ui_with_layout(vec2(width, 0.0), Layout::top_down(Align::Min), |ui| {
        ui.set_width(width);
        ui.add(Label::new(clamped(text)).wrap());
        if let Some(roll) = roll {
            ui.add_space(ROLL_GAP);
            roll_line(ui, roll, bar);
        }
    });
}

/// The range field of a filter, in its column: its minimum, a dash, its maximum, each half a
/// number only, brought back within the range of the stat when the player leaves it; its
/// tooltip under the pointer.
fn range_field(ui: &mut Ui, key: FilterKey, choice: &mut Filter, limit: Limit, i18n: &Translator) {
    let halves = [Half::Minimum, Half::Maximum];
    let ids = halves.map(|half| ui.make_persistent_id(("stat-range", key, half)));
    let focused = ui.memory(|memory| ids.iter().any(|&id| memory.has_focus(id)));
    let line = if focused {
        color::WEAK
    } else {
        color::CONTROL_LINE
    };
    let (column, half_width) = if limit.wide() {
        (WIDE_RANGE_COLUMN, WIDE_HALF_WIDTH)
    } else {
        (RANGE_COLUMN, HALF_WIDTH)
    };
    let stroke = Stroke::new(1.0, line);
    let dash_width =
        column - 2.0 * (f32::from(RANGE_PADDING.left) + stroke.width) - 2.0 * half_width;
    let field = Frame::NONE
        .fill(color::CONTROL)
        .stroke(stroke)
        .corner_radius(CornerRadius::same(size::RADIUS))
        .inner_margin(RANGE_PADDING)
        .show(ui, |ui| {
            // The halves and the dash one line of numbers high, the dash in the middle.
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.spacing_mut().interact_size.y = 0.0;
            let height = ui.fonts_mut(|fonts| fonts.row_height(&text::range_field()));
            for (index, (half, id)) in halves.into_iter().zip(ids).enumerate() {
                if index > 0 {
                    let galley = widgets::text_line(ui, "–", text::range_field());
                    let (rect, _) =
                        ui.allocate_exact_size(vec2(dash_width, height), Sense::hover());
                    let at = rect.center() - galley.size() / 2.0;
                    ui.painter().galley(at, galley, color::FAINT);
                }
                let mut text = NumberField {
                    text: choice.text_mut(half),
                    negative: limit.negative(),
                };
                let response = ui.add(
                    TextEdit::singleline(&mut text)
                        .id(id)
                        .font(text::range_field())
                        .text_color(color::TEXT)
                        // The caret of an empty half in its middle.
                        .align(Align2::CENTER_CENTER)
                        .frame(Frame::NONE)
                        .desired_width(half_width),
                );
                if response.lost_focus() {
                    choice.settle(half);
                }
            }
        });
    let rect = field.response.rect;
    if ui.rect_contains_pointer(rect) {
        field_tooltip(ui, key, rect, limit, i18n);
    }
}

/// The lines of the tooltip of the range field of `limit`, with their colors: the rolls it
/// accepts, then "Lower is better" in gray for a line better lower.
fn tooltip_lines(limit: Limit, i18n: &Translator) -> Vec<(String, Color32)> {
    let rolls = limit
        .rolls
        .map(|rolls| i18n.filter_rolls(&format::number(rolls.min), &format::number(rolls.max)));
    let lower = (limit.half == Half::Maximum).then(|| i18n.text(Message::FilterLowerBetter));
    [(rolls, color::TEXT), (lower, color::WEAK)]
        .into_iter()
        .filter_map(|(line, tint)| Some((line?, tint)))
        .collect()
}

/// The tooltip of a range field in `field`: the rolls it accepts, then "Lower is better" in
/// gray for a line better lower; none when it has nothing to say. Under the field, their
/// right edges together, so that it stays in the window.
fn field_tooltip(ui: &Ui, key: FilterKey, field: Rect, limit: Limit, i18n: &Translator) {
    let lines = tooltip_lines(limit, i18n);
    if lines.is_empty() {
        return;
    }
    let at = pos2(field.right(), field.bottom() + TOOLTIP_GAP);
    tooltip(
        ui,
        Id::NULL.with(("stat-range-tooltip", key)),
        at,
        Align2::RIGHT_TOP,
        lines,
    );
}

/// The tooltip of the "not allowed" sign of a line the trade site can't search, in `sign`:
/// two sentences, one per line. Under the sign, opening to the right since the sign is on the
/// left edge.
fn unsearchable_tooltip(ui: &Ui, key: FilterKey, sign: Rect, i18n: &Translator) {
    let lines = [Message::FilterUnsearchable, Message::FilterUnsearchableHint]
        .map(|message| (i18n.text(message), color::TEXT));
    let at = pos2(
        sign.left() - UNSEARCHABLE_TOOLTIP_SHIFT,
        sign.bottom() + TOOLTIP_GAP,
    );
    tooltip(
        ui,
        Id::NULL.with(("unsearchable-tooltip", key)),
        at,
        Align2::LEFT_TOP,
        lines.into(),
    );
}

/// A tooltip of `lines` in their colors, its corner `pivot` at `at`.
fn tooltip(ui: &Ui, id: Id, at: Pos2, pivot: Align2, lines: Vec<(String, Color32)>) {
    Area::new(id)
        .order(Order::Tooltip)
        .interactable(false)
        .pivot(pivot)
        .fixed_pos(at)
        .show(ui.ctx(), |ui| {
            Frame::NONE
                .fill(color::BUBBLE)
                .corner_radius(CornerRadius::same(size::BUBBLE_RADIUS))
                .shadow(theme::floating_shadow())
                .inner_margin(TOOLTIP_PADDING)
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = Vec2::ZERO;
                    for (index, (line, tint)) in lines.into_iter().enumerate() {
                        if index > 0 {
                            ui.add_space(TOOLTIP_LINE_GAP);
                        }
                        let galley = widgets::text_line(ui, line, text::note());
                        let (rect, _) = ui.allocate_exact_size(galley.size(), Sense::hover());
                        ui.painter().galley(rect.min, galley, tint);
                    }
                });
        });
}

/// The text of a stat, cut with "…" past its last line; the label shows the whole text in
/// its tooltip.
fn clamped(text: &str) -> LayoutJob {
    let mut job = LayoutJob::single_section(
        text.to_owned(),
        TextFormat::simple(text::body(), color::MODIFIER),
    );
    job.wrap.max_rows = MAX_LINES;
    job.wrap.overflow_character = Some('…');
    job
}

/// The bar of a roll drawn in the style `bar`, and its ranges; or the tier of a prefix or
/// suffix and the ranges of that tier.
fn roll_line(ui: &mut Ui, roll: &Roll, bar: RollBar) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = GAP;
        match roll {
            Roll::Bar { share, ranges } => {
                let (rect, _) = ui.allocate_exact_size(BAR_SIZE, Sense::hover());
                let radius = BAR_SIZE.y / 2.0;
                let painter = ui.painter();
                painter.rect_filled(rect, radius, color::CONTROL_LINE);
                match bar {
                    RollBar::Position => {
                        let fill = Rect::from_min_size(
                            rect.min,
                            vec2(rect.width() * share.clamp(0.0, 1.0), rect.height()),
                        );
                        painter.rect_filled(fill, radius, color::ACCENT);
                    }
                }
                range_label(ui, ranges);
            }
            Roll::Tier { tier, ranges } => {
                let tint = match tier {
                    1 => color::TIER_1,
                    2 | 3 => color::ACCENT,
                    _ => color::FAINT,
                };
                ui.label(
                    RichText::new(format!("T{tier}"))
                        .font(text::tier())
                        .color(tint),
                );
                if !ranges.is_empty() {
                    range_label(ui, ranges);
                }
            }
        }
    });
}

/// The ranges of a roll, in faint text.
fn range_label(ui: &mut Ui, ranges: &[Range]) {
    let galley = widgets::text_line(ui, ranges_text(ranges), text::roll());
    let (rect, _) = ui.allocate_exact_size(galley.size(), Sense::hover());
    ui.painter().galley(rect.min, galley, color::FAINT);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::CopiedItem;

    fn parse(text: &str) -> Item {
        Item::parse(&CopiedItem::from_text(text).expect("an item")).expect("read")
    }

    fn database(implicit: &[&str], explicit: &[&str]) -> UniqueModifiers {
        let owned = |lines: &[&str]| lines.iter().map(|&line| line.to_owned()).collect();
        UniqueModifiers {
            implicit: owned(implicit),
            explicit: owned(explicit),
        }
    }

    /// The rows of `item`, its lines read in English.
    fn rows<'a>(item: &'a Item, unique_modifiers: Option<&UniqueModifiers>) -> Vec<Row<'a>> {
        let lines = CopiedLines::new(
            item.language(),
            item.modifiers(),
            &LineStats::default(),
            None,
        );
        let stats = LineStats::new(item.language(), item.modifiers(), None);
        super::rows(item, &lines, &stats, unique_modifiers, base_type(item))
    }

    /// The base type the search of `item` looks for, among its filters: none for a unique.
    fn base_type(item: &Item) -> Option<&str> {
        (item.rarity() != Rarity::Unique).then(|| item.base_type())
    }

    /// The rows as texts: a title (`Prefixes 2/3`), the note of a corrupted item, or a stat,
    /// first `■ ` for a checked filter, `□ ` for an unchecked one or `⊘ ` for a line the trade
    /// site can't search, then its minimum or its
    /// maximum and its roll (`■ +55 to maximum Life ≥55 [40–60 0.75]`, `■ 18% reduced
    /// Movement Speed ≤18`, `■ +3 to Level [T1 3–3]`).
    fn texts(item: &Item, unique_modifiers: Option<&UniqueModifiers>) -> Vec<String> {
        rows(item, unique_modifiers)
            .into_iter()
            .map(|row| match row {
                Row::Group(Group::Prefixes { filled, max }) => format!("Prefixes {filled}/{max}"),
                Row::Group(Group::Suffixes { filled, max }) => format!("Suffixes {filled}/{max}"),
                Row::Group(group) => format!("{group:?}"),
                Row::CorruptedOnly => "Corrupted only".to_owned(),
                Row::Stat(Stat {
                    text,
                    filter,
                    unsearchable,
                    roll,
                    ..
                }) => {
                    let check = match filter {
                        _ if unsearchable => "⊘ ",
                        Some(StatFilter { checked: true, .. }) => "■ ",
                        Some(StatFilter { checked: false, .. }) => "□ ",
                        None => "",
                    };
                    let limit = filter
                        .and_then(|filter| filter.limit)
                        .map(|limit| {
                            let sign = match limit.half {
                                Half::Minimum => '≥',
                                Half::Maximum => '≤',
                            };
                            format!(" {sign}{}", format::number(limit.value))
                        })
                        .unwrap_or_default();
                    let roll = match roll {
                        None => String::new(),
                        Some(Roll::Bar { share, ranges }) => {
                            format!(" [{} {share:.2}]", ranges_text(&ranges))
                        }
                        Some(Roll::Tier { tier, ranges }) => {
                            format!(" [T{tier} {}]", ranges_text(&ranges))
                        }
                    };
                    let text = match text {
                        StatText::Line(text) => text.to_owned(),
                        StatText::BaseType(base_type) => format!("Base: {base_type}"),
                        StatText::Property(property, value) => {
                            format!("{property:?}: {}", format::number(value))
                        }
                    };
                    format!("{check}{text}{limit}{roll}")
                }
            })
            .collect()
    }

    #[test]
    fn a_unique_copied_with_ctrl_c_gets_its_ranges_from_the_price_database() {
        let headhunter = parse(
            "Item Class: Belts\nRarity: Unique\nHeadhunter\nHeavy Belt\n--------\n\
             Item Level: 80\n--------\n26% increased Stun Threshold (implicit)\n--------\n\
             When you kill a Rare monster, you gain its Modifiers for 60 seconds\n\
             +55 to maximum Life",
        );
        let modifiers = database(
            &["(20-30)% increased Stun Threshold"],
            &[
                "+(40-60) to maximum Life",
                "When you kill a Rare monster, you gain its Modifiers for 60 seconds",
            ],
        );

        assert_eq!(
            texts(&headhunter, Some(&modifiers)),
            [
                "Implicit",
                "■ 26% increased Stun Threshold ≥26 [20–30 0.60]",
                "Unique",
                // The stats that vary first.
                "■ +55 to maximum Life ≥55 [40–60 0.75]",
                "When you kill a Rare monster, you gain its Modifiers for 60 seconds",
            ]
        );
    }

    #[test]
    fn a_line_missing_from_the_price_database_may_vary_like_an_unknown_one() {
        let headhunter = parse(
            "Item Class: Belts\nRarity: Unique\nHeadhunter\nHeavy Belt\n--------\n\
             Item Level: 80\n--------\n26% increased Stun Threshold (implicit)\n--------\n\
             When you kill a Rare monster, you gain its Modifiers for 60 seconds\n\
             +55 to maximum Life",
        );
        let unknown = [
            "Implicit",
            "■ 26% increased Stun Threshold ≥26",
            "Unique",
            "■ +55 to maximum Life ≥55",
            // The string of its stat writes the 60: it never varies.
            "When you kill a Rare monster, you gain its Modifiers for 60 seconds",
        ];
        let partial = database(
            &["26% increased Stun Threshold"],
            &["When you kill a Rare monster, you gain its Modifiers for 60 seconds"],
        );

        // A unique without modifiers in the price database.
        assert_eq!(
            texts(&headhunter, Some(&UniqueModifiers::default())),
            unknown
        );
        assert_eq!(texts(&headhunter, None), unknown);
        // A line found without range never varies; a line not found may.
        assert_eq!(
            texts(&headhunter, Some(&partial)),
            [
                "Implicit",
                "26% increased Stun Threshold",
                "Unique",
                "■ +55 to maximum Life ≥55",
                "When you kill a Rare monster, you gain its Modifiers for 60 seconds",
            ]
        );
    }

    #[test]
    fn a_number_written_by_the_string_of_a_stat_has_no_range_from_the_price_database() {
        let unique = parse(
            "Item Class: Gloves\nRarity: Unique\nTest Grip\nSuede Bracers\n--------\n\
             Item Level: 80\n--------\n1% more Unarmed Damage per 5 Strength",
        );
        let modifiers = database(&[], &["1% more Unarmed Damage per (4-6) Strength"]);

        // The 1 is written by the string of the stat: the field holds the 5, in its range.
        assert_eq!(
            texts(&unique, Some(&modifiers)),
            [
                "Unique",
                "■ 1% more Unarmed Damage per 5 Strength ≥5 [4–6 0.50]"
            ]
        );
    }

    #[test]
    fn an_implicit_and_an_explicit_of_the_same_stat_keep_their_own_ranges() {
        let ring = parse(
            "Item Class: Rings\nRarity: Unique\nVenom Loop\nAmethyst Ring\n--------\n\
             Item Level: 80\n--------\n\
             +10% to Chaos Resistance (implicit)\n--------\n+20% to Chaos Resistance\n\
             Adds 1 to 55 Lightning damage to Attacks",
        );
        let modifiers = database(
            &["+(7-13)% to Chaos Resistance"],
            &[
                "+(17-23)% to Chaos Resistance",
                "Adds 1 to (50-60) Lightning damage to Attacks",
            ],
        );

        assert_eq!(
            texts(&ring, Some(&modifiers)),
            [
                "Implicit",
                "■ +10% to Chaos Resistance ≥10 [7–13 0.50]",
                "Unique",
                "■ +20% to Chaos Resistance ≥20 [17–23 0.50]",
                // The fixed number before the roll keeps its place; the minimum is the
                // average of both numbers, as on the trade site.
                "■ Adds 1 to 55 Lightning damage to Attacks ≥28 [50–60 0.50]",
            ]
        );
    }

    #[test]
    fn a_unique_copied_with_ctrl_alt_c_has_its_own_ranges() {
        let focus = parse(include_str!("../item/test-items/en-unique-focus.txt"));

        assert_eq!(
            texts(&focus, None),
            [
                "Unique",
                "■ 56% increased Energy Shield ≥56 [50–70 0.30]",
                "■ +26% to Lightning Resistance ≥26 [20–30 0.60]",
                "40% increased Mana Regeneration Rate while stationary",
                "+5% to Maximum Lightning Resistance",
                "40% increased Mana Regeneration Rate",
            ]
        );
    }

    #[test]
    fn a_negative_range_of_the_advanced_copy_is_without_its_sign() {
        let ring = parse(
            "Item Class: Rings\nRarity: Unique\nCold Loop\nRuby Ring\n--------\n\
             { Unique Modifier — Elemental, Fire, Resistance }\n-15(-20--10)% to Fire Resistance\n\
             { Unique Modifier — Elemental, Cold, Resistance }\n2(-5-10)% to Cold Resistance",
        );

        assert_eq!(
            texts(&ring, None),
            [
                "Unique",
                "■ -15% to Fire Resistance ≥-15 [10–20 0.50]",
                // A range across zero needs its sign: no bar.
                "2% to Cold Resistance",
            ]
        );
    }

    #[test]
    fn a_french_unique_copied_with_ctrl_c_has_its_numbers_as_filters_without_bar() {
        let crossbow = parse(
            "Classe d'objet: Arbalètes\nRareté: Unique\nMurmure des brumes\nArbalète de fortune\n\
             --------\nNiveau de l'objet: 78\n--------\nLes Attaques Ricochent sur les ennemis\n\
             36% d'Augmentation d'Accumulation de Gel",
        );

        assert_eq!(
            texts(&crossbow, None),
            [
                "Unique",
                "■ 36% d'Augmentation d'Accumulation de Gel ≥36",
                "Les Attaques Ricochent sur les ennemis",
            ]
        );
    }

    #[test]
    fn a_rare_shows_its_enchantment_runes_implicit_prefixes_and_suffixes() {
        let crossbow = parse(include_str!(
            "../item/test-items/en-rare-crossbow-all-modifiers.txt"
        ));

        assert_eq!(
            texts(&crossbow, None),
            [
                // The base type checked, the item level unchecked at the level of the item.
                "Item",
                "■ Base: Gemini Crossbow",
                "□ ItemLevel: 80 ≥80",
                // 90 % of the DPS, the elemental one unchecked.
                "Weapon",
                "■ PhysicalDps: 288 ≥259",
                "□ ElementalDps: 267 ≥240",
                "Enchant",
                // The enchantment and the runes unchecked, with their value.
                "□ 45% increased Elemental Damage with Attacks ≥45",
                "Runes",
                "□ 18% increased Physical Damage ≥18",
                "□ Gain 30 Mana per enemy killed ≥30",
                "Implicit",
                // A line without number: no minimum.
                "■ Loads an additional bolt",
                "Prefixes 3/3",
                // 90 % of the copied value, of the average of two numbers; the damage and
                // the attack speed of the weapon unchecked, since its DPS counts them.
                "□ Adds 10 to 273 Lightning Damage ≥127 [T2 1–16 · 239–300]",
                "□ Adds 24 to 51 Physical Damage ≥34 [T3 23–35 · 39–59]",
                // Not a damage of the weapon: its attacks get it from any item.
                "■ 109% increased Elemental Damage with Attacks ≥98 [T2 100–119]",
                "Suffixes 3/3",
                "■ Leeches 7.59% of Physical Damage as Mana ≥6.83 [T1 7–7.9]",
                "■ Gain 21 Mana per enemy killed ≥19 [T3 21–27]",
                "□ 18% increased Attack Speed ≥16 [T1 17–19]",
                "Corrupted only",
            ]
        );
    }

    #[test]
    fn a_rare_copied_with_ctrl_c_has_its_explicit_modifiers_together() {
        let ring = parse(include_str!("../item/test-items/en-plain-rare-ring.txt"));

        assert_eq!(
            texts(&ring, None),
            [
                "Item",
                "■ Base: Prismatic Ring",
                "□ ItemLevel: 79 ≥79",
                "Implicit",
                "■ +8% to all Elemental Resistances ≥7",
                "Explicit",
                "■ +143 to Evasion Rating ≥129",
                "■ +12 to Strength ≥11",
                "Corrupted only",
            ]
        );
    }

    #[test]
    fn a_fixed_implicit_of_a_rare_is_a_filter_without_bar_after_those_that_vary() {
        let boots = parse(
            "Item Class: Boots\nRarity: Rare\nGale Tread\nLattice Sandals\n--------\n\
             { Implicit Modifier }\n5(5-5)% increased Movement Speed\n\
             { Implicit Modifier }\n+12(10-15) to Dexterity\n--------\n\
             { Prefix Modifier \"Hale\" (Tier: 3) — Life }\n+86(85-99) to maximum Life\n\
             { Prefix Modifier \"Sturdy\" (Tier: 5) — Defences }\n\
             +20(15-25) to Evasion Rating\n+10(8-12)% increased Evasion Rating",
        );

        assert_eq!(
            texts(&boots, None),
            [
                "Item",
                "■ Base: Lattice Sandals",
                "Implicit",
                // 90 % of 12, within the range of the implicit.
                "■ +12 to Dexterity ≥11 [10–15 0.40]",
                // A fixed line keeps its checkbox, without a minimum.
                "■ 5% increased Movement Speed",
                "Prefixes 2/3",
                "■ +86 to maximum Life ≥77 [T3 85–99]",
                // A hybrid modifier: its tier and the ranges of its lines under the last one,
                // each line with its checkbox and its minimum.
                "■ +20 to Evasion Rating ≥18",
                "■ +10% increased Evasion Rating ≥9 [T5 15–25 · 8–12]",
            ]
        );
    }

    #[test]
    fn a_jewel_has_two_prefixes_and_two_suffixes_at_most() {
        let english = parse(
            "Item Class: Jewels\nRarity: Rare\nGlow Spark\nRuby\n--------\n\
             { Prefix Modifier \"Hale\" (Tier: 2) — Life }\n+12(10-14) to maximum Life",
        );
        let french = parse(
            "Classe d'objet: Joyaux\nRareté: Rare\nÉclat\nRubis\n--------\n\
             { Préfixe : « Sain » (Palier : 2) — Vie }\n+12(10-14) de Vie maximale",
        );

        assert!(english.is_jewel());
        assert_eq!(
            texts(&english, None),
            [
                "Item",
                "■ Base: Ruby",
                "Prefixes 1/2",
                "■ +12 to maximum Life ≥11 [T2 10–14]"
            ]
        );
        assert_eq!(
            texts(&french, None),
            [
                "Item",
                "■ Base: Rubis",
                "Prefixes 1/2",
                "■ +12 de Vie maximale ≥11 [T2 10–14]"
            ]
        );
    }

    #[test]
    fn the_attack_speed_of_an_item_that_isnt_a_weapon_stays_checked() {
        let quiver = parse(
            "Item Class: Quivers\nRarity: Magic\nQuick Broadhead Quiver\n--------\n\
             { Suffix Modifier \"of Skill\" (Tier: 4) }\n8(7-9)% increased Attack Speed",
        );

        assert_eq!(
            texts(&quiver, None),
            [
                "Item",
                "■ Base: Quick Broadhead Quiver",
                "Suffixes 1/1",
                "■ 8% increased Attack Speed ≥7 [T4 7–9]",
            ]
        );
    }

    #[test]
    fn a_magic_item_has_one_prefix_and_one_suffix_at_most() {
        let mace = parse(include_str!("../item/test-items/en-magic-mace.txt"));

        assert_eq!(
            texts(&mace, None),
            [
                // Its whole name here: the application finds its base type among the names of
                // the trade site.
                "Item",
                "■ Base: Crackling Temple Maul of the Brute",
                "□ ItemLevel: 32 ≥32",
                "Weapon",
                "■ PhysicalDps: 64 ≥58",
                "□ ElementalDps: 31 ≥28",
                "Prefixes 1/1",
                "□ Adds 1 to 50 Lightning Damage ≥23 [T7 1–4 · 46–66]",
                "Suffixes 1/1",
                "■ +8 to Strength ≥7 [T8 5–8]",
            ]
        );
    }

    #[test]
    fn a_limit_stays_within_the_range_of_its_stat() {
        let ring = parse(
            "Item Class: Rings\nRarity: Unique\nVenom Loop\nAmethyst Ring\n--------\n\
             Item Level: 80\n--------\nAdds 1 to 55 Lightning damage to Attacks",
        );
        let modifiers = database(&[], &["Adds 1 to (50-60) Lightning damage to Attacks"]);
        let boots = parse(
            "Item Class: Boots\nRarity: Rare\nGale Tread\nLattice Sandals\n--------\n\
             { Implicit Modifier }\n+10(10-15) to Dexterity",
        );
        // The fixed number counts in the range of the average; the tooltip doesn't tell it,
        // since it isn't a roll.
        assert_eq!(
            limit(&rows(&ring, Some(&modifiers))[1]),
            Limit {
                value: 28.0,
                half: Half::Minimum,
                bounds: Some(Range {
                    min: 25.5,
                    max: 30.5
                }),
                rolls: None,
            }
        );
        // 90 % of 10 is under the lowest roll of the implicit.
        let range = Some(Range {
            min: 10.0,
            max: 15.0,
        });
        assert_eq!(
            limit(&rows(&boots, None)[3]),
            Limit {
                value: 10.0,
                half: Half::Minimum,
                bounds: range,
                rolls: range,
            }
        );
    }

    /// The limit of the filter of `row`.
    fn limit(row: &Row<'_>) -> Limit {
        match row {
            Row::Stat(Stat {
                filter:
                    Some(StatFilter {
                        limit: Some(limit), ..
                    }),
                ..
            }) => *limit,
            row => panic!("{row:?}"),
        }
    }

    #[test]
    fn a_line_better_lower_is_searched_with_a_maximum() {
        let unique = parse(
            "Item Class: Boots\nRarity: Unique\nStormtread\nLattice Sandals\n--------\n\
             { Unique Modifier }\n+52(40-60) to maximum Life\n\
             { Unique Modifier }\n18(15-25)% reduced Movement Speed",
        );
        let rare = parse(
            "Item Class: Rings\nRarity: Rare\nGale Loop\nRuby Ring\n--------\n\
             { Implicit Modifier }\n12(10-15)% increased Charm Charges used\n--------\n\
             { Suffix Modifier \"of the Hare\" (Tier: 2) }\n18(15-25)% reduced Movement Speed",
        );

        assert_eq!(
            texts(&unique, None),
            [
                "Unique",
                "■ +52 to maximum Life ≥52 [40–60 0.60]",
                // Its bar still shows where its value falls in its range.
                "■ 18% reduced Movement Speed ≤18 [15–25 0.30]",
            ]
        );
        // 110 % of the copied value, within the range of the implicit.
        assert_eq!(
            texts(&rare, None),
            [
                "Item",
                "■ Base: Ruby Ring",
                "Implicit",
                "■ 12% increased Charm Charges used ≤13 [10–15 0.40]",
                "Suffixes 1/3",
                "■ 18% reduced Movement Speed ≤20 [T2 15–25]",
            ]
        );
        let lower = limit(&rows(&unique, None)[2]);
        assert_eq!(lower.half, Half::Maximum);
        assert_eq!(
            StatFilter {
                checked: true,
                limit: Some(lower),
            }
            .default_choice(),
            Filter {
                checked: true,
                minimum: String::new(),
                maximum: "18".to_owned(),
                bounds: Some(Range {
                    min: 15.0,
                    max: 25.0
                }),
            }
        );
    }

    #[test]
    fn a_rare_limit_leaves_10_percent_on_the_worse_side_also_for_a_negative_value() {
        let whole = StatLine::parse("-15% to Fire Resistance");
        let decimal = StatLine::parse("-7.5% to Fire Resistance");
        let limit = |value, line, half| comparable(value, line, half);

        assert!((limit(12.0, &whole, Half::Minimum) - 11.0).abs() < 1e-9);
        assert!((limit(18.0, &whole, Half::Maximum) - 20.0).abs() < 1e-9);
        // The item passes its own filter.
        assert!((limit(-15.0, &whole, Half::Minimum) - -17.0).abs() < 1e-9);
        assert!((limit(-15.0, &whole, Half::Maximum) - -14.0).abs() < 1e-9);
        assert!((limit(-7.5, &decimal, Half::Minimum) - -8.25).abs() < 1e-9);
    }

    #[test]
    fn the_tooltip_of_a_range_field_tells_its_rolls_and_its_better_side() {
        let i18n = Translator::new(crate::language::Language::English);
        let range = Some(Range {
            min: 15.0,
            max: 25.0,
        });
        let tooltip = |half, rolls| {
            let limit = Limit {
                value: 18.0,
                half,
                bounds: range,
                rolls,
            };
            tooltip_lines(limit, &i18n)
        };
        let rolls = ("Rolls 15 to 25".to_owned(), color::TEXT);
        let lower = ("Lower is better".to_owned(), color::WEAK);

        assert_eq!(tooltip(Half::Minimum, range), std::slice::from_ref(&rolls));
        assert_eq!(tooltip(Half::Maximum, range), [rolls, lower.clone()]);
        // A line of several numbers, or a rare before the modifier tables.
        assert_eq!(tooltip(Half::Maximum, None), [lower]);
        assert_eq!(tooltip(Half::Minimum, None), []);
    }

    #[test]
    fn a_range_field_holds_four_characters_a_side_when_its_stat_reaches_them() {
        let line = StatLine::parse("+2250 Armour if you've Blocked Recently");
        let wide = |value, min, max| {
            Limit::new(value, Half::Minimum, Some(Range { min, max }), value, &line).wide()
        };

        assert!(!wide(245.0, 200.0, 320.0));
        assert!(!wide(-15.0, -20.0, -10.0));
        assert!(wide(2250.0, 1500.0, 3000.0));
        // A decimal point or a minus sign counts.
        assert!(wide(16.3, 15.0, 20.0));
        assert!(wide(-20.5, -25.0, -10.0));
        // Without a range, e.g. a rare at 90 % before the modifier tables: its value.
        assert!(Limit::new(6.83, Half::Minimum, None, 7.59, &line).wide());
        assert!(!Limit::new(163.0, Half::Minimum, None, 181.0, &line).wide());
    }

    #[test]
    fn a_negative_stat_takes_a_minus_sign() {
        let line = StatLine::parse("-15% to Fire Resistance");
        let negative =
            |value, bounds| Limit::new(value, Half::Minimum, bounds, value, &line).negative();

        assert!(negative(-15.0, None));
        assert!(!negative(15.0, None));
        // A range across zero.
        assert!(negative(
            2.0,
            Some(Range {
                min: -5.0,
                max: 10.0
            })
        ));
        // The price database may write a negative range without its sign.
        assert_eq!(
            signed(
                Range {
                    min: 10.0,
                    max: 20.0
                },
                -15.0
            ),
            Range {
                min: -20.0,
                max: -10.0
            }
        );
    }

    #[test]
    fn a_corrupted_item_is_compared_with_corrupted_offers_only() {
        let headhunter = parse(
            "Item Class: Belts\nRarity: Unique\nHeadhunter\nHeavy Belt\n--------\n\
             Item Level: 80\n--------\n+55 to maximum Life\n--------\nCorrupted",
        );
        let staff = parse(
            "Item Class: Staves\nRarity: Normal\nChiming Staff\n--------\n\
             Item Level: 82\n--------\nCorrupted",
        );

        assert_eq!(
            texts(&headhunter, None),
            ["Unique", "■ +55 to maximum Life ≥55", "Corrupted only"]
        );
        // An item without modifiers has its filters to show.
        let stats = |item| {
            Stats::new(
                item,
                &CopiedLines::default(),
                &LineStats::default(),
                None,
                base_type(item),
            )
        };
        assert!(!stats(&staff).is_empty());
        assert_eq!(
            texts(&staff, None),
            [
                "Item",
                "■ Base: Chiming Staff",
                "□ ItemLevel: 82 ≥82",
                "Corrupted only"
            ]
        );
        let orb = parse("Item Class: Stackable Currency\nRarity: Currency\nExalted Orb");
        assert!(stats(&orb).is_empty());
    }

    #[test]
    fn a_line_the_trade_site_cant_search_isnt_a_filter() {
        let unique = parse(
            "Item Class: Boots\nRarity: Unique\nStormtread\nLattice Sandals\n--------\n\
             Item Level: 72\n--------\n\
             { Unique Modifier }\n+52(40-60) to maximum Life\n\
             { Unique Modifier }\nGain 3(2-4) Thunder Charges when you Dodge Roll\n\
             { Unique Modifier }\n20(15-25)% chance to gain Onslaught for 3 seconds when you kill an\n\
             enemy affected by Abyssal Wasting\n\
             { Unique Modifier }\nLife that would be lost by taking Damage is instead Reserved\n\
             until you take no Damage to Life for 5(4-6) seconds",
        );
        let lines = CopiedLines::new(
            unique.language(),
            unique.modifiers(),
            &LineStats::default(),
            None,
        );
        let line_stats = LineStats::new(unique.language(), unique.modifiers(), None);
        let stats = Stats::new(&unique, &lines, &line_stats, None, None);

        assert_eq!(
            texts(&unique, None),
            [
                "Unique",
                "■ +52 to maximum Life ≥52 [40–60 0.60]",
                // Its roll still shows, without a filter.
                "⊘ Gain 3 Thunder Charges when you Dodge Roll [2–4 0.50]",
                // The second line of a stat is searched by the first.
                // Its field holds the 20, not the 3 its string writes.
                "■ 20% chance to gain Onslaught for 3 seconds when you kill an ≥20 [15–25 0.50]",
                "enemy affected by Abyssal Wasting",
                // Or by the line with its numbers.
                "Life that would be lost by taking Damage is instead Reserved",
                "■ until you take no Damage to Life for 5 seconds ≤5 [4–6 0.50]",
            ]
        );
        let filters = Filters::default();
        assert_eq!(stats.active_filters(&filters), 3);
        let checked: Vec<FilterKey> = stats
            .checked(&filters)
            .into_iter()
            .map(|(key, _)| key)
            .collect();
        let key = |modifier, line| FilterKey::Stat(StatKey { modifier, line });
        assert_eq!(checked, [key(0, 0), key(2, 0), key(3, 1)]);
    }

    #[test]
    fn the_active_filters_count_the_choices_of_the_player() {
        let crossbow = parse(include_str!(
            "../item/test-items/en-rare-crossbow-all-modifiers.txt"
        ));
        let mut filters = Filters::default();

        let lines = CopiedLines::new(
            crossbow.language(),
            crossbow.modifiers(),
            &LineStats::default(),
            None,
        );
        let line_stats = LineStats::new(crossbow.language(), crossbow.modifiers(), None);
        let stats = Stats::new(&crossbow, &lines, &line_stats, None, base_type(&crossbow));
        let keys: Vec<FilterKey> = stats
            .rows
            .iter()
            .filter_map(|row| match row {
                Row::Stat(stat) => Some(stat.key),
                _ => None,
            })
            .collect();
        let choice = |checked| Filter {
            checked,
            minimum: String::new(),
            maximum: String::new(),
            bounds: None,
        };

        // The base type, the physical DPS, the implicit and three modifiers; the item level,
        // the elemental DPS, the enchantment, the two runes, the damage and the attack speed
        // unchecked.
        assert_eq!(stats.active_filters(&filters), 6);
        // A rune checked by the player.
        filters.set(keys[5], choice(true));
        assert_eq!(stats.active_filters(&filters), 7);
        // The implicit unchecked.
        filters.set(keys[7], choice(false));
        assert_eq!(stats.active_filters(&filters), 6);
        filters.clear();
        assert_eq!(stats.active_filters(&filters), 6);
    }

    #[test]
    fn a_legacy_unique_keeps_its_value_out_of_the_range_of_today() {
        let belt = parse(
            "Item Class: Belts\nRarity: Unique\nHeadhunter\nHeavy Belt\n--------\n\
             Item Level: 80\n--------\n+65 to maximum Life",
        );
        let modifiers = database(&[], &["+(40-60) to maximum Life"]);
        let rows = rows(&belt, Some(&modifiers));

        assert_eq!(
            rows[1],
            Row::Stat(Stat {
                key: FilterKey::Stat(StatKey {
                    modifier: 0,
                    line: 0
                }),
                text: StatText::Line("+65 to maximum Life"),
                filter: Some(StatFilter {
                    checked: true,
                    limit: Some(Limit {
                        value: 65.0,
                        half: Half::Minimum,
                        bounds: Some(Range {
                            min: 40.0,
                            max: 65.0
                        }),
                        rolls: Some(Range {
                            min: 40.0,
                            max: 60.0
                        }),
                    }),
                }),
                unsearchable: false,
                roll: Some(Roll::Bar {
                    share: 1.0,
                    ranges: vec![Range {
                        min: 40.0,
                        max: 60.0
                    }],
                }),
            })
        );
    }

    #[test]
    fn a_range_of_one_value_is_full() {
        assert!((share(5.0, Range { min: 5.0, max: 5.0 }) - 1.0).abs() < 1e-9);
        assert!(
            (share(
                30.0,
                Range {
                    min: 20.0,
                    max: 40.0
                }
            ) - 0.5)
                .abs()
                < 1e-9
        );
    }
}
