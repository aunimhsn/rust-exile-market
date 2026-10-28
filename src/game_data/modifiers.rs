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
    lines: Vec<TableLine>,
    /// Each tag and its weight: the first tag of the list a base has tells whether the
    /// modifier rolls on it (above zero) or not.
    spawn: Vec<(String, u32)>,
    tiers: Vec<TableTier>,
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum TableAffix {
    Prefix,
    Suffix,
}

#[derive(Deserialize, PartialEq)]
struct TableLine {
    /// The English template of the line, as Exiled Exchange 2 writes it.
    template: String,
    /// What the value of the stat is divided by to be shown, for a line of one value that can
    /// be written without number: its value is then that of the stat (1000 for `Inherent Rage
    /// loss starts 1 second later`, shown in seconds; no such line rolls on a base on
    /// 2026-10-28, the generator writes the divisor when one does).
    #[serde(default = "one")]
    divisor: f64,
}

fn one() -> f64 {
    1.0
}

#[derive(Deserialize)]
struct TableTier {
    level: u32,
    /// For each line, the range of each of its values, with the sign of the stat.
    ranges: Vec<Vec<[f64; 2]>>,
}

impl Table {
    /// Whether the modifier rolls on a base with `tags`.
    fn rolls_on(&self, tags: &[String]) -> bool {
        self.spawn
            .iter()
            .find(|(tag, _)| tags.contains(tag))
            .is_some_and(|&(_, weight)| weight > 0)
    }

    /// Whether the table holds tiers of the same modifier as `other`, maybe on other bases.
    fn same_modifier(&self, other: &Self) -> bool {
        self.kind == other.kind && self.affix == other.affix && self.lines == other.lines
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
    /// The number of values of each line match given, in their order: the ranges of a tier
    /// follow them (two for `Adds # to # Physical Damage`; for a line without number, one
    /// when its string gives the value, `Loads an additional bolt`, none otherwise).
    pub counts: Vec<usize>,
}

impl ModifierTiers {
    /// The tier of the copied modifier.
    #[must_use]
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "checked by the tests: the application shows the tier the advanced copy writes"
        )
    )]
    pub fn tier(&self) -> Option<&Tier> {
        self.tiers.get(self.found)
    }
}

/// Every tier of `modifier`, a prefix or suffix of an item whose base type is `base_type` in
/// `language` (the name its search looks for), its lines matched with their stats in
/// `matches` ([`super::modifier_stats`]).
///
/// None for a modifier the tables don't hold: an implicit, a rune, an enchantment, a modifier
/// whose header doesn't tell its affix, a line without stat, a base without tags; or whose
/// ranges are in no tier: a modifier of the lords of the Abyss is desecrated, like some
/// ordinary ones, and rolls its own range.
///
/// With the ranges of the advanced copy, only a tier with those ranges: the tier that holds the
/// copied values first, then the one that holds them give or take the rounding of the game, then
/// the one whose values left it (any value out of its tier, e.g. on a sanctified item). Without
/// them (a line without number has no range), only a tier that holds the values. When several
/// tiers fit as well, the best one is taken; when several modifiers do, the first of the file.
#[must_use]
pub fn modifier_tiers(
    language: Language,
    base_type: &str,
    modifier: &Modifier,
    matches: &[LineMatch<'_>],
) -> Option<ModifierTiers> {
    let affix = match modifier.kind {
        ModifierKind::Explicit(Some(Affix::Prefix)) => TableAffix::Prefix,
        ModifierKind::Explicit(Some(Affix::Suffix)) => TableAffix::Suffix,
        _ => return None,
    };
    let tags = bases::tags(language, base_type)?;
    let copied = Copied::of(modifier, matches)?;
    find(&TABLES, affix, tags, &copied)
}

/// The tiers of `copied` among `tables`, on a base with `tags`: those of the modifier of
/// `affix` that holds it best.
fn find(
    tables: &[Table],
    affix: TableAffix,
    tags: &[String],
    copied: &Copied<'_>,
) -> Option<ModifierTiers> {
    let mut best: Option<(Fit, ModifierTiers)> = None;
    for (index, table) in tables.iter().enumerate() {
        if table.affix != affix {
            continue;
        }
        let Some(order) = copied.order(&table.lines) else {
            continue;
        };
        // Each modifier once, at its first table.
        let first = tables
            .get(..index)
            .is_none_or(|before| !before.iter().any(|other| other.same_modifier(table)));
        if !first {
            continue;
        }
        let Some(tiers) = tiers_on_base(tables, table, tags, &order, copied) else {
            continue;
        };
        let values = copied.values(&table.lines, &order);
        // The best tier first, on a tie.
        let found = tiers
            .iter()
            .enumerate()
            .filter_map(|(index, tier)| Some((copied.fit(&values, tier)?, index)))
            .min_by_key(|&(fit, _)| fit);
        if let Some((fit, found)) = found
            && best.as_ref().is_none_or(|(best, _)| fit < *best)
        {
            let counts = copied.lines.iter().map(CopiedLine::count).collect();
            best = Some((
                fit,
                ModifierTiers {
                    tiers,
                    found,
                    counts,
                },
            ));
        }
    }
    best.map(|(_, tiers)| tiers)
}

/// Every tier of the modifier of `table` that rolls on a base with `tags`, the best first, the
/// ranges of each in the order of the copied lines (`order`, [`Copied::order`]): a type can
/// roll other ranges on other bases (the added damage of a two-handed weapon). None when a
/// line of a tier hasn't as many values as the copied one.
fn tiers_on_base(
    tables: &[Table],
    table: &Table,
    tags: &[String],
    order: &[usize],
    copied: &Copied<'_>,
) -> Option<Vec<Tier>> {
    let mut tiers: Vec<&TableTier> = tables
        .iter()
        .filter(|other| other.same_modifier(table) && other.rolls_on(tags))
        .flat_map(|other| &other.tiers)
        .collect();
    tiers.sort_by_key(|tier| tier.level);
    tiers
        .iter()
        .rev()
        .zip(1..)
        .map(|(tier, number)| {
            Some(Tier {
                number,
                level: tier.level,
                ranges: ranges_in_order(tier, order, copied)?,
            })
        })
        .collect()
}

/// The ranges of `tier` in the order of the copied lines.
fn ranges_in_order(tier: &TableTier, order: &[usize], copied: &Copied<'_>) -> Option<Vec<Range>> {
    let mut ranges = Vec::new();
    for (line, &index) in copied.lines.iter().zip(order) {
        let line_ranges = tier.ranges.get(index)?;
        if line_ranges.len() != line.count() {
            return None;
        }
        ranges.extend(line_ranges.iter().map(|&[min, max]| Range {
            min: min.min(max),
            max: min.max(max),
        }));
    }
    Some(ranges)
}

/// How well a tier holds the copied modifier, the best first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Fit {
    /// Its values are within the ranges of the tier.
    Within,
    /// Its values are within the ranges of the tier, give or take the rounding of the game.
    Rounded,
    /// Its values are out of the ranges of the tier, which are those the advanced copy wrote:
    /// any value that left its tier, e.g. on a sanctified item (`+74(80-89) to maximum Mana`).
    Written,
}

/// A copied line of a modifier, the lines of a stat written on several lines counting as one.
struct CopiedLine<'a> {
    /// The English template of its stat.
    template: &'a str,
    /// The values of its `#`, with the sign of the stat.
    values: Vec<f64>,
    /// The value its string gives, for a line without `#` (`Loads an additional bolt` is 1),
    /// as the stat stores it.
    written: Option<f64>,
}

impl CopiedLine<'_> {
    /// How many values the line has.
    fn count(&self) -> usize {
        if self.values.is_empty() {
            usize::from(self.written.is_some())
        } else {
            self.values.len()
        }
    }
}

/// The lines of a copied modifier, and the ranges of its tier when the advanced copy gave them.
struct Copied<'a> {
    lines: Vec<CopiedLine<'a>>,
    /// The ranges the advanced copy gave for each value of the lines, in their order, as the
    /// game writes them: without their sign.
    ranges: Option<Vec<Range>>,
}

impl<'a> Copied<'a> {
    fn of(modifier: &Modifier, matches: &[LineMatch<'a>]) -> Option<Self> {
        let mut lines = Vec::new();
        let mut ranges = Some(Vec::new());
        for line_match in matches {
            let stat = line_match.stat.as_ref()?;
            // A line the trade site only knows has no template of Exiled Exchange 2.
            if stat.source != StatSource::GameData {
                return None;
            }
            lines.push(CopiedLine {
                template: stat.stat.reference.as_str(),
                values: stat.values.clone(),
                written: stat.matcher_value.filter(|_| stat.values.is_empty()),
            });
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
        // The advanced copy writes no range for a line without number (`Loads an additional
        // bolt`): the modifier is then found by its values only, as it is when a range is
        // missing.
        let count: usize = lines.iter().map(CopiedLine::count).sum();
        let ranges = ranges.filter(|ranges| {
            lines.iter().all(|line| line.written.is_none()) && ranges.len() == count
        });
        Some(Self { lines, ranges })
    }

    /// For each copied line, the index of the line of the same template in `lines`, the lines
    /// of a table: its lines in another order are the same modifier. None when the lines
    /// differ.
    fn order(&self, lines: &[TableLine]) -> Option<Vec<usize>> {
        if lines.len() != self.lines.len() {
            return None;
        }
        let mut used = vec![false; lines.len()];
        self.lines
            .iter()
            .map(|copied| {
                let index = lines
                    .iter()
                    .zip(&used)
                    .position(|(line, &used)| !used && line.template == copied.template)?;
                *used.get_mut(index)? = true;
                Some(index)
            })
            .collect()
    }

    /// The values of the copied lines, in their order, as the table of `lines` shows them: the
    /// value a line without number gives, divided as its table line says.
    fn values(&self, lines: &[TableLine], order: &[usize]) -> Vec<f64> {
        let mut values = Vec::new();
        for (copied, &index) in self.lines.iter().zip(order) {
            values.extend(&copied.values);
            if let Some(written) = copied.written {
                let divisor = lines.get(index).map_or(1.0, |line| line.divisor);
                values.push(written / divisor);
            }
        }
        values
    }

    /// How well `tier` holds the copied `values`; none when its ranges aren't those the advanced
    /// copy gave, or when it doesn't hold them and the copy gave no range.
    fn fit(&self, values: &[f64], tier: &Tier) -> Option<Fit> {
        if values.len() != tier.ranges.len() {
            return None;
        }
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
            values
                .iter()
                .zip(&tier.ranges)
                .all(|(&value, range)| range.min - margin <= value && value <= range.max + margin)
        };
        if within(0.0) {
            Some(Fit::Within)
        } else if within(ROUNDING) {
            Some(Fit::Rounded)
        } else {
            self.ranges.as_ref().map(|_| Fit::Written)
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
    use crate::item::{CopiedItem, Item, ModifierOrigin, StatLine};

    #[test]
    fn every_table_of_the_embedded_file_is_read() {
        let text = include_str!("../../assets/game-data/modifiers.ndjson");

        assert_eq!(TABLES.len(), text.lines().count());
    }

    fn range(min: f64, max: f64) -> Range {
        Range { min, max }
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
            (
                include_str!("../item/test-items/en-owner-rare-sceptre-sanctified.txt"),
                None,
            ),
            (
                include_str!("../item/test-items/fr-rare-sceptre-sanctified.txt"),
                None,
            ),
        ] {
            for (tier, (number, ranges)) in tiers_of(text, base_type) {
                // The crossbow was copied before a patch added a tier above its leech, `of
                // the Desperate` (8–8.9), the S1 of the offers checked on the trade site on
                // 2026-10-27.
                let number = if ranges == [range(7.0, 7.9)] {
                    2
                } else {
                    number
                };
                if let Some(tier) = tier {
                    assert_eq!(tier, (number, ranges));
                    found += 1;
                } else {
                    missing.push(ranges);
                }
            }
        }
        assert_eq!(found, 31);
        // A line whose stat the tables have under another tier is a mistake; a line the tables
        // don't have isn't: the lightning damage of the mace, copied before a patch changed
        // its ranges, and the two modifiers of Amanamu, a lord of the Abyss, desecrated with
        // ranges of their own.
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
    fn a_sanctified_value_out_of_its_tier_finds_it_by_the_copied_ranges() {
        for text in [
            include_str!("../item/test-items/en-owner-rare-sceptre-sanctified.txt"),
            include_str!("../item/test-items/fr-rare-sceptre-sanctified.txt"),
        ] {
            let found: Vec<_> = tiers_of(text, None)
                .into_iter()
                .filter_map(|(found, _)| found)
                .collect();
            // `+74(80-89) to maximum Mana` and `29(30)% reduced Attribute Requirements`.
            assert!(found.contains(&(5, vec![range(80.0, 89.0)])), "{found:?}");
            assert!(found.contains(&(2, vec![range(30.0, 30.0)])), "{found:?}");
        }

        // A line of two values, one of them out of its tier: `4(1-4) to 75(60-71)`.
        let text = include_str!("../item/test-items/en-owner-rare-sceptre-sanctified.txt")
            .replace("60(60-71)", "75(60-71)");
        let (found, written) = tiers_of(&text, None).swap_remove(0);
        assert_eq!(written, (1, vec![range(1.0, 4.0), range(60.0, 71.0)]));
        assert_eq!(found, Some(written));

        // Without the ranges of the advanced copy, a value out of every tier finds none.
        let mana = modifier(Some(Affix::Prefix), &["+74 to maximum Mana"]);
        assert_eq!(
            tiers("Rattling Sceptre", &mana).and_then(|tiers| Some(tiers.tier()?.number)),
            Some(6),
            "74 is in the range of tier 6"
        );
        let mana = modifier(Some(Affix::Prefix), &["+400 to maximum Mana"]);
        assert_eq!(tiers("Rattling Sceptre", &mana), None);
    }

    /// A modifier of `affix` with `lines`, without the ranges of the advanced copy: found by
    /// the values the stats read in the text.
    fn modifier(affix: Option<Affix>, lines: &[&str]) -> Modifier {
        Modifier {
            kind: ModifierKind::Explicit(affix),
            origin: None,
            name: None,
            tier: None,
            tags: Vec::new(),
            unrevealed: false,
            lines: lines
                .iter()
                .map(|text| StatLine {
                    text: (*text).to_owned(),
                    rolls: Vec::new(),
                })
                .collect(),
        }
    }

    /// The tiers of `modifier` on `base_type`, in English.
    fn tiers(base_type: &str, modifier: &Modifier) -> Option<ModifierTiers> {
        let matches = modifier_stats(Language::English, modifier, None);
        modifier_tiers(Language::English, base_type, modifier, &matches)
    }

    #[test]
    fn a_tier_is_numbered_among_the_tiers_of_the_base() {
        let lightning = modifier(Some(Affix::Prefix), &["Adds 10 to 273 Lightning Damage"]);

        let tiers = tiers("Gemini Crossbow", &lightning).expect("its tiers");

        // The tiers of a two-handed weapon, not those of a one-handed one.
        assert_eq!(tiers.tiers.len(), 10);
        assert_eq!(
            tiers.tiers.first().map(|tier| tier.ranges.clone()),
            Some(vec![range(1.0, 19.0), range(310.0, 358.0)])
        );
        assert_eq!(tiers.tiers.last().map(|tier| tier.level), Some(1));
        assert_eq!(tiers.tier().map(|tier| tier.number), Some(2));
    }

    #[test]
    fn a_line_written_without_number_finds_its_tier_by_the_value_of_its_string() {
        let number_of = |line: &str| {
            let bolts = modifier(Some(Affix::Suffix), &[line]);
            tiers("Gemini Crossbow", &bolts).and_then(|tiers| Some(tiers.tier()?.number))
        };

        assert_eq!(number_of("Loads an additional bolt"), Some(2));
        assert_eq!(number_of("Loads 2 additional bolts"), Some(1));
        assert_eq!(number_of("Loads 3 additional bolts"), None);
    }

    #[test]
    fn the_lines_of_a_hybrid_in_another_order_are_the_same_modifier() {
        let champion = modifier(
            Some(Affix::Prefix),
            &["+100 to Accuracy Rating", "50% increased Physical Damage"],
        );

        let tiers = tiers("Gemini Crossbow", &champion).expect("its tiers");

        let tier = tiers.tier().expect("its tier");
        assert_eq!(tier.number, 4);
        // In the order of the copied lines.
        assert_eq!(tier.ranges, [range(98.0, 123.0), range(45.0, 54.0)]);
    }

    #[test]
    fn a_modifier_outside_the_prefixes_and_suffixes_has_no_tiers() {
        let mut implicit = modifier(None, &["Loads an additional bolt"]);
        implicit.kind = ModifierKind::Implicit;
        // A header the parser doesn't know tells no affix.
        let unknown = modifier(None, &["Adds 10 to 273 Lightning Damage"]);
        let mut desecrated = modifier(Some(Affix::Prefix), &["Adds 10 to 273 Lightning Damage"]);
        desecrated.origin = Some(ModifierOrigin::Desecrated);

        assert_eq!(tiers("Gemini Crossbow", &implicit), None);
        assert_eq!(tiers("Gemini Crossbow", &unknown), None);
        // A base the data doesn't know.
        assert_eq!(tiers("Unknown Crossbow", &desecrated), None);
        // An ordinary modifier desecrated is in the tables.
        assert!(tiers("Gemini Crossbow", &desecrated).is_some());
    }

    #[test]
    fn the_value_of_a_line_without_number_is_divided_as_its_table_shows_it() {
        let tables = read(concat!(
            r#"{"type": "Rage", "affix": "suffix", "lines": [{"template": "Inherent Rage loss starts 1 second later", "divisor": 1000}], "#,
            r#""spawn": [["weapon", 1]], "tiers": [{"level": 1, "ranges": [[[1, 1]]]}, {"level": 50, "ranges": [[[2, 3]]]}]}"#
        ));
        let copied = |values: Vec<f64>, written| Copied {
            lines: vec![CopiedLine {
                template: "Inherent Rage loss starts 1 second later",
                values,
                written,
            }],
            ranges: None,
        };
        let number = |copied| {
            find(&tables, TableAffix::Suffix, &["weapon".to_owned()], &copied)
                .and_then(|tiers| Some(tiers.tier()?.number))
        };

        // `Inherent Rage loss starts 1 second later` gives the 1000 milliseconds of the stat.
        assert_eq!(number(copied(Vec::new(), Some(1000.0))), Some(2));
        assert_eq!(number(copied(vec![2.5], None)), Some(1));
    }

    #[test]
    fn a_line_without_value_is_found_with_the_other_lines_of_its_modifier() {
        let instant = modifier(
            Some(Affix::Prefix),
            &["Instant Recovery", "50% reduced Amount Recovered"],
        );

        let tiers = tiers("Ultimate Life Flask", &instant).expect("its tiers");

        assert_eq!(tiers.tier().map(|tier| tier.number), Some(1));
        assert_eq!(
            tiers.tier().map(|tier| tier.ranges.clone()),
            Some(vec![range(-50.0, -50.0)])
        );
    }

    #[test]
    fn the_modifiers_of_the_time_lost_jewels_arent_in_the_tables_yet() {
        // Written within "Small Passive Skills in Radius also grant…" by the game, step 13k.
        let ailments = modifier(
            Some(Affix::Suffix),
            &["5% increased chance to inflict Ailments"],
        );

        assert_eq!(tiers("Time-Lost Emerald", &ailments), None);
    }

    #[test]
    fn two_tables_with_the_same_lines_in_another_order_are_two_modifiers() {
        let table = |kind: &str, lines: [&str; 2], level: u32| {
            format!(
                r#"{{"type": "{kind}", "affix": "prefix", "lines": [{{"template": "{}"}}, {{"template": "{}"}}], "spawn": [["weapon", 1]], "tiers": [{{"level": {level}, "ranges": [[[1, 5]], [[10, 20]]]}}]}}"#,
                lines[0], lines[1]
            )
        };
        let tables = read(
            &[
                table("Hybrid", ["A #", "B #"], 1),
                table("Hybrid", ["B #", "A #"], 50),
            ]
            .join("\n"),
        );
        let copied = Copied {
            lines: vec![
                CopiedLine {
                    template: "B #",
                    values: vec![15.0],
                    written: None,
                },
                CopiedLine {
                    template: "A #",
                    values: vec![3.0],
                    written: None,
                },
            ],
            ranges: None,
        };

        let tiers =
            find(&tables, TableAffix::Prefix, &["weapon".to_owned()], &copied).expect("its tiers");

        // The first table holds the copied values, its lines put in the copied order; the
        // second, its own lines in the copied order, holds neither 15 in 1–5 nor 3 in 10–20.
        assert_eq!(tiers.tiers.len(), 1);
        assert_eq!(tiers.tiers.first().map(|tier| tier.level), Some(1));
        assert_eq!(
            tiers.tier().map(|tier| tier.ranges.clone()),
            Some(vec![range(10.0, 20.0), range(1.0, 5.0)])
        );
    }

    #[test]
    fn a_line_with_a_divisor_has_one_value() {
        // The divisor is for the value of a line written without number, a single value.
        for table in TABLES.iter() {
            for (index, line) in table.lines.iter().enumerate() {
                if (line.divisor - 1.0).abs() > f64::EPSILON {
                    for tier in &table.tiers {
                        assert_eq!(
                            tier.ranges.get(index).map(Vec::len),
                            Some(1),
                            "{}",
                            line.template
                        );
                    }
                }
            }
        }
    }
}
