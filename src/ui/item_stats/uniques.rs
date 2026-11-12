//! The rows of a unique: its lines with a bar of where each value falls in its range, or fixed.

use super::model::{Group, Limit, Origin, Roll, Row, Stat, StatFilter, StatText};
use super::modifier_stats::{Reading, added, is_enchant, is_implicit, is_rune, stats};
use super::push_group;
use super::rolls::{bar, bounds, copied_ranges, magnitudes, value};
use crate::filters::{FilterKey, StatKey};
use crate::item::{Item, ModifierKind, ModifierOrigin, StatLine};

/// The rows of a unique: its enchantments, its runes, the implicit modifiers of its base,
/// then its own modifiers.
pub(super) fn unique_rows<'a>(item: &'a Item, reading: &Reading<'_>) -> Vec<Row<'a>> {
    // A stat of a unique varies when it is a filter.
    let varies = |_: &StatLine, stat: &Stat<'_>| stat.filter.is_some();
    let mut rows = Vec::new();
    let added_stat = |key, line| added(key, line, reading);
    push_group(
        &mut rows,
        Group::Enchant,
        stats(item, is_enchant, added_stat, varies),
    );
    push_group(
        &mut rows,
        Group::Runes,
        stats(item, is_rune, added_stat, varies),
    );
    // The origin of a modifier under its last line.
    let line_stat = |key: StatKey, line| {
        let origin = item
            .modifiers()
            .get(key.modifier)
            .filter(|modifier| key.line + 1 == modifier.lines.len())
            .and_then(|modifier| modifier.origin);
        unique_stat(key, line, origin, item.has_rolls_from_prices(), reading)
    };
    push_group(
        &mut rows,
        Group::Implicit,
        stats(item, is_implicit, line_stat, varies),
    );
    push_group(
        &mut rows,
        Group::Unique,
        stats(
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
/// For a unique copied without ranges, whose `rolls_from_prices`, a line with a number none
/// of whose values got a range from the prices may vary: a filter unchecked at its value,
/// without bar nor range. The `origin` of its modifier (Vaal) follows its range, or stands
/// alone under a line without bar.
fn unique_stat<'a>(
    key: StatKey,
    whole: &'a StatLine,
    origin: Option<ModifierOrigin>,
    rolls_from_prices: bool,
    reading: &Reading<'_>,
) -> Stat<'a> {
    let line = &reading.counted(key, whole);
    let ranges = copied_ranges(line);
    let origin = origin.map(Origin::Written);
    let fixed = Stat {
        key: FilterKey::Stat(key),
        text: StatText::line(&whole.text),
        filter: None,
        unsearchable: false,
        roll: origin.map(|origin| Roll::Affix {
            share: None,
            tier: None,
            origin: Some(origin),
        }),
    };
    if rolls_from_prices && ranges.iter().all(Option::is_none) {
        let Some(value) = value(line) else {
            return fixed;
        };
        return Stat {
            filter: Some(StatFilter {
                checked: false,
                limit: Some(Limit::new(value, reading.half(key), None, value, line)),
            }),
            ..fixed
        };
    }
    // A filter when one of its ranges is wider than one value.
    let Some(roll) = bar(magnitudes(line, &ranges), origin) else {
        return fixed;
    };
    let Some(value) = value(line) else {
        return fixed;
    };
    Stat {
        filter: Some(StatFilter {
            checked: true,
            // A legacy unique keeps its value, out of the range of today.
            limit: Some(Limit::new(
                value,
                reading.half(key),
                bounds(line, &ranges),
                value,
                line,
            )),
        }),
        roll: Some(roll),
        ..fixed
    }
}

#[cfg(test)]
mod tests;
