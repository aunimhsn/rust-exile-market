//! The other readings of the lines of an item copied from the in-game trade in which a line adds
//! up several modifiers of its stat, whichever: the lines whose tier the best reading can't be
//! sure of, since the candidates of the best reading only split a line into a pure modifier and
//! another whose lines follow each other (the `47% d'Augmentation de l'Armure et de l'Évasion` of
//! the owner's body armour adds up two hybrids, one of whose other lines is a sum too).

use super::{
    Affix, Candidate, Copied, CopiedLine, ROUNDING, Range, Room, TABLES, Table, TableAffix,
    first_of_its_modifier, holds, tiers_on_base,
};
use crate::item::{Modifier, ModifierOrigin};

/// The most steps the search of the other readings takes: some fifty on a random rare item, a
/// few hundred on the most tangled ones (1,349 at most on 5,600 items drawn, 2026-10-29).
const MAX_STEPS: usize = 50_000;

/// A line of the item that the best reading reads.
struct Line<'a> {
    /// The index among the modifiers of the item of its modifier.
    modifier: usize,
    origin: Option<ModifierOrigin>,
    copied: CopiedLine<'a>,
    /// Its values among those of every line.
    values: std::ops::Range<usize>,
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
    /// Whether it's the tier a reading takes for the values of its lines when no other modifier
    /// adds up with it: the one that holds them best, the worst on a tie, as
    /// [`super::Tie::Worst`] does.
    alone: bool,
}

/// The indexes among `modifiers` of the lines that another reading, with a line adding up several
/// modifiers, reads otherwise than the best `reading` of the `candidates`: on a base with `tags`,
/// among the tiers `item_level` allows, `copied` giving the lines of each modifier (those of
/// [`super::trade_tiers`]).
///
/// Another reading covers the lines the best one reads, each within the sum of the ranges of its
/// modifiers, in the `room` of the item, one fractured modifier at most, two modifiers of one type
/// never; its modifiers aren't bound to follow each other, the in-game trade copy sorting its
/// lines by stat, not by modifier. In each part of the item it reads otherwise (the lines a
/// modifier of either reading joins), it leaves the lines in doubt when it adds up several
/// modifiers on a line there and, as the best reading prefers fewer split lines (a modifier
/// holding a line alone is likelier than two adding up), when it makes no more modifiers there
/// than the best one (the two hybrids of the owner's body armour, three prefixes either way, while
/// the best reading splits no line), or when the best one splits a line there (the `217%
/// increased Physical Damage` and `+200 to Accuracy Rating` of a bow with a prefix open: the
/// hybrid T1 and a pure T3, or a pure T1, the hybrid T5 and a pure accuracy, three prefixes).
pub(super) fn doubts(
    tags: &[String],
    item_level: Option<u32>,
    modifiers: &[Modifier],
    copied: &[Option<Copied<'_>>],
    candidates: &[Candidate],
    reading: &[usize],
    room: Room,
) -> Vec<usize> {
    let chosen: Vec<&Candidate> = reading
        .iter()
        .filter_map(|&index| candidates.get(index))
        .collect();
    let read = |modifier: usize| {
        chosen
            .iter()
            .any(|candidate| candidate.modifiers.contains(&modifier))
    };
    let mut lines = Vec::new();
    let mut values = Vec::new();
    for (index, (modifier, copied)) in modifiers.iter().zip(copied).enumerate() {
        let Some(copied) = copied.as_ref().filter(|_| read(index)) else {
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
            });
        }
    }
    let placements = placements(&TABLES, tags, item_level, &lines, &values);
    let mut search = Search::new(&lines, &values, &placements, room, &chosen, modifiers.len());
    search.walk(0);
    if search.steps >= MAX_STEPS {
        log::warn!(
            "Game data, item copied from the in-game trade: the other readings of its lines looked at past {MAX_STEPS} steps"
        );
    }
    search.doubts
}

/// Every tier of every modifier of `tables` on a base with `tags`, among those `item_level`
/// allows, whose lines are among `lines`, of one origin; without a tier that can't hold the
/// `values` whatever adds up with it.
fn placements<'a>(
    tables: &'a [Table],
    tags: &[String],
    item_level: Option<u32>,
    lines: &[Line<'a>],
    values: &[f64],
) -> Vec<Placement<'a>> {
    let allowed = |level: u32| item_level.is_none_or(|item_level| level <= item_level);
    let mut placements = Vec::new();
    for (index, table) in tables.iter().enumerate() {
        if !first_of_its_modifier(tables, index) {
            continue;
        }
        for mapped in mappings(table, lines) {
            let copied = Copied {
                lines: mapped
                    .iter()
                    .filter_map(|&line| lines.get(line))
                    .map(|line| line.copied.clone())
                    .collect(),
                ranges: None,
            };
            let Some(order) = copied.order(&table.lines) else {
                continue;
            };
            // The value of a line without number is that of its stat: divided, it would differ
            // from a modifier to another (no such line rolls on a base, 2026-10-28).
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
            let Some(tiers) = tiers_on_base(tables, table, tags, &order, &copied) else {
                continue;
            };
            let indexes: Vec<usize> = mapped
                .iter()
                .filter_map(|&line| lines.get(line))
                .flat_map(|line| line.values.clone())
                .collect();
            let own: Vec<f64> = indexes
                .iter()
                .filter_map(|&index| values.get(index).copied())
                .collect();
            // The tier a reading takes for these values alone.
            let alone = tiers
                .iter()
                .enumerate()
                .filter(|(_, tier)| allowed(tier.level))
                .filter_map(|(index, tier)| Some((holds(&own, &tier.ranges)?, index)))
                .min_by(|&(fit, index), &(other_fit, other)| {
                    fit.cmp(&other_fit).then(other.cmp(&index))
                })
                .map(|(_, index)| index);
            let mut modifiers: Vec<usize> = mapped
                .iter()
                .filter_map(|&line| lines.get(line))
                .map(|line| line.modifier)
                .collect();
            modifiers.dedup();
            let affix = match table.affix {
                TableAffix::Prefix => Affix::Prefix,
                TableAffix::Suffix => Affix::Suffix,
            };
            let fractured = mapped
                .first()
                .and_then(|&line| lines.get(line))
                .is_some_and(|line| line.origin == Some(ModifierOrigin::Fractured));
            for (found, tier) in tiers.iter().enumerate() {
                if !allowed(tier.level) || tier.ranges.len() != indexes.len() {
                    continue;
                }
                placements.push(Placement {
                    kind: &table.kind,
                    fractured,
                    shown: Shown {
                        affix,
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
                    alone: alone == Some(found),
                });
            }
        }
    }
    // A value whose ranges only add up one way: a tier past it can't be among those of its line.
    let (rising, falling) = directions(&placements, values.len());
    placements.retain(|placement| {
        placement.ranges.iter().all(|&(index, range)| {
            let value = values.get(index).copied().unwrap_or_default();
            let rises = rising.get(index).copied().unwrap_or(false);
            let falls = falling.get(index).copied().unwrap_or(false);
            !(rises && range.min > value + ROUNDING || falls && range.max < value - ROUNDING)
        })
    });
    placements
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
                        line.copied.template == table_line.template
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
/// on its line, each with the affixes it can be: a modifier, or the two of a split line.
fn best_shown(chosen: &[&Candidate], count: usize) -> Vec<Vec<Vec<Shown>>> {
    let mut shown = vec![Vec::new(); count];
    for candidate in chosen {
        let whole: Vec<Shown> = candidate
            .affixes
            .iter()
            .filter_map(|(affix, tiers)| {
                Some(Shown {
                    affix: *affix,
                    tier: tiers.tier()?.number,
                    modifiers: candidate.modifiers.clone().collect(),
                    ranges: tiers.tier()?.ranges.clone(),
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
        let Some(tier) = added.tiers.tier() else {
            continue;
        };
        if let Some(shown) = shown.get_mut(added.line) {
            shown.push(vec![Shown {
                affix: added.affix,
                tier: tier.number,
                modifiers: vec![added.line],
                ranges: tier.ranges.clone(),
            }]);
        }
    }
    shown
}

/// Whether the modifiers `other` are those `best` reads on a line: one for each, among the
/// affixes it can be.
fn same(best: &[Vec<Shown>], other: &[&Shown]) -> bool {
    if best.len() != other.len() {
        return false;
    }
    let mut used = vec![false; other.len()];
    best.iter().all(|options| {
        let found = other
            .iter()
            .zip(&used)
            .position(|(shown, &used)| !used && options.contains(shown));
        found
            .and_then(|index| used.get_mut(index))
            .map(|used| *used = true)
            .is_some()
    })
}

/// The search of the other readings, line by line: at each line, the modifiers whose first line
/// it is, until the line holds its values.
struct Search<'s, 'a> {
    lines: &'s [Line<'a>],
    values: &'s [f64],
    placements: &'s [Placement<'a>],
    /// For each line, the placements whose first line it is.
    starting: Vec<Vec<usize>>,
    room: Room,
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
    /// For each value, whether its sum only rises, or only falls, as modifiers are added.
    rising: Vec<bool>,
    falling: Vec<bool>,
    /// For each modifier of the item, what the best reading reads on its line.
    best: Vec<Vec<Vec<Shown>>>,
    /// The candidates of the best reading: the indexes among the modifiers of the item of their
    /// lines, and how many modifiers each makes, two for a split line.
    parts: Vec<(std::ops::Range<usize>, usize)>,
    steps: usize,
    /// The indexes among the modifiers of the item of the lines in doubt.
    doubts: Vec<usize>,
}

impl<'s, 'a> Search<'s, 'a> {
    fn new(
        lines: &'s [Line<'a>],
        values: &'s [f64],
        placements: &'s [Placement<'a>],
        room: Room,
        chosen: &[&Candidate],
        modifiers: usize,
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
        Self {
            lines,
            values,
            placements,
            starting,
            room,
            sums: vec![Range { min: 0.0, max: 0.0 }; values.len()],
            counts: vec![0; lines.len()],
            chosen: Vec::new(),
            prefixes: 0,
            suffixes: 0,
            fractured: 0,
            rising,
            falling,
            best: best_shown(chosen, modifiers),
            parts: chosen
                .iter()
                .map(|candidate| (candidate.modifiers.clone(), candidate.count()))
                .collect(),
            steps: 0,
            doubts: Vec::new(),
        }
    }

    /// Every reading from the line `line` on, the chosen modifiers kept, each compared with the
    /// best one once it reads every line.
    fn walk(&mut self, line: usize) {
        if line >= self.lines.len() {
            self.compare();
            return;
        }
        self.choose(line, 0);
    }

    /// Every set of the modifiers whose first line is `line`, from its `from`th on, added to
    /// the chosen ones; the next line once `line` holds its values.
    fn choose(&mut self, line: usize, from: usize) {
        if self.steps >= MAX_STEPS {
            return;
        }
        self.steps += 1;
        if self.holds(line) {
            self.walk(line + 1);
        }
        let count = self.starting.get(line).map_or(0, Vec::len);
        for next in from..count {
            let Some(&index) = self
                .starting
                .get(line)
                .and_then(|starting| starting.get(next))
            else {
                continue;
            };
            if !self.fits(index) {
                continue;
            }
            self.add(index, true);
            if !self.past(index) {
                self.choose(line, next + 1);
            }
            self.add(index, false);
        }
    }

    /// Whether `line` has a modifier and the sum of their ranges holds its values.
    fn holds(&self, index: usize) -> bool {
        let Some(line) = self.lines.get(index) else {
            return false;
        };
        let covered = self.counts.get(index).is_some_and(|&count| count > 0);
        let (Some(values), Some(sums)) = (
            self.values.get(line.values.clone()),
            self.sums.get(line.values.clone()),
        ) else {
            return false;
        };
        covered && holds(values, sums).is_some()
    }

    /// Whether the modifier of `index` can be added: its type not chosen yet, room for its
    /// affix, a second fractured modifier never.
    fn fits(&self, index: usize) -> bool {
        let Some(placement) = self.placements.get(index) else {
            return false;
        };
        let kind = self
            .chosen
            .iter()
            .filter_map(|&chosen| self.placements.get(chosen))
            .any(|chosen| chosen.kind == placement.kind);
        let room = match placement.shown.affix {
            Affix::Prefix => self.prefixes < self.room.prefixes,
            Affix::Suffix => self.suffixes < self.room.suffixes,
        };
        !kind && room && !(placement.fractured && self.fractured >= 1)
    }

    /// Adds the modifier of `index` to the chosen ones, or takes it away.
    fn add(&mut self, index: usize, added: bool) {
        let Some(placement) = self.placements.get(index) else {
            return;
        };
        let sign = if added { 1.0 } else { -1.0 };
        for &(value, range) in &placement.ranges {
            if let Some(sum) = self.sums.get_mut(value) {
                sum.min += sign * range.min;
                sum.max += sign * range.max;
            }
        }
        let step = |count: &mut usize| {
            *count = if added {
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
        if added {
            self.chosen.push(index);
        } else {
            self.chosen.pop();
        }
    }

    /// Whether a value of the modifier of `index` is past what its line can hold, whatever
    /// other modifiers add up with it.
    fn past(&self, index: usize) -> bool {
        let Some(placement) = self.placements.get(index) else {
            return true;
        };
        placement.ranges.iter().any(|&(value, _)| {
            let (Some(&copied), Some(sum)) = (self.values.get(value), self.sums.get(value)) else {
                return true;
            };
            let rises = self.rising.get(value).copied().unwrap_or(false);
            let falls = self.falling.get(value).copied().unwrap_or(false);
            rises && sum.min > copied + ROUNDING || falls && sum.max < copied - ROUNDING
        })
    }

    /// Adds to the doubts the lines that the chosen modifiers, a reading of every line, read
    /// otherwise than the best reading, in each part of the item read apart (the lines a modifier
    /// of either reading joins): when a line of the part adds up several modifiers, and they are
    /// no more modifiers there than the best reading makes or it splits a line there; a line it
    /// splits elsewhere leaves the others as they are (the `18% increased Rarity of Items found`
    /// of a tiara, a prefix or a suffix, also a prefix and a suffix added up, one more modifier).
    fn compare(&mut self) {
        let placements = self.placements;
        let chosen: Vec<&Placement<'_>> = self
            .chosen
            .iter()
            .filter_map(|&index| placements.get(index))
            .collect();
        if !self.counts.iter().any(|&count| count > 1) {
            return;
        }
        // A modifier alone on its lines takes the tier the best reading would: another tier of
        // it is the same reading.
        let alone = |placement: &Placement<'_>| {
            placement
                .lines
                .iter()
                .all(|&line| self.counts.get(line) == Some(&1))
        };
        if chosen
            .iter()
            .any(|placement| alone(placement) && !placement.alone)
        {
            return;
        }
        let mut otherwise: Vec<usize> = Vec::new();
        for (modifier, best) in self.best.iter().enumerate() {
            let read: Vec<&Shown> = chosen
                .iter()
                .filter(|placement| placement.shown.modifiers.contains(&modifier))
                .map(|placement| &placement.shown)
                .collect();
            if !same(best, &read) {
                otherwise.push(modifier);
            }
        }
        let mut parents: Vec<usize> = (0..self.best.len()).collect();
        for placement in &chosen {
            join(&mut parents, placement.shown.modifiers.iter().copied());
        }
        for (modifiers, _) in &self.parts {
            join(&mut parents, modifiers.clone());
        }
        let part_of: Vec<usize> = (0..parents.len())
            .map(|modifier| root(&mut parents, modifier))
            .collect();
        let part = |modifier: usize| part_of.get(modifier).copied();
        let mut roots: Vec<Option<usize>> = otherwise.iter().map(|&line| part(line)).collect();
        roots.sort_unstable();
        roots.dedup();
        for root in roots {
            let summed = self
                .lines
                .iter()
                .zip(&self.counts)
                .any(|(line, &count)| count > 1 && part(line.modifier) == root);
            let made = chosen
                .iter()
                .filter(|placement| {
                    placement
                        .shown
                        .modifiers
                        .first()
                        .and_then(|&first| part(first))
                        == root
                })
                .count();
            let best = self
                .parts
                .iter()
                .filter(|(modifiers, _)| part(modifiers.start) == root);
            let (best_made, split) = best.fold((0, false), |(made, split), &(_, count)| {
                (made + count, split || count > 1)
            });
            if !summed || (made > best_made && !split) {
                continue;
            }
            for &line in &otherwise {
                if part(line) == root && !self.doubts.contains(&line) {
                    self.doubts.push(line);
                }
            }
        }
    }
}

/// The first of the lines joined to the line `index` among `parents`, the line each joins.
fn root(parents: &mut [usize], mut index: usize) -> usize {
    while let Some(&parent) = parents.get(index) {
        if parent == index {
            break;
        }
        let next = parents.get(parent).copied().unwrap_or(parent);
        if let Some(slot) = parents.get_mut(index) {
            *slot = next;
        }
        index = next;
    }
    index
}

/// Joins the lines `modifiers` among `parents`.
fn join(parents: &mut [usize], modifiers: impl IntoIterator<Item = usize>) {
    let mut modifiers = modifiers.into_iter();
    let Some(first) = modifiers.next() else {
        return;
    };
    let first = root(parents, first);
    for modifier in modifiers {
        let other = root(parents, modifier);
        if let Some(slot) = parents.get_mut(other) {
            *slot = first;
        }
    }
}
