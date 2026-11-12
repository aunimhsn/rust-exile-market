//! The filters of the properties of the item: its base type, item level, sockets, quality and
//! required level, the properties of a waystone or a gem, the DPS of a weapon, the pseudo
//! totals, and the count of its unrevealed modifiers; with the values of the game they stay
//! within.

use super::model::{Limit, Roll, Stat, StatFilter, StatText};
use super::rolls::{COMPARABLE_MARGIN, comparable, decimals};
use crate::filters::{FilterKey, Half, Property};
use crate::game_data::{self, Category};
use crate::item::{
    self, Item, MAX_GEM_LEVEL, MAX_GEM_QUALITY, MAX_GEM_SOCKETS, Range, Rarity, USUAL_QUALITY,
    WaystoneProperty,
};
use crate::trade::link;
use crate::trade::pseudo::{Pseudo, PseudoTotal};
use crate::trade::stat_filters::LineStats;

/// The quality from which the quality of a charm is checked, in percent.
const CHARM_QUALITY: u32 = 10;

/// The highest required level and item level of a rare for leveling.
const LEVELING_LEVEL: u32 = 75;

/// The item levels: the field of the item level stays within them. Checked on the trade site
/// on 2026-10-28: 262 offers of item level 100 in Forbidden Rites, none above.
const ITEM_LEVELS: Range = Range {
    min: 1.0,
    max: 100.0,
};

/// The augmentable sockets an item can have: the field of the sockets stays within them. No
/// offer had 7 or more on the trade site on 2026-10-28.
const SOCKETS: Range = Range { min: 0.0, max: 6.0 };

/// The highest quality of an item whose base the game data doesn't know: the highest of any
/// item, the 75 % of a Refined Breach Ring (checked on the trade site on 2026-10-31). The field
/// of the quality stays within the highest quality of the base ([`Category::max_quality`]), or
/// within this one.
const MAX_QUALITY: u32 = 75;

/// The quality from which the quality of a gem is checked, like Exiled Exchange 2, in percent.
const GEM_QUALITY: u32 = 16;

/// The sockets from which the sockets of a gem are checked: every gem has 2.
const GEM_SOCKETS: u32 = 3;

/// The required levels: the field of the required level stays within them. Checked on the
/// trade site on 2026-10-28: offers requiring level 90, none above.
const REQUIRED_LEVELS: Range = Range {
    min: 1.0,
    max: 90.0,
};

/// The unrevealed modifiers an item can have: three prefixes and three suffixes.
const UNREVEALED_MODIFIERS: Range = Range { min: 0.0, max: 6.0 };

/// The filters of the properties of a rare, magic or normal item, like Exiled Exchange 2: its
/// base type, checked (unchecked, its item class is searched in its place), the base of a
/// waystone written without its tier, a bar of its tier under it; its item level, checked on a
/// normal item only (a player who checks it on another usually looks for a level of crafting),
/// with the level of the item as its minimum; then its sockets, its quality and its required
/// level when they are filters, and the number of its unrevealed modifiers, checked with that
/// number as its minimum.
pub(super) fn item_filters<'a>(item: &'a Item, base_type: Option<&'a str>) -> Vec<Stat<'a>> {
    let base_type_filter = base_type.map(|base_type| {
        let waystone = if item.is_waystone() {
            item::base_and_tier(item.language(), base_type)
        } else {
            None
        };
        Stat {
            key: FilterKey::BaseType,
            text: StatText::BaseType {
                base_type: waystone.map_or(base_type, |(name, _)| name),
                class: link::searched_category(item, base_type)
                    .is_some()
                    .then(|| item.class()),
            },
            filter: Some(StatFilter {
                checked: true,
                limit: None,
            }),
            unsearchable: false,
            roll: waystone.map(|(_, tier)| Roll::WaystoneTier(tier)),
        }
    });
    let item_level = item.item_level().map(|level| {
        let level = f64::from(level);
        let checked = item.rarity() == Rarity::Normal;
        let limit = Limit::minimum(level).within(ITEM_LEVELS);
        property_filter(Property::ItemLevel, level, limit, checked)
    });
    let category = base_type.and_then(|base_type| game_data::category(item.language(), base_type));
    base_type_filter
        .into_iter()
        .chain(item_level)
        .chain(sockets_filter(item, category))
        .chain(quality_filter(item, category))
        .chain(required_level_filter(item))
        .chain(unrevealed_filter(item))
        .collect()
}

/// The filters of the properties of a waystone, in the order of the item: the item rarity and
/// the drop chance checked, what the players run a map for, the others unchecked; a percentage
/// with a minimum at 90 % of its value, rounded down, as a modifier, the revives with their
/// number; none for a property of zero or below, which searches nothing.
pub(super) fn waystone_filters(item: &Item) -> Vec<Stat<'static>> {
    item.waystone_properties()
        .into_iter()
        .filter(|found| found.value > 0)
        .map(|found| {
            let value = f64::from(found.value);
            let minimum = if found.property.is_percent() {
                (value * (1.0 - COMPARABLE_MARGIN)).floor()
            } else {
                value
            };
            let checked = matches!(
                found.property,
                WaystoneProperty::ItemRarity | WaystoneProperty::DropChance
            );
            property_filter(
                Property::Waystone(found.property),
                value,
                Limit::minimum(minimum),
                checked,
            )
        })
        .collect()
}

/// The filters of a skill gem, each with its value as its minimum: its level, always checked, so
/// that a gem of level 15 isn't compared with gems of level 1 sold for nothing; its quality,
/// checked from 16 %, like Exiled Exchange 2; its sockets, checked from 3, every gem having 2.
/// None for a support gem, which has no level.
pub(super) fn gem_filters(item: &Item) -> Vec<Stat<'static>> {
    let Some(level) = item.skill_gem_level() else {
        return Vec::new();
    };
    // Within the values a gem can take, from `min` to `max`.
    let filter = |property, value: u32, (min, max): (u32, u32), checked| {
        let value = f64::from(value);
        let bounds = Range {
            min: f64::from(min),
            max: f64::from(max),
        };
        property_filter(
            property,
            value,
            Limit::minimum(value).within(bounds),
            checked,
        )
    };
    let quality = item
        .quality()
        .filter(|&quality| quality > 0)
        .map(|quality| {
            let checked = quality >= GEM_QUALITY;
            filter(Property::Quality, quality, (0, MAX_GEM_QUALITY), checked)
        });
    let sockets = item
        .sockets()
        .filter(|&sockets| sockets > 0)
        .map(|sockets| {
            let checked = sockets >= GEM_SOCKETS;
            filter(Property::GemSockets, sockets, (0, MAX_GEM_SOCKETS), checked)
        });
    [filter(Property::GemLevel, level, (1, MAX_GEM_LEVEL), true)]
        .into_iter()
        .chain(quality)
        .chain(sockets)
        .collect()
}

/// The filter of the number of unrevealed modifiers of an item that has some, checked with
/// that number as its minimum.
fn unrevealed_filter(item: &Item) -> Option<Stat<'static>> {
    let count = item.unrevealed_modifiers();
    (count > 0).then(|| {
        let count = f64::from(count);
        Stat {
            key: FilterKey::Pseudo(Pseudo::UnrevealedModifiers),
            text: StatText::Pseudo(Pseudo::UnrevealedModifiers, count),
            filter: Some(StatFilter {
                checked: true,
                limit: Some(Limit::minimum(count).within(UNREVEALED_MODIFIERS)),
            }),
            unsearchable: false,
            roll: None,
        }
    })
}

/// The filter of the augmentable sockets of an item that has some, with their number as its
/// minimum: checked on a normal item, or when the item has more than its base usually has (an
/// exceptional base; never for a base of an unknown `category`), or is corrupted (no socket can
/// be added any more).
fn sockets_filter(item: &Item, category: Option<Category>) -> Option<Stat<'static>> {
    let sockets = item.sockets().filter(|&sockets| sockets > 0)?;
    let more = category.is_some_and(|category| sockets > category.usual_sockets());
    let checked = item.rarity() == Rarity::Normal || more || item.is_corrupted();
    let sockets = f64::from(sockets);
    Some(property_filter(
        Property::Sockets,
        sockets,
        Limit::minimum(sockets).within(SOCKETS),
        checked,
    ))
}

/// The filter of the quality of an item of the base `category`, with its value as its minimum,
/// within the highest quality of the base: the quality of a catalyst, with its kind, unchecked
/// (the trade site doesn't tell the kinds apart); like Exiled Exchange 2, for a charm, checked
/// from 10 %; for a flask from 20 %, checked above; for any other item above 20 % (an
/// exceptional quality), checked except on a rare, whose craft is usually done; and on a normal
/// item that has quality, unchecked.
fn quality_filter(item: &Item, category: Option<Category>) -> Option<Stat<'_>> {
    let quality = item.quality().filter(|&quality| quality > 0)?;
    let max = category.map_or(MAX_QUALITY, Category::max_quality);
    let limit = Limit::minimum(f64::from(quality)).within(Range {
        min: 0.0,
        max: f64::from(max),
    });
    if let Some(kind) = item.quality_kind() {
        return Some(Stat {
            key: FilterKey::Property(Property::Quality),
            text: StatText::CatalystQuality(kind, f64::from(quality)),
            filter: Some(StatFilter {
                checked: false,
                limit: Some(limit),
            }),
            unsearchable: false,
            roll: None,
        });
    }
    let checked = match category {
        Some(Category::Charm) => quality >= CHARM_QUALITY,
        Some(Category::Flask) if quality >= USUAL_QUALITY => quality > USUAL_QUALITY,
        _ if quality > USUAL_QUALITY => item.rarity() != Rarity::Rare,
        _ if item.rarity() == Rarity::Normal => false,
        _ => return None,
    };
    Some(property_filter(
        Property::Quality,
        f64::from(quality),
        limit,
        checked,
    ))
}

/// The filter of the required level of a rare for leveling (required level and item level of
/// 75 or under), unchecked, with the required level as its maximum: the offers a character of
/// that level can use.
fn required_level_filter(item: &Item) -> Option<Stat<'static>> {
    let leveling = |level: u32| level <= LEVELING_LEVEL;
    let level = item.requirements().level.filter(|&level| {
        item.rarity() == Rarity::Rare && leveling(level) && item.item_level().is_some_and(leveling)
    })?;
    let level = f64::from(level);
    Some(property_filter(
        Property::RequiredLevel,
        level,
        Limit::maximum(level).within(REQUIRED_LEVELS),
        false,
    ))
}

/// The filters of the DPS of a weapon, each kind it deals, with a minimum at 90 % of its
/// DPS, rounded down so that the weapon passes it: when `checkable`, the physical one checked,
/// the elemental one when it is at least the physical one, so that a weapon mostly elemental is
/// compared with elemental ones, or in its place. On a rare or magic item, a DPS whose minimum
/// any weapon of its base reaches compares nothing ([`LineStats::rules_out`]): unchecked.
pub(super) fn dps_filters(item: &Item, stats: &LineStats, checkable: bool) -> Vec<Stat<'static>> {
    let Some(weapon) = item.weapon() else {
        return Vec::new();
    };
    let dps = weapon.dps();
    let minimum = |dps: f64| (dps * (1.0 - COMPARABLE_MARGIN)).floor();
    // A normal item is priced by its base: its DPS is.
    let compares = |property, dps| {
        !item.is_priced_by_stats() || stats.compares(FilterKey::Property(property), minimum(dps))
    };
    let physical = compares(Property::PhysicalDps, dps.physical);
    let elemental = compares(Property::ElementalDps, dps.elemental)
        && (dps.elemental >= dps.physical || !physical);
    [
        (Property::PhysicalDps, dps.physical, physical),
        (Property::ElementalDps, dps.elemental, elemental),
    ]
    .into_iter()
    .filter_map(|(property, dps, checked)| {
        // As the item block shows it.
        let shown = dps.round();
        (shown >= 1.0).then(|| {
            property_filter(
                property,
                shown,
                Limit::minimum(minimum(dps)),
                checkable && checked,
            )
        })
    })
    .collect()
}

/// The filter of `property`, of the value `value`, with the limit `limit`.
fn property_filter(property: Property, value: f64, limit: Limit, checked: bool) -> Stat<'static> {
    Stat {
        key: FilterKey::Property(property),
        text: StatText::Property(property, value),
        filter: Some(StatFilter {
            checked,
            limit: Some(limit),
        }),
        unsearchable: false,
        roll: None,
    }
}

/// The filter of a pseudo total, with a minimum at 90 % of its value; when `checkable`,
/// checked if it compares the item ([`LineStats::rules_out`]): not when the lines of its base
/// (its implicits at their best roll, its runes, its enchantments) reach its minimum, which
/// any item of its base may then pass.
pub(super) fn pseudo_filter(
    total: &PseudoTotal,
    stats: &LineStats,
    checkable: bool,
) -> Stat<'static> {
    let minimum = comparable(total.value, decimals(total.value), Half::Minimum);
    let compares = stats.compares(FilterKey::Pseudo(total.pseudo), minimum);
    Stat {
        key: FilterKey::Pseudo(total.pseudo),
        text: StatText::Pseudo(total.pseudo, total.value),
        filter: Some(StatFilter {
            checked: checkable && compares,
            limit: Some(Limit::minimum(minimum)),
        }),
        unsearchable: false,
        roll: None,
    }
}

#[cfg(test)]
mod tests;
