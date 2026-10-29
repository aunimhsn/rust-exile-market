//! The modifier tables, from RePoE (`assets/game-data/modifiers.ndjson`): every tier of a
//! prefix or suffix on the base of an item, found from the stats of its lines, or from their
//! values alone for an item copied from the in-game trade.

use std::sync::LazyLock;

use serde::Deserialize;

use super::bases;
use super::lines::{LineMatch, StatSource, modifier_stats};
use crate::item::{Affix, Modifier, ModifierKind, ModifierOrigin, Range};
use crate::language::Language;

/// How far a copied value can be from a tier: the game rounds the values it scales, e.g. a
/// regeneration stored per minute shown per second with one decimal.
const ROUNDING: f64 = 0.051;

/// The most ways to read the lines of an item copied from the in-game trade that are looked
/// at: a handful on a real item, each hybrid the lines may be doubling them.
const MAX_READINGS: usize = 4096;

/// The tiers of a type of modifier that roll on the same bases, as the generator writes them.
#[derive(Deserialize)]
struct Table {
    /// The modifiers of one type can't be on the same item: its tiers.
    #[serde(rename = "type")]
    kind: String,
    affix: TableAffix,
    lines: Vec<TableLine>,
    /// Each tag and its weight: the first tag of the list a base has tells whether the
    /// modifier rolls on it (above zero) or not.
    spawn: Vec<(String, u32)>,
    tiers: Vec<TableTier>,
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum TableAffix {
    Prefix,
    Suffix,
}

#[derive(Deserialize, PartialEq)]
struct TableLine {
    /// The English template of the line, as Exiled Exchange 2 writes it.
    template: String,
    /// What the value of the stat is divided by to be shown, for a line of one value that can
    /// be written without number: its value is then that of the stat (1000 for `Inherent Rage
    /// loss starts 1 second later`, shown in seconds; no such line rolls on a base on
    /// 2026-10-28, the generator writes the divisor when one does).
    #[serde(default = "one")]
    divisor: f64,
}

fn one() -> f64 {
    1.0
}

#[derive(Deserialize)]
struct TableTier {
    level: u32,
    /// For each line, the range of each of its values, with the sign of the stat.
    ranges: Vec<Vec<[f64; 2]>>,
}

impl Table {
    /// Whether the modifier rolls on a base with `tags`.
    fn rolls_on(&self, tags: &[String]) -> bool {
        self.spawn
            .iter()
            .find(|(tag, _)| tags.contains(tag))
            .is_some_and(|&(_, weight)| weight > 0)
    }

    /// Whether the table holds tiers of the same modifier as `other`, maybe on other bases.
    fn same_modifier(&self, other: &Self) -> bool {
        self.kind == other.kind && self.affix == other.affix && self.lines == other.lines
    }
}

static TABLES: LazyLock<Vec<Table>> =
    LazyLock::new(|| read(include_str!("../../assets/game-data/modifiers.ndjson")));

/// A tier of a modifier on a base.
#[derive(Debug, Clone, PartialEq)]
pub struct Tier {
    /// 1 is the best.
    pub number: u32,
    /// The level an item needs for the tier to roll.
    pub level: u32,
    /// The range of each value of the copied lines, in their order, with the sign of the stat
    /// (`#% reduced` rolls negative values).
    pub ranges: Vec<Range>,
}

/// Every tier of a modifier on the base of its item.
#[derive(Debug, Clone, PartialEq)]
pub struct ModifierTiers {
    /// The best first: tier 1, tier 2…
    pub tiers: Vec<Tier>,
    /// The index in `tiers` of the tier of the copied modifier.
    pub found: usize,
    /// The number of values of each line match given, in their order: the ranges of a tier
    /// follow them (two for `Adds # to # Physical Damage`; for a line without number, one
    /// when its string gives the value, `Loads an additional bolt`, none otherwise).
    pub counts: Vec<usize>,
}

impl ModifierTiers {
    /// The tier of the copied modifier.
    #[must_use]
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "checked by the tests: the application shows the tier the advanced copy writes"
        )
    )]
    pub fn tier(&self) -> Option<&Tier> {
        self.tiers.get(self.found)
    }
}

/// Every tier of `modifier`, a prefix or suffix of an item whose base type is `base_type` in
/// `language` (the name its search looks for), its lines matched with their stats in
/// `matches` ([`super::modifier_stats`]).
///
/// None for a modifier the tables don't hold: an implicit, a rune, an enchantment, a modifier
/// whose header doesn't tell its affix, a line without stat, a base without tags; or whose
/// ranges are in no tier: a modifier of the lords of the Abyss is desecrated, like some
/// ordinary ones, and rolls its own range.
///
/// With the ranges of the advanced copy, only a tier with those ranges: the tier that holds the
/// copied values first, then the one that holds them give or take the rounding of the game, then
/// the one whose values left it (any value out of its tier, e.g. on a sanctified item). Without
/// them (a line without number has no range), only a tier that holds the values. When several
/// tiers fit as well, the best one is taken; when several modifiers do, the first of the file.
#[must_use]
pub fn modifier_tiers(
    language: Language,
    base_type: &str,
    modifier: &Modifier,
    matches: &[LineMatch<'_>],
) -> Option<ModifierTiers> {
    let affix = match modifier.kind {
        ModifierKind::Explicit(Some(Affix::Prefix)) => TableAffix::Prefix,
        ModifierKind::Explicit(Some(Affix::Suffix)) => TableAffix::Suffix,
        _ => return None,
    };
    let tags = bases::tags(language, base_type)?;
    let copied = Copied::of(modifier, matches)?;
    find(&TABLES, affix, tags, &copied, Tie::Best)
}

/// How many prefixes and suffixes an item can have: those of its rarity, with those its runes
/// and implicits add or take away (`+1 Suffix Modifier allowed`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "the tiers found are shown in step 13k2e3c")
)]
pub struct AffixLimits {
    pub prefixes: u32,
    pub suffixes: u32,
}

/// The tiers of a modifier copied from the in-game trade, found from its values.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "the tiers found are shown in step 13k2e3c")
)]
pub struct FoundTiers {
    /// The indexes among the modifiers of the item of the lines read as one modifier: one
    /// line, or the consecutive lines of a hybrid.
    pub modifiers: std::ops::Range<usize>,
    /// Each affix the modifier can be, with its tiers on the base, the copied one the worst
    /// that holds its values: a prefix or a suffix, or both when the item has room for either
    /// (the `18% increased Rarity of Items found` of a helmet). The ranges of a tier, and
    /// `counts`, follow the line matches of [`modifier_stats`] for each of these modifiers in
    /// their order.
    pub affixes: Vec<(Affix, ModifierTiers)>,
    /// The other affix, whose tables hold the values too, left out since the item has no room
    /// for it: a suffix, the prefixes being full.
    pub ruled_out: Option<Affix>,
}

/// The tiers of the explicit modifiers of a rare or magic item copied from the in-game trade,
/// found from their values in the tables of its base (`base_type` in `language`) among the
/// tiers its `item_level` allows; the other modifiers count as the affix their header tells. A
/// modifier no tier holds isn't in the list, nor counted as a prefix or a suffix: a stat the
/// tables don't know, or a line adding up two modifiers of the same stat.
///
/// Only the explicit modifiers without header are read, one line each, as the parser reads
/// such a copy: a modifier whose header the parser doesn't know isn't a line of the in-game
/// trade, the caller leaves it out. Each line is read alone, unless a line has no tier that
/// way, or the item more prefixes or suffixes than `limits`, or two fractured modifiers (a Fracturing Orb
/// fractures one modifier of an item not fractured yet): the consecutive lines of one origin
/// are then read as one modifier when they fit a hybrid of the tables (the fractured `41%
/// increased Energy Shield` and `+34 to maximum Mana` of a tiara), as the lines of the hybrids
/// follow each other on the owner's copies. An affix the full prefixes or suffixes rule out is
/// left out, unless no reading fits them, the tables being then out of step with the game. A
/// value in several tiers takes the worst of them.
///
/// A hybrid whose other line is read on its own (a sanctified value out of its tier, or two
/// lines apart) counts twice and may rule out an affix wrongly: a rare case, left as it is.
#[must_use]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "the tiers found are shown in step 13k2e3c")
)]
pub fn trade_tiers(
    language: Language,
    base_type: &str,
    item_level: Option<u32>,
    modifiers: &[Modifier],
    limits: AffixLimits,
) -> Vec<FoundTiers> {
    let Some(tags) = bases::tags(language, base_type) else {
        return Vec::new();
    };
    let copied: Vec<Option<Copied<'_>>> = modifiers
        .iter()
        .map(|modifier| {
            let open = modifier.kind == ModifierKind::Explicit(None)
                && modifier.tier.is_none()
                && !modifier.unrevealed
                && !modifier.lines.is_empty();
            open.then(|| Copied::of(modifier, &modifier_stats(language, modifier, None)))
                .flatten()
        })
        .collect();
    let candidates = candidates(modifiers, &copied, tags, Tie::Worst { item_level });
    let known = |affix| {
        modifiers
            .iter()
            .filter(|modifier| modifier.kind == ModifierKind::Explicit(Some(affix)))
            .count()
    };
    let room = |limit: u32, affix| {
        usize::try_from(limit)
            .unwrap_or(usize::MAX)
            .saturating_sub(known(affix))
    };
    let room = Room {
        prefixes: room(limits.prefixes, Affix::Prefix),
        suffixes: room(limits.suffixes, Affix::Suffix),
    };
    let fractured = modifiers
        .iter()
        .filter(|modifier| modifier.origin == Some(ModifierOrigin::Fractured))
        .count();
    let mut search = Search::new(&candidates, modifiers.len(), room, fractured);
    search.walk(0, &mut Vec::new());
    let Some((key, reading)) = search.best else {
        return Vec::new();
    };
    // A modifier that can be both, when the others leave room for one affix only.
    let both = if key.room {
        room.both(&reading, &candidates)
    } else {
        log::warn!(
            "Game data, {base_type} copied from the in-game trade: no reading of its lines fits {} prefixes and {} suffixes",
            limits.prefixes,
            limits.suffixes
        );
        (true, true)
    };
    reading
        .iter()
        .filter_map(|&index| {
            let candidate = candidates.get(index)?;
            let kept = |affix| match (candidate.set(), affix) {
                (AffixSet::Only(_), _) => true,
                (AffixSet::Both, Affix::Prefix) => both.0,
                (AffixSet::Both, Affix::Suffix) => both.1,
            };
            let (affixes, ruled_out): (Vec<_>, Vec<_>) = candidate
                .affixes
                .iter()
                .cloned()
                .partition(|(affix, _)| kept(*affix));
            Some(FoundTiers {
                modifiers: candidate.modifiers.clone(),
                affixes,
                ruled_out: ruled_out.first().map(|(affix, _)| *affix),
            })
        })
        .collect()
}

/// Consecutive modifiers of an item copied from the in-game trade read as one, and the affixes
/// whose tables hold their values.
struct Candidate {
    modifiers: std::ops::Range<usize>,
    /// Whether its modifiers are fractured.
    fractured: bool,
    /// A prefix, a suffix, or both, never none.
    affixes: Vec<(Affix, ModifierTiers)>,
}

impl Candidate {
    fn set(&self) -> AffixSet {
        match self.affixes.as_slice() {
            [(affix, _)] => AffixSet::Only(*affix),
            _ => AffixSet::Both,
        }
    }
}

/// The affixes a modifier can be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AffixSet {
    Only(Affix),
    Both,
}

/// The most lines a modifier of the tables has: a hybrid of three lines.
static MOST_LINES: LazyLock<usize> = LazyLock::new(|| {
    TABLES
        .iter()
        .map(|table| table.lines.len())
        .max()
        .unwrap_or(1)
});

/// Every modifier or run of modifiers among `modifiers` whose values the tables of a base with
/// `tags` hold, the tier taken on a `tie`, `copied` giving the lines of each modifier to read
/// (none for a modifier that isn't, or whose lines have no stat of the game data): one
/// modifier, or up to as many as the lines of a hybrid, of the same origin. The shorter first
/// at each start.
fn candidates(
    modifiers: &[Modifier],
    copied: &[Option<Copied<'_>>],
    tags: &[String],
    tie: Tie,
) -> Vec<Candidate> {
    let mut candidates = Vec::new();
    for (start, first) in modifiers.iter().enumerate() {
        let mut lines = Vec::new();
        let run = modifiers.iter().zip(copied).skip(start).take(*MOST_LINES);
        for (end, (modifier, part)) in (start + 1..).zip(run) {
            let Some(part) = part.as_ref().filter(|_| modifier.origin == first.origin) else {
                break;
            };
            lines.extend(part.lines.iter().cloned());
            let joined = Copied {
                lines: lines.clone(),
                ranges: None,
            };
            let affixes: Vec<_> = [
                (Affix::Prefix, TableAffix::Prefix),
                (Affix::Suffix, TableAffix::Suffix),
            ]
            .into_iter()
            .filter_map(|(affix, table_affix)| {
                Some((affix, find(&TABLES, table_affix, tags, &joined, tie)?))
            })
            .collect();
            if !affixes.is_empty() {
                candidates.push(Candidate {
                    modifiers: start..end,
                    fractured: first.origin == Some(ModifierOrigin::Fractured),
                    affixes,
                });
            }
        }
    }
    candidates
}

/// How many prefixes and suffixes the modifiers read from their values can add to the item.
#[derive(Debug, Clone, Copy)]
struct Room {
    prefixes: usize,
    suffixes: usize,
}

impl Room {
    /// The bounds of how many of the modifiers that can be both are prefixes, the others
    /// suffixes, when the `chosen` candidates fit the room; none when they don't.
    fn bounds(self, chosen: &[usize], candidates: &[Candidate]) -> Option<(usize, usize)> {
        let (mut prefixes, mut suffixes, mut both) = (0_usize, 0_usize, 0_usize);
        for candidate in chosen.iter().filter_map(|&index| candidates.get(index)) {
            match candidate.set() {
                AffixSet::Only(Affix::Prefix) => prefixes += 1,
                AffixSet::Only(Affix::Suffix) => suffixes += 1,
                AffixSet::Both => both += 1,
            }
        }
        let lowest = (both + suffixes).saturating_sub(self.suffixes);
        let highest = both.min(self.prefixes.checked_sub(prefixes)?);
        (lowest <= highest).then_some((lowest, highest))
    }

    /// Whether a modifier that can be both can be a prefix, and a suffix, while the others of
    /// the `chosen` candidates fit the room.
    fn both(self, chosen: &[usize], candidates: &[Candidate]) -> (bool, bool) {
        let both = chosen
            .iter()
            .filter_map(|&index| candidates.get(index))
            .filter(|candidate| candidate.set() == AffixSet::Both)
            .count();
        self.bounds(chosen, candidates)
            .map_or((true, true), |(lowest, highest)| {
                (highest >= 1, lowest < both)
            })
    }
}

/// How good a reading is, the better the greater: one fractured modifier at most, then the
/// room of the item, then the lines read with a tier, then the lines read alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct ReadingKey {
    fractured: bool,
    room: bool,
    /// How many lines are read with a tier: a line only a hybrid holds is read in it
    /// (`30% increased Physical Damage` of a weapon, the pure modifier starting at 40%).
    read: usize,
    /// How many modifiers the lines make, those without tier counted.
    modifiers: usize,
}

/// The ways to read the modifiers of an item, looked at one by one: for each, the candidates
/// chosen, the other modifiers without tier.
struct Search<'a> {
    candidates: &'a [Candidate],
    /// For each modifier of the item, the candidates that start at it, the shorter first.
    starting: Vec<Vec<usize>>,
    room: Room,
    /// How many modifiers of the item are fractured.
    fractured: usize,
    readings: usize,
    /// The best reading, the first one on a tie.
    best: Option<(ReadingKey, Vec<usize>)>,
}

impl<'a> Search<'a> {
    fn new(candidates: &'a [Candidate], count: usize, room: Room, fractured: usize) -> Self {
        let mut starting = vec![Vec::new(); count];
        for (index, candidate) in candidates.iter().enumerate() {
            if let Some(starting) = starting.get_mut(candidate.modifiers.start) {
                starting.push(index);
            }
        }
        Self {
            candidates,
            starting,
            room,
            fractured,
            readings: 0,
            best: None,
        }
    }

    /// Every reading from the modifier `position` on, after the candidates `chosen`, at most
    /// [`MAX_READINGS`]: the lines alone first, a modifier no candidate starts at read
    /// without tier.
    fn walk(&mut self, position: usize, chosen: &mut Vec<usize>) {
        if self.readings >= MAX_READINGS {
            return;
        }
        let Some(starting) = self.starting.get(position) else {
            self.readings += 1;
            self.keep(chosen);
            return;
        };
        let starting = starting.clone();
        let alone = starting.iter().any(|&index| {
            self.candidates
                .get(index)
                .is_some_and(|candidate| candidate.modifiers.len() == 1)
        });
        if !alone {
            self.walk(position + 1, chosen);
        }
        for index in starting {
            let Some(end) = self.candidates.get(index).map(|c| c.modifiers.end) else {
                continue;
            };
            chosen.push(index);
            self.walk(end, chosen);
            chosen.pop();
        }
    }

    fn keep(&mut self, chosen: &[usize]) {
        let (mut read, mut joined, mut fractured_joined) = (0, 0, 0);
        for candidate in chosen
            .iter()
            .filter_map(|&index| self.candidates.get(index))
        {
            read += candidate.modifiers.len();
            let lines = candidate.modifiers.len() - 1;
            joined += lines;
            if candidate.fractured {
                fractured_joined += lines;
            }
        }
        let key = ReadingKey {
            fractured: self.fractured.saturating_sub(fractured_joined) <= 1,
            room: self.room.bounds(chosen, self.candidates).is_some(),
            read,
            modifiers: self.starting.len() - joined,
        };
        if self.best.as_ref().is_none_or(|(best, _)| key > *best) {
            self.best = Some((key, chosen.to_vec()));
        }
    }
}

/// Which tier is taken when the copied values are in several tiers of a modifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tie {
    /// The best: the advanced copy wrote the ranges of the tier, which tell it.
    Best,
    /// Without the ranges of the copy, the worst tier that could have rolled the values on an
    /// item of `item_level` (any level when the copy doesn't write it).
    Worst { item_level: Option<u32> },
}

/// The tiers of `copied` among `tables`, on a base with `tags`: those of the modifier of
/// `affix` that holds it best, the tier taken on a `tie`.
fn find(
    tables: &[Table],
    affix: TableAffix,
    tags: &[String],
    copied: &Copied<'_>,
    tie: Tie,
) -> Option<ModifierTiers> {
    let mut best: Option<(Fit, ModifierTiers)> = None;
    for (index, table) in tables.iter().enumerate() {
        if table.affix != affix {
            continue;
        }
        let Some(order) = copied.order(&table.lines) else {
            continue;
        };
        // Each modifier once, at its first table.
        let first = tables
            .get(..index)
            .is_none_or(|before| !before.iter().any(|other| other.same_modifier(table)));
        if !first {
            continue;
        }
        let Some(tiers) = tiers_on_base(tables, table, tags, &order, copied) else {
            continue;
        };
        let values = copied.values(&table.lines, &order);
        let found = tiers
            .iter()
            .enumerate()
            .filter(|(_, tier)| match tie {
                Tie::Best | Tie::Worst { item_level: None } => true,
                Tie::Worst {
                    item_level: Some(item_level),
                } => tier.level <= item_level,
            })
            .filter_map(|(index, tier)| Some((copied.fit(&values, tier)?, index)))
            .min_by(|&(fit, index), &(other_fit, other)| {
                let tier = match tie {
                    Tie::Best => index.cmp(&other),
                    Tie::Worst { .. } => other.cmp(&index),
                };
                fit.cmp(&other_fit).then(tier)
            });
        if let Some((fit, found)) = found
            && best.as_ref().is_none_or(|(best, _)| fit < *best)
        {
            let counts = copied.lines.iter().map(CopiedLine::count).collect();
            best = Some((
                fit,
                ModifierTiers {
                    tiers,
                    found,
                    counts,
                },
            ));
        }
    }
    best.map(|(_, tiers)| tiers)
}

/// Every tier of the modifier of `table` that rolls on a base with `tags`, the best first, the
/// ranges of each in the order of the copied lines (`order`, [`Copied::order`]): a type can
/// roll other ranges on other bases (the added damage of a two-handed weapon). None when a
/// line of a tier hasn't as many values as the copied one.
fn tiers_on_base(
    tables: &[Table],
    table: &Table,
    tags: &[String],
    order: &[usize],
    copied: &Copied<'_>,
) -> Option<Vec<Tier>> {
    let mut tiers: Vec<&TableTier> = tables
        .iter()
        .filter(|other| other.same_modifier(table) && other.rolls_on(tags))
        .flat_map(|other| &other.tiers)
        .collect();
    tiers.sort_by_key(|tier| tier.level);
    tiers
        .iter()
        .rev()
        .zip(1..)
        .map(|(tier, number)| {
            Some(Tier {
                number,
                level: tier.level,
                ranges: ranges_in_order(tier, order, copied)?,
            })
        })
        .collect()
}

/// The ranges of `tier` in the order of the copied lines.
fn ranges_in_order(tier: &TableTier, order: &[usize], copied: &Copied<'_>) -> Option<Vec<Range>> {
    let mut ranges = Vec::new();
    for (line, &index) in copied.lines.iter().zip(order) {
        let line_ranges = tier.ranges.get(index)?;
        if line_ranges.len() != line.count() {
            return None;
        }
        ranges.extend(line_ranges.iter().map(|&[min, max]| Range {
            min: min.min(max),
            max: min.max(max),
        }));
    }
    Some(ranges)
}

/// How well a tier holds the copied modifier, the best first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Fit {
    /// Its values are within the ranges of the tier.
    Within,
    /// Its values are within the ranges of the tier, give or take the rounding of the game.
    Rounded,
    /// Its values are out of the ranges of the tier, which are those the advanced copy wrote:
    /// any value that left its tier, e.g. on a sanctified item (`+74(80-89) to maximum Mana`).
    Written,
}

/// A copied line of a modifier, the lines of a stat written on several lines counting as one.
#[derive(Clone)]
struct CopiedLine<'a> {
    /// The English template of its stat.
    template: &'a str,
    /// The values of its `#`, with the sign of the stat.
    values: Vec<f64>,
    /// The value its string gives, for a line without `#` (`Loads an additional bolt` is 1),
    /// as the stat stores it.
    written: Option<f64>,
}

impl CopiedLine<'_> {
    /// How many values the line has.
    fn count(&self) -> usize {
        if self.values.is_empty() {
            usize::from(self.written.is_some())
        } else {
            self.values.len()
        }
    }
}

/// The lines of a copied modifier, and the ranges of its tier when the advanced copy gave them.
struct Copied<'a> {
    lines: Vec<CopiedLine<'a>>,
    /// The ranges the advanced copy gave for each value of the lines, in their order, as the
    /// game writes them: without their sign.
    ranges: Option<Vec<Range>>,
}

impl<'a> Copied<'a> {
    fn of(modifier: &Modifier, matches: &[LineMatch<'a>]) -> Option<Self> {
        let mut lines = Vec::new();
        let mut ranges = Some(Vec::new());
        for line_match in matches {
            let stat = line_match.stat.as_ref()?;
            // A line the trade site only knows has no template of Exiled Exchange 2.
            if stat.source != StatSource::GameData {
                return None;
            }
            lines.push(CopiedLine {
                template: stat.stat.reference.as_str(),
                values: stat.values.clone(),
                written: stat.matcher_value.filter(|_| stat.values.is_empty()),
            });
            let rolls = modifier
                .lines
                .get(line_match.lines.clone())?
                .iter()
                .flat_map(|line| &line.rolls);
            let counted = rolls
                .zip(&stat.counted)
                .filter_map(|(roll, &counted)| counted.then_some(roll.range));
            for range in counted {
                match (&mut ranges, range) {
                    (Some(ranges), Some(range)) => ranges.push(range),
                    _ => ranges = None,
                }
            }
        }
        // The advanced copy writes no range for a line without number (`Loads an additional
        // bolt`): the modifier is then found by its values only, as it is when a range is
        // missing.
        let count: usize = lines.iter().map(CopiedLine::count).sum();
        let ranges = ranges.filter(|ranges| {
            lines.iter().all(|line| line.written.is_none()) && ranges.len() == count
        });
        Some(Self { lines, ranges })
    }

    /// For each copied line, the index of the line of the same template in `lines`, the lines
    /// of a table: its lines in another order are the same modifier. None when the lines
    /// differ.
    fn order(&self, lines: &[TableLine]) -> Option<Vec<usize>> {
        if lines.len() != self.lines.len() {
            return None;
        }
        let mut used = vec![false; lines.len()];
        self.lines
            .iter()
            .map(|copied| {
                let index = lines
                    .iter()
                    .zip(&used)
                    .position(|(line, &used)| !used && line.template == copied.template)?;
                *used.get_mut(index)? = true;
                Some(index)
            })
            .collect()
    }

    /// The values of the copied lines, in their order, as the table of `lines` shows them: the
    /// value a line without number gives, divided as its table line says.
    fn values(&self, lines: &[TableLine], order: &[usize]) -> Vec<f64> {
        let mut values = Vec::new();
        for (copied, &index) in self.lines.iter().zip(order) {
            values.extend(&copied.values);
            if let Some(written) = copied.written {
                let divisor = lines.get(index).map_or(1.0, |line| line.divisor);
                values.push(written / divisor);
            }
        }
        values
    }

    /// How well `tier` holds the copied `values`; none when its ranges aren't those the advanced
    /// copy gave, or when it doesn't hold them and the copy gave no range.
    fn fit(&self, values: &[f64], tier: &Tier) -> Option<Fit> {
        if values.len() != tier.ranges.len() {
            return None;
        }
        if let Some(ranges) = &self.ranges {
            let same = ranges.iter().zip(&tier.ranges).all(|(copied, range)| {
                range.magnitude().is_some_and(|range| {
                    (copied.min - range.min).abs() < ROUNDING
                        && (copied.max - range.max).abs() < ROUNDING
                })
            });
            if !same {
                return None;
            }
        }
        let within = |margin: f64| {
            values
                .iter()
                .zip(&tier.ranges)
                .all(|(&value, range)| range.min - margin <= value && value <= range.max + margin)
        };
        if within(0.0) {
            Some(Fit::Within)
        } else if within(ROUNDING) {
            Some(Fit::Rounded)
        } else {
            self.ranges.as_ref().map(|_| Fit::Written)
        }
    }
}

/// Reads one table per line; a line that can't be read is logged and left out (the tests read
/// every line of the embedded file).
fn read(text: &str) -> Vec<Table> {
    let mut tables = Vec::new();
    for (index, line) in text.lines().enumerate() {
        match serde_json::from_str::<Table>(line) {
            Ok(table) => tables.push(table),
            Err(error) => log::error!(
                "Game data, modifier table {} unreadable: {error}",
                index + 1
            ),
        }
    }
    tables
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::{CopiedItem, Item, Rarity, StatLine};

    #[test]
    fn every_table_of_the_embedded_file_is_read() {
        let text = include_str!("../../assets/game-data/modifiers.ndjson");

        assert_eq!(TABLES.len(), text.lines().count());
    }

    fn range(min: f64, max: f64) -> Range {
        Range { min, max }
    }

    /// A tier number and the ranges of its values, as the game writes them.
    type TierRanges = (u32, Vec<Range>);

    /// The tier found from the tables for each prefix and suffix of `text`, and the tier the
    /// advanced copy writes, with its ranges: (found, written). A magic item gives its
    /// `base_type`, found among the names of the trade site in the application.
    fn tiers_of(text: &str, base_type: Option<&str>) -> Vec<(Option<TierRanges>, TierRanges)> {
        let item = Item::parse(&CopiedItem::from_text(text).expect("an item")).expect("read");
        let base_type = base_type.unwrap_or(item.base_type());
        item.modifiers()
            .iter()
            .filter(|modifier| {
                matches!(modifier.kind, ModifierKind::Explicit(Some(_))) && modifier.tier.is_some()
            })
            .map(|modifier| {
                let matches = modifier_stats(item.language(), modifier, None);
                let found = modifier_tiers(item.language(), base_type, modifier, &matches)
                    .and_then(|tiers| {
                        let tier = tiers.tier()?;
                        Some((
                            tier.number,
                            tier.ranges
                                .iter()
                                .filter_map(|range| range.magnitude())
                                .collect(),
                        ))
                    });
                let written = (
                    modifier.tier.expect("a tier"),
                    modifier
                        .lines
                        .iter()
                        .flat_map(|line| &line.rolls)
                        .filter_map(|roll| roll.range)
                        .collect(),
                );
                (found, written)
            })
            .collect()
    }

    #[test]
    fn the_tables_find_the_tier_written_by_the_advanced_copy() {
        let mut found = 0;
        let mut missing = Vec::new();
        for (text, base_type) in [
            (
                include_str!("../item/test-items/en-rare-crossbow-all-modifiers.txt"),
                None,
            ),
            (
                include_str!("../item/test-items/en-rare-bow-fractured.txt"),
                None,
            ),
            (
                include_str!("../item/test-items/en-magic-mace.txt"),
                Some("Temple Maul"),
            ),
            (include_str!("../item/test-items/fr-rare-staff.txt"), None),
            (
                include_str!("../item/test-items/fr-rare-body-armour.txt"),
                None,
            ),
            (
                include_str!("../item/test-items/en-owner-rare-sceptre-sanctified.txt"),
                None,
            ),
            (
                include_str!("../item/test-items/fr-rare-sceptre-sanctified.txt"),
                None,
            ),
        ] {
            for (tier, (number, ranges)) in tiers_of(text, base_type) {
                // The crossbow was copied before a patch added a tier above its leech, `of
                // the Desperate` (8–8.9), the S1 of the offers checked on the trade site on
                // 2026-10-27.
                let number = if ranges == [range(7.0, 7.9)] {
                    2
                } else {
                    number
                };
                if let Some(tier) = tier {
                    assert_eq!(tier, (number, ranges));
                    found += 1;
                } else {
                    missing.push(ranges);
                }
            }
        }
        assert_eq!(found, 31);
        // A line whose stat the tables have under another tier is a mistake; a line the tables
        // don't have isn't: the lightning damage of the mace, copied before a patch changed
        // its ranges, and the two modifiers of Amanamu, a lord of the Abyss, desecrated with
        // ranges of their own.
        assert_eq!(
            missing,
            [
                vec![range(1.0, 4.0), range(46.0, 66.0)],
                vec![range(40.0, 50.0)],
                vec![range(13.0, 17.0)],
            ]
        );
    }

    #[test]
    fn a_sanctified_value_out_of_its_tier_finds_it_by_the_copied_ranges() {
        for text in [
            include_str!("../item/test-items/en-owner-rare-sceptre-sanctified.txt"),
            include_str!("../item/test-items/fr-rare-sceptre-sanctified.txt"),
        ] {
            let found: Vec<_> = tiers_of(text, None)
                .into_iter()
                .filter_map(|(found, _)| found)
                .collect();
            // `+74(80-89) to maximum Mana` and `29(30)% reduced Attribute Requirements`.
            assert!(found.contains(&(5, vec![range(80.0, 89.0)])), "{found:?}");
            assert!(found.contains(&(2, vec![range(30.0, 30.0)])), "{found:?}");
        }

        // A line of two values, one of them out of its tier: `4(1-4) to 75(60-71)`.
        let text = include_str!("../item/test-items/en-owner-rare-sceptre-sanctified.txt")
            .replace("60(60-71)", "75(60-71)");
        let (found, written) = tiers_of(&text, None).swap_remove(0);
        assert_eq!(written, (1, vec![range(1.0, 4.0), range(60.0, 71.0)]));
        assert_eq!(found, Some(written));

        // Without the ranges of the advanced copy, a value out of every tier finds none.
        let mana = modifier(Some(Affix::Prefix), &["+74 to maximum Mana"]);
        assert_eq!(
            tiers("Rattling Sceptre", &mana).and_then(|tiers| Some(tiers.tier()?.number)),
            Some(6),
            "74 is in the range of tier 6"
        );
        let mana = modifier(Some(Affix::Prefix), &["+400 to maximum Mana"]);
        assert_eq!(tiers("Rattling Sceptre", &mana), None);
    }

    /// A modifier of `affix` with `lines`, without the ranges of the advanced copy: found by
    /// the values the stats read in the text.
    fn modifier(affix: Option<Affix>, lines: &[&str]) -> Modifier {
        Modifier {
            kind: ModifierKind::Explicit(affix),
            origin: None,
            name: None,
            tier: None,
            tags: Vec::new(),
            unrevealed: false,
            lines: lines
                .iter()
                .map(|text| StatLine {
                    text: (*text).to_owned(),
                    rolls: Vec::new(),
                })
                .collect(),
        }
    }

    /// The tiers of `modifier` on `base_type`, in English.
    fn tiers(base_type: &str, modifier: &Modifier) -> Option<ModifierTiers> {
        let matches = modifier_stats(Language::English, modifier, None);
        modifier_tiers(Language::English, base_type, modifier, &matches)
    }

    #[test]
    fn a_tier_is_numbered_among_the_tiers_of_the_base() {
        let lightning = modifier(Some(Affix::Prefix), &["Adds 10 to 273 Lightning Damage"]);

        let tiers = tiers("Gemini Crossbow", &lightning).expect("its tiers");

        // The tiers of a two-handed weapon, not those of a one-handed one.
        assert_eq!(tiers.tiers.len(), 10);
        assert_eq!(
            tiers.tiers.first().map(|tier| tier.ranges.clone()),
            Some(vec![range(1.0, 19.0), range(310.0, 358.0)])
        );
        assert_eq!(tiers.tiers.last().map(|tier| tier.level), Some(1));
        assert_eq!(tiers.tier().map(|tier| tier.number), Some(2));
    }

    #[test]
    fn a_line_written_without_number_finds_its_tier_by_the_value_of_its_string() {
        let number_of = |line: &str| {
            let bolts = modifier(Some(Affix::Suffix), &[line]);
            tiers("Gemini Crossbow", &bolts).and_then(|tiers| Some(tiers.tier()?.number))
        };

        assert_eq!(number_of("Loads an additional bolt"), Some(2));
        assert_eq!(number_of("Loads 2 additional bolts"), Some(1));
        assert_eq!(number_of("Loads 3 additional bolts"), None);
    }

    #[test]
    fn the_lines_of_a_hybrid_in_another_order_are_the_same_modifier() {
        let champion = modifier(
            Some(Affix::Prefix),
            &["+100 to Accuracy Rating", "50% increased Physical Damage"],
        );

        let tiers = tiers("Gemini Crossbow", &champion).expect("its tiers");

        let tier = tiers.tier().expect("its tier");
        assert_eq!(tier.number, 4);
        // In the order of the copied lines.
        assert_eq!(tier.ranges, [range(98.0, 123.0), range(45.0, 54.0)]);
    }

    #[test]
    fn a_modifier_outside_the_prefixes_and_suffixes_has_no_tiers() {
        let mut implicit = modifier(None, &["Loads an additional bolt"]);
        implicit.kind = ModifierKind::Implicit;
        // A header the parser doesn't know tells no affix.
        let unknown = modifier(None, &["Adds 10 to 273 Lightning Damage"]);
        let mut desecrated = modifier(Some(Affix::Prefix), &["Adds 10 to 273 Lightning Damage"]);
        desecrated.origin = Some(ModifierOrigin::Desecrated);

        assert_eq!(tiers("Gemini Crossbow", &implicit), None);
        assert_eq!(tiers("Gemini Crossbow", &unknown), None);
        // A base the data doesn't know.
        assert_eq!(tiers("Unknown Crossbow", &desecrated), None);
        // An ordinary modifier desecrated is in the tables.
        assert!(tiers("Gemini Crossbow", &desecrated).is_some());
    }

    #[test]
    fn the_value_of_a_line_without_number_is_divided_as_its_table_shows_it() {
        let tables = read(concat!(
            r#"{"type": "Rage", "affix": "suffix", "lines": [{"template": "Inherent Rage loss starts 1 second later", "divisor": 1000}], "#,
            r#""spawn": [["weapon", 1]], "tiers": [{"level": 1, "ranges": [[[1, 1]]]}, {"level": 50, "ranges": [[[2, 3]]]}]}"#
        ));
        let copied = |values: Vec<f64>, written| Copied {
            lines: vec![CopiedLine {
                template: "Inherent Rage loss starts 1 second later",
                values,
                written,
            }],
            ranges: None,
        };
        let number = |copied| {
            find(
                &tables,
                TableAffix::Suffix,
                &["weapon".to_owned()],
                &copied,
                Tie::Best,
            )
            .and_then(|tiers| Some(tiers.tier()?.number))
        };

        // `Inherent Rage loss starts 1 second later` gives the 1000 milliseconds of the stat.
        assert_eq!(number(copied(Vec::new(), Some(1000.0))), Some(2));
        assert_eq!(number(copied(vec![2.5], None)), Some(1));
    }

    #[test]
    fn a_line_without_value_is_found_with_the_other_lines_of_its_modifier() {
        let instant = modifier(
            Some(Affix::Prefix),
            &["Instant Recovery", "50% reduced Amount Recovered"],
        );

        let tiers = tiers("Ultimate Life Flask", &instant).expect("its tiers");

        assert_eq!(tiers.tier().map(|tier| tier.number), Some(1));
        assert_eq!(
            tiers.tier().map(|tier| tier.ranges.clone()),
            Some(vec![range(-50.0, -50.0)])
        );
    }

    #[test]
    fn the_modifiers_of_the_time_lost_jewels_arent_in_the_tables_yet() {
        // Written within "Small Passive Skills in Radius also grant…" by the game, step 13k.
        let ailments = modifier(
            Some(Affix::Suffix),
            &["5% increased chance to inflict Ailments"],
        );

        assert_eq!(tiers("Time-Lost Emerald", &ailments), None);
    }

    #[test]
    fn two_tables_with_the_same_lines_in_another_order_are_two_modifiers() {
        let table = |kind: &str, lines: [&str; 2], level: u32| {
            format!(
                r#"{{"type": "{kind}", "affix": "prefix", "lines": [{{"template": "{}"}}, {{"template": "{}"}}], "spawn": [["weapon", 1]], "tiers": [{{"level": {level}, "ranges": [[[1, 5]], [[10, 20]]]}}]}}"#,
                lines[0], lines[1]
            )
        };
        let tables = read(
            &[
                table("Hybrid", ["A #", "B #"], 1),
                table("Hybrid", ["B #", "A #"], 50),
            ]
            .join("\n"),
        );
        let copied = Copied {
            lines: vec![
                CopiedLine {
                    template: "B #",
                    values: vec![15.0],
                    written: None,
                },
                CopiedLine {
                    template: "A #",
                    values: vec![3.0],
                    written: None,
                },
            ],
            ranges: None,
        };

        let tiers = find(
            &tables,
            TableAffix::Prefix,
            &["weapon".to_owned()],
            &copied,
            Tie::Best,
        )
        .expect("its tiers");

        // The first table holds the copied values, its lines put in the copied order; the
        // second, its own lines in the copied order, holds neither 15 in 1–5 nor 3 in 10–20.
        assert_eq!(tiers.tiers.len(), 1);
        assert_eq!(tiers.tiers.first().map(|tier| tier.level), Some(1));
        assert_eq!(
            tiers.tier().map(|tier| tier.ranges.clone()),
            Some(vec![range(10.0, 20.0), range(1.0, 5.0)])
        );
    }

    #[test]
    fn a_line_with_a_divisor_has_one_value() {
        // The divisor is for the value of a line written without number, a single value.
        for table in TABLES.iter() {
            for (index, line) in table.lines.iter().enumerate() {
                if (line.divisor - 1.0).abs() > f64::EPSILON {
                    for tier in &table.tiers {
                        assert_eq!(
                            tier.ranges.get(index).map(Vec::len),
                            Some(1),
                            "{}",
                            line.template
                        );
                    }
                }
            }
        }
    }

    /// The tiara and the bow copied by the owner from the in-game trade, English and French.
    const TIARA_EN: &str =
        include_str!("../item/test-items/en-owner-trade-rare-helmet-fractured.txt");
    const TIARA_FR: &str = include_str!("../item/test-items/fr-trade-rare-helmet-fractured.txt");
    const BOW_EN: &str = include_str!("../item/test-items/en-owner-trade-rare-bow-mirrored.txt");
    const BOW_FR: &str = include_str!("../item/test-items/fr-trade-rare-bow-mirrored.txt");

    /// The tier number of each affix a modifier can be.
    type AffixTiers = Vec<(Affix, u32)>;

    fn parse(text: &str) -> Item {
        Item::parse(&CopiedItem::from_text(text).expect("an item")).expect("read")
    }

    /// The tiers of `modifiers` found from their values on the base of `item`, with at most
    /// `limits` prefixes and suffixes.
    fn found_tiers(
        item: &Item,
        modifiers: &[Modifier],
        (prefixes, suffixes): (u32, u32),
    ) -> Vec<FoundTiers> {
        let limits = AffixLimits { prefixes, suffixes };
        trade_tiers(
            item.language(),
            item.base_type(),
            item.item_level(),
            modifiers,
            limits,
        )
    }

    /// For each modifier read from its values, the texts of its lines and the tier number of
    /// each affix it can be.
    fn lines_read(modifiers: &[Modifier], found: &[FoundTiers]) -> Vec<(Vec<String>, AffixTiers)> {
        found
            .iter()
            .map(|found| {
                let lines = modifiers[found.modifiers.clone()]
                    .iter()
                    .flat_map(|modifier| &modifier.lines)
                    .map(|line| line.text.clone())
                    .collect();
                let affixes = found
                    .affixes
                    .iter()
                    .map(|(affix, tiers)| (*affix, tiers.tier().expect("its tier").number))
                    .collect();
                (lines, affixes)
            })
            .collect()
    }

    /// For each modifier read from its values, how many lines it has and the tier number of
    /// each affix it can be: the same in every language.
    fn summary(modifiers: &[Modifier], found: &[FoundTiers]) -> Vec<(usize, AffixTiers)> {
        lines_read(modifiers, found)
            .into_iter()
            .map(|(lines, affixes)| (lines.len(), affixes))
            .collect()
    }

    #[test]
    fn the_tiers_of_a_tiara_copied_from_the_in_game_trade_are_found_from_its_values() {
        use Affix::{Prefix, Suffix};
        for text in [TIARA_EN, TIARA_FR] {
            let item = parse(text);

            let found = found_tiers(&item, item.modifiers(), (3, 3));

            assert_eq!(
                summary(item.modifiers(), &found),
                [
                    // The fractured `41% increased Energy Shield` and `+34 to maximum Mana`,
                    // one hybrid: read apart, the tiara would have four prefixes.
                    (2, vec![(Prefix, 1)]),
                    (1, vec![(Prefix, 1)]),
                    // `18% increased Rarity of Items found` rolls as a prefix and as a suffix:
                    // the prefixes are full.
                    (1, vec![(Suffix, 1)]),
                    (1, vec![(Suffix, 1)]),
                    // `+40% to Cold Resistance`.
                    (1, vec![(Suffix, 2)]),
                    // The desecrated `+69 to maximum Energy Shield`.
                    (1, vec![(Prefix, 1)]),
                ]
            );
            let rarity = found.get(2).expect("the rarity");
            assert_eq!(rarity.ruled_out, Some(Prefix), "the prefixes are full");
            let hybrid = found.first().and_then(|found| found.affixes.first());
            assert_eq!(
                hybrid.and_then(|(_, tiers)| Some(tiers.tier()?.ranges.clone())),
                Some(vec![range(39.0, 42.0), range(33.0, 39.0)]),
                "in the order of the copied lines"
            );
        }
    }

    #[test]
    fn an_affix_stays_unknown_while_the_item_has_room_for_both() {
        let item = parse(TIARA_EN);
        // Without the desecrated `+69 to maximum Energy Shield`, a prefix is open.
        let modifiers = &item.modifiers()[..item.modifiers().len() - 1];

        let found = found_tiers(&item, modifiers, (3, 3));

        let rarity = found
            .iter()
            .find(|found| {
                modifiers[found.modifiers.clone()][0].lines[0]
                    .text
                    .contains("Rarity")
            })
            .expect("the rarity");
        let tiers: Vec<_> = rarity
            .affixes
            .iter()
            .map(|(affix, tiers)| {
                let tier = tiers.tier().expect("its tier");
                (*affix, tier.number, tier.ranges.clone())
            })
            .collect();
        // As the mock-up shows it: "T1 16–19 Prefix", "T1 15–18 Suffix".
        assert_eq!(
            tiers,
            [
                (Affix::Prefix, 1, vec![range(16.0, 19.0)]),
                (Affix::Suffix, 1, vec![range(15.0, 18.0)]),
            ]
        );
    }

    #[test]
    fn a_modifier_whose_header_tells_its_affix_takes_its_room() {
        let item = parse(TIARA_EN);
        let mut modifiers = item.modifiers().to_vec();
        // A desecrated prefix not revealed yet in place of `+69 to maximum Energy Shield`.
        let mut unrevealed = modifier(Some(Affix::Prefix), &[]);
        unrevealed.unrevealed = true;
        *modifiers.last_mut().expect("a modifier") = unrevealed;

        let found = found_tiers(&item, &modifiers, (3, 3));

        let rarity = lines_read(&modifiers, &found)
            .into_iter()
            .find(|(lines, _)| lines == &["18% increased Rarity of Items found"]);
        assert_eq!(
            rarity.map(|(_, affixes)| affixes),
            Some(vec![(Affix::Suffix, 1)])
        );
    }

    #[test]
    fn the_lines_of_two_origins_arent_read_as_one_modifier() {
        let item = parse(TIARA_EN);
        let mut modifiers = item.modifiers().to_vec();
        let mana = modifiers
            .iter_mut()
            .find(|modifier| modifier.lines[0].text == "+34 to maximum Mana")
            .expect("the mana");
        mana.origin = None;

        let found = found_tiers(&item, &modifiers, (3, 3));

        let read = lines_read(&modifiers, &found);
        assert_eq!(read[0].0, ["41% increased Energy Shield"]);
        assert_eq!(read[1].0, ["+34 to maximum Mana"]);
    }

    #[test]
    fn without_a_reading_that_fits_the_room_every_affix_is_kept() {
        use Affix::{Prefix, Suffix};
        let item = parse(TIARA_EN);

        // One prefix and one suffix, as on a magic item: no reading fits.
        let found = found_tiers(&item, item.modifiers(), (1, 1));

        assert_eq!(
            summary(item.modifiers(), &found),
            [
                (2, vec![(Prefix, 1)]),
                (1, vec![(Prefix, 1)]),
                (1, vec![(Prefix, 1), (Suffix, 1)]),
                (1, vec![(Suffix, 1)]),
                (1, vec![(Suffix, 2)]),
                (1, vec![(Prefix, 1)]),
            ]
        );
    }

    #[test]
    fn the_tiers_of_a_bow_copied_from_the_in_game_trade_are_found_from_its_values() {
        use Affix::{Prefix, Suffix};
        let item = parse(BOW_EN);

        // Its rune allows a fourth suffix.
        let found = found_tiers(&item, item.modifiers(), (3, 4));

        let read = lines_read(item.modifiers(), &found);
        let line = |text: &str| vec![text.to_owned()];
        assert_eq!(
            read,
            [
                (line("+5% to Critical Hit Chance"), vec![(Suffix, 1)]),
                // `258% increased Physical Damage` adds up two modifiers: no tier holds it
                // (step 13k2e4). The accuracy is the other line of one of them, a hybrid,
                // read alone until then.
                (line("+195 to Accuracy Rating"), vec![(Prefix, 5)]),
                (line("+25% to Critical Damage Bonus"), vec![(Suffix, 1)]),
                (
                    line("+4 to Level of all Projectile Skills"),
                    vec![(Suffix, 1)]
                ),
                (
                    line("+181% Surpassing chance to fire an additional Arrow"),
                    vec![(Suffix, 1)]
                ),
                (line("Adds 36 to 56 Physical Damage"), vec![(Prefix, 1)]),
            ]
        );

        let item = parse(BOW_FR);
        let found = found_tiers(&item, item.modifiers(), (3, 4));
        assert_eq!(
            summary(item.modifiers(), &found),
            [
                (1, vec![(Suffix, 1)]),
                (1, vec![(Prefix, 1)]),
                (1, vec![(Prefix, 1)]),
                (1, vec![(Suffix, 1)]),
                (1, vec![(Suffix, 1)]),
                (1, vec![(Suffix, 1)]),
                // The desecrated `79% d'Augmentation des Dégâts Physiques` and `+193 au Score
                // de Précision`, one hybrid.
                (2, vec![(Prefix, 1)]),
            ]
        );
    }

    /// The rare and magic items of the advanced copy, with the base type of a magic one.
    const ADVANCED_COPIES: [(&str, Option<&str>); 11] = [
        (
            include_str!("../item/test-items/en-rare-crossbow-all-modifiers.txt"),
            None,
        ),
        (
            include_str!("../item/test-items/en-rare-bow-fractured.txt"),
            None,
        ),
        (
            include_str!("../item/test-items/en-magic-mace.txt"),
            Some("Temple Maul"),
        ),
        (include_str!("../item/test-items/fr-rare-staff.txt"), None),
        (
            include_str!("../item/test-items/fr-rare-body-armour.txt"),
            None,
        ),
        (
            include_str!("../item/test-items/en-owner-rare-sceptre-sanctified.txt"),
            None,
        ),
        (
            include_str!("../item/test-items/fr-rare-sceptre-sanctified.txt"),
            None,
        ),
        (
            include_str!("../item/test-items/en-owner-rare-helmet-runeforged.txt"),
            None,
        ),
        (
            include_str!("../item/test-items/fr-rare-helmet-runeforged.txt"),
            None,
        ),
        (
            include_str!("../item/test-items/en-owner-rare-focus-unrevealed.txt"),
            None,
        ),
        (
            include_str!("../item/test-items/fr-rare-focus-unrevealed.txt"),
            None,
        ),
    ];

    /// A modifier of the advanced copy written as the in-game trade writes it.
    struct Written {
        /// The indexes of the modifiers its lines make, one per stat.
        modifiers: std::ops::Range<usize>,
        affix: Affix,
        /// Its tier, found by the ranges of the copy.
        tier: Option<u32>,
        /// Whether its values are within these ranges: not a sanctified value out of its tier.
        within: bool,
    }

    /// The explicit modifiers of an advanced copy as the in-game trade writes them, one per
    /// stat without header nor range, and what the copy wrote of each.
    fn as_in_game_trade(item: &Item, base_type: &str) -> (Vec<Modifier>, Vec<Written>) {
        let mut modifiers = Vec::new();
        let mut written = Vec::new();
        for modifier in item.modifiers() {
            let ModifierKind::Explicit(Some(affix)) = modifier.kind else {
                modifiers.push(modifier.clone());
                continue;
            };
            if modifier.unrevealed {
                modifiers.push(modifier.clone());
                continue;
            }
            let matches = modifier_stats(item.language(), modifier, None);
            let tier = modifier_tiers(item.language(), base_type, modifier, &matches)
                .and_then(|tiers| Some(tiers.tier()?.number));
            let rolls = modifier.lines.iter().flat_map(|line| &line.rolls);
            let within = rolls
                .filter_map(|roll| Some((roll.value, roll.range?)))
                .all(|(value, range)| range.min <= value && value <= range.max);
            let start = modifiers.len();
            for line_match in &matches {
                let mut lines = modifier.lines[line_match.lines.clone()].to_vec();
                for roll in lines.iter_mut().flat_map(|line| &mut line.rolls) {
                    roll.range = None;
                }
                modifiers.push(Modifier {
                    kind: ModifierKind::Explicit(None),
                    origin: modifier.origin,
                    name: None,
                    tier: None,
                    tags: Vec::new(),
                    unrevealed: false,
                    lines,
                });
            }
            written.push(Written {
                modifiers: start..modifiers.len(),
                affix,
                tier,
                within,
            });
        }
        (modifiers, written)
    }

    #[test]
    fn the_tiers_found_from_the_values_alone_are_those_of_the_advanced_copy() {
        let mut found = 0;
        let mut without_tier = Vec::new();
        let mut unknown_affix = Vec::new();
        for (text, base_type) in ADVANCED_COPIES {
            let item = parse(text);
            let base_type = base_type.unwrap_or(item.base_type());
            let limit = if item.rarity() == Rarity::Magic { 1 } else { 3 };
            let limits = AffixLimits {
                prefixes: limit,
                suffixes: limit,
            };
            let (modifiers, written) = as_in_game_trade(&item, base_type);

            let read = trade_tiers(
                item.language(),
                base_type,
                item.item_level(),
                &modifiers,
                limits,
            );

            for Written {
                modifiers: lines,
                affix,
                tier,
                within,
            } in written
            {
                let text = modifiers[lines.start].lines[0].text.clone();
                let Some(read) = read
                    .iter()
                    .find(|read| read.modifiers.contains(&lines.start))
                else {
                    without_tier.push(text);
                    continue;
                };
                // Never a hybrid made of the lines of two modifiers, nor one cut in two.
                assert_eq!(read.modifiers, lines, "{text}");
                let tiers: Vec<_> = read
                    .affixes
                    .iter()
                    .map(|(affix, tiers)| (*affix, tiers.tier().map(|tier| tier.number)))
                    .collect();
                let same = tiers
                    .iter()
                    .any(|&(read, number)| read == affix && (!within || number == tier));
                assert!(same, "{text}: {tiers:?}, written {affix:?} {tier:?}");
                if tiers.len() > 1 {
                    unknown_affix.push(text);
                } else {
                    found += 1;
                }
            }
        }
        assert_eq!(found, 43);
        assert_eq!(
            without_tier,
            [
                // Two modifiers of an essence, without tier in the advanced copy too.
                "20% chance to gain Onslaught on Killing Hits with this Weapon",
                "+3 to Level of all Attack Skills",
                // Copied before a patch changed its ranges.
                "Adds 1 to 50 Lightning Damage",
                // Modifiers of Amanamu, a lord of the Abyss, outside the tables.
                "Rajoute 41% des Dégâts sous forme de Dégâts de Chaos",
                "+14% aux Résistances au feu et au chaos",
                // A sanctified value out of its tier, 29(30)%.
                "29% reduced Attribute Requirements",
                "29% de Réduction du prérequis d'Attributs",
                // Amanamu again.
                "35% increased Presence Area of Effect",
                "Votre Présence a 35% d'Augmentation de Zone d'effet",
            ]
        );
        // The rarity of the runeforged crown rolls as both, the line of Amanamu after it
        // unknown: the crown seems to have room for a third prefix.
        assert_eq!(
            unknown_affix,
            [
                "15% increased Rarity of Items found",
                "15% d'Augmentation de la Rareté des Objets cédés",
            ]
        );
    }

    #[test]
    fn a_value_in_several_tiers_takes_the_worst_without_the_ranges_of_the_copy() {
        let tables = read(concat!(
            r##"{"type": "Life", "affix": "prefix", "lines": [{"template": "# to maximum Life"}], "##,
            r#""spawn": [["helmet", 1]], "tiers": [{"level": 1, "ranges": [[[1, 10]]]}, {"level": 50, "ranges": [[[5, 15]]]}]}"#
        ));
        let copied = Copied {
            lines: vec![CopiedLine {
                template: "# to maximum Life",
                values: vec![7.0],
                written: None,
            }],
            ranges: None,
        };
        let number = |tie| {
            find(
                &tables,
                TableAffix::Prefix,
                &["helmet".to_owned()],
                &copied,
                tie,
            )
            .and_then(|tiers| Some(tiers.tier()?.number))
        };

        assert_eq!(number(Tie::Best), Some(1));
        assert_eq!(number(Tie::Worst { item_level: None }), Some(2));
    }

    #[test]
    fn a_base_the_data_doesnt_know_has_no_tiers_found() {
        let item = parse(TIARA_EN);
        let limits = AffixLimits {
            prefixes: 3,
            suffixes: 3,
        };

        let found = trade_tiers(
            Language::English,
            "Unknown Tiara",
            Some(82),
            item.modifiers(),
            limits,
        );

        assert_eq!(found, []);
    }

    #[test]
    fn many_lines_that_pair_as_hybrids_are_read_within_a_bound() {
        let item = parse(TIARA_EN);
        // Each line makes a hybrid with the one before and the one after: more readings than
        // the bound.
        let modifiers: Vec<_> = (0..20)
            .map(|index| {
                let line = if index % 2 == 0 {
                    "41% increased Energy Shield"
                } else {
                    "+34 to maximum Mana"
                };
                modifier(None, &[line])
            })
            .collect();

        let found = found_tiers(&item, &modifiers, (10, 0));

        // The readings that fit, ten hybrids, are past the bound: the lines are read alone,
        // the pure energy shield T6 and mana T8, every affix kept.
        let alone = [(1, vec![(Affix::Prefix, 6)]), (1, vec![(Affix::Prefix, 8)])];
        let expected: Vec<_> = alone.iter().cycle().take(20).cloned().collect();
        assert_eq!(summary(&modifiers, &found), expected);
    }

    /// The lines of two pure prefixes of a helmet that also fit the hybrid of both, T3
    /// (`27–32 · 26–32`).
    fn energy_shield_and_life() -> Vec<Modifier> {
        vec![
            modifier(None, &["30% increased Energy Shield"]),
            modifier(None, &["+30 to maximum Life"]),
        ]
    }

    #[test]
    fn lines_that_fit_a_hybrid_are_read_alone_while_the_item_has_room() {
        use Affix::Prefix;
        let item = parse(TIARA_EN);
        let modifiers = energy_shield_and_life();

        let found = found_tiers(&item, &modifiers, (3, 3));
        assert_eq!(
            summary(&modifiers, &found),
            [(1, vec![(Prefix, 6)]), (1, vec![(Prefix, 8)])]
        );

        // With room for one prefix only, the hybrid.
        let found = found_tiers(&item, &modifiers, (1, 3));
        assert_eq!(summary(&modifiers, &found), [(2, vec![(Prefix, 3)])]);
    }

    #[test]
    fn a_line_only_a_hybrid_holds_is_read_in_it() {
        let item = parse(BOW_EN);
        // The pure `#% increased Physical Damage` starts at 40%, the hybrid with the accuracy
        // at 15%: read alone, the physical damage would have no tier.
        let modifiers = [
            modifier(None, &["30% increased Physical Damage"]),
            modifier(None, &["+60 to Accuracy Rating"]),
        ];

        let found = found_tiers(&item, &modifiers, (3, 3));

        assert_eq!(summary(&modifiers, &found), [(2, vec![(Affix::Prefix, 6)])]);
    }

    #[test]
    fn two_fractured_lines_are_one_hybrid_even_when_the_item_has_room() {
        let item = parse(TIARA_EN);

        // Room for four prefixes: the two fractured lines read alone would fit it, but an item
        // has one fractured modifier only.
        let found = found_tiers(&item, item.modifiers(), (4, 3));

        let first = found.first().map(|found| found.modifiers.len());
        assert_eq!(first, Some(2));
    }

    #[test]
    fn a_tier_above_the_item_level_isnt_found() {
        let modifiers = energy_shield_and_life();
        let limits = AffixLimits {
            prefixes: 1,
            suffixes: 3,
        };
        let read = |item_level| {
            let found = trade_tiers(
                Language::English,
                "Ancestral Tiara",
                Some(item_level),
                &modifiers,
                limits,
            );
            summary(&modifiers, &found)
        };

        // The hybrid T3 needs the level 46: on an item of level 40, the lines are two
        // prefixes, more than its room, every affix kept.
        assert_eq!(read(46), [(2, vec![(Affix::Prefix, 3)])]);
        assert_eq!(
            read(40),
            [(1, vec![(Affix::Prefix, 6)]), (1, vec![(Affix::Prefix, 8)])]
        );

        // The prefix of rarity `16–19` needs the level 47, the suffix `15–18` the level 40.
        let rarity = [modifier(None, &["18% increased Rarity of Items found"])];
        let affixes = |item_level| {
            let found = trade_tiers(
                Language::English,
                "Ancestral Tiara",
                Some(item_level),
                &rarity,
                limits,
            );
            summary(&rarity, &found)
        };
        assert_eq!(
            affixes(47),
            [(1, vec![(Affix::Prefix, 1), (Affix::Suffix, 1)])]
        );
        assert_eq!(affixes(45), [(1, vec![(Affix::Suffix, 1)])]);
    }

    /// A candidate of the modifiers `modifiers` that can be `affixes`, without tiers.
    fn candidate(
        modifiers: std::ops::Range<usize>,
        affixes: &[Affix],
        fractured: bool,
    ) -> Candidate {
        let tiers = ModifierTiers {
            tiers: Vec::new(),
            found: 0,
            counts: Vec::new(),
        };
        Candidate {
            modifiers,
            fractured,
            affixes: affixes
                .iter()
                .map(|&affix| (affix, tiers.clone()))
                .collect(),
        }
    }

    #[test]
    fn the_room_tells_the_affix_of_the_modifiers_that_can_be_both() {
        use Affix::{Prefix, Suffix};
        let candidates = [
            candidate(0..1, &[Prefix], false),
            candidate(1..2, &[Prefix, Suffix], false),
            candidate(2..3, &[Prefix, Suffix], false),
            candidate(3..4, &[Suffix], false),
        ];
        let all = [0, 1, 2, 3];
        let room = |prefixes, suffixes| Room { prefixes, suffixes };

        // One more prefix and one more suffix: one of the two is a prefix, either.
        assert_eq!(room(2, 2).bounds(&all, &candidates), Some((1, 1)));
        assert_eq!(room(2, 2).both(&all, &candidates), (true, true));
        // No room for another suffix: both are prefixes.
        assert_eq!(room(3, 1).bounds(&all, &candidates), Some((2, 2)));
        assert_eq!(room(3, 1).both(&all, &candidates), (true, false));
        // No room for another prefix: both are suffixes.
        assert_eq!(room(1, 3).both(&all, &candidates), (false, true));
        // Too few: none fits, every affix kept.
        assert_eq!(room(1, 2).bounds(&all, &candidates), None);
        assert_eq!(room(1, 2).both(&all, &candidates), (true, true));
        assert_eq!(room(0, 5).bounds(&all, &candidates), None);
        assert_eq!(room(0, 0).bounds(&[], &candidates), Some((0, 0)));
    }

    #[test]
    fn the_first_reading_of_the_most_modifiers_that_fits_is_taken() {
        use Affix::Prefix;
        let candidates = [
            candidate(0..1, &[Prefix], false),
            candidate(0..2, &[Prefix], false),
            candidate(1..2, &[Prefix], false),
            candidate(1..3, &[Prefix], false),
            candidate(2..3, &[Prefix], false),
        ];
        let room = Room {
            prefixes: 2,
            suffixes: 0,
        };
        let mut search = Search::new(&candidates, 3, room, 0);

        search.walk(0, &mut Vec::new());

        // The three lines alone make three prefixes; then `A, BC` and `AB, C`, the first one
        // looked at taken.
        assert_eq!(search.readings, 3);
        assert_eq!(search.best.map(|(_, reading)| reading), Some(vec![0, 3]));
    }

    #[test]
    fn the_readings_looked_at_are_bounded() {
        use Affix::Prefix;
        let count = 30;
        let candidates: Vec<_> = (0..count)
            .flat_map(|start| {
                let pair =
                    (start + 2 <= count).then(|| candidate(start..start + 2, &[Prefix], false));
                [Some(candidate(start..start + 1, &[Prefix], false)), pair]
            })
            .flatten()
            .collect();
        let room = Room {
            prefixes: 0,
            suffixes: 0,
        };
        let mut search = Search::new(&candidates, count, room, 0);

        search.walk(0, &mut Vec::new());

        assert_eq!(search.readings, MAX_READINGS);
        // The lines alone, looked at first.
        assert_eq!(search.best.map(|(key, _)| key.modifiers), Some(count));
    }

    #[test]
    fn a_modifier_without_line_isnt_read() {
        let item = parse(TIARA_EN);
        // Its lines would be those of the line before: the fire resistance alone.
        let modifiers = [
            modifier(None, &["+43% to Fire Resistance"]),
            modifier(None, &[]),
        ];

        let found = found_tiers(&item, &modifiers, (3, 3));

        assert_eq!(found.len(), 1);
        assert_eq!(
            found.first().map(|found| found.modifiers.clone()),
            Some(0..1)
        );
    }
}
