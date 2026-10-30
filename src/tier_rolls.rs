//! The rolls of each line of a prefix or suffix of the copied item over every tier of its
//! modifier on the base of the item, from the modifier tables: its bar, the range its field
//! stays in and the rolls its tooltip tells. Read once when the item is copied, for a modifier
//! whose header tells its affix and its tier, and for the other lines of a hybrid whose part a
//! line adds up, over the tiers of the hybrid; not for that line, a sum over no tiers.
//!
//! The ranges of a tier follow the values of each line match of the modifier, and the values
//! of a match the numbers its stat counts on its lines, in their order: the numbers are read
//! from the same matches as the tiers, so that each range lands on its number.

use crate::filters::StatKey;
use crate::game_data::{self, Tier};
use crate::item::{Modifier, Range, TierDoubt};
use crate::language::Language;
use crate::trade::stat_filters::counted_numbers;

/// The tiers of the lowest and of the highest roll of a line over every tier: "T13–T1".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TierSpan {
    pub lowest: u32,
    pub highest: u32,
}

/// The rolls of a line over every tier of its modifier.
#[derive(Debug, Clone, PartialEq)]
pub struct LineRolls {
    /// For each number of the line, in its order, from the lowest roll of its worst tier to
    /// the highest of tier 1, as the game writes it: without its sign; none for a number its
    /// stat doesn't count (the 3 of `#% chance to gain Onslaught for 3 seconds`).
    pub ranges: Vec<Option<Range>>,
    /// The tiers of its lowest and highest rolls, for a line of one number its stat counts.
    pub tiers: Option<TierSpan>,
}

/// The rolls over every tier of the lines of the copied item that have them.
#[derive(Debug, Default)]
pub struct TierRolls {
    lines: Vec<(StatKey, LineRolls)>,
}

impl TierRolls {
    /// The rolls of the lines of `modifiers`, written in `language`, on the base `base_type`
    /// (the name its search looks for; none for a magic item before the names of the trade
    /// site).
    #[must_use]
    pub fn new(language: Language, modifiers: &[Modifier], base_type: Option<&str>) -> Self {
        let Some(base_type) = base_type else {
            return Self::default();
        };
        let mut lines = Vec::new();
        for (index, modifier) in modifiers.iter().enumerate() {
            let summed = match &modifier.doubt {
                Some(TierDoubt::Split { line, .. }) => Some(*line),
                _ if modifier.tier.is_some() => None,
                _ => continue,
            };
            let matches = game_data::modifier_stats(language, modifier, None);
            let Some(tiers) = game_data::modifier_tiers(language, base_type, modifier, &matches)
            else {
                continue;
            };
            let mut start = 0;
            for (found, &count) in matches.iter().zip(&tiers.counts) {
                let numbers = counted_numbers(found, modifier);
                if let Some(values) = share_values(start, count, &numbers) {
                    for (line, values) in found.lines.clone().zip(values) {
                        if summed == Some(line) {
                            continue;
                        }
                        let rolls = modifier.lines.get(line).map_or(0, |line| line.rolls.len());
                        if let Some(rolls) = line_rolls(&tiers.tiers, &values, rolls) {
                            let key = StatKey {
                                modifier: index,
                                line,
                            };
                            lines.push((key, rolls));
                        }
                    }
                }
                start += count;
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

/// The values of a line match, `count` of them from `start` among the ranges of a tier,
/// shared among its lines in their order: for each line, the pairs of the index of a number
/// its stat counts (`numbers`, for each line) and the index of its value. None when the
/// numbers aren't as many as the values: a line without number whose string gives the value
/// (`Loads an additional bolt`) has no roll.
fn share_values(
    start: usize,
    count: usize,
    numbers: &[Vec<usize>],
) -> Option<Vec<Vec<(usize, usize)>>> {
    let total: usize = numbers.iter().map(Vec::len).sum();
    if total != count {
        return None;
    }
    let mut values = start..;
    Some(
        numbers
            .iter()
            .map(|line| line.iter().copied().zip(&mut values).collect())
            .collect(),
    )
}

/// The rolls over `tiers`, the best first, of a line of `rolls` numbers, whose `values` pair
/// the index of a number with the index of its value among the ranges of a tier; none for a
/// line without such a number, or when a tier hasn't the value, or its range crosses zero.
fn line_rolls(tiers: &[Tier], values: &[(usize, usize)], rolls: usize) -> Option<LineRolls> {
    if values.is_empty() {
        return None;
    }
    let mut ranges = vec![None; rolls];
    for &(number, value) in values {
        let over = tiers
            .iter()
            .map(|tier| tier.ranges.get(value)?.magnitude())
            .reduce(|over, range| {
                let (over, range) = (over?, range?);
                Some(Range {
                    min: over.min.min(range.min),
                    max: over.max.max(range.max),
                })
            })
            .flatten()?;
        *ranges.get_mut(number)? = Some(over);
    }
    let tiers = match values {
        &[(number, value)] => {
            let over = ranges.get(number).copied().flatten()?;
            let rolls = |tier: &&Tier| tier.ranges.get(value).and_then(|range| range.magnitude());
            // The worst tier that rolls the lowest value, the best that rolls the highest.
            let lowest = tiers
                .iter()
                .rev()
                .find(|tier| rolls(tier).is_some_and(|rolls| rolls.min <= over.min))?;
            let highest = tiers
                .iter()
                .find(|tier| rolls(tier).is_some_and(|rolls| rolls.max >= over.max))?;
            Some(TierSpan {
                lowest: lowest.number,
                highest: highest.number,
            })
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

    fn rolls_of(text: &str) -> TierRolls {
        let item = Item::parse(&CopiedItem::from_text(text).expect("an item")).expect("read");
        TierRolls::new(item.language(), item.modifiers(), Some(item.base_type()))
    }

    fn key(modifier: usize, line: usize) -> StatKey {
        StatKey { modifier, line }
    }

    fn one(min: f64, max: f64, lowest: u32, highest: u32) -> LineRolls {
        LineRolls {
            ranges: vec![Some(range(min, max))],
            tiers: Some(TierSpan { lowest, highest }),
        }
    }

    #[test]
    fn a_line_rolls_from_the_lowest_roll_of_its_worst_tier_to_the_highest_of_tier_1() {
        let rolls = rolls_of(
            "Item Class: Body Armours\nRarity: Rare\nDoom Shell\nAncestral Mail\n--------\n\
             Item Level: 82\n--------\n\
             { Prefix Modifier \"Vigorous\" (Tier: 7) — Life }\n+86(85-99) to maximum Life\n\
             { Prefix Modifier \"Lacquered\" (Tier: 4) — Defences }\n\
             +98(86-102) to Armour\n+88(79-94) to Evasion Rating\n\
             { Suffix Modifier \"of the Penguin\" (Tier: 1) — Elemental, Cold, Resistance }\n\
             +41(41-45)% to Cold Resistance",
        );

        assert_eq!(rolls.get(key(0, 0)), Some(&one(10.0, 214.0, 13, 1)));
        // Each line of a hybrid over the tiers of its own numbers.
        assert_eq!(rolls.get(key(1, 0)), Some(&one(9.0, 170.0, 8, 1)));
        assert_eq!(rolls.get(key(1, 1)), Some(&one(6.0, 161.0, 8, 1)));
        assert_eq!(rolls.get(key(2, 0)), Some(&one(6.0, 45.0, 8, 1)));
    }

    #[test]
    fn a_line_of_two_numbers_rolls_each_over_the_tiers_without_tiers_to_tell() {
        let rolls = rolls_of(
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
                ranges: vec![Some(range(1.0, 39.0)), Some(range(4.0, 66.0))],
                tiers: None,
            })
        );
        // Fixed in its tier, not over the tiers.
        assert_eq!(rolls.get(key(1, 0)), Some(&one(1.0, 4.0, 4, 1)));
    }

    #[test]
    fn a_reduced_line_rolls_without_its_sign() {
        let rolls = rolls_of(
            "Item Class: Belts\nRarity: Rare\nGlyph Cord\nDouble Belt\n--------\n\
             Item Level: 82\n--------\n\
             { Suffix Modifier \"of Sipping\" (Tier: 2) — Flask }\n\
             21(20-22)% reduced Flask Charges used",
        );

        // The table rolls -8 to -25: the game writes 8 to 25.
        assert_eq!(rolls.get(key(0, 0)), Some(&one(8.0, 25.0, 6, 1)));
    }

    #[test]
    fn a_french_line_rolls_over_the_tiers_of_its_english_modifier() {
        let rolls = rolls_of(include_str!("item/test-items/fr-rare-body-armour.txt"));

        // After its rune and its implicit, `+70(61-73) de Bouclier d'énergie maximal`, the
        // energy shield T4 among 11.
        assert_eq!(
            rolls.get(key(2, 0)).and_then(|rolls| rolls.tiers),
            Some(TierSpan {
                lowest: 11,
                highest: 1
            })
        );
    }

    #[test]
    fn a_line_has_no_rolls_without_its_tier_nor_base() {
        // A header the parser doesn't know gives no tier: its modifier isn't looked for in the
        // tables.
        let unknown = "Item Class: Body Armours\nRarity: Rare\nDoom Shell\nAncestral Mail\n\
                       --------\nItem Level: 82\n--------\n{ Strange Modifier }\n\
                       +86(85-99) to maximum Life";
        assert_eq!(rolls_of(unknown).get(key(0, 0)), None);

        let advanced = "Item Class: Body Armours\nRarity: Rare\nDoom Shell\nAncestral Mail\n\
                        --------\nItem Level: 82\n--------\n\
                        { Prefix Modifier \"Vigorous\" (Tier: 7) — Life }\n\
                        +86(85-99) to maximum Life";
        let item = Item::parse(&CopiedItem::from_text(advanced).expect("an item")).expect("read");
        let rolls = TierRolls::new(item.language(), item.modifiers(), None);
        assert_eq!(rolls.get(key(0, 0)), None);
    }

    #[test]
    fn the_other_line_of_a_hybrid_whose_part_a_line_adds_up_rolls_over_the_tiers_of_the_hybrid() {
        use crate::item::{Affix, ModifierKind, PartTier, StatLine};

        let part = |tiers: Vec<u32>, min, max| PartTier {
            affix: Affix::Prefix,
            tiers,
            ranges: vec![range(min, max)],
        };
        // 258% is a pure T1 (170–179) and the hybrid T1 (75–79) with the accuracy.
        let split = Modifier {
            kind: ModifierKind::Explicit(Some(Affix::Prefix)),
            origin: None,
            name: None,
            tier: None,
            tags: Vec::new(),
            unrevealed: false,
            lines: vec![
                StatLine::parse("258% increased Physical Damage"),
                StatLine::parse("+195(175-200) to Accuracy Rating"),
            ],
            doubt: Some(TierDoubt::Split {
                line: 0,
                whole: part(vec![1], 75.0, 79.0),
                added: part(vec![1], 170.0, 179.0),
            }),
        };

        let rolls = TierRolls::new(
            Language::English,
            std::slice::from_ref(&split),
            Some("Warmonger Bow"),
        );

        // A sum over no tiers; the accuracy over the 8 tiers of the hybrid.
        assert_eq!(rolls.get(key(0, 0)), None);
        assert_eq!(rolls.get(key(0, 1)), Some(&one(16.0, 200.0, 8, 1)));

        // A part whose ranges the hybrid doesn't give the line: another table, no rolls.
        let mut other = split.clone();
        other.doubt = Some(TierDoubt::Split {
            line: 0,
            whole: part(vec![1], 76.0, 79.0),
            added: part(vec![1], 170.0, 179.0),
        });
        let rolls = TierRolls::new(Language::English, &[other], Some("Warmonger Bow"));
        assert_eq!(rolls.get(key(0, 1)), None);

        // Two pure modifiers on one line: no rolls.
        let mut pure = split;
        pure.lines.truncate(1);
        pure.doubt = Some(TierDoubt::Split {
            line: 0,
            whole: part(vec![1], 75.0, 79.0),
            added: part(vec![1], 170.0, 179.0),
        });
        let rolls = TierRolls::new(Language::English, &[pure], Some("Warmonger Bow"));
        assert_eq!(rolls.get(key(0, 0)), None);
    }

    #[test]
    fn the_values_of_a_match_are_shared_among_its_lines() {
        // A stat on two lines, a number on each.
        assert_eq!(
            share_values(3, 2, &[vec![0], vec![0]]),
            Some(vec![vec![(0, 3)], vec![(0, 4)]])
        );
        // A stat on two lines, its number on the first only: the other line has no roll.
        assert_eq!(
            share_values(0, 1, &[vec![0], vec![]]),
            Some(vec![vec![(0, 0)], vec![]])
        );
        // Two numbers on a line.
        assert_eq!(
            share_values(1, 2, &[vec![0, 1]]),
            Some(vec![vec![(0, 1), (1, 2)]])
        );
        // A line without number whose string gives the value: no roll.
        assert_eq!(share_values(0, 1, &[vec![]]), None);
        // More numbers than values.
        assert_eq!(share_values(0, 1, &[vec![0, 1]]), None);
    }

    #[test]
    fn a_range_across_zero_has_no_rolls() {
        let tier = |number, min, max| Tier {
            number,
            level: 1,
            ranges: vec![range(min, max)],
        };

        assert_eq!(
            line_rolls(&[tier(1, 5.0, 10.0), tier(2, 1.0, 4.0)], &[(0, 0)], 1),
            Some(one(1.0, 10.0, 2, 1))
        );
        assert_eq!(
            line_rolls(&[tier(1, 5.0, 10.0), tier(2, -4.0, 4.0)], &[(0, 0)], 1),
            None
        );
        // A value no tier has.
        assert_eq!(line_rolls(&[tier(1, 5.0, 10.0)], &[(0, 1)], 1), None);
        // A line of two numbers whose stat counts the first only: the 3 of `for 3 seconds`.
        assert_eq!(
            line_rolls(&[tier(1, 5.0, 10.0), tier(2, 1.0, 4.0)], &[(0, 0)], 2),
            Some(LineRolls {
                ranges: vec![Some(range(1.0, 10.0)), None],
                tiers: Some(TierSpan {
                    lowest: 2,
                    highest: 1
                }),
            })
        );
    }
}
