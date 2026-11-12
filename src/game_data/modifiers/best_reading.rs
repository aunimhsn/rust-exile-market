//! The best reading of the lines of an item copied from the in-game trade: every way to choose
//! among its candidates, ranked, within the room the item has for prefixes and suffixes.

use super::candidates::{AffixSet, Candidate};
use crate::item::Affix;

/// The most ways to read the lines of an item copied from the in-game trade that are looked
/// at: a handful on a real item, each hybrid the lines may be doubling them (24 at most on
/// the items drawn at random by `trade_tiers/game_tests/random_items.rs`, 2026-11-10). Past
/// them, no line is read: the best of the readings looked at may not be the best of all.
pub(super) const MAX_READINGS: usize = 4096;

/// How many prefixes and suffixes the modifiers read from their values can add to the item.
#[derive(Debug, Clone, Copy)]
pub(super) struct Room {
    pub(super) prefixes: usize,
    pub(super) suffixes: usize,
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
    pub(super) fn both(self, chosen: &[usize], candidates: &[Candidate]) -> (bool, bool) {
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
    /// How many modifiers the lines make, those without tier counted: the lines alone
    /// first. Where a hybrid holds them too in the room of the item, the other readings
    /// leave them without tier ([`super::doubts::doubts`]), whichever is read here.
    modifiers: usize,
}

/// The best reading of the lines of an item among its candidates, and what the readings as
/// good as it leave in doubt.
pub(super) struct BestReading<'a> {
    candidates: &'a [Candidate],
    /// The indexes among `candidates` of those it reads, in the order of their lines.
    chosen: Vec<usize>,
    /// How many prefixes and suffixes the lines can add to the item.
    pub(super) room: Room,
    /// Whether it fits that room.
    pub(super) fits_room: bool,
    /// The lines that a reading as good reads otherwise, with the other lines of their
    /// modifiers: the indexes among the modifiers of the item.
    pub(super) ambiguous: Vec<usize>,
    /// Whether a reading as good leaves another room to the modifiers that can be a prefix or
    /// a suffix.
    pub(super) room_in_doubt: bool,
    /// The readings looked at to find it, [`MAX_READINGS`] at most.
    pub(super) readings: usize,
}

impl BestReading<'_> {
    /// The candidates it reads, in the order of their lines.
    pub(super) fn candidates(&self) -> impl Iterator<Item = &Candidate> {
        self.chosen
            .iter()
            .filter_map(|&index| self.candidates.get(index))
    }

    /// Whether a modifier of it that can be both can be a prefix, and a suffix
    /// ([`Room::both`]).
    pub(super) fn both(&self) -> (bool, bool) {
        self.room.both(&self.chosen, self.candidates)
    }
}

/// The search of the best reading stopped at [`MAX_READINGS`], some readings not looked at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Cut {
    /// The readings it looked at before it stopped.
    pub(super) readings: usize,
}

/// The best reading of the `count` modifiers of an item among `candidates`, within `room`,
/// `fractured` of the modifiers being fractured. Past [`MAX_READINGS`], none: a reading not
/// looked at may be as good as the best of the others, or better.
pub(super) fn best_reading(
    candidates: &[Candidate],
    count: usize,
    room: Room,
    fractured: usize,
) -> Result<BestReading<'_>, Cut> {
    let mut search = Search::new(candidates, count, room, fractured);
    search.walk(0, &mut Vec::new());
    match search.best {
        Some((key, chosen)) if !search.cut => Ok(BestReading {
            candidates,
            chosen,
            room,
            fits_room: key.room,
            ambiguous: search.ambiguous,
            room_in_doubt: search.room_in_doubt,
            readings: search.readings,
        }),
        // Cut, the search holds the best of the readings it looked at only, which it doesn't
        // give. Not cut, it always holds one: the reading of no line for an item without any.
        Some(_) | None => Err(Cut {
            readings: search.readings,
        }),
    }
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
    /// Whether the search stopped at [`MAX_READINGS`], some readings not looked at.
    cut: bool,
    /// The best reading so far, the first one on a tie.
    best: Option<(ReadingKey, Vec<usize>)>,
    /// The lines that a reading as good as the best one reads otherwise
    /// ([`BestReading::ambiguous`]).
    ambiguous: Vec<usize>,
    /// Whether a reading as good as the best one leaves another room
    /// ([`BestReading::room_in_doubt`]).
    room_in_doubt: bool,
}

impl<'a> Search<'a> {
    /// A search among `candidates` that has looked at no reading yet, of the `count` modifiers
    /// of an item, `fractured` of them fractured, within `room`.
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
            cut: false,
            best: None,
            ambiguous: Vec::new(),
            room_in_doubt: false,
        }
    }

    /// Every reading from the modifier `position` on, after the candidates `chosen`, at most
    /// [`MAX_READINGS`]: the lines alone first, a modifier no candidate starts at read
    /// without tier.
    fn walk(&mut self, position: usize, chosen: &mut Vec<usize>) {
        if self.readings >= MAX_READINGS {
            self.cut = true;
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

    /// Ranks the reading of the candidates `chosen` against the best one so far: kept when
    /// better; as good, the lines both read otherwise are ambiguous, and their room in doubt
    /// when they leave another one.
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
            // As good as the best one: nothing tells which of them rolled, and the order of
            // the lines, which made one the first, doesn't.
            Some((best, reading)) if key == *best => {
                let lines = differently_read(self.candidates, reading, chosen);
                for line in lines {
                    if !self.ambiguous.contains(&line) {
                        self.ambiguous.push(line);
                    }
                }
                let both = |reading| self.room.both(reading, self.candidates);
                if both(reading) != both(chosen) {
                    self.room_in_doubt = true;
                }
            }
            _ => {
                self.best = Some((key, chosen.to_vec()));
                self.ambiguous.clear();
                self.room_in_doubt = false;
            }
        }
    }
}

/// The lines that the readings `one` and `other`, as good, read otherwise: those of the
/// candidates only one of them has, a line split into other modifiers (the `30% increased
/// Rarity of Items found` of a tiara, a prefix T1 and a suffix T2, or a T2 and a T1), joined
/// with another line, or read by one of them only, with the other lines of these modifiers,
/// which would otherwise be read alone.
fn differently_read(candidates: &[Candidate], one: &[usize], other: &[usize]) -> Vec<usize> {
    let only_one = one.iter().filter(|index| !other.contains(index));
    let only_other = other.iter().filter(|index| !one.contains(index));
    only_one
        .chain(only_other)
        .filter_map(|&index| candidates.get(index))
        .flat_map(|candidate| candidate.modifiers.clone())
        .collect()
}

#[cfg(test)]
mod tests;
