//! The modifier tables, from RePoE (`assets/game-data/modifiers.ndjson`): every tier of a
//! prefix or suffix on the base of an item, found from the stats of its lines.

use std::sync::LazyLock;

use serde::Deserialize;

use super::bases;
use super::lines::{LineMatch, StatSource};
use crate::item::{Affix, Modifier, ModifierKind, Range};
use crate::language::Language;

/// How far a copied value can be from a tier: the game rounds the values it scales, e.g. a
/// regeneration stored per minute shown per second with one decimal.
const ROUNDING: f64 = 0.051;

/// The tiers of a type of modifier that roll on the same bases, as the generator writes them.
#[derive(Deserialize)]
struct Table {
    /// The modifiers of one type can't be on the same item: its tiers.
    #[serde(rename = "type")]
    kind: String,
    affix: TableAffix,
    /// The English template of each line, as Exiled Exchange 2 writes it.
    lines: Vec<String>,
    /// Each tag and its weight: the first tag of the list a base has tells whether the
    /// modifier rolls on it (above zero) or not.
    spawn: Vec<(String, u32)>,
    tiers: Vec<TableTier>,
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum TableAffix {
    Prefix,
    Suffix,
}

#[derive(Deserialize)]
struct TableTier {
    level: u32,
    /// The range of each `#` of the lines, in their order, with the sign of the stat.
    ranges: Vec<[f64; 2]>,
}

impl Table {
    /// Whether the modifier rolls on a base with `tags`.
    fn rolls_on(&self, tags: &[String]) -> bool {
        self.spawn
            .iter()
            .find(|(tag, _)| tags.contains(tag))
            .is_some_and(|&(_, weight)| weight > 0)
    }
}

static TABLES: LazyLock<Vec<Table>> =
    LazyLock::new(|| read(include_str!("../../assets/game-data/modifiers.ndjson")));

/// A tier of a modifier on a base.
#[derive(Debug, Clone, PartialEq)]
pub struct Tier {
    /// 1 is the best.
    pub number: u32,
    /// The level an item needs for the tier to roll.
    pub level: u32,
    /// The range of each value of the copied lines, in their order, with the sign of the stat
    /// (`#% reduced` rolls negative values).
    pub ranges: Vec<Range>,
}

/// Every tier of a modifier on the base of its item.
#[derive(Debug, Clone, PartialEq)]
pub struct ModifierTiers {
    /// The best first: tier 1, tier 2…
    pub tiers: Vec<Tier>,
    /// The index in `tiers` of the tier of the copied modifier.
    pub found: usize,
}

impl ModifierTiers {
    /// The tier of the copied modifier.
    #[must_use]
    pub fn tier(&self) -> Option<&Tier> {
        self.tiers.get(self.found)
    }
}

/// Every tier of `modifier`, a prefix or suffix of an item whose base type is `base_type` in
/// `language` (the name its search looks for), its lines matched with their stats in
/// `matches` ([`super::modifier_stats`]).
///
/// None for a modifier the tables don't hold: an implicit, a rune, an enchantment, a line
/// without stat, a base without tags; or whose values (and ranges, with the advanced copy) are
/// in no tier: a modifier of the lords of the Abyss is desecrated, like some ordinary ones,
/// and rolls its own range.
#[must_use]
pub fn modifier_tiers(
    language: Language,
    base_type: &str,
    modifier: &Modifier,
    matches: &[LineMatch<'_>],
) -> Option<ModifierTiers> {
    let affixes: &[TableAffix] = match modifier.kind {
        ModifierKind::Explicit(Some(Affix::Prefix)) => &[TableAffix::Prefix],
        ModifierKind::Explicit(Some(Affix::Suffix)) => &[TableAffix::Suffix],
        // The plain copy doesn't tell a prefix from a suffix.
        ModifierKind::Explicit(None) => &[TableAffix::Prefix, TableAffix::Suffix],
        _ => return None,
    };
    let tags = bases::tags(language, base_type)?;
    let copied = Copied::of(modifier, matches)?;

    let mut best: Option<(Fit, ModifierTiers)> = None;
    let mut seen: Vec<(&str, TableAffix)> = Vec::new();
    for table in TABLES.iter() {
        let key = (table.kind.as_str(), table.affix);
        if !affixes.contains(&table.affix) || seen.contains(&key) {
            continue;
        }
        let Some(order) = copied.order(&table.lines) else {
            continue;
        };
        seen.push(key);
        // Every table of the type, prefix or suffix, that rolls on the base: a type can roll
        // other ranges on other bases (the added damage of a two-handed weapon).
        let mut tiers: Vec<&TableTier> = TABLES
            .iter()
            .filter(|other| {
                (other.kind.as_str(), other.affix) == key
                    && other.lines == table.lines
                    && other.rolls_on(tags)
            })
            .flat_map(|other| &other.tiers)
            .collect();
        tiers.sort_by_key(|tier| tier.level);
        let tiers: Option<Vec<Tier>> = tiers
            .iter()
            .rev()
            .zip(1..)
            .map(|(tier, number)| {
                Some(Tier {
                    number,
                    level: tier.level,
                    ranges: order
                        .iter()
                        .map(|&index| {
                            let [min, max] = *tier.ranges.get(index)?;
                            Some(Range {
                                min: min.min(max),
                                max: min.max(max),
                            })
                        })
                        .collect::<Option<_>>()?,
                })
            })
            .collect();
        let Some(tiers) = tiers else {
            continue;
        };
        let found = tiers
            .iter()
            .enumerate()
            .filter_map(|(index, tier)| Some((copied.fit(tier)?, index)))
            .min_by_key(|&(fit, _)| fit);
        if let Some((fit, found)) = found
            && best.as_ref().is_none_or(|(best, _)| fit < *best)
        {
            best = Some((fit, ModifierTiers { tiers, found }));
        }
    }
    best.map(|(_, tiers)| tiers)
}

/// How well a tier holds the copied modifier, the best first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Fit {
    /// Its values are within the ranges of the tier.
    Within,
    /// Its values are within the ranges of the tier, give or take the rounding of the game.
    Rounded,
}

/// The values of a copied modifier, and the ranges of its tier when the advanced copy gave
/// them, with the English template of each line.
struct Copied<'a> {
    /// The English template of each stat of the modifier, and the index of its first value.
    stats: Vec<(&'a str, usize)>,
    values: Vec<f64>,
    /// The ranges the advanced copy gave, as the game writes them: without their sign.
    ranges: Option<Vec<Range>>,
}

impl<'a> Copied<'a> {
    fn of(modifier: &Modifier, matches: &[LineMatch<'a>]) -> Option<Self> {
        let mut stats = Vec::new();
        let mut values = Vec::new();
        let mut ranges = Some(Vec::new());
        for line_match in matches {
            let stat = line_match.stat.as_ref()?;
            // A line the trade site only knows has no template of Exiled Exchange 2.
            if stat.source != StatSource::GameData {
                return None;
            }
            stats.push((stat.stat.reference.as_str(), values.len()));
            values.extend(&stat.values);
            let rolls = modifier
                .lines
                .get(line_match.lines.clone())?
                .iter()
                .flat_map(|line| &line.rolls);
            let counted = rolls
                .zip(&stat.counted)
                .filter_map(|(roll, &counted)| counted.then_some(roll.range));
            for range in counted {
                match (&mut ranges, range) {
                    (Some(ranges), Some(range)) => ranges.push(range),
                    _ => ranges = None,
                }
            }
        }
        let ranges = ranges.filter(|ranges| ranges.len() == values.len());
        Some(Self {
            stats,
            values,
            ranges,
        })
    }

    /// For each value of the copied lines, the index of its range in a tier of the table whose
    /// lines are `lines`: its lines in another order are the same modifier. None when the
    /// lines differ.
    fn order(&self, lines: &[String]) -> Option<Vec<usize>> {
        if lines.len() != self.stats.len() {
            return None;
        }
        let mut order = vec![0; self.values.len()];
        let mut used = vec![false; lines.len()];
        let mut first_in_table = 0;
        let mut starts = Vec::new();
        for line in lines {
            starts.push(first_in_table);
            first_in_table += line.matches('#').count();
        }
        if first_in_table != self.values.len() {
            return None;
        }
        for (index, &(reference, first)) in self.stats.iter().enumerate() {
            let line = (0..lines.len()).find(|&line| {
                !used.get(line).copied().unwrap_or(true)
                    && lines.get(line).is_some_and(|text| text == reference)
            })?;
            *used.get_mut(line)? = true;
            let next = self
                .stats
                .get(index + 1)
                .map_or(self.values.len(), |&(_, next)| next);
            let start = *starts.get(line)?;
            for (offset, slot) in order.get_mut(first..next)?.iter_mut().enumerate() {
                *slot = start + offset;
            }
        }
        Some(order)
    }

    /// How well `tier` holds the copied modifier; none when it doesn't hold it, or its ranges
    /// aren't those the advanced copy gave.
    fn fit(&self, tier: &Tier) -> Option<Fit> {
        if let Some(ranges) = &self.ranges {
            let same = ranges.iter().zip(&tier.ranges).all(|(copied, range)| {
                range.magnitude().is_some_and(|range| {
                    (copied.min - range.min).abs() < ROUNDING
                        && (copied.max - range.max).abs() < ROUNDING
                })
            });
            if !same {
                return None;
            }
        }
        let within = |margin: f64| {
            self.values
                .iter()
                .zip(&tier.ranges)
                .all(|(&value, range)| range.min - margin <= value && value <= range.max + margin)
        };
        if within(0.0) {
            Some(Fit::Within)
        } else if within(ROUNDING) {
            Some(Fit::Rounded)
        } else {
            None
        }
    }
}

/// Reads one table per line; a line that can't be read is logged and left out (the tests read
/// every line of the embedded file).
fn read(text: &str) -> Vec<Table> {
    let mut tables = Vec::new();
    for (index, line) in text.lines().enumerate() {
        match serde_json::from_str::<Table>(line) {
            Ok(table) => tables.push(table),
            Err(error) => log::error!(
                "Game data, modifier table {} unreadable: {error}",
                index + 1
            ),
        }
    }
    tables
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_data::modifier_stats;
    use crate::item::{CopiedItem, Item};

    #[test]
    fn every_table_of_the_embedded_file_is_read() {
        let text = include_str!("../../assets/game-data/modifiers.ndjson");

        assert_eq!(TABLES.len(), text.lines().count());
    }

    /// A tier number and the ranges of its values, as the game writes them.
    type TierRanges = (u32, Vec<Range>);

    /// The tier found from the tables for each prefix and suffix of `text`, and the tier the
    /// advanced copy writes, with its ranges: (found, written). A magic item gives its
    /// `base_type`, found among the names of the trade site in the application.
    fn tiers_of(text: &str, base_type: Option<&str>) -> Vec<(Option<TierRanges>, TierRanges)> {
        let item = Item::parse(&CopiedItem::from_text(text).expect("an item")).expect("read");
        let base_type = base_type.unwrap_or(item.base_type());
        item.modifiers()
            .iter()
            .filter(|modifier| {
                matches!(modifier.kind, ModifierKind::Explicit(Some(_))) && modifier.tier.is_some()
            })
            .map(|modifier| {
                let matches = modifier_stats(item.language(), modifier, None);
                let found = modifier_tiers(item.language(), base_type, modifier, &matches)
                    .and_then(|tiers| {
                        let tier = tiers.tier()?;
                        Some((
                            tier.number,
                            tier.ranges
                                .iter()
                                .filter_map(|range| range.magnitude())
                                .collect(),
                        ))
                    });
                let written = (
                    modifier.tier.expect("a tier"),
                    modifier
                        .lines
                        .iter()
                        .flat_map(|line| &line.rolls)
                        .filter_map(|roll| roll.range)
                        .collect(),
                );
                (found, written)
            })
            .collect()
    }

    #[test]
    fn the_tables_find_the_tier_written_by_the_advanced_copy() {
        let mut found = 0;
        let mut missing = Vec::new();
        for (text, base_type) in [
            (
                include_str!("../item/test-items/en-rare-crossbow-all-modifiers.txt"),
                None,
            ),
            (
                include_str!("../item/test-items/en-rare-bow-fractured.txt"),
                None,
            ),
            (
                include_str!("../item/test-items/en-magic-mace.txt"),
                Some("Temple Maul"),
            ),
            (include_str!("../item/test-items/fr-rare-staff.txt"), None),
            (
                include_str!("../item/test-items/fr-rare-body-armour.txt"),
                None,
            ),
        ] {
            for (tier, (number, ranges)) in tiers_of(text, base_type) {
                // The crossbow was copied before a patch added a tier above its leech, `of
                // the Desperate` (8–8.9), the S1 of the offers checked on the trade site on
                // 2026-10-27.
                let leech = [Range { min: 7.0, max: 7.9 }];
                let number = if ranges == leech { 2 } else { number };
                if let Some(tier) = tier {
                    assert_eq!(tier, (number, ranges));
                    found += 1;
                } else {
                    missing.push(ranges);
                }
            }
        }
        assert_eq!(found, 21);
        // A line whose stat the tables have under another tier is a mistake; a line the tables
        // don't have isn't: the lightning damage of the mace, copied before a patch changed
        // its ranges, and the two modifiers of Amanamu, a lord of the Abyss, desecrated with
        // ranges of their own.
        let range = |min, max| Range { min, max };
        assert_eq!(
            missing,
            [
                vec![range(1.0, 4.0), range(46.0, 66.0)],
                vec![range(40.0, 50.0)],
                vec![range(13.0, 17.0)],
            ]
        );
    }

    #[test]
    fn a_tier_is_numbered_among_the_tiers_of_the_base() {
        let text = include_str!("../item/test-items/en-rare-crossbow-all-modifiers.txt");
        let item = Item::parse(&CopiedItem::from_text(text).expect("an item")).expect("read");
        let lightning = item
            .modifiers()
            .iter()
            .find(|modifier| modifier.name.as_deref() == Some("Electrocuting"))
            .expect("the lightning damage");
        let matches = modifier_stats(item.language(), lightning, None);

        let tiers = modifier_tiers(item.language(), item.base_type(), lightning, &matches)
            .expect("its tiers");

        // The tiers of a two-handed weapon, not those of a one-handed one.
        assert_eq!(tiers.tiers.len(), 10);
        assert_eq!(
            tiers.tiers.first().map(|tier| tier.ranges.clone()),
            Some(vec![
                Range {
                    min: 1.0,
                    max: 19.0
                },
                Range {
                    min: 310.0,
                    max: 358.0
                }
            ])
        );
        assert_eq!(tiers.tiers.last().map(|tier| tier.level), Some(1));
        assert_eq!(tiers.tier().map(|tier| tier.number), Some(2));
    }
}
