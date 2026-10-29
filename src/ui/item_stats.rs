//! The stats of the item, unfolded under the item block: for a rare, magic or normal item,
//! first the filters of its base type, item level, sockets, quality, required level and DPS,
//! then of its pseudo totals for a rare or magic one, then its modifiers in the color of the
//! game, grouped, the stats that vary first. A unique, or an implicit modifier of a rare, shows
//! a bar of where its value falls in its range; a line of a prefix or a suffix, a bar over
//! every tier of its modifier on the base of the item, from the modifier tables, and its range
//! field stays within them; under the last line of its modifier, its tier and the range of that
//! tier.
//!
//! A stat that is a filter of the search starts with its checkbox and keeps the column of its
//! range field on the right, also when it has no limit; a stat of a unique that never varies
//! isn't a filter and starts at the left edge; a line the trade site can't search shows the
//! "not allowed" sign in place of its checkbox. A line better lower is searched with a
//! maximum, any other with a minimum. The damage and attack speed modifiers of a weapon start
//! unchecked: its DPS counts them, the physical one checked, the elemental one when it is at
//! least the physical one. The pseudo totals start checked, and the lines they add up
//! unchecked. A desecrated modifier not revealed yet is a line of its group without filter, on a
//! band of the desecrated green; its item gets a filter of its number of unrevealed modifiers,
//! and its other stats start unchecked, with a note under them.
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
use crate::format;
use crate::game_data::{self, Category};
use crate::i18n::{Message, Translator};
use crate::item::{
    Affix, AffixTier, Item, Modifier, ModifierKind, ModifierOrigin, Range, Rarity, StatLine,
    TierDoubt,
};
use crate::roll_difference::CopiedLines;
use crate::tier_rolls::{LineRolls, TierRolls, TierSpan};
use crate::trade::link;
use crate::trade::pseudo::{Pseudo, PseudoTotal};
use crate::trade::stat_filters::{LineStat, LineStats};

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
/// The range field raised by 1 px on the line of its checkbox: taller than the checkbox, it
/// looked too low (tuned by eye by the owner on the mock-up). The row keeps its height.
const RANGE_RAISE: Margin = Margin {
    left: 0,
    right: 0,
    top: -1,
    bottom: 1,
};
/// The space between a range field and its tooltip, under it.
const TOOLTIP_GAP: f32 = 8.0;
/// The space around the text of the tooltip of a range field, and between its lines.
const TOOLTIP_PADDING: Margin = Margin::symmetric(8, 5);
const TOOLTIP_LINE_GAP: f32 = 3.0;
/// The space above a small line of a tooltip, the tiers of the rolls right under them.
const TOOLTIP_SMALL_LINE_GAP: f32 = 1.0;
/// How far the tooltip of the "not allowed" sign starts left of the sign.
const UNSEARCHABLE_TOOLTIP_SHIFT: f32 = 4.0;
/// The space between the rows of several tiers under a line.
const TIER_ROW_GAP: f32 = 1.0;
/// The lines of the text of a stat, at most; the whole text is in its tooltip.
const MAX_LINES: usize = 2;
/// The bar of a roll.
const BAR_SIZE: Vec2 = vec2(96.0, 4.0);
/// The space above the roll of a stat.
const ROLL_GAP: f32 = 3.0;
/// The space between the range of a tier and the origin of its modifier.
const ORIGIN_GAP: f32 = 6.0;
/// The separator of the ranges of a stat with several rolls.
const RANGE_SEPARATOR: &str = " · ";
/// The share of the copied value the limit of a rare, magic or normal item leaves below it
/// for a minimum, above it for the maximum of a line better lower, to find comparable items
/// and not only identical ones.
const COMPARABLE_MARGIN: f64 = 0.1;
/// The highest quality of an item that isn't exceptional, in percent.
const MAX_USUAL_QUALITY: u32 = 20;
/// The quality from which the quality of a charm is checked, in percent.
const CHARM_QUALITY: u32 = 10;
/// The highest required level and item level of a rare for leveling.
const LEVELING_LEVEL: u32 = 75;
/// The item levels: the field of the item level stays within them. Checked on the trade site
/// on 2026-10-28: 262 offers of item level 100 in Forbidden Rites, none above.
const ITEM_LEVELS: Range = Range {
    min: 1.0,
    max: 100.0,
};
/// The augmentable sockets an item can have: the field of the sockets stays within them. No
/// offer had 7 or more on the trade site on 2026-10-28.
const SOCKETS: Range = Range { min: 0.0, max: 6.0 };
/// The qualities, of any item: the field of the quality stays within them. Checked on the trade
/// site on 2026-10-28: Breach rings of 75 % (the quality of a catalyst), none above; the
/// quality of each kind of item (20 %, an exceptional base, a catalyst) is left to step 13k.
const QUALITIES: Range = Range {
    min: 0.0,
    max: 75.0,
};
/// The required levels: the field of the required level stays within them. Checked on the
/// trade site on 2026-10-28: offers requiring level 90, none above.
const REQUIRED_LEVELS: Range = Range {
    min: 1.0,
    max: 90.0,
};
/// The unrevealed modifiers an item can have: three prefixes and three suffixes.
const UNREVEALED_MODIFIERS: Range = Range { min: 0.0, max: 6.0 };
/// The inner margin of the band of an unrevealed modifier, around its words.
const UNREVEALED_PADDING: Margin = Margin::symmetric(6, 3);
/// The band of an unrevealed modifier reaches 6 px out of the column of the stats, so that its
/// words stay at the left edge.
const UNREVEALED_OUTSET: Margin = Margin::symmetric(-6, 0);

/// A small title of the stats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Group {
    /// The base type, item level, sockets, quality and required level of a rare, magic or
    /// normal item.
    Item,
    /// The DPS of a weapon.
    Weapon,
    /// The totals the trade site adds up from several lines.
    Pseudo,
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
    /// The explicit modifiers of a magic or rare item whose header doesn't tell their affix:
    /// a header the parser doesn't know, or lines before the first header.
    Explicit,
    /// The modifiers of a unique, no prefix nor suffix.
    Unique,
}

/// The roll of a stat.
#[derive(Debug, Clone, PartialEq)]
enum Roll {
    /// Where its value falls in its ranges, from 0 to 1, and the ranges; then, under the last
    /// line of a Vaal modifier of a unique, its origin.
    Bar {
        share: f32,
        ranges: Vec<Range>,
        origin: Option<ModifierOrigin>,
    },
    /// A line of a prefix or a suffix: where its value falls over every tier of its modifier
    /// on the base, from 0 to 1, when the modifier tables know them; under the last line of the
    /// modifier, its tier and the ranges of that tier, of all its lines for a hybrid, so that it
    /// reads as one modifier, then its origin when it is desecrated, fractured or crafted. Also
    /// the origin alone under a line of a unique that never varies (a fixed Vaal modifier),
    /// without bar nor tier.
    Affix {
        share: Option<f32>,
        tier: Option<ModifierTier>,
        origin: Option<ModifierOrigin>,
    },
    /// Under the last line of a modifier copied from the in-game trade that can be a prefix or
    /// a suffix: the tier of each and its ranges, one under the other, a word after the ranges
    /// ("Prefix"), without bar; then its origin on a line of its own, since the line has it
    /// whichever affix it is.
    Tiers {
        rows: Vec<TierRow>,
        origin: Option<ModifierOrigin>,
    },
}

/// The tier of a prefix or suffix, written by the advanced copy or found from the values, and
/// the ranges of that tier.
#[derive(Debug, Clone, PartialEq)]
struct ModifierTier {
    /// 1 is the best.
    number: u32,
    ranges: Vec<Range>,
}

/// A tier among several under a line, and the word after its ranges.
#[derive(Debug, Clone, PartialEq)]
struct TierRow {
    tier: ModifierTier,
    word: Message,
}

impl TierRow {
    /// The tier `found` for one affix of a line that can be either, "Prefix" after its ranges.
    fn affix(found: &AffixTier) -> Self {
        Self {
            tier: ModifierTier {
                number: found.tier,
                ranges: found.ranges.clone(),
            },
            word: match found.affix {
                Affix::Prefix => Message::AffixPrefix,
                Affix::Suffix => Message::AffixSuffix,
            },
        }
    }
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
    /// The tiers of the lowest and of the highest of `rolls`, for a prefix or a suffix, which
    /// the tooltip writes under them: "T13–T1".
    tiers: Option<TierSpan>,
}

impl Limit {
    /// A minimum of `value`, with no range to stay in.
    fn minimum(value: f64) -> Self {
        Self {
            value,
            half: Half::Minimum,
            bounds: None,
            rolls: None,
            tiers: None,
        }
    }

    /// A maximum of `value`, with no range to stay in.
    fn maximum(value: f64) -> Self {
        Self {
            half: Half::Maximum,
            ..Self::minimum(value)
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
            tiers: None,
        }
    }

    /// The limit kept within `bounds`, all the values its property can take, widened to hold
    /// its value: an item the checks didn't meet, or of a patch that raised them, keeps it.
    fn within(self, bounds: Range) -> Self {
        Self {
            bounds: Some(bounds.holding(self.value)),
            ..self
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
    /// The base type, with the item class searched in its place when it is unchecked; none
    /// for a base the game data doesn't know, searched among any base instead.
    BaseType {
        base_type: &'a str,
        class: Option<&'a str>,
    },
    /// A property of the item, with its value.
    Property(Property, f64),
    Pseudo(Pseudo, f64),
    /// A desecrated modifier not revealed yet, of its affix.
    Unrevealed(Affix),
}

impl<'a> StatText<'a> {
    fn text(self, i18n: &Translator) -> Cow<'a, str> {
        match self {
            Self::Line(text) => Cow::Borrowed(text),
            Self::BaseType { base_type, .. } => Cow::Owned(i18n.filter_base_type(base_type)),
            Self::Property(property, value) => {
                Cow::Owned(i18n.filter_property(property, &format::number(value)))
            }
            Self::Pseudo(pseudo, value) => {
                Cow::Owned(i18n.filter_pseudo(pseudo, &format::number(value)))
            }
            Self::Unrevealed(Affix::Prefix) => i18n.text(Message::UnrevealedPrefix).into(),
            Self::Unrevealed(Affix::Suffix) => i18n.text(Message::UnrevealedSuffix).into(),
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
    /// Under the stats of an item with an unrevealed modifier.
    UnrevealedNote,
    /// Under the stats of a rare or magic item copied from the in-game trade, whose tiers are
    /// found from the values.
    TradeCopyNote,
}

/// The number of stats of `item`, for the pill that unfolds them.
#[must_use]
pub fn count(item: &Item) -> u32 {
    len(item.modifiers())
}

/// The stats and filters of an item, decided once for a frame.
pub struct Stats<'a> {
    item: &'a Item,
    rows: Vec<Row<'a>>,
}

impl<'a> Stats<'a> {
    /// The stats of `item`, whose modifier `lines` tell which are better lower, their
    /// `stats` which the trade site can search, and their `tiers` the rolls of every tier of
    /// a prefix or suffix; `base_type` is the base type its search looks for, a filter.
    #[must_use]
    pub fn new(
        item: &'a Item,
        lines: &CopiedLines,
        stats: &LineStats,
        tiers: &TierRolls,
        base_type: Option<&'a str>,
    ) -> Self {
        Self {
            item,
            rows: rows(item, lines, stats, tiers, base_type),
        }
    }

    /// Nothing to unfold: no stat, no filter.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// The number of filters checked for the search, beside the pill of the stats, with the
    /// state tags of the item lit.
    #[must_use]
    pub fn active_filters(&self, filters: &Filters) -> u32 {
        let lit: Vec<_> = filters.lit_states_of(self.item).collect();
        len(&self.checked(filters)).saturating_add(len(&lit))
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

/// The rows of the stats of `item`, whose modifier `lines` tell which are better lower, their
/// `stats` which the trade site can search, and their `tiers` the rolls of every tier of a
/// prefix or suffix; `base_type` is the base type its search looks for, a filter.
fn rows<'a>(
    item: &'a Item,
    lines: &CopiedLines,
    stats: &LineStats,
    tiers: &TierRolls,
    base_type: Option<&'a str>,
) -> Vec<Row<'a>> {
    let mut rows = Vec::new();
    // An unrevealed modifier can change the item: its other stats start unchecked.
    let unrevealed = item.unrevealed_modifiers() > 0;
    let dps = if item.rarity().has_item_filters() {
        push_group(&mut rows, Group::Item, item_filters(item, base_type));
        dps_filters(item, !unrevealed)
    } else {
        Vec::new()
    };
    let pseudo = if matches!(item.rarity(), Rarity::Magic | Rarity::Rare) {
        stats
            .pseudo_totals()
            .iter()
            .map(|total| pseudo_filter(total, !unrevealed))
            .collect()
    } else {
        Vec::new()
    };
    let reading = Reading {
        lines,
        stats,
        tiers,
        dps_filters: !dps.is_empty(),
        pseudo_filters: !pseudo.is_empty(),
        unrevealed,
    };
    push_group(&mut rows, Group::Weapon, dps);
    push_group(&mut rows, Group::Pseudo, pseudo);
    if item.rarity() == Rarity::Unique {
        rows.extend(unique_rows(item, &reading));
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
    if unrevealed {
        rows.push(Row::UnrevealedNote);
    }
    // Once a tier is found, also for a line that can be a prefix or a suffix: none before the
    // names of the trade site tell the base of a magic item, nor on a base the game data
    // doesn't know.
    let found = item.modifiers().iter().any(|modifier| {
        let with_tier =
            matches!(modifier.kind, ModifierKind::Explicit(Some(_))) && modifier.tier.is_some();
        let in_doubt = matches!(modifier.doubt, Some(TierDoubt::EitherAffix { .. }));
        with_tier || in_doubt
    });
    if item.has_tiers_from_values() && found {
        rows.push(Row::TradeCopyNote);
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

/// The filters of the properties of a rare, magic or normal item, like Exiled Exchange 2: its
/// base type, checked (unchecked, its item class is searched in its place); its item level,
/// checked on a normal item only (a player who checks it on another usually looks for a level
/// of crafting), with the level of the item as its minimum; then its sockets, its quality and
/// its required level when they are filters, and the number of its unrevealed modifiers, checked
/// with that number as its minimum.
fn item_filters<'a>(item: &'a Item, base_type: Option<&'a str>) -> Vec<Stat<'a>> {
    let base_type_filter = base_type.map(|base_type| Stat {
        key: FilterKey::BaseType,
        text: StatText::BaseType {
            base_type,
            class: link::searched_category(item, base_type)
                .is_some()
                .then(|| item.class()),
        },
        filter: Some(StatFilter {
            checked: true,
            limit: None,
        }),
        unsearchable: false,
        roll: None,
    });
    let item_level = item.item_level().map(|level| {
        let level = f64::from(level);
        let checked = item.rarity() == Rarity::Normal;
        let limit = Limit::minimum(level).within(ITEM_LEVELS);
        property_filter(Property::ItemLevel, level, limit, checked)
    });
    let category = base_type.and_then(|base_type| game_data::category(item.language(), base_type));
    base_type_filter
        .into_iter()
        .chain(item_level)
        .chain(sockets_filter(item, category))
        .chain(quality_filter(item, category))
        .chain(required_level_filter(item))
        .chain(unrevealed_filter(item))
        .collect()
}

/// The filter of the number of unrevealed modifiers of an item that has some, checked with
/// that number as its minimum.
fn unrevealed_filter(item: &Item) -> Option<Stat<'static>> {
    let count = item.unrevealed_modifiers();
    (count > 0).then(|| {
        let count = f64::from(count);
        Stat {
            key: FilterKey::Pseudo(Pseudo::UnrevealedModifiers),
            text: StatText::Pseudo(Pseudo::UnrevealedModifiers, count),
            filter: Some(StatFilter {
                checked: true,
                limit: Some(Limit::minimum(count).within(UNREVEALED_MODIFIERS)),
            }),
            unsearchable: false,
            roll: None,
        }
    })
}

/// The filter of the augmentable sockets of an item that has some, with their number as its
/// minimum: checked on a normal item, or when the item has more than its base usually has (an
/// exceptional base; never for a base of an unknown `category`), or is corrupted (no socket can
/// be added any more).
fn sockets_filter(item: &Item, category: Option<Category>) -> Option<Stat<'static>> {
    let sockets = item.sockets().filter(|&sockets| sockets > 0)?;
    let more = category.is_some_and(|category| sockets > category.usual_sockets());
    let checked = item.rarity() == Rarity::Normal || more || item.is_corrupted();
    let sockets = f64::from(sockets);
    Some(property_filter(
        Property::Sockets,
        sockets,
        Limit::minimum(sockets).within(SOCKETS),
        checked,
    ))
}

/// The filter of the quality of an item of the base `category`, with its value as its minimum,
/// like Exiled Exchange 2: for a charm, checked from 10 %; for a flask from 20 %, checked above;
/// for any other item above 20 % (an exceptional quality), checked except on a rare, whose craft
/// is usually done; and on a normal item that has quality, unchecked.
fn quality_filter(item: &Item, category: Option<Category>) -> Option<Stat<'static>> {
    let quality = item.quality().filter(|&quality| quality > 0)?;
    let checked = match category {
        Some(Category::Charm) => quality >= CHARM_QUALITY,
        Some(Category::Flask) if quality >= MAX_USUAL_QUALITY => quality > MAX_USUAL_QUALITY,
        _ if quality > MAX_USUAL_QUALITY => item.rarity() != Rarity::Rare,
        _ if item.rarity() == Rarity::Normal => false,
        _ => return None,
    };
    let quality = f64::from(quality);
    Some(property_filter(
        Property::Quality,
        quality,
        Limit::minimum(quality).within(QUALITIES),
        checked,
    ))
}

/// The filter of the required level of a rare for leveling (required level and item level of
/// 75 or under), unchecked, with the required level as its maximum: the offers a character of
/// that level can use.
fn required_level_filter(item: &Item) -> Option<Stat<'static>> {
    let leveling = |level: u32| level <= LEVELING_LEVEL;
    let level = item.requirements().level.filter(|&level| {
        item.rarity() == Rarity::Rare && leveling(level) && item.item_level().is_some_and(leveling)
    })?;
    let level = f64::from(level);
    Some(property_filter(
        Property::RequiredLevel,
        level,
        Limit::maximum(level).within(REQUIRED_LEVELS),
        false,
    ))
}

/// The filters of the DPS of a weapon, each kind it deals, with a minimum at 90 % of its
/// DPS, rounded down so that the weapon passes it: when `checkable`, the physical one checked,
/// the elemental one when it is at least the physical one, so that a weapon mostly elemental is
/// compared with elemental ones.
fn dps_filters(item: &Item, checkable: bool) -> Vec<Stat<'static>> {
    let Some(weapon) = item.weapon() else {
        return Vec::new();
    };
    let dps = weapon.dps();
    [
        (Property::PhysicalDps, dps.physical, true),
        (
            Property::ElementalDps,
            dps.elemental,
            dps.elemental >= dps.physical,
        ),
    ]
    .into_iter()
    .filter_map(|(property, dps, checked)| {
        // As the item block shows it.
        let shown = dps.round();
        (shown >= 1.0).then(|| {
            let minimum = (dps * (1.0 - COMPARABLE_MARGIN)).floor();
            property_filter(
                property,
                shown,
                Limit::minimum(minimum),
                checkable && checked,
            )
        })
    })
    .collect()
}

/// The filter of `property`, of the value `value`, with the limit `limit`.
fn property_filter(property: Property, value: f64, limit: Limit, checked: bool) -> Stat<'static> {
    Stat {
        key: FilterKey::Property(property),
        text: StatText::Property(property, value),
        filter: Some(StatFilter {
            checked,
            limit: Some(limit),
        }),
        unsearchable: false,
        roll: None,
    }
}

/// The filter of a pseudo total, with a minimum at 90 % of its value.
fn pseudo_filter(total: &PseudoTotal, checked: bool) -> Stat<'static> {
    let whole = total.value.fract() == 0.0;
    Stat {
        key: FilterKey::Pseudo(total.pseudo),
        text: StatText::Pseudo(total.pseudo, total.value),
        filter: Some(StatFilter {
            checked,
            limit: Some(Limit::minimum(comparable(
                total.value,
                whole,
                Half::Minimum,
            ))),
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
    /// They tell the rolls of every tier of a line of a prefix or suffix.
    tiers: &'r TierRolls,
    /// The item has the filters of the DPS of a weapon.
    dps_filters: bool,
    /// The item has the filters of its pseudo totals.
    pseudo_filters: bool,
    /// The item has an unrevealed modifier, which can change it.
    unrevealed: bool,
}

impl Reading<'_> {
    /// The line `key` starts checked: unchecked when the DPS of the weapon counts it, its
    /// damage or its attack speed, when a pseudo total adds it up, or on an item with an
    /// unrevealed modifier.
    fn checked(&self, key: StatKey) -> bool {
        let weapon_damage = self.dps_filters && self.stats.weapon_damage(key);
        let added_up = self.pseudo_filters && self.stats.added_up(key);
        !weapon_damage && !added_up && !self.unrevealed
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
fn unique_rows<'a>(item: &'a Item, reading: &Reading<'_>) -> Vec<Row<'a>> {
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
    // The origin of a modifier under its last line.
    let line_stat = |key: StatKey, line| {
        let origin = item
            .modifiers()
            .get(key.modifier)
            .filter(|modifier| key.line + 1 == modifier.lines.len())
            .and_then(|modifier| modifier.origin);
        unique_stat(key, line, origin, reading)
    };
    push_group(
        &mut rows,
        Group::Implicit,
        stats(item, is_implicit, line_stat, varies),
    );
    push_group(
        &mut rows,
        Group::Unique,
        stats(
            item,
            |kind| matches!(kind, ModifierKind::Explicit(_)),
            line_stat,
            varies,
        ),
    );
    rows
}

/// The rows of a rare, magic or normal item: its enchantments, its runes, its implicit
/// modifiers, then its prefixes and suffixes.
fn affix_rows<'a>(item: &'a Item, reading: &Reading<'_>) -> Vec<Row<'a>> {
    // A rune or an implicit can add affixes or take some away (`+1 Suffix Modifier allowed`).
    let limits = reading.stats.affix_limits(item);
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
            max: limits.prefixes,
        },
        prefixes.stats,
    );
    push_group(
        &mut rows,
        Group::Suffixes {
            filled: suffixes.count,
            max: limits.suffixes,
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
            varies(&counted),
            reading.half(key),
            reading.checked(key),
        )),
        unsearchable: false,
        roll: bar(magnitudes(&counted, &ranges), None),
    }
}

/// A filter of a rare, magic or normal item, when the line `varies` with a minimum at 90 % of
/// the copied value, or a maximum at 110 % when `half` is the maximum.
fn comparable_filter(
    line: &StatLine,
    bounds: Option<Range>,
    varies: bool,
    half: Half,
    checked: bool,
) -> StatFilter {
    StatFilter {
        checked,
        limit: varies.then(|| value(line)).flatten().map(|value| {
            let whole = line.rolls.iter().all(|roll| roll.value.fract() == 0.0);
            let limit = comparable(value, whole, half);
            Limit::new(limit, half, bounds, value, line)
        }),
    }
}

/// The limit 10 % of `value` below it for a minimum, above it for a maximum, also for a
/// negative value (`-15` gets a minimum of `-17`, not `-14`, which the item itself wouldn't
/// pass): rounded when the numbers are `whole`.
fn comparable(value: f64, whole: bool, half: Half) -> f64 {
    let margin = value.abs() * COMPARABLE_MARGIN;
    let limit = match half {
        Half::Minimum => value - margin,
        Half::Maximum => value + margin,
    };
    if whole { limit.round() } else { limit }
}

/// The number of modifiers of an affix, and their stats.
struct AffixStats<'a> {
    count: u32,
    stats: Vec<Stat<'a>>,
}

/// The explicit modifiers of `affix` (`None`: a header that doesn't tell it), those with a
/// line that varies first and those not revealed yet last: each line with its bar over every
/// tier of its modifier, and its tier and the ranges of that tier under its last line; the
/// tier of each affix for a modifier copied from the in-game trade that can be either.
fn affix_stats<'a>(item: &'a Item, affix: Option<Affix>, reading: &Reading<'_>) -> AffixStats<'a> {
    let mut modifiers: Vec<(usize, &Modifier)> = item
        .modifiers()
        .iter()
        .enumerate()
        .filter(|(_, modifier)| modifier.kind == ModifierKind::Explicit(affix))
        .collect();
    let line_varies = |modifier: usize, (line, text): (usize, &StatLine)| {
        let key = StatKey { modifier, line };
        let counted = reading.counted(key, text);
        affix_varies(&counted, over_tiers(key, reading).as_ref())
    };
    modifiers.sort_by_key(|&(index, modifier)| {
        let fixed = !modifier
            .lines
            .iter()
            .enumerate()
            .any(|line| line_varies(index, line));
        (modifier.unrevealed, fixed)
    });
    let stats = modifiers
        .iter()
        .flat_map(|&(index, modifier)| -> Vec<Stat<'a>> {
            if modifier.unrevealed
                && let ModifierKind::Explicit(Some(affix)) = modifier.kind
            {
                // No line yet: the key of its first one, which no filter has.
                return vec![Stat {
                    key: FilterKey::Stat(StatKey {
                        modifier: index,
                        line: 0,
                    }),
                    text: StatText::Unrevealed(affix),
                    filter: None,
                    unsearchable: false,
                    roll: None,
                }];
            }
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
                    let tier = modifier.tier.filter(|_| line_index == last).map(|number| {
                        let ranges = modifier
                            .lines
                            .iter()
                            .flat_map(|line| magnitudes(line, &copied_ranges(line)))
                            .map(|(_, range)| range)
                            .collect();
                        ModifierTier { number, ranges }
                    });
                    let origin = modifier.origin.filter(|_| line_index == last);
                    let mut stat = affix_stat(key, line, tier, origin, reading);
                    if line_index == last
                        && let Some(TierDoubt::EitherAffix { prefix, suffix }) = &modifier.doubt
                    {
                        stat.roll = Some(Roll::Tiers {
                            rows: vec![TierRow::affix(prefix), TierRow::affix(suffix)],
                            origin,
                        });
                    }
                    stat
                })
                .collect()
        })
        .collect();
    AffixStats {
        count: len(&modifiers),
        stats,
    }
}

/// The line `key` of a prefix or suffix, `line`: a filter that stays within the rolls of every
/// tier of its modifier, its bar over them, and under its last line the tier of the modifier
/// and the ranges of that tier, then its origin.
fn affix_stat<'a>(
    key: StatKey,
    line: &'a StatLine,
    tier: Option<ModifierTier>,
    origin: Option<ModifierOrigin>,
    reading: &Reading<'_>,
) -> Stat<'a> {
    let counted = reading.counted(key, line);
    let over = over_tiers(key, reading);
    let mut filter = comparable_filter(
        &counted,
        over.as_ref()
            .and_then(|over| bounds(&counted, &over.ranges)),
        affix_varies(&counted, over.as_ref()),
        reading.half(key),
        reading.checked(key),
    );
    if let Some(limit) = &mut filter.limit {
        limit.tiers = over
            .as_ref()
            .and_then(|over| over.tiers)
            .filter(|_| limit.rolls.is_some());
    }
    let share = over
        .as_ref()
        .and_then(|over| varying_share(&magnitudes(&counted, &over.ranges)));
    Stat {
        key: FilterKey::Stat(key),
        text: StatText::Line(&line.text),
        filter: Some(filter),
        unsearchable: false,
        roll: (share.is_some() || tier.is_some() || origin.is_some()).then_some(Roll::Affix {
            share,
            tier,
            origin,
        }),
    }
}

/// The rolls over every tier of the line `key`, for the numbers its stat counts; none when
/// the modifier tables don't know them.
fn over_tiers(key: StatKey, reading: &Reading<'_>) -> Option<LineRolls> {
    let rolls = reading.tiers.get(key)?;
    let ranges = reading.pick(key, &rolls.ranges);
    ranges.iter().any(Option::is_some).then_some(LineRolls {
        ranges,
        tiers: rolls.tiers,
    })
}

/// A line of a prefix or suffix varies over the tiers of its modifier when they are known (a
/// line fixed in its tier, `+3 to Level of all Projectile Skills` of tier 2, varies when the
/// other tiers roll other values), or else in its range.
fn affix_varies(counted: &StatLine, over: Option<&LineRolls>) -> bool {
    over.map_or_else(
        || varies(counted),
        |over| {
            over.ranges
                .iter()
                .flatten()
                .any(|range| range.max > range.min)
        },
    )
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
/// varies, a fixed line otherwise; a line without a range in the advanced copy never varies.
/// The `origin` of its modifier (Vaal) follows its range, or stands alone under a fixed line.
fn unique_stat<'a>(
    key: StatKey,
    whole: &'a StatLine,
    origin: Option<ModifierOrigin>,
    reading: &Reading<'_>,
) -> Stat<'a> {
    let line = &reading.counted(key, whole);
    let ranges = copied_ranges(line);
    let fixed = Stat {
        key: FilterKey::Stat(key),
        text: StatText::Line(&whole.text),
        filter: None,
        unsearchable: false,
        roll: origin.map(|origin| Roll::Affix {
            share: None,
            tier: None,
            origin: Some(origin),
        }),
    };
    // A filter when one of its ranges is wider than one value.
    let Some(roll) = bar(magnitudes(line, &ranges), origin) else {
        return fixed;
    };
    let Some(value) = value(line) else {
        return fixed;
    };
    Stat {
        filter: Some(StatFilter {
            checked: true,
            // A legacy unique keeps its value, out of the range of today.
            limit: Some(Limit::new(
                value,
                reading.half(key),
                bounds(line, &ranges),
                value,
                line,
            )),
        }),
        roll: Some(roll),
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

/// `range`, from its lowest to its highest value, with the sign of `value`: the game writes
/// the range of a negative number without its sign.
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

/// The bar of `rolls`, those whose range is wider than one value ([`varying_share`]), their
/// ranges and the `origin` of their modifier; none when no roll varies.
fn bar(rolls: Vec<(f64, Range)>, origin: Option<ModifierOrigin>) -> Option<Roll> {
    let share = varying_share(&rolls)?;
    Some(Roll::Bar {
        share,
        ranges: rolls
            .into_iter()
            .map(|(_, range)| range)
            .filter(|range| range.max > range.min)
            .collect(),
        origin,
    })
}

/// Where `rolls` fall, from 0 to 1: the average of where each value falls in its range, of
/// those whose range is wider than one value; none when no roll varies.
fn varying_share(rolls: &[(f64, Range)]) -> Option<f32> {
    let average = average(
        rolls
            .iter()
            .filter(|(_, range)| range.max > range.min)
            .map(|&(value, range)| share(value.abs(), range)),
    )?;
    #[expect(clippy::cast_possible_truncation, reason = "a share from 0 to 1")]
    Some(average as f32)
}

/// Where `value` falls in `range`, from 0 to 1; 1 for a range of one value.
fn share(value: f64, range: Range) -> f64 {
    if range.max <= range.min {
        return 1.0;
    }
    ((value - range.min) / (range.max - range.min)).clamp(0.0, 1.0)
}

/// The ranges of a stat, e.g. `40–60`, or `8–12 · 16–22` for several rolls; a range of one
/// value as one number, as the game writes it (`29(30)%` of a sanctified item).
fn ranges_text(ranges: &[Range]) -> String {
    ranges
        .iter()
        .map(|range| {
            let (min, max) = (format::number(range.min), format::number(range.max));
            if min == max {
                min
            } else {
                format!("{min}–{max}")
            }
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
            Row::UnrevealedNote => note(ui, i18n.text(Message::UnrevealedNote)),
            Row::TradeCopyNote => note(ui, i18n.text(Message::TradeCopyNote)),
        }
    }
}

/// A note under the stats, in faint text, wrapped.
fn note(ui: &mut Ui, words: String) {
    ui.label(RichText::new(words).font(text::note()).color(color::FAINT));
}

/// A small title, with the fill of the prefixes and suffixes: lighter while one is open; and
/// "min – max" on its right, above the range fields of the filters.
fn group_title(ui: &mut Ui, group: Group, range: bool, i18n: &Translator) {
    let (message, fill) = match group {
        Group::Item => (Message::GroupItem, None),
        Group::Weapon => (Message::GroupWeapon, None),
        Group::Pseudo => (Message::GroupPseudo, None),
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
    if let StatText::Unrevealed(_) = stat.text {
        unrevealed_row(ui, &text);
        return;
    }
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
                None,
                (width - GUTTER - GAP - GAP - RANGE_COLUMN).max(0.0),
                bar,
                i18n,
            );
            return;
        }
        let Some(filter) = stat.filter else {
            stat_text(ui, &text, stat.roll.as_ref(), None, width, bar, i18n);
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
        let note = match stat.text {
            StatText::BaseType {
                class: Some(class), ..
            } if !choice.checked => Some(i18n.filter_item_class(class)),
            _ => None,
        };
        stat_text(
            ui,
            &text,
            stat.roll.as_ref(),
            note.as_deref(),
            (width - GUTTER - GAP - GAP - column).max(0.0),
            bar,
            i18n,
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

/// A modifier not revealed yet: its words in the desecrated green, in the face of the modifier
/// lines, from the left edge, on a faint band of that green, like the row of the game.
fn unrevealed_row(ui: &mut Ui, words: &str) {
    Frame::new()
        .fill(color::UNREVEALED_BAND)
        .corner_radius(size::RADIUS)
        .inner_margin(UNREVEALED_PADDING)
        .outer_margin(UNREVEALED_OUTSET)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new(words)
                    .font(text::body())
                    .color(color::DESECRATED),
            );
        });
}

/// The text of a stat and its roll, or a `note` in faint text under it, `width` across.
fn stat_text(
    ui: &mut Ui,
    text: &str,
    roll: Option<&Roll>,
    note: Option<&str>,
    width: f32,
    bar: RollBar,
    i18n: &Translator,
) {
    ui.allocate_ui_with_layout(vec2(width, 0.0), Layout::top_down(Align::Min), |ui| {
        ui.set_width(width);
        ui.add(Label::new(clamped(text)).wrap());
        if let Some(roll) = roll {
            ui.add_space(ROLL_GAP);
            roll_line(ui, roll, bar, i18n);
        }
        if let Some(note) = note {
            ui.add_space(ROLL_GAP);
            let galley = widgets::truncated_line(ui, note, text::roll(), width);
            let (rect, _) = ui.allocate_exact_size(galley.size(), Sense::hover());
            ui.painter().galley(rect.min, galley, color::FAINT);
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
        .outer_margin(RANGE_RAISE)
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
    // The field as it is drawn, without the margin that raises it.
    let rect = field.response.rect - RANGE_RAISE;
    if ui.rect_contains_pointer(rect) {
        field_tooltip(ui, key, rect, limit, i18n);
    }
}

/// A line of a tooltip: its text, its color, and whether it is small, like the ranges under
/// the stats.
#[derive(Debug, Clone, PartialEq)]
struct TooltipLine {
    text: String,
    tint: Color32,
    small: bool,
}

impl TooltipLine {
    fn new(text: String, tint: Color32) -> Self {
        Self {
            text,
            tint,
            small: false,
        }
    }
}

/// The lines of the tooltip of the range field of `limit`: the rolls it accepts, for a prefix
/// or suffix their tiers right under them, small and faint ("T13–T1"), then "Lower is better"
/// in gray for a line better lower.
fn tooltip_lines(limit: Limit, i18n: &Translator) -> Vec<TooltipLine> {
    let rolls = limit.rolls.map(|rolls| {
        TooltipLine::new(
            i18n.filter_rolls(&format::number(rolls.min), &format::number(rolls.max)),
            color::TEXT,
        )
    });
    let tiers = limit.rolls.and(limit.tiers).map(|tiers| TooltipLine {
        text: format!("T{}–T{}", tiers.lowest, tiers.highest),
        tint: color::FAINT,
        small: true,
    });
    let lower = (limit.half == Half::Maximum)
        .then(|| TooltipLine::new(i18n.text(Message::FilterLowerBetter), color::WEAK));
    [rolls, tiers, lower].into_iter().flatten().collect()
}

/// The color of a tier: tier 1 in a vivid orange, tiers 2 and 3 in white, the others faint.
fn tier_color(tier: u32) -> Color32 {
    match tier {
        1 => color::TIER_1,
        2 | 3 => color::ACCENT,
        _ => color::FAINT,
    }
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
        .map(|message| TooltipLine::new(i18n.text(message), color::TEXT));
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
fn tooltip(ui: &Ui, id: Id, at: Pos2, pivot: Align2, lines: Vec<TooltipLine>) {
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
                    for (index, line) in lines.into_iter().enumerate() {
                        if index > 0 {
                            let gap = if line.small {
                                TOOLTIP_SMALL_LINE_GAP
                            } else {
                                TOOLTIP_LINE_GAP
                            };
                            ui.add_space(gap);
                        }
                        let font = if line.small {
                            text::roll()
                        } else {
                            text::note()
                        };
                        let galley = widgets::text_line(ui, line.text, font);
                        let (rect, _) = ui.allocate_exact_size(galley.size(), Sense::hover());
                        ui.painter().galley(rect.min, galley, line.tint);
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

/// The bar of a roll drawn in the style `bar`, and its ranges; for a line of a prefix or
/// suffix, its bar over every tier, then under the last line of its modifier its tier, the
/// ranges of that tier and its origin; or the tiers of a line that can be a prefix or a
/// suffix, one under the other.
fn roll_line(ui: &mut Ui, roll: &Roll, bar: RollBar, i18n: &Translator) {
    match roll {
        Roll::Bar {
            share,
            ranges,
            origin,
        } => roll_row(ui, |ui| {
            roll_bar(ui, *share, bar);
            range_label(ui, ranges);
            if let Some(origin) = origin {
                origin_word(ui, *origin, i18n);
            }
        }),
        Roll::Affix {
            share,
            tier,
            origin,
        } => roll_row(ui, |ui| {
            if let Some(share) = share {
                roll_bar(ui, *share, bar);
            }
            if let Some(tier) = tier {
                tier_label(ui, tier);
            }
            if let Some(origin) = origin {
                origin_word(ui, *origin, i18n);
            }
        }),
        Roll::Tiers { rows, origin } => {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = TIER_ROW_GAP;
                for row in rows {
                    roll_row(ui, |ui| {
                        tier_label(ui, &row.tier);
                        small_word(ui, i18n.text(row.word), color::WEAK);
                    });
                }
                // On a line of its own: the line has it whichever affix it is.
                if let Some(origin) = origin {
                    roll_row(ui, |ui| origin_word(ui, *origin, i18n));
                }
            });
        }
    }
}

/// One row of a roll, its parts `GAP` apart.
fn roll_row(ui: &mut Ui, parts: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = GAP;
        parts(ui);
    });
}

/// A tier, `T1` in its color, and its ranges.
fn tier_label(ui: &mut Ui, tier: &ModifierTier) {
    ui.label(
        RichText::new(format!("T{}", tier.number))
            .font(text::tier())
            .color(tier_color(tier.number)),
    );
    if !tier.ranges.is_empty() {
        range_label(ui, &tier.ranges);
    }
}

/// The word of an origin, small, in its color: the last item of the line of a roll.
fn origin_word(ui: &mut Ui, origin: ModifierOrigin, i18n: &Translator) {
    let (message, tint) = origin_label(origin);
    small_word(ui, i18n.text(message), tint);
}

/// A small word after the ranges of a tier, in `tint`: its origin, or the affix of a tier among
/// several ("Prefix") in gray.
fn small_word(ui: &mut Ui, word: String, tint: Color32) {
    // The space before it only.
    ui.spacing_mut().item_spacing.x = ORIGIN_GAP;
    ui.label(RichText::new(word).font(text::origin()).color(tint));
}

/// The word of an origin, and its color.
fn origin_label(origin: ModifierOrigin) -> (Message, Color32) {
    match origin {
        ModifierOrigin::Desecrated => (Message::OriginDesecrated, color::DESECRATED),
        ModifierOrigin::Fractured => (Message::OriginFractured, color::FRACTURED),
        ModifierOrigin::Crafted => (Message::OriginCrafted, color::CRAFTED),
        ModifierOrigin::Vaal => (Message::OriginVaal, color::VAAL),
    }
}

/// The bar of a roll, filled to `share` in the style `bar`.
fn roll_bar(ui: &mut Ui, share: f32, bar: RollBar) {
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
    use crate::item::{CopiedItem, ItemState};

    fn parse(text: &str) -> Item {
        Item::parse(&CopiedItem::from_text(text).expect("an item")).expect("read")
    }

    /// The rows of `item`, its lines read in English.
    fn rows(item: &Item) -> Vec<Row<'_>> {
        let lines = CopiedLines::new(
            item.language(),
            item.modifiers(),
            &LineStats::default(),
            None,
        );
        let stats = LineStats::new(item.language(), item.modifiers(), None);
        let tiers = TierRolls::new(item.language(), item.modifiers(), base_type(item));
        super::rows(item, &lines, &stats, &tiers, base_type(item))
    }

    /// The base type the search of `item` looks for, among its filters: none for a unique.
    fn base_type(item: &Item) -> Option<&str> {
        (item.rarity() != Rarity::Unique).then(|| item.base_type())
    }

    /// The rows of `item` as texts, as [`row_texts`] writes them.
    fn texts(item: &Item) -> Vec<String> {
        row_texts(rows(item))
    }

    /// The rows as texts: a title (`Prefixes 2/3`), a note, or a stat,
    /// first `■ ` for a checked filter, `□ ` for an unchecked one or `⊘ ` for a line the trade
    /// site can't search, then its minimum or its maximum and its roll (`■ +55 to maximum Life
    /// ≥55 [40–60 0.75]`, `■ 18% reduced Movement Speed ≤18`, a line of a prefix or suffix
    /// with its bar over every tier and, under its last line, its tier, its ranges and its
    /// origin: `■ +86 to maximum Life ≥77 [0.37 T7 85–99]`, `[0.80 T2 71–79 Desecrated]`).
    fn row_texts<'a>(rows: impl IntoIterator<Item = Row<'a>>) -> Vec<String> {
        rows.into_iter()
            .map(|row| match row {
                Row::Group(Group::Prefixes { filled, max }) => format!("Prefixes {filled}/{max}"),
                Row::Group(Group::Suffixes { filled, max }) => format!("Suffixes {filled}/{max}"),
                Row::Group(group) => format!("{group:?}"),
                Row::UnrevealedNote => "Unrevealed note".to_owned(),
                Row::TradeCopyNote => "Trade copy note".to_owned(),
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
                        Some(Roll::Bar {
                            share,
                            ranges,
                            origin,
                        }) => {
                            let origin = origin
                                .map(|origin| format!(" {origin:?}"))
                                .unwrap_or_default();
                            format!(" [{} {share:.2}{origin}]", ranges_text(&ranges))
                        }
                        Some(Roll::Affix {
                            share,
                            tier,
                            origin,
                        }) => {
                            let share = share.map(|share| format!("{share:.2}"));
                            let tier = tier.as_ref().map(tier_text);
                            let origin = origin.map(|origin| format!("{origin:?}"));
                            let parts: Vec<String> =
                                [share, tier, origin].into_iter().flatten().collect();
                            format!(" [{}]", parts.join(" "))
                        }
                        Some(Roll::Tiers { rows, origin }) => {
                            // The origin on a line of its own, after the tiers.
                            let rows: Vec<String> = rows
                                .iter()
                                .map(|row| format!("{} {:?}", tier_text(&row.tier), row.word))
                                .chain(origin.map(|origin| format!("{origin:?}")))
                                .collect();
                            format!(" [{}]", rows.join(" / "))
                        }
                    };
                    let text = match text {
                        StatText::Line(text) => text.to_owned(),
                        StatText::BaseType { base_type, .. } => format!("Base: {base_type}"),
                        StatText::Property(property, value) => {
                            format!("{property:?}: {}", format::number(value))
                        }
                        StatText::Pseudo(pseudo, value) => {
                            format!("{pseudo:?}: {}", format::number(value))
                        }
                        StatText::Unrevealed(affix) => format!("Unrevealed {affix:?}"),
                    };
                    format!("{check}{text}{limit}{roll}")
                }
            })
            .collect()
    }

    /// A tier as [`row_texts`] writes it: `T1 15–18`.
    fn tier_text(tier: &ModifierTier) -> String {
        [format!("T{}", tier.number), ranges_text(&tier.ranges)]
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// `item`, copied from the in-game trade, read again with the tiers found from its values,
    /// as the session does.
    fn read_again(mut item: Item) -> Item {
        let stats = LineStats::new(item.language(), item.modifiers(), None);
        let modifiers = crate::game_data::with_trade_tiers(
            item.language(),
            item.base_type(),
            item.item_level(),
            item.modifiers(),
            stats.affix_limits(&item),
        );
        item.set_modifiers(modifiers);
        item
    }

    #[test]
    fn an_item_copied_from_the_in_game_trade_shows_its_tiers_found_and_a_note() {
        let tiara = read_again(parse(include_str!(
            "../item/test-items/en-owner-trade-rare-helmet-fractured.txt"
        )));

        let texts = texts(&tiara);
        let prefixes = texts
            .iter()
            .position(|text| text.starts_with("Prefixes"))
            .expect("its prefixes");
        assert_eq!(
            texts[prefixes..],
            [
                "Prefixes 3/3",
                // The fractured hybrid, its lines read together.
                "■ 41% increased Energy Shield ≥37 [0.97]",
                "□ +34 to maximum Mana ≥31 [0.85 T1 39–42 · 33–39 Fractured]",
                "■ 98% increased Energy Shield ≥88 [0.98 T1 92–100]",
                "■ +69 to maximum Energy Shield ≥62 [0.94 T1 61–73 Desecrated]",
                "Suffixes 3/3",
                // A prefix too, but the prefixes are full.
                "■ 18% increased Rarity of Items found ≥16 [1.00 T1 15–18]",
                "□ +43% to Fire Resistance ≥39 [0.95 T1 41–45]",
                "□ +40% to Cold Resistance ≥36 [0.87 T2 36–40]",
                "Trade copy note",
            ]
        );
    }

    #[test]
    fn a_line_of_the_in_game_trade_that_can_be_a_prefix_or_a_suffix_shows_the_tier_of_each() {
        let tiara = include_str!("../item/test-items/en-owner-trade-rare-helmet-fractured.txt");
        // Without the desecrated `+69 to maximum Energy Shield`, a prefix is open.
        let open = read_again(parse(
            &tiara.replace("+69 to maximum Energy Shield (desecrated)\n", ""),
        ));

        let texts = texts(&open);
        let explicit = texts
            .iter()
            .position(|text| text == "Explicit")
            .expect("its explicit modifiers");
        assert_eq!(
            texts[explicit..],
            [
                "Explicit",
                // Without bar, the tier and range of each affix one under the other.
                "■ 18% increased Rarity of Items found ≥16 [T1 16–19 AffixPrefix / T1 15–18 AffixSuffix]",
                "Trade copy note",
            ]
        );
    }

    #[test]
    fn the_origin_of_a_line_in_doubt_has_a_line_of_its_own_under_its_tiers() {
        let tiara = include_str!("../item/test-items/en-owner-trade-rare-helmet-fractured.txt");
        // A prefix open, and the rarity desecrated.
        let open = read_again(parse(
            &tiara
                .replace("+69 to maximum Energy Shield (desecrated)\n", "")
                .replace(
                    "18% increased Rarity of Items found\n",
                    "18% increased Rarity of Items found (desecrated)\n",
                ),
        ));

        assert!(texts(&open).contains(
            &"■ 18% increased Rarity of Items found ≥16 [T1 16–19 AffixPrefix / T1 15–18 AffixSuffix / Desecrated]"
                .to_owned()
        ));
    }

    #[test]
    fn a_line_that_can_be_a_prefix_is_one_once_the_suffixes_are_full() {
        let tiara = include_str!("../item/test-items/en-owner-trade-rare-helmet-fractured.txt");
        // A third resistance in place of the desecrated line: the suffixes full, a prefix open.
        let resistances = read_again(parse(&tiara.replace(
            "+69 to maximum Energy Shield (desecrated)",
            "+30% to Lightning Resistance",
        )));

        let texts = texts(&resistances);
        let rarity = texts
            .iter()
            .position(|text| text.contains("Rarity"))
            .expect("the rarity");
        let suffixes = texts
            .iter()
            .position(|text| text.starts_with("Suffixes"))
            .expect("its suffixes");
        assert!(rarity < suffixes, "a prefix: {texts:?}");
        assert!(texts[rarity].ends_with("T1 16–19]"), "{}", texts[rarity]);
    }

    #[test]
    fn the_note_shows_also_when_the_only_tiers_found_are_those_of_a_line_in_doubt() {
        // A helmet with one modifier, which a prefix and a suffix both roll.
        let helmet = read_again(parse(
            "Item Class: Helmets\nRarity: Rare\nChimeric Visage\nAncestral Tiara\n--------\n\
             Item Level: 82\n--------\n18% increased Rarity of Items found",
        ));

        let texts = texts(&helmet);
        assert_eq!(
            texts[texts.len() - 3..],
            [
                "Explicit",
                "■ 18% increased Rarity of Items found ≥16 [T1 16–19 AffixPrefix / T1 15–18 AffixSuffix]",
                "Trade copy note",
            ]
        );
    }

    #[test]
    fn only_the_tiers_found_from_the_values_have_the_note() {
        // Copied from the in-game trade, not read again yet: no tier found.
        let tiara = parse(include_str!(
            "../item/test-items/en-owner-trade-rare-helmet-fractured.txt"
        ));
        assert!(!texts(&tiara).contains(&"Trade copy note".to_owned()));
        for text in [
            include_str!("../item/test-items/en-rare-crossbow-all-modifiers.txt"),
            include_str!("../item/test-items/en-owner-trade-unique-belt-mutated.txt"),
            include_str!("../item/test-items/en-normal-helmet.txt"),
        ] {
            let item = parse(text);

            assert!(
                !texts(&item).contains(&"Trade copy note".to_owned()),
                "{text}"
            );
        }
    }

    #[test]
    fn a_line_of_a_unique_without_range_never_varies() {
        let headhunter = parse(
            "Item Class: Belts\nRarity: Unique\nHeadhunter\nHeavy Belt\n--------\n\
             Item Level: 80\n--------\n{ Implicit Modifier }\n\
             26(20-30)% increased Stun Threshold\n--------\n{ Unique Modifier }\n\
             When you kill a Rare monster, you gain its Modifiers for 60 seconds\n\
             { Unique Modifier }\n+55(40-60) to maximum Life",
        );

        assert_eq!(
            texts(&headhunter),
            [
                "Implicit",
                "■ 26% increased Stun Threshold ≥26 [20–30 0.60]",
                "Unique",
                // The stats that vary first.
                "■ +55 to maximum Life ≥55 [40–60 0.75]",
                // The string of its stat writes the 60: it never varies.
                "When you kill a Rare monster, you gain its Modifiers for 60 seconds",
            ]
        );
    }

    #[test]
    fn the_vaal_modifier_of_a_unique_says_vaal_after_its_range() {
        let belt = parse(include_str!("../item/test-items/fr-unique-belt-vaal.txt"));

        let texts = texts(&belt);
        let vaal: Vec<&String> = texts.iter().filter(|text| text.contains("Vaal")).collect();
        // A filter like the other modifiers of the unique, its origin after its range.
        assert_eq!(
            vaal,
            ["■ +112 au Score d'Évasion tant que votre Vie est Basse ≥112 [100–150 0.24 Vaal]"]
        );
    }

    #[test]
    fn the_vaal_modifier_of_a_unique_says_vaal_under_a_fixed_line() {
        let unique = parse(
            "Item Class: Rings\nRarity: Unique\nTest Loop\nGold Ring\n--------\n\
             Item Level: 80\n--------\n{ Vaal Unique Modifier }\n\
             +5% to Maximum Lightning Resistance\n--------\nCorrupted",
        );

        assert_eq!(
            texts(&unique),
            ["Unique", "+5% to Maximum Lightning Resistance [Vaal]"]
        );
    }

    #[test]
    fn the_line_of_a_unique_without_header_never_varies() {
        let ring = parse(include_str!(
            "../item/test-items/fr-unique-ring-mirrored.txt"
        ));

        // Like a stat that never varies: no checkbox, from the left edge.
        assert_eq!(texts(&ring), ["Unique", "Reflète la Bague opposée"]);
    }

    #[test]
    fn a_unique_copied_without_any_range_has_only_fixed_lines() {
        let unique = parse(
            "Item Class: Rings\nRarity: Unique\nTest Loop\nGold Ring\n--------\n\
             Item Level: 80\n--------\n{ Unique Modifier }\n\
             +5% to Maximum Lightning Resistance\n{ Unique Modifier }\n+52 to maximum Life",
        );

        // The advanced copy writes a range for every number that rolls.
        assert_eq!(
            texts(&unique),
            [
                "Unique",
                "+5% to Maximum Lightning Resistance",
                "+52 to maximum Life"
            ]
        );
    }

    #[test]
    fn a_number_written_by_the_string_of_a_stat_has_no_range() {
        let unique = parse(
            "Item Class: Gloves\nRarity: Unique\nTest Grip\nSuede Bracers\n--------\n\
             Item Level: 80\n--------\n{ Unique Modifier }\n\
             1% more Unarmed Damage per 5(4-6) Strength",
        );

        // The 1 is written by the string of the stat: the field holds the 5, in its range.
        assert_eq!(
            texts(&unique),
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
             Item Level: 80\n--------\n{ Implicit Modifier }\n\
             +10(7-13)% to Chaos Resistance\n--------\n{ Unique Modifier }\n\
             +20(17-23)% to Chaos Resistance\n{ Unique Modifier }\n\
             Adds 1 to 55(50-60) Lightning damage to Attacks",
        );

        assert_eq!(
            texts(&ring),
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
            texts(&focus),
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
            texts(&ring),
            [
                "Unique",
                "■ -15% to Fire Resistance ≥-15 [10–20 0.50]",
                // A range across zero needs its sign: no bar.
                "2% to Cold Resistance",
            ]
        );
    }

    #[test]
    fn a_rare_shows_its_enchantment_runes_implicit_prefixes_and_suffixes() {
        let crossbow = parse(include_str!(
            "../item/test-items/en-rare-crossbow-all-modifiers.txt"
        ));

        assert_eq!(
            texts(&crossbow),
            [
                // The base type checked, the item level unchecked at the level of the item.
                "Item",
                "■ Base: Gemini Crossbow",
                "□ ItemLevel: 80 ≥80",
                // Corrupted: no socket can be added any more.
                "■ Sockets: 2 ≥2",
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
                // the attack speed of the weapon unchecked, since its DPS counts them. The bar
                // over every tier of the crossbow: of two numbers, the average of where each
                // falls (10 in 1–19, 273 in 7–358). Its origin after the range of its tier.
                "□ Adds 10 to 273 Lightning Damage ≥127 [0.63 T2 1–16 · 239–300 Fractured]",
                "□ Adds 24 to 51 Physical Damage ≥34 [0.47 T3 23–35 · 39–59]",
                // Not a damage of the weapon: its attacks get it from any item.
                "■ 109% increased Elemental Damage with Attacks ≥98 [0.71 T2 100–119]",
                "Suffixes 3/3",
                // Copied before a tier was added above it: tier 1 then, 7.59 in 5–8.9 now.
                "■ Leeches 7.59% of Physical Damage as Mana ≥6.83 [0.66 T1 7–7.9]",
                "■ Gain 21 Mana per enemy killed ≥19 [0.44 T3 21–27 Desecrated]",
                "□ 18% increased Attack Speed ≥16 [0.93 T1 17–19]",
            ]
        );
    }

    #[test]
    fn the_origin_of_a_modifier_follows_the_range_of_its_tier_under_its_last_line() {
        let armour = parse(include_str!("../item/test-items/fr-rare-body-armour.txt"));
        let origins: Vec<String> = texts(&armour)
            .into_iter()
            .filter(|text| text.ends_with("Crafted]") || text.ends_with("Desecrated]"))
            .collect();
        assert_eq!(origins.len(), 2, "{origins:?}");
        assert!(origins[0].contains("76% d'Augmentation du Bouclier d'énergie"));
        assert!(origins[1].contains("+14% aux Résistances au feu et au chaos"));

        let ring = parse(
            "Item Class: Rings\nRarity: Rare\nRune Loop\nGold Ring\n--------\n\
             Item Level: 79\n--------\n\
             { Fractured Prefix Modifier \"Hale\" (Tier: 9) — Life, Mana }\n\
             +12(10-14) to maximum Life\n+10(9-13) to maximum Mana",
        );
        let lines: Vec<String> = texts(&ring)
            .into_iter()
            .filter(|text| text.contains("to maximum"))
            .collect();
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert!(!lines[0].contains("Fractured"), "{lines:?}");
        assert!(lines[1].ends_with("Fractured]"), "{lines:?}");
    }

    #[test]
    fn a_modifier_whose_header_is_unknown_is_explicit_without_tier() {
        let ring = parse(
            "Item Class: Rings\nRarity: Rare\nRune Loop\nPrismatic Ring\n--------\n\
             Item Level: 79\n--------\n{ Implicit Modifier }\n\
             +8(7-10)% to all Elemental Resistances\n--------\n{ Strange Modifier }\n\
             +143(120-150) to Evasion Rating\n\
             { Suffix Modifier \"of the Brute\" (Tier: 7) — Attribute }\n+12(9-12) to Strength\n\
             --------\nCorrupted",
        );

        assert_eq!(
            texts(&ring),
            [
                "Item",
                "■ Base: Prismatic Ring",
                "□ ItemLevel: 79 ≥79",
                // The totals in place of the lines they add up: each element of the
                // resistances counts.
                "Pseudo",
                "■ ElementalResistance: 24 ≥22",
                "■ Strength: 12 ≥11",
                "Implicit",
                "□ +8% to all Elemental Resistances ≥7 [7–10 0.33]",
                "Suffixes 1/3",
                "□ +12 to Strength ≥11 [0.25 T7 9–12]",
                // Neither a prefix nor a suffix: no tier, like a modifier the tables don't
                // know.
                "Explicit",
                "■ +143 to Evasion Rating ≥129",
            ]
        );
    }

    #[test]
    fn the_affixes_a_rune_or_an_implicit_allows_count_in_the_fill() {
        let english = parse(
            "Item Class: Amulets\nRarity: Rare\nDoom Idol\nGloam Amulet\n--------\n\
             Item Level: 79\n--------\n+1 Suffix Modifier allowed (rune)\n--------\n\
             { Implicit Modifier }\n-1 Prefix Modifier allowed\n--------\n\
             { Prefix Modifier \"Hale\" (Tier: 9) }\n+12(10-14) to maximum Life\n\
             { Suffix Modifier \"of the Brute\" (Tier: 7) — Attribute }\n+12(9-12) to Strength",
        );
        let french = parse(
            "Classe d'objet: Amulettes\nRareté: Rare\nIdole funeste\nAmulette crépusculaire\n\
             --------\nNiveau de l'objet: 79\n--------\n\
             +1 Modificateur suffixe autorisé (rune)\n--------\n\
             { Modificateur Implicite }\n-1 Modificateur préfixe autorisé\n--------\n\
             { Préfixe : « Sain » (Palier : 9) }\n+12(10-14) de Vie maximale\n\
             { Suffixe : « de la Brute » (Palier : 7) — Attribut }\n+12(9-12) de Force",
        );

        let groups = |item: &Item| -> Vec<String> {
            texts(item)
                .into_iter()
                .filter(|text| text.starts_with("Prefixes") || text.starts_with("Suffixes"))
                .collect()
        };
        assert_eq!(groups(&english), ["Prefixes 1/2", "Suffixes 1/4"]);
        assert_eq!(groups(&french), ["Prefixes 1/2", "Suffixes 1/4"]);
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
            texts(&boots),
            [
                "Item",
                "■ Base: Lattice Sandals",
                "Pseudo",
                "■ Life: 86 ≥77",
                "■ Dexterity: 12 ≥11",
                "Implicit",
                // 90 % of 12, within the range of the implicit.
                "□ +12 to Dexterity ≥11 [10–15 0.40]",
                // A fixed line keeps its checkbox, without a minimum.
                "■ 5% increased Movement Speed",
                "Prefixes 2/3",
                // Its bar over every tier of the life of boots, from 10 (T9) to 149 (T1).
                "□ +86 to maximum Life ≥77 [0.55 T3 85–99]",
                // A hybrid modifier: its tier and the ranges of its lines under the last one,
                // each line with its checkbox and its minimum; without bar, since no modifier
                // of the tables rolls these ranges.
                "■ +20 to Evasion Rating ≥18",
                "■ +10% increased Evasion Rating ≥9 [T5 15–25 · 8–12]",
            ]
        );
    }

    #[test]
    fn a_prefix_or_suffix_rolls_over_every_tier_of_its_modifier_on_the_base() {
        let mail = parse(
            "Item Class: Body Armours\nRarity: Rare\nDoom Shell\nAncestral Mail\n--------\n\
             Item Level: 82\n--------\n\
             { Prefix Modifier \"Vigorous\" (Tier: 7) — Life }\n+86(85-99) to maximum Life\n\
             { Prefix Modifier \"Lacquered\" (Tier: 4) — Defences }\n\
             +98(86-102) to Armour\n+88(79-94) to Evasion Rating\n\
             { Suffix Modifier \"of the Penguin\" (Tier: 1) — Elemental, Cold, Resistance }\n\
             +41(41-45)% to Cold Resistance",
        );
        let rows = rows(&mail);
        let texts = row_texts(rows.iter().cloned());
        let prefixes = texts
            .iter()
            .position(|text| text == "Prefixes 2/3")
            .expect("its prefixes");

        assert_eq!(
            texts[prefixes..],
            [
                "Prefixes 2/3",
                // 76 of the 204 between 10 (T13) and 214 (T1); a pseudo total adds it up.
                "□ +86 to maximum Life ≥77 [0.37 T7 85–99]",
                // Each line of a hybrid with its bar over its own tiers.
                "■ +98 to Armour ≥88 [0.55]",
                "■ +88 to Evasion Rating ≥79 [0.53 T4 86–102 · 79–94]",
                "Suffixes 1/3",
                "□ +41% to Cold Resistance ≥37 [0.90 T1 41–45]",
            ]
        );
        let limit_of = |text: &str| {
            let row = rows
                .iter()
                .find(|row| matches!(row, Row::Stat(Stat { text: StatText::Line(line), .. }) if *line == text))
                .expect("its row");
            limit(row)
        };
        // The field stays within the rolls of every tier, which its tooltip tells.
        let life = Range {
            min: 10.0,
            max: 214.0,
        };
        assert_eq!(
            limit_of("+86 to maximum Life"),
            Limit {
                value: 77.0,
                half: Half::Minimum,
                bounds: Some(life),
                rolls: Some(life),
                tiers: Some(TierSpan {
                    lowest: 13,
                    highest: 1,
                }),
            }
        );
        assert_eq!(
            limit_of("+88 to Evasion Rating").tiers,
            Some(TierSpan {
                lowest: 8,
                highest: 1,
            })
        );
    }

    #[test]
    fn a_line_fixed_in_its_tier_varies_over_the_tiers() {
        let bow = parse(
            "Item Class: Bows\nRarity: Rare\nStorm Bite\nGemini Bow\n--------\n\
             Item Level: 81\n--------\n\
             { Suffix Modifier \"of the Sniper\" (Tier: 2) — Gem }\n\
             +3 to Level of all Projectile Skills",
        );

        // A bar and a range field: the other tiers roll 1, 2 and 4.
        assert_eq!(
            texts(&bow).last().map(String::as_str),
            Some("■ +3 to Level of all Projectile Skills ≥3 [0.67 T2]")
        );
    }

    #[test]
    fn the_pseudo_totals_are_checked_and_the_lines_they_add_up_unchecked() {
        let ring = parse(
            "Item Class: Rings\nRarity: Rare\nStorm Spiral\nSapphire Ring\n--------\n\
             Item Level: 77\n--------\n{ Implicit Modifier }\n\
             +22(20-30)% to Cold Resistance\n--------\n\
             { Prefix Modifier \"Rotund\" (Tier: 4) — Life }\n+65(60-69) to maximum Life\n\
             { Prefix Modifier \"Agile\" (Tier: 6) — Defences }\n+60(52-79) to Evasion Rating\n\
             { Suffix Modifier \"of the Furnace\" (Tier: 2) — Elemental, Fire, Resistance }\n\
             +38(36-40)% to Fire Resistance\n\
             { Suffix Modifier \"of the Squall\" (Tier: 5) — Elemental, Lightning, Resistance }\n\
             +23(21-25)% to Lightning Resistance\n\
             { Suffix Modifier \"of the Lynx\" (Tier: 7) — Attribute }\n+12(9-12) to Dexterity",
        );

        assert_eq!(
            texts(&ring),
            [
                "Item",
                "■ Base: Sapphire Ring",
                "□ ItemLevel: 77 ≥77",
                "Pseudo",
                "■ ElementalResistance: 83 ≥75",
                "■ Life: 65 ≥59",
                "■ Dexterity: 12 ≥11",
                "Implicit",
                "□ +22% to Cold Resistance ≥20 [20–30 0.20]",
                "Prefixes 2/3",
                "□ +65 to maximum Life ≥59 [0.50 T4 60–69]",
                // Not added up by a total.
                "■ +60 to Evasion Rating ≥54 [0.23 T6 52–79]",
                "Suffixes 3/3",
                "□ +38% to Fire Resistance ≥34 [0.82 T2 36–40]",
                "□ +23% to Lightning Resistance ≥21 [0.44 T5 21–25]",
                "□ +12 to Dexterity ≥11 [0.25 T7 9–12]",
            ]
        );
    }

    #[test]
    fn a_unique_has_no_pseudo_totals() {
        let unique = parse(
            "Item Class: Rings\nRarity: Unique\nTest Loop\nGold Ring\n--------\n\
             Item Level: 80\n--------\n{ Unique Modifier }\n+52(40-60) to maximum Life\n\
             { Unique Modifier }\n+30(25-35)% to Fire Resistance",
        );

        assert_eq!(
            texts(&unique),
            [
                "Unique",
                "■ +52 to maximum Life ≥52 [40–60 0.60]",
                "■ +30% to Fire Resistance ≥30 [25–35 0.50]",
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
            texts(&english),
            [
                "Item",
                "■ Base: Ruby",
                "Pseudo",
                "■ Life: 12 ≥11",
                "Prefixes 1/2",
                "□ +12 to maximum Life ≥11 [T2 10–14]"
            ]
        );
        assert_eq!(
            texts(&french),
            [
                "Item",
                "■ Base: Rubis",
                "Pseudo",
                "■ Life: 12 ≥11",
                "Prefixes 1/2",
                "□ +12 de Vie maximale ≥11 [T2 10–14]"
            ]
        );
    }

    #[test]
    fn a_weapon_mostly_elemental_is_searched_by_its_elemental_dps_too() {
        let weapon = |physical: &str, lightning: &str| {
            parse(&format!(
                "Item Class: Two Hand Maces\nRarity: Rare\nStorm Fall\nTemple Maul\n--------\n\
                 Physical Damage: {physical}\nLightning Damage: {lightning} (lightning)\n\
                 Attacks per Second: 1.00\n--------\nItem Level: 80"
            ))
        };
        let dps = |item: &Item| texts(item)[4..6].to_vec();

        assert_eq!(
            dps(&weapon("10-20", "1-59")),
            ["■ PhysicalDps: 15 ≥13", "■ ElementalDps: 30 ≥27"]
        );
        // 90 % of a small DPS, of 2.5 and not of the 3 shown, rounded down: the weapon
        // passes its own filter.
        assert_eq!(
            dps(&weapon("10-20", "1-4")),
            ["■ PhysicalDps: 15 ≥13", "□ ElementalDps: 3 ≥2"]
        );
    }

    #[test]
    fn an_unrevealed_modifier_is_a_line_of_its_group_and_unchecks_the_other_stats() {
        let english = parse(include_str!(
            "../item/test-items/en-owner-rare-focus-unrevealed.txt"
        ));
        let french = parse(include_str!(
            "../item/test-items/fr-rare-focus-unrevealed.txt"
        ));
        assert_eq!(
            texts(&english),
            [
                "Item",
                "■ Base: Plumed Focus",
                "□ ItemLevel: 39 ≥39",
                "□ RequiredLevel: 36 ≤36",
                // Searched with its number as minimum.
                "■ UnrevealedModifiers: 1 ≥1",
                "Pseudo",
                "□ ElementalResistance: 42 ≥38",
                // Counted in the fill, last of its group, without filter.
                "Prefixes 3/3",
                "□ 52% increased Spell Damage ≥47 [0.42 T4 45–54]",
                "□ +36 to maximum Energy Shield ≥32 [0.32 T6 36–41]",
                "Unrevealed Prefix",
                "Suffixes 2/3",
                "□ +24% to Cold Resistance ≥22 [0.46 T5 21–25]",
                "□ +18% to Fire Resistance ≥16 [0.31 T6 16–20]",
                "Unrevealed note",
            ]
        );
        let french = texts(&french);
        assert_eq!(french[4], "■ UnrevealedModifiers: 1 ≥1");
        assert_eq!(french[10], "Unrevealed Prefix");
        assert_eq!(french.last().map(String::as_str), Some("Unrevealed note"));
    }

    #[test]
    fn a_corrupted_item_counts_each_unrevealed_modifier() {
        let ring = parse(
            "Item Class: Rings\nRarity: Rare\nWoe Loop\nRuby Ring\n--------\n\
             Item Level: 80\n--------\n\
             { Prefix Modifier \"Veiled\" }\nDesecrated Prefix\n\
             { Suffix Modifier \"of the Veil\" }\nDesecrated Suffix\n--------\nCorrupted",
        );
        assert_eq!(
            texts(&ring),
            [
                "Item",
                "■ Base: Ruby Ring",
                "□ ItemLevel: 80 ≥80",
                "■ UnrevealedModifiers: 2 ≥2",
                "Prefixes 1/3",
                "Unrevealed Prefix",
                "Suffixes 1/3",
                "Unrevealed Suffix",
                "Unrevealed note",
            ]
        );
    }

    #[test]
    fn a_weapon_with_an_unrevealed_modifier_starts_with_its_dps_unchecked() {
        let weapon = parse(
            "Item Class: Two Hand Maces\nRarity: Rare\nStorm Fall\nTemple Maul\n--------\n\
             Physical Damage: 10-20\nLightning Damage: 1-59 (lightning)\n\
             Attacks per Second: 1.00\n--------\nItem Level: 80\n--------\n\
             { Prefix Modifier \"Veiled\" }\nDesecrated Prefix",
        );
        assert_eq!(
            texts(&weapon),
            [
                "Item",
                "■ Base: Temple Maul",
                "□ ItemLevel: 80 ≥80",
                "■ UnrevealedModifiers: 1 ≥1",
                "Weapon",
                // An unrevealed prefix can add damage.
                "□ PhysicalDps: 15 ≥13",
                "□ ElementalDps: 30 ≥27",
                "Prefixes 1/3",
                "Unrevealed Prefix",
                "Unrevealed note",
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
            texts(&quiver),
            [
                "Item",
                "■ Base: Quick Broadhead Quiver",
                "Suffixes 1/1",
                "■ 8% increased Attack Speed ≥7 [T4 7–9]",
            ]
        );
    }

    /// The rows of the Item group of `item`, without its title.
    fn item_group(item: &Item) -> Vec<String> {
        item_group_of(item, base_type(item))
    }

    /// The rows of the Item group of `item` searched by `base_type`, without its title: the
    /// base type of a magic item is found among the names of the trade site.
    fn item_group_of<'a>(item: &'a Item, base_type: Option<&'a str>) -> Vec<String> {
        let lines = CopiedLines::default();
        let stats = LineStats::new(item.language(), item.modifiers(), None);
        let rows = super::rows(item, &lines, &stats, &TierRolls::default(), base_type);
        row_texts(
            rows.into_iter()
                .skip_while(|row| *row != Row::Group(Group::Item))
                .skip(1)
                .take_while(|row| matches!(row, Row::Stat(_))),
        )
    }

    #[test]
    fn a_normal_item_is_searched_by_its_base_sockets_item_level_and_quality() {
        let shield = parse(include_str!(
            "../item/test-items/fr-normal-exceptional-shield.txt"
        ));
        let helmet = parse(include_str!("../item/test-items/en-normal-helmet.txt"));
        let targe = parse(
            "Item Class: Shields\nRarity: Normal\nExceptional Mammoth Targe\n--------\n\
             Quality: +24% (augmented)\nArmour: 139 (augmented)\n--------\nItem Level: 82",
        );

        // An exceptional base by its sockets: the trade site doesn't know its name.
        assert_eq!(
            item_group(&shield),
            [
                "■ Base: Targe de mammouth",
                "■ ItemLevel: 82 ≥82",
                "■ Sockets: 2 ≥2",
            ]
        );
        assert_eq!(
            item_group(&helmet),
            [
                "■ Base: Divine Crown",
                "■ ItemLevel: 81 ≥81",
                "□ Quality: 9 ≥9",
            ]
        );
        // An exceptional base by its quality.
        assert_eq!(
            item_group(&targe),
            [
                "■ Base: Mammoth Targe",
                "■ ItemLevel: 82 ≥82",
                "■ Quality: 24 ≥24",
            ]
        );
    }

    #[test]
    fn the_base_type_tells_the_item_class_searched_when_it_is_unchecked() {
        // The class under the base type `base_type` of `item`, when it is unchecked.
        let class_of = |item: &Item, base_type: &str| {
            let lines = CopiedLines::default();
            let stats = LineStats::new(item.language(), item.modifiers(), None);
            let tiers = TierRolls::default();
            super::rows(item, &lines, &stats, &tiers, Some(base_type))
                .into_iter()
                .find_map(|row| match row {
                    Row::Stat(Stat {
                        text: StatText::BaseType { class, .. },
                        ..
                    }) => Some(class.map(str::to_owned)),
                    _ => None,
                })
        };

        // The item class of the copied text, in its language.
        let armour = parse(include_str!("../item/test-items/fr-rare-body-armour.txt"));
        assert_eq!(
            class_of(&armour, armour.base_type()),
            Some(Some("Armures".to_owned()))
        );
        // A magic item, by the base type its name holds.
        let mace = parse(include_str!("../item/test-items/en-magic-mace.txt"));
        assert_eq!(
            class_of(&mace, "Temple Maul"),
            Some(Some("Two Hand Maces".to_owned()))
        );
        // A base the game data doesn't know is searched among any base instead.
        let ring = parse("Item Class: Rings\nRarity: Rare\nWoe Loop\nMystery Ring");
        assert_eq!(class_of(&ring, ring.base_type()), Some(None));
    }

    /// The filter of `property` among the filters of `item`.
    fn property_filter_of(item: &Item, property: Property) -> StatFilter {
        rows(item)
            .into_iter()
            .find_map(|row| match row {
                Row::Stat(Stat {
                    key: FilterKey::Property(key),
                    filter: Some(filter),
                    ..
                }) if key == property => Some(filter),
                _ => None,
            })
            .expect("the filter of the property")
    }

    /// The value the player types in `half` of the field of `property`, as it is when they
    /// leave it.
    fn settled(item: &Item, property: Property, half: Half, typed: &str) -> String {
        let mut choice = property_filter_of(item, property).default_choice();
        *choice.text_mut(half) = typed.to_owned();
        choice.settle(half);
        choice.text(half).to_owned()
    }

    #[test]
    fn the_fields_of_the_properties_stay_within_the_values_of_the_game() {
        let crossbow = parse(include_str!(
            "../item/test-items/en-rare-crossbow-all-modifiers.txt"
        ));
        let leveling = parse(
            "Item Class: Body Armours\nRarity: Rare\nHate Shell\nWaxed Jacket\n--------\n\
             Quality: +23% (augmented)\nEvasion Rating: 120\n--------\n\
             Requires: Level 40, 30 Dex\n--------\nItem Level: 45",
        );

        assert_eq!(
            settled(&crossbow, Property::ItemLevel, Half::Minimum, "5000"),
            "100"
        );
        assert_eq!(
            settled(&crossbow, Property::ItemLevel, Half::Minimum, "0"),
            "1"
        );
        assert_eq!(
            settled(&crossbow, Property::Sockets, Half::Minimum, "9"),
            "6"
        );
        assert_eq!(
            settled(&leveling, Property::Quality, Half::Minimum, "80"),
            "75"
        );
        assert_eq!(
            settled(&leveling, Property::RequiredLevel, Half::Maximum, "95"),
            "90"
        );
    }

    #[test]
    fn a_property_out_of_the_values_of_the_game_widens_its_field() {
        let item = parse(
            "Item Class: Rings\nRarity: Rare\nStorm Loop\nGold Ring\n--------\n\
             Item Level: 101",
        );

        // Its own value stays; a higher one is brought back to it.
        assert_eq!(
            settled(&item, Property::ItemLevel, Half::Minimum, "101"),
            "101"
        );
        assert_eq!(
            settled(&item, Property::ItemLevel, Half::Minimum, "5000"),
            "101"
        );
    }

    #[test]
    fn the_sockets_are_checked_above_those_of_the_base_or_on_a_corrupted_item() {
        let armour = |sockets: &str, corrupted: &str| {
            parse(&format!(
                "Item Class: Body Armours\nRarity: Rare\nHate Shell\nWaxed Jacket\n--------\n\
                 Sockets: {sockets}\n--------\nItem Level: 80\n--------\n\
                 {{ Prefix Modifier }}\n+80 to maximum Life{corrupted}"
            ))
        };
        let sockets = |item: &Item| {
            item_group(item)
                .into_iter()
                .find(|text| text.contains("Sockets"))
        };

        // A body armour usually has two.
        assert_eq!(
            sockets(&armour("S S", "")).as_deref(),
            Some("□ Sockets: 2 ≥2")
        );
        assert_eq!(
            sockets(&armour("S S S", "")).as_deref(),
            Some("■ Sockets: 3 ≥3")
        );
        assert_eq!(
            sockets(&armour("S", "\n--------\nCorrupted")).as_deref(),
            Some("■ Sockets: 1 ≥1")
        );
        let none = parse(
            "Item Class: Body Armours\nRarity: Rare\nHate Shell\nWaxed Jacket\n--------\n\
             Item Level: 80\n--------\n{ Prefix Modifier }\n+80 to maximum Life",
        );
        assert_eq!(sockets(&none), None);
    }

    #[test]
    fn a_sanctified_item_keeps_its_values_out_of_their_tier() {
        for text in [
            include_str!("../item/test-items/en-owner-rare-sceptre-sanctified.txt"),
            include_str!("../item/test-items/fr-rare-sceptre-sanctified.txt"),
        ] {
            let rows = texts(&parse(text));
            let line = |end: &str| {
                rows.iter()
                    .find(|row| row.ends_with(end))
                    .unwrap_or_else(|| panic!("{end}: {rows:#?}"))
                    .clone()
            };

            // `+74(80-89) to maximum Mana`: its tier, its value under its range.
            assert!(line(" ≥67 [0.42 T5 80–89]").starts_with("□ +74 "));
            // `29(30)% reduced Attribute Requirements`, a range of one value; unchecked, as
            // every stat of an item with an unrevealed modifier.
            assert!(line(" ≥26 [0.70 T2 30]").starts_with("□ 29% "));
        }
    }

    #[test]
    fn a_sanctified_value_above_every_tier_keeps_its_field_and_a_full_bar() {
        let text = include_str!("../item/test-items/en-owner-rare-sceptre-sanctified.txt")
            .replace("+74(80-89)", "+250(80-89)");
        let item = parse(&text);

        assert!(
            texts(&item).contains(&"□ +250 to maximum Mana ≥225 [1.00 T5 80–89]".to_owned()),
            "{:#?}",
            texts(&item)
        );
        let limit = rows(&item)
            .into_iter()
            .find_map(|row| match row {
                Row::Stat(Stat {
                    text: StatText::Line("+250 to maximum Mana"),
                    filter,
                    ..
                }) => filter?.limit,
                _ => None,
            })
            .expect("its limit");
        // The field holds the value, above the 164 of tier 1; the tooltip tells the rolls.
        assert_eq!(
            limit.bounds,
            Some(Range {
                min: 10.0,
                max: 250.0
            })
        );
        assert_eq!(
            limit.rolls,
            Some(Range {
                min: 10.0,
                max: 164.0
            })
        );
    }

    #[test]
    fn an_exceptional_quality_is_checked_except_on_a_rare() {
        let bow = parse(include_str!("../item/test-items/en-rare-bow-fractured.txt"));
        let magic = parse(
            "Item Class: Gloves\nRarity: Magic\nVaal Mitts of the Fox\n--------\n\
             Quality: +23% (augmented)\n--------\nItem Level: 70\n--------\n\
             { Suffix Modifier }\n+20 to Dexterity",
        );
        let twenty = parse(
            "Item Class: Gloves\nRarity: Magic\nVaal Mitts of the Fox\n--------\n\
             Quality: +20% (augmented)\n--------\nItem Level: 70\n--------\n\
             { Suffix Modifier }\n+20 to Dexterity",
        );

        // Its bow usually has two sockets; its required level is above 75.
        assert_eq!(
            item_group(&bow),
            [
                "■ Base: Obliterator Bow",
                "□ ItemLevel: 81 ≥81",
                "□ Sockets: 2 ≥2",
                "□ Quality: 25 ≥25",
            ]
        );
        assert!(item_group(&magic).contains(&"■ Quality: 23 ≥23".to_owned()));
        assert!(
            !item_group(&twenty)
                .iter()
                .any(|text| text.contains("Quality"))
        );
    }

    #[test]
    fn a_rare_for_leveling_has_its_required_level_as_a_maximum() {
        let ring = |required: u32, item_level: u32| {
            parse(&format!(
                "Item Class: Rings\nRarity: Rare\nHate Loop\nRuby Ring\n--------\n\
                 Requires: Level {required}\n--------\nItem Level: {item_level}\n--------\n\
                 {{ Prefix Modifier }}\n+30 to maximum Life"
            ))
        };
        let required = |item: &Item| {
            item_group(item)
                .into_iter()
                .find(|text| text.contains("RequiredLevel"))
        };

        assert_eq!(
            required(&ring(40, 45)).as_deref(),
            Some("□ RequiredLevel: 40 ≤40")
        );
        assert_eq!(
            required(&ring(75, 75)).as_deref(),
            Some("□ RequiredLevel: 75 ≤75")
        );
        assert_eq!(required(&ring(40, 80)), None);
        assert_eq!(required(&ring(78, 80)), None);
        // A magic item, even for leveling.
        let magic = parse(
            "Item Class: Rings\nRarity: Magic\nVital Ruby Ring\n--------\n\
             Requires: Level 40\n--------\nItem Level: 45\n--------\n\
             { Prefix Modifier }\n+30 to maximum Life",
        );
        assert_eq!(required(&magic), None);
    }

    #[test]
    fn the_sockets_of_a_base_the_data_doesnt_know_are_checked_only_on_a_corrupted_item() {
        let armour = |corrupted: &str| {
            parse(&format!(
                "Item Class: Body Armours\nRarity: Rare\nHate Shell\nPatch Jacket\n--------\n\
                 Sockets: S S S\n--------\nItem Level: 80\n--------\n\
                 {{ Prefix Modifier }}\n+80 to maximum Life{corrupted}"
            ))
        };
        let sockets = |item: &Item| {
            item_group(item)
                .into_iter()
                .find(|text| text.contains("Sockets"))
        };

        assert_eq!(sockets(&armour("")).as_deref(), Some("□ Sockets: 3 ≥3"));
        assert_eq!(
            sockets(&armour("\n--------\nCorrupted")).as_deref(),
            Some("■ Sockets: 3 ≥3")
        );
    }

    #[test]
    fn the_quality_of_a_charm_counts_from_10_and_of_a_flask_from_20() {
        let quality = |class: &str, name: &str, base: &str, quality: u32| {
            let item = parse(&format!(
                "Item Class: {class}\nRarity: Magic\n{name}\n--------\n\
                 Quality: +{quality}% (augmented)\n--------\nItem Level: 60\n--------\n\
                 {{ Suffix Modifier }}\n+10% to Fire Resistance"
            ));
            item_group_of(&item, Some(base))
                .into_iter()
                .find(|text| text.contains("Quality"))
        };
        let charm = |value| quality("Charms", "Ruby Charm of the Fox", "Ruby Charm", value);
        let flask = |value| {
            quality(
                "Life Flasks",
                "Ultimate Life Flask of the Fox",
                "Ultimate Life Flask",
                value,
            )
        };

        assert_eq!(charm(5).as_deref(), Some("□ Quality: 5 ≥5"));
        assert_eq!(charm(10).as_deref(), Some("■ Quality: 10 ≥10"));
        assert_eq!(flask(15), None);
        assert_eq!(flask(20).as_deref(), Some("□ Quality: 20 ≥20"));
        assert_eq!(flask(23).as_deref(), Some("■ Quality: 23 ≥23"));
    }

    #[test]
    fn a_magic_item_has_one_prefix_and_one_suffix_at_most() {
        let mace = parse(include_str!("../item/test-items/en-magic-mace.txt"));

        assert_eq!(
            texts(&mace),
            [
                // Its whole name here: the application finds its base type among the names of
                // the trade site.
                "Item",
                "■ Base: Crackling Temple Maul of the Brute",
                "□ ItemLevel: 32 ≥32",
                "Weapon",
                "■ PhysicalDps: 64 ≥57",
                "□ ElementalDps: 31 ≥27",
                "Pseudo",
                "■ Strength: 8 ≥7",
                "Prefixes 1/1",
                "□ Adds 1 to 50 Lightning Damage ≥23 [T7 1–4 · 46–66]",
                "Suffixes 1/1",
                "□ +8 to Strength ≥7 [T8 5–8]",
            ]
        );
    }

    #[test]
    fn a_limit_stays_within_the_range_of_its_stat() {
        let ring = parse(
            "Item Class: Rings\nRarity: Unique\nVenom Loop\nAmethyst Ring\n--------\n\
             Item Level: 80\n--------\n{ Unique Modifier }\n\
             Adds 1 to 55(50-60) Lightning damage to Attacks",
        );
        let boots = parse(
            "Item Class: Boots\nRarity: Rare\nGale Tread\nLattice Sandals\n--------\n\
             { Implicit Modifier }\n+10(10-15) to Dexterity",
        );
        // The fixed number counts in the range of the average; the tooltip doesn't tell it,
        // since it isn't a roll.
        assert_eq!(
            limit(&rows(&ring)[1]),
            Limit {
                value: 28.0,
                half: Half::Minimum,
                bounds: Some(Range {
                    min: 25.5,
                    max: 30.5
                }),
                rolls: None,
                tiers: None,
            }
        );
        // 90 % of 10 is under the lowest roll of the implicit.
        let range = Some(Range {
            min: 10.0,
            max: 15.0,
        });
        assert_eq!(
            limit(rows(&boots).last().expect("its implicit")),
            Limit {
                value: 10.0,
                half: Half::Minimum,
                bounds: range,
                rolls: range,
                tiers: None,
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
            texts(&unique),
            [
                "Unique",
                "■ +52 to maximum Life ≥52 [40–60 0.60]",
                // Its bar still shows where its value falls in its range.
                "■ 18% reduced Movement Speed ≤18 [15–25 0.30]",
            ]
        );
        // 110 % of the copied value, within the range of the implicit.
        assert_eq!(
            texts(&rare),
            [
                "Item",
                "■ Base: Ruby Ring",
                "Implicit",
                "■ 12% increased Charm Charges used ≤13 [10–15 0.40]",
                "Suffixes 1/3",
                "■ 18% reduced Movement Speed ≤20 [T2 15–25]",
            ]
        );
        let lower = limit(&rows(&unique)[2]);
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
        let whole = |value, half| comparable(value, true, half);

        assert!((whole(12.0, Half::Minimum) - 11.0).abs() < 1e-9);
        assert!((whole(18.0, Half::Maximum) - 20.0).abs() < 1e-9);
        // The item passes its own filter.
        assert!((whole(-15.0, Half::Minimum) - -17.0).abs() < 1e-9);
        assert!((whole(-15.0, Half::Maximum) - -14.0).abs() < 1e-9);
        assert!((comparable(-7.5, false, Half::Minimum) - -8.25).abs() < 1e-9);
    }

    #[test]
    fn the_tooltip_of_a_range_field_tells_its_rolls_and_its_better_side() {
        let i18n = Translator::new(crate::language::Language::English);
        let range = Some(Range {
            min: 15.0,
            max: 25.0,
        });
        let tooltip = |half, rolls, tiers| {
            let limit = Limit {
                value: 18.0,
                half,
                bounds: range,
                rolls,
                tiers,
            };
            tooltip_lines(limit, &i18n)
        };
        let rolls = TooltipLine::new("Rolls 15 to 25".to_owned(), color::TEXT);
        let lower = TooltipLine::new("Lower is better".to_owned(), color::WEAK);

        assert_eq!(
            tooltip(Half::Minimum, range, None),
            std::slice::from_ref(&rolls)
        );
        assert_eq!(
            tooltip(Half::Maximum, range, None),
            [rolls.clone(), lower.clone()]
        );
        // A line of several numbers.
        assert_eq!(
            tooltip(Half::Maximum, None, None),
            std::slice::from_ref(&lower)
        );
        assert_eq!(tooltip(Half::Minimum, None, None), []);
        // The required level of a rare for leveling, searched with a maximum: lower is better.
        assert_eq!(
            tooltip_lines(Limit::maximum(40.0), &i18n),
            std::slice::from_ref(&lower)
        );
        // A prefix or suffix: the tiers of its rolls right under them, small and faint.
        let tiers = TooltipLine {
            text: "T5–T1".to_owned(),
            tint: color::FAINT,
            small: true,
        };
        assert_eq!(
            tooltip(
                Half::Maximum,
                range,
                Some(TierSpan {
                    lowest: 5,
                    highest: 1,
                })
            ),
            [rolls, tiers, lower]
        );
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
        // The game writes a negative range without its sign.
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
    fn a_corrupted_item_has_no_note_and_a_lit_state_tag_counts_among_the_active_filters() {
        let headhunter = parse(
            "Item Class: Belts\nRarity: Unique\nHeadhunter\nHeavy Belt\n--------\n\
             Item Level: 80\n--------\n{ Unique Modifier }\n+55(40-60) to maximum Life\n\
             --------\nCorrupted",
        );
        let staff = parse(
            "Item Class: Staves\nRarity: Normal\nChiming Staff\n--------\n\
             Item Level: 82\n--------\nCorrupted",
        );
        let stats = |item| {
            Stats::new(
                item,
                &CopiedLines::default(),
                &LineStats::default(),
                &TierRolls::default(),
                base_type(item),
            )
        };

        // The Corrupted tag says what is searched.
        assert_eq!(
            texts(&headhunter),
            ["Unique", "■ +55 to maximum Life ≥55 [40–60 0.75]"]
        );
        let headhunter_stats = Stats {
            item: &headhunter,
            rows: rows(&headhunter),
        };
        let mut filters = Filters::default();
        assert_eq!(headhunter_stats.active_filters(&filters), 1);
        filters.toggle_state(ItemState::Corrupted);
        assert_eq!(headhunter_stats.active_filters(&filters), 2);
        // A state the item doesn't have isn't counted.
        filters.toggle_state(ItemState::Mirrored);
        assert_eq!(headhunter_stats.active_filters(&filters), 2);
        // An item without modifiers has its filters to show.
        assert!(!stats(&staff).is_empty());
        assert_eq!(
            texts(&staff),
            ["Item", "■ Base: Chiming Staff", "■ ItemLevel: 82 ≥82"]
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
        let stats = Stats::new(&unique, &lines, &line_stats, &TierRolls::default(), None);

        assert_eq!(
            texts(&unique),
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
        let tiers = TierRolls::new(
            crossbow.language(),
            crossbow.modifiers(),
            base_type(&crossbow),
        );
        let stats = Stats::new(&crossbow, &lines, &line_stats, &tiers, base_type(&crossbow));
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

        // The base type, the sockets, the physical DPS, the implicit and three modifiers; the
        // item level, the elemental DPS, the enchantment, the two runes, the damage and the
        // attack speed unchecked.
        assert_eq!(stats.active_filters(&filters), 7);
        // A rune checked by the player.
        filters.set(keys[6], choice(true));
        assert_eq!(stats.active_filters(&filters), 8);
        // The implicit unchecked.
        filters.set(keys[8], choice(false));
        assert_eq!(stats.active_filters(&filters), 7);
        filters.clear();
        assert_eq!(stats.active_filters(&filters), 7);
    }

    #[test]
    fn a_legacy_unique_keeps_its_value_out_of_the_range_of_today() {
        let belt = parse(
            "Item Class: Belts\nRarity: Unique\nHeadhunter\nHeavy Belt\n--------\n\
             Item Level: 80\n--------\n{ Unique Modifier }\n+65(40-60) to maximum Life",
        );
        let rows = rows(&belt);

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
                        tiers: None,
                    }),
                }),
                unsearchable: false,
                roll: Some(Roll::Bar {
                    share: 1.0,
                    ranges: vec![Range {
                        min: 40.0,
                        max: 60.0
                    }],
                    origin: None,
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
