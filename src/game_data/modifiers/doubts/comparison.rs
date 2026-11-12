//! A reading compared with the best one: the modifiers each shows on a line, how a reading
//! reads each part of the item against the best one, and the lines it leaves in doubt.

use super::placements::Placement;
use super::{Line, Shown};
use crate::game_data::modifiers::best_reading::BestReading;
use crate::game_data::modifiers::candidates::{AffixSet, Candidate};
use crate::item::Affix;

/// The best reading as the other readings are compared with it.
pub(super) struct BestShown {
    /// For each modifier of the item, what the best reading reads on its line.
    pub(super) lines: Vec<Vec<Vec<Shown>>>,
    pub(super) candidates: Vec<BestCandidate>,
}

/// A reading of the lines of the item, as the walk chose it.
pub(super) struct Reading<'r, 'a> {
    pub(super) lines: &'r [Line<'a>],
    /// The placements it is made of.
    pub(super) chosen: Vec<&'r Placement<'a>>,
    /// For each line, how many of them it has.
    pub(super) counts: &'r [usize],
}

/// What a reading reads otherwise than the best one.
pub(super) struct Compared {
    /// The modifiers of the item whose lines it reads otherwise: none, it is the best reading.
    pub(super) otherwise: Vec<usize>,
    /// Those of them it leaves in doubt ([`PartRead::leaves_in_doubt`]).
    pub(super) in_doubt: Vec<usize>,
    /// Whether, leaving lines in doubt, it leaves another room than the best reading to the
    /// modifiers that can be a prefix or a suffix.
    pub(super) other_room: bool,
}

impl BestShown {
    /// `best` as it is compared, on an item of `count` modifiers.
    pub(super) fn new(best: &BestReading<'_>, count: usize) -> Self {
        let chosen: Vec<&Candidate> = best.candidates().collect();
        Self {
            lines: best_shown(&chosen, count),
            candidates: chosen
                .iter()
                .map(|candidate| BestCandidate {
                    modifiers: candidate.modifiers.clone(),
                    sets: candidate.sets().collect(),
                })
                .collect(),
        }
    }

    /// How many modifiers the item has.
    pub(super) fn modifier_count(&self) -> usize {
        self.lines.len()
    }

    /// Compares `reading`, which reads every line the best reading reads, with the best one:
    /// the lines it reads otherwise in each part of the item are in doubt as
    /// [`PartRead::leaves_in_doubt`] says.
    pub(super) fn compare(&self, reading: &Reading<'_, '_>) -> Compared {
        let otherwise = self.otherwise(&reading.chosen);
        if otherwise.is_empty() {
            return Compared {
                otherwise,
                in_doubt: Vec::new(),
                other_room: false,
            };
        }
        let mut parts = Parts::new(self.modifier_count());
        for placement in &reading.chosen {
            parts.join(placement.shown.modifiers.iter().copied());
        }
        for candidate in &self.candidates {
            parts.join(candidate.modifiers.clone());
        }
        let part_of: Vec<usize> = (0..self.modifier_count())
            .map(|modifier| parts.root(modifier))
            .collect();
        let part = |modifier: usize| part_of.get(modifier).copied();
        let mut roots: Vec<Option<usize>> = otherwise.iter().map(|&line| part(line)).collect();
        roots.sort_unstable();
        roots.dedup();
        let mut in_doubt = Vec::new();
        for root in roots {
            let part_read = PartRead::new(self, reading, |modifier| part(modifier) == root);
            if part_read.leaves_in_doubt() {
                in_doubt.extend(otherwise.iter().filter(|&&line| part(line) == root));
            }
        }
        Compared {
            other_room: !in_doubt.is_empty() && !self.same_room(&reading.chosen),
            otherwise,
            in_doubt,
        }
    }

    /// The modifiers of the item whose lines the placements `chosen` read otherwise than the
    /// best reading.
    fn otherwise(&self, chosen: &[&Placement<'_>]) -> Vec<usize> {
        let mut otherwise = Vec::new();
        for (modifier, best) in self.lines.iter().enumerate() {
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
            .candidates
            .iter()
            .filter(|candidate| candidate.sets == [AffixSet::Both])
            .map(|candidate| &candidate.modifiers)
            .collect();
        let same = |affix| {
            let best = self
                .candidates
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
    pub(super) sets: Vec<AffixSet>,
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
    /// How `reading` reads, against the `best` one, the part of the item made of the modifiers
    /// `in_part` tells.
    fn new(best: &BestShown, reading: &Reading<'_, '_>, in_part: impl Fn(usize) -> bool) -> Self {
        let lines = || {
            let lines = reading.lines.iter().zip(reading.counts);
            lines.filter(|(line, _)| in_part(line.modifier))
        };
        let best = || {
            let candidates = best.candidates.iter();
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
        let mut joined: Vec<&[usize]> = reading
            .chosen
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

    /// Whether the reading leaves in doubt the lines it reads otherwise there
    /// ([`super::doubts`]): always, but a reading that adds up modifiers on a line in more
    /// modifiers than the best one, a modifier holding a line alone being likelier than two
    /// adding up, and one that keeps the lines of each modifier, a value in two tiers taking
    /// the worst.
    fn leaves_in_doubt(self) -> bool {
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
    /// `count` lines, each a part of its own.
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

#[cfg(test)]
mod tests;
