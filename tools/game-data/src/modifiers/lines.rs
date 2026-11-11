//! The lines of a modifier, written as the English templates of Exiled Exchange 2 from the
//! stat translations of RePoE, and the range of each of their values.

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};

use super::{DESECRATED, SourceModifier, StatRange};
use crate::Result;

/// The stats of the waystones that roll while their line writes no value: the game writes
/// `Monsters are Armoured` alone, whatever share of their strength RePoE adds to their armour
/// (35–55%).
const UNWRITTEN_ROLLS: [&str; 2] = [
    "map_monsters_additional_strength_ratio_%_for_armour",
    "map_monsters_additional_dexterity_ratio_%_for_evasion",
];

/// Whether `tag` is one of the Time-Lost jewels, whose modifiers the game writes within the
/// words of [`RADIUS_GRANTS`].
pub(super) fn is_radius_jewel_tag(tag: &str) -> bool {
    tag == "radius_jewel" || tag.ends_with("_radius_jewel")
}

/// The words the game writes before a line the small passive skills in the radius of a
/// Time-Lost jewel grant.
const SMALL_GRANTS: &str = "Small Passive Skills in Radius also grant ";

/// The words the game writes before a line the notables in the radius of a Time-Lost jewel
/// grant.
const NOTABLE_GRANTS: &str = "Notable Passive Skills in Radius also grant ";

/// The words the game writes before a line of a Time-Lost jewel, which RePoE doesn't write (its
/// modifiers have the stats of the line alone, `elemental_damage_+%`).
const RADIUS_GRANTS: [&str; 2] = [SMALL_GRANTS, NOTABLE_GRANTS];

/// The lines of a Time-Lost jewel about the radius itself, written alone, without the words of
/// [`RADIUS_GRANTS`], as templates of Exiled Exchange 2.
const RADIUS_LINES: [&str; 3] = [
    "#% increased Effect of Small Passive Skills in Radius",
    "#% increased Effect of Notable Passive Skills in Radius",
    "Upgrades Radius to #",
];

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

/// A line of a modifier.
#[derive(Serialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Line {
    /// Its English template, as Exiled Exchange 2 writes it (`Adds # to # Lightning Damage`).
    pub(super) template: String,
    /// What the value of the stat is divided by to be shown, e.g. 1000 for milliseconds shown
    /// in seconds: the value of a line written without number (`Inherent Rage loss starts 1
    /// second later`) is that of the stat, 1000.
    #[serde(skip_serializing_if = "is_one")]
    divisor: i64,
    /// Whether the template is one of Exiled Exchange 2: a line without template can't be
    /// found, nor its modifier.
    #[serde(skip)]
    pub(super) known: bool,
    /// The string of the game whose template is known without the `+` of its number
    /// ([`known_template`]), printed by the generator; the same for every tier of a
    /// translation, so it doesn't split the tiers of a table.
    #[serde(skip)]
    pub(super) unsigned: Option<String>,
}

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "the signature serde asks for"
)]
fn is_one(value: &i64) -> bool {
    *value == 1
}

/// The range of each value of a line.
pub(super) type LineRanges = Vec<[ShownValue; 2]>;

/// A value of a stat as the game shows it, written as an integer when it is one.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ShownValue {
    /// The number times [`ShownValue::SCALE`], exact for the handlers of the game.
    scaled: i64,
}

impl ShownValue {
    const SCALE: i64 = 60_000;
}

impl Serialize for ShownValue {
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

/// The English template of Exiled Exchange 2 (its `ref`) of each of its English strings, and of
/// each template itself: `Loads # additional bolts` is `Loads an additional bolt`.
pub type Templates = BTreeMap<String, String>;

/// The translation of each stat, the first of `files` that writes it, the first of its file.
pub(super) fn by_stat<'a>(files: &[&'a [Translation]]) -> HashMap<&'a str, &'a Translation> {
    let mut by_stat = HashMap::new();
    for translation in files.iter().copied().flatten() {
        for id in &translation.ids {
            by_stat.entry(id.as_str()).or_insert(translation);
        }
    }
    by_stat
}

/// The lines of `modifier`, each with the English template of Exiled Exchange 2 of one of its
/// strings, and the range of each of their values. A line without template is marked unknown
/// (written as the game writes it, without range): [`reduce`] leaves its modifier out.
pub(super) fn lines(
    modifier: &SourceModifier,
    by_stat: &HashMap<&str, &Translation>,
    templates: &Templates,
) -> Result<(Vec<Line>, Vec<LineRanges>)> {
    let mut lines = Vec::new();
    let mut ranges = Vec::new();
    // The translations written, once each: one can write several stats.
    let mut written: Vec<&Translation> = Vec::new();
    for stat in &modifier.stats {
        let Some(&translation) = by_stat.get(stat.id.as_str()) else {
            // A stat the game doesn't write, e.g. `dummy_stat_display_nothing`; one that rolls
            // is written by a translation RePoE doesn't have (`minion_area_of_effect_+%` of a
            // hybrid of the Abyss on the jewels), whose line can't be found.
            if stat.rolls() {
                lines.push(Line {
                    template: stat.id.clone(),
                    divisor: 1,
                    known: false,
                    unsigned: None,
                });
                ranges.push(Vec::new());
            }
            continue;
        };
        // The game writes no line of values all 0: the `0% increased Quantity of Waystones
        // found in Map` RePoE gives the additional Essence of a tablet.
        let zero = translation
            .ids
            .iter()
            .filter_map(|id| modifier.stat(id))
            .all(StatRange::is_zero);
        if translation.hidden == Some(true)
            || zero
            || written
                .iter()
                .any(|&before| std::ptr::eq(before, translation))
        {
            continue;
        }
        written.push(translation);
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
    // The values are those of the string that writes the most of them: `Loads {0} additional
    // bolts`, not `Loads an additional bolt` (`max_by_key` keeps the last of the maximums: the
    // variants reversed, the first one).
    let general = translation
        .english
        .iter()
        .rev()
        .max_by_key(|variant| placeholders(&variant.string).len())
        .unwrap_or(first);
    let strings: Vec<String> = translation
        .english
        .iter()
        .map(|variant| template(&variant.string))
        .collect();
    let Some(Known { template, unsigned }) = known_template(modifier, &strings, templates)? else {
        // Without template, the line can't be found: written as the game writes it, its string
        // with the most numbers; one of the Time-Lost jewels, which the game writes within the
        // words of the passive skills that grant it, saying so.
        let jewel = if modifier.rolls_on_radius_jewel() {
            " (Time-Lost jewels: not known within the words of the passive skills, with its number)"
        } else {
            ""
        };
        let line = Line {
            template: format!("{}{jewel}", template(&general.string)),
            divisor: 1,
            known: false,
            unsigned: None,
        };
        return Ok((line, Vec::new()));
    };
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
                    .is_some_and(|id| modifier.stat(id).is_some())
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
        let rolls = modifier.stat(id).is_some_and(StatRange::rolls);
        if rolls && !indexes.contains(&index) && !UNWRITTEN_ROLLS.contains(&id.as_str()) {
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
        let (min, max) = modifier
            .stat(id)
            .map_or((0, 0), |stat| (stat.min, stat.max));
        ranges.push([
            shown_value(min, handlers(index))?,
            shown_value(max, handlers(index))?,
        ]);
    }
    // A string without number of Exiled Exchange 2 gives the value of the stat, e.g. 1000
    // milliseconds for `Inherent Rage loss starts 1 second later`, which the application
    // divides to compare it with the ranges as the game shows them; a line without such a
    // string needs no divisor.
    let without_number: Vec<String> = translation
        .english
        .iter()
        .filter(|variant| placeholders(&variant.string).is_empty())
        .map(|variant| self::template(&variant.string))
        .collect();
    let written = known_template(modifier, &without_number, templates)?
        .is_some_and(|known| known.template == template);
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
        template: template.clone(),
        divisor,
        known: true,
        unsigned,
    };
    Ok((line, ranges))
}

/// The template of Exiled Exchange 2 of a line of `modifier` written one of `strings` (templates of
/// the game, [`template`]), the first it knows, else the first it knows without the `+` of its
/// numbers, which the application reads in their values (`+#% Monster Elemental Resistances` of the
/// waystones, `#% Monster Elemental Resistances`). On a Time-Lost jewel, the line within the words
/// of [`RADIUS_GRANTS`] of which it knows one kind only (`Small Passive Skills in Radius also grant
/// #% increased Elemental Damage`), with its number when a string writes one: known without its
/// number only (`Small Passive Skills in Radius also grant Attack Hits apply Incision`, while the
/// game writes `… 4% chance for Attack Hits to apply Incision`, seen on the trade site on
/// 2026-10-30), the line can't be found, and has none. A line about the radius itself
/// ([`RADIUS_LINES`]) is written alone; another line known within neither has none. Of a desecrated
/// modifier, which it knows within both (`#% increased maximum Life`), the notables: the trade site
/// has a desecrated stat for them only, as for every other desecrated modifier of these jewels
/// (checked on 2026-10-30).
fn known_template<'a>(
    modifier: &SourceModifier,
    strings: &[String],
    templates: &'a Templates,
) -> Result<Option<Known<'a>>> {
    let alone = || {
        strings
            .iter()
            .find_map(|string| templates.get(string))
            .map(Known::exact)
            .or_else(|| {
                strings.iter().find_map(|string| {
                    let template = templates.get(&string.replace("+#", "#"))?;
                    Some(Known {
                        template,
                        unsigned: Some(string.clone()),
                    })
                })
            })
    };
    if !modifier.rolls_on_radius_jewel() {
        return Ok(alone());
    }
    let within = |words: &str, string: &String| templates.get(&format!("{words}{string}"));
    let granted: Vec<&str> = RADIUS_GRANTS
        .into_iter()
        .filter(|words| strings.iter().any(|string| within(words, string).is_some()))
        .collect();
    let words = match granted.as_slice() {
        [] => {
            return Ok(alone().filter(|known| RADIUS_LINES.contains(&known.template.as_str())));
        }
        &[words] => words,
        [_, _] if modifier.domain == DESECRATED => NOTABLE_GRANTS,
        _ => {
            // Named by its string with a number, as the lines left out are printed.
            let named = strings
                .iter()
                .find(|string| string.contains('#'))
                .or(strings.first());
            return Err(format!(
                "{} is known within the words of both kinds of passive skills",
                named.map_or("", String::as_str)
            )
            .into());
        }
    };
    let numbered = strings.iter().any(|string| string.contains('#'));
    Ok(strings
        .iter()
        .filter(|string| !numbered || string.contains('#'))
        .find_map(|string| within(words, string))
        .map(Known::exact))
}

/// The template of Exiled Exchange 2 [`known_template`] finds, and the string of the game it
/// knows without the `+` of its number, when it is so.
struct Known<'a> {
    template: &'a String,
    unsigned: Option<String>,
}

impl<'a> Known<'a> {
    /// A template known as the game writes the line.
    fn exact(template: &'a String) -> Self {
        Self {
            template,
            unsigned: None,
        }
    }
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
fn shown_value(value: i64, handlers: &[String]) -> Result<ShownValue> {
    let divisor = divisor_of(handlers)?;
    let scaled = value * ShownValue::SCALE;
    if scaled % divisor != 0 {
        return Err(format!("{value} can't be divided by {divisor}").into());
    }
    Ok(ShownValue {
        scaled: scaled / divisor,
    })
}

#[cfg(test)]
mod tests;
