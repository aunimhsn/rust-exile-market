//! The other readings of the lines of an item copied from the in-game trade, which tell the
//! lines whose tier the best reading can't be sure of: its candidates only join lines that
//! follow each other, and only split a line into a pure modifier and another (the `47%
//! d'Augmentation de l'Armure et de l'Évasion` of the owner's body armour adds up two hybrids,
//! one of whose other lines is a sum too), and it reads each line alone where a hybrid holds
//! it too (`30% increased Energy Shield` and `+30 to maximum Life` of a helmet: two pure
//! modifiers, or their hybrid).

use super::{
    Affix, AffixSet, BaseSpawn, Candidate, Copied, CopiedLine, CopiedTier, ROUNDING, Range, Room,
    Table, TradeItem, first_of_its_modifier, holds, tiers_on_base,
};
use crate::item::{Modifier, ModifierOrigin};

/// The most steps the search of the other readings takes: some fifty on a random rare item, a
/// few hundred on the most tangled ones (1,810 at most on the items drawn at random by
/// `tests/random_items.rs`, 2026-11-11; up to 2,734 on the draws of 2026-10-30, whose code
/// wasn't kept).
pub(super) const MAX_STEPS: usize = 50_000;

/// The best reading of the lines of an item, as [`super::trade_tiers`] found it.
pub(super) struct Best<'b> {
    pub(super) candidates: &'b [Candidate],
    /// The indexes among `candidates` of those it reads.
    pub(super) reading: &'b [usize],
    /// How many prefixes and suffixes the lines can add to the item.
    pub(super) room: Room,
    /// Whether it fits that room.
    pub(super) fits_room: bool,
}

/// What the other readings leave in doubt.
pub(super) struct Doubts {
    /// The indexes among the modifiers of the item of the lines in doubt, with those another
    /// reading reads and the best one leaves without tier, which have none to lose.
    pub(super) lines: Vec<usize>,
    /// Whether a reading that leaves lines in doubt leaves another room to the modifiers that
    /// can be a prefix or a suffix: an affix the full prefixes or suffixes seemed to rule out
    /// may then be the one.
    pub(super) other_room: bool,
    /// How the search ended; unless it concluded, every line read is in doubt.
    pub(super) outcome: Outcome,
    /// Whether the search met a reading of the lines, which all fit the room of the item: the
    /// best one, or another one when the best one doesn't fit it.
    pub(super) fits: bool,
    /// The steps it took, [`MAX_STEPS`] at most.
    pub(super) steps: usize,
}

/// How the search of the other readings ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Outcome {
    /// Every other reading looked at, the best one met among them (or the best one doesn't fit
    /// the room, which every other reading does).
    Concluded,
    /// Stopped at the most steps, some readings not looked at.
    Cut,
    /// Every reading looked at, none the same as the best one: a line the search can't read,
    /// or a best reading the game can't roll (two modifiers of one type).
    BestNotMet,
}

/// A line of the item copied from the in-game trade, with its stat of the game data.
struct Line<'a> {
    /// The index among the modifiers of the item of its modifier.
    modifier: usize,
    origin: Option<ModifierOrigin>,
    copied: CopiedLine<'a>,
    /// Its values among those of every line.
    values: std::ops::Range<usize>,
    /// Whether the best reading reads it: another reading reads it too, while it may leave
    /// without tier a line the best one leaves so.
    read_by_best: bool,
}

/// A modifier as a reading shows it, compared from a reading to another.
#[derive(Debug, Clone, PartialEq)]
struct Shown {
    affix: Affix,
    tier: u32,
    /// The indexes among the modifiers of the item of its lines, in their order.
    modifiers: Vec<usize>,
    /// The range of each value of its lines, in their order.
    ranges: Vec<Range>,
}

/// A tier of a modifier of the tables on some lines of the item.
struct Placement<'a> {
    /// Two modifiers of one type can't be on the same item.
    kind: &'a str,
    fractured: bool,
    shown: Shown,
    /// Its lines, by index among the [`Line`]s, in their order.
    lines: Vec<usize>,
    /// The range it adds to each value of its lines, by index among the values of every line.
    ranges: Vec<(usize, Range)>,
}

/// The indexes among `modifiers` of the lines that another reading of the item reads otherwise
/// than the `best` one, and that it leaves in doubt: on the base of `item`, among the tiers its
/// level allows, `copied` giving the lines of each modifier (those of [`super::trade_tiers`]).
///
/// Another reading reads the lines the best one reads, and maybe lines it leaves without tier,
/// each within the sum of the ranges of its modifiers, in the room of the item, as many
/// fractured modifiers as the best reading or one, two modifiers of one type never, a line
/// without value never shared; its modifiers aren't bound to follow each other, the in-game
/// trade copy sorting its lines by stat, not by modifier, and a modifier written on several
/// lines may have each read apart. In each part of the item it reads otherwise (the lines a
/// modifier of either reading joins), it leaves the lines in doubt ([`PartRead::in_doubt`]):
///
/// - when the best reading splits a line there (the `217% increased Physical Damage` and `+200
///   to Accuracy Rating` of a bow with a prefix open: the hybrid T1 and a pure T3, or a pure
///   T1, the hybrid T5 and a pure accuracy, three prefixes; a `119% increased Armour` split
///   with its neighbour, while it may be summed with the hybrid of another line);
/// - when it reads a line there that the best reading leaves without tier: a hybrid whose
///   other line no modifier holds alone, lines apart (`30% increased Physical Damage`, the pure
///   modifier starting at 40%, and `+150 to Accuracy Rating` two lines away, read alone as a
///   pure modifier), the best reading being the one that reads the most lines among its
///   candidates only;
/// - when it adds up several modifiers on a line there and makes no more modifiers there than
///   the best one (the two hybrids of the owner's body armour, three prefixes either way,
///   while the best reading splits no line): a modifier holding a line alone is likelier than
///   two adding up, not than as many read otherwise;
/// - when it adds up none and joins the lines there into other modifiers: a hybrid where the
///   best reading reads its lines alone, each a pure modifier (`30% increased Energy Shield`
///   and `+30 to maximum Life` of a helmet), or lines apart. Nothing tells which rolled, and
///   when the application can't tell, it doesn't decide (owner's rule, 2026-11-10; on the
///   items drawn at random, such lines were a hybrid far more often than pure modifiers).
///
/// A reading that keeps the lines of each modifier and differs by a tier isn't a doubt: a value
/// in two tiers takes the worst, and one that differs from the best one only by the tier of a
/// modifier of a split line, the neighbouring tier the best one gives it too, is the best one.
pub(super) fn doubts(
    tables: &[Table],
    item: &TradeItem<'_>,
    modifiers: &[Modifier],
    copied: &[Option<Copied<'_>>],
    best: &Best<'_>,
) -> Doubts {
    doubts_within(tables, item, modifiers, copied, best, MAX_STEPS)
}

/// [`doubts`], in at most `max_steps` steps.
fn doubts_within(
    tables: &[Table],
    item: &TradeItem<'_>,
    modifiers: &[Modifier],
    copied: &[Option<Copied<'_>>],
    best: &Best<'_>,
    max_steps: usize,
) -> Doubts {
    let chosen: Vec<&Candidate> = best
        .reading
        .iter()
        .filter_map(|&index| best.candidates.get(index))
        .collect();
    let read_by_best = |modifier: usize| {
        chosen
            .iter()
            .any(|candidate| candidate.modifiers.contains(&modifier))
    };
    let mut lines = Vec::new();
    let mut values = Vec::new();
    for (index, (modifier, copied)) in modifiers.iter().zip(copied).enumerate() {
        let Some(copied) = copied else {
            continue;
        };
        for line in &copied.lines {
            let start = values.len();
            values.extend(&line.values);
            values.extend(line.written.filter(|_| line.values.is_empty()));
            lines.push(Line {
                modifier: index,
                origin: modifier.origin,
                copied: line.clone(),
                values: start..values.len(),
                read_by_best: read_by_best(index),
            });
        }
    }
    let placements = placements(tables, item.base, item.item_level, &lines, &values);
    let ground = Ground::new(
        &lines,
        &values,
        &placements,
        best,
        &chosen,
        modifiers.len(),
        max_steps,
    );
    let mut state = State::new(&ground);
    state.walk(&ground, 0);
    let outcome = if state.cut {
        Outcome::Cut
    } else if state.met == Met::Best || !best.fits_room {
        Outcome::Concluded
    } else {
        Outcome::BestNotMet
    };
    if outcome == Outcome::Concluded {
        return Doubts {
            lines: state.doubts,
            other_room: state.other_room,
            outcome,
            fits: state.met != Met::Nothing,
            steps: state.steps,
        };
    }
    // Without every reading, or without the best one among them: no line keeps a tier the
    // search couldn't vouch for.
    let mut every: Vec<usize> = lines.iter().map(|line| line.modifier).collect();
    every.dedup();
    Doubts {
        lines: every,
        other_room: true,
        outcome,
        fits: state.met != Met::Nothing,
        steps: state.steps,
    }
}

/// Every tier of every modifier of `tables` on `base`, among those `item_level`
/// allows, whose lines are among `lines`, of one origin; without a tier that can't hold the
/// `values` whatever adds up with it.
fn placements<'a>(
    tables: &'a [Table],
    base: &BaseSpawn,
    item_level: Option<u32>,
    lines: &[Line<'a>],
    values: &[f64],
) -> Vec<Placement<'a>> {
    let allowed = |level: u32| item_level.is_none_or(|item_level| level <= item_level);
    let mut placements = Vec::new();
    for (index, table) in tables.iter().enumerate() {
        for mapped in mappings(table, lines) {
            // The lines of a mapping are of one origin.
            let origin = mapped
                .first()
                .and_then(|&line| lines.get(line))
                .and_then(|line| line.origin);
            if !table.may_hold(origin) {
                continue;
            }
            let copied = Copied {
                lines: mapped
                    .iter()
                    .filter_map(|&line| lines.get(line))
                    .map(|line| line.copied.clone())
                    .collect(),
                ranges: None,
                tier: CopiedTier::Unknown,
                origin,
            };
            let Some(order) = copied.order(&table.lines) else {
                continue;
            };
            if !first_of_its_modifier(tables, index) {
                continue;
            }
            // The value of a line without number is that of its stat: divided, it would differ
            // from a modifier to another (no such line rolls on a base, 2026-10-28; the best
            // reading isn't met then, and every line is in doubt).
            let divided = copied.lines.iter().zip(&order).any(|(line, &index)| {
                line.written.is_some()
                    && table
                        .lines
                        .get(index)
                        .is_some_and(|line| (line.divisor - 1.0).abs() > f64::EPSILON)
            });
            if divided {
                continue;
            }
            let Some(tiers) = tiers_on_base(tables, table, base, &order, &copied) else {
                continue;
            };
            let indexes: Vec<usize> = mapped
                .iter()
                .filter_map(|&line| lines.get(line))
                .flat_map(|line| line.values.clone())
                .collect();
            let mut modifiers: Vec<usize> = mapped
                .iter()
                .filter_map(|&line| lines.get(line))
                .map(|line| line.modifier)
                .collect();
            modifiers.dedup();
            let fractured = origin == Some(ModifierOrigin::Fractured);
            for tier in &tiers {
                if !allowed(tier.level) || tier.ranges.len() != indexes.len() {
                    continue;
                }
                placements.push(Placement {
                    kind: &table.kind,
                    fractured,
                    shown: Shown {
                        affix: table.affix.into(),
                        tier: tier.number,
                        modifiers: modifiers.clone(),
                        ranges: tier.ranges.clone(),
                    },
                    lines: mapped.clone(),
                    ranges: indexes
                        .iter()
                        .copied()
                        .zip(tier.ranges.iter().copied())
                        .collect(),
                });
            }
        }
    }
    // A value whose ranges only add up one way: a tier past it can't be among those of its line.
    let (rising, falling) = directions(&placements, values.len());
    placements.retain(|placement| {
        placement
            .ranges
            .iter()
            .all(|&(index, range)| !past(&rising, &falling, values, index, range))
    });
    placements
}

/// Whether the sum `sum` of the ranges on the value of `index` among `values` is past it,
/// whatever other ranges add up with it: only when they all rise (`rising`) or all fall.
fn past(rising: &[bool], falling: &[bool], values: &[f64], index: usize, sum: Range) -> bool {
    let Some(&value) = values.get(index) else {
        return true;
    };
    let rises = rising.get(index).copied().unwrap_or(false);
    let falls = falling.get(index).copied().unwrap_or(false);
    rises && sum.min > value + ROUNDING || falls && sum.max < value - ROUNDING
}

/// For each value, whether every range a modifier adds to it is above zero (the sum only rises
/// as modifiers are added), and whether every one is below zero.
fn directions(placements: &[Placement<'_>], count: usize) -> (Vec<bool>, Vec<bool>) {
    let mut rising = vec![true; count];
    let mut falling = vec![true; count];
    for &(index, range) in placements.iter().flat_map(|placement| &placement.ranges) {
        if let Some(rises) = rising.get_mut(index) {
            *rises &= range.min >= 0.0;
        }
        if let Some(falls) = falling.get_mut(index) {
            *falls &= range.max <= 0.0;
        }
    }
    (rising, falling)
}

/// Each way the lines of `table` are lines of the item among `lines`, each its own, of one
/// origin: for each, the indexes of the lines of the item, in their order.
fn mappings(table: &Table, lines: &[Line<'_>]) -> Vec<Vec<usize>> {
    let mut ways: Vec<Vec<usize>> = vec![Vec::new()];
    for table_line in &table.lines {
        ways = ways
            .into_iter()
            .flat_map(|way| {
                lines
                    .iter()
                    .enumerate()
                    .filter(|(index, line)| {
                        line.copied.is(&table_line.template)
                            && !way.contains(index)
                            && way
                                .first()
                                .and_then(|&first| lines.get(first))
                                .is_none_or(|first| first.origin == line.origin)
                    })
                    .map(|(index, _)| {
                        let mut way = way.clone();
                        way.push(index);
                        way
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
    }
    for way in &mut ways {
        way.sort_unstable();
    }
    ways.sort();
    ways.dedup();
    ways
}

/// For each modifier of the item, the modifiers the `chosen` candidates of the best reading read
/// on its line, each with the affixes and tiers it can be: a modifier, or the two of a split
/// line, one of which may be in two neighbouring tiers.
fn best_shown(chosen: &[&Candidate], count: usize) -> Vec<Vec<Vec<Shown>>> {
    let mut shown = vec![Vec::new(); count];
    for candidate in chosen {
        let whole: Vec<Shown> = candidate
            .affixes
            .iter()
            .flat_map(|(affix, tiers)| {
                tiers.possible().map(|tier| Shown {
                    affix: *affix,
                    tier: tier.number,
                    modifiers: candidate.modifiers.clone().collect(),
                    ranges: tier.ranges.clone(),
                })
            })
            .collect();
        for modifier in candidate.modifiers.clone() {
            if let Some(shown) = shown.get_mut(modifier) {
                shown.push(whole.clone());
            }
        }
        let Some(added) = &candidate.added else {
            continue;
        };
        let options: Vec<Shown> = added
            .tiers
            .possible()
            .map(|tier| Shown {
                affix: added.affix,
                tier: tier.number,
                modifiers: vec![added.line],
                ranges: tier.ranges.clone(),
            })
            .collect();
        if let Some(shown) = shown.get_mut(added.line).filter(|_| !options.is_empty()) {
            shown.push(options);
        }
    }
    shown
}

/// Whether the modifiers `other` are those `best` reads on a line: one for each, among the
/// affixes and tiers it can be, whatever their order. Each is matched against every one left,
/// two modifiers of `best` being able to share an option (the best reading has two modifiers on
/// a line at most).
fn same(best: &[Vec<Shown>], other: &[&Shown]) -> bool {
    let Some((options, rest)) = best.split_first() else {
        return other.is_empty();
    };
    other.iter().enumerate().any(|(index, shown)| {
        options.contains(shown) && {
            let mut left = other.to_vec();
            left.remove(index);
            same(rest, &left)
        }
    })
}

/// How a reading reads a part of the item, the lines a modifier of it or of the best reading
/// joins, against the best one: the first that holds, in this order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PartRead {
    /// The best reading splits a line there.
    BestSplits,
    /// The reading reads a line there that the best one leaves without tier.
    ReadsMore,
    /// The reading adds up several modifiers on a line there.
    Sums {
        /// How many modifiers it makes there.
        made: usize,
        /// How many the best reading makes there.
        best_made: usize,
    },
    /// The reading adds up none and joins the lines there into other modifiers than the best
    /// one: a hybrid where the best one reads pure modifiers, or lines apart.
    JoinsOtherwise,
    /// The reading keeps the lines of each modifier of the best one: other tiers of them, or
    /// other modifiers of the same lines.
    SameLines,
}

impl PartRead {
    /// How the placements `chosen`, a reading with `counts` modifiers on each line, read
    /// against the best reading of `ground` the part of the item made of the modifiers
    /// `in_part` tells.
    fn new(
        ground: &Ground<'_, '_>,
        chosen: &[&Placement<'_>],
        counts: &[usize],
        in_part: impl Fn(usize) -> bool,
    ) -> Self {
        let lines = || {
            let lines = ground.lines.iter().zip(counts);
            lines.filter(|(line, _)| in_part(line.modifier))
        };
        let best = || {
            let candidates = ground.best_candidates.iter();
            candidates.filter(|candidate| in_part(candidate.modifiers.start))
        };
        if best().any(|candidate| candidate.sets.len() > 1) {
            return Self::BestSplits;
        }
        // A line the best reading leaves without tier is in a part by a modifier of the
        // reading only.
        if lines().any(|(line, _)| !line.read_by_best) {
            return Self::ReadsMore;
        }
        // The lines of each modifier of the reading there, then of the best one, each of
        // its candidates making one modifier since none splits a line.
        let mut joined: Vec<&[usize]> = chosen
            .iter()
            .map(|placement| placement.shown.modifiers.as_slice())
            .filter(|modifiers| modifiers.first().is_some_and(|&first| in_part(first)))
            .collect();
        joined.sort_unstable();
        if lines().any(|(_, &count)| count > 1) {
            return Self::Sums {
                made: joined.len(),
                best_made: best().count(),
            };
        }
        let mut best_joined: Vec<Vec<usize>> = best()
            .map(|candidate| candidate.modifiers.clone().collect())
            .collect();
        best_joined.sort_unstable();
        if joined == best_joined {
            Self::SameLines
        } else {
            Self::JoinsOtherwise
        }
    }

    /// Whether the lines the reading reads otherwise there are in doubt ([`doubts`]): always,
    /// but a reading that adds up modifiers on a line in more modifiers than the best one, a
    /// modifier holding a line alone being likelier than two adding up, and one that keeps
    /// the lines of each modifier, a value in two tiers taking the worst.
    fn in_doubt(self) -> bool {
        match self {
            Self::BestSplits | Self::ReadsMore | Self::JoinsOtherwise => true,
            Self::Sums { made, best_made } => made <= best_made,
            Self::SameLines => false,
        }
    }
}

/// The parts of the item: the lines joined by a modifier, each part known by one of its lines.
struct Parts {
    /// For each line, the line it joins, itself for the line that knows its part.
    parents: Vec<usize>,
}

impl Parts {
    fn new(count: usize) -> Self {
        Self {
            parents: (0..count).collect(),
        }
    }

    /// The line that knows the part of the line `index`.
    fn root(&mut self, mut index: usize) -> usize {
        while let Some(&parent) = self.parents.get(index) {
            if parent == index {
                break;
            }
            let next = self.parents.get(parent).copied().unwrap_or(parent);
            if let Some(slot) = self.parents.get_mut(index) {
                *slot = next;
            }
            index = next;
        }
        index
    }

    /// Joins the parts of the lines `lines`.
    fn join(&mut self, lines: impl IntoIterator<Item = usize>) {
        let mut lines = lines.into_iter();
        let Some(first) = lines.next() else {
            return;
        };
        let first = self.root(first);
        for line in lines {
            let other = self.root(line);
            if let Some(slot) = self.parents.get_mut(other) {
                *slot = first;
            }
        }
    }
}

/// A candidate of the best reading: its lines, and the affixes of each modifier it makes, two
/// for a split line.
struct BestCandidate {
    modifiers: std::ops::Range<usize>,
    sets: Vec<AffixSet>,
}

/// What the search of the other readings reads from, the same all along.
struct Ground<'s, 'a> {
    lines: &'s [Line<'a>],
    values: &'s [f64],
    placements: &'s [Placement<'a>],
    /// For each line, the placements whose first line it is.
    starting: Vec<Vec<usize>>,
    room: Room,
    /// The most fractured modifiers a reading can have: one, or as many as the best reading.
    fractured: usize,
    /// For each value, whether its sum only rises, or only falls, as modifiers are added.
    rising: Vec<bool>,
    falling: Vec<bool>,
    /// For each modifier of the item, what the best reading reads on its line.
    best: Vec<Vec<Vec<Shown>>>,
    best_candidates: Vec<BestCandidate>,
    max_steps: usize,
}

impl<'s, 'a> Ground<'s, 'a> {
    /// `count` is the number of modifiers of the item, `max_steps` the most steps the search
    /// takes.
    fn new(
        lines: &'s [Line<'a>],
        values: &'s [f64],
        placements: &'s [Placement<'a>],
        best: &Best<'_>,
        chosen: &[&Candidate],
        count: usize,
        max_steps: usize,
    ) -> Self {
        let mut starting = vec![Vec::new(); lines.len()];
        for (index, placement) in placements.iter().enumerate() {
            if let Some(starting) = placement
                .lines
                .first()
                .and_then(|&first| starting.get_mut(first))
            {
                starting.push(index);
            }
        }
        let (rising, falling) = directions(placements, values.len());
        let fractured: usize = chosen
            .iter()
            .filter(|candidate| candidate.fractured)
            .map(|candidate| candidate.count())
            .sum();
        Self {
            lines,
            values,
            placements,
            starting,
            room: best.room,
            fractured: fractured.max(1),
            rising,
            falling,
            best: best_shown(chosen, count),
            best_candidates: chosen
                .iter()
                .map(|candidate| BestCandidate {
                    modifiers: candidate.modifiers.clone(),
                    sets: candidate.sets().collect(),
                })
                .collect(),
            max_steps,
        }
    }

    /// The modifiers of the item whose lines the placements `chosen` read otherwise than the
    /// best reading.
    fn otherwise(&self, chosen: &[&Placement<'_>]) -> Vec<usize> {
        let mut otherwise = Vec::new();
        for (modifier, best) in self.best.iter().enumerate() {
            let shown: Vec<&Shown> = chosen
                .iter()
                .filter(|placement| placement.shown.modifiers.contains(&modifier))
                .map(|placement| &placement.shown)
                .collect();
            if !same(best, &shown) {
                otherwise.push(modifier);
            }
        }
        otherwise
    }

    /// Whether the placements `chosen` leave the modifiers that can be a prefix or a suffix the
    /// room the best reading leaves them: as many prefixes, and as many suffixes, on the other
    /// lines.
    fn same_room(&self, chosen: &[&Placement<'_>]) -> bool {
        let free: Vec<&std::ops::Range<usize>> = self
            .best_candidates
            .iter()
            .filter(|candidate| candidate.sets == [AffixSet::Both])
            .map(|candidate| &candidate.modifiers)
            .collect();
        let same = |affix| {
            let best = self
                .best_candidates
                .iter()
                .flat_map(|candidate| &candidate.sets)
                .filter(|&&set| set == AffixSet::Only(affix))
                .count();
            let other = chosen
                .iter()
                .filter(|placement| placement.shown.affix == affix)
                .filter(|placement| {
                    !free.iter().any(|&modifiers| {
                        placement
                            .shown
                            .modifiers
                            .iter()
                            .copied()
                            .eq(modifiers.clone())
                    })
                })
                .count();
            best == other
        };
        same(Affix::Prefix) && same(Affix::Suffix)
    }
}

/// The readings of the lines the search met, each of them in the room of the item: the more
/// it knows the greater.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Met {
    Nothing,
    /// Some readings, none of them the best one.
    Others,
    /// The best reading, maybe among others.
    Best,
}

/// Where the search of the other readings is: the modifiers chosen so far, and what it found.
struct State {
    /// For each value, the sum of the ranges of the chosen modifiers.
    sums: Vec<Range>,
    /// For each line, how many chosen modifiers it has.
    counts: Vec<usize>,
    /// The placements chosen, in the order they were added.
    chosen: Vec<usize>,
    /// How many of them are prefixes, suffixes and fractured.
    prefixes: usize,
    suffixes: usize,
    fractured: usize,
    steps: usize,
    /// Whether the search stopped at the most steps, some readings not looked at.
    cut: bool,
    /// The readings met so far.
    met: Met,
    /// Whether a reading that leaves lines in doubt leaves another room than the best one to the
    /// modifiers that can be a prefix or a suffix.
    other_room: bool,
    /// The indexes among the modifiers of the item of the lines in doubt.
    doubts: Vec<usize>,
}

impl State {
    fn new(ground: &Ground<'_, '_>) -> Self {
        Self {
            sums: vec![Range { min: 0.0, max: 0.0 }; ground.values.len()],
            counts: vec![0; ground.lines.len()],
            chosen: Vec::new(),
            prefixes: 0,
            suffixes: 0,
            fractured: 0,
            steps: 0,
            cut: false,
            met: Met::Nothing,
            other_room: false,
            doubts: Vec::new(),
        }
    }

    /// Every reading from the line `line` on, the chosen modifiers kept, each compared with the
    /// best one once it reads every line.
    fn walk(&mut self, ground: &Ground<'_, '_>, line: usize) {
        if line >= ground.lines.len() {
            self.compare(ground);
            return;
        }
        self.choose(ground, line, 0);
    }

    /// Every set of the modifiers whose first line is `line`, from its `from`th on, added to
    /// the chosen ones; the next line once `line` is settled.
    fn choose(&mut self, ground: &Ground<'_, '_>, line: usize, from: usize) {
        if self.steps >= ground.max_steps {
            self.cut = true;
            return;
        }
        self.steps += 1;
        if self.line_settled(ground, line) {
            self.walk(ground, line + 1);
        }
        let Some(starting) = ground.starting.get(line) else {
            return;
        };
        for (next, &index) in starting.iter().enumerate().skip(from) {
            if !self.fits(ground, index) {
                continue;
            }
            self.push(ground, index);
            if !self.past(ground, index) {
                self.choose(ground, line, next + 1);
            }
            self.pop(ground);
        }
    }

    /// Whether the search can leave the line of `index` as it is: it has modifiers and the sum
    /// of their ranges holds its values; or it has none, a line the best reading leaves
    /// without tier too.
    fn line_settled(&self, ground: &Ground<'_, '_>, index: usize) -> bool {
        let (Some(line), Some(&count)) = (ground.lines.get(index), self.counts.get(index)) else {
            return false;
        };
        if count == 0 {
            return !line.read_by_best;
        }
        let (Some(values), Some(sums)) = (
            ground.values.get(line.values.clone()),
            self.sums.get(line.values.clone()),
        ) else {
            return false;
        };
        holds(values, sums).is_some()
    }

    /// Whether the placement of `index` can be added: its type not chosen yet, room for its
    /// affix and for a fractured modifier, no modifier yet on a line of it without value.
    fn fits(&self, ground: &Ground<'_, '_>, index: usize) -> bool {
        let Some(placement) = ground.placements.get(index) else {
            return false;
        };
        let kind = self
            .chosen
            .iter()
            .filter_map(|&chosen| ground.placements.get(chosen))
            .any(|chosen| chosen.kind == placement.kind);
        let room = match placement.shown.affix {
            Affix::Prefix => self.prefixes < ground.room.prefixes,
            Affix::Suffix => self.suffixes < ground.room.suffixes,
        };
        let fractured = !placement.fractured || self.fractured < ground.fractured;
        // A line without value can't add up modifiers.
        let shared = placement.lines.iter().any(|&line| {
            ground
                .lines
                .get(line)
                .is_some_and(|line| line.values.is_empty())
                && self.counts.get(line).is_some_and(|&count| count > 0)
        });
        !kind && room && fractured && !shared
    }

    /// Adds the placement of `index` to the chosen ones.
    fn push(&mut self, ground: &Ground<'_, '_>, index: usize) {
        self.apply(ground, index, 1.0);
        self.chosen.push(index);
    }

    /// Takes the last placement chosen away.
    fn pop(&mut self, ground: &Ground<'_, '_>) {
        if let Some(index) = self.chosen.pop() {
            self.apply(ground, index, -1.0);
        }
    }

    /// Adds the placement of `index` to the sums and counts (`sign` 1), or takes it away (-1).
    fn apply(&mut self, ground: &Ground<'_, '_>, index: usize, sign: f64) {
        let Some(placement) = ground.placements.get(index) else {
            return;
        };
        for &(value, range) in &placement.ranges {
            if let Some(sum) = self.sums.get_mut(value) {
                sum.min += sign * range.min;
                sum.max += sign * range.max;
            }
        }
        let step = |count: &mut usize| {
            *count = if sign > 0.0 {
                *count + 1
            } else {
                count.saturating_sub(1)
            };
        };
        for &line in &placement.lines {
            if let Some(count) = self.counts.get_mut(line) {
                step(count);
            }
        }
        match placement.shown.affix {
            Affix::Prefix => step(&mut self.prefixes),
            Affix::Suffix => step(&mut self.suffixes),
        }
        if placement.fractured {
            step(&mut self.fractured);
        }
    }

    /// Whether a value of the placement of `index` is past what its line can hold, whatever
    /// other modifiers add up with it.
    fn past(&self, ground: &Ground<'_, '_>, index: usize) -> bool {
        let Some(placement) = ground.placements.get(index) else {
            return true;
        };
        placement.ranges.iter().any(|&(value, _)| {
            self.sums
                .get(value)
                .is_none_or(|&sum| past(&ground.rising, &ground.falling, ground.values, value, sum))
        })
    }

    /// Compares the chosen placements, a reading of every line the best reading reads, with the
    /// best one: the lines it reads otherwise in each part of the item go in doubt as
    /// [`PartRead::in_doubt`] says.
    fn compare(&mut self, ground: &Ground<'_, '_>) {
        let chosen: Vec<&Placement<'_>> = self
            .chosen
            .iter()
            .filter_map(|&index| ground.placements.get(index))
            .collect();
        let otherwise = ground.otherwise(&chosen);
        if otherwise.is_empty() {
            self.met = Met::Best;
            return;
        }
        self.met = self.met.max(Met::Others);
        let mut parts = Parts::new(ground.best.len());
        for placement in &chosen {
            parts.join(placement.shown.modifiers.iter().copied());
        }
        for candidate in &ground.best_candidates {
            parts.join(candidate.modifiers.clone());
        }
        let part_of: Vec<usize> = (0..ground.best.len())
            .map(|modifier| parts.root(modifier))
            .collect();
        let part = |modifier: usize| part_of.get(modifier).copied();
        let mut roots: Vec<Option<usize>> = otherwise.iter().map(|&line| part(line)).collect();
        roots.sort_unstable();
        roots.dedup();
        for root in roots {
            let part_read = PartRead::new(ground, &chosen, &self.counts, |modifier| {
                part(modifier) == root
            });
            if !part_read.in_doubt() {
                continue;
            }
            if !ground.same_room(&chosen) {
                self.other_room = true;
            }
            for &line in &otherwise {
                if part(line) == root && !self.doubts.contains(&line) {
                    self.doubts.push(line);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
