//! The stats of the trade site (`/api/trade2/data/stats`), the second lookup of a line: a line
//! the stats of Exiled Exchange 2 don't know yet (a modifier added by a patch of the game)
//! finds its trade id by its template on the trade site of its language.
//!
//! A template writes one side of its stat only (`#% reduced Charm Charges used`, without the
//! `increased` one), and some end with a note the copied text never writes (`# to Accuracy
//! Rating (Local)`). A template is looked up without its note, and with it in case a line
//! writes it; the stats of the same template are one stat with all their ids, like a local and
//! a global stat in Exiled Exchange 2.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;

use super::stats::{Better, Matcher, Stat, Stats, TradeGroup, normalize};
use crate::item_names::StatTemplate;
use crate::language::{Language, PerLanguage};

/// The stats of the trade site in English and in French.
pub struct TradeStats {
    stats: PerLanguage<Stats>,
}

impl TradeStats {
    /// The stats of the templates of each language.
    #[must_use]
    pub fn new(english: &[StatTemplate], french: &[StatTemplate]) -> Self {
        let english_strings: HashMap<&str, String> = english
            .iter()
            .map(|template| (template.id.as_str(), string(&template.text)))
            .collect();
        Self {
            stats: PerLanguage {
                english: stats(english, &english_strings),
                french: stats(french, &english_strings),
            },
        }
    }

    pub(super) fn of(&self, language: Language) -> &Stats {
        self.stats.of(language)
    }
}

impl fmt::Debug for TradeStats {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TradeStats")
            .field("english", &self.stats.english.len())
            .field("french", &self.stats.french.len())
            .finish()
    }
}

/// The stats of the templates of one language, one per string (without its note) and option.
/// The English template of a stat is the one of its first id in `english`, or else its own
/// string; its value is written as in its template. Linear in the number of templates: the
/// strings and the ids of each stat are told apart by sets, not by searching its lists.
fn stats(templates: &[StatTemplate], english: &HashMap<&str, String>) -> Stats {
    let mut list: Vec<Stat> = Vec::new();
    let mut by_string: HashMap<(String, Option<i32>), usize> = HashMap::new();
    let mut seen_strings: HashSet<(usize, String)> = HashSet::new();
    let mut seen_ids: HashSet<(usize, String)> = HashSet::new();
    for template in templates {
        let Some((group, id, option)) = parse_id(&template.id) else {
            continue;
        };
        let string = string(&template.text);
        let index = *by_string
            .entry((string.clone(), option))
            .or_insert_with(|| {
                let reference = english.get(template.id.as_str()).unwrap_or(&string);
                let index = list.len();
                list.push(Stat {
                    reference: reference.clone(),
                    id: None,
                    better: Better::Neither,
                    matchers: Vec::new(),
                    ids: BTreeMap::new(),
                    option: option.is_some(),
                    inverted: false,
                });
                index
            });
        let Some(stat) = list.get_mut(index) else {
            continue;
        };
        for string in [string, normalize(&template.text)] {
            if seen_strings.insert((index, string.clone())) {
                stat.matchers.push(Matcher {
                    string,
                    value: option.map(f64::from),
                    negate: false,
                });
            }
        }
        // The id holds its group: the same id is the same group.
        if seen_ids.insert((index, id.clone())) {
            stat.ids.entry(group).or_default().push(id);
        }
    }
    Stats::new(list)
}

/// The group, the id and the option of a trade id: `enchant.stat_2954116742|46224` is the
/// option 46224 of `enchant.stat_2954116742`. None for a group without modifier lines, or an
/// option that isn't a number.
fn parse_id(id: &str) -> Option<(TradeGroup, String, Option<i32>)> {
    let (name, _) = id.split_once('.')?;
    let group = TradeGroup::of_line(name)?;
    match id.split_once('|') {
        Some((id, option)) => Some((group, id.to_owned(), Some(option.parse().ok()?))),
        None => Some((group, id.to_owned(), None)),
    }
}

/// A template as a line is looked up: normalized, without the note that ends some of them
/// (`(Local)`, `(Jewel)`, `(Gold Piles)`…), a few words after a space, without number.
fn string(text: &str) -> String {
    let text = text.trim_end();
    let note = text
        .strip_suffix(')')
        .and_then(|rest| rest.rsplit_once(" ("))
        .filter(|(_, note)| !note.contains(|character: char| "#()\n".contains(character)))
        .filter(|(_, note)| !note.contains(|character: char| character.is_ascii_digit()));
    normalize(note.map_or(text, |(before, _)| before))
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// The templates of `pairs` of id and text.
    fn templates(pairs: &[(&str, &str)]) -> Vec<StatTemplate> {
        pairs
            .iter()
            .map(|&(id, text)| StatTemplate {
                id: id.to_owned(),
                text: text.to_owned(),
            })
            .collect()
    }

    /// A few templates of the trade site on 2026-10-26, which Exiled Exchange 2 doesn't
    /// know (the Sacrifice), or not in their group (the Mark skills and the prefixes allowed as a
    /// rune), and fake new
    /// modifiers; one it knows (the maximum Life).
    pub fn trade_stats() -> TradeStats {
        TradeStats::new(
            &templates(&[
                ("explicit.stat_3299347043", "# to maximum Life"),
                (
                    "explicit.stat_3076483222|49977",
                    "Sacrifice up to 10 Chaos Orbs to receive double on Trial completion",
                ),
                ("rune.stat_1992191903", "# to Level of all Mark Skills"),
                ("rune.stat_3182714256", "# Prefix Modifier allowed"),
                ("explicit.stat_1000000001", "#% reduced Damage per Patch"),
                (
                    "explicit.stat_1000000002",
                    "Gain # Patches\nwhile it Patches",
                ),
            ]),
            &templates(&[
                (
                    "explicit.stat_1000000001",
                    "#% de Réduction des Dégâts par patch",
                ),
                (
                    "explicit.stat_1000000003",
                    "#% de Réduction de quelque chose",
                ),
            ]),
        )
    }

    #[test]
    fn the_note_at_the_end_of_a_template_is_taken_out() {
        assert_eq!(
            string("# to Accuracy Rating (Local)"),
            "# to Accuracy Rating"
        );
        assert_eq!(string("# uses remaining (Gold Piles) "), "# uses remaining");
        assert_eq!(
            string("+#% to Spirit (Piles d'Or)"),
            "+#% to Spirit",
            "a note of French"
        );
        assert_eq!(
            string("Adds Irradiated to a Map \n# use remaining"),
            "Adds Irradiated to a Map\n# use remaining"
        );
        assert_eq!(
            string("Oasis(Ancestral Bond-Zealot's Oath)"),
            "Oasis(Ancestral Bond-Zealot's Oath)",
            "a part of the string, without a space before"
        );
        assert_eq!(string("# to Level (# times)"), "# to Level (# times)");
    }

    #[test]
    fn a_trade_id_gives_its_group_and_its_option() {
        assert_eq!(
            parse_id("enchant.stat_2954116742|46224"),
            Some((
                TradeGroup::Enchant,
                "enchant.stat_2954116742".to_owned(),
                Some(46224)
            ))
        );
        assert_eq!(
            parse_id("rune.stat_1992191903"),
            Some((TradeGroup::Rune, "rune.stat_1992191903".to_owned(), None))
        );
        assert_eq!(parse_id("pseudo.pseudo_total_life"), None);
        assert_eq!(parse_id("sanctum.stat_2878762585"), None);
        assert_eq!(parse_id("explicit.stat_1|first"), None);
        assert_eq!(parse_id("explicit"), None);
    }

    #[test]
    fn the_stats_of_the_same_template_are_one_stat_with_all_their_ids() {
        let english = templates(&[
            ("explicit.stat_803737631", "# to Accuracy Rating"),
            ("explicit.stat_691932474", "# to Accuracy Rating (Local)"),
            ("fractured.stat_691932474", "# to Accuracy Rating (Local)"),
            ("explicit.stat_691932474", "# to Accuracy Rating (Local)"),
        ]);
        let stats = TradeStats::new(&english, &[]);

        let found: Vec<_> = stats
            .of(Language::English)
            .find("# to Accuracy Rating")
            .collect();

        let [(stat, 0)] = found.as_slice() else {
            panic!("{found:?}");
        };
        assert_eq!(
            stats
                .of(Language::English)
                .find("# to Accuracy Rating (Local)")
                .count(),
            1,
            "with its note too"
        );
        assert_eq!(
            stat.trade_ids(TradeGroup::Explicit),
            ["explicit.stat_803737631", "explicit.stat_691932474"]
        );
        assert_eq!(
            stat.trade_ids(TradeGroup::Fractured),
            ["fractured.stat_691932474"]
        );
    }

    #[test]
    fn a_stat_has_the_english_template_of_its_id() {
        let stats = trade_stats();

        let (stat, _) = stats
            .of(Language::French)
            .find("#% de Réduction des Dégâts par patch")
            .next()
            .unwrap();
        assert_eq!(stat.reference, "#% reduced Damage per Patch");

        let (stat, _) = stats
            .of(Language::French)
            .find("#% de Réduction de quelque chose")
            .next()
            .unwrap();
        assert_eq!(
            stat.reference, "#% de Réduction de quelque chose",
            "no English template: its own"
        );
    }

    #[test]
    fn an_option_is_the_value_of_its_template() {
        let stats = trade_stats();

        let (stat, index) = stats
            .of(Language::English)
            .find("Sacrifice up to 10 Chaos Orbs to receive double on Trial completion")
            .next()
            .unwrap();

        assert!(stat.option);
        assert_eq!(stat.matchers[index].value, Some(49977.0));
        assert_eq!(
            stat.trade_ids(TradeGroup::Explicit),
            ["explicit.stat_3076483222"]
        );
    }

    /// The most templates the trade site may send in a language (`MAX_STAT_TEMPLATES` of
    /// `trade::site`), of one text: searching the lists of a stat for each would take minutes
    /// (the security audit of 2026-11-14).
    #[test]
    fn many_templates_of_one_text_are_grouped_at_once() {
        const COUNT: u32 = 50_000;
        // A note of letters for each number: a note with a digit stays in the text.
        let note = |number: u32| -> String {
            number
                .to_string()
                .bytes()
                .map(|digit| char::from(b'a' + (digit - b'0')))
                .collect()
        };
        let english: Vec<StatTemplate> = (0..COUNT)
            .flat_map(|number| {
                [
                    (format!("explicit.stat_{number}"), "# to Life".to_owned()),
                    (
                        "explicit.stat_1".to_owned(),
                        format!("# to Mana ({})", note(number)),
                    ),
                    (
                        format!("explicit.stat_2|{number}"),
                        "Allocates #".to_owned(),
                    ),
                ]
            })
            .map(|(id, text)| StatTemplate { id, text })
            .collect();
        let start = std::time::Instant::now();

        let stats = TradeStats::new(&english, &[]);

        assert!(start.elapsed() < std::time::Duration::from_secs(5));
        let english = stats.of(Language::English);
        let (life, _) = english.find("# to Life").next().unwrap();
        assert_eq!(life.trade_ids(TradeGroup::Explicit).len(), 50_000);
        assert_eq!(english.find("# to Mana").count(), 1);
        assert_eq!(english.find("# to Mana (bcd)").count(), 1);
        assert_eq!(english.find("Allocates #").count(), 50_000);
    }
}
