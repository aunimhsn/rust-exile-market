//! The walk through the other readings: the modifiers chosen line after line, within the
//! room of the item and the bound of the search, each reading compared with the best one.

use super::comparison::{BestShown, Reading};
use super::placements::{Directions, Placement};
use super::{ItemLines, Line};
use crate::game_data::modifiers::best_reading::{BestReading, Room};
use crate::game_data::modifiers::candidates::Candidate;
use crate::game_data::modifiers::copied::holds;
use crate::item::{Affix, Range};

/// What the walk reads from, the same all along.
pub(super) struct Ground<'s, 'a> {
    pub(super) lines: &'s [Line<'a>],
    pub(super) values: &'s [f64],
    pub(super) placements: &'s [Placement<'a>],
    /// For each line, the placements whose first line it is.
    pub(super) starting: Vec<Vec<usize>>,
    pub(super) room: Room,
    /// The most fractured modifiers a reading can have: one, or as many as the best reading.
    pub(super) fractured: usize,
    pub(super) directions: Directions,
    /// The best reading, which each reading is compared with.
    pub(super) best: BestShown,
    /// The most steps the walk takes.
    pub(super) max_steps: usize,
}

impl<'s, 'a> Ground<'s, 'a> {
    /// The ground of a walk of at most `max_steps` steps through the readings the `placements`
    /// make of `lines`, each compared with the `best` one.
    pub(super) fn new(
        lines: &'s ItemLines<'a>,
        placements: &'s [Placement<'a>],
        best: &BestReading<'_>,
        max_steps: usize,
    ) -> Self {
        let mut starting = vec![Vec::new(); lines.all.len()];
        for (index, placement) in placements.iter().enumerate() {
            if let Some(starting) = placement
                .lines
                .first()
                .and_then(|&first| starting.get_mut(first))
            {
                starting.push(index);
            }
        }
        let fractured: usize = best
            .candidates()
            .filter(|candidate| candidate.fractured)
            .map(Candidate::count)
            .sum();
        Self {
            lines: &lines.all,
            values: &lines.values,
            placements,
            starting,
            room: best.room,
            fractured: fractured.max(1),
            directions: Directions::of(placements, lines.values.len()),
            best: BestShown::new(best, lines.modifier_count),
            max_steps,
        }
    }
}

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
    cut: bool,
    /// The readings met so far.
    met: Met,
    /// For each modifier of the item, whether every reading met reads its line as the best
    /// reading does.
    agreed: Vec<bool>,
    /// Whether a reading that leaves lines in doubt leaves another room than the best one to the
    /// modifiers that can be a prefix or a suffix.
    pub(super) other_room: bool,
    /// The modifiers of the item whose line a reading met leaves in doubt
    /// ([`super::comparison::Compared::in_doubt`]), all of them among [`State::disputed`].
    pub(super) in_doubt: Vec<usize>,
}

impl State {
    /// The start of a walk on `ground`: no modifier chosen, no reading met.
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
            agreed: vec![true; ground.best.modifier_count()],
            other_room: false,
            in_doubt: Vec::new(),
        }
    }

    /// The readings met, once every reading is looked at: none for a search that stopped at
    /// the most steps, which can't tell what the others would have been.
    pub(super) fn met(&self) -> Option<Met> {
        (!self.cut).then_some(self.met)
    }

    /// The modifiers of the item whose line a reading met reads otherwise than the best one,
    /// those it leaves in doubt among them.
    pub(super) fn disputed(&self) -> Vec<usize> {
        let disputed = self
            .agreed
            .iter()
            .enumerate()
            .filter(|&(_, &agreed)| !agreed);
        disputed.map(|(modifier, _)| modifier).collect()
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
                .is_none_or(|&sum| ground.directions.is_past(ground.values, value, sum))
        })
    }

    /// Compares the chosen placements, a reading of every line the best reading reads, with the
    /// best one ([`BestShown::compare`]), and keeps what it tells.
    fn compare(&mut self, ground: &Ground<'_, '_>) {
        let reading = Reading {
            lines: ground.lines,
            chosen: self
                .chosen
                .iter()
                .filter_map(|&index| ground.placements.get(index))
                .collect(),
            counts: &self.counts,
        };
        let compared = ground.best.compare(&reading);
        for (modifier, agreed) in self.agreed.iter_mut().enumerate() {
            *agreed &= !compared.otherwise.contains(&modifier);
        }
        if compared.otherwise.is_empty() {
            self.met = Met::Best;
            return;
        }
        self.met = self.met.max(Met::Others);
        self.other_room |= compared.other_room;
        for line in compared.in_doubt {
            if !self.in_doubt.contains(&line) {
                self.in_doubt.push(line);
            }
        }
    }
}

#[cfg(test)]
mod tests;
