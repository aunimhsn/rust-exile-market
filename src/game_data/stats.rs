//! The stats of the game, from Exiled Exchange 2 (`assets/game-data/stats-<language>.ndjson`):
//! the strings that match a modifier line in each language, and the trade ids of each stat.

use std::collections::{BTreeMap, HashMap};
use std::sync::{LazyLock, OnceLock};

use serde::Deserialize;

use crate::item::{Modifier, ModifierKind, ModifierOrigin};
use crate::language::Language;

/// A group of stats on the trade site: the same stat has an id in each group where it can
/// be, e.g. `explicit.stat_3299347043` and `fractured.stat_3299347043`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TradeGroup {
    Explicit,
    Implicit,
    Fractured,
    Desecrated,
    Crafted,
    Enchant,
    /// Added by a rune or a soul core: "Augment" on the trade site.
    Rune,
    Pseudo,
    Skill,
}

impl TradeGroup {
    /// The group where the lines of `modifier` are searched on the trade site. A desecrated or
    /// a fractured line is searched by its explicit id, like Exiled Exchange 2: it finds the
    /// offers where the line is desecrated, fractured or plain (checked on the trade site on
    /// 2026-10-28), and every desecrated and fractured stat of the game data has one.
    #[must_use]
    pub fn of(modifier: &Modifier) -> Self {
        match modifier.kind {
            ModifierKind::Implicit => Self::Implicit,
            ModifierKind::Explicit(_) => match modifier.origin {
                Some(ModifierOrigin::Crafted) => Self::Crafted,
                Some(ModifierOrigin::Fractured | ModifierOrigin::Desecrated) | None => {
                    Self::Explicit
                }
            },
            // The trade site lists the modifiers of the corruption with the enchantments.
            ModifierKind::Enchant | ModifierKind::Corruption => Self::Enchant,
            ModifierKind::Rune => Self::Rune,
        }
    }

    /// The group named `name` on the trade site (`explicit`, `rune`…), among the groups of
    /// the modifier lines: none for the pseudo stats, the skills, the Sanctum or a new group.
    #[must_use]
    pub fn of_line(name: &str) -> Option<Self> {
        match name {
            "explicit" => Some(Self::Explicit),
            "implicit" => Some(Self::Implicit),
            "fractured" => Some(Self::Fractured),
            "desecrated" => Some(Self::Desecrated),
            "crafted" => Some(Self::Crafted),
            "enchant" => Some(Self::Enchant),
            "rune" => Some(Self::Rune),
            _ => None,
        }
    }
}

/// Which side of the value of a stat is better for the player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(try_from = "i8")]
pub enum Better {
    Higher,
    Lower,
    /// Neither: e.g. the number of souls of a timeless jewel.
    Neither,
}

impl TryFrom<i8> for Better {
    type Error = String;

    fn try_from(value: i8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Higher),
            -1 => Ok(Self::Lower),
            0 => Ok(Self::Neither),
            other => Err(format!("unknown better side {other}")),
        }
    }
}

/// A stat of the game.
#[derive(Debug, PartialEq, Deserialize)]
pub struct Stat {
    /// Its English template, e.g. `# to maximum Life`, the same in every language.
    #[serde(rename = "ref")]
    pub reference: String,
    /// Its id in the files of the game, e.g. `base_maximum_life`; none for a stat of the trade
    /// site only, such as a pseudo stat.
    pub id: Option<String>,
    pub better: Better,
    pub(super) matchers: Vec<Matcher>,
    #[serde(default)]
    pub(super) ids: BTreeMap<TradeGroup, Vec<String>>,
    /// Its value on the trade site is an option, e.g. the skill of `# to Level of all Arc
    /// Skills`, given by [`Matcher::value`].
    #[serde(default)]
    pub option: bool,
    /// Its value on the trade site has the other sign, e.g. `#% less Damage`.
    #[serde(default)]
    pub inverted: bool,
}

impl Stat {
    /// The ids of the stat in `group`: none when it can't be there; two for a stat that is
    /// local on some items and global on others (`# to Accuracy Rating` of a weapon or of a
    /// ring), which the copied text doesn't tell apart.
    #[must_use]
    pub fn trade_ids(&self, group: TradeGroup) -> &[String] {
        self.ids.get(&group).map_or(&[], Vec::as_slice)
    }
}

/// A string of a language that matches a line of the stat, with `#` for its values, e.g.
/// `#% reduced Charm Charges used`.
#[derive(Debug, PartialEq, Deserialize)]
pub(super) struct Matcher {
    pub string: String,
    /// The value the string gives: the option of an option stat, or the value of a string
    /// without `#` (`Blind Enemies on Hit` is a chance of 100%).
    pub value: Option<f64>,
    /// The string writes the opposite of the stat: `reduced` for an `increased` stat.
    #[serde(default)]
    pub negate: bool,
}

/// The stats of a language, found by the strings of their lines.
pub(super) struct Stats {
    list: Vec<Stat>,
    /// The index of the stat and of its matcher, by string written as [`normalize`] does.
    by_string: HashMap<String, Vec<(usize, usize)>>,
    /// The indexes of the stats by their English template, built at the first
    /// [`Stats::twin`]: only the English stats are asked for twins.
    by_reference: OnceLock<HashMap<String, Vec<usize>>>,
    /// The most lines a string has.
    pub max_lines: usize,
}

static ENGLISH: LazyLock<Stats> =
    LazyLock::new(|| Stats::read(include_str!("../../assets/game-data/stats-en.ndjson")));
static FRENCH: LazyLock<Stats> =
    LazyLock::new(|| Stats::read(include_str!("../../assets/game-data/stats-fr.ndjson")));

impl Stats {
    pub fn of(language: Language) -> &'static Self {
        match language {
            Language::English => &ENGLISH,
            Language::French => &FRENCH,
        }
    }

    /// Reads one stat per line; a line that can't be read is logged and left out (the tests
    /// read every line of the embedded files).
    fn read(text: &str) -> Self {
        let mut stats = Vec::new();
        for (index, line) in text.lines().enumerate() {
            match serde_json::from_str::<Stat>(line) {
                Ok(stat) => stats.push(stat),
                Err(error) => log::error!("Game data, stat {} unreadable: {error}", index + 1),
            }
        }
        Self::new(stats)
    }

    /// The stats of `stats`, found by their strings.
    pub(super) fn new(stats: Vec<Stat>) -> Self {
        let mut by_string: HashMap<String, Vec<(usize, usize)>> = HashMap::new();
        let mut max_lines = 1;
        for (stat_index, stat) in stats.iter().enumerate() {
            for (matcher_index, matcher) in stat.matchers.iter().enumerate() {
                let string = normalize(&matcher.string);
                max_lines = max_lines.max(string.lines().count());
                let entries = by_string.entry(string).or_default();
                // The same string once normalized, e.g. with and without a no-break space.
                let same = |&(other_stat, other_matcher): &(usize, usize)| {
                    other_stat == stat_index
                        && stat
                            .matchers
                            .get(other_matcher)
                            .is_some_and(|other: &Matcher| {
                                other.negate == matcher.negate && other.value == matcher.value
                            })
                };
                if !entries.iter().any(same) {
                    entries.push((stat_index, matcher_index));
                }
            }
        }
        Self {
            list: stats,
            by_string,
            by_reference: OnceLock::new(),
            max_lines,
        }
    }

    /// The stats with a line written `string` (normalized), and the index of the matcher of
    /// that line among the matchers of its stat.
    pub fn find<'a>(&'a self, string: &str) -> impl Iterator<Item = (&'a Stat, usize)> + use<'a> {
        self.by_string
            .get(string)
            .into_iter()
            .flatten()
            .filter_map(|&(stat, matcher_index)| Some((self.list.get(stat)?, matcher_index)))
    }

    /// The matcher of these stats at the place of the matcher `matcher_index` of `stat`, a
    /// stat of another language: the stat with the same English template and game id, whose
    /// strings are in the same order in every language (checked on 2026-10-26: 2,616 of the
    /// 2,901 French strings with `#` of the equipment line up). None when two stats have
    /// this template and id, or when the matcher there doesn't negate like the other or
    /// doesn't give the same value: the strings don't line up.
    pub fn twin(&self, stat: &Stat, matcher_index: usize) -> Option<&Matcher> {
        let by_reference = self.by_reference.get_or_init(|| {
            let mut by_reference: HashMap<String, Vec<usize>> = HashMap::new();
            for (index, stat) in self.list.iter().enumerate() {
                by_reference
                    .entry(stat.reference.clone())
                    .or_default()
                    .push(index);
            }
            by_reference
        });
        let mut same = by_reference
            .get(&stat.reference)?
            .iter()
            .filter_map(|&index| self.list.get(index))
            .filter(|other| other.id == stat.id);
        let twin = same.next()?;
        if same.next().is_some() {
            return None;
        }
        let matcher = twin.matchers.get(matcher_index)?;
        let other = stat.matchers.get(matcher_index)?;
        (matcher.negate == other.negate && matcher.value == other.value).then_some(matcher)
    }

    pub fn len(&self) -> usize {
        self.list.len()
    }

    /// The stat of the game id `id`, when there is one.
    #[cfg(test)]
    pub fn by_id(&self, id: &str) -> Option<&Stat> {
        self.list.iter().find(|stat| stat.id.as_deref() == Some(id))
    }

    /// The strings with `#` of the stats that can be on equipment, and the index of each.
    #[cfg(test)]
    pub fn equipment_strings(&self) -> impl Iterator<Item = (&Stat, usize)> {
        const GROUPS: [TradeGroup; 7] = [
            TradeGroup::Explicit,
            TradeGroup::Implicit,
            TradeGroup::Fractured,
            TradeGroup::Desecrated,
            TradeGroup::Crafted,
            TradeGroup::Enchant,
            TradeGroup::Rune,
        ];
        self.list
            .iter()
            .filter(|stat| {
                GROUPS
                    .iter()
                    .any(|&group| !stat.trade_ids(group).is_empty())
            })
            .flat_map(|stat| {
                let strings = stat.matchers.iter().enumerate();
                let with_hash = strings.filter(|(_, matcher)| matcher.string.contains('#'));
                with_hash.map(move |(index, _)| (stat, index))
            })
    }
}

/// A line as the strings are looked up: the no-break spaces of French (`#\u{a0}%`) as
/// spaces, and each line without the spaces around it.
pub(super) fn normalize(text: &str) -> String {
    text.replace(['\u{a0}', '\u{202f}'], " ")
        .lines()
        .map(str::trim)
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_stat_of_the_embedded_files_is_read() {
        for (language, text) in [
            (
                Language::English,
                include_str!("../../assets/game-data/stats-en.ndjson"),
            ),
            (
                Language::French,
                include_str!("../../assets/game-data/stats-fr.ndjson"),
            ),
        ] {
            assert_eq!(
                Stats::of(language).len(),
                text.lines().count(),
                "{language:?}"
            );
        }
    }

    /// A desecrated or a fractured line is searched by its explicit id ([`TradeGroup::of`]):
    /// without one, it couldn't be searched at all.
    #[test]
    fn every_desecrated_or_fractured_stat_has_an_explicit_id() {
        for language in [Language::English, Language::French] {
            let missing: Vec<&str> = Stats::of(language)
                .list
                .iter()
                .filter(|stat| {
                    [TradeGroup::Desecrated, TradeGroup::Fractured]
                        .into_iter()
                        .any(|group| !stat.trade_ids(group).is_empty())
                })
                .filter(|stat| stat.trade_ids(TradeGroup::Explicit).is_empty())
                .map(|stat| stat.reference.as_str())
                .collect();
            assert!(missing.is_empty(), "{language:?}: {missing:#?}");
        }
    }

    #[test]
    fn a_stat_has_its_ids_in_each_group() {
        let (stat, matcher_index) = Stats::of(Language::English)
            .find("# to maximum Life")
            .next()
            .unwrap();
        let matcher = &stat.matchers[matcher_index];

        assert_eq!(stat.id.as_deref(), Some("base_maximum_life"));
        assert_eq!(stat.better, Better::Higher);
        assert_eq!(
            stat.trade_ids(TradeGroup::Explicit),
            ["explicit.stat_3299347043"]
        );
        assert_eq!(
            stat.trade_ids(TradeGroup::Fractured),
            ["fractured.stat_3299347043"]
        );
        assert_eq!(stat.trade_ids(TradeGroup::Skill), [] as [String; 0]);
        assert!(!matcher.negate);
    }

    #[test]
    fn a_stat_that_cant_be_read_is_left_out() {
        let stats = Stats::read(concat!(
            r##"{"ref":"# to maximum Life","better":1,"matchers":[{"string":"# to maximum Life"}]}"##,
            "\n",
            r##"{"ref":"# to Strength","better":2,"matchers":[]}"##,
            "\n",
            "not a stat\n",
        ));

        assert_eq!(stats.len(), 1);
        assert_eq!(stats.find("# to maximum Life").count(), 1);
        assert!(Better::try_from(2).is_err());
    }

    #[test]
    fn a_string_written_twice_once_normalized_is_found_once() {
        let stats = Stats::read(
            r##"{"ref":"# to all Attributes","better":1,"matchers":[{"string":"#\u00a0à tous les Attributs"},{"string":"# à tous les Attributs"}]}"##,
        );

        assert_eq!(stats.find("# à tous les Attributs").count(), 1);
    }

    #[test]
    fn the_twin_of_a_string_is_the_english_string_at_its_place() {
        let english = Stats::read(concat!(
            r##"{"ref":"#% increased Armour","id":"armour","better":1,"matchers":[{"string":"#% increased Armour"},{"string":"#% reduced Armour","negate":true}]}"##,
            "\n",
            r##"{"ref":"# to Strength","id":"strength","better":1,"matchers":[{"string":"# to Strength"}]}"##,
        ));
        let french = Stats::read(concat!(
            r##"{"ref":"#% increased Armour","id":"armour","better":1,"matchers":[{"string":"#% d'Augmentation de l'Armure"},{"string":"#% de Réduction de l'Armure","negate":true}]}"##,
            "\n",
            r##"{"ref":"# to Strength","id":"strength","better":1,"matchers":[{"string":"# à la Force","negate":true}]}"##,
            "\n",
            r##"{"ref":"# to Dexterity","id":"dexterity","better":1,"matchers":[{"string":"# à la Dextérité"}]}"##,
        ));
        let twin = |string| {
            let (stat, index) = french.find(string).next().unwrap();
            english
                .twin(stat, index)
                .map(|matcher| matcher.string.as_str())
        };

        assert_eq!(
            twin("#% de Réduction de l'Armure"),
            Some("#% reduced Armour")
        );
        assert_eq!(
            twin("#% d'Augmentation de l'Armure"),
            Some("#% increased Armour")
        );
        assert_eq!(twin("# à la Force"), None, "the strings don't line up");
        assert_eq!(twin("# à la Dextérité"), None, "no such stat in English");
    }

    #[test]
    fn two_stats_with_the_same_template_and_id_have_no_twin() {
        let english = Stats::read(concat!(
            r##"{"ref":"# to Strength","id":"strength","better":1,"matchers":[{"string":"# to Strength"}]}"##,
            "\n",
            r##"{"ref":"# to Strength","id":"strength","better":1,"matchers":[{"string":"# to Strength of the other"}]}"##,
        ));
        let french = Stats::read(
            r##"{"ref":"# to Strength","id":"strength","better":1,"matchers":[{"string":"# à la Force"}]}"##,
        );

        let (stat, index) = french.find("# à la Force").next().unwrap();

        assert!(english.twin(stat, index).is_none());
    }

    #[test]
    fn the_no_break_spaces_and_the_spaces_around_the_lines_are_normalized() {
        assert_eq!(
            normalize("#\u{a0}à tous les Attributs "),
            "# à tous les Attributs"
        );
        assert_eq!(
            normalize("Increases and Reductions to\n Fire Damage"),
            "Increases and Reductions to\nFire Damage"
        );
    }
}
