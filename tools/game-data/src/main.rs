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

use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use github::{get, latest_commit, raw_url};
use report::Report;

const EXILED_EXCHANGE: &str = "Kvan7/Exiled-Exchange-2";
/// The data of Path of Exile 2 exported by the RePoE fork, in `data/`.
const REPOE_DATA: &str = "repoe-fork/poe2";
/// The code of the RePoE fork, with its license.
const REPOE_CODE: &str = "repoe-fork/repoe";
const LANGUAGES: [&str; 2] = ["en", "fr"];

pub(crate) type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn main() -> Result<()> {
    let mut arguments = std::env::args().skip(1);
    let reference = arguments.next().unwrap_or_else(|| "master".to_owned());
    let repoe_reference = arguments.next().unwrap_or_else(|| "master".to_owned());
    let repoe_code_reference = arguments.next().unwrap_or_else(|| "master".to_owned());
    let agent = ureq::Agent::new_with_defaults();
    let commit = latest_commit(&agent, EXILED_EXCHANGE, &reference)?;
    println!("Exiled Exchange 2, commit {}", commit.sha);
    let repoe_commit = latest_commit(&agent, REPOE_DATA, &repoe_reference)?;
    let repoe_version = get(
        &agent,
        &raw_url(REPOE_DATA, &repoe_commit.sha, "exported-version.txt"),
    )?;
    println!(
        "RePoE, commit {}, version {}",
        repoe_commit.sha,
        repoe_version.trim()
    );

    let output = output_directory();
    fs::create_dir_all(&output)?;
    let mut report = Report::default();
    let mut sources = Vec::new();
    for language in LANGUAGES {
        let source = get(
            &agent,
            &raw_url(
                EXILED_EXCHANGE,
                &commit.sha,
                &format!("renderer/public/data/{language}/stats.ndjson"),
            ),
        )?;
        sources.push((
            language,
            stats::reduce(&source).map_err(|error| format!("{language}: {error}"))?,
        ));
    }
    let english = sources
        .iter()
        .find(|(language, _)| *language == "en")
        .map(|(_, stats)| stats.as_slice())
        .ok_or("no English stats")?;
    let (tables, base_spawns) = modifier_tables(
        &agent,
        &repoe_commit.sha,
        &stats::templates(english),
        &mut report,
    )?;
    println!("{} modifier tables", tables.len());
    write_lines(&output.join("modifiers.ndjson"), &tables)?;

    let mut ids_by_language = Vec::new();
    for (language, stats) in sources {
        println!("{language}: {} stats", stats.len());
        ids_by_language.push(
            stats
                .iter()
                .map(|stat| (stat.reference.clone(), stat.ids.clone()))
                .collect::<BTreeMap<_, _>>(),
        );
        write_lines(&output.join(format!("stats-{language}.ndjson")), &stats)?;

        let source = get(
            &agent,
            &raw_url(
                EXILED_EXCHANGE,
                &commit.sha,
                &format!("renderer/public/data/{language}/items.ndjson"),
            ),
        )?;
        let bases = bases::reduce(&source, &base_spawns, language, &mut report)
            .map_err(|error| format!("{language}: {error}"))?;
        println!("{language}: {} bases", bases.len());
        write_lines(&output.join(format!("bases-{language}.ndjson")), &bases)?;
    }
    if let [english, french] = ids_by_language.as_slice() {
        stats::report_differences(english, french, &mut report);
    }

    notice::write(
        &agent,
        &output,
        (&commit, &repoe_commit),
        repoe_version.trim(),
        &repoe_code_reference,
    )?;
    fs::write(
        output.join("report.md"),
        report.to_markdown(&format!(
            "the commit {} of Exiled Exchange 2 and the commit {} of the data of RePoE",
            commit.sha, repoe_commit.sha
        )),
    )?;
    println!("Written to {}", output.canonicalize()?.display());
    Ok(())
}

/// The modifier tables of the commit `commit` of the data of RePoE, and the tags of the bases.
fn modifier_tables(
    agent: &ureq::Agent,
    commit: &str,
    templates: &modifiers::Templates,
    report: &mut Report,
) -> Result<(Vec<modifiers::Table>, modifiers::BaseSpawns)> {
    let repoe = |path: &str| get(agent, &raw_url(REPOE_DATA, commit, &format!("data/{path}")));
    modifiers::reduce_with_report(
        &serde_json::from_str(&repoe("mods.json")?)?,
        &serde_json::from_str(&repoe("base_items.json")?)?,
        &serde_json::from_str::<Vec<_>>(&repoe("stat_translations/stat_descriptions.json")?)?,
        &serde_json::from_str::<Vec<_>>(&repoe(
            "stat_translations/tablet_stat_descriptions.json",
        )?)?,
        templates,
        modifiers::CURRENCIES,
        report,
    )
}

/// `assets/game-data/` of the application.
fn output_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/game-data")
}

/// Writes one stat or one base per line.
fn write_lines<T: Serialize>(path: &Path, lines: &[T]) -> Result<()> {
    let mut text = String::new();
    for line in lines {
        text.push_str(&serde_json::to_string(line)?);
        text.push('\n');
    }
    fs::write(path, text)?;
    Ok(())
}
