//! The rolls of each line of a prefix or suffix of the copied item over every tier of its
//! modifier on the base of the item, from the modifier tables: its bar, the range its field
//! stays in and the rolls its tooltip tells. Read once when the item is copied, for a modifier
//! whose header tells its affix and its tier, and for the other lines of a hybrid whose part a
//! line adds up, over the tiers of the hybrid; not for that line, a sum over no tiers. A
//! modifier of one tier only on the base has no tier to tell: its lines roll over that tier. Nor
//! has a modifier whose tier the game doesn't write, of the Genesis tree: its lines roll over the
//! ranges of the advanced copy (`Reinforcing`, `Minions have 21(19-22)% increased maximum
//! Life`), or over the tier the tables find for a fixed value, which the copy writes without
//! range (`of the Despot` of a belt, `+2 to Level of all Minion Skills`).
//!
//! The ranges of a tier follow the values of each line match of the modifier, and the values
//! of a match the numbers its stat counts on its lines, in their order: the numbers are read
//! from the same matches as the tiers, so that each range lands on its number.

use crate::filters::StatKey;
use crate::game_data::{self, Tier};
use crate::item::{Modifier, ModifierKind, Range, TierDoubt};
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
    /// stat doesn't count (the 3 of `#% chance to gain Onslaught for 3 seconds`), except for a
    /// modifier written without tier, the range the copy writes of every number, those its
    /// stat doesn't count left out by the reader (`Reading::pick` in `src/ui/item_stats.rs`).
    pub ranges: Vec<Option<Range>>,
    /// The tiers of its lowest and highest rolls, for a line of one number its stat counts.
    pub tiers: Option<TierSpan>,
}

/// The rolls over every tier of the lines of the copied item that have them.
#[derive(Debug, Default)]
pub struct TierRolls {
    lines: Vec<(StatKey, LineRolls)>,
    /// The index of each modifier without tier to tell: of one tier only on the base of the
    /// item, whose T1 would read as "the best" while it is "the only one", or whose tier the game
    /// doesn't write (the Genesis tree).
    single: Vec<usize>,
}

impl TierRolls {
    /// The rolls of the lines of `modifiers`, written in `language`, on the base `base_type`
    /// (the name its search looks for; none for a magic item before the names of the trade
    /// site) of an item of `item_level` (none when the copy doesn't write it), which the tiers
    /// of a split line are among.
    #[must_use]
    pub fn new(
        language: Language,
        modifiers: &[Modifier],
        base_type: Option<&str>,
        item_level: Option<u32>,
    ) -> Self {
        let mut lines = Vec::new();
        let mut single = Vec::new();
        for (index, modifier) in modifiers.iter().enumerate() {
            // Its copied ranges, whatever its base.
            if written_without_tier(modifier) {
                single.push(index);
                lines.extend(copied_rolls(index, modifier));
                continue;
            }
            let Some(base_type) = base_type else {
                continue;
            };
            let fixed = fixed_without_tier(modifier);
            let summed = match &modifier.doubt {
                Some(TierDoubt::Split { line, .. }) => Some(*line),
                _ if modifier.tier.is_some() || fixed => None,
                _ => continue,
            };
            let matches = game_data::modifier_stats(language, modifier, None);
            let Some(tiers) =
                game_data::modifier_tiers(language, base_type, item_level, modifier, &matches)
            else {
                continue;
            };
            // A fixed value without tier: only of a modifier whose tier the game doesn't write
            // (the Genesis tree, an essence), over that tier, as a line of the in-game trade
            // read in it, a tier of fixed values too (not a value of a modifier whose tier the
            // game writes, nor one that falls in a tier of the tree that rolls, `19–22`).
            let rolled = if fixed {
                let unwritten = |tier: &&Tier| {
                    tiers.tier_unwritten && tier.ranges.iter().all(|range| range.is_one_value())
                };
                match tiers.tier().filter(unwritten) {
                    Some(tier) => std::slice::from_ref(tier),
                    None => continue,
                }
            } else {
                tiers.tiers.as_slice()
            };
            // The hybrid of a split line of one tier only too: its other lines tell no tiers.
            let one_tier = tiers.single() || fixed;
            if fixed {
                single.push(index);
            } else if one_tier && summed.is_none() {
                match modifier.tier {
                    Some(1) => single.push(index),
                    // The game knows more tiers than the data, e.g. after a patch: the tier
                    // the copy writes is kept.
                    tier => log::warn!(
                        "Game data, one tier only for a modifier copied as tier {tier:?}: {}",
                        modifier.lines.first().map_or("", |line| line.text.as_str())
                    ),
                }
            }
            let mut start = 0;
            for (found, &count) in matches.iter().zip(&tiers.counts) {
                let numbers = counted_numbers(found, modifier);
                if let Some(values) = share_values(start, count, &numbers) {
                    for (line, values) in found.lines.clone().zip(values) {
                        if summed == Some(line) {
                            continue;
                        }
                        let rolls = modifier.lines.get(line).map_or(0, |line| line.rolls.len());
                        if let Some(mut rolls) = line_rolls(rolled, &values, rolls) {
                            if one_tier {
                                // "T1–T1" would tell nothing.
                                rolls.tiers = None;
                            }
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
        Self { lines, single }
    }

    /// The rolls of the line `key` over every tier; none for a line without them.
    #[must_use]
    pub fn get(&self, key: StatKey) -> Option<&LineRolls> {
        self.lines
            .iter()
            .find(|(line, _)| *line == key)
            .map(|(_, rolls)| rolls)
    }

    /// Whether the modifier `index` has no tier to tell: one tier only on the base of the item,
    /// or a tier the game doesn't write.
    #[must_use]
    pub fn single(&self, index: usize) -> bool {
        self.single.contains(&index)
    }
}

/// Whether the advanced copy writes `modifier`, a prefix or suffix, with its ranges but without
/// tier: the game writes none for a modifier of the Genesis tree (`Reinforcing`, `Minions have
/// 21(19-22)% increased maximum Life`, the third of its six tiers), nor does the application for
/// such a line copied from the in-game trade (`with_trade_tiers`).
fn written_without_tier(modifier: &Modifier) -> bool {
    matches!(modifier.kind, ModifierKind::Explicit(Some(_)))
        && modifier.tier.is_none()
        && modifier.doubt.is_none()
        && !modifier.unrevealed
        && modifier
            .lines
            .iter()
            .any(|line| line.rolls.iter().any(|roll| roll.range.is_some()))
}

/// Whether the advanced copy writes `modifier`, a prefix or suffix, without tier nor range: a
/// fixed value of a modifier whose tier the game doesn't write (`of the Despot` of the Genesis
/// tree on a belt, `+2 to Level of all Minion Skills`; `of the Essence` on a bow, `+2 to Level of
/// all Attack Skills`).
fn fixed_without_tier(modifier: &Modifier) -> bool {
    matches!(modifier.kind, ModifierKind::Explicit(Some(_)))
        && modifier.tier.is_none()
        && modifier.doubt.is_none()
        && !modifier.unrevealed
        && modifier
            .lines
            .iter()
            .all(|line| line.rolls.iter().all(|roll| roll.range.is_none()))
}

/// The rolls of each line of the modifier `index`, `modifier`, over the ranges the advanced copy
/// writes, without their sign.
fn copied_rolls(index: usize, modifier: &Modifier) -> impl Iterator<Item = (StatKey, LineRolls)> {
    modifier
        .lines
        .iter()
        .enumerate()
        .filter_map(move |(line, text)| {
            let ranges: Vec<Option<Range>> = text
                .rolls
                .iter()
                .map(|roll| roll.range.and_then(Range::magnitude))
                .collect();
            ranges.iter().any(Option::is_some).then(|| {
                let key = StatKey {
                    modifier: index,
                    line,
                };
                (
                    key,
                    LineRolls {
                        ranges,
                        tiers: None,
                    },
                )
            })
        })
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
                Some(over.spanning(range))
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
        TierRolls::new(
            item.language(),
            item.modifiers(),
            Some(item.base_type()),
            item.item_level(),
        )
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
    fn a_modifier_of_one_tier_only_on_its_base_has_no_tier_to_tell() {
        let rolls = rolls_of(
            "Item Class: Jewels\nRarity: Rare\nViper Glimmer\nRuby\n--------\n\
             Item Level: 43\n--------\n\
             { Prefix Modifier \"Brutish\" (Tier: 1) — Minion, Life }\n\
             Minions have 12(5-15)% increased maximum Life",
        );

        assert!(rolls.single(0));
        // Over its only tier, without "T1–T1".
        assert_eq!(
            rolls.get(key(0, 0)),
            Some(&LineRolls {
                ranges: vec![Some(range(5.0, 15.0))],
                tiers: None,
            })
        );
    }

    #[test]
    fn a_modifier_copied_with_another_tier_than_the_only_one_of_the_data_keeps_it() {
        // After a patch that adds a tier the data doesn't know yet.
        let rolls = rolls_of(
            "Item Class: Jewels\nRarity: Rare\nViper Glimmer\nRuby\n--------\n\
             Item Level: 43\n--------\n\
             { Prefix Modifier \"Brutish\" (Tier: 2) — Minion, Life }\n\
             Minions have 12(5-15)% increased maximum Life",
        );

        assert!(!rolls.single(0));
        // Nor "T1–T1" against its T2.
        assert_eq!(rolls.get(key(0, 0)).map(|rolls| rolls.tiers), Some(None));
    }

    #[test]
    fn a_hybrid_of_one_tier_only_has_no_tier_to_tell() {
        let flask = "Item Class: Life Flasks\nRarity: Magic\nSaturated Ultimate Life Flask\n\
                     --------\nItem Level: 82\n--------\n\
                     { Prefix Modifier \"Saturated\" (Tier: 1) — Life }\n\
                     Instant Recovery\n50(50)% reduced Amount Recovered";
        let item = Item::parse(&CopiedItem::from_text(flask).expect("an item")).expect("read");
        // The base of a magic item, from the names of the trade site.
        let rolls = TierRolls::new(
            item.language(),
            item.modifiers(),
            Some("Ultimate Life Flask"),
            item.item_level(),
        );

        assert!(rolls.single(0));
        // A line without number has no rolls.
        assert_eq!(rolls.get(key(0, 0)), None);
        assert_eq!(
            rolls.get(key(0, 1)),
            Some(&LineRolls {
                ranges: vec![Some(range(50.0, 50.0))],
                tiers: None,
            })
        );
    }

    #[test]
    fn a_fixed_value_without_tier_rolls_over_its_tier_only_of_the_genesis_tree_or_an_essence() {
        // `of the Despot` of the Genesis tree on a belt, the best of its two tiers (+1, +2),
        // which the copy writes without tier nor range: fixed in its tier.
        let belt = rolls_of(
            "Item Class: Belts\nRarity: Rare\nGrip Clasp\nHeavy Belt\n--------\n\
             Item Level: 79\n--------\n\
             { Suffix Modifier \"of the Despot\" — Minion, Gem }\n\
             +2 to Level of all Minion Skills",
        );
        assert!(belt.single(0));
        let fixed = LineRolls {
            ranges: vec![Some(range(2.0, 2.0))],
            tiers: None,
        };
        assert_eq!(belt.get(key(0, 0)), Some(&fixed));

        // An essence on a bow, of one value.
        let bow = rolls_of(
            "Item Class: Bows\nRarity: Rare\nMiracle Siege\nObliterator Bow\n--------\n\
             Item Level: 81\n--------\n\
             { Suffix Modifier \"of the Essence\" — Attack }\n\
             +2 to Level of all Attack Skills",
        );
        assert!(bow.single(0));
        assert_eq!(bow.get(key(0, 0)), Some(&fixed));
        // On a wand, where its value is the ordinary T2 of 4 as well: the essence's, fixed.
        let wand = |header: &str| {
            rolls_of(&format!(
                "Item Class: Wands\nRarity: Rare\nWoe Song\nAcrid Wand\n--------\n\
                 Item Level: 81\n--------\n\
                 {{ Suffix Modifier {header} — Caster, Gem }}\n\
                 +3 to Level of all Spell Skills"
            ))
        };
        let essence = wand("\"of the Essence\"");
        assert!(essence.single(0));
        assert_eq!(
            essence.get(key(0, 0)),
            Some(&LineRolls {
                ranges: vec![Some(range(3.0, 3.0))],
                tiers: None,
            })
        );
        assert!(!wand("\"of the Magus\" (Tier: 2)").single(0));

        // The same without tier on a quiver, a modifier of the tables, of one tier there, but
        // neither of the tree nor of an essence (none adds it): no rolls.
        let quiver = |header: &str| {
            rolls_of(&format!(
                "Item Class: Quivers\nRarity: Rare\nStorm Song\nFire Quiver\n--------\n\
                 Item Level: 79\n--------\n\
                 {{ Suffix Modifier {header} — Attack, Gem }}\n\
                 +1 to Level of all Projectile Skills"
            ))
        };
        assert!(quiver("\"of the Archer\" (Tier: 1)").single(0));
        let essence = quiver("\"of the Essence\"");
        assert!(!essence.single(0));
        assert_eq!(essence.get(key(0, 0)), None);

        // Nor a value of an essence that falls in a tier of the tree that rolls, `Reinforcing`
        // (19–22), which the copy would write with its range.
        let ring = rolls_of(
            "Item Class: Rings\nRarity: Rare\nGlyph Circle\nBreach Ring\n--------\n\
             Item Level: 77\n--------\n\
             { Prefix Modifier \"of the Essence\" — Life, Minion }\n\
             Minions have 21% increased maximum Life",
        );
        assert!(!ring.single(0));
        assert_eq!(ring.get(key(0, 0)), None);
    }

    #[test]
    fn a_modifier_written_without_tier_rolls_over_its_copied_range() {
        let rolls = rolls_of(
            "Item Class: Rings\nRarity: Rare\nGlyph Circle\nBreach Ring\n--------\n\
             Item Level: 77\n--------\n\
             { Prefix Modifier \"Reinforcing\" — Life, Minion }\n\
             Minions have 21(19-22)% increased maximum Life\n\
             { Prefix Modifier \"Aqua\" (Tier: 7) — Mana }\n\
             +76(65-79) to maximum Mana",
        );

        // The Genesis tree, whose tier the game doesn't write: over the copied range.
        assert!(rolls.single(0));
        assert_eq!(
            rolls.get(key(0, 0)),
            Some(&LineRolls {
                ranges: vec![Some(range(19.0, 22.0))],
                tiers: None,
            })
        );
        assert!(!rolls.single(1));
        assert!(
            rolls
                .get(key(1, 0))
                .is_some_and(|rolls| rolls.tiers.is_some())
        );

        // Copied from the in-game trade, without its range: no rolls.
        let rolls = rolls_of(
            "Item Class: Rings\nRarity: Rare\nGlyph Circle\nBreach Ring\n--------\n\
             Item Level: 77\n--------\n\
             Minions have 21% increased maximum Life",
        );
        assert!(!rolls.single(0));
        assert_eq!(rolls.get(key(0, 0)), None);

        // In French, and before the base is known (a magic item before the names of the trade
        // site): its copied range needs no base.
        let french = "Classe d'objet: Bagues\nRareté: Rare\nCercle glyphe\nBague de Brèche\n\
                      --------\nNiveau de l'objet: 77\n--------\n\
                      { Préfixe : « Renforçant » — Vie, Créature }\n\
                      Les Créatures ont 21(19-22)% d'Augmentation de Vie maximale";
        let item = Item::parse(&CopiedItem::from_text(french).expect("an item")).expect("read");
        let rolls = TierRolls::new(item.language(), item.modifiers(), None, None);
        assert!(rolls.single(0));
        assert_eq!(
            rolls.get(key(0, 0)),
            Some(&LineRolls {
                ranges: vec![Some(range(19.0, 22.0))],
                tiers: None,
            })
        );
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
        let rolls = TierRolls::new(item.language(), item.modifiers(), None, None);
        assert_eq!(rolls.get(key(0, 0)), None);
    }

    #[test]
    fn the_other_line_of_a_hybrid_whose_part_a_line_adds_up_rolls_over_the_tiers_of_the_hybrid() {
        use crate::item::{Affix, ModifierKind, PartTier, StatLine};

        let part = |tiers: Vec<u32>, min, max| PartTier {
            affix: Affix::Prefix,
            tiers,
            hides_number: false,
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
            None,
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
        let rolls = TierRolls::new(Language::English, &[other], Some("Warmonger Bow"), None);
        assert_eq!(rolls.get(key(0, 1)), None);

        // Two pure modifiers on one line: no rolls.
        let mut pure = split;
        pure.lines.truncate(1);
        pure.doubt = Some(TierDoubt::Split {
            line: 0,
            whole: part(vec![1], 75.0, 79.0),
            added: part(vec![1], 170.0, 179.0),
        });
        let rolls = TierRolls::new(Language::English, &[pure], Some("Warmonger Bow"), None);
        assert_eq!(rolls.get(key(0, 0)), None);
    }

    #[test]
    fn the_other_line_of_a_hybrid_of_one_tier_only_whose_part_a_line_adds_up_tells_no_tiers() {
        use crate::item::{Affix, ModifierKind, ModifierOrigin, PartTier, StatLine};

        let part = |min, max| PartTier {
            affix: Affix::Prefix,
            tiers: vec![1],
            hides_number: true,
            ranges: vec![range(min, max)],
        };
        // The hybrid of Kurgal (110–154, and 15% reduced Attack Speed), desecrated, and a pure
        // T1 (170–179) of a maul.
        let split = Modifier {
            kind: ModifierKind::Explicit(Some(Affix::Prefix)),
            origin: Some(ModifierOrigin::Desecrated),
            name: None,
            tier: None,
            tags: Vec::new(),
            unrevealed: false,
            lines: vec![
                StatLine::parse("300% increased Physical Damage"),
                StatLine::parse("15% reduced Attack Speed"),
            ],
            doubt: Some(TierDoubt::Split {
                line: 0,
                whole: part(110.0, 154.0),
                added: part(170.0, 179.0),
            }),
        };

        let rolls = TierRolls::new(
            Language::English,
            std::slice::from_ref(&split),
            Some("Temple Maul"),
            Some(82),
        );

        // Over its one tier, without "T1–T1"; the modifier not counted as one of one tier
        // only, the split line having no tier.
        let speed = rolls.get(key(0, 1)).expect("the rolls of the Attack Speed");
        assert_eq!(speed.ranges, [Some(range(15.0, 15.0))]);
        assert_eq!(speed.tiers, None);
        assert!(!rolls.single(0));
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
