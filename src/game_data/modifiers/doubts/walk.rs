//! The walk through the other readings: the modifiers chosen line after line, within the
//! room of the item and the bound of the search, each reading compared with the best one.

use super::comparison::{Ground, PartRead, Parts};
use super::placements::{Placement, past};
use crate::game_data::modifiers::copied::holds;
use crate::item::{Affix, Range};

/// The readings of the lines the search met, each of them in the room of the item: the more
/// it knows the greater.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Met {
    Nothing,
    /// Some readings, none of them the best one.
    Others,
    /// The best reading, maybe among others.
    Best,
}

/// Where the search of the other readings is: the modifiers chosen so far, and what it found.
pub(super) struct State {
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
    pub(super) steps: usize,
    /// Whether the search stopped at the most steps, some readings not looked at.
    pub(super) cut: bool,
    /// The readings met so far.
    pub(super) met: Met,
    /// For each modifier of the item, whether every reading met reads its line as the best
    /// reading does.
    pub(super) agreed: Vec<bool>,
    /// Whether a reading that leaves lines in doubt leaves another room than the best one to the
    /// modifiers that can be a prefix or a suffix.
    pub(super) other_room: bool,
    /// The indexes among the modifiers of the item of the lines in doubt.
    pub(super) doubts: Vec<usize>,
}

impl State {
    pub(super) fn new(ground: &Ground<'_, '_>) -> Self {
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
            agreed: vec![true; ground.best.len()],
            other_room: false,
            doubts: Vec::new(),
        }
    }

    /// Every reading from the line `line` on, the chosen modifiers kept, each compared with the
    /// best one once it reads every line.
    pub(super) fn walk(&mut self, ground: &Ground<'_, '_>, line: usize) {
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
        for (modifier, agreed) in self.agreed.iter_mut().enumerate() {
            *agreed &= !otherwise.contains(&modifier);
        }
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
