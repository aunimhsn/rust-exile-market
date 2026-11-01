//! The stats of the item, unfolded under the item block: for a rare, magic or normal item,
//! first the filters of its base type, item level, sockets, quality, required level and DPS,
//! then of its pseudo totals for a rare or magic one, then its modifiers in the color of the
//! game, grouped, the stats that vary first. A unique, or an implicit modifier of a rare, shows
//! a bar of where its value falls in its range; a line of a prefix or a suffix, a bar over
//! every tier of its modifier on the base of the item, from the modifier tables, and its range
//! field stays within them; under the last line of its modifier, its tier and the range of that
//! tier.
//!
//! A waystone has the filters of its properties in a group of their own, the item rarity and
//! the drop chance checked, its modifiers unchecked: a buyer pays for its bonuses, not for its
//! monsters; the filter of its base shows a bar of its tier, and the tier, in its color.
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
//! and its other stats start unchecked, with a note under them. A line of a Time-Lost jewel is
//! written without the words of its radius, a tag of the passive skills they name after its
//! range.
//!
//! The rows are decided apart from the drawing ([`rows`]), and tested.

use std::borrow::Cow;

use egui::text::{LayoutJob, TextFormat};
use egui::{
    Align, Align2, Area, Color32, CornerRadius, Frame, Id, Label, Layout, Margin, Order, Pos2,
    Rect, Response, RichText, Sense, Stroke, TextEdit, Ui, Vec2, ViewportId, pos2, vec2,
};

use super::item_block::waystone_tier_color;
use super::number_field::NumberField;
use super::theme::{self, color, size, text};
use super::widgets;
use crate::filters::{self, Filter, FilterKey, Filters, Half, Property, StatKey, Window};
use crate::format;
use crate::game_data::{self, Category};
use crate::i18n::{Message, Translator};
use crate::item::{
    self, Affix, AffixTier, Currency, Item, Lord, MAX_GEM_LEVEL, MAX_GEM_QUALITY, MAX_GEM_SOCKETS,
    MAX_WAYSTONE_TIER, Modifier, ModifierKind, ModifierOrigin, PartTier, QualityKind, Range,
    Rarity, StatLine, TierDoubt, WaystoneProperty,
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
/// The range field raised by 3 px on the line of its checkbox: taller than the checkbox, it
/// looked too low (tuned by eye by the owner on the mock-up: 1 px in step 13h, 3 px in step
/// 13k4a, a whole pixel so that every row is the same). The row keeps its height.
const RANGE_RAISE: Margin = Margin {
    left: 0,
    right: 0,
    top: -3,
    bottom: 3,
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
/// The widest tooltip of a word after the ranges of a tier, wrapped past it: the tooltip of
/// "Hybrid" names a line of the item, which can be long.
const WORD_TIP_WIDTH: f32 = 230.0;
/// The lines of the text of a stat, at most; the whole text is in its tooltip.
const MAX_LINES: usize = 4;
/// The bar of a roll.
const BAR_SIZE: Vec2 = vec2(96.0, 4.0);
/// The space above the roll of a stat.
const ROLL_GAP: f32 = 3.0;
/// The space between the range of a tier and the origin of its modifier.
const ORIGIN_GAP: f32 = 6.0;
/// The space around the tag of the radius of a line of a Time-Lost jewel, inside its block.
const RADIUS_TAG_PADDING: Vec2 = vec2(5.0, 0.0);
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
/// The highest quality of an item whose base the game data doesn't know: the highest of any
/// item, the 75 % of a Refined Breach Ring (checked on the trade site on 2026-10-31). The field
/// of the quality stays within the highest quality of the base ([`Category::max_quality`]), or
/// within this one.
const MAX_QUALITY: u32 = 75;
/// The quality from which the quality of a gem is checked, like Exiled Exchange 2, in percent.
const GEM_QUALITY: u32 = 16;
/// The sockets from which the sockets of a gem are checked: every gem has 2.
const GEM_SOCKETS: u32 = 3;
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
    /// The properties of a waystone.
    Waystone,
    /// The level, the quality and the sockets of a skill gem.
    Gem,
    /// The totals the trade site adds up from several lines.
    Pseudo,
    /// The enchantments, and the modifiers added by the corruption.
    Enchant,
    Runes,
    Implicit,
    /// Their count and the most the item can have; none for a corrupted waystone, its count
    /// alone ([`game_data::AffixLimits::counted_only`]).
    Prefixes {
        filled: u32,
        max: Option<u32>,
    },
    Suffixes {
        filled: u32,
        max: Option<u32>,
    },
    /// The explicit modifiers of a magic or rare item whose header doesn't tell their affix:
    /// a header the parser doesn't know, or lines before the first header.
    Explicit,
    /// The modifiers of a unique, no prefix nor suffix.
    Unique,
}

/// The word after the roll of a modifier: its origin, as the game writes it, or the currency
/// that adds it, which the modifier tables tell. Written as the word alone in a debug output
/// (`Desecrated`, `Essence`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Origin {
    Written(ModifierOrigin),
    Currency(Currency),
}

impl Origin {
    /// The word after a prefix or suffix of `origin` that `currency` adds: the currency, an
    /// essence (which the game writes as an ordinary modifier) or an alloy (crafted), so that a
    /// line without tier says why; its origin otherwise, also for a modifier of a currency
    /// fractured or desecrated.
    fn of_affix(origin: Option<ModifierOrigin>, currency: Option<Currency>) -> Option<Self> {
        match (origin, currency) {
            (None | Some(ModifierOrigin::Crafted), Some(currency)) => {
                Some(Self::Currency(currency))
            }
            (origin, _) => origin.map(Self::Written),
        }
    }

    /// The word after the prefix or suffix `modifier` that `currency` adds ([`Self::of_affix`]);
    /// for a line that adds up two modifiers, the currency of a part follows the tier of that
    /// part, in place of "Crafted" under the line.
    fn of_modifier(modifier: &Modifier, currency: Option<Currency>) -> Option<Self> {
        match &modifier.doubt {
            Some(TierDoubt::Split { whole, added, .. })
                if whole.currency.or(added.currency).is_some() =>
            {
                let crafted = Some(ModifierOrigin::Crafted);
                modifier
                    .origin
                    .filter(|&origin| Some(origin) != crafted)
                    .map(Self::Written)
            }
            _ => Self::of_affix(modifier.origin, currency),
        }
    }
}

impl std::fmt::Debug for Origin {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Written(origin) => origin.fmt(formatter),
            Self::Currency(currency) => currency.fmt(formatter),
        }
    }
}

/// The roll of a stat, the lines its tooltips name borrowed from the item.
#[derive(Debug, Clone, PartialEq)]
enum Roll<'a> {
    /// Where its value falls in its ranges, from 0 to 1, and the ranges; then, under the last
    /// line of a Vaal modifier of a unique, its origin.
    Bar {
        share: f32,
        ranges: Vec<Range>,
        origin: Option<Origin>,
    },
    /// A line of a prefix or a suffix: where its value falls over every tier of its modifier
    /// on the base, from 0 to 1, when the modifier tables know them; under the last line of the
    /// modifier, its tier and the ranges of that tier, of all its lines for a hybrid, so that it
    /// reads as one modifier, then its origin when it is desecrated, fractured or crafted, or the
    /// currency that adds it (an essence, an alloy). Also
    /// the origin alone under a line of a unique that never varies (a fixed Vaal modifier),
    /// without bar nor tier. Under the other lines of a hybrid whose part a line adds up, the
    /// tier of the hybrid, "Hybrid" after its ranges. Boxed, so that a row stays small.
    Affix {
        share: Option<f32>,
        tier: Option<Box<TierRow<'a>>>,
        origin: Option<Origin>,
    },
    /// Under the last line of a modifier copied from the in-game trade that can be a prefix or
    /// a suffix: the tier of each and its ranges, one under the other, a word after the ranges
    /// ("Prefix"), without bar; then its origin on a line of its own, since the line has it
    /// whichever affix it is. The same under a line that adds up two modifiers, for the tier
    /// of each.
    Tiers {
        rows: Vec<TierRow<'a>>,
        origin: Option<Origin>,
    },
    /// Under the filter of the base of a waystone: a bar of its tier over every tier, then the
    /// tier, both in the color of the tier, which tells how dangerous the map is.
    WaystoneTier(u32),
}

/// The tier of a prefix or suffix, written by the advanced copy or found from the values, and
/// the ranges of that tier.
#[derive(Debug, Clone, PartialEq)]
struct ModifierTier {
    /// 1 is the best; none for a modifier of one tier only on its base, whose T1 would read as
    /// "the best" while it is "the only one".
    number: Option<u32>,
    ranges: Vec<Range>,
}

/// A tier under a line, or two neighbouring ones, and the word after its ranges.
#[derive(Debug, Clone, PartialEq)]
struct TierRow<'a> {
    /// The tier, or the worse of two neighbouring tiers.
    tier: ModifierTier,
    /// The better of two neighbouring tiers the modifier may be: `T1/T2`.
    better: Option<u32>,
    /// In gray after the ranges: the affix of the tier ("Prefix"), or "Hybrid".
    word: Option<Message>,
    /// The lord of the Abyss named before the affix: "Amanamu Prefix".
    lord: Option<Lord>,
    /// Whether its tier, an affix of a line in doubt without range (a fixed value), is written
    /// in the style of its word: "T2 Suffix".
    quiet: bool,
    /// The tooltip of that word.
    tip: Option<WordTip<'a>>,
    /// The currency that adds the part of a line that adds up two modifiers, an essence or an
    /// alloy, named last, as after the ranges of any modifier it adds.
    currency: Option<Currency>,
}

/// The tooltip of the word after the ranges of a tier.
#[derive(Debug, Clone, PartialEq)]
enum WordTip<'a> {
    /// A sentence: "Added up with the prefix on this line".
    Text(Message),
    /// On the line that adds up the part of a hybrid, its other lines: "With +195 to Accuracy
    /// Rating".
    HybridLines(Vec<&'a str>),
    /// Under the other lines of that hybrid, the ranges of its part and the line that adds it
    /// up: "With 75–79 of 258% increased Physical Damage".
    HybridPart { ranges: Vec<Range>, line: &'a str },
}

impl<'a> TierRow<'a> {
    /// Whether its tier, without range, is written as its word: "T2 Suffix"; only under a line
    /// in doubt, the other rows keeping the color of their tier ("T1 Hybrid suffix" under the
    /// other line of a hybrid split from a summed line, whose T1 the summed line writes too).
    fn quiet_tier(&self) -> bool {
        // Set by `TierRow::affix` only: a word, no ranges, one tier.
        self.quiet
    }

    /// The tier `tier` of a modifier, without word.
    fn plain(tier: ModifierTier) -> Self {
        Self {
            tier,
            better: None,
            word: None,
            lord: None,
            quiet: false,
            tip: None,
            currency: None,
        }
    }

    /// The tier `found` for one affix of a line that can be either, "Prefix" after its ranges;
    /// without number for a modifier of one tier only, or whose tier the game doesn't write.
    fn affix(found: &AffixTier) -> Self {
        Self {
            tier: ModifierTier {
                number: (!found.hides_number).then_some(found.tier),
                ranges: found.ranges.clone(),
            },
            better: None,
            word: Some(affix_word(found.affix)),
            lord: found.lord,
            quiet: found.ranges.is_empty(),
            tip: None,
            currency: None,
        }
    }

    /// The tiers of a part of a line that adds up two modifiers, over the range of both when
    /// they are two, with `word` and its `tip`.
    fn part(part: &PartTier, word: Option<Message>, tip: Option<WordTip<'a>>) -> Option<Self> {
        Self::tiers(part, part.ranges.clone(), word, tip)
    }

    /// The tiers `part` may be, the best first (one, or two neighbouring ones), over `ranges`,
    /// with `word` and its `tip`; without number for a modifier of one tier only, or whose tier
    /// the game doesn't write.
    fn tiers(
        part: &PartTier,
        ranges: Vec<Range>,
        word: Option<Message>,
        tip: Option<WordTip<'a>>,
    ) -> Option<Self> {
        let worse = *part.tiers.last()?;
        Some(Self {
            tier: ModifierTier {
                number: (!part.hides_number).then_some(worse),
                ranges,
            },
            better: part
                .tiers
                .first()
                .copied()
                .filter(|&better| better != worse),
            word,
            lord: None,
            quiet: false,
            tip,
            currency: part.currency,
        })
    }
}

/// The words after the ranges of the tier of `row`, in `i18n`: its affix, after the lord whose
/// modifier it is ("Amanamu Prefix"); after its tier written as they are ("T2 Suffix") under a
/// line in doubt without range (a fixed value), so that it reads as one with them (owner's choice
/// on 2026-10-30).
fn row_word(row: &TierRow, i18n: &Translator) -> Option<String> {
    let word = row.word?;
    let text = match (row.lord, word) {
        (Some(lord), Message::AffixPrefix) => i18n.lord_affix(Affix::Prefix, lord),
        (Some(lord), Message::AffixSuffix) => i18n.lord_affix(Affix::Suffix, lord),
        _ => i18n.text(word),
    };
    Some(match row.tier.number.filter(|_| row.quiet_tier()) {
        Some(number) => format!("T{number} {text}"),
        None => text,
    })
}

/// The word of an affix after the ranges of its tier.
fn affix_word(affix: Affix) -> Message {
    match affix {
        Affix::Prefix => Message::AffixPrefix,
        Affix::Suffix => Message::AffixSuffix,
    }
}

/// The tiers of the parts `whole` and `added` of a line that adds up two modifiers, one under
/// the other: the prefix first, then, of two parts of one affix, the pure one (`added`) before
/// the hybrid; "Prefix" and "Suffix" after the ranges of a prefix and a suffix, the tooltip of
/// "Suffix" saying they are added up, which a line that can be either doesn't have. A hybrid
/// `whole`, of the `others` lines, says "Hybrid" instead ([`hybrid_word`]), its tooltip naming
/// them.
fn split_rows<'a>(whole: &PartTier, added: &PartTier, others: &[&'a str]) -> Vec<TierRow<'a>> {
    let mut parts = [(added, false), (whole, true)];
    parts.sort_by_key(|&(part, is_whole)| (part.affix == Affix::Suffix, is_whole));
    let both = whole.affix != added.affix;
    parts
        .iter()
        .filter_map(|&(part, is_whole)| {
            let (word, tip) = if is_whole && !others.is_empty() {
                (
                    Some(hybrid_word(whole, added)),
                    Some(WordTip::HybridLines(others.to_vec())),
                )
            } else {
                (
                    both.then(|| affix_word(part.affix)),
                    (both && part.affix == Affix::Suffix)
                        .then_some(WordTip::Text(Message::AddedUpWithPrefix)),
                )
            };
            TierRow::part(part, word, tip)
        })
        .collect()
}

/// The word after the ranges of the hybrid `whole` whose part a line adds up with `added`,
/// on that line and under its other lines: "Hybrid", with its affix when `added` is of the
/// other affix ("Hybrid prefix"), the line then in Explicit.
fn hybrid_word(whole: &PartTier, added: &PartTier) -> Message {
    match whole.affix {
        _ if whole.affix == added.affix => Message::Hybrid,
        Affix::Prefix => Message::HybridPrefix,
        Affix::Suffix => Message::HybridSuffix,
    }
}

/// The lines of `modifier` other than its line `summed`, with their indexes: the other lines
/// of the hybrid whose part that line adds up.
fn other_lines(
    modifier: &Modifier,
    summed: usize,
) -> impl DoubleEndedIterator<Item = (usize, &StatLine)> {
    modifier
        .lines
        .iter()
        .enumerate()
        .filter(move |&(index, _)| index != summed)
}

/// The tier of the hybrid `whole` of `modifier`, whose line `summed` adds up its part with
/// `added`, under the last of its other lines: the ranges of these lines over its tiers,
/// "Hybrid" after them ([`hybrid_word`]), its tooltip naming that part and the line.
fn hybrid_row<'a>(
    modifier: &'a Modifier,
    summed: usize,
    whole: &PartTier,
    added: &PartTier,
) -> Option<TierRow<'a>> {
    // As under any modifier, none for fixed values.
    let ranges = Range::unless_fixed(
        other_lines(modifier, summed).flat_map(|(_, line)| magnitudes(line, &copied_ranges(line))),
    );
    let tip = WordTip::HybridPart {
        ranges: whole.ranges.clone(),
        line: &modifier.lines.get(summed)?.text,
    };
    TierRow::tiers(whole, ranges, Some(hybrid_word(whole, added)), Some(tip))
}

/// How the bar of a roll is drawn: a setting from step 19, which adds the zone searched.
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

/// The passive skills in the radius of a Time-Lost jewel that a line of it is granted to; not
/// the radius of the jewel itself (`Radius: Small`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RadiusSkills {
    Small,
    Notable,
}

impl RadiusSkills {
    /// Its tag, after the range of the line: "Small", "Notable".
    fn message(self) -> Message {
        match self {
            Self::Small => Message::RadiusSmall,
            Self::Notable => Message::RadiusNotable,
        }
    }
}

/// The words a Time-Lost jewel writes before each of its lines, in each language of the game,
/// and the passive skills they name.
const RADIUS_WORDS: [(&str, RadiusSkills); 4] = [
    (
        "Small Passive Skills in Radius also grant ",
        RadiusSkills::Small,
    ),
    (
        "Notable Passive Skills in Radius also grant ",
        RadiusSkills::Notable,
    ),
    (
        "Les Talents Mineurs dans le Rayon octroient également ",
        RadiusSkills::Small,
    ),
    (
        "Les Talents éminents dans le Rayon octroient également ",
        RadiusSkills::Notable,
    ),
];

/// The text of a stat.
#[derive(Debug, Clone, Copy, PartialEq)]
enum StatText<'a> {
    /// A line of a modifier.
    Line(&'a str),
    /// A stat the game writes on several lines, its lines joined by a space, so that nothing
    /// comes between them.
    Lines(&'a [StatLine]),
    /// A line of a Time-Lost jewel, without the words of its radius (`line`), and the whole
    /// line, which names it to a screen reader.
    RadiusLine {
        skills: RadiusSkills,
        line: &'a str,
        whole: &'a str,
    },
    /// The base type, with the item class searched in its place when it is unchecked; none
    /// for a base the game data doesn't know, searched among any base instead.
    BaseType {
        base_type: &'a str,
        class: Option<&'a str>,
    },
    /// A property of the item, with its value.
    Property(Property, f64),
    /// The quality of a catalyst, with its kind.
    CatalystQuality(&'a QualityKind, f64),
    Pseudo(Pseudo, f64),
    /// A desecrated modifier not revealed yet, of its affix.
    Unrevealed(Affix),
}

/// The value of `property` in its filter: a percentage of a waystone with its sign, as the
/// game writes it (`+11`), a zero without.
pub(super) fn property_value(property: Property, value: f64) -> String {
    match property {
        Property::Waystone(waystone) if waystone.is_percent() && value > 0.0 => {
            format!("+{}", format::number(value))
        }
        _ => format::number(value),
    }
}

impl<'a> StatText<'a> {
    /// A line of a modifier, without the words of its radius for a line of a Time-Lost jewel.
    fn line(text: &'a str) -> Self {
        RADIUS_WORDS
            .iter()
            .find_map(|&(words, skills)| {
                let line = text.strip_prefix(words)?;
                Some(Self::RadiusLine {
                    skills,
                    line,
                    whole: text,
                })
            })
            .unwrap_or(Self::Line(text))
    }

    /// The passive skills a line of a Time-Lost jewel is granted to.
    fn radius_skills(self) -> Option<RadiusSkills> {
        match self {
            Self::RadiusLine { skills, .. } => Some(skills),
            _ => None,
        }
    }

    /// The text that names the stat to a screen reader: the whole line of a Time-Lost jewel,
    /// with the words of its radius shown as a tag.
    fn label(self, i18n: &Translator) -> Cow<'a, str> {
        match self {
            Self::RadiusLine { whole, .. } => Cow::Borrowed(whole),
            _ => self.text(i18n),
        }
    }

    /// The tooltip of the text: the quality of a catalyst, whose kinds the trade site doesn't
    /// tell apart.
    fn tip(self) -> Option<Message> {
        matches!(self, Self::CatalystQuality(..)).then_some(Message::FilterAnyQualityKind)
    }

    fn text(self, i18n: &Translator) -> Cow<'a, str> {
        match self {
            Self::Line(text) | Self::RadiusLine { line: text, .. } => Cow::Borrowed(text),
            Self::Lines(lines) => Cow::Owned(
                lines
                    .iter()
                    .map(|line| line.text.as_str())
                    .collect::<Vec<_>>()
                    .join(" "),
            ),
            Self::BaseType { base_type, .. } => Cow::Owned(i18n.filter_base_type(base_type)),
            Self::Property(property, value) => {
                Cow::Owned(i18n.filter_property(property, &property_value(property, value)))
            }
            Self::CatalystQuality(kind, value) => {
                Cow::Owned(i18n.filter_catalyst_quality(kind, &format::number(value)))
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
    roll: Option<Roll<'a>>,
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
    /// Under the stats of a unique copied from the in-game trade, whose rolls come from the
    /// price database.
    UniqueRollsNote,
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
        push_group(&mut rows, Group::Waystone, waystone_filters(item));
        dps_filters(item, !unrevealed)
    } else {
        push_group(&mut rows, Group::Gem, gem_filters(item));
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
        unchecked: unrevealed || item.is_waystone(),
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
    let mut rows = join_stat_lines(item, stats, rows);
    if unrevealed {
        rows.push(Row::UnrevealedNote);
    }
    // Once a tier is found, also for a line that can be a prefix or a suffix, or that adds up
    // two modifiers, or one of the Genesis tree, read with the ranges of its tier but without
    // its number, which the game doesn't write: none before the names of the trade site tell
    // the base of a magic item, nor on a base the game data doesn't know.
    let found = item.modifiers().iter().any(|modifier| {
        let ranged = modifier
            .lines
            .iter()
            .flat_map(|line| &line.rolls)
            .any(|roll| roll.range.is_some());
        let read = matches!(modifier.kind, ModifierKind::Explicit(Some(_)))
            && (modifier.tier.is_some() || ranged);
        let in_doubt = modifier.doubt.is_some();
        read || in_doubt
    });
    if item.has_tiers_from_values() && found {
        rows.push(Row::TradeCopyNote);
    }
    // Once a range comes from the prices: none before they arrive.
    let with_range = || {
        item.modifiers()
            .iter()
            .filter(|modifier| {
                matches!(
                    modifier.kind,
                    ModifierKind::Implicit | ModifierKind::Explicit(_)
                )
            })
            .flat_map(|modifier| &modifier.lines)
            .flat_map(|line| &line.rolls)
            .any(|roll| roll.range.is_some())
    };
    if item.has_rolls_from_prices() && with_range() {
        rows.push(Row::UniqueRollsNote);
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

/// The rows of a stat the game writes on several lines joined into one, after
/// [`searchable`]: its text the lines joined, with the filter of its searched line, and under
/// it the roll of its lines, so that its bar never comes between them (`Lorsque vous collectez
/// une Infusion élémentaire, vous avez 23% de chances de gagner une` then `Infusion
/// élémentaire supplémentaire du même type` of Kurgal). Rows that can't be joined stay apart:
/// a stat with a bar on two of its lines (`Alternating every 5 seconds:`), or a line of a
/// Time-Lost jewel.
fn join_stat_lines<'a>(item: &'a Item, stats: &LineStats, rows: Vec<Row<'a>>) -> Vec<Row<'a>> {
    // The line of the modifier that searches the stat of `key`, which its rows share.
    let anchor = |key: StatKey| stats.searched_line(key);
    let mut joined = Vec::with_capacity(rows.len());
    let mut group: Vec<(StatKey, Stat<'a>)> = Vec::new();
    for row in rows {
        match row {
            Row::Stat(
                stat @ Stat {
                    key: FilterKey::Stat(key),
                    ..
                },
            ) => {
                let follows = group.last().is_some_and(|(last, _)| {
                    last.modifier == key.modifier
                        && last.line + 1 == key.line
                        && anchor(*last) == anchor(key)
                });
                if !follows {
                    flush_stat_lines(item, &mut group, &mut joined, anchor);
                }
                group.push((key, stat));
            }
            row => {
                flush_stat_lines(item, &mut group, &mut joined, anchor);
                joined.push(row);
            }
        }
    }
    flush_stat_lines(item, &mut group, &mut joined, anchor);
    joined
}

/// Pushes the rows of `group`, lines of one stat that follow each other, to `rows`: one row
/// when they can be joined, otherwise each as it is.
fn flush_stat_lines<'a>(
    item: &'a Item,
    group: &mut Vec<(StatKey, Stat<'a>)>,
    rows: &mut Vec<Row<'a>>,
    anchor: impl Fn(StatKey) -> StatKey,
) {
    let stats = std::mem::take(group);
    match joined_stat(item, &stats, anchor) {
        Some(stat) => rows.push(Row::Stat(stat)),
        None => rows.extend(stats.into_iter().map(|(_, stat)| Row::Stat(stat))),
    }
}

/// The row of the lines of one stat, `stats`, in their order; none for a single line, or for
/// lines that can't be joined.
fn joined_stat<'a>(
    item: &'a Item,
    stats: &[(StatKey, Stat<'a>)],
    anchor: impl Fn(StatKey) -> StatKey,
) -> Option<Stat<'a>> {
    let [(first, _), .., (last, _)] = stats else {
        return None;
    };
    if !stats
        .iter()
        .all(|(_, stat)| matches!(stat.text, StatText::Line(_)))
    {
        return None;
    }
    let lines = item
        .modifiers()
        .get(first.modifier)?
        .lines
        .get(first.line..=last.line)?;
    let searched = anchor(*first);
    let (_, searched_stat) = stats.iter().find(|(key, _)| *key == searched)?;
    let mut rolls = stats.iter().filter_map(|(_, stat)| stat.roll.clone());
    let roll = match rolls.next() {
        Some(first) => Some(rolls.try_fold(first, joined_roll)?),
        None => None,
    };
    Some(Stat {
        key: FilterKey::Stat(searched),
        text: StatText::Lines(lines),
        filter: searched_stat.filter,
        unsearchable: searched_stat.unsearchable,
        roll,
    })
}

/// The roll under the joined lines of a stat, from the roll of a line, `roll`, and of the
/// next one: the bar of the line with a number, and the tier and the origin written under its
/// last line; none when both can't be told under one text (two bars, two tiers).
fn joined_roll<'a>(roll: Roll<'a>, next: Roll<'a>) -> Option<Roll<'a>> {
    match (roll, next) {
        (
            Roll::Affix {
                share,
                tier,
                origin,
            },
            Roll::Affix {
                share: next_share,
                tier: next_tier,
                origin: next_origin,
            },
        ) if (share.is_none() || next_share.is_none())
            && (tier.is_none() || next_tier.is_none()) =>
        {
            Some(Roll::Affix {
                share: share.or(next_share),
                tier: tier.or(next_tier),
                origin: next_origin.or(origin),
            })
        }
        // The bar of a unique and the origin under its last line.
        (
            Roll::Bar {
                share,
                ranges,
                origin: bar_origin,
            },
            Roll::Affix {
                share: None,
                tier: None,
                origin,
            },
        )
        | (
            Roll::Affix {
                share: None,
                tier: None,
                origin,
            },
            Roll::Bar {
                share,
                ranges,
                origin: bar_origin,
            },
        ) => Some(Roll::Bar {
            share,
            ranges,
            origin: origin.or(bar_origin),
        }),
        _ => None,
    }
}

/// The filters of the properties of a rare, magic or normal item, like Exiled Exchange 2: its
/// base type, checked (unchecked, its item class is searched in its place), the base of a
/// waystone written without its tier, a bar of its tier under it; its item level, checked on a
/// normal item only (a player who checks it on another usually looks for a level of crafting),
/// with the level of the item as its minimum; then its sockets, its quality and its required
/// level when they are filters, and the number of its unrevealed modifiers, checked with that
/// number as its minimum.
fn item_filters<'a>(item: &'a Item, base_type: Option<&'a str>) -> Vec<Stat<'a>> {
    let base_type_filter = base_type.map(|base_type| {
        let waystone = if item.is_waystone() {
            item::base_and_tier(item.language(), base_type)
        } else {
            None
        };
        Stat {
            key: FilterKey::BaseType,
            text: StatText::BaseType {
                base_type: waystone.map_or(base_type, |(name, _)| name),
                class: link::searched_category(item, base_type)
                    .is_some()
                    .then(|| item.class()),
            },
            filter: Some(StatFilter {
                checked: true,
                limit: None,
            }),
            unsearchable: false,
            roll: waystone.map(|(_, tier)| Roll::WaystoneTier(tier)),
        }
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

/// The filters of the properties of a waystone, in the order of the item: the item rarity and
/// the drop chance checked, what the players run a map for, the others unchecked; a percentage
/// with a minimum at 90 % of its value, rounded down, as a modifier, the revives with their
/// number; none for a property of zero or below, which searches nothing.
fn waystone_filters(item: &Item) -> Vec<Stat<'static>> {
    item.waystone_properties()
        .into_iter()
        .filter(|found| found.value > 0)
        .map(|found| {
            let value = f64::from(found.value);
            let minimum = if found.property.is_percent() {
                (value * (1.0 - COMPARABLE_MARGIN)).floor()
            } else {
                value
            };
            let checked = matches!(
                found.property,
                WaystoneProperty::ItemRarity | WaystoneProperty::DropChance
            );
            property_filter(
                Property::Waystone(found.property),
                value,
                Limit::minimum(minimum),
                checked,
            )
        })
        .collect()
}

/// The filters of a skill gem, each with its value as its minimum: its level, always checked, so
/// that a gem of level 15 isn't compared with gems of level 1 sold for nothing; its quality,
/// checked from 16 %, like Exiled Exchange 2; its sockets, checked from 3, every gem having 2.
/// None for a support gem, which has no level.
fn gem_filters(item: &Item) -> Vec<Stat<'static>> {
    let Some(level) = item.skill_gem_level() else {
        return Vec::new();
    };
    // Within the values a gem can take, from `min` to `max`.
    let filter = |property, value: u32, (min, max): (u32, u32), checked| {
        let value = f64::from(value);
        let bounds = Range {
            min: f64::from(min),
            max: f64::from(max),
        };
        property_filter(
            property,
            value,
            Limit::minimum(value).within(bounds),
            checked,
        )
    };
    let quality = item
        .quality()
        .filter(|&quality| quality > 0)
        .map(|quality| {
            let checked = quality >= GEM_QUALITY;
            filter(Property::Quality, quality, (0, MAX_GEM_QUALITY), checked)
        });
    let sockets = item
        .sockets()
        .filter(|&sockets| sockets > 0)
        .map(|sockets| {
            let checked = sockets >= GEM_SOCKETS;
            filter(Property::GemSockets, sockets, (0, MAX_GEM_SOCKETS), checked)
        });
    [filter(Property::GemLevel, level, (1, MAX_GEM_LEVEL), true)]
        .into_iter()
        .chain(quality)
        .chain(sockets)
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
/// within the highest quality of the base: the quality of a catalyst, with its kind, unchecked
/// (the trade site doesn't tell the kinds apart); like Exiled Exchange 2, for a charm, checked
/// from 10 %; for a flask from 20 %, checked above; for any other item above 20 % (an
/// exceptional quality), checked except on a rare, whose craft is usually done; and on a normal
/// item that has quality, unchecked.
fn quality_filter(item: &Item, category: Option<Category>) -> Option<Stat<'_>> {
    let quality = item.quality().filter(|&quality| quality > 0)?;
    let max = category.map_or(MAX_QUALITY, Category::max_quality);
    let limit = Limit::minimum(f64::from(quality)).within(Range {
        min: 0.0,
        max: f64::from(max),
    });
    if let Some(kind) = item.quality_kind() {
        return Some(Stat {
            key: FilterKey::Property(Property::Quality),
            text: StatText::CatalystQuality(kind, f64::from(quality)),
            filter: Some(StatFilter {
                checked: false,
                limit: Some(limit),
            }),
            unsearchable: false,
            roll: None,
        });
    }
    let checked = match category {
        Some(Category::Charm) => quality >= CHARM_QUALITY,
        Some(Category::Flask) if quality >= MAX_USUAL_QUALITY => quality > MAX_USUAL_QUALITY,
        _ if quality > MAX_USUAL_QUALITY => item.rarity() != Rarity::Rare,
        _ if item.rarity() == Rarity::Normal => false,
        _ => return None,
    };
    Some(property_filter(
        Property::Quality,
        f64::from(quality),
        limit,
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
    /// Every line starts unchecked: the item has an unrevealed modifier, which can change it,
    /// or is a waystone, whose buyer pays for its properties, not for its monsters (like Exiled
    /// Exchange 2).
    unchecked: bool,
}

impl Reading<'_> {
    /// The line `key` starts checked: unchecked when the DPS of the weapon counts it, its
    /// damage or its attack speed, when a pseudo total adds it up, or on an item whose lines
    /// all start unchecked (an unrevealed modifier, a waystone).
    fn checked(&self, key: StatKey) -> bool {
        let weapon_damage = self.dps_filters && self.stats.weapon_damage(key);
        let added_up = self.pseudo_filters && self.stats.added_up(key);
        !weapon_damage && !added_up && !self.unchecked
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
        unique_stat(key, line, origin, item.has_rolls_from_prices(), reading)
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
    let max = |limit: u32| (!limits.counted_only).then_some(limit);
    let line_varies = |line: &StatLine, _: &Stat<'_>| varies(line);
    let prefixes = affix_stats(item, Some(Affix::Prefix), reading);
    let suffixes = affix_stats(item, Some(Affix::Suffix), reading);
    let filled = |affix| {
        item.modifiers()
            .iter()
            .map(|modifier| affix_count(modifier, affix))
            .sum()
    };
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
            filled: filled(Affix::Prefix),
            max: max(limits.prefixes),
        },
        prefixes,
    );
    push_group(
        &mut rows,
        Group::Suffixes {
            filled: filled(Affix::Suffix),
            max: max(limits.suffixes),
        },
        suffixes,
    );
    push_group(&mut rows, Group::Explicit, affix_stats(item, None, reading));
    rows
}

/// A line of an enchantment or a rune: a filter, since the trade site searches them, without
/// a roll; unchecked, a rune being easy to change.
fn added<'a>(key: StatKey, line: &'a StatLine, reading: &Reading<'_>) -> Stat<'a> {
    let half = reading.half(key);
    let counted = reading.counted(key, line);
    Stat {
        key: FilterKey::Stat(key),
        text: StatText::line(&line.text),
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
        text: StatText::line(&line.text),
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

/// The prefixes or suffixes of `affix` that `modifier` counts in the room of its item: one of
/// its affix, two for a line that adds up two modifiers of that affix, one for a line that adds
/// up a prefix and a suffix; none for a modifier whose affix is unknown.
fn affix_count(modifier: &Modifier, affix: Affix) -> u32 {
    match (&modifier.doubt, modifier.kind) {
        (Some(TierDoubt::Split { whole, added, .. }), _) => {
            u32::from(whole.affix == affix) + u32::from(added.affix == affix)
        }
        (_, ModifierKind::Explicit(Some(kind))) => u32::from(kind == affix),
        _ => 0,
    }
}

/// The explicit modifiers of `affix` (`None`: a header that doesn't tell it), those with a
/// line that varies first and those not revealed yet last: each line with its bar over every
/// tier of its modifier, and its tier and the ranges of that tier under its last line; the
/// tier of each affix for a modifier copied from the in-game trade that can be either, and
/// the tier of each part under a line that adds up two modifiers.
fn affix_stats<'a>(item: &'a Item, affix: Option<Affix>, reading: &Reading<'_>) -> Vec<Stat<'a>> {
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
    modifiers
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
            let origin = Origin::of_modifier(modifier, reading.tiers.currency(index));
            // The origin under the tiers of a line that adds up two modifiers, the tier of the
            // hybrid under the last of its other lines.
            let (origin_line, tier_line) = match &modifier.doubt {
                Some(TierDoubt::Split { line, .. }) => (
                    *line,
                    other_lines(modifier, *line)
                        .next_back()
                        .map(|(index, _)| index),
                ),
                _ => (last, Some(last)),
            };
            modifier
                .lines
                .iter()
                .enumerate()
                .map(move |(line_index, line)| {
                    let key = StatKey {
                        modifier: index,
                        line: line_index,
                    };
                    let tier = match &modifier.doubt {
                        _ if tier_line != Some(line_index) => None,
                        Some(TierDoubt::Split {
                            line, whole, added, ..
                        }) => hybrid_row(modifier, *line, whole, added),
                        _ => {
                            // A modifier of one tier only: the ranges of that tier without its
                            // number, also when the copy writes no tier; nothing when its
                            // value is fixed.
                            let single = reading.tiers.single(index);
                            let number = modifier.tier.filter(|_| !single);
                            let ranges = tier_ranges(modifier);
                            (number.is_some() || (single && !ranges.is_empty()))
                                .then(|| TierRow::plain(ModifierTier { number, ranges }))
                        }
                    };
                    let origin = origin.filter(|_| line_index == origin_line);
                    let mut stat = affix_stat(key, line, tier, origin, reading);
                    let rows = match &modifier.doubt {
                        Some(TierDoubt::EitherAffix { prefix, suffix }) if line_index == last => {
                            Some(vec![TierRow::affix(prefix), TierRow::affix(suffix)])
                        }
                        Some(TierDoubt::Split {
                            line, whole, added, ..
                        }) if line_index == *line => {
                            let others: Vec<&str> = other_lines(modifier, *line)
                                .map(|(_, other)| other.text.as_str())
                                .collect();
                            Some(split_rows(whole, added, &others))
                        }
                        _ => None,
                    };
                    if let Some(rows) = rows {
                        stat.roll = Some(Roll::Tiers { rows, origin });
                    }
                    stat
                })
                .collect()
        })
        .collect()
}

/// The line `key` of a prefix or suffix, `line`: a filter that stays within the rolls of every
/// tier of its modifier, its bar over them, and under its last line the tier of the modifier
/// and the ranges of that tier, then its origin.
fn affix_stat<'a>(
    key: StatKey,
    line: &'a StatLine,
    tier: Option<TierRow<'a>>,
    origin: Option<Origin>,
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
        text: StatText::line(&line.text),
        filter: Some(filter),
        unsearchable: false,
        roll: (share.is_some() || tier.is_some() || origin.is_some()).then_some(Roll::Affix {
            share,
            tier: tier.map(Box::new),
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
/// For a unique copied without ranges, whose `rolls_from_prices`, a line with a number none
/// of whose values got a range from the prices may vary: a filter unchecked at its value,
/// without bar nor range. The `origin` of its modifier (Vaal) follows its range, or stands
/// alone under a line without bar.
fn unique_stat<'a>(
    key: StatKey,
    whole: &'a StatLine,
    origin: Option<ModifierOrigin>,
    rolls_from_prices: bool,
    reading: &Reading<'_>,
) -> Stat<'a> {
    let line = &reading.counted(key, whole);
    let ranges = copied_ranges(line);
    let origin = origin.map(Origin::Written);
    let fixed = Stat {
        key: FilterKey::Stat(key),
        text: StatText::line(&whole.text),
        filter: None,
        unsearchable: false,
        roll: origin.map(|origin| Roll::Affix {
            share: None,
            tier: None,
            origin: Some(origin),
        }),
    };
    if rolls_from_prices && ranges.iter().all(Option::is_none) {
        let Some(value) = value(line) else {
            return fixed;
        };
        return Stat {
            filter: Some(StatFilter {
                checked: false,
                limit: Some(Limit::new(value, reading.half(key), None, value, line)),
            }),
            ..fixed
        };
    }
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

/// The ranges written under the last line of a prefix or suffix, those the copy writes, in their
/// order; none for a fixed value ([`Range::unless_fixed`]: `50(50)% reduced Amount Recovered`, or
/// the range the tables put on a line of the in-game trade, `+2 to Level of all Minion Skills`).
fn tier_ranges(modifier: &Modifier) -> Vec<Range> {
    Range::unless_fixed(
        modifier
            .lines
            .iter()
            .flat_map(|line| magnitudes(line, &copied_ranges(line))),
    )
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
fn bar<'a>(rolls: Vec<(f64, Range)>, origin: Option<Origin>) -> Option<Roll<'a>> {
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
            Row::UniqueRollsNote => note(ui, i18n.text(Message::UniqueRollsNote)),
        }
    }
}

/// A note under the stats, in faint text, wrapped.
fn note(ui: &mut Ui, words: String) {
    ui.label(RichText::new(words).font(text::note()).color(color::FAINT));
}

/// A small title, with the fill of the prefixes and suffixes: lighter while one is open, the count
/// alone without most; and "min – max" on its right, above the range fields of the filters.
fn group_title(ui: &mut Ui, group: Group, range: bool, i18n: &Translator) {
    let (message, fill) = match group {
        Group::Item => (Message::GroupItem, None),
        Group::Weapon => (Message::GroupWeapon, None),
        Group::Waystone => (Message::GroupWaystone, None),
        Group::Gem => (Message::GroupGem, None),
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
            let (words, tint) = match max {
                Some(max) if filled < max => (i18n.affix_fill(filled, max), color::ACCENT),
                Some(max) => (i18n.affix_fill(filled, max), color::WEAK),
                None => (filled.to_string(), color::WEAK),
            };
            ui.label(RichText::new(words).font(text::label()).color(tint));
        }
        if range {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                small_title(ui, Message::FilterRange);
            });
        }
    });
}

/// A stat in the color of the modifiers, on four lines at most, and its roll under it; a
/// filter between its checkbox and the column of its range field.
fn stat_row(ui: &mut Ui, stat: &Stat<'_>, filters: &mut Filters, i18n: &Translator, bar: RollBar) {
    if let StatText::Unrevealed(_) = stat.text {
        unrevealed_row(ui, &stat.text.text(i18n));
        return;
    }
    // The row starts on a whole pixel, so that its checkbox and its range field round alike
    // on every row.
    let top = ui.cursor().top();
    let pixels = ui.pixels_per_point();
    ui.add_space((top * pixels).ceil() / pixels - top);
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
                stat,
                None,
                (width - GUTTER - GAP - GAP - RANGE_COLUMN).max(0.0),
                bar,
                i18n,
            );
            return;
        }
        let Some(filter) = stat.filter else {
            stat_text(ui, stat, None, width, bar, i18n);
            return;
        };
        let mut choice = filters.get(stat.key, || filter.default_choice());
        let before = choice.clone();
        // The checkbox on the first line of the text.
        let label = stat.text.label(i18n);
        if widgets::check_box(ui, choice.checked, vec2(GUTTER, line_height), &label).clicked() {
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
            stat,
            note.as_deref(),
            (width - GUTTER - GAP - GAP - column).max(0.0),
            bar,
            i18n,
        );
        ui.add_space(GAP);
        let edited = filter
            .limit
            .and_then(|limit| range_field(ui, stat.key, &mut choice, limit, i18n));
        if choice != before {
            filters.set(stat.key, choice);
        }
        // Settled if its field disappears before the player leaves it.
        if let Some(half) = edited {
            let window = if ui.ctx().viewport_id() == ViewportId::ROOT {
                Window::Main
            } else {
                Window::OverTheGame
            };
            filters.edit(window, stat.key, half);
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

/// The text of a stat and its roll, the tag of the passive skills of its radius after it (alone
/// under a line without roll), or a `note` in faint text under it, `width` across.
fn stat_text(
    ui: &mut Ui,
    stat: &Stat,
    note: Option<&str>,
    width: f32,
    bar: RollBar,
    i18n: &Translator,
) {
    ui.allocate_ui_with_layout(vec2(width, 0.0), Layout::top_down(Align::Min), |ui| {
        ui.set_width(width);
        let response = ui.add(Label::new(clamped(&stat.text.text(i18n))).wrap());
        if let Some(tip) = stat.text.tip() {
            response.on_hover_ui(|ui| {
                ui.set_max_width(WORD_TIP_WIDTH);
                ui.add(Label::new(i18n.text(tip)).wrap());
            });
        }
        let skills = stat.text.radius_skills();
        if let Some(roll) = &stat.roll {
            ui.add_space(ROLL_GAP);
            roll_line(ui, roll, skills, bar, i18n);
        } else if let Some(skills) = skills {
            ui.add_space(ROLL_GAP);
            roll_row(ui, |ui| radius_tag(ui, skills, i18n));
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
/// tooltip under the pointer; the half the player is in, if any.
fn range_field(
    ui: &mut Ui,
    key: FilterKey,
    choice: &mut Filter,
    limit: Limit,
    i18n: &Translator,
) -> Option<Half> {
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
    let mut edited = None;
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
                // Read in egui, as `Context::text_edit_focused`: the window may not have the
                // keyboard of the system yet.
                if ui.memory(|memory| memory.has_focus(id)) {
                    edited = Some(half);
                }
            }
        });
    // The field as it is drawn, without the margin that raises it.
    let rect = field.response.rect - RANGE_RAISE;
    if ui.rect_contains_pointer(rect) {
        field_tooltip(ui, key, rect, limit, i18n);
    }
    edited
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
/// suffix, one under the other, then its origin on a row of its own. Last, the tag of the
/// passive `skills` of the radius of a line of a Time-Lost jewel.
fn roll_line(
    ui: &mut Ui,
    roll: &Roll,
    skills: Option<RadiusSkills>,
    bar: RollBar,
    i18n: &Translator,
) {
    // What ends the roll: its origin, then the tag.
    let ending = |ui: &mut Ui, origin: Option<Origin>| {
        if let Some(origin) = origin {
            origin_word(ui, origin, i18n);
        }
        if let Some(skills) = skills {
            radius_tag(ui, skills, i18n);
        }
    };
    match roll {
        Roll::Bar {
            share,
            ranges,
            origin,
        } => roll_row(ui, |ui| {
            roll_bar(ui, *share, bar, color::ACCENT);
            range_label(ui, ranges);
            ending(ui, *origin);
        }),
        Roll::Affix {
            share,
            tier,
            origin,
        } => roll_row(ui, |ui| {
            if let Some(share) = share {
                roll_bar(ui, *share, bar, color::ACCENT);
            }
            if let Some(tier) = tier {
                tier_row(ui, tier, i18n);
            }
            ending(ui, *origin);
        }),
        Roll::Tiers { rows, origin } => {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = TIER_ROW_GAP;
                for row in rows {
                    roll_row(ui, |ui| tier_row(ui, row, i18n));
                }
                // On a line of its own: the line has it whichever affix it is.
                if origin.is_some() || skills.is_some() {
                    roll_row(ui, |ui| ending(ui, *origin));
                }
            });
        }
        Roll::WaystoneTier(tier) => roll_row(ui, |ui| {
            let tint = waystone_tier_color(*tier);
            roll_bar(ui, waystone_tier_share(*tier), bar, tint);
            ui.label(
                RichText::new(format!("T{tier}"))
                    .font(text::tier())
                    .color(tint),
            );
        }),
    }
}

/// Where the tier of a waystone falls over every tier, from 0 for tier 1 to 1 for the highest.
fn waystone_tier_share(tier: u32) -> f32 {
    let share = f64::from(tier.saturating_sub(1)) / f64::from(MAX_WAYSTONE_TIER - 1);
    #[expect(clippy::cast_possible_truncation, reason = "a share from 0 to 1")]
    let share = share.clamp(0.0, 1.0) as f32;
    share
}

/// A tier and its ranges ([`tier_label`]), then the word after them in gray, with its
/// tooltip wrapped past [`WORD_TIP_WIDTH`].
fn tier_row(ui: &mut Ui, row: &TierRow, i18n: &Translator) {
    let word = row_word(row, i18n);
    if !row.quiet_tier() {
        tier_label(ui, &row.tier, row.better);
    }
    if let Some(word) = word {
        let response = small_word(ui, word, color::WEAK);
        if let Some(tip) = &row.tip {
            let text = match tip {
                WordTip::Text(message) => i18n.text(*message),
                WordTip::HybridLines(lines) => i18n.hybrid_with(lines),
                WordTip::HybridPart { ranges, line } => {
                    i18n.hybrid_part(&ranges_text(ranges), line)
                }
            };
            response.on_hover_ui(|ui| {
                ui.set_max_width(WORD_TIP_WIDTH);
                ui.add(Label::new(text).wrap());
            });
        }
    }
    if let Some(currency) = row.currency {
        origin_word(ui, Origin::Currency(currency), i18n);
    }
}

/// One row of a roll, its parts `GAP` apart.
fn roll_row(ui: &mut Ui, parts: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = GAP;
        parts(ui);
    });
}

/// A tier, `T1` in its color, and its ranges (the ranges alone for a modifier of one tier only);
/// or two neighbouring tiers, the `better` and `tier`, `T1/T2` in the color of the worse, and
/// the ranges of both.
fn tier_label(ui: &mut Ui, tier: &ModifierTier, better: Option<u32>) {
    if let Some(number) = tier.number {
        let label = match better {
            Some(better) => format!("T{better}/T{number}"),
            None => format!("T{number}"),
        };
        ui.label(
            RichText::new(label)
                .font(text::tier())
                .color(tier_color(number)),
        );
    }
    if !tier.ranges.is_empty() {
        range_label(ui, &tier.ranges);
    }
}

/// The word of an origin, small, in its color: the last item of the line of a roll.
fn origin_word(ui: &mut Ui, origin: Origin, i18n: &Translator) {
    let (message, tint) = origin_label(origin);
    small_word(ui, i18n.text(message), tint);
}

/// A small word in `tint`: an origin, after the ranges of a tier or on a row of its own, or the
/// affix of a tier among several ("Prefix") in gray, after its ranges; `ORIGIN_GAP` after what
/// comes before it.
fn small_word(ui: &mut Ui, word: String, tint: Color32) -> Response {
    space_before(ui, ORIGIN_GAP);
    ui.label(RichText::new(word).font(text::origin()).color(tint))
}

/// The tag of the passive `skills` a line of a Time-Lost jewel is granted to, small, gray on a
/// block: "Small", `GAP` after what comes before it.
fn radius_tag(ui: &mut Ui, skills: RadiusSkills, i18n: &Translator) {
    space_before(ui, GAP);
    let galley = widgets::text_line(ui, i18n.text(skills.message()), text::origin());
    let (rect, _) =
        ui.allocate_exact_size(galley.size() + 2.0 * RADIUS_TAG_PADDING, Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(size::RADIUS), color::CONTROL);
    painter.galley(rect.min + RADIUS_TAG_PADDING, galley, color::WEAK);
}

/// Makes the space between what the row of `ui` already holds and the next item `gap`: egui puts
/// the spacing of the row after an item, so the item before set it. Nothing on an empty row.
fn space_before(ui: &mut Ui, gap: f32) {
    if ui.min_rect().width() > 0.0 {
        ui.add_space(gap - ui.spacing().item_spacing.x);
    }
}

/// The word of an origin, and its color: that of the crafted modifiers for a currency, as the
/// trade site files them.
fn origin_label(origin: Origin) -> (Message, Color32) {
    match origin {
        Origin::Written(ModifierOrigin::Desecrated) => {
            (Message::OriginDesecrated, color::DESECRATED)
        }
        Origin::Written(ModifierOrigin::Fractured) => (Message::OriginFractured, color::FRACTURED),
        Origin::Written(ModifierOrigin::Crafted) => (Message::OriginCrafted, color::CRAFTED),
        Origin::Written(ModifierOrigin::Vaal) => (Message::OriginVaal, color::VAAL),
        Origin::Currency(Currency::Essence) => (Message::OriginEssence, color::CRAFTED),
        Origin::Currency(Currency::Alloy) => (Message::OriginAlloy, color::CRAFTED),
    }
}

/// The bar of a roll, filled to `share` in the style `bar`, in `tint`.
fn roll_bar(ui: &mut Ui, share: f32, bar: RollBar, tint: Color32) {
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
            painter.rect_filled(fill, radius, tint);
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
mod tests;
