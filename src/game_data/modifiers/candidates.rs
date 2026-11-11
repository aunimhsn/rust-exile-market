//! The candidates of the reading of an item copied from the in-game trade: each line, and each
//! run of consecutive lines, that a modifier of the tables holds, or that adds up two.

use super::ModifierTiers;
use super::base_tiers::ItemTables;
use super::copied::{Copied, CopiedTier};
use super::lookup::{Tie, find};
use super::split_lines::{AddedTiers, splits};
use super::tables::{Table, TableAffix};
use crate::item::{Affix, ModifierOrigin};

/// Consecutive modifiers of an item copied from the in-game trade read as one, and the affixes
/// whose tables hold their values; or one of these lines split into two modifiers.
pub(super) struct Candidate {
    pub(super) modifiers: std::ops::Range<usize>,
    /// Whether its modifiers are fractured.
    pub(super) fractured: bool,
    /// A prefix, a suffix, or both, never none; one for a split line.
    pub(super) affixes: Vec<(Affix, ModifierTiers)>,
    /// The other modifier of a split line.
    pub(super) added: Option<AddedTiers>,
}

impl Candidate {
    pub(super) fn set(&self) -> AffixSet {
        match self.affixes.as_slice() {
            [(affix, _)] => AffixSet::Only(*affix),
            _ => AffixSet::Both,
        }
    }

    /// The affixes each of its modifiers can be: two modifiers for a split line.
    pub(super) fn sets(&self) -> impl Iterator<Item = AffixSet> {
        let added = self.added.as_ref().map(|added| AffixSet::Only(added.affix));
        std::iter::once(self.set()).chain(added)
    }

    /// How many modifiers it makes.
    pub(super) fn count(&self) -> usize {
        1 + usize::from(self.added.is_some())
    }
}

/// The affixes a modifier can be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AffixSet {
    Only(Affix),
    Both,
}

/// The most lines a modifier of `tables` has: a hybrid of three lines in those of the game.
fn most_lines(tables: &[Table]) -> usize {
    tables
        .iter()
        .map(|table| table.lines.len())
        .max()
        .unwrap_or(1)
}

/// Every modifier or run of modifiers of an item whose values `tables` hold, the tier the
/// worst among those the level of the item allows, `copied` giving for each modifier of the
/// item its lines to read (none for a modifier that isn't read, or whose lines have no stat of
/// the game data): one modifier, or up to as many as the lines of a hybrid, of the same
/// origin; then each way one of their lines adds up two modifiers ([`splits`]). The shorter
/// first at each start.
pub(super) fn candidates(tables: &ItemTables<'_>, copied: &[Option<Copied<'_>>]) -> Vec<Candidate> {
    let most_lines = most_lines(tables.all);
    let mut candidates = Vec::new();
    for (start, first) in copied.iter().enumerate() {
        // A run starts at a modifier to read, and goes on while the next ones are of its
        // origin.
        let Some(origin) = first.as_ref().map(|first| first.origin) else {
            continue;
        };
        let mut lines = Vec::new();
        // The index among the modifiers of the item of the modifier of each line.
        let mut owners = Vec::new();
        let run = copied.iter().skip(start).take(most_lines);
        for (end, part) in (start + 1..).zip(run) {
            let Some(part) = part.as_ref().filter(|part| part.origin == origin) else {
                break;
            };
            lines.extend(part.lines.iter().cloned());
            owners.extend(std::iter::repeat_n(end - 1, part.lines.len()));
            let joined = Copied {
                lines: lines.clone(),
                ranges: None,
                tier: CopiedTier::Unknown,
                origin,
            };
            candidates.extend(run_candidates(tables, &joined, &owners, start..end));
        }
    }
    candidates
}

/// The candidates of the run of modifiers `modifiers` of the item, read as the lines `joined`,
/// `owners` giving the index among the modifiers of the item of the modifier of each line: the
/// modifier that holds them all, with each affix whose tables hold them; then each way one of
/// these lines adds up two modifiers.
fn run_candidates(
    tables: &ItemTables<'_>,
    joined: &Copied<'_>,
    owners: &[usize],
    modifiers: std::ops::Range<usize>,
) -> Vec<Candidate> {
    let fractured = joined.origin == Some(ModifierOrigin::Fractured);
    let affixes: Vec<_> = [
        (Affix::Prefix, TableAffix::Prefix),
        (Affix::Suffix, TableAffix::Suffix),
    ]
    .into_iter()
    .filter_map(|(affix, table_affix)| {
        Some((affix, find(tables, table_affix, joined, Tie::Worst)?))
    })
    .collect();
    let whole = (!affixes.is_empty()).then(|| Candidate {
        modifiers: modifiers.clone(),
        fractured,
        affixes,
        added: None,
    });
    // A modifier of one line only may be split: the line the game writes for both.
    let alone = |owner: usize| owners.iter().filter(|&&other| other == owner).count() == 1;
    let split = owners
        .iter()
        .enumerate()
        .filter(|&(_, &owner)| alone(owner))
        .flat_map(|(summed, &owner)| splits(tables, joined, (summed, owner)))
        .map(|split| Candidate {
            modifiers: modifiers.clone(),
            fractured,
            affixes: vec![(split.affix, split.tiers)],
            added: Some(split.added),
        });
    whole.into_iter().chain(split).collect()
}
