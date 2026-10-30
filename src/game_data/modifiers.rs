//! The modifier tables, from RePoE (`assets/game-data/modifiers.ndjson`): every tier of a
//! prefix or suffix on the base of an item, found from the stats of its lines, or from their
//! values alone for an item copied from the in-game trade.

use std::sync::LazyLock;

use serde::Deserialize;

use super::bases;
use super::lines::{LineMatch, StatSource, modifier_stats};
use crate::item::{
    Affix, AffixTier, Modifier, ModifierKind, ModifierOrigin, PartTier, Range, TierDoubt,
};
use crate::language::Language;

mod summed_readings;

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

/// The prefix first, as the two pure modifiers of a split line are read.
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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

impl From<TableAffix> for Affix {
    fn from(affix: TableAffix) -> Self {
        match affix {
            TableAffix::Prefix => Self::Prefix,
            TableAffix::Suffix => Self::Suffix,
        }
    }
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
    /// The index in `tiers` of the tier of the copied modifier; for a modifier in two
    /// neighbouring tiers, the worse of both: [`Self::possible`] gives both.
    pub found: usize,
    /// The index in `tiers` of the tier above `found` (always `found - 1`) the copied modifier
    /// may be as well: a part of a line that adds up two modifiers, in either of two
    /// neighbouring tiers while the other part is sure (the pure T1 or T2 of the `248%
    /// increased Physical Damage` of a bow, with the hybrid T1 on `+195 to Accuracy Rating`);
    /// none otherwise.
    pub neighbour: Option<usize>,
    /// The number of values of each line match given, in their order: the ranges of a tier
    /// follow them (two for `Adds # to # Physical Damage`; for a line without number, one
    /// when its string gives the value, `Loads an additional bolt`, none otherwise).
    pub counts: Vec<usize>,
}

impl ModifierTiers {
    /// The tier of the copied modifier.
    #[must_use]
    pub fn tier(&self) -> Option<&Tier> {
        self.tiers.get(self.found)
    }

    /// The tiers the copied modifier may be, the best first: the neighbouring one, then its
    /// tier.
    pub fn possible(&self) -> impl Iterator<Item = &Tier> {
        let neighbour = self.neighbour.and_then(|index| self.tiers.get(index));
        neighbour.into_iter().chain(self.tier())
    }

    /// These tiers, the copied modifier in its tier or in `other`, a neighbouring one: the
    /// worse of both found, the better its neighbour.
    fn or_tier(&self, other: usize) -> Self {
        Self {
            found: self.found.max(other),
            neighbour: Some(self.found.min(other)),
            ..self.clone()
        }
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
/// For a line that adds up two modifiers ([`TierDoubt::Split`]), the tiers of the modifier of
/// every line ([`split_tiers`]).
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
    if let Some(TierDoubt::Split { line, whole, .. }) = &modifier.doubt {
        return split_tiers(language, base_type, modifier, matches, (*line, whole));
    }
    let affix = match modifier.kind {
        ModifierKind::Explicit(Some(Affix::Prefix)) => TableAffix::Prefix,
        ModifierKind::Explicit(Some(Affix::Suffix)) => TableAffix::Suffix,
        _ => return None,
    };
    let tags = bases::tags(language, base_type)?;
    let copied = Copied::of(modifier, matches)?;
    find(&TABLES, affix, tags, &copied, Tie::Best)
}

/// The tiers of the modifier of every line of `modifier`, whose line `line` adds up its part
/// `whole` and a pure modifier: found from the values of its other lines, that line taken at
/// the lowest roll of its part, the sum being in no tier.
fn split_tiers(
    language: Language,
    base_type: &str,
    modifier: &Modifier,
    matches: &[LineMatch<'_>],
    (line, whole): (usize, &PartTier),
) -> Option<ModifierTiers> {
    let affix = match whole.affix {
        Affix::Prefix => TableAffix::Prefix,
        Affix::Suffix => TableAffix::Suffix,
    };
    let tags = bases::tags(language, base_type)?;
    let mut copied = Copied::of(modifier, matches)?;
    let summed = matches
        .iter()
        .position(|found| found.lines == (line..line + 1))?;
    let values = &mut copied.lines.get_mut(summed)?.values;
    if values.len() != whole.ranges.len() {
        return None;
    }
    for (value, range) in values.iter_mut().zip(&whole.ranges) {
        // The ranges as the game writes them, the values with the sign of the stat.
        *value = range.min.copysign(*value);
    }
    // The summed line has no range: the tier is found by the values.
    copied.ranges = None;
    find(&TABLES, affix, tags, &copied, Tie::Best)
}

/// How many prefixes and suffixes an item can have: those of its rarity, with those its runes
/// and implicits add or take away (`+1 Suffix Modifier allowed`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AffixLimits {
    pub prefixes: u32,
    pub suffixes: u32,
}

/// The tiers of a modifier copied from the in-game trade, found from its values.
#[derive(Debug, Clone, PartialEq)]
pub struct FoundTiers {
    /// The indexes among the modifiers of the item of the lines read as one modifier: one
    /// line, or the consecutive lines of a hybrid; for a line that adds up two modifiers, the
    /// lines of the one of `affixes`, the line itself or with the other lines of its hybrid.
    pub modifiers: std::ops::Range<usize>,
    /// Each affix the modifier can be, with its tiers on the base, the copied one the worst
    /// that holds its values: a prefix or a suffix, or both when the item has room for either
    /// (the `18% increased Rarity of Items found` of a helmet), the prefix first, as
    /// [`TierDoubt::EitherAffix`] reads them. The ranges of a tier, and
    /// `counts`, follow the line matches of [`modifier_stats`] for each of these modifiers in
    /// their order.
    pub affixes: Vec<(Affix, ModifierTiers)>,
    /// For a line that adds up two modifiers of its stat, the other one: a pure modifier of
    /// that line (the pure 170–179 of the `258% increased Physical Damage` of a bow, the 75–79
    /// of the hybrid with `+195 to Accuracy Rating` in `affixes`); of two pure modifiers of a
    /// prefix and a suffix, the suffix. One of both may be in two neighbouring tiers
    /// ([`ModifierTiers::neighbour`]).
    pub added: Option<AddedTiers>,
}

/// The other modifier of a line copied from the in-game trade that adds up two modifiers of
/// its stat, a pure one, found from the value of that line.
#[derive(Debug, Clone, PartialEq)]
pub struct AddedTiers {
    /// The index among the modifiers of the item of the line.
    pub line: usize,
    pub affix: Affix,
    /// Its tiers on the base, the copied one that holds its part of the value, the ranges of
    /// a tier following the values of the line.
    pub tiers: ModifierTiers,
}

/// The tiers of the explicit modifiers of a rare or magic item copied from the in-game trade,
/// found from their values in the tables of its base (`base_type` in `language`) among the
/// tiers its `item_level` allows; the other modifiers count as the affix their header tells. A
/// modifier no tier holds isn't in the list, nor counted as a prefix or a suffix: a stat the
/// tables don't know; nor a line adding up two modifiers of the same stat in more than one way
/// (two ways in two neighbouring tiers of one of them being one), with the other lines of its
/// hybrids, which take their room.
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
/// A line no other reading holds is split into two modifiers of two types with its stat, as
/// the game writes them on one line (the `258% increased Physical Damage` of a bow, a pure
/// 179% and the 79% of the hybrid with `+195 to Accuracy Rating`): a pure modifier, and
/// another, pure or a hybrid whose other lines follow or precede it with the same origin; the
/// split counts two affixes, and a reading with fewer split lines is better: a line a modifier
/// holds alone is split only when the room of the item needs it, or when the other line of the
/// hybrid has no tier alone (the `9% increased Armour` of a body armour, under every pure tier,
/// with `+100 to maximum Life`: the hybrid T6 and the pure T7). Two ways to split a line that
/// differ only by the tier of one of its modifiers, two neighbouring tiers, are one, that
/// modifier in both (`248%` with `+195 to Accuracy Rating`: the hybrid T1 and the pure T1 or
/// T2). A line that two readings as good split in other ways (a rarity of a prefix of tier 1
/// and a suffix of tier 2, or of tier 2 and tier 1) stays without tier, with the other lines of
/// its hybrids, while the other modifiers are read as the best of these readings does, with its
/// room. No line of a `sanctified` item is split: its values can leave their tiers.
///
/// A line stays without tier too, with the other lines of its hybrids, when another reading
/// fits the item as well ([`summed_readings::doubts`]), with a line adding up several
/// modifiers, whichever, or any reading where the best one splits a line: the `47%
/// d'Augmentation de l'Armure et de l'Évasion` of the owner's body armour adds up two hybrids,
/// one with `+23 à l'Armure` and `+19 au Score d'Évasion`, the other with the life; a line split
/// with its neighbour may be summed with another line's hybrid, the copy sorting its lines by
/// stat, not by modifier. Such a reading
/// that leaves another room to a line that can be a prefix or a suffix keeps both its affixes;
/// a search that can't conclude leaves every line it reads without tier.
///
/// A hybrid whose other line is read on its own (a sanctified value out of its tier, or two
/// lines apart) counts twice and may rule out an affix wrongly: a rare case, left as it is.
#[must_use]
pub fn trade_tiers(
    language: Language,
    base_type: &str,
    item_level: Option<u32>,
    modifiers: &[Modifier],
    limits: AffixLimits,
    sanctified: bool,
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
    let candidates = candidates(modifiers, &copied, tags, item_level, !sanctified);
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
    let mut ambiguous = search.ambiguous;
    // Another reading in doubt may take another room, which no longer rules out an affix.
    let mut room_in_doubt = false;
    if !sanctified {
        let best = summed_readings::Best {
            candidates: &candidates,
            reading: &reading,
            room,
            fits_room: key.room,
        };
        let doubts = summed_readings::doubts(tags, item_level, modifiers, &copied, &best);
        doubts.outcome.log(base_type);
        room_in_doubt = doubts.other_room;
        for line in doubts.lines {
            if !ambiguous.contains(&line) {
                ambiguous.push(line);
            }
        }
    }
    // A modifier that can be both, when the others leave room for one affix only: the lines
    // split in two ways as good take their room too.
    let both = if !key.room {
        log::warn!(
            "Game data, {base_type} copied from the in-game trade: no reading of its lines fits {} prefixes and {} suffixes",
            limits.prefixes,
            limits.suffixes
        );
        (true, true)
    } else if room_in_doubt {
        (true, true)
    } else {
        room.both(&reading, &candidates)
    };
    reading
        .iter()
        .filter_map(|&index| {
            // A line split in two ways as good, and the other lines of its hybrids: without
            // tier, the others read as the best reading does.
            let candidate = candidates.get(index).filter(|candidate| {
                !ambiguous
                    .iter()
                    .any(|line| candidate.modifiers.contains(line))
            })?;
            let kept = |affix| match (candidate.set(), affix) {
                (AffixSet::Only(_), _) => true,
                (AffixSet::Both, Affix::Prefix) => both.0,
                (AffixSet::Both, Affix::Suffix) => both.1,
            };
            Some(FoundTiers {
                modifiers: candidate.modifiers.clone(),
                // Without the affix the item has no room for, whose tables hold the values
                // too: a prefix, the prefixes being full.
                affixes: candidate
                    .affixes
                    .iter()
                    .filter(|(affix, _)| kept(*affix))
                    .cloned()
                    .collect(),
                added: candidate.added.clone(),
            })
        })
        .collect()
}

/// The modifiers of a rare or magic item copied from the in-game trade read again with the
/// tiers found from their values ([`trade_tiers`], with the same arguments), as the advanced
/// copy writes them: the lines of a hybrid made one modifier, a prefix or a suffix, its tier,
/// and on each value its stat counts the range of that tier. A modifier that can be a prefix
/// or a suffix stays explicit, without affix nor tier, with the tier of each
/// ([`TierDoubt::EitherAffix`]); a line that adds up two modifiers is one modifier with the
/// other lines of its hybrid, without tier, with the tiers of each part
/// ([`TierDoubt::Split`]); one that no tier holds stays as it was.
#[must_use]
pub fn with_trade_tiers(
    language: Language,
    base_type: &str,
    item_level: Option<u32>,
    modifiers: &[Modifier],
    limits: AffixLimits,
    sanctified: bool,
) -> Vec<Modifier> {
    let found = trade_tiers(
        language, base_type, item_level, modifiers, limits, sanctified,
    );
    let mut read = Vec::with_capacity(modifiers.len());
    let mut next = 0;
    while let Some(modifier) = modifiers.get(next) {
        let joined = found
            .iter()
            .filter(|found| found.modifiers.start == next)
            .find_map(|found| {
                let parts = modifiers.get(found.modifiers.clone())?;
                Some((
                    found.modifiers.end,
                    read_found(language, base_type, parts, found)?,
                ))
            });
        if let Some((end, joined)) = joined {
            read.push(joined);
            next = end;
        } else {
            read.push(modifier.clone());
            next += 1;
        }
    }
    read
}

/// The modifiers `parts`, read as one modifier in `found`: of its affix with its tier; or, when
/// `found` has both affixes, explicit with the tier of each; or, for a line that adds up two
/// modifiers, with the tiers of each part, logged when they can't be written on the item
/// (`base_type`).
fn read_found(
    language: Language,
    base_type: &str,
    parts: &[Modifier],
    found: &FoundTiers,
) -> Option<Modifier> {
    if let Some(added) = &found.added {
        let [(affix, tiers)] = found.affixes.as_slice() else {
            return None;
        };
        let split = read_split(
            language,
            parts,
            found.modifiers.start,
            (*affix, tiers),
            added,
        );
        if split.is_none() {
            // Counted as two affixes by the reading, but left without tier.
            let lines: Vec<&str> = parts
                .iter()
                .flat_map(|part| &part.lines)
                .map(|line| line.text.as_str())
                .collect();
            log::warn!(
                "Game data, {base_type} copied from the in-game trade: the split of {lines:?} can't be written, its lines left without tier"
            );
        }
        return split;
    }
    match found.affixes.as_slice() {
        [(affix, tiers)] => with_tier(language, parts, *affix, tiers),
        [(Affix::Prefix, prefix), (Affix::Suffix, suffix)] => Some(Modifier {
            doubt: Some(TierDoubt::EitherAffix {
                prefix: affix_tier(language, parts, Affix::Prefix, prefix)?,
                suffix: affix_tier(language, parts, Affix::Suffix, suffix)?,
            }),
            ..joined(parts)?
        }),
        _ => None,
    }
}

/// The modifiers `parts`, the first the modifier `first` of the item, made one modifier whose
/// line `added.line` adds up the modifier `whole` (of every line of `parts`) and `added`
/// (of that line only): of their affix, or explicit without affix for a prefix and a suffix,
/// without tier, with the tiers of each part; the other lines of a hybrid `whole` with its
/// ranges over its tiers, that line without range.
fn read_split(
    language: Language,
    parts: &[Modifier],
    first: usize,
    (affix, whole): (Affix, &ModifierTiers),
    added: &AddedTiers,
) -> Option<Modifier> {
    let summed = added.line.checked_sub(first)?;
    // The index of that line among the lines of `parts`: each has one line, as the in-game
    // trade writes them, but the lines are counted.
    let line: usize = parts.iter().take(summed).map(|part| part.lines.len()).sum();
    let alone = parts.get(summed..=summed)?;
    let mut joined = joined(parts)?;
    for (index, other) in joined.lines.iter_mut().enumerate() {
        if index == line {
            continue;
        }
        // A line whose tiers give other values a range keeps its values without range.
        if let Some(ranges) = spanned_ranges(language, parts, whole, index) {
            for (roll, range) in other.rolls.iter_mut().zip(ranges) {
                roll.range = range;
            }
        }
    }
    Some(Modifier {
        kind: ModifierKind::Explicit((affix == added.affix).then_some(affix)),
        tier: None,
        doubt: Some(TierDoubt::Split {
            line,
            whole: part_tier(language, parts, affix, whole, line)?,
            added: part_tier(language, alone, added.affix, &added.tiers, 0)?,
        }),
        ..joined
    })
}

/// The tiers `tiers` may be (one, or two neighbouring ones) of the modifiers `parts` read as
/// one modifier of `affix`, with the range over these tiers of each value of their line
/// `line` its stat counts ([`spanned_ranges`]); none when the tiers don't give a range to the
/// same values of that line, whose ranges then can't be told apart.
fn part_tier(
    language: Language,
    parts: &[Modifier],
    affix: Affix,
    tiers: &ModifierTiers,
    line: usize,
) -> Option<PartTier> {
    Some(PartTier {
        affix,
        tiers: tiers.possible().map(|tier| tier.number).collect(),
        ranges: spanned_ranges(language, parts, tiers, line)?
            .into_iter()
            .flatten()
            .collect(),
    })
}

/// For each value of the line `line` of the modifiers `parts` read as one, the range over the
/// tiers `tiers` may be ([`with_ranges`]), none for a value its stat doesn't count; none when
/// the tiers don't give a range to the same values.
fn spanned_ranges(
    language: Language,
    parts: &[Modifier],
    tiers: &ModifierTiers,
    line: usize,
) -> Option<Vec<Option<Range>>> {
    let mut ranges: Option<Vec<Option<Range>>> = None;
    for tier in tiers.possible() {
        let read = with_ranges(language, parts, tier, &tiers.counts)?;
        // The range of each value, none for a value its stat doesn't count.
        let line_ranges: Vec<Option<Range>> = read
            .lines
            .get(line)?
            .rolls
            .iter()
            .map(|roll| roll.range)
            .collect();
        ranges = Some(match ranges {
            None => line_ranges,
            Some(spanned) => spanned
                .into_iter()
                .zip(line_ranges)
                .map(|pair| match pair {
                    (Some(spanned), Some(range)) => {
                        Some(Some(spanned.holding(range.min).holding(range.max)))
                    }
                    (None, None) => Some(None),
                    _ => None,
                })
                .collect::<Option<_>>()?,
        });
    }
    ranges
}

/// The modifiers `parts` made one, with the lines of each in their order.
fn joined(parts: &[Modifier]) -> Option<Modifier> {
    Some(Modifier {
        lines: parts
            .iter()
            .flat_map(|part| part.lines.iter().cloned())
            .collect(),
        ..parts.first()?.clone()
    })
}

/// The modifiers `parts` made one modifier of `affix`, with the tier of `tiers` and the ranges
/// of that tier on its values ([`with_ranges`]).
fn with_tier(
    language: Language,
    parts: &[Modifier],
    affix: Affix,
    tiers: &ModifierTiers,
) -> Option<Modifier> {
    Some(Modifier {
        kind: ModifierKind::Explicit(Some(affix)),
        tier: Some(tiers.tier()?.number),
        ..with_ranges(language, parts, tiers.tier()?, &tiers.counts)?
    })
}

/// The tier of `tiers` that holds the values of the modifiers `parts`, as `affix`, with the
/// ranges [`with_ranges`] puts on their values, in their order, as the game writes them.
fn affix_tier(
    language: Language,
    parts: &[Modifier],
    affix: Affix,
    tiers: &ModifierTiers,
) -> Option<AffixTier> {
    let ranged = with_ranges(language, parts, tiers.tier()?, &tiers.counts)?;
    Some(AffixTier {
        affix,
        tier: tiers.tier()?.number,
        ranges: ranged
            .lines
            .iter()
            .flat_map(|line| &line.rolls)
            .filter_map(|roll| roll.range)
            .collect(),
    })
}

/// The modifiers `parts` made one, with the ranges of `tier` on the values their stats count,
/// in their order, without their sign, `counts` giving the number of values of each line match
/// ([`ModifierTiers::counts`]). A line whose counted values aren't as many as those of its stat
/// in the tables (none for `Loads an additional bolt`, whose string gives the value) keeps them
/// without range, as does a range across zero, which needs its sign.
fn with_ranges(
    language: Language,
    parts: &[Modifier],
    tier: &Tier,
    counts: &[usize],
) -> Option<Modifier> {
    let mut joined = joined(parts)?;
    // The ranges and `counts` follow the line matches of each part, in their order.
    let mut ranges = tier.ranges.iter();
    let mut counts = counts.iter();
    let mut offset = 0;
    for part in parts {
        for line_match in modifier_stats(language, part, None) {
            let count = *counts.next()?;
            let taken: Vec<Range> = ranges.by_ref().take(count).copied().collect();
            let stat = line_match.stat.as_ref()?;
            let lines = offset + line_match.lines.start..offset + line_match.lines.end;
            let rolls: Vec<_> = joined
                .lines
                .get_mut(lines)?
                .iter_mut()
                .flat_map(|line| &mut line.rolls)
                .zip(&stat.counted)
                .filter_map(|(roll, &counted)| counted.then_some(roll))
                .collect();
            if rolls.len() == taken.len() {
                for (roll, range) in rolls.into_iter().zip(taken) {
                    // As the game writes it: without its sign.
                    roll.range = range.magnitude();
                }
            }
        }
        offset += part.lines.len();
    }
    Some(joined)
}

/// Consecutive modifiers of an item copied from the in-game trade read as one, and the affixes
/// whose tables hold their values; or one of these lines split into two modifiers.
struct Candidate {
    modifiers: std::ops::Range<usize>,
    /// Whether its modifiers are fractured.
    fractured: bool,
    /// A prefix, a suffix, or both, never none; one for a split line.
    affixes: Vec<(Affix, ModifierTiers)>,
    /// The other modifier of a split line.
    added: Option<AddedTiers>,
}

impl Candidate {
    fn set(&self) -> AffixSet {
        match self.affixes.as_slice() {
            [(affix, _)] => AffixSet::Only(*affix),
            _ => AffixSet::Both,
        }
    }

    /// The affixes each of its modifiers can be: two modifiers for a split line.
    fn sets(&self) -> impl Iterator<Item = AffixSet> {
        let added = self.added.as_ref().map(|added| AffixSet::Only(added.affix));
        std::iter::once(self.set()).chain(added)
    }

    /// How many modifiers it makes.
    fn count(&self) -> usize {
        1 + usize::from(self.added.is_some())
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
/// `tags` hold, the tier the worst among those `item_level` allows, `copied` giving the lines
/// of each modifier to read (none for a modifier that isn't, or whose lines have no stat of the
/// game data): one modifier, or up to as many as the lines of a hybrid, of the same origin;
/// then, when they `may_split`, each way one of their lines adds up two modifiers
/// ([`splits`]). The shorter first at each start.
fn candidates(
    modifiers: &[Modifier],
    copied: &[Option<Copied<'_>>],
    tags: &[String],
    item_level: Option<u32>,
    may_split: bool,
) -> Vec<Candidate> {
    let tie = Tie::Worst { item_level };
    let mut candidates = Vec::new();
    for (start, first) in modifiers.iter().enumerate() {
        let mut lines = Vec::new();
        // The index among `modifiers` of the modifier of each line.
        let mut owners = Vec::new();
        let run = modifiers.iter().zip(copied).skip(start).take(*MOST_LINES);
        for (end, (modifier, part)) in (start + 1..).zip(run) {
            let Some(part) = part.as_ref().filter(|_| modifier.origin == first.origin) else {
                break;
            };
            lines.extend(part.lines.iter().cloned());
            owners.extend(std::iter::repeat_n(end - 1, part.lines.len()));
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
            let fractured = first.origin == Some(ModifierOrigin::Fractured);
            if !affixes.is_empty() {
                candidates.push(Candidate {
                    modifiers: start..end,
                    fractured,
                    affixes,
                    added: None,
                });
            }
            for (summed, &owner) in owners.iter().enumerate() {
                // A modifier of one line only: the line the game writes for both.
                if !may_split || owners.iter().filter(|&&other| other == owner).count() != 1 {
                    continue;
                }
                for split in splits(&TABLES, tags, &joined, (summed, owner), item_level) {
                    candidates.push(Candidate {
                        modifiers: start..end,
                        fractured,
                        affixes: vec![(split.affix, split.tiers)],
                        added: Some(split.added),
                    });
                }
            }
        }
    }
    candidates
}

/// A way a copied line adds up two modifiers of its stat: the modifier of every copied line,
/// with the summed one, pure or a hybrid, of `affix` with its `tiers`, and a pure modifier of
/// another type of the summed line only.
#[derive(Clone)]
struct Split {
    /// The indexes among the tables of both modifiers, the summed one first.
    tables: (usize, usize),
    affix: Affix,
    tiers: ModifierTiers,
    added: AddedTiers,
    fit: Fit,
}

/// Every way the line `summed` of `copied` (the lines of consecutive modifiers), the line of
/// the modifier `owner` of the item, adds up two modifiers of its stat on a base with `tags`:
/// the modifier of every copied line, and a pure modifier of another type of the summed line
/// only, whose values add up on that line, their tiers among those `item_level` allows. Only
/// the ways that fit best, those of two neighbouring tiers of one modifier made one
/// ([`neighbours_joined`]); none for a line without number. Of two pure modifiers, the prefix
/// is the summed one, as [`TierDoubt::EitherAffix`] reads a prefix first.
fn splits(
    tables: &[Table],
    tags: &[String],
    copied: &Copied<'_>,
    (summed, owner): (usize, usize),
    item_level: Option<u32>,
) -> Vec<Split> {
    let Some(line) = copied
        .lines
        .get(summed)
        .filter(|line| !line.values.is_empty())
    else {
        return Vec::new();
    };
    let alone = Copied {
        lines: vec![line.clone()],
        ranges: None,
    };
    // The values of the summed line among those of every line, in their order.
    let offset: usize = copied
        .lines
        .iter()
        .take(summed)
        .map(CopiedLine::count)
        .sum();
    let summed_values = offset..offset + line.count();
    let allowed = |tier: &Tier| item_level.is_none_or(|level| tier.level <= level);
    let counts: Vec<usize> = copied.lines.iter().map(CopiedLine::count).collect();
    // The pure modifiers of the summed line.
    let pure: Vec<_> = tables
        .iter()
        .enumerate()
        .filter_map(|(index, table)| {
            let order = alone.order(&table.lines)?;
            let tiers = tiers_on_base(tables, table, tags, &order, &alone)?;
            first_of_its_modifier(tables, index).then_some((index, table, tiers))
        })
        .collect();
    // Of two pure modifiers, the prefix first.
    let rank = |table: &Table, index: usize| (table.affix, index);
    let mut ways = Vec::new();
    for (whole_index, whole) in tables.iter().enumerate() {
        let Some(order) = copied.order(&whole.lines) else {
            continue;
        };
        if !first_of_its_modifier(tables, whole_index) {
            continue;
        }
        let Some(whole_tiers) = tiers_on_base(tables, whole, tags, &order, copied) else {
            continue;
        };
        let values = copied.values(&whole.lines, &order);
        for (added_index, added, added_tiers) in &pure {
            // Each pair of pure modifiers once, the prefix first.
            if added.kind == whole.kind
                || (whole.lines.len() == 1 && rank(added, *added_index) <= rank(whole, whole_index))
            {
                continue;
            }
            for (found, whole_tier) in whole_tiers.iter().enumerate() {
                for (added_found, added_tier) in added_tiers.iter().enumerate() {
                    if !allowed(whole_tier) || !allowed(added_tier) {
                        continue;
                    }
                    let Some(fit) = split_fit(&values, whole_tier, added_tier, &summed_values)
                    else {
                        continue;
                    };
                    ways.push(Split {
                        tables: (whole_index, *added_index),
                        affix: whole.affix.into(),
                        tiers: ModifierTiers {
                            tiers: whole_tiers.clone(),
                            found,
                            neighbour: None,
                            counts: counts.clone(),
                        },
                        added: AddedTiers {
                            line: owner,
                            affix: added.affix.into(),
                            tiers: ModifierTiers {
                                tiers: added_tiers.clone(),
                                found: added_found,
                                neighbour: None,
                                counts: vec![line.count()],
                            },
                        },
                        fit,
                    });
                }
            }
        }
    }
    let best = ways.iter().map(|way| way.fit).min();
    ways.retain(|way| Some(way.fit) == best);
    neighbours_joined(&ways)
}

/// The `ways` a line adds up two modifiers, those of the same two modifiers following each
/// other, with the two ways of a pair of modifiers that differ only by the tier of one of them,
/// two neighbouring tiers, made one (the pure T1 or T2 of the `248% increased Physical Damage`
/// of a bow, with the hybrid T1 on `+195 to Accuracy Rating`). Two ways that differ by both
/// tiers (the `30% increased Rarity of Items found` of a tiara, a prefix T1 and a suffix T2, or a
/// T2 and a T1), or more ways of a pair (its `33%`, also both T1), stay apart: a modifier in three
/// tiers leaves its line without tier.
fn neighbours_joined(ways: &[Split]) -> Vec<Split> {
    let mut joined = Vec::new();
    for pair in ways.chunk_by(|one, other| one.tables == other.tables) {
        match pair {
            [one, other] => match one.with_neighbour(other) {
                Some(way) => joined.push(way),
                None => joined.extend([one.clone(), other.clone()]),
            },
            _ => joined.extend(pair.iter().cloned()),
        }
    }
    joined
}

impl Split {
    /// The way this one and `other`, of the same modifiers, make when they differ only by the
    /// tier of one modifier, two neighbouring tiers.
    fn with_neighbour(&self, other: &Self) -> Option<Self> {
        let (whole, added) = (&self.tiers, &self.added.tiers);
        let (other_whole, other_added) = (&other.tiers, &other.added.tiers);
        if whole.found == other_whole.found && added.found.abs_diff(other_added.found) == 1 {
            Some(Self {
                added: AddedTiers {
                    tiers: added.or_tier(other_added.found),
                    ..self.added.clone()
                },
                ..self.clone()
            })
        } else if added.found == other_added.found && whole.found.abs_diff(other_whole.found) == 1 {
            Some(Self {
                tiers: whole.or_tier(other_whole.found),
                ..self.clone()
            })
        } else {
            None
        }
    }
}

/// Whether no table of `tables` before the one at `index` holds its modifier: each modifier
/// is looked at once, at its first table.
fn first_of_its_modifier(tables: &[Table], index: usize) -> bool {
    let Some((table, before)) = tables.get(..=index).and_then(<[Table]>::split_last) else {
        return false;
    };
    !before.iter().any(|other| other.same_modifier(table))
}

/// How well the tier `whole` of the modifier of every copied line, with the tier `added` of
/// another on the summed line, hold the copied `values`: those in `summed`, of the summed
/// line, within the sum of the ranges of both.
fn split_fit(
    values: &[f64],
    whole: &Tier,
    added: &Tier,
    summed: &std::ops::Range<usize>,
) -> Option<Fit> {
    if values.len() != whole.ranges.len() || added.ranges.len() != summed.len() {
        return None;
    }
    let ranges: Vec<Range> = whole
        .ranges
        .iter()
        .enumerate()
        .map(|(index, range)| {
            let added = index
                .checked_sub(summed.start)
                .filter(|_| summed.contains(&index))
                .and_then(|index| added.ranges.get(index));
            added.map_or(*range, |added| Range {
                min: range.min + added.min,
                max: range.max + added.max,
            })
        })
        .collect();
    holds(values, &ranges)
}

/// How well `ranges` hold `values`: within them, or give or take the rounding of the game;
/// none otherwise.
fn holds(values: &[f64], ranges: &[Range]) -> Option<Fit> {
    let within = |margin: f64| {
        values
            .iter()
            .zip(ranges)
            .all(|(&value, range)| range.min - margin <= value && value <= range.max + margin)
    };
    if within(0.0) {
        Some(Fit::Within)
    } else if within(ROUNDING) {
        Some(Fit::Rounded)
    } else {
        None
    }
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
        let sets = chosen
            .iter()
            .filter_map(|&index| candidates.get(index))
            .flat_map(Candidate::sets);
        for set in sets {
            match set {
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
            .flat_map(Candidate::sets)
            .filter(|&set| set == AffixSet::Both)
            .count();
        self.bounds(chosen, candidates)
            .map_or((true, true), |(lowest, highest)| {
                (highest >= 1, lowest < both)
            })
    }
}

/// How good a reading is, the better the greater: one fractured modifier at most, then the
/// room of the item, then the lines read with a tier, then the lines not split, then the
/// lines read alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct ReadingKey {
    fractured: bool,
    room: bool,
    /// How many lines are read with a tier: a line only a hybrid holds is read in it
    /// (`30% increased Physical Damage` of a weapon, the pure modifier starting at 40%), and
    /// one no tier holds split into two modifiers.
    read: usize,
    /// How many lines aren't split into two modifiers: a line that a modifier holds alone
    /// isn't split (`18% increased Rarity of Items found` would also fit two of 9%).
    whole: usize,
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
    /// The lines of the splits that readings as good as the best one don't share, with the
    /// other lines of their hybrids: the indexes among the modifiers of the item.
    ambiguous: Vec<usize>,
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
            ambiguous: Vec::new(),
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
            self.candidates.get(index).is_some_and(|candidate| {
                candidate.modifiers.len() == 1 && candidate.added.is_none()
            })
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
        let (mut read, mut joined, mut split) = (0, 0, 0);
        // The fractured modifiers, the lines alone counted.
        let mut fractured = self.fractured;
        for candidate in chosen
            .iter()
            .filter_map(|&index| self.candidates.get(index))
        {
            let lines = candidate.modifiers.len();
            read += lines;
            joined += lines - 1;
            split += candidate.count() - 1;
            if candidate.fractured {
                // Its fractured lines are among those of the item: as many modifiers as it makes.
                fractured = fractured + candidate.count() - lines;
            }
        }
        let key = ReadingKey {
            fractured: fractured <= 1,
            room: self.room.bounds(chosen, self.candidates).is_some(),
            read,
            whole: self.starting.len() - split,
            modifiers: self.starting.len() - joined + split,
        };
        match &self.best {
            Some((best, _)) if key < *best => {}
            Some((best, reading)) if key == *best => {
                let lines = differently_split(self.candidates, reading, chosen);
                for line in lines {
                    if !self.ambiguous.contains(&line) {
                        self.ambiguous.push(line);
                    }
                }
            }
            _ => {
                self.best = Some((key, chosen.to_vec()));
                self.ambiguous.clear();
            }
        }
    }
}

/// The lines of the splits that the readings `one` and `other`, as good, don't share: the line
/// one only splits, or splits into other modifiers, with the other lines of its hybrid, which
/// would otherwise be read alone (the `30% increased Rarity of Items found` of a tiara, a prefix
/// T1 and a suffix T2, or a T2 and a T1).
fn differently_split(candidates: &[Candidate], one: &[usize], other: &[usize]) -> Vec<usize> {
    let splits = |reading: &[usize]| -> Vec<usize> {
        reading
            .iter()
            .copied()
            .filter(|&index| {
                candidates
                    .get(index)
                    .is_some_and(|candidate| candidate.added.is_some())
            })
            .collect()
    };
    let (one, other) = (splits(one), splits(other));
    let only_one = one.iter().filter(|index| !other.contains(index));
    let only_other = other.iter().filter(|index| !one.contains(index));
    only_one
        .chain(only_other)
        .filter_map(|&index| candidates.get(index))
        .flat_map(|candidate| candidate.modifiers.clone())
        .collect()
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
        if !first_of_its_modifier(tables, index) {
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
                    neighbour: None,
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
        holds(values, &tier.ranges).or_else(|| self.ranges.as_ref().map(|_| Fit::Written))
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
mod tests;
