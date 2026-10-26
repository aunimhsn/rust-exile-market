//! The stats of the item, unfolded under the item block: its modifiers in the color of the
//! game, grouped, the stats that vary first. A unique, or an implicit modifier of a rare,
//! shows a bar of where its value falls in its range; a prefix or a suffix, its tier and the
//! range of that tier (its bar over every tier comes with the modifier tables, step 13).
//!
//! A stat that is a filter of the search starts with its checkbox and keeps the column of its
//! minimum on the right, also when it has no minimum; a stat of a unique that never varies
//! isn't a filter and starts at the left edge. A corrupted item ends with a note: the search
//! compares it with corrupted offers only.
//!
//! The rows are decided apart from the drawing ([`rows`]), and tested.

use egui::text::{LayoutJob, TextFormat};
use egui::{
    Align, Align2, CornerRadius, Frame, Label, Layout, Margin, Rect, RichText, Sense, Stroke,
    TextEdit, Ui, Vec2, vec2,
};

use super::number_field::NumberField;
use super::theme::{color, size, text};
use super::widgets;
use crate::filters::{self, Filter, Filters, StatKey};
use crate::i18n::{Message, Translator};
use crate::item::{Affix, Item, ItemState, Modifier, ModifierKind, Range, Rarity, StatLine};
use crate::prices::UniqueModifiers;
use crate::{format, rolls};

/// The space between two rows.
const ROW_GAP: f32 = 6.0;
/// The space added above a small title, after the first.
const GROUP_GAP: f32 = 6.0;
/// The column on the left of a filter, for its checkbox.
const GUTTER: f32 = 20.0;
/// The space between the columns of a stat, and between a bar and its range.
const GAP: f32 = 8.0;
/// The column on the right of a filter, for its minimum: the text never runs under it.
const MIN_COLUMN: f32 = 46.0;
/// The space around the number of a minimum field.
const MIN_PADDING: Margin = Margin::symmetric(6, 1);
/// The lines of the text of a stat, at most; the whole text is in its tooltip.
const MAX_LINES: usize = 2;
/// The bar of a roll.
const BAR_SIZE: Vec2 = vec2(96.0, 4.0);
/// The space above the roll of a stat.
const ROLL_GAP: f32 = 3.0;
/// The separator of the ranges of a stat with several rolls.
const RANGE_SEPARATOR: &str = " · ";
/// The share of the copied value given to the minimum of a rare, magic or normal item, to
/// find comparable items and not only identical ones.
const COMPARABLE_SHARE: f64 = 0.9;

/// A small title of the stats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Group {
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

/// A filter of the search, as it starts.
#[derive(Debug, Clone, Copy, PartialEq)]
struct StatFilter {
    checked: bool,
    /// None for a line that never varies, e.g. a fixed implicit of a rare.
    minimum: Option<Minimum>,
}

/// The minimum of a filter: its value at first, and the range it stays in, when known.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Minimum {
    value: f64,
    bounds: Option<Range>,
}

impl Minimum {
    /// A minimum of `value`, brought back within `bounds`.
    fn new(value: f64, bounds: Option<Range>) -> Self {
        let value = bounds.map_or(value, |bounds| bounds.clamp(value));
        Self { value, bounds }
    }

    /// The minus sign is allowed in the field.
    fn negative(self) -> bool {
        self.value < 0.0 || self.bounds.is_some_and(|bounds| bounds.min < 0.0)
    }
}

impl StatFilter {
    /// The filter as the player finds it.
    fn default_choice(self) -> Filter {
        Filter {
            checked: self.checked,
            minimum: self
                .minimum
                .map(|minimum| filters::minimum_text(minimum.value))
                .unwrap_or_default(),
            bounds: self.minimum.and_then(|minimum| minimum.bounds),
        }
    }
}

/// A stat: a line of a modifier.
#[derive(Debug, Clone, PartialEq)]
struct Stat<'a> {
    key: StatKey,
    text: &'a str,
    /// A filter of the search, with its checkbox and its minimum.
    filter: Option<StatFilter>,
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
    /// The stats of `item`; `unique_modifiers` are the modifiers of its unique in the price
    /// database, the ranges of a unique copied without them.
    #[must_use]
    pub fn new(item: &'a Item, unique_modifiers: Option<&UniqueModifiers>) -> Self {
        Self {
            rows: rows(item, unique_modifiers),
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
        let checked = self
            .rows
            .iter()
            .filter(|row| match row {
                Row::Stat(Stat {
                    key,
                    filter: Some(filter),
                    ..
                }) => filters.get(*key, || filter.default_choice()).checked,
                _ => false,
            })
            .count();
        u32::try_from(checked).unwrap_or(u32::MAX)
    }
}

fn len<T>(list: &[T]) -> u32 {
    u32::try_from(list.len()).unwrap_or(u32::MAX)
}

/// The rows of the stats of `item`; `unique_modifiers` are the modifiers of its unique in the
/// price database, the ranges of a unique copied without them.
fn rows<'a>(item: &'a Item, unique_modifiers: Option<&UniqueModifiers>) -> Vec<Row<'a>> {
    let mut rows = if item.rarity() == Rarity::Unique {
        unique_rows(item, unique_modifiers)
    } else {
        affix_rows(item)
    };
    // The corrupted state is always a filter; a currency is never corrupted.
    let corrupted = [ItemState::Corrupted, ItemState::TwiceCorrupted]
        .into_iter()
        .any(|state| item.has_state(state));
    if corrupted && item.rarity() != Rarity::Currency {
        rows.push(Row::CorruptedOnly);
    }
    rows
}

/// The rows of a unique: its enchantments, its runes, the implicit modifiers of its base,
/// then its own modifiers.
fn unique_rows<'a>(item: &'a Item, unique_modifiers: Option<&UniqueModifiers>) -> Vec<Row<'a>> {
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
    push_group(
        &mut rows,
        Group::Enchant,
        stats(item, is_enchant, added, varies),
    );
    push_group(&mut rows, Group::Runes, stats(item, is_rune, added, varies));
    push_group(
        &mut rows,
        Group::Implicit,
        stats(
            item,
            is_implicit,
            |key, line| unique_stat(key, line, implicit),
            varies,
        ),
    );
    push_group(
        &mut rows,
        Group::Unique,
        stats(
            item,
            |kind| matches!(kind, ModifierKind::Explicit(_)),
            |key, line| unique_stat(key, line, explicit),
            varies,
        ),
    );
    rows
}

/// The rows of a rare, magic or normal item: its enchantments, its runes, its implicit
/// modifiers, then its prefixes and suffixes.
fn affix_rows(item: &Item) -> Vec<Row<'_>> {
    let max = match item.rarity() {
        Rarity::Magic => 1,
        _ if item.is_jewel() => 2,
        _ => 3,
    };
    let line_varies = |line: &StatLine, _: &Stat<'_>| varies(line);
    let prefixes = affix_stats(item, Some(Affix::Prefix));
    let suffixes = affix_stats(item, Some(Affix::Suffix));
    let mut rows = Vec::new();
    push_group(
        &mut rows,
        Group::Enchant,
        stats(item, is_enchant, added, line_varies),
    );
    push_group(
        &mut rows,
        Group::Runes,
        stats(item, is_rune, added, line_varies),
    );
    push_group(
        &mut rows,
        Group::Implicit,
        stats(item, is_implicit, implicit_stat, line_varies),
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
    push_group(&mut rows, Group::Explicit, affix_stats(item, None).stats);
    rows
}

/// A line of an enchantment or a rune: a filter, since the trade site searches them, without
/// a roll; unchecked, a rune being easy to change.
fn added(key: StatKey, line: &StatLine) -> Stat<'_> {
    Stat {
        key,
        text: &line.text,
        filter: Some(StatFilter {
            checked: false,
            minimum: value(line).map(|value| Minimum::new(value, None)),
        }),
        roll: None,
    }
}

/// A line of an implicit modifier of a rare, magic or normal item: a filter, rolled in the
/// range its base gives it.
fn implicit_stat(key: StatKey, line: &StatLine) -> Stat<'_> {
    let ranges = copied_ranges(line);
    Stat {
        key,
        text: &line.text,
        filter: Some(comparable_filter(line, bounds(line, &ranges))),
        roll: bar(magnitudes(line, &ranges)),
    }
}

/// A filter of a rare, magic or normal item: checked, with a minimum at 90 % of the copied
/// value when the line varies.
fn comparable_filter(line: &StatLine, bounds: Option<Range>) -> StatFilter {
    StatFilter {
        checked: true,
        minimum: varies(line).then(|| value(line)).flatten().map(|value| {
            Minimum::new(
                comparable(value, line),
                bounds.map(|bounds| bounds.holding(value)),
            )
        }),
    }
}

/// 90 % of `value`, a value of `line`: rounded when the line has whole numbers only.
fn comparable(value: f64, line: &StatLine) -> f64 {
    let share = value * COMPARABLE_SHARE;
    if line.rolls.iter().all(|roll| roll.value.fract() == 0.0) {
        share.round()
    } else {
        share
    }
}

/// The number of modifiers of an affix, and their stats.
struct AffixStats<'a> {
    count: u32,
    stats: Vec<Stat<'a>>,
}

/// The explicit modifiers of `affix` (`None`: copied without their affixes), those with a
/// line that varies first, each with its tier and its ranges under its last line.
fn affix_stats(item: &Item, affix: Option<Affix>) -> AffixStats<'_> {
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
                .map(move |(line_index, line)| Stat {
                    key: StatKey {
                        modifier: index,
                        line: line_index,
                    },
                    text: &line.text,
                    // The range of every tier comes with the modifier tables (step 13).
                    filter: Some(comparable_filter(line, None)),
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

/// A line of a unique: a filter checked with its value as minimum and its bar when it
/// varies, at the left edge otherwise.
fn unique_stat<'a>(key: StatKey, line: &'a StatLine, source: RangeSource<'_>) -> Stat<'a> {
    let ranges = match source {
        RangeSource::Copied => Some(copied_ranges(line)),
        RangeSource::Database(modifiers) => rolls::unique_ranges(&line.text, modifiers)
            // One range or none for each number of the line.
            .filter(|ranges| ranges.len() == line.rolls.len()),
        RangeSource::Unknown => None,
    };
    let fixed = Stat {
        key,
        text: &line.text,
        filter: None,
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
            minimum: Some(Minimum::new(
                value,
                bounds.map(|bounds| bounds.holding(value)),
            )),
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
        show(ui, self.rows, filters, i18n);
    }
}

fn show(ui: &mut Ui, rows: Vec<Row<'_>>, filters: &mut Filters, i18n: &Translator) {
    // "min" on the right of the first title, above the minimums.
    let mut minimum_title = rows.iter().any(|row| {
        matches!(
            row,
            Row::Stat(Stat {
                filter: Some(StatFilter {
                    minimum: Some(_),
                    ..
                }),
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
                group_title(ui, group, std::mem::take(&mut minimum_title), i18n);
            }
            Row::Stat(stat) => stat_row(ui, &stat, filters),
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
/// "min" on its right, above the minimums of the filters.
fn group_title(ui: &mut Ui, group: Group, minimum: bool, i18n: &Translator) {
    let (message, fill) = match group {
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
        if minimum {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                small_title(ui, Message::FilterMinimum);
            });
        }
    });
}

/// A stat in the color of the modifiers, on two lines at most, and its roll under it; a
/// filter between its checkbox and the column of its minimum.
fn stat_row(ui: &mut Ui, stat: &Stat<'_>, filters: &mut Filters) {
    ui.horizontal_top(|ui| {
        let width = ui.available_width();
        let Some(filter) = stat.filter else {
            stat_text(ui, stat, width);
            return;
        };
        let mut choice = filters.get(stat.key, || filter.default_choice());
        let before = choice.clone();
        // The checkbox on the first line of the text.
        let line_height = ui.fonts_mut(|fonts| fonts.row_height(&text::body()));
        if widgets::check_box(ui, choice.checked, vec2(GUTTER, line_height), stat.text).clicked() {
            choice.checked = !choice.checked;
        }
        ui.add_space(GAP);
        stat_text(ui, stat, (width - GUTTER - GAP - GAP - MIN_COLUMN).max(0.0));
        ui.add_space(GAP);
        if let Some(minimum) = filter.minimum {
            minimum_field(ui, stat.key, &mut choice, minimum.negative());
        }
        if choice != before {
            filters.set(stat.key, choice);
        }
    });
}

/// The text of a stat and its roll, `width` across.
fn stat_text(ui: &mut Ui, stat: &Stat<'_>, width: f32) {
    ui.allocate_ui_with_layout(vec2(width, 0.0), Layout::top_down(Align::Min), |ui| {
        ui.set_width(width);
        ui.add(Label::new(clamped(stat.text)).wrap());
        if let Some(roll) = &stat.roll {
            ui.add_space(ROLL_GAP);
            roll_line(ui, roll);
        }
    });
}

/// The minimum of a filter, in its column: a number only, brought back within the range of
/// the stat when the player leaves the field.
fn minimum_field(ui: &mut Ui, key: StatKey, choice: &mut Filter, negative: bool) {
    let id = ui.make_persistent_id(("stat-minimum", key));
    let focused = ui.memory(|memory| memory.has_focus(id));
    let line = if focused {
        color::WEAK
    } else {
        color::CONTROL_LINE
    };
    let frame = Frame::NONE
        .fill(color::CONTROL)
        .stroke(Stroke::new(1.0, line))
        .corner_radius(CornerRadius::same(size::RADIUS))
        .inner_margin(MIN_PADDING);
    let mut field = NumberField {
        text: &mut choice.minimum,
        negative,
    };
    let response = ui.add(
        TextEdit::singleline(&mut field)
            .id(id)
            .font(text::minimum())
            .text_color(color::TEXT)
            .align(Align2::RIGHT_CENTER)
            .frame(frame)
            .desired_width(MIN_COLUMN),
    );
    if response.lost_focus() {
        choice.minimum = choice
            .searched_minimum()
            .map(filters::minimum_text)
            .unwrap_or_default();
    }
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

/// The bar of a roll and its ranges, or the tier of a prefix or suffix and the ranges of
/// that tier.
fn roll_line(ui: &mut Ui, roll: &Roll) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = GAP;
        match roll {
            Roll::Bar { share, ranges } => {
                let (rect, _) = ui.allocate_exact_size(BAR_SIZE, Sense::hover());
                let radius = BAR_SIZE.y / 2.0;
                let painter = ui.painter();
                painter.rect_filled(rect, radius, color::CONTROL_LINE);
                let fill = Rect::from_min_size(
                    rect.min,
                    vec2(rect.width() * share.clamp(0.0, 1.0), rect.height()),
                );
                painter.rect_filled(fill, radius, color::ACCENT);
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

    /// The rows as texts: a title (`Prefixes 2/3`), the note of a corrupted item, or a stat,
    /// first `■ ` for a checked filter or `□ ` for an unchecked one, then its minimum and its
    /// roll (`■ +55 to maximum Life ≥55 [40–60 0.75]`, `■ +3 to Level [T1 3–3]`).
    fn texts(item: &Item, unique_modifiers: Option<&UniqueModifiers>) -> Vec<String> {
        rows(item, unique_modifiers)
            .into_iter()
            .map(|row| match row {
                Row::Group(Group::Prefixes { filled, max }) => format!("Prefixes {filled}/{max}"),
                Row::Group(Group::Suffixes { filled, max }) => format!("Suffixes {filled}/{max}"),
                Row::Group(group) => format!("{group:?}"),
                Row::CorruptedOnly => "Corrupted only".to_owned(),
                Row::Stat(Stat {
                    text, filter, roll, ..
                }) => {
                    let check = match filter {
                        Some(StatFilter { checked: true, .. }) => "■ ",
                        Some(StatFilter { checked: false, .. }) => "□ ",
                        None => "",
                    };
                    let minimum = filter
                        .and_then(|filter| filter.minimum)
                        .map(|minimum| format!(" ≥{}", format::number(minimum.value)))
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
                    format!("{check}{text}{minimum}{roll}")
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
            "■ When you kill a Rare monster, you gain its Modifiers for 60 seconds ≥60",
            "■ +55 to maximum Life ≥55",
        ];
        let partial = database(
            &[],
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
                "■ 26% increased Stun Threshold ≥26",
                "Unique",
                "■ +55 to maximum Life ≥55",
                "When you kill a Rare monster, you gain its Modifiers for 60 seconds",
            ]
        );
    }

    #[test]
    fn an_implicit_and_an_explicit_of_the_same_stat_keep_their_own_ranges() {
        let ring = parse(
            "Item Class: Rings\nRarity: Unique\nVenom Loop\nAmethyst Ring\n--------\n\
             Item Level: 80\n--------\n\
             +10% to Chaos Resistance (implicit)\n--------\n+20% to Chaos Resistance\n\
             Adds 1 to 55 Lightning Damage to Attacks",
        );
        let modifiers = database(
            &["+(7-13)% to Chaos Resistance"],
            &[
                "+(17-23)% to Chaos Resistance",
                "Adds 1 to (50-60) Lightning Damage to Attacks",
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
                "■ Adds 1 to 55 Lightning Damage to Attacks ≥28 [50–60 0.50]",
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
                // 90 % of the copied value, of the average of two numbers.
                "■ Adds 10 to 273 Lightning Damage ≥127 [T2 1–16 · 239–300]",
                "■ Adds 24 to 51 Physical Damage ≥34 [T3 23–35 · 39–59]",
                "■ 109% increased Elemental Damage with Attacks ≥98 [T2 100–119]",
                "Suffixes 3/3",
                "■ Leeches 7.59% of Physical Damage as Mana ≥6.83 [T1 7–7.9]",
                "■ Gain 21 Mana per enemy killed ≥19 [T3 21–27]",
                "■ 18% increased Attack Speed ≥16 [T1 17–19]",
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
            ["Prefixes 1/2", "■ +12 to maximum Life ≥11 [T2 10–14]"]
        );
        assert_eq!(
            texts(&french, None),
            ["Prefixes 1/2", "■ +12 de Vie maximale ≥11 [T2 10–14]"]
        );
    }

    #[test]
    fn a_magic_item_has_one_prefix_and_one_suffix_at_most() {
        let mace = parse(include_str!("../item/test-items/en-magic-mace.txt"));

        assert_eq!(
            texts(&mace, None),
            [
                "Prefixes 1/1",
                "■ Adds 1 to 50 Lightning Damage ≥23 [T7 1–4 · 46–66]",
                "Suffixes 1/1",
                "■ +8 to Strength ≥7 [T8 5–8]",
            ]
        );
    }

    #[test]
    fn a_minimum_stays_within_the_range_of_its_stat() {
        let ring = parse(
            "Item Class: Rings\nRarity: Unique\nVenom Loop\nAmethyst Ring\n--------\n\
             Item Level: 80\n--------\nAdds 1 to 55 Lightning Damage to Attacks",
        );
        let modifiers = database(&[], &["Adds 1 to (50-60) Lightning Damage to Attacks"]);
        let boots = parse(
            "Item Class: Boots\nRarity: Rare\nGale Tread\nLattice Sandals\n--------\n\
             { Implicit Modifier }\n+10(10-15) to Dexterity",
        );
        let minimum = |rows: Vec<Row<'_>>| match &rows[1] {
            Row::Stat(Stat {
                filter:
                    Some(StatFilter {
                        minimum: Some(minimum),
                        ..
                    }),
                ..
            }) => *minimum,
            row => panic!("{row:?}"),
        };

        // The fixed number counts in the range of the average.
        assert_eq!(
            minimum(rows(&ring, Some(&modifiers))),
            Minimum {
                value: 28.0,
                bounds: Some(Range {
                    min: 25.5,
                    max: 30.5
                }),
            }
        );
        // 90 % of 10 is under the lowest roll of the implicit.
        assert_eq!(
            minimum(rows(&boots, None)),
            Minimum {
                value: 10.0,
                bounds: Some(Range {
                    min: 10.0,
                    max: 15.0
                }),
            }
        );
    }

    #[test]
    fn a_negative_stat_takes_a_minus_sign() {
        let negative = |value, bounds| Minimum::new(value, bounds).negative();

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
        assert!(!Stats::new(&staff, None).is_empty());
        assert_eq!(texts(&staff, None), ["Corrupted only"]);
        let clean =
            parse("Item Class: Staves\nRarity: Normal\nChiming Staff\n--------\nItem Level: 82");
        assert!(Stats::new(&clean, None).is_empty());
    }

    #[test]
    fn the_active_filters_count_the_choices_of_the_player() {
        let crossbow = parse(include_str!(
            "../item/test-items/en-rare-crossbow-all-modifiers.txt"
        ));
        let mut filters = Filters::default();

        let stats = Stats::new(&crossbow, None);
        let keys: Vec<StatKey> = stats
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
            bounds: None,
        };

        // The enchantment and the two runes unchecked.
        assert_eq!(stats.active_filters(&filters), 7);
        // A rune checked by the player.
        filters.set(keys[1], choice(true));
        assert_eq!(stats.active_filters(&filters), 8);
        // The implicit unchecked.
        filters.set(keys[3], choice(false));
        assert_eq!(stats.active_filters(&filters), 7);
        filters.clear();
        assert_eq!(stats.active_filters(&filters), 7);
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
                key: StatKey {
                    modifier: 0,
                    line: 0
                },
                text: "+65 to maximum Life",
                filter: Some(StatFilter {
                    checked: true,
                    minimum: Some(Minimum {
                        value: 65.0,
                        bounds: Some(Range {
                            min: 40.0,
                            max: 65.0
                        }),
                    }),
                }),
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
