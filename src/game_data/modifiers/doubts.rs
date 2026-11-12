//! The doubts on the reading of an item copied from the in-game trade: the lines whose tier
//! the best reading can't be sure of, told by every other reading of them. The candidates of
//! the best reading only join lines that follow each other, and only split a line into a pure
//! modifier and another (the `47% d'Augmentation de l'Armure et de l'Évasion` of the owner's
//! body armour adds up two hybrids, one of whose other lines is a sum too), and it reads each
//! line alone where a hybrid holds it too (`30% increased Energy Shield` and `+30 to maximum
//! Life` of a helmet: two pure modifiers, or their hybrid).

use super::base_tiers::ItemTables;
use super::best_reading::BestReading;
use super::copied::{Copied, CopiedLine};
use crate::item::{Affix, ModifierOrigin, Range};
use placements::placements;
use walk::{Ground, Met, State};

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

/// What the other readings leave in doubt.
pub(super) struct Doubts {
    /// The indexes among the modifiers of the item of the lines in doubt, with those another
    /// reading reads and the best one leaves without tier, which have none to lose.
    pub(super) in_doubt: Vec<usize>,
    /// Whether a reading that leaves lines in doubt leaves another room to the modifiers that
    /// can be a prefix or a suffix: an affix the full prefixes or suffixes seemed to rule out
    /// may then be the one.
    pub(super) other_room: bool,
    /// How the search ended, which tells what it can vouch for.
    pub(super) outcome: Outcome,
    /// The steps it took, [`MAX_STEPS`] at most.
    pub(super) steps: usize,
}

/// How the search of the other readings ended, each of the readings it met fitting the room
/// of the item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Outcome {
    /// Every reading looked at, the best one, which fits the room, met among them: the lines
    /// in doubt are those the other readings tell.
    BestMet,
    /// Every reading looked at, the best one not fitting the room while another does, which
    /// the candidates don't hold (a hybrid whose lines don't follow each other): a line keeps
    /// its tier only when every reading met reads it as the best one does.
    OnlyAnotherFits,
    /// Every reading looked at, none fitting the room, the tables being out of step with the
    /// game: no line is in doubt, the lines keep the tiers of the best reading.
    NoneFits,
    /// Stopped at the most steps, some readings not looked at: every line read is in doubt.
    Cut,
    /// Every reading looked at, none the same as the best one, which fits the room: a line
    /// the search can't read, or a best reading the game can't roll (two modifiers of one
    /// type). Every line read is in doubt.
    BestNotMet,
}

impl Outcome {
    /// How a search ended that met the readings `met`, none when it stopped at the most steps,
    /// the best reading fitting the room of the item or not.
    fn of(met: Option<Met>, best_fits_room: bool) -> Self {
        match (met, best_fits_room) {
            (None, _) => Self::Cut,
            (Some(Met::Best), true) => Self::BestMet,
            (Some(Met::Nothing | Met::Others), true) => Self::BestNotMet,
            // A reading met fits the room: the best one isn't among them, but by a way to
            // tell two readings the same that the search of the best one doesn't know.
            (Some(Met::Others | Met::Best), false) => Self::OnlyAnotherFits,
            (Some(Met::Nothing), false) => Self::NoneFits,
        }
    }
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

/// The lines of the item the search reads, with the values of them all.
struct ItemLines<'a> {
    all: Vec<Line<'a>>,
    /// The values of every line, in the order of the lines ([`Line::values`]).
    values: Vec<f64>,
    /// How many modifiers the item has, read or not.
    modifier_count: usize,
}

impl<'a> ItemLines<'a> {
    /// The lines of an item, `copied` giving for each of its modifiers its lines to read, those
    /// the `best` reading reads told from those it leaves without tier.
    fn of(copied: &[Option<Copied<'a>>], best: &BestReading<'_>) -> Self {
        let read_by_best = |modifier: usize| {
            best.candidates()
                .any(|candidate| candidate.modifiers.contains(&modifier))
        };
        let mut all = Vec::new();
        let mut values = Vec::new();
        for (index, copied) in copied.iter().enumerate() {
            let Some(copied) = copied else {
                continue;
            };
            for line in &copied.lines {
                let start = values.len();
                values.extend(&line.values);
                values.extend(line.written.filter(|_| line.values.is_empty()));
                all.push(Line {
                    modifier: index,
                    origin: copied.origin,
                    copied: line.clone(),
                    values: start..values.len(),
                    read_by_best: read_by_best(index),
                });
            }
        }
        Self {
            all,
            values,
            modifier_count: copied.len(),
        }
    }

    /// The index of each modifier of the item that has lines to read.
    fn modifiers(&self) -> Vec<usize> {
        let mut modifiers: Vec<usize> = self.all.iter().map(|line| line.modifier).collect();
        modifiers.dedup();
        modifiers
    }
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

/// The lines that another reading of the item reads otherwise than the `best` one, and that
/// it leaves in doubt: among `tables`, `copied` giving for each modifier of the item its lines
/// to read (those of [`super::trade_tiers`]).
///
/// Another reading reads the lines the best one reads, and maybe lines it leaves without tier,
/// each within the sum of the ranges of its modifiers, in the room of the item, as many
/// fractured modifiers as the best reading or one, two modifiers of one type never, a line
/// without value never shared; its modifiers aren't bound to follow each other, the in-game
/// trade copy sorting its lines by stat, not by modifier, and a modifier written on several
/// lines may have each read apart. In each part of the item it reads otherwise (the lines a
/// modifier of either reading joins), it leaves the lines in doubt
/// (`PartRead::leaves_in_doubt` in `comparison.rs`):
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
    tables: &ItemTables<'_>,
    copied: &[Option<Copied<'_>>],
    best: &BestReading<'_>,
) -> Doubts {
    doubts_within(tables, copied, best, MAX_STEPS)
}

/// [`doubts`], in at most `max_steps` steps.
fn doubts_within(
    tables: &ItemTables<'_>,
    copied: &[Option<Copied<'_>>],
    best: &BestReading<'_>,
    max_steps: usize,
) -> Doubts {
    let lines = ItemLines::of(copied, best);
    let placements = placements(tables, &lines);
    let ground = Ground::new(&lines, &placements, best, max_steps);
    let mut state = State::new(&ground);
    state.walk(&ground, 0);
    let outcome = Outcome::of(state.met(), best.fits_room);
    let (in_doubt, other_room) = match outcome {
        Outcome::BestMet => (state.in_doubt, state.other_room),
        // A best reading that doesn't fit the room of the item isn't a reading of it: a line
        // keeps its tier only if every reading met, which all fit the room, reads it the
        // same, whatever they make of the others (more modifiers adding up, another tier).
        // Without any, the tables are out of step with the game, and the tiers stay.
        Outcome::OnlyAnotherFits | Outcome::NoneFits => (state.disputed(), state.other_room),
        // Without every reading, or without the best one among them: no line keeps a tier the
        // search couldn't vouch for.
        Outcome::Cut | Outcome::BestNotMet => (lines.modifiers(), true),
    };
    Doubts {
        in_doubt,
        other_room,
        outcome,
        steps: state.steps,
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod game_tests;
