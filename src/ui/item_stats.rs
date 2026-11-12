//! The stats of the item, unfolded under the item block: for a rare, magic or normal item,
//! first the filters of its base type, item level, sockets, quality, required level and DPS,
//! then of its pseudo totals for a rare or magic one, then its modifiers in the color of the
//! game, grouped, the stats that vary first. A unique, or an implicit modifier of a rare, shows
//! a bar of where its value falls in its range; a line of a prefix or a suffix, a bar over
//! every tier of its modifier on the base of the item, from the modifier tables, and its range
//! field stays within them; under the last line of its modifier, its tier and the range of that
//! tier.
//!
//! A waystone has the filters of its properties in a group of their own, the item rarity and
//! the drop chance checked, its modifiers unchecked: a buyer pays for its bonuses, not for its
//! monsters; the filter of its base shows a bar of its tier, and the tier, in its color.
//!
//! A stat that is a filter of the search starts with its checkbox and keeps the column of its
//! range field on the right, also when it has no limit; a stat of a unique that never varies
//! isn't a filter and starts at the left edge; a line the trade site can't search shows the
//! "not allowed" sign in place of its checkbox. A line better lower is searched with a
//! maximum, any other with a minimum.
//!
//! The DPS of a weapon and the pseudo totals start checked: the physical DPS, the elemental
//! one when it is at least the physical one or in its place. On a rare or magic item, those
//! any item of its base reaches start unchecked, since they compare nothing
//! ([`LineStats::rules_out`]): its implicits at their best roll, with a catalyst on a ring,
//! an amulet or a jewel, its runes and its enchantments, its damage at the usual quality. A
//! line that a DPS or a total checked at first counts starts unchecked (the damage and the
//! attack speed of a weapon, a line a total adds up). Every modifier of a rare starts
//! unchecked, like Exiled Exchange 2: its value lies in the few modifiers the buyer needs,
//! which the application can't tell, and several checked at once leave few offers.
//!
//! A desecrated modifier not revealed yet is a line of its group without filter, on a band of
//! the desecrated green; its item gets a filter of its number of unrevealed modifiers, and
//! its other stats start unchecked, with a note under them. A line of a Time-Lost jewel is
//! written without the words of its radius, a tag of the passive skills they name after its
//! range.
//!
//! The rows are decided apart from the drawing ([`rows`]), and tested, one file per rule: the
//! `model` of a row; the filters of the `properties` of the item; what the stats of its
//! modifier lines share (`modifier_stats`), the rows of a unique (`uniques`) or of a rare, magic
//! or normal item (`affixes`); the tiers under a line that adds up two modifiers
//! (`split_lines`); a stat on several lines in one row (`joined_lines`); the arithmetic of the
//! `rolls`. The `view` draws them.

use affixes::affix_rows;
use joined_lines::join_stat_lines;
use model::{Group, Row, Stat};
use modifier_stats::Reading;
use properties::{dps_filters, gem_filters, item_filters, pseudo_filter, waystone_filters};
use uniques::unique_rows;

use crate::filters::{Filter, FilterKey, Filters, Property};
use crate::format;
use crate::item::{Item, ModifierKind, Rarity};
use crate::roll_difference::CopiedLines;
use crate::tier_rolls::TierRolls;
use crate::trade::stat_filters::{LineStat, LineStats};

mod affixes;
#[cfg(test)]
mod fixtures;
mod joined_lines;
mod model;
mod modifier_stats;
mod properties;
mod rolls;
mod split_lines;
mod uniques;
mod view;

/// The number of stats of `item`, for the pill that unfolds them.
#[must_use]
pub fn count(item: &Item) -> u32 {
    len(item.modifiers())
}

/// The stats and filters of an item, decided once for a frame.
pub struct Stats<'a> {
    item: &'a Item,
    rows: Vec<Row<'a>>,
}

impl<'a> Stats<'a> {
    /// The stats of `item`, whose modifier `lines` tell which are better lower, their
    /// `stats` which the trade site can search, and their `tiers` the rolls of every tier of
    /// a prefix or suffix; `base_type` is the base type its search looks for, a filter.
    #[must_use]
    pub fn new(
        item: &'a Item,
        lines: &CopiedLines,
        stats: &LineStats,
        tiers: &TierRolls,
        base_type: Option<&'a str>,
    ) -> Self {
        Self {
            item,
            rows: rows(item, lines, stats, tiers, base_type),
        }
    }

    /// Nothing to unfold: no stat, no filter.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// The number of filters checked for the search, beside the pill of the stats, with the
    /// state tags of the item lit.
    #[must_use]
    pub fn active_filters(&self, filters: &Filters) -> u32 {
        let lit: Vec<_> = filters.lit_states_of(self.item).collect();
        len(&self.checked(filters)).saturating_add(len(&lit))
    }

    /// The filters of the search, checked or not, as the player set them or else by default.
    #[must_use]
    pub fn filters(&self, filters: &Filters) -> Vec<(FilterKey, Filter)> {
        self.rows
            .iter()
            .filter_map(|row| match row {
                Row::Stat(Stat {
                    key,
                    filter: Some(filter),
                    ..
                }) => Some((*key, filters.get(*key, || filter.default_choice()))),
                _ => None,
            })
            .collect()
    }

    /// The filters checked for the search, as the player set them or else by default.
    #[must_use]
    pub fn checked(&self, filters: &Filters) -> Vec<(FilterKey, Filter)> {
        let mut checked = self.filters(filters);
        checked.retain(|(_, filter)| filter.checked);
        checked
    }
}

fn len<T>(list: &[T]) -> u32 {
    u32::try_from(list.len()).unwrap_or(u32::MAX)
}

/// The value of `property` in its filter: a percentage of a waystone with its sign, as the
/// game writes it (`+11`), a zero without.
pub(super) fn property_value(property: Property, value: f64) -> String {
    match property {
        Property::Waystone(waystone) if waystone.is_percent() && value > 0.0 => {
            format!("+{}", format::number(value))
        }
        _ => format::number(value),
    }
}

/// The rows of the stats of `item`, whose modifier `lines` tell which are better lower, their
/// `stats` which the trade site can search, and their `tiers` the rolls of every tier of a
/// prefix or suffix; `base_type` is the base type its search looks for, a filter.
fn rows<'a>(
    item: &'a Item,
    lines: &CopiedLines,
    stats: &LineStats,
    tiers: &TierRolls,
    base_type: Option<&'a str>,
) -> Vec<Row<'a>> {
    let mut rows = Vec::new();
    // An unrevealed modifier can change the item: its other stats start unchecked.
    let unrevealed = item.unrevealed_modifiers() > 0;
    let dps = if item.rarity().has_item_filters() {
        push_group(&mut rows, Group::Item, item_filters(item, base_type));
        push_group(&mut rows, Group::Waystone, waystone_filters(item));
        dps_filters(item, stats, !unrevealed)
    } else {
        push_group(&mut rows, Group::Gem, gem_filters(item));
        Vec::new()
    };
    let pseudo = if matches!(item.rarity(), Rarity::Magic | Rarity::Rare) {
        stats
            .pseudo_totals()
            .iter()
            .map(|total| pseudo_filter(total, stats, !unrevealed))
            .collect()
    } else {
        Vec::new()
    };
    let reading = Reading {
        lines,
        stats,
        tiers,
        checked_sums: dps
            .iter()
            .chain(&pseudo)
            .filter(|stat| stat.filter.is_some_and(|filter| filter.checked))
            .map(|stat| stat.key)
            .collect(),
        unchecked: unrevealed || item.rarity() == Rarity::Rare || item.is_waystone(),
    };
    push_group(&mut rows, Group::Weapon, dps);
    push_group(&mut rows, Group::Pseudo, pseudo);
    if item.rarity() == Rarity::Unique {
        rows.extend(unique_rows(item, &reading));
    } else {
        rows.extend(affix_rows(item, &reading));
    }
    for row in &mut rows {
        if let Row::Stat(stat) = row
            && let FilterKey::Stat(key) = stat.key
        {
            searchable(stat, stats.get(key));
        }
    }
    let mut rows = join_stat_lines(item, stats, rows);
    if unrevealed {
        rows.push(Row::UnrevealedNote);
    }
    // Once a tier is found, also for a line that can be a prefix or a suffix, or that adds up
    // two modifiers, or one of the Genesis tree, read with the ranges of its tier but without
    // its number, which the game doesn't write: none before the names of the trade site tell
    // the base of a magic item, nor on a base the game data doesn't know, nor on a sanctified
    // item, whose values find no tier.
    let found = item.modifiers().iter().any(|modifier| {
        let ranged = modifier
            .lines
            .iter()
            .flat_map(|line| &line.rolls)
            .any(|roll| roll.range.is_some());
        let read = matches!(modifier.kind, ModifierKind::Explicit(Some(_)))
            && (modifier.tier.is_some() || ranged);
        let in_doubt = modifier.doubt.is_some();
        read || in_doubt
    });
    if item.has_tiers_from_values() && found {
        rows.push(Row::TradeCopyNote);
    }
    // Once a range comes from the prices: none before they arrive.
    let with_range = || {
        item.modifiers()
            .iter()
            .filter(|modifier| {
                matches!(
                    modifier.kind,
                    ModifierKind::Implicit | ModifierKind::Explicit(_)
                )
            })
            .flat_map(|modifier| &modifier.lines)
            .flat_map(|line| &line.rolls)
            .any(|roll| roll.range.is_some())
    };
    if item.has_rolls_from_prices() && with_range() {
        rows.push(Row::UniqueRollsNote);
    }
    rows
}

/// Takes the filter away from `stat` when the trade site can't search it as its `line`:
/// a line it can't search shows the "not allowed" sign instead, and another line of a stat
/// written on several lines is searched by its first line with a number of the stat.
fn searchable(stat: &mut Stat<'_>, line: Option<&LineStat>) {
    match line {
        Some(LineStat::Searched(_)) => {}
        Some(LineStat::Continued) => stat.filter = None,
        Some(LineStat::Unsearchable) | None => stat.unsearchable = stat.filter.take().is_some(),
    }
}

/// Adds the title of `group` and its stats, when it has some.
fn push_group<'a>(rows: &mut Vec<Row<'a>>, group: Group, stats: Vec<Stat<'a>>) {
    if !stats.is_empty() {
        rows.push(Row::Group(group));
        rows.extend(stats.into_iter().map(Row::Stat));
    }
}

#[cfg(test)]
mod tests;
