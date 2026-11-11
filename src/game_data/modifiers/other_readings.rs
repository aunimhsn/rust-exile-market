//! The other readings of the lines of an item copied from the in-game trade, which tell the
//! lines whose tier the best reading can't be sure of: its candidates only join lines that
//! follow each other, and only split a line into a pure modifier and another (the `47%
//! d'Augmentation de l'Armure et de l'Évasion` of the owner's body armour adds up two hybrids,
//! one of whose other lines is a sum too), and it reads each line alone where a hybrid holds
//! it too (`30% increased Energy Shield` and `+30 to maximum Life` of a helmet: two pure
//! modifiers, or their hybrid).

use super::best_reading::Room;
use super::candidates::{Candidate, TradeItem};
use super::copied::{Copied, CopiedLine};
use super::tables::Table;
use crate::item::{Affix, Modifier, ModifierOrigin, Range};
use comparison::Ground;
use placements::placements;
use walk::{Met, State};

mod comparison;
#[cfg(test)]
mod fixtures;
mod placements;
mod walk;

/// The most steps the search of the other readings takes: some fifty on a random rare item, a
/// few hundred on the most tangled ones (1,810 at most on the items drawn at random by
/// `trade_tiers/game_tests/random_items.rs`, 2026-11-11; up to 2,734 on the draws of
/// 2026-10-30, whose code wasn't kept).
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
///
/// A best reading that doesn't fit the room of the item isn't a reading of it, and none of the
/// above holds for it: once another reading fits, a line is in doubt as soon as one of them
/// reads it otherwise, whatever it makes of the others.
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
        let mut doubts = state.doubts;
        // A best reading that doesn't fit the room of the item isn't a reading of it: a line
        // keeps its tier only if every reading met, which all fit the room, reads it the
        // same, whatever they make of the others (more modifiers adding up, another tier).
        // Without any, the tables are out of step with the game, and the tiers stay.
        if !best.fits_room {
            let disputed = lines.iter().filter(|line| {
                let agreed = state.agreed.get(line.modifier).copied().unwrap_or(false);
                line.read_by_best && !agreed
            });
            for line in disputed {
                if !doubts.contains(&line.modifier) {
                    doubts.push(line.modifier);
                }
            }
        }
        return Doubts {
            lines: doubts,
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

#[cfg(test)]
mod game_tests;
