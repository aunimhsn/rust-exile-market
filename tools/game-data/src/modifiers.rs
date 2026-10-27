//! The modifier tables, from the RePoE fork (`mods.json`, `base_items.json` and
//! `stat_translations/stat_descriptions.json`): the tiers of each prefix and suffix that rolls
//! on the equipment, the jewels, the flasks and the charms, and the tags of each base that
//! tell where a modifier rolls.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde::{Deserialize, Serialize};

use crate::Result;

/// The domains of the modifiers kept: the equipment, the jewels, the flasks and the charms.
const DOMAINS: [&str; 3] = ["item", "misc", "flask"];

/// A modifier of `mods.json`, with the fields the generator reads.
#[derive(Deserialize)]
pub struct SourceModifier {
    domain: String,
    generation_type: String,
    is_essence_only: bool,
    /// The modifiers of one type can't be on the same item: its tiers.
    #[serde(rename = "type")]
    kind: String,
    required_level: u32,
    spawn_weights: Vec<SpawnWeight>,
    stats: Vec<SourceStat>,
}

/// The weight of a modifier on the bases with a tag: the first tag of the list a base has
/// tells whether the modifier rolls on it (above zero) or not.
#[derive(Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SpawnWeight {
    tag: String,
    weight: u32,
}

#[derive(Deserialize)]
struct SourceStat {
    id: String,
    min: i64,
    max: i64,
}

/// A base of `base_items.json`, with the fields the generator reads.
#[derive(Deserialize)]
pub struct SourceBase {
    name: String,
    release_state: String,
    tags: Vec<String>,
}

/// A stat translation of `stat_descriptions.json`: the text of one stat or more.
#[derive(Deserialize)]
pub struct Translation {
    ids: Vec<String>,
    #[serde(rename = "English")]
    english: Vec<Variant>,
    hidden: Option<bool>,
}

/// One way of writing the stats of a translation, e.g. `{0}% increased maximum Life` and,
/// for a negative value, `{0}% reduced maximum Life`.
#[derive(Deserialize)]
struct Variant {
    string: String,
    /// For each stat of the translation, the handlers of its value, e.g.
    /// `per_minute_to_per_second`.
    index_handlers: Vec<Vec<String>>,
}

/// The tiers of a type of modifier that roll on the same bases, as the application reads them
/// (`src/game_data/modifiers.rs`).
#[derive(Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Modifier {
    #[serde(rename = "type")]
    kind: String,
    affix: String,
    /// The English template of each line, as Exiled Exchange 2 writes it (`Adds # to #
    /// Lightning Damage`).
    lines: Vec<String>,
    /// Each tag and its weight, as `["bow", 1]`.
    spawn: Vec<(String, u32)>,
    tiers: Vec<Tier>,
}

#[derive(Serialize, PartialEq, Eq, PartialOrd, Ord)]
struct Tier {
    level: u32,
    /// The range of each `#` of the lines, in their order, as the game shows them (per second
    /// for a value stored per minute), with the sign of the stat.
    ranges: Vec<[Number; 2]>,
}

/// A number written as an integer when it is one.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Number {
    /// The number times [`Number::SCALE`], exact for the handlers of the game.
    scaled: i64,
}

impl Number {
    const SCALE: i64 = 60_000;
}

impl Serialize for Number {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        if self.scaled % Self::SCALE == 0 {
            serializer.serialize_i64(self.scaled / Self::SCALE)
        } else {
            #[expect(
                clippy::cast_precision_loss,
                reason = "the values of the game are far below 2^52"
            )]
            let value = self.scaled as f64 / Self::SCALE as f64;
            // Two decimals, as the game shows them at most.
            serializer.serialize_f64((value * 100.0).round() / 100.0)
        }
    }
}

/// The modifiers of one type and affix, with the same lines, that roll on the same bases.
type Key = (String, String, Vec<String>, Vec<SpawnWeight>);

/// The tags of each base, by its English name.
pub type BaseTags = BTreeMap<String, Vec<String>>;

/// The modifier tables of `modifiers`, the lines of their stats written as the English
/// templates of Exiled Exchange 2 (`references`), and the tags of the bases that the tables
/// read, by English name of the base.
pub fn reduce(
    modifiers: &BTreeMap<String, SourceModifier>,
    bases: &BTreeMap<String, SourceBase>,
    translations: &[Translation],
    references: &BTreeSet<String>,
) -> Result<(Vec<Modifier>, BaseTags)> {
    let mut by_stat: HashMap<&str, usize> = HashMap::new();
    for (index, translation) in translations.iter().enumerate() {
        for id in &translation.ids {
            by_stat.entry(id.as_str()).or_insert(index);
        }
    }
    let mut grouped: BTreeMap<Key, Vec<Tier>> = BTreeMap::new();
    let mut unknown_lines = BTreeSet::new();
    for (id, modifier) in modifiers {
        let rolls = DOMAINS.contains(&modifier.domain.as_str())
            && ["prefix", "suffix"].contains(&modifier.generation_type.as_str())
            && !modifier.is_essence_only
            && modifier.spawn_weights.iter().any(|spawn| spawn.weight > 0);
        if !rolls {
            continue;
        }
        let (lines, ranges) = lines(
            modifier,
            translations,
            &by_stat,
            references,
            &mut unknown_lines,
        )
        .map_err(|error| format!("{id}: {error}"))?;
        grouped
            .entry((
                modifier.kind.clone(),
                modifier.generation_type.clone(),
                lines,
                modifier.spawn_weights.clone(),
            ))
            .or_default()
            .push(Tier {
                level: modifier.required_level,
                ranges,
            });
    }
    for line in unknown_lines {
        println!("Modifier line without template: {line}");
    }

    let tags_used: BTreeSet<String> = grouped
        .keys()
        .flat_map(|(_, _, _, spawn)| spawn.iter().map(|spawn| spawn.tag.clone()))
        .collect();
    let mut base_tags = BaseTags::new();
    for base in bases.values() {
        if base.release_state != "released" {
            continue;
        }
        let tags: Vec<String> = base
            .tags
            .iter()
            .filter(|tag| tags_used.contains(*tag))
            .cloned()
            .collect();
        // A name of several bases, e.g. the "Random Helmet" of each attribute, the
        // application can't tell apart: it gets no tag.
        match base_tags.get(&base.name) {
            Some(known) if *known != tags => {
                base_tags.insert(base.name.clone(), Vec::new());
            }
            Some(_) => {}
            None => {
                base_tags.insert(base.name.clone(), tags);
            }
        }
    }

    let base_tag_set: BTreeSet<&String> = base_tags.values().flatten().collect();
    let mut tables = Vec::new();
    for ((kind, affix, lines, spawn), mut tiers) in grouped {
        tiers.sort();
        // Only the tags of the bases matter, in their order: the first one a base has decides.
        let spawn = spawn
            .into_iter()
            .filter(|spawn| base_tag_set.contains(&spawn.tag))
            .map(|spawn| (spawn.tag, spawn.weight))
            .collect();
        tables.push(Modifier {
            kind,
            affix,
            lines,
            spawn,
            tiers,
        });
    }
    Ok((tables, base_tags))
}

/// The lines of `modifier`, each as the English template of Exiled Exchange 2, and the range
/// of each `#` of these lines; a line without template is written as the game writes it, and
/// added to `unknown`.
fn lines(
    modifier: &SourceModifier,
    translations: &[Translation],
    by_stat: &HashMap<&str, usize>,
    references: &BTreeSet<String>,
    unknown: &mut BTreeSet<String>,
) -> Result<(Vec<String>, Vec<[Number; 2]>)> {
    let mut lines = Vec::new();
    let mut ranges = Vec::new();
    // The translations written, once each: one can write several stats.
    let mut written = Vec::new();
    for stat in &modifier.stats {
        let Some(&index) = by_stat.get(stat.id.as_str()) else {
            // A stat the game doesn't write, e.g. `dummy_stat_display_nothing`.
            continue;
        };
        let Some(translation) = translations.get(index) else {
            continue;
        };
        if translation.hidden == Some(true) || written.contains(&index) {
            continue;
        }
        written.push(index);
        let variant = translation
            .english
            .iter()
            .find(|variant| references.contains(&template(&variant.string)))
            .or_else(|| translation.english.first())
            .ok_or("a translation without text")?;
        let line = template(&variant.string);
        if !references.contains(&line) {
            unknown.insert(line.clone());
        }
        for index in placeholders(&variant.string) {
            let id = translation
                .ids
                .get(index)
                .ok_or_else(|| format!("no stat {index} in {}", variant.string))?;
            let (min, max) = modifier
                .stats
                .iter()
                .find(|stat| stat.id == *id)
                .map_or((0, 0), |stat| (stat.min, stat.max));
            let handlers = variant
                .index_handlers
                .get(index)
                .map_or(&[][..], Vec::as_slice);
            ranges.push([scale(min, handlers)?, scale(max, handlers)?]);
        }
        lines.push(line);
    }
    Ok((lines, ranges))
}

/// The template of a string of the game, as Exiled Exchange 2 writes it: `{0:+d} to
/// [Accuracy|Accuracy] Rating` is `# to Accuracy Rating`.
fn template(string: &str) -> String {
    let mut text = String::new();
    let mut rest = string;
    while let Some(start) = rest.find(['{', '[']) {
        text.push_str(&rest[..start]);
        let close = if rest[start..].starts_with('{') {
            '}'
        } else {
            ']'
        };
        let Some(end) = rest[start..].find(close).map(|end| start + end) else {
            break;
        };
        let inside = &rest[start + 1..end];
        if close == '}' {
            text.push('#');
        } else {
            // A link of the game, `[Accuracy|Accuracy]`: its text is after the bar.
            text.push_str(inside.rsplit('|').next().unwrap_or(inside));
        }
        rest = &rest[end + 1..];
    }
    text.push_str(rest);
    text
}

/// The index of the stat of each placeholder of `string`, in their order: `Adds {0} to {1}`
/// gives 0 and 1, and `{}` the next index.
fn placeholders(string: &str) -> Vec<usize> {
    let mut indexes = Vec::new();
    let mut rest = string;
    while let Some(start) = rest.find('{') {
        let Some(end) = rest[start..].find('}').map(|end| start + end) else {
            break;
        };
        let inside = &rest[start + 1..end];
        let number = inside.split(':').next().unwrap_or_default();
        indexes.push(number.parse().unwrap_or(indexes.len()));
        rest = &rest[end + 1..];
    }
    indexes
}

/// A value of a stat as the game shows it, e.g. per second for a value stored per minute;
/// the sign stays that of the stat (`negate` writes `reduced` in place of a minus sign).
fn scale(value: i64, handlers: &[String]) -> Result<Number> {
    let mut scaled = value * Number::SCALE;
    for handler in handlers {
        let divisor = match handler.as_str() {
            "negate" => 1,
            "divide_by_one_hundred" => 100,
            "per_minute_to_per_second" | "per_minute_to_per_second_2dp" => 60,
            "divide_by_ten_1dp_if_required" => 10,
            "milliseconds_to_seconds" => 1000,
            // A new handler is decided before it is embedded.
            other => return Err(format!("unknown handler {other}").into()),
        };
        if scaled % divisor != 0 {
            return Err(format!("{value} can't be divided by {divisor}").into());
        }
        scaled /= divisor;
    }
    Ok(Number { scaled })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_string_of_the_game_becomes_the_template_of_exiled_exchange_2() {
        assert_eq!(
            template("{0:+d} to [Accuracy|Accuracy] Rating"),
            "# to Accuracy Rating"
        );
        assert_eq!(
            template("[LifeLeech|Leech] {0}% of [Physical] [Attack] Damage as Life"),
            "Leech #% of Physical Attack Damage as Life"
        );
        assert_eq!(placeholders("Adds {1} to {0} Damage, {} more"), [1, 0, 2]);
    }

    #[test]
    fn a_value_per_minute_is_shown_per_second() {
        let per_second = scale(1386, &["per_minute_to_per_second".to_owned()]).expect("scaled");
        let permyriad = scale(
            -790,
            &["divide_by_one_hundred".to_owned(), "negate".to_owned()],
        )
        .expect("scaled");

        assert_eq!(serde_json::to_string(&per_second).expect("written"), "23.1");
        assert_eq!(serde_json::to_string(&permyriad).expect("written"), "-7.9");
        assert!(scale(1, &["new_handler".to_owned()]).is_err());
    }
}
