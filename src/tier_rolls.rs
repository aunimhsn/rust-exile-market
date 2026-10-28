//! The rolls of each line of a prefix or suffix of the copied item over every tier of its
//! modifier on the base of the item, from the modifier tables: its bar, the range its field
//! stays in and the rolls its tooltip tells. Read once when the item is copied, with the
//! advanced copy only: the plain copy tells neither the affix nor the tier of a modifier.

use std::ops::Range as Span;

use crate::filters::StatKey;
use crate::game_data::{self, Tier};
use crate::item::{Modifier, Range};
use crate::language::Language;
use crate::trade::stat_filters::LineStats;

/// The rolls of a line over every tier of its modifier.
#[derive(Debug, Clone, PartialEq)]
pub struct LineRolls {
    /// For each number of the line its stat counts, from the lowest roll of its worst tier to
    /// the highest of tier 1, as the game writes it: without its sign.
    pub ranges: Vec<Range>,
    /// The tier of the lowest roll and of the highest, for a line of one number.
    pub tiers: Option<(u32, u32)>,
}

/// The rolls over every tier of the lines of the copied item that have them.
#[derive(Debug, Default)]
pub struct TierRolls {
    lines: Vec<(StatKey, LineRolls)>,
}

impl TierRolls {
    /// The rolls of the lines of `modifiers`, written in `language`, whose numbers `stats`
    /// tell, on the base `base_type` (the name its search looks for; none for a magic item
    /// before the names of the trade site).
    #[must_use]
    pub fn new(
        language: Language,
        modifiers: &[Modifier],
        stats: &LineStats,
        base_type: Option<&str>,
    ) -> Self {
        let Some(base_type) = base_type else {
            return Self::default();
        };
        let mut lines = Vec::new();
        for (index, modifier) in modifiers.iter().enumerate() {
            if modifier.tier.is_none() {
                continue;
            }
            let matches = game_data::modifier_stats(language, modifier, None);
            let Some(tiers) = game_data::modifier_tiers(language, base_type, modifier, &matches)
            else {
                continue;
            };
            // The ranges of a tier follow the values of each line match, and the values of a
            // match its lines, each with the numbers its stat counts.
            let mut start = 0;
            for (found, &count) in matches.iter().zip(&tiers.counts) {
                let end = start + count;
                let mut next = start;
                for line in found.lines.clone() {
                    let key = StatKey {
                        modifier: index,
                        line,
                    };
                    let numbers = stats.numbers(key).map_or(0, <[usize]>::len);
                    if numbers == 0 {
                        continue;
                    }
                    let span = next..next + numbers;
                    next = span.end;
                    if next > end {
                        break;
                    }
                    if let Some(rolls) = line_rolls(&tiers.tiers, span) {
                        lines.push((key, rolls));
                    }
                }
                start = end;
            }
        }
        Self { lines }
    }

    /// The rolls of the line `key` over every tier; none for a line without them.
    #[must_use]
    pub fn get(&self, key: StatKey) -> Option<&LineRolls> {
        self.lines
            .iter()
            .find(|(line, _)| *line == key)
            .map(|(_, rolls)| rolls)
    }
}

/// The rolls over `tiers`, the best first, of the values at `span` among their ranges; none
/// when a tier hasn't them, or one of them crosses zero.
fn line_rolls(tiers: &[Tier], span: Span<usize>) -> Option<LineRolls> {
    let first = span.start;
    let mut ranges = Vec::new();
    for value in span {
        let mut over: Option<Range> = None;
        for tier in tiers {
            let range = tier.ranges.get(value)?.magnitude()?;
            over = Some(over.map_or(range, |over| Range {
                min: over.min.min(range.min),
                max: over.max.max(range.max),
            }));
        }
        ranges.push(over?);
    }
    let tiers = match ranges.as_slice() {
        [range] => {
            let rolls = |tier: &&Tier| tier.ranges.get(first).and_then(|range| range.magnitude());
            // The worst tier that rolls the lowest value, the best that rolls the highest.
            let lowest = tiers
                .iter()
                .rev()
                .find(|tier| rolls(tier).is_some_and(|rolls| rolls.min <= range.min))?;
            let highest = tiers
                .iter()
                .find(|tier| rolls(tier).is_some_and(|rolls| rolls.max >= range.max))?;
            Some((lowest.number, highest.number))
        }
        _ => None,
    };
    Some(LineRolls { ranges, tiers })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::{CopiedItem, Item};

    fn range(min: f64, max: f64) -> Range {
        Range { min, max }
    }

    fn rolls_of(text: &str) -> (Item, TierRolls) {
        let item = Item::parse(&CopiedItem::from_text(text).expect("an item")).expect("read");
        let stats = LineStats::new(item.language(), item.modifiers(), None);
        let rolls = TierRolls::new(
            item.language(),
            item.modifiers(),
            &stats,
            Some(item.base_type()),
        );
        (item, rolls)
    }

    fn key(modifier: usize, line: usize) -> StatKey {
        StatKey { modifier, line }
    }

    #[test]
    fn a_line_rolls_from_the_lowest_roll_of_its_worst_tier_to_the_highest_of_tier_1() {
        let (_, rolls) = rolls_of(
            "Item Class: Body Armours\nRarity: Rare\nDoom Shell\nAncestral Mail\n--------\n\
             Item Level: 82\n--------\n\
             { Prefix Modifier \"Vigorous\" (Tier: 7) — Life }\n+86(85-99) to maximum Life\n\
             { Prefix Modifier \"Lacquered\" (Tier: 4) — Defences }\n\
             +98(86-102) to Armour\n+88(79-94) to Evasion Rating\n\
             { Suffix Modifier \"of the Penguin\" (Tier: 1) — Elemental, Cold, Resistance }\n\
             +41(41-45)% to Cold Resistance",
        );

        assert_eq!(
            rolls.get(key(0, 0)),
            Some(&LineRolls {
                ranges: vec![range(10.0, 214.0)],
                tiers: Some((13, 1)),
            })
        );
        // Each line of a hybrid over the tiers of its own numbers.
        assert_eq!(
            rolls.get(key(1, 0)),
            Some(&LineRolls {
                ranges: vec![range(9.0, 170.0)],
                tiers: Some((8, 1)),
            })
        );
        assert_eq!(
            rolls.get(key(1, 1)),
            Some(&LineRolls {
                ranges: vec![range(6.0, 161.0)],
                tiers: Some((8, 1)),
            })
        );
        assert_eq!(
            rolls.get(key(2, 0)),
            Some(&LineRolls {
                ranges: vec![range(6.0, 45.0)],
                tiers: Some((8, 1)),
            })
        );
    }

    #[test]
    fn a_line_of_two_numbers_rolls_each_over_the_tiers_without_tiers_to_tell() {
        let (_, rolls) = rolls_of(
            "Item Class: Bows\nRarity: Rare\nStorm Bite\nGemini Bow\n--------\n\
             Item Level: 81\n--------\n\
             { Prefix Modifier \"Tempered\" (Tier: 2) — Damage, Physical, Attack }\n\
             Adds 21(21-31) to 38(36-53) Physical Damage\n\
             { Suffix Modifier \"of the Sniper\" (Tier: 2) — Gem }\n\
             +3 to Level of all Projectile Skills",
        );

        assert_eq!(
            rolls.get(key(0, 0)),
            Some(&LineRolls {
                ranges: vec![range(1.0, 39.0), range(4.0, 66.0)],
                tiers: None,
            })
        );
        // Fixed in its tier, not over the tiers.
        assert_eq!(
            rolls.get(key(1, 0)),
            Some(&LineRolls {
                ranges: vec![range(1.0, 4.0)],
                tiers: Some((4, 1)),
            })
        );
    }

    #[test]
    fn a_line_has_no_rolls_without_the_advanced_copy_nor_base() {
        let plain = "Item Class: Body Armours\nRarity: Rare\nDoom Shell\nAncestral Mail\n\
                     --------\nItem Level: 82\n--------\n+86 to maximum Life";
        let (_, rolls) = rolls_of(plain);
        assert_eq!(rolls.get(key(0, 0)), None);

        let advanced = "Item Class: Body Armours\nRarity: Rare\nDoom Shell\nAncestral Mail\n\
                        --------\nItem Level: 82\n--------\n\
                        { Prefix Modifier \"Vigorous\" (Tier: 7) — Life }\n\
                        +86(85-99) to maximum Life";
        let item = Item::parse(&CopiedItem::from_text(advanced).expect("an item")).expect("read");
        let stats = LineStats::new(item.language(), item.modifiers(), None);
        let rolls = TierRolls::new(item.language(), item.modifiers(), &stats, None);
        assert_eq!(rolls.get(key(0, 0)), None);
    }
}
