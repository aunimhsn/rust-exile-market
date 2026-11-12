//! The rows of a unique: its lines with a bar of where each value falls in its range, or fixed.

use super::model::{Group, Limit, Origin, Row, Stat, StatFilter, StatRoll, StatText};
use super::modifier_stats::{
    LineContext, added_stat, is_enchant, is_implicit, is_rune, stats_of_kind,
};
use super::push_group;
use super::rolls::{bar, copied_ranges, magnitudes, searched_value, value_bounds};
use crate::filters::{FilterKey, StatKey};
use crate::item::{Item, ModifierKind, StatLine};

/// The rows of a unique: its enchantments, its runes, the implicit modifiers of its base,
/// then its own modifiers.
pub(super) fn unique_rows<'a>(item: &'a Item, context: &LineContext<'_>) -> Vec<Row<'a>> {
    // A stat of a unique varies when it is a filter.
    let varies = |_: &StatLine, stat: &Stat<'_>| stat.filter.is_some();
    let mut rows = Vec::new();
    push_group(
        &mut rows,
        Group::Enchant,
        stats_of_kind(
            item,
            is_enchant,
            |key, line| added_stat(key, line, context),
            varies,
        ),
    );
    push_group(
        &mut rows,
        Group::Runes,
        stats_of_kind(
            item,
            is_rune,
            |key, line| added_stat(key, line, context),
            varies,
        ),
    );
    // The origin of a modifier under its last line.
    let line_stat = |key: StatKey, line| {
        let origin = item
            .modifiers()
            .get(key.modifier)
            .filter(|modifier| key.line + 1 == modifier.lines.len())
            .and_then(|modifier| modifier.origin)
            .map(Origin::Written);
        if item.has_rolls_from_prices() && has_no_range(&context.counted(key, line)) {
            unpriced_stat(key, line, origin, context)
        } else {
            unique_stat(key, line, origin, context)
        }
    };
    push_group(
        &mut rows,
        Group::Implicit,
        stats_of_kind(item, is_implicit, line_stat, varies),
    );
    push_group(
        &mut rows,
        Group::Unique,
        stats_of_kind(
            item,
            |kind| matches!(kind, ModifierKind::Explicit(_)),
            line_stat,
            varies,
        ),
    );
    rows
}

/// A line of a unique: a filter checked with its value filling its half and its bar when it
/// varies, a fixed line otherwise; a line without a range in the advanced copy never varies.
/// The `origin` of its modifier (Vaal) follows its range, or stands alone under a line without
/// bar.
fn unique_stat<'a>(
    key: StatKey,
    whole: &'a StatLine,
    origin: Option<Origin>,
    context: &LineContext<'_>,
) -> Stat<'a> {
    let line = &context.counted(key, whole);
    let ranges = copied_ranges(line);
    let fixed = fixed_stat(key, whole, origin);
    // A filter when one of its ranges is wider than one value.
    let Some(roll) = bar(magnitudes(line, &ranges), origin) else {
        return fixed;
    };
    let Some(value) = searched_value(line) else {
        return fixed;
    };
    Stat {
        filter: Some(StatFilter {
            checked: true,
            // A legacy unique keeps its value, out of the range of today.
            limit: Some(Limit::new(
                value,
                context.half(key),
                value_bounds(line, &ranges),
                value,
                line,
            )),
        }),
        roll: Some(roll),
        ..fixed
    }
}

/// None of the values of the counted `line` has a range: on a unique copied without ranges,
/// which `has_rolls_from_prices`, none got one from the prices.
fn has_no_range(line: &StatLine) -> bool {
    copied_ranges(line).iter().all(Option::is_none)
}

/// A line of a unique copied without ranges none of whose values got a range from the prices:
/// with a number, it may vary, a filter unchecked at its value, without bar nor range; fixed
/// otherwise. The `origin` of its modifier (Vaal) stands alone under it.
fn unpriced_stat<'a>(
    key: StatKey,
    whole: &'a StatLine,
    origin: Option<Origin>,
    context: &LineContext<'_>,
) -> Stat<'a> {
    let line = &context.counted(key, whole);
    let fixed = fixed_stat(key, whole, origin);
    let Some(value) = searched_value(line) else {
        return fixed;
    };
    Stat {
        filter: Some(StatFilter {
            checked: false,
            limit: Some(Limit::new(value, context.half(key), None, value, line)),
        }),
        ..fixed
    }
}

/// A line of a unique that never varies, `whole`, of the key `key`: no filter, the `origin` of
/// its modifier alone under it.
fn fixed_stat(key: StatKey, whole: &StatLine, origin: Option<Origin>) -> Stat<'_> {
    Stat {
        key: FilterKey::Stat(key),
        text: StatText::line(&whole.text),
        filter: None,
        is_unsearchable: false,
        roll: origin.map(|origin| StatRoll::Affix {
            share: None,
            tier: None,
            origin: Some(origin),
        }),
    }
}

#[cfg(test)]
mod tests;
