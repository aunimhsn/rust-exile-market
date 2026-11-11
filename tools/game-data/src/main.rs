//! Generates the game data embedded in the application, in `assets/game-data/`, from the
//! stats and the items of Exiled Exchange 2 and the modifiers of the RePoE fork (see
//! `docs/game-data.md`). Run it again after each patch of the game:
//!
//! - `cargo run -p game-data` reads the latest commit of each;
//! - `cargo run -p game-data -- <commit of Exiled Exchange 2> [<commit of the data of RePoE>
//!   [<commit of the code of RePoE>]]` reads given ones, the code of RePoE for its license.
//!
//! The files it writes have one stat, one base or one modifier per line, so that the changes
//! of a patch read well in a diff. What it leaves out or finds odd goes to `report.md` next to
//! them, to compare from a patch to the next.

mod bases;
mod github;
mod modifiers;
mod notice;
mod report;
mod stats;

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use github::{Commit, GitHub};
use report::Report;

/// The stats and the items of Exiled Exchange 2, in `renderer/public/data/<language>/`.
const EXILED_EXCHANGE: &str = "Kvan7/Exiled-Exchange-2";
/// The data of Path of Exile 2 exported by the RePoE fork, in `data/`.
const REPOE_DATA: &str = "repoe-fork/poe2";
/// The code of the RePoE fork, with its license.
const REPOE_CODE: &str = "repoe-fork/repoe";
/// The languages of the stats and the bases, English first.
const LANGUAGES: [&str; 2] = ["en", "fr"];

pub(crate) type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// What the generator reads of each repository, from its arguments: a branch or a commit,
/// `master` when it isn't given.
struct References {
    exiled_exchange: String,
    repoe_data: String,
    repoe_code: String,
}

impl References {
    fn from_arguments() -> Self {
        let mut arguments = std::env::args().skip(1);
        let mut next = || arguments.next().unwrap_or_else(|| "master".to_owned());
        Self {
            exiled_exchange: next(),
            repoe_data: next(),
            repoe_code: next(),
        }
    }
}

/// The commits the generated data are read from.
struct Commits {
    exiled_exchange: Commit,
    repoe_data: Commit,
    /// The version of the game the data of RePoE were exported from.
    repoe_version: String,
}

impl Commits {
    /// The commits of `references`, printed.
    fn read(github: &GitHub, references: &References) -> Result<Self> {
        let exiled_exchange = github.commit(EXILED_EXCHANGE, &references.exiled_exchange)?;
        println!("Exiled Exchange 2, commit {}", exiled_exchange.sha);
        let repoe_data = github.commit(REPOE_DATA, &references.repoe_data)?;
        let repoe_version = github
            .file(REPOE_DATA, &repoe_data.sha, "exported-version.txt")?
            .trim()
            .to_owned();
        println!("RePoE, commit {}, version {repoe_version}", repoe_data.sha);
        Ok(Self {
            exiled_exchange,
            repoe_data,
            repoe_version,
        })
    }

    /// The file `name` of Exiled Exchange 2 in `language`.
    fn exiled_exchange_file(&self, github: &GitHub, language: &str, name: &str) -> Result<String> {
        github.file(
            EXILED_EXCHANGE,
            &self.exiled_exchange.sha,
            &format!("renderer/public/data/{language}/{name}"),
        )
    }
}

fn main() -> Result<()> {
    let references = References::from_arguments();
    let github = GitHub::new();
    let commits = Commits::read(&github, &references)?;
    let output = output_directory();
    fs::create_dir_all(&output)?;
    let mut report = Report::default();

    let stats_by_language = LANGUAGES
        .into_iter()
        .map(|language| {
            let source = commits.exiled_exchange_file(&github, language, "stats.ndjson")?;
            let reduced = stats::reduce(&source).map_err(|error| format!("{language}: {error}"))?;
            Ok((language, reduced))
        })
        .collect::<Result<Vec<_>>>()?;
    let [(_, english), (_, french)] = stats_by_language.as_slice() else {
        return Err("the stats of other languages than English and French".into());
    };
    let (tables, base_spawns) = modifier_tables(
        &github,
        &commits.repoe_data.sha,
        &stats::templates(english),
        &mut report,
    )?;
    println!("{} modifier tables", tables.len());
    write_ndjson(&output.join("modifiers.ndjson"), &tables)?;

    for (language, stats) in &stats_by_language {
        println!("{language}: {} stats", stats.len());
        write_ndjson(&output.join(format!("stats-{language}.ndjson")), stats)?;
        let source = commits.exiled_exchange_file(&github, language, "items.ndjson")?;
        let bases = bases::reduce(&source, &base_spawns, language, &mut report)
            .map_err(|error| format!("{language}: {error}"))?;
        println!("{language}: {} bases", bases.len());
        write_ndjson(&output.join(format!("bases-{language}.ndjson")), &bases)?;
    }
    stats::report_differences(english, french, &mut report);

    notice::write(&github, &output, &commits, &references.repoe_code)?;
    let read_from = format!(
        "the commit {} of Exiled Exchange 2 and the commit {} of the data of RePoE",
        commits.exiled_exchange.sha, commits.repoe_data.sha
    );
    fs::write(output.join("report.md"), report.to_markdown(&read_from))?;
    println!("Written to {}", output.canonicalize()?.display());
    Ok(())
}

/// The modifier tables of the commit `commit` of the data of RePoE, and the domain and the tags
/// of the bases.
fn modifier_tables(
    github: &GitHub,
    commit: &str,
    templates: &modifiers::Templates,
    report: &mut Report,
) -> Result<(Vec<modifiers::Table>, modifiers::BaseSpawns)> {
    let file = |path: &str| github.file(REPOE_DATA, commit, &format!("data/{path}"));
    let files = modifiers::RepoeFiles {
        modifiers: serde_json::from_str(&file("mods.json")?)?,
        bases: serde_json::from_str(&file("base_items.json")?)?,
        translations: serde_json::from_str(&file("stat_translations/stat_descriptions.json")?)?,
        tablet_translations: serde_json::from_str(&file(
            "stat_translations/tablet_stat_descriptions.json",
        )?)?,
    };
    modifiers::reduce(&files, templates, modifiers::CURRENCIES, report)
}

/// `assets/game-data/` of the application.
fn output_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/game-data")
}

/// Writes `records` in `path`, one per line: a stat, a base or a table.
fn write_ndjson<T: Serialize>(path: &Path, records: &[T]) -> Result<()> {
    let mut text = String::new();
    for record in records {
        text.push_str(&serde_json::to_string(record)?);
        text.push('\n');
    }
    fs::write(path, text)?;
    Ok(())
}
