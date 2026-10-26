//! Generates the game data embedded in the application, in `assets/game-data/`, from the
//! stats of Exiled Exchange 2 (see `docs/game-data.md`). Run it again after each patch of
//! the game:
//!
//! - `cargo run -p game-data` reads the latest commit of Exiled Exchange 2;
//! - `cargo run -p game-data -- <commit>` reads a given one.
//!
//! The files it writes have one stat per line, so that the changes of a patch read well in a
//! diff.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const REPOSITORY: &str = "Kvan7/Exiled-Exchange-2";
const LANGUAGES: [&str; 2] = ["en", "fr"];
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

type Result<T> = std::result::Result<T, Box<dyn Error>>;

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
struct Stat {
    #[serde(rename = "ref")]
    reference: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    better: i8,
    matchers: Vec<Matcher>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    ids: BTreeMap<String, Vec<String>>,
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

#[derive(Deserialize)]
struct Commit {
    sha: String,
    commit: CommitDetail,
}

#[derive(Deserialize)]
struct CommitDetail {
    committer: Committer,
}

#[derive(Deserialize)]
struct Committer {
    date: String,
}

fn main() -> Result<()> {
    let reference = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "master".to_owned());
    let agent = ureq::Agent::new_with_defaults();
    let commit: Commit = serde_json::from_str(&get(
        &agent,
        &format!("https://api.github.com/repos/{REPOSITORY}/commits/{reference}"),
    )?)?;
    println!("Exiled Exchange 2, commit {}", commit.sha);

    let output = output_directory();
    fs::create_dir_all(&output)?;
    let mut ids_by_language = Vec::new();
    for language in LANGUAGES {
        let source = get(
            &agent,
            &raw_url(
                &commit.sha,
                &format!("renderer/public/data/{language}/stats.ndjson"),
            ),
        )?;
        let stats = reduce(&source).map_err(|error| format!("{language}: {error}"))?;
        println!("{language}: {} stats", stats.len());
        ids_by_language.push(
            stats
                .iter()
                .map(|stat| (stat.reference.clone(), stat.ids.clone()))
                .collect::<BTreeMap<_, _>>(),
        );
        write_lines(&output.join(format!("stats-{language}.ndjson")), &stats)?;
    }
    if let [english, french] = ids_by_language.as_slice() {
        report_differences(english, french);
    }

    let license = get(&agent, &raw_url(&commit.sha, "LICENSE"))?;
    fs::write(
        output.join("NOTICE.md"),
        notice(&commit.sha, &commit.commit.committer.date, &license),
    )?;
    println!("Written to {}", output.canonicalize()?.display());
    Ok(())
}

/// `assets/game-data/` of the application.
fn output_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/game-data")
}

fn raw_url(commit: &str, path: &str) -> String {
    format!("https://raw.githubusercontent.com/{REPOSITORY}/{commit}/{path}")
}

fn get(agent: &ureq::Agent, url: &str) -> Result<String> {
    Ok(agent
        .get(url)
        .header(
            "User-Agent",
            "rust-exile-market game data generator (+https://github.com/aunimhsn/rust-exile-market)",
        )
        .call()?
        .body_mut()
        .with_config()
        .limit(64 * 1024 * 1024)
        .read_to_string()?)
}

/// The stats of a `stats.ndjson`, with only the fields and the groups the application reads.
fn reduce(source: &str) -> Result<Vec<Stat>> {
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

/// Writes one stat per line.
fn write_lines(path: &Path, stats: &[Stat]) -> Result<()> {
    let mut text = String::new();
    for stat in stats {
        text.push_str(&serde_json::to_string(stat)?);
        text.push('\n');
    }
    fs::write(path, text)?;
    Ok(())
}

/// Prints the stats of one language only, and those whose trade ids differ between the
/// languages, to look at after a patch: the application reads each language on its own.
fn report_differences(
    english: &BTreeMap<String, BTreeMap<String, Vec<String>>>,
    french: &BTreeMap<String, BTreeMap<String, Vec<String>>>,
) {
    let references: BTreeSet<_> = english.keys().chain(french.keys()).collect();
    for reference in references {
        match (english.get(reference), french.get(reference)) {
            (Some(_), None) => println!("English only: {reference}"),
            (None, Some(_)) => println!("French only: {reference}"),
            (Some(english), Some(french)) if english != french => {
                println!("Other trade ids in French: {reference}");
            }
            _ => {}
        }
    }
}

fn notice(commit: &str, date: &str, license: &str) -> String {
    format!(
        "# Game data notice

Generated by `tools/game-data` (`cargo run -p game-data`, see `docs/game-data.md`): don't edit these files by hand, generate them again.

`stats-en.ndjson` and `stats-fr.ndjson` are the stats of [Exiled Exchange 2](https://github.com/{REPOSITORY}) (`renderer/public/data/<language>/stats.ndjson`), commit `{commit}` of {date}, reduced to the fields the application reads and without the groups of the Sanctum.

Their texts and ids come from the files of Path of Exile 2 and belong to Grinding Gear Games; they are used here in a free, non-commercial tool. The files of Exiled Exchange 2 are under the MIT license, whose notice follows:

```text
{}
```

This product isn't affiliated with or endorsed by Grinding Gear Games in any way.
",
        license.trim_end()
    )
}
