//! The rows of the stats as they are decided, before they are drawn: the small titles, the
//! stats with their text, their filter and their roll, the tiers written under a line.

use std::borrow::Cow;

use super::property_value;
use crate::filters::{self, Filter, FilterKey, Half, Property};
use crate::format;
use crate::i18n::{Message, Translator};
use crate::item::{
    Affix, AffixTier, Currency, Lord, Modifier, ModifierOrigin, PartTier, QualityKind, Range,
    StatLine, TierDoubt,
};
use crate::tier_rolls::TierSpan;
use crate::trade::pseudo::Pseudo;

/// A small title of the stats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Group {
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
    /// alone ([`crate::game_data::AffixLimits::counted_only`]).
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
pub(super) enum Origin {
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
    pub(super) fn of_modifier(modifier: &Modifier, currency: Option<Currency>) -> Option<Self> {
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
pub(super) enum StatRoll<'a> {
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
pub(super) struct ModifierTier {
    /// 1 is the best; none for a modifier of one tier only on its base, whose T1 would read as
    /// "the best" while it is "the only one".
    pub(super) number: Option<u32>,
    pub(super) ranges: Vec<Range>,
}

/// A tier under a line, or two neighbouring ones, and the word after its ranges.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct TierRow<'a> {
    /// The tier, or the worse of two neighbouring tiers.
    pub(super) tier: ModifierTier,
    /// The better of two neighbouring tiers the modifier may be: `T1/T2`.
    pub(super) better: Option<u32>,
    /// In gray after the ranges: the affix of the tier ("Prefix"), or "Hybrid".
    pub(super) word: Option<Message>,
    /// The lord of the Abyss named before the affix: "Amanamu Prefix".
    pub(super) lord: Option<Lord>,
    /// Whether its tier is written in the style of its word, "T2 Suffix": only for an affix of a
    /// line in doubt without range (a fixed value), set by [`Self::affix`]; the other rows keep
    /// the color of their tier ("T1 Hybrid suffix" under the other line of a hybrid split from a
    /// summed line, whose T1 the summed line writes too).
    pub(super) is_tier_in_word: bool,
    /// The tooltip of that word.
    pub(super) tip: Option<WordTip<'a>>,
    /// The currency that adds the part of a line that adds up two modifiers, an essence or an
    /// alloy, named last, as after the ranges of any modifier it adds.
    pub(super) currency: Option<Currency>,
}

/// The tooltip of the word after the ranges of a tier.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum WordTip<'a> {
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
    /// The tier `tier` of a modifier, without word.
    pub(super) fn plain(tier: ModifierTier) -> Self {
        Self {
            tier,
            better: None,
            word: None,
            lord: None,
            is_tier_in_word: false,
            tip: None,
            currency: None,
        }
    }

    /// The tier `found` for one affix of a line that can be either, "Prefix" after its ranges;
    /// without number for a modifier of one tier only, or whose tier the game doesn't write.
    pub(super) fn affix(found: &AffixTier) -> Self {
        Self {
            tier: ModifierTier {
                number: (!found.hides_number).then_some(found.tier),
                ranges: found.ranges.clone(),
            },
            better: None,
            word: Some(affix_word(found.affix)),
            lord: found.lord,
            is_tier_in_word: found.ranges.is_empty(),
            tip: None,
            currency: None,
        }
    }

    /// The tiers of a part of a line that adds up two modifiers, over the range of both when
    /// they are two, with `word` and its `tip`.
    pub(super) fn part(
        part: &PartTier,
        word: Option<Message>,
        tip: Option<WordTip<'a>>,
    ) -> Option<Self> {
        Self::tiers(part, part.ranges.clone(), word, tip)
    }

    /// The tiers `part` may be, the best first (one, or two neighbouring ones), over `ranges`,
    /// with `word` and its `tip`; without number for a modifier of one tier only, or whose tier
    /// the game doesn't write.
    pub(super) fn tiers(
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
            is_tier_in_word: false,
            tip,
            currency: part.currency,
        })
    }
}

/// The words after the ranges of the tier of `row`, in `i18n`: its affix, after the lord whose
/// modifier it is ("Amanamu Prefix"); after its tier written as they are ("T2 Suffix") under a
/// line in doubt without range (a fixed value), so that it reads as one with them (owner's choice
/// on 2026-10-30).
pub(super) fn tier_word(row: &TierRow, i18n: &Translator) -> Option<String> {
    let word = row.word?;
    let text = match (row.lord, word) {
        (Some(lord), Message::AffixPrefix) => i18n.lord_affix(Affix::Prefix, lord),
        (Some(lord), Message::AffixSuffix) => i18n.lord_affix(Affix::Suffix, lord),
        _ => i18n.text(word),
    };
    Some(match row.tier.number.filter(|_| row.is_tier_in_word) {
        Some(number) => format!("T{number} {text}"),
        None => text,
    })
}

/// The word of an affix after the ranges of its tier.
pub(super) fn affix_word(affix: Affix) -> Message {
    match affix {
        Affix::Prefix => Message::AffixPrefix,
        Affix::Suffix => Message::AffixSuffix,
    }
}

/// A filter of the search, as it starts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct StatFilter {
    pub(super) checked: bool,
    /// None for a line that never varies, e.g. a fixed implicit of a rare.
    pub(super) limit: Option<Limit>,
}

/// The limit of a filter at first: its value, the half of the range field it fills, and the
/// range it stays in, when known.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Limit {
    pub(super) value: f64,
    /// The maximum for a line better lower, the minimum otherwise.
    pub(super) half: Half,
    pub(super) bounds: Option<Range>,
    /// The rolls the tooltip of the field tells: the range of today of a line with one
    /// number; the field of a line with several holds their average, not a roll.
    pub(super) rolls: Option<Range>,
    /// The tiers of the lowest and of the highest of `rolls`, for a prefix or a suffix, which
    /// the tooltip writes under them: "T13–T1".
    pub(super) tiers: Option<TierSpan>,
}

/// The characters a number of a stat reaches for its range field to hold four a side.
const WIDE_CHARACTERS: usize = 4;

impl Limit {
    /// A minimum of `value`, with no range to stay in.
    pub(super) fn minimum(value: f64) -> Self {
        Self {
            value,
            half: Half::Minimum,
            bounds: None,
            rolls: None,
            tiers: None,
        }
    }

    /// A maximum of `value`, with no range to stay in.
    pub(super) fn maximum(value: f64) -> Self {
        Self {
            half: Half::Maximum,
            ..Self::minimum(value)
        }
    }

    /// A limit of `value` filling `half`, for `line` copied with the value `copied`, which
    /// rolls in `range`: its bounds are `range`, widened to hold `copied` for a legacy unique,
    /// rolled in a range that changed since.
    pub(super) fn new(
        value: f64,
        half: Half,
        range: Option<Range>,
        copied: f64,
        line: &StatLine,
    ) -> Self {
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
    pub(super) fn within(self, bounds: Range) -> Self {
        Self {
            bounds: Some(bounds.holding(self.value)),
            ..self
        }
    }

    /// The minus sign is allowed in the field.
    pub(super) fn allows_minus_sign(self) -> bool {
        self.value < 0.0 || self.bounds.is_some_and(|bounds| bounds.min < 0.0)
    }

    /// The halves of the field hold four characters: a number of its stat reaches them.
    pub(super) fn is_wide(self) -> bool {
        let reaches = |value: f64| filters::limit_text(value).chars().count() >= WIDE_CHARACTERS;
        reaches(self.value)
            || self
                .bounds
                .is_some_and(|bounds| reaches(bounds.min) || reaches(bounds.max))
    }
}

impl StatFilter {
    /// The filter as the player finds it.
    pub(super) fn default_choice(self) -> Filter {
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
pub(super) enum RadiusSkills {
    Small,
    Notable,
}

impl RadiusSkills {
    /// Its tag, after the range of the line: "Small", "Notable".
    pub(super) fn message(self) -> Message {
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
pub(super) enum StatText<'a> {
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

impl<'a> StatText<'a> {
    /// A line of a modifier, without the words of its radius for a line of a Time-Lost jewel.
    pub(super) fn line(text: &'a str) -> Self {
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
    pub(super) fn radius_skills(self) -> Option<RadiusSkills> {
        match self {
            Self::RadiusLine { skills, .. } => Some(skills),
            _ => None,
        }
    }

    /// The text that names the stat to a screen reader: the whole line of a Time-Lost jewel,
    /// with the words of its radius shown as a tag.
    pub(super) fn label(self, i18n: &Translator) -> Cow<'a, str> {
        match self {
            Self::RadiusLine { whole, .. } => Cow::Borrowed(whole),
            _ => self.text(i18n),
        }
    }

    /// The tooltip of the text: the quality of a catalyst, whose kinds the trade site doesn't
    /// tell apart.
    pub(super) fn tip(self) -> Option<Message> {
        matches!(self, Self::CatalystQuality(..)).then_some(Message::FilterAnyQualityKind)
    }

    pub(super) fn text(self, i18n: &Translator) -> Cow<'a, str> {
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
pub(super) struct Stat<'a> {
    pub(super) key: FilterKey,
    pub(super) text: StatText<'a>,
    /// A filter of the search, with its checkbox and its range field.
    pub(super) filter: Option<StatFilter>,
    /// A line the trade site can't search, which would be a filter: the "not allowed" sign in
    /// place of its checkbox.
    pub(super) is_unsearchable: bool,
    pub(super) roll: Option<StatRoll<'a>>,
}

/// A row of the stats.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Row<'a> {
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

#[cfg(test)]
mod tests;
