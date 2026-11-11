//! The candidates of the reading of an item copied from the in-game trade: each line, and each
//! run of consecutive lines, that a modifier of the tables holds, or that adds up two; and the
//! item they are read on (`TradeItem`), which the files after this one read.

use super::ModifierTiers;
use super::copied::{Copied, CopiedTier};
use super::lookup::{Tie, find};
use super::split_lines::{AddedTiers, splits};
use super::tables::{Table, TableAffix};
use crate::game_data::bases::BaseSpawn;
use crate::item::{Affix, Modifier, ModifierOrigin};
use crate::language::Language;

/// The item whose lines are read from the in-game trade: the language of its lines, its base
/// and its level.
pub(super) struct TradeItem<'a> {
    pub(super) language: Language,
    pub(super) base: &'a BaseSpawn,
    pub(super) item_level: Option<u32>,
}

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

/// Every modifier or run of modifiers among `modifiers` whose values those of `tables` on `base`
/// hold, the tier the worst among those `item_level` allows, `copied` giving the lines
/// of each modifier to read (none for a modifier that isn't, or whose lines have no stat of the
/// game data): one modifier, or up to as many as the lines of a hybrid, of the same origin;
/// then each way one of their lines adds up two modifiers ([`splits`]). The shorter first at
/// each start.
pub(super) fn candidates(
    tables: &[Table],
    modifiers: &[Modifier],
    copied: &[Option<Copied<'_>>],
    base: &BaseSpawn,
    item_level: Option<u32>,
) -> Vec<Candidate> {
    let tie = Tie::Worst { item_level };
    let most_lines = most_lines(tables);
    let mut candidates = Vec::new();
    for (start, first) in modifiers.iter().enumerate() {
        let mut lines = Vec::new();
        // The index among `modifiers` of the modifier of each line.
        let mut owners = Vec::new();
        let run = modifiers.iter().zip(copied).skip(start).take(most_lines);
        for (end, (modifier, part)) in (start + 1..).zip(run) {
            let Some(part) = part.as_ref().filter(|_| modifier.origin == first.origin) else {
                break;
            };
            lines.extend(part.lines.iter().cloned());
            owners.extend(std::iter::repeat_n(end - 1, part.lines.len()));
            let joined = Copied {
                lines: lines.clone(),
                ranges: None,
                tier: CopiedTier::Unknown,
                origin: first.origin,
            };
            let affixes: Vec<_> = [
                (Affix::Prefix, TableAffix::Prefix),
                (Affix::Suffix, TableAffix::Suffix),
            ]
            .into_iter()
            .filter_map(|(affix, table_affix)| {
                Some((affix, find(tables, table_affix, base, &joined, tie)?))
            })
            .collect();
            let fractured = first.origin == Some(ModifierOrigin::Fractured);
            if !affixes.is_empty() {
                candidates.push(Candidate {
                    modifiers: start..end,
                    fractured,
                    affixes,
                    added: None,
                });
            }
            for (summed, &owner) in owners.iter().enumerate() {
                // A modifier of one line only: the line the game writes for both.
                if owners.iter().filter(|&&other| other == owner).count() != 1 {
                    continue;
                }
                for split in splits(tables, base, &joined, (summed, owner), item_level) {
                    candidates.push(Candidate {
                        modifiers: start..end,
                        fractured,
                        affixes: vec![(split.affix, split.tiers)],
                        added: Some(split.added),
                    });
                }
            }
        }
    }
    candidates
}
