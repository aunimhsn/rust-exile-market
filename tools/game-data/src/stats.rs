//! The stats of Exiled Exchange 2 (`renderer/public/data/<language>/stats.ndjson`), reduced to
//! the fields and the groups of the trade site the application reads.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::Result;
use crate::modifiers;
use crate::report::Report;

/// The groups of the trade site that the application can search.
const GROUPS: [&str; 9] = [
    "explicit",
    "implicit",
    "fractured",
    "desecrated",
    "crafted",
    "enchant",
    "rune",
    "pseudo",
    "skill",
];
/// The groups left out: the relics of the Sanctum aren't priced by the application.
const LEFT_OUT_GROUPS: [&str; 1] = ["sanctum"];

/// A stat of `renderer/public/data/<language>/stats.ndjson`, with the fields the application
/// reads; the others (`oils`, `dp`, `fromAreaMods`) are left out.
#[derive(Deserialize)]
struct SourceStat {
    #[serde(rename = "ref")]
    reference: String,
    id: Option<String>,
    better: i8,
    matchers: Vec<SourceMatcher>,
    trade: SourceTrade,
}

#[derive(Deserialize)]
struct SourceMatcher {
    string: String,
    value: Option<serde_json::Number>,
    #[serde(default)]
    negate: bool,
    /// The string of the advanced copy, when it differs.
    advanced: Option<String>,
}

#[derive(Deserialize)]
struct SourceTrade {
    ids: Option<BTreeMap<String, Vec<String>>>,
    #[serde(default)]
    option: bool,
    #[serde(default)]
    inverted: bool,
}

/// A stat as the application reads it (`src/game_data/stats.rs`).
#[derive(Serialize)]
pub(crate) struct Stat {
    #[serde(rename = "ref")]
    pub(crate) reference: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    better: i8,
    matchers: Vec<Matcher>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) ids: BTreeMap<String, Vec<String>>,
    #[serde(skip_serializing_if = "is_false")]
    option: bool,
    #[serde(skip_serializing_if = "is_false")]
    inverted: bool,
}

#[derive(Serialize, PartialEq)]
struct Matcher {
    string: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<serde_json::Number>,
    #[serde(skip_serializing_if = "is_false")]
    negate: bool,
}

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "the signature serde asks for"
)]
fn is_false(value: &bool) -> bool {
    !value
}

/// The English templates of `stats`, the English stats of Exiled Exchange 2, by each of their
/// strings.
pub(crate) fn templates(stats: &[Stat]) -> modifiers::Templates {
    let mut templates = modifiers::Templates::new();
    for stat in stats {
        templates.insert(stat.reference.clone(), stat.reference.clone());
    }
    // A template is its own first: a string of another stat can be written the same.
    for stat in stats {
        for matcher in &stat.matchers {
            templates
                .entry(matcher.string.clone())
                .or_insert_with(|| stat.reference.clone());
        }
    }
    templates
}

/// The stats of a `stats.ndjson`, with only the fields and the groups the application reads.
pub(crate) fn reduce(source: &str) -> Result<Vec<Stat>> {
    let mut stats = Vec::new();
    for (index, line) in source.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let stat: SourceStat =
            serde_json::from_str(line).map_err(|error| format!("line {}: {error}", index + 1))?;
        stats.push(reduce_stat(stat)?);
    }
    Ok(stats)
}

fn reduce_stat(stat: SourceStat) -> Result<Stat> {
    let mut ids = BTreeMap::new();
    for (group, group_ids) in stat.trade.ids.unwrap_or_default() {
        if LEFT_OUT_GROUPS.contains(&group.as_str()) {
            continue;
        }
        // A new group of the trade site is decided before it is embedded.
        if !GROUPS.contains(&group.as_str()) {
            return Err(format!("unknown group {group} in {}", stat.reference).into());
        }
        ids.insert(group, group_ids);
    }
    let mut matchers = Vec::new();
    for matcher in stat.matchers {
        let strings = std::iter::once(matcher.string).chain(matcher.advanced);
        for string in strings {
            let reduced = Matcher {
                string,
                value: matcher.value.clone(),
                negate: matcher.negate,
            };
            if !matchers.contains(&reduced) {
                matchers.push(reduced);
            }
        }
    }
    Ok(Stat {
        reference: stat.reference,
        id: stat.id,
        better: stat.better,
        matchers,
        ids,
        option: stat.trade.option,
        inverted: stat.trade.inverted,
    })
}

/// Notes the stats of one language only, and those whose trade ids differ between the
/// languages, to look at after a patch: the application reads each language on its own.
pub(crate) fn report_differences(
    english: &BTreeMap<String, BTreeMap<String, Vec<String>>>,
    french: &BTreeMap<String, BTreeMap<String, Vec<String>>>,
    report: &mut Report,
) {
    let references: BTreeSet<_> = english.keys().chain(french.keys()).collect();
    for reference in references {
        match (english.get(reference), french.get(reference)) {
            (Some(_), None) => report.note("Stats in English only", reference),
            (None, Some(_)) => report.note("Stats in French only", reference),
            (Some(english), Some(french)) if english != french => {
                report.note("Stats with other trade ids in French", reference);
            }
            _ => {}
        }
    }
}
