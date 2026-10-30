//! The modifier tables, from the RePoE fork (`mods.json`, `base_items.json` and
//! `stat_translations/stat_descriptions.json`): the tiers of each prefix and suffix that rolls
//! on the equipment, the jewels, the flasks and the charms, and the tags of each base that
//! tell where a modifier rolls.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde::{Deserialize, Serialize};

use crate::Result;

/// The domain of the modifiers of the equipment, which the application takes when none is
/// written.
pub const EQUIPMENT: &str = "item";

/// The domains of the modifiers kept: the equipment, the jewels, the flasks and the charms;
/// each one a domain the application knows (`Domain` in `src/game_data/bases.rs`).
const DOMAINS: [&str; 3] = [EQUIPMENT, "misc", "flask"];

/// Whether `tag` is one of the Time-Lost jewels: their modifiers are written by the game
/// within `Small Passive Skills in Radius also grant …` or `Notable Passive Skills…`, which
/// RePoE doesn't say; left out until step 13k.
fn radius_jewel(tag: &str) -> bool {
    tag == "radius_jewel" || tag.ends_with("_radius_jewel")
}

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
    /// The domain of the modifiers that roll on it: `item` for the equipment, `flask` for the
    /// flasks and the charms, `misc` for the jewels.
    domain: String,
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
#[derive(Serialize)]
pub struct Modifier {
    #[serde(rename = "type")]
    kind: String,
    affix: String,
    /// The domain of the bases it rolls on, written when it isn't `item`: a tag of every base,
    /// `default`, rolls on the bases of its domain only (the charges of the flasks).
    #[serde(skip_serializing_if = "is_item")]
    domain: String,
    lines: Vec<Line>,
    /// Each tag and its weight, as `["bow", 1]`.
    spawn: Vec<(String, u32)>,
    tiers: Vec<Tier>,
}

/// A line of a modifier.
#[derive(Serialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Line {
    /// Its English template, as Exiled Exchange 2 writes it (`Adds # to # Lightning Damage`).
    template: String,
    /// What the value of the stat is divided by to be shown, e.g. 1000 for milliseconds shown
    /// in seconds: the value of a line written without number (`Inherent Rage loss starts 1
    /// second later`) is that of the stat, 1000.
    #[serde(skip_serializing_if = "is_one")]
    divisor: i64,
    /// Whether the template is one of Exiled Exchange 2: a line without template can't be
    /// found, nor its modifier.
    #[serde(skip)]
    known: bool,
}

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "the signature serde asks for"
)]
fn is_one(value: &i64) -> bool {
    *value == 1
}

/// Whether `domain` is one of the modifiers kept, which the application knows.
pub fn is_kept(domain: &str) -> bool {
    DOMAINS.contains(&domain)
}

/// Whether `domain` is that of the equipment, left unwritten.
pub fn is_item(domain: &String) -> bool {
    domain == EQUIPMENT
}

#[derive(Serialize, PartialEq, Eq, PartialOrd, Ord)]
struct Tier {
    level: u32,
    /// For each line, the range of each of its values, as the game shows them (per second for
    /// a value stored per minute), with the sign of the stat.
    ranges: Vec<LineRanges>,
}

/// The range of each value of a line.
type LineRanges = Vec<[Number; 2]>;

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
                reason = "the values of the game are far below 2^53"
            )]
            let value = self.scaled as f64 / Self::SCALE as f64;
            // Two decimals, as the game shows them at most.
            serializer.serialize_f64((value * 100.0).round() / 100.0)
        }
    }
}

/// The modifiers of one type and affix, with the same lines, that roll on the same bases.
type Key = (String, String, String, Vec<Line>, Vec<SpawnWeight>);

/// What tells the modifiers that roll on a base: their domain and the tags of the base.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BaseSpawn {
    pub domain: String,
    pub tags: Vec<String>,
}

/// The domain and the tags of each base, by its English name.
pub type BaseTags = BTreeMap<String, BaseSpawn>;

/// The English template of Exiled Exchange 2 (its `ref`) of each of its English strings, and of
/// each template itself: `Loads # additional bolts` is `Loads an additional bolt`.
pub type Templates = BTreeMap<String, String>;

/// The modifier tables of `modifiers`, the lines of their stats written as the English
/// templates of Exiled Exchange 2, and the tags of the bases that the tables read, by English
/// name of the base.
pub fn reduce(
    modifiers: &BTreeMap<String, SourceModifier>,
    bases: &BTreeMap<String, SourceBase>,
    translations: &[Translation],
    templates: &Templates,
) -> Result<(Vec<Modifier>, BaseTags)> {
    let mut by_stat: HashMap<&str, usize> = HashMap::new();
    for (index, translation) in translations.iter().enumerate() {
        for id in &translation.ids {
            by_stat.entry(id.as_str()).or_insert(index);
        }
    }
    let mut grouped: BTreeMap<Key, Vec<Tier>> = BTreeMap::new();
    for (id, modifier) in modifiers {
        let rolls = DOMAINS.contains(&modifier.domain.as_str())
            && ["prefix", "suffix"].contains(&modifier.generation_type.as_str())
            && !modifier.is_essence_only
            && modifier
                .spawn_weights
                .iter()
                .any(|spawn| spawn.weight > 0 && !radius_jewel(&spawn.tag));
        if !rolls {
            continue;
        }
        let (lines, ranges) = lines(modifier, translations, &by_stat, templates)
            .map_err(|error| format!("{id}: {error}"))?;
        grouped
            .entry((
                modifier.kind.clone(),
                modifier.generation_type.clone(),
                modifier.domain.clone(),
                lines,
                modifier.spawn_weights.clone(),
            ))
            .or_default()
            .push(Tier {
                level: modifier.required_level,
                ranges,
            });
    }

    let released: Vec<&SourceBase> = bases
        .values()
        .filter(|base| base.release_state == "released")
        .collect();
    let tags_of_bases: BTreeSet<&String> = released.iter().flat_map(|base| &base.tags).collect();
    let mut tables = Vec::new();
    let mut unknown_lines = BTreeSet::new();
    for ((kind, affix, domain, lines, spawn), mut tiers) in grouped {
        // Only the tags of the bases matter, in their order: the first one a base has decides.
        let spawn: Vec<(String, u32)> = spawn
            .into_iter()
            .filter(|spawn| tags_of_bases.contains(&spawn.tag))
            .map(|spawn| (spawn.tag, spawn.weight))
            .collect();
        // A modifier that rolls on no released base, e.g. one of Path of Exile left in the
        // files.
        if !spawn.iter().any(|&(_, weight)| weight > 0) {
            continue;
        }
        // A line without template can't be found, nor its modifier.
        let unknown: Vec<&Line> = lines.iter().filter(|line| !line.known).collect();
        if !unknown.is_empty() {
            unknown_lines.extend(unknown.iter().map(|line| line.template.clone()));
            continue;
        }
        tiers.sort();
        tables.push(Modifier {
            kind,
            affix,
            domain,
            lines,
            spawn,
            tiers,
        });
    }
    for line in unknown_lines {
        println!("Modifier line without template, its modifier left out: {line}");
    }
    let base_tags = base_tags(&released, &tables);
    Ok((tables, base_tags))
}

/// The domain of each base among `released` and its tags that tell where a modifier of
/// `tables` rolls, by its name.
fn base_tags(released: &[&SourceBase], tables: &[Modifier]) -> BaseTags {
    let tags_used: BTreeSet<&String> = tables
        .iter()
        .flat_map(|table| table.spawn.iter().map(|(tag, _)| tag))
        .collect();
    let mut base_tags = BaseTags::new();
    for base in released {
        let spawn = BaseSpawn {
            domain: base.domain.clone(),
            tags: base
                .tags
                .iter()
                .filter(|tag| tags_used.contains(tag))
                .cloned()
                .collect(),
        };
        // A name of several bases, e.g. the "Random Helmet" of each attribute, the
        // application can't tell apart: it gets no tag.
        match base_tags.get_mut(&base.name) {
            Some(known) if *known != spawn => known.tags.clear(),
            Some(_) => {}
            None => {
                base_tags.insert(base.name.clone(), spawn);
            }
        }
    }
    base_tags
}

/// The lines of `modifier`, each with the English template of Exiled Exchange 2 of one of its
/// strings, and the range of each of their values. A line without template is marked unknown
/// (written as the game writes it, without range): [`reduce`] leaves its modifier out.
fn lines(
    modifier: &SourceModifier,
    translations: &[Translation],
    by_stat: &HashMap<&str, usize>,
    templates: &Templates,
) -> Result<(Vec<Line>, Vec<LineRanges>)> {
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
        let (line, line_ranges) = line(modifier, translation, templates)?;
        lines.push(line);
        ranges.push(line_ranges);
    }
    Ok((lines, ranges))
}

/// The line `translation` writes for `modifier`, and the range of each of its values.
fn line(
    modifier: &SourceModifier,
    translation: &Translation,
    templates: &Templates,
) -> Result<(Line, LineRanges)> {
    let first = translation
        .english
        .first()
        .ok_or("a translation without text")?;
    let Some(template) = translation
        .english
        .iter()
        .find_map(|variant| templates.get(&template(&variant.string)))
        .cloned()
    else {
        // Without template, the line can't be found: written as the game writes it.
        let line = Line {
            template: template(&first.string),
            divisor: 1,
            known: false,
        };
        return Ok((line, Vec::new()));
    };
    // The values are those of the string that writes the most of them: `Loads {0} additional
    // bolts`, not `Loads an additional bolt` (`max_by_key` keeps the last of the maximums: the
    // variants reversed, the first one).
    let general = translation
        .english
        .iter()
        .rev()
        .max_by_key(|variant| placeholders(&variant.string).len())
        .unwrap_or(first);
    let mut indexes = placeholders(&general.string);
    // A template with a value no string writes (`Upgrades Radius to #`, `Upgrades Radius to
    // Medium`) has the values of its stats, as Exiled Exchange 2 gives them; a template
    // without value (`Instant Recovery`) has none.
    if indexes.is_empty() && template.contains('#') {
        indexes = (0..translation.ids.len())
            .filter(|&index| {
                translation
                    .ids
                    .get(index)
                    .is_some_and(|id| stat_of(modifier, id).is_some())
            })
            .collect();
    }
    let handlers = |index: usize| {
        general
            .index_handlers
            .get(index)
            .map_or(&[][..], Vec::as_slice)
    };
    // A stat that rolls is written by a value of its line, or its range would be lost.
    for (index, id) in translation.ids.iter().enumerate() {
        let rolls = stat_of(modifier, id).is_some_and(|stat| stat.min != stat.max);
        if rolls && !indexes.contains(&index) {
            return Err(format!("{id} rolls but {} doesn't write it", general.string).into());
        }
    }
    let mut ranges = Vec::new();
    for &index in &indexes {
        let id = translation
            .ids
            .get(index)
            .ok_or_else(|| format!("no stat {index} in {}", general.string))?;
        // A stat the modifier doesn't have is 0, e.g. `local_weapon_no_physical_damage`.
        let (min, max) = stat_of(modifier, id).map_or((0, 0), |stat| (stat.min, stat.max));
        ranges.push([scale(min, handlers(index))?, scale(max, handlers(index))?]);
    }
    // A string without number of Exiled Exchange 2 gives the value of the stat, e.g. 1000
    // milliseconds for `Inherent Rage loss starts 1 second later`, which the application
    // divides to compare it with the ranges as the game shows them; a line without such a
    // string needs no divisor.
    let written = translation.english.iter().any(|variant| {
        placeholders(&variant.string).is_empty()
            && templates.contains_key(&self::template(&variant.string))
    });
    let divisor = match (written, indexes.as_slice()) {
        (false, _) | (true, []) => 1,
        (true, &[index]) => divisor_of(handlers(index))?,
        // Which of its values the string without number gives is to decide.
        (true, _) => {
            return Err(format!(
                "{} has several values and a string without number",
                general.string
            )
            .into());
        }
    };
    let line = Line {
        template,
        divisor,
        known: true,
    };
    Ok((line, ranges))
}

/// The stat `id` of `modifier`, if it has it.
fn stat_of<'a>(modifier: &'a SourceModifier, id: &str) -> Option<&'a SourceStat> {
    modifier.stats.iter().find(|stat| stat.id == id)
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
            // Not closed: written as it is.
            rest = &rest[start..];
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

/// What the value of a stat is divided by to be shown with `handlers`, e.g. 60 for a value
/// stored per minute shown per second; the sign stays that of the stat (`negate` writes
/// `reduced` in place of a minus sign).
fn divisor_of(handlers: &[String]) -> Result<i64> {
    let mut divisor = 1;
    for handler in handlers {
        divisor *= match handler.as_str() {
            "negate" => 1,
            "divide_by_one_hundred" => 100,
            "per_minute_to_per_second"
            | "per_minute_to_per_second_2dp"
            | "per_minute_to_per_second_2dp_if_required" => 60,
            "divide_by_ten_1dp_if_required" => 10,
            "milliseconds_to_seconds" => 1000,
            // A new handler is decided before it is embedded.
            other => return Err(format!("unknown handler {other}").into()),
        };
    }
    Ok(divisor)
}

/// A value of a stat as the game shows it with `handlers` ([`divisor_of`]).
fn scale(value: i64, handlers: &[String]) -> Result<Number> {
    let divisor = divisor_of(handlers)?;
    let scaled = value * Number::SCALE;
    if scaled % divisor != 0 {
        return Err(format!("{value} can't be divided by {divisor}").into());
    }
    Ok(Number {
        scaled: scaled / divisor,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tables and the tags of the bases from `modifiers` of `mods.json`, `bases` of
    /// `base_items.json` and `translations` of `stat_descriptions.json`, written as JSON, the
    /// templates of Exiled Exchange 2 being `templates` (string, template).
    fn tables(
        modifiers: &str,
        bases: &str,
        translations: &str,
        templates: &[(&str, &str)],
    ) -> (serde_json::Value, BaseTags) {
        let templates = templates
            .iter()
            .map(|&(string, template)| (string.to_owned(), template.to_owned()))
            .collect();
        let (tables, tags) = reduce(
            &serde_json::from_str(modifiers).expect("modifiers"),
            &serde_json::from_str(bases).expect("bases"),
            &serde_json::from_str::<Vec<_>>(translations).expect("translations"),
            &templates,
        )
        .expect("reduced");
        (serde_json::to_value(tables).expect("written"), tags)
    }

    /// A modifier of `mods.json`, rolling where `spawn` says, with `stats`.
    fn modifier(kind: &str, level: u32, spawn: &str, stats: &str) -> String {
        format!(
            r#"{{"domain": "item", "generation_type": "suffix", "is_essence_only": false, "type": "{kind}", "required_level": {level}, "spawn_weights": {spawn}, "stats": {stats}}}"#
        )
    }

    const BASES: &str = r#"{
        "Bow": {"domain": "item", "name": "Bow", "release_state": "released", "tags": ["bow", "weapon", "default"]},
        "Ring": {"domain": "item", "name": "Ring", "release_state": "released", "tags": ["ring", "default"]},
        "Flask": {"domain": "flask", "name": "Flask", "release_state": "released", "tags": ["life_flask", "default"]},
        "Old": {"domain": "item", "name": "Old", "release_state": "unreleased", "tags": ["fishing_rod", "default"]}
    }"#;

    #[test]
    fn the_tiers_of_a_type_on_the_same_bases_make_one_table_sorted_by_level() {
        let weapon = r#"[{"tag": "weapon", "weight": 1}, {"tag": "default", "weight": 0}]"#;
        let ring = r#"[{"tag": "ring", "weight": 1}, {"tag": "default", "weight": 0}]"#;
        let speed = |min: u32, max: u32| {
            format!(r#"[{{"id": "attack_speed_+%", "min": {min}, "max": {max}}}]"#)
        };
        let modifiers = format!(
            r#"{{"Speed2": {}, "Speed1": {}, "RingSpeed1": {}}}"#,
            modifier("Speed", 20, weapon, &speed(8, 10)),
            modifier("Speed", 1, weapon, &speed(5, 7)),
            modifier("Speed", 1, ring, &speed(3, 4)),
        );
        let translations = r#"[{"ids": ["attack_speed_+%"], "English": [{"string": "{0}% increased Attack Speed", "index_handlers": [[]]}], "hidden": null}]"#;

        let (tables, _) = tables(
            &modifiers,
            BASES,
            translations,
            &[("#% increased Attack Speed", "#% increased Attack Speed")],
        );

        assert_eq!(
            tables,
            // In the order of their spawn weights.
            serde_json::json!([
                {"type": "Speed", "affix": "suffix", "lines": [{"template": "#% increased Attack Speed"}],
                 "spawn": [["ring", 1], ["default", 0]],
                 "tiers": [{"level": 1, "ranges": [[[3, 4]]]}]},
                {"type": "Speed", "affix": "suffix", "lines": [{"template": "#% increased Attack Speed"}],
                 "spawn": [["weapon", 1], ["default", 0]],
                 "tiers": [{"level": 1, "ranges": [[[5, 7]]]}, {"level": 20, "ranges": [[[8, 10]]]}]}
            ])
        );
    }

    #[test]
    fn a_table_rolling_on_no_released_base_is_left_out_with_the_tags_of_no_base() {
        let modifiers = format!(
            r#"{{"Fishing": {}, "Ring": {}}}"#,
            modifier(
                "Fishing",
                1,
                r#"[{"tag": "fishing_rod", "weight": 1}, {"tag": "default", "weight": 0}]"#,
                r#"[{"id": "fish_quantity_+%", "min": 5, "max": 10}]"#
            ),
            modifier(
                "Life",
                1,
                r#"[{"tag": "amulet", "weight": 1}, {"tag": "ring", "weight": 1}, {"tag": "default", "weight": 0}]"#,
                r#"[{"id": "base_maximum_life", "min": 5, "max": 10}]"#
            ),
        );
        let translations = r#"[
            {"ids": ["fish_quantity_+%"], "English": [{"string": "{0}% increased Quantity of Fish Caught", "index_handlers": [[]]}], "hidden": null},
            {"ids": ["base_maximum_life"], "English": [{"string": "{0:+d} to maximum Life", "index_handlers": [[]]}], "hidden": null}
        ]"#;

        let (tables, tags) = tables(
            &modifiers,
            BASES,
            translations,
            &[("# to maximum Life", "# to maximum Life")],
        );

        let types: Vec<_> = tables
            .as_array()
            .expect("tables")
            .iter()
            .map(|table| (table["type"].clone(), table["spawn"].clone()))
            .collect();
        assert_eq!(
            types,
            [(
                serde_json::json!("Life"),
                serde_json::json!([["ring", 1], ["default", 0]])
            )]
        );
        // The tags of the tables only, of the released bases only, with their domain.
        let spawn = |domain: &str, tags: &[&str]| BaseSpawn {
            domain: domain.to_owned(),
            tags: tags.iter().map(|&tag| tag.to_owned()).collect(),
        };
        assert_eq!(tags.get("Ring"), Some(&spawn("item", &["ring", "default"])));
        assert_eq!(tags.get("Bow"), Some(&spawn("item", &["default"])));
        assert_eq!(tags.get("Flask"), Some(&spawn("flask", &["default"])));
        assert_eq!(tags.get("Old"), None);
    }

    #[test]
    fn a_name_of_several_bases_with_other_tags_gets_none() {
        let bases = r#"{
            "StrHelmet": {"domain": "item", "name": "Random Helmet", "release_state": "released", "tags": ["str_armour", "default"]},
            "IntHelmet": {"domain": "item", "name": "Random Helmet", "release_state": "released", "tags": ["int_armour", "default"]},
            "Cap": {"domain": "item", "name": "Cap", "release_state": "released", "tags": ["int_armour", "default"]}
        }"#;
        let modifiers = format!(
            r#"{{"Shield": {}}}"#,
            modifier(
                "EnergyShield",
                1,
                r#"[{"tag": "int_armour", "weight": 1}, {"tag": "default", "weight": 0}]"#,
                r#"[{"id": "local_energy_shield", "min": 5, "max": 10}]"#
            ),
        );
        let translations = r#"[{"ids": ["local_energy_shield"], "English": [{"string": "{0:+d} to maximum Energy Shield", "index_handlers": [[]]}], "hidden": null}]"#;

        let (_, tags) = tables(
            &modifiers,
            bases,
            translations,
            &[("# to maximum Energy Shield", "# to maximum Energy Shield")],
        );

        let tags_of = |name: &str| tags.get(name).map(|spawn| spawn.tags.clone());
        assert_eq!(tags_of("Random Helmet"), Some(Vec::new()));
        assert_eq!(
            tags_of("Cap"),
            Some(vec!["int_armour".to_owned(), "default".to_owned()])
        );
    }

    #[test]
    fn a_table_keeps_its_domain_when_it_isnt_that_of_the_equipment() {
        let charges = |domain: &str| {
            modifier(
                "Charges",
                1,
                r#"[{"tag": "default", "weight": 1}]"#,
                r#"[{"id": "charges_gained_+%", "min": 23, "max": 30}]"#,
            )
            .replace(r#""domain": "item""#, &format!(r#""domain": "{domain}""#))
        };
        let modifiers = format!(
            r#"{{"FlaskCharges": {}, "ItemCharges": {}}}"#,
            charges("flask"),
            charges("item"),
        );
        let translations = r#"[{"ids": ["charges_gained_+%"], "English": [{"string": "{0}% increased Charges gained", "index_handlers": [[]]}], "hidden": null}]"#;

        let (tables, _) = tables(
            &modifiers,
            BASES,
            translations,
            &[("#% increased Charges gained", "#% increased Charges gained")],
        );

        let domains: Vec<_> = tables
            .as_array()
            .expect("tables")
            .iter()
            .map(|table| table.get("domain").cloned())
            .collect();
        // One table per domain, the domain of the equipment unwritten.
        assert_eq!(domains, [Some(serde_json::json!("flask")), None]);
    }

    /// The lines and the tiers of the one table of a modifier of `stats`, written by
    /// `translation`, with `templates`.
    fn one_table(
        stats: &[&str],
        translation: &str,
        templates: &[(&str, &str)],
    ) -> serde_json::Value {
        let weapon = r#"[{"tag": "weapon", "weight": 1}, {"tag": "default", "weight": 0}]"#;
        let modifiers: Vec<String> = stats
            .iter()
            .enumerate()
            .map(|(index, stats)| {
                format!(
                    r#""Tier{index}": {}"#,
                    modifier(
                        "Kind",
                        10 * u32::try_from(index).expect("small"),
                        weapon,
                        stats
                    )
                )
            })
            .collect();
        let (tables, _) = tables(
            &format!("{{{}}}", modifiers.join(", ")),
            BASES,
            &format!("[{translation}]"),
            templates,
        );
        serde_json::json!([tables[0]["lines"].clone(), tables[0]["tiers"].clone()])
    }

    #[test]
    fn a_line_written_without_number_takes_its_values_from_the_string_that_writes_them() {
        let bolts = one_table(
            &[
                r#"[{"id": "base_number_of_crossbow_bolts", "min": 1, "max": 1}]"#,
                r#"[{"id": "base_number_of_crossbow_bolts", "min": 2, "max": 2}]"#,
            ],
            r#"{"ids": ["base_number_of_crossbow_bolts", "quality_display"], "English": [
                {"string": "Loads an additional bolt", "index_handlers": [[], []]},
                {"string": "Loads {0} additional bolts", "index_handlers": [[], []]}
            ], "hidden": null}"#,
            &[
                ("Loads an additional bolt", "Loads an additional bolt"),
                ("Loads # additional bolts", "Loads an additional bolt"),
            ],
        );

        assert_eq!(
            bolts,
            serde_json::json!([
                [{"template": "Loads an additional bolt"}],
                [{"level": 0, "ranges": [[[1, 1]]]}, {"level": 10, "ranges": [[[2, 2]]]}]
            ])
        );
    }

    #[test]
    fn a_line_shown_scaled_keeps_its_divisor_for_the_value_of_its_string_without_number() {
        let rage = one_table(
            &[r#"[{"id": "rage_loss_delay_ms_+", "min": 1000, "max": 2000}]"#],
            r#"{"ids": ["rage_loss_delay_ms_+"], "English": [
                {"string": "Inherent [Rage|Rage] loss starts 1 second later", "index_handlers": [[]]},
                {"string": "Inherent [Rage|Rage] loss starts {0} seconds later", "index_handlers": [["milliseconds_to_seconds"]]}
            ], "hidden": null}"#,
            &[(
                "Inherent Rage loss starts 1 second later",
                "Inherent Rage loss starts 1 second later",
            )],
        );

        assert_eq!(
            rage,
            serde_json::json!([
                [{"template": "Inherent Rage loss starts 1 second later", "divisor": 1000}],
                [{"level": 0, "ranges": [[[1, 2]]]}]
            ])
        );
    }

    #[test]
    fn a_line_without_number_in_every_string_has_the_values_of_its_stat() {
        let radius = one_table(
            &[r#"[{"id": "local_jewel_display_radius_change", "min": 1, "max": 1}]"#],
            r#"{"ids": ["local_jewel_display_radius_change"], "English": [
                {"string": "Upgrades Radius to Medium", "index_handlers": [[]]},
                {"string": "Upgrades Radius to Large", "index_handlers": [[]]}
            ], "hidden": null}"#,
            // Found by a string of Exiled Exchange 2, not by its template.
            &[("Upgrades Radius to Medium", "Upgrades Radius to #")],
        );

        assert_eq!(
            radius,
            serde_json::json!([
                [{"template": "Upgrades Radius to #"}],
                [{"level": 0, "ranges": [[[1, 1]]]}]
            ])
        );
    }

    #[test]
    fn a_number_of_a_stat_the_modifier_doesnt_have_is_zero() {
        let added = one_table(
            &[r#"[{"id": "local_maximum_added_fire_damage", "min": 10, "max": 15}]"#],
            r#"{"ids": ["local_minimum_added_fire_damage", "local_maximum_added_fire_damage"], "English": [
                {"string": "Adds {0} to {1} [Fire|Fire] Damage", "index_handlers": [[], []]}
            ], "hidden": null}"#,
            &[("Adds # to # Fire Damage", "Adds # to # Fire Damage")],
        );

        assert_eq!(
            added,
            serde_json::json!([
                [{"template": "Adds # to # Fire Damage"}],
                [{"level": 0, "ranges": [[[0, 0], [10, 15]]]}]
            ])
        );
    }

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
        // A brace or a bracket not closed is written as it is.
        assert_eq!(template("abc {0"), "abc {0");
        assert_eq!(template("abc [Fire"), "abc [Fire");
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

    #[test]
    fn a_line_without_value_has_no_range() {
        let instant = one_table(
            &[r#"[{"id": "local_flask_recovers_instantly", "min": 1, "max": 1}]"#],
            r#"{"ids": ["local_flask_recovers_instantly"], "English": [
                {"string": "Instant Recovery", "index_handlers": [[]]}
            ], "hidden": null}"#,
            &[("Instant Recovery", "Instant Recovery")],
        );

        assert_eq!(
            instant,
            serde_json::json!([
                [{"template": "Instant Recovery"}],
                [{"level": 0, "ranges": [[]]}]
            ])
        );
    }

    #[test]
    fn only_the_prefixes_and_suffixes_that_roll_are_kept_without_their_hidden_stats() {
        let life = r#"[{"id": "base_maximum_life", "min": 5, "max": 10}, {"id": "hidden_stat", "min": 1, "max": 1}]"#;
        let ring = r#"[{"tag": "ring", "weight": 1}, {"tag": "default", "weight": 0}]"#;
        let kept = modifier("Life", 1, ring, life);
        let other = |field: &str, value: &str| {
            let text = kept.replace(field, value);
            assert_ne!(text, kept);
            text
        };
        let modifiers = format!(
            r#"{{"Kept": {kept}, "Monster": {}, "Unique": {}, "Essence": {}, "Nowhere": {}, "TimeLost": {}}}"#,
            other(r#""domain": "item""#, r#""domain": "monster""#),
            other(
                r#""generation_type": "suffix""#,
                r#""generation_type": "unique""#
            ),
            other(r#""is_essence_only": false"#, r#""is_essence_only": true"#),
            other(
                r#"{"tag": "ring", "weight": 1}"#,
                r#"{"tag": "ring", "weight": 0}"#
            ),
            other(
                r#"{"tag": "ring", "weight": 1}"#,
                r#"{"tag": "str_radius_jewel", "weight": 1}"#
            ),
        );
        let translations = r#"[
            {"ids": ["base_maximum_life"], "English": [{"string": "{0:+d} to maximum Life", "index_handlers": [[]]}], "hidden": null},
            {"ids": ["hidden_stat"], "English": [{"string": "Hidden {0}", "index_handlers": [[]]}], "hidden": true}
        ]"#;

        let (tables, _) = tables(
            &modifiers,
            BASES,
            translations,
            &[("# to maximum Life", "# to maximum Life")],
        );

        assert_eq!(
            tables,
            serde_json::json!([
                {"type": "Life", "affix": "suffix", "lines": [{"template": "# to maximum Life"}],
                 "spawn": [["ring", 1], ["default", 0]],
                 "tiers": [{"level": 1, "ranges": [[[5, 10]]]}]}
            ])
        );
    }

    #[test]
    fn a_stat_that_rolls_without_being_written_stops_the_generator() {
        let modifiers = format!(
            r#"{{"Bolts": {}}}"#,
            modifier(
                "Bolts",
                1,
                r#"[{"tag": "weapon", "weight": 1}]"#,
                r#"[{"id": "base_number_of_crossbow_bolts", "min": 1, "max": 2}]"#
            )
        );
        let translations = r#"[{"ids": ["base_number_of_crossbow_bolts"], "English": [
            {"string": "Loads an additional bolt", "index_handlers": [[]]}
        ], "hidden": null}]"#;

        let error = reduce(
            &serde_json::from_str(&modifiers).expect("modifiers"),
            &serde_json::from_str(BASES).expect("bases"),
            &serde_json::from_str::<Vec<_>>(translations).expect("translations"),
            &[(
                "Loads an additional bolt".to_owned(),
                "Loads an additional bolt".to_owned(),
            )]
            .into(),
        )
        .err()
        .map(|error| error.to_string());

        assert_eq!(
            error.as_deref(),
            Some(
                "Bolts: base_number_of_crossbow_bolts rolls but Loads an additional bolt doesn't write it"
            )
        );
    }

    #[test]
    fn a_modifier_with_a_line_without_template_is_left_out() {
        let modifiers = format!(
            r#"{{"Hook": {}}}"#,
            modifier(
                "Hook",
                1,
                r#"[{"tag": "weapon", "weight": 1}]"#,
                r#"[{"id": "fishing_hook_type", "min": 1, "max": 2}]"#
            )
        );
        let translations = r#"[{"ids": ["fishing_hook_type"], "English": [
            {"string": "Karui Stone Hook", "index_handlers": [[]]}
        ], "hidden": null}]"#;

        let (tables, _) = tables(&modifiers, BASES, translations, &[]);

        assert_eq!(tables, serde_json::json!([]));
    }

    #[test]
    fn a_line_of_several_values_with_a_string_without_number_stops_the_generator() {
        let modifiers = format!(
            r#"{{"Thorns": {}}}"#,
            modifier(
                "Thorns",
                1,
                r#"[{"tag": "weapon", "weight": 1}]"#,
                r#"[{"id": "minimum_thorns", "min": 1, "max": 2}, {"id": "maximum_thorns", "min": 3, "max": 4}]"#
            )
        );
        let translations = r#"[{"ids": ["minimum_thorns", "maximum_thorns"], "English": [
            {"string": "Thorns", "index_handlers": [[], []]},
            {"string": "{0} to {1} Thorns damage", "index_handlers": [[], []]}
        ], "hidden": null}]"#;

        let error = reduce(
            &serde_json::from_str(&modifiers).expect("modifiers"),
            &serde_json::from_str(BASES).expect("bases"),
            &serde_json::from_str::<Vec<_>>(translations).expect("translations"),
            &[("Thorns".to_owned(), "# to # Thorns damage".to_owned())].into(),
        )
        .err()
        .map(|error| error.to_string());

        assert_eq!(
            error.as_deref(),
            Some("Thorns: {0} to {1} Thorns damage has several values and a string without number")
        );
    }
}
