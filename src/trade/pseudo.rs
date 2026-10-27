//! The pseudo totals of the trade site: a stat it adds up from the modifier lines of an item
//! of every group (implicit, explicit, rune, enchantment…), such as the total elemental
//! resistance. A rare or magic item is searched by its totals in place of the lines they add
//! up, so that the offers found have the same total from other modifiers, like Exiled
//! Exchange 2.
//!
//! The rules are those of Exiled Exchange 2, checked on the trade site on 2026-10-27: its
//! total maximum life counts each point of strength twice, its total maximum mana each point
//! of intelligence, also from `+# to all Attributes`.

use crate::filters::StatKey;

/// A pseudo total.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pseudo {
    /// Fire, cold and lightning, each element of a line counted.
    ElementalResistance,
    ChaosResistance,
    Life,
    Mana,
    Strength,
    Dexterity,
    Intelligence,
    /// The `+# to all Attributes` lines alone.
    AllAttributes,
}

impl Pseudo {
    /// Its id on the trade site.
    #[must_use]
    pub fn trade_id(self) -> &'static str {
        match self {
            Self::ElementalResistance => "pseudo.pseudo_total_elemental_resistance",
            Self::ChaosResistance => "pseudo.pseudo_total_chaos_resistance",
            Self::Life => "pseudo.pseudo_total_life",
            Self::Mana => "pseudo.pseudo_total_mana",
            Self::Strength => "pseudo.pseudo_total_strength",
            Self::Dexterity => "pseudo.pseudo_total_dexterity",
            Self::Intelligence => "pseudo.pseudo_total_intelligence",
            Self::AllAttributes => "pseudo.pseudo_total_all_attributes",
        }
    }
}

// The game stats the totals add up.
const FIRE: &str = "base_fire_damage_resistance_%";
const COLD: &str = "base_cold_damage_resistance_%";
const LIGHTNING: &str = "base_lightning_damage_resistance_%";
const FIRE_AND_COLD: &str = "fire_and_cold_damage_resistance_%";
const FIRE_AND_LIGHTNING: &str = "fire_and_lightning_damage_resistance_%";
const COLD_AND_LIGHTNING: &str = "cold_and_lightning_damage_resistance_%";
const ALL_ELEMENTS: &str = "base_resist_all_elements_%";
const CHAOS: &str = "base_chaos_damage_resistance_%";
const FIRE_AND_CHAOS: &str = "fire_and_chaos_damage_resistance_%";
const COLD_AND_CHAOS: &str = "cold_and_chaos_damage_resistance_%";
const LIGHTNING_AND_CHAOS: &str = "lightning_and_chaos_damage_resistance_%";
const LIFE: &str = "base_maximum_life";
const MANA: &str = "base_maximum_mana";
const STRENGTH: &str = "additional_strength";
const DEXTERITY: &str = "additional_dexterity";
const INTELLIGENCE: &str = "additional_intelligence";
const STRENGTH_AND_DEXTERITY: &str = "additional_strength_and_dexterity";
const STRENGTH_AND_INTELLIGENCE: &str = "additional_strength_and_intelligence";
const DEXTERITY_AND_INTELLIGENCE: &str = "additional_dexterity_and_intelligence";
const ALL_ATTRIBUTES: &str = "additional_all_attributes";

/// The lines of an attribute of its own, single or with another one.
const OWN_ATTRIBUTES: [&str; 6] = [
    STRENGTH,
    DEXTERITY,
    INTELLIGENCE,
    STRENGTH_AND_DEXTERITY,
    STRENGTH_AND_INTELLIGENCE,
    DEXTERITY_AND_INTELLIGENCE,
];

/// How a pseudo total adds up the lines of an item, and when it is a filter.
struct Rule {
    pseudo: Pseudo,
    /// The game stats it adds up, each times its multiplier: `+#% to Fire and Cold
    /// Resistances` counts twice in the total elemental resistance.
    stats: &'static [(&'static str, f64)],
    /// A filter only when the item has a line of one of these stats; of any of `stats` when
    /// empty.
    required: &'static [&'static str],
    /// Not a filter when the item has a line of one of these stats.
    excluded: &'static [&'static str],
}

/// The rules, in the order of the filters.
const RULES: [Rule; 8] = [
    Rule {
        pseudo: Pseudo::ElementalResistance,
        stats: &[
            (FIRE, 1.0),
            (COLD, 1.0),
            (LIGHTNING, 1.0),
            (FIRE_AND_COLD, 2.0),
            (FIRE_AND_LIGHTNING, 2.0),
            (COLD_AND_LIGHTNING, 2.0),
            (ALL_ELEMENTS, 3.0),
            (FIRE_AND_CHAOS, 1.0),
            (COLD_AND_CHAOS, 1.0),
            (LIGHTNING_AND_CHAOS, 1.0),
        ],
        required: &[],
        excluded: &[],
    },
    Rule {
        pseudo: Pseudo::ChaosResistance,
        stats: &[
            (CHAOS, 1.0),
            (FIRE_AND_CHAOS, 1.0),
            (COLD_AND_CHAOS, 1.0),
            (LIGHTNING_AND_CHAOS, 1.0),
        ],
        required: &[],
        excluded: &[],
    },
    // The attributes count without a line of life or mana, but don't make the total a filter.
    Rule {
        pseudo: Pseudo::Life,
        stats: &[
            (LIFE, 1.0),
            (STRENGTH, 2.0),
            (STRENGTH_AND_DEXTERITY, 2.0),
            (STRENGTH_AND_INTELLIGENCE, 2.0),
            (ALL_ATTRIBUTES, 2.0),
        ],
        required: &[LIFE],
        excluded: &[],
    },
    Rule {
        pseudo: Pseudo::Mana,
        stats: &[
            (MANA, 1.0),
            (INTELLIGENCE, 2.0),
            (STRENGTH_AND_INTELLIGENCE, 2.0),
            (DEXTERITY_AND_INTELLIGENCE, 2.0),
            (ALL_ATTRIBUTES, 2.0),
        ],
        required: &[MANA],
        excluded: &[],
    },
    // An attribute is a filter with a line of its own: `+# to all Attributes` alone doesn't
    // make three filters, but one of its own.
    Rule {
        pseudo: Pseudo::Strength,
        stats: &[
            (STRENGTH, 1.0),
            (STRENGTH_AND_DEXTERITY, 1.0),
            (STRENGTH_AND_INTELLIGENCE, 1.0),
            (ALL_ATTRIBUTES, 1.0),
        ],
        required: &[STRENGTH, STRENGTH_AND_DEXTERITY, STRENGTH_AND_INTELLIGENCE],
        excluded: &[],
    },
    Rule {
        pseudo: Pseudo::Dexterity,
        stats: &[
            (DEXTERITY, 1.0),
            (STRENGTH_AND_DEXTERITY, 1.0),
            (DEXTERITY_AND_INTELLIGENCE, 1.0),
            (ALL_ATTRIBUTES, 1.0),
        ],
        required: &[
            DEXTERITY,
            STRENGTH_AND_DEXTERITY,
            DEXTERITY_AND_INTELLIGENCE,
        ],
        excluded: &[],
    },
    Rule {
        pseudo: Pseudo::Intelligence,
        stats: &[
            (INTELLIGENCE, 1.0),
            (STRENGTH_AND_INTELLIGENCE, 1.0),
            (DEXTERITY_AND_INTELLIGENCE, 1.0),
            (ALL_ATTRIBUTES, 1.0),
        ],
        required: &[
            INTELLIGENCE,
            STRENGTH_AND_INTELLIGENCE,
            DEXTERITY_AND_INTELLIGENCE,
        ],
        excluded: &[],
    },
    Rule {
        pseudo: Pseudo::AllAttributes,
        stats: &[(ALL_ATTRIBUTES, 1.0)],
        required: &[],
        excluded: &OWN_ATTRIBUTES,
    },
];

/// A pseudo total of the item, a filter.
#[derive(Debug, Clone, PartialEq)]
pub struct PseudoTotal {
    pub pseudo: Pseudo,
    pub value: f64,
    /// The lines it adds up.
    pub lines: Vec<StatKey>,
}

/// A modifier line the trade site searches: its key, the game id of its stat and its value
/// there, the value of the game for a stat that isn't inverted, as those of the rules.
#[derive(Debug, Clone, Copy)]
pub struct CountedLine<'a> {
    pub key: StatKey,
    pub game_id: &'a str,
    pub value: f64,
}

/// The pseudo totals of the item of `lines` that are filters, in the order of the rules:
/// those of its rules whose lines it has, above zero.
#[must_use]
pub fn totals(lines: &[CountedLine<'_>]) -> Vec<PseudoTotal> {
    let has = |stats: &[&str]| lines.iter().any(|line| stats.contains(&line.game_id));
    RULES
        .iter()
        .filter_map(|rule| {
            let added: Vec<(StatKey, f64)> = lines
                .iter()
                .filter_map(|line| {
                    let &(_, multiplier) =
                        rule.stats.iter().find(|(stat, _)| *stat == line.game_id)?;
                    Some((line.key, line.value * multiplier))
                })
                .collect();
            let value: f64 = added.iter().map(|(_, value)| value).sum();
            let required = rule.required.is_empty() || has(rule.required);
            (required && !has(rule.excluded) && value > 0.0).then(|| PseudoTotal {
                pseudo: rule.pseudo,
                value,
                lines: added.into_iter().map(|(key, _)| key).collect(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_data;
    use crate::language::Language;

    /// The lines of `stats`, one per modifier.
    fn lines<'a>(stats: &[(&'a str, f64)]) -> Vec<CountedLine<'a>> {
        stats
            .iter()
            .enumerate()
            .map(|(modifier, &(game_id, value))| CountedLine {
                key: StatKey { modifier, line: 0 },
                game_id,
                value,
            })
            .collect()
    }

    /// The totals of the item of `stats`, with their values.
    fn values(stats: &[(&str, f64)]) -> Vec<(Pseudo, f64)> {
        totals(&lines(stats))
            .into_iter()
            .map(|total| (total.pseudo, total.value))
            .collect()
    }

    #[test]
    fn the_elemental_resistances_count_each_element_of_a_line() {
        assert_eq!(
            values(&[
                (FIRE, 38.0),
                (COLD, 22.0),
                (FIRE_AND_LIGHTNING, 10.0),
                (ALL_ELEMENTS, 5.0),
            ]),
            [(Pseudo::ElementalResistance, 95.0)]
        );
    }

    #[test]
    fn a_resistance_to_an_element_and_chaos_counts_in_both_totals() {
        assert_eq!(
            values(&[(COLD_AND_CHAOS, 12.0), (CHAOS, 9.0)]),
            [
                (Pseudo::ElementalResistance, 12.0),
                (Pseudo::ChaosResistance, 21.0),
            ]
        );
    }

    #[test]
    fn the_life_and_the_mana_count_their_attribute_twice_with_a_line_of_their_own() {
        assert_eq!(
            values(&[(LIFE, 65.0), (STRENGTH, 12.0), (ALL_ATTRIBUTES, 5.0)]),
            [(Pseudo::Life, 99.0), (Pseudo::Strength, 17.0)]
        );
        assert_eq!(
            values(&[(MANA, 40.0), (STRENGTH_AND_INTELLIGENCE, 10.0)]),
            [
                (Pseudo::Mana, 60.0),
                (Pseudo::Strength, 10.0),
                (Pseudo::Intelligence, 10.0),
            ]
        );
        // Without a line of life, the strength isn't a total of life.
        assert_eq!(values(&[(STRENGTH, 16.0)]), [(Pseudo::Strength, 16.0)]);
    }

    #[test]
    fn all_attributes_alone_are_one_total() {
        assert_eq!(
            values(&[(ALL_ATTRIBUTES, 10.0)]),
            [(Pseudo::AllAttributes, 10.0)]
        );
        // With an attribute of its own, they count in it only.
        assert_eq!(
            values(&[(ALL_ATTRIBUTES, 10.0), (DEXTERITY, 20.0)]),
            [(Pseudo::Dexterity, 30.0)]
        );
    }

    #[test]
    fn a_total_of_zero_or_below_isnt_a_filter() {
        assert_eq!(values(&[(CHAOS, -10.0)]), []);
        assert_eq!(
            values(&[(CHAOS, -10.0), (FIRE_AND_CHAOS, 15.0)]),
            [
                (Pseudo::ElementalResistance, 15.0),
                (Pseudo::ChaosResistance, 5.0),
            ]
        );
        assert_eq!(values(&[(LIFE, 0.0)]), []);
    }

    #[test]
    fn a_total_knows_the_lines_it_adds_up() {
        let lines = lines(&[(FIRE, 38.0), (LIFE, 65.0), (COLD, 22.0), (STRENGTH, 12.0)]);
        let added: Vec<(Pseudo, Vec<usize>)> = totals(&lines)
            .into_iter()
            .map(|total| {
                let modifiers = total.lines.iter().map(|key| key.modifier).collect();
                (total.pseudo, modifiers)
            })
            .collect();

        assert_eq!(
            added,
            [
                (Pseudo::ElementalResistance, vec![0, 2]),
                (Pseudo::Life, vec![1, 3]),
                (Pseudo::Strength, vec![3]),
            ]
        );
    }

    /// The totals add up the values of the trade site, which are those of the game: no stat of
    /// the rules has the other sign there.
    #[test]
    fn the_stats_of_the_rules_are_stats_of_the_game_not_inverted() {
        let mut stats: Vec<&str> = RULES
            .iter()
            .flat_map(|rule| rule.stats.iter().map(|&(stat, _)| stat))
            .collect();
        stats.sort_unstable();
        stats.dedup();
        for language in [Language::English, Language::French] {
            for stat in &stats {
                assert_eq!(
                    game_data::stat_inverted(language, stat),
                    Some(false),
                    "{stat} in the {language:?} game data"
                );
            }
        }
    }
}
