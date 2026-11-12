//! The lookup of a copied modifier in the tables: the modifier with its lines that holds it
//! best, and the tier taken when several hold it (`find`, for a modifier of either copy);
//! then the tiers of a modifier as the advanced copy writes it (`modifier_tiers`,
//! `lacks_table`).

use super::copied::{Copied, CopiedTier, Fit};
use super::item_tables::{ItemBase, ItemTables};
use super::split_lines::split_tiers;
use super::tables::{Table, TableAffix};
use super::{ModifierTiers, Tier};
use crate::game_data::lines::LineMatch;
use crate::item::{Affix, Lord, Modifier, ModifierKind, TierDoubt};

/// Every tier of `modifier`, a prefix or suffix of the item of `base`, its lines matched with
/// their stats in `matches` ([`crate::game_data::modifier_stats`]).
///
/// None for a modifier the tables don't hold: an implicit, a rune, an enchantment, a modifier
/// whose header doesn't tell its affix, a line without stat, a base without tags; or whose
/// ranges are in no tier, e.g. a line copied before a patch changed them.
///
/// For a line that adds up two modifiers ([`TierDoubt::Split`]), the tiers of the modifier of
/// every line ([`split_tiers`]), among those the level of the item allows.
///
/// With the ranges of the advanced copy, only a tier with those ranges: the tier that holds the
/// copied values first, then the one that holds them give or take the rounding of the game, then
/// the one whose values left it (any value out of its tier, e.g. on a sanctified item). Without
/// them (a line without number has no range), only a tier that holds the values. When several
/// tiers fit as well, the one of the number the advanced copy wrote is taken, or, for a header
/// written without tier, one of a modifier whose tier the game doesn't write (the Genesis tree,
/// an essence, an alloy: the `+3 to Level of all Spell Skills` of an essence on a wand, not the
/// ordinary T2); then the best one; when several modifiers do, the first of the file, an
/// ordinary one before one of the Genesis tree, of an essence or of an alloy, before a
/// desecrated one.
#[must_use]
pub fn modifier_tiers(
    base: ItemBase<'_>,
    modifier: &Modifier,
    matches: &[LineMatch<'_>],
) -> Option<ModifierTiers> {
    let tables = ItemTables::of_game(base)?;
    if let Some(TierDoubt::Split { line, whole, .. }) = &modifier.doubt {
        return split_tiers(&tables, modifier, matches, (*line, whole));
    }
    let (affix, copied) = affix_and_copied(modifier, matches)?;
    find(&tables, affix, &copied, Tie::Best)
}

/// What the tables are searched with for `modifier`: its affix, and its lines as copied; none
/// for a modifier that isn't a prefix or suffix, or a line without its stat.
fn affix_and_copied<'a>(
    modifier: &'a Modifier,
    matches: &'a [LineMatch<'_>],
) -> Option<(TableAffix, Copied<'a>)> {
    let affix = match modifier.kind {
        ModifierKind::Explicit(Some(Affix::Prefix)) => TableAffix::Prefix,
        ModifierKind::Explicit(Some(Affix::Suffix)) => TableAffix::Suffix,
        ModifierKind::Explicit(None)
        | ModifierKind::Implicit
        | ModifierKind::Enchant
        | ModifierKind::Rune
        | ModifierKind::Corruption => return None,
    };
    Some((affix, Copied::of(modifier, matches)?))
}

/// Whether the tables should hold `modifier`, of the item of `base`, and don't: a prefix or
/// suffix the advanced copy writes with its tier, on a base the data know, each of its lines
/// with its stat, and no table with these ranges, whatever the level of the item, which isn't
/// read: the copy wrote the tier. The advanced copy tells the truth: either the data are
/// behind the game, after a patch, or the item kept the ranges of an older patch (in Standard),
/// which this can't tell apart; worth a line in the log ([`modifier_tiers`] gives nothing).
#[must_use]
pub fn lacks_table(base: ItemBase<'_>, modifier: &Modifier, matches: &[LineMatch<'_>]) -> bool {
    if modifier.tier.is_none() || modifier.doubt.is_some() {
        return false;
    }
    let Some(tables) = ItemTables::of_game(base) else {
        return false;
    };
    affix_and_copied(modifier, matches)
        .is_some_and(|(affix, copied)| find(&tables, affix, &copied, Tie::Best).is_none())
}

/// Which tier is taken when the copied values are in several tiers of a modifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Tie {
    /// The best, whatever the level of the item: the advanced copy wrote the ranges of the
    /// tier, which tell it.
    Best,
    /// Without the ranges of the copy, the worst tier that could have rolled the values,
    /// among those the level of the item allows ([`ItemTables::allows`]).
    Worst,
}

/// The tiers of `copied` among `tables`, on the base of the item: those of the modifier of
/// `affix` that holds it best, the tier taken on a `tie`. Of two tiers that hold it as well,
/// the one of the number the advanced copy wrote, maybe of another modifier: a modifier of one
/// tier only, a lord's, can't hide a T3 of an ordinary one with the same lines and ranges;
/// when the copy wrote the header without tier, one whose tier the game doesn't write: the `+3
/// to Level of all Spell Skills` of an essence on a wand isn't the ordinary T2 of the same
/// value.
pub(super) fn find(
    tables: &ItemTables<'_>,
    affix: TableAffix,
    copied: &Copied<'_>,
    tie: Tie,
) -> Option<ModifierTiers> {
    // Whether a tier of `table` isn't the one the advanced copy wrote.
    let other_number = |table: &Table, tier: &Tier| match copied.tier {
        CopiedTier::Written(number) => tier.number != number,
        CopiedTier::Unwritten => !table.tier_unwritten(),
        CopiedTier::Unknown => false,
    };
    let mut best: Option<((Fit, bool), ModifierTiers)> = None;
    for held in tables.holding_as(copied, affix) {
        let table = held.table;
        let found = held
            .tiers
            .iter()
            .enumerate()
            .filter(|(_, tier)| match tie {
                Tie::Best => true,
                Tie::Worst => tables.allows(tier),
            })
            .filter_map(|(index, tier)| {
                Some((
                    (copied.fit(&held.values, tier)?, other_number(table, tier)),
                    index,
                ))
            })
            .min_by(|&(fit, index), &(other_fit, other)| {
                let tier = match tie {
                    Tie::Best => index.cmp(&other),
                    Tie::Worst => other.cmp(&index),
                };
                fit.cmp(&other_fit).then(tier)
            });
        if let Some((fit, found)) = found
            && best.as_ref().is_none_or(|(best, _)| fit < *best)
        {
            best = Some((fit, held.into_tiers(found, copied)));
        } else if let Some(((fit, _), (best_fit, best_tiers))) = found.zip(best.as_mut())
            && fit == *best_fit
        {
            // The modifiers of two lords hold it as well: neither is named (the flask charges
            // of Kurgal, `10–15`, and of Ulaman, `10–18`, on the belts). An ordinary table comes
            // before them, so a lord never takes its place; the desecrated tables of the Genesis
            // tree come after them, but share no line and base with a lord's in a range of both
            // (the exposure of the rings, `10–15` of Kurgal, `25–35` of the tree).
            if best_tiers.lord != table.lord.map(Lord::from) {
                best_tiers.lord = None;
            }
            // A modifier of a currency and another one hold it as well: the currency isn't
            // named. An ordinary table comes first, so a tie with a tier written keeps none (the
            // `+5 to Level of all Spell Skills` of a staff written T1, the essence's as well).
            if best_tiers.currency != table.currency() {
                best_tiers.currency = None;
            }
        }
    }
    best.map(|(_, tiers)| tiers)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod game_tests;
