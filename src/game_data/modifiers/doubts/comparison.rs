//! A reading compared with the best one: what the search reads from, the modifiers each
//! reading shows on a line, and how a reading reads each part of the item against the best one.

use super::placements::{Placement, directions};
use super::{Line, Shown};
use crate::game_data::modifiers::best_reading::{BestReading, Room};
use crate::game_data::modifiers::candidates::{AffixSet, Candidate};
use crate::item::Affix;

/// What the search of the other readings reads from, the same all along.
pub(super) struct Ground<'s, 'a> {
    pub(super) lines: &'s [Line<'a>],
    pub(super) values: &'s [f64],
    pub(super) placements: &'s [Placement<'a>],
    /// For each line, the placements whose first line it is.
    pub(super) starting: Vec<Vec<usize>>,
    pub(super) room: Room,
    /// The most fractured modifiers a reading can have: one, or as many as the best reading.
    pub(super) fractured: usize,
    /// For each value, whether its sum only rises, or only falls, as modifiers are added.
    pub(super) rising: Vec<bool>,
    pub(super) falling: Vec<bool>,
    /// For each modifier of the item, what the best reading reads on its line.
    pub(super) best: Vec<Vec<Vec<Shown>>>,
    pub(super) best_candidates: Vec<BestCandidate>,
    pub(super) max_steps: usize,
}

impl<'s, 'a> Ground<'s, 'a> {
    /// `count` is the number of modifiers of the item, `max_steps` the most steps the search
    /// takes.
    pub(super) fn new(
        lines: &'s [Line<'a>],
        values: &'s [f64],
        placements: &'s [Placement<'a>],
        best: &BestReading<'_>,
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
    pub(super) fn otherwise(&self, chosen: &[&Placement<'_>]) -> Vec<usize> {
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
    pub(super) fn same_room(&self, chosen: &[&Placement<'_>]) -> bool {
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

/// A candidate of the best reading: its lines, and the affixes of each modifier it makes, two
/// for a split line.
pub(super) struct BestCandidate {
    pub(super) modifiers: std::ops::Range<usize>,
    sets: Vec<AffixSet>,
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
pub(super) enum PartRead {
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
    pub(super) fn new(
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

    /// Whether the lines the reading reads otherwise there are in doubt ([`super::doubts`]):
    /// always, but a reading that adds up modifiers on a line in more modifiers than the best
    /// one, a modifier holding a line alone being likelier than two adding up, and one that
    /// keeps the lines of each modifier, a value in two tiers taking the worst.
    pub(super) fn in_doubt(self) -> bool {
        match self {
            Self::BestSplits | Self::ReadsMore | Self::JoinsOtherwise => true,
            Self::Sums { made, best_made } => made <= best_made,
            Self::SameLines => false,
        }
    }
}

/// The parts of the item: the lines joined by a modifier, each part known by one of its lines.
pub(super) struct Parts {
    /// For each line, the line it joins, itself for the line that knows its part.
    parents: Vec<usize>,
}

impl Parts {
    pub(super) fn new(count: usize) -> Self {
        Self {
            parents: (0..count).collect(),
        }
    }

    /// The line that knows the part of the line `index`.
    pub(super) fn root(&mut self, mut index: usize) -> usize {
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
    pub(super) fn join(&mut self, lines: impl IntoIterator<Item = usize>) {
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

#[cfg(test)]
mod tests;
