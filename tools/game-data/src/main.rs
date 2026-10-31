//! Generates the game data embedded in the application, in `assets/game-data/`, from the
//! stats and the items of Exiled Exchange 2 and the modifiers of the RePoE fork (see
//! `docs/game-data.md`). Run it again after each patch of the game:
//!
//! - `cargo run -p game-data` reads the latest commit of each;
//! - `cargo run -p game-data -- <commit of Exiled Exchange 2> [<commit of the data of RePoE>
//!   [<commit of the code of RePoE>]]` reads given ones, the code of RePoE for its license.
//!
//! The files it writes have one stat, one base or one modifier per line, so that the changes
//! of a patch read well in a diff.

mod modifiers;

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const REPOSITORY: &str = "Kvan7/Exiled-Exchange-2";
/// The data of Path of Exile 2 exported by the RePoE fork, in `data/`.
const REPOE: &str = "repoe-fork/poe2";
/// The code of the RePoE fork, with its license.
const REPOE_CODE: &str = "repoe-fork/repoe";
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
/// The categories of the bases the application searches by their filters: the equipment,
/// the jewels, the flasks, the charms, the waystones (`Map`) and the tablets (`TowerAugment`).
const CATEGORIES: [&str; 32] = [
    "Amulet",
    "Belt",
    "Body Armour",
    "Boots",
    "Bow",
    "Buckler",
    "Charm",
    "Crossbow",
    "Dagger",
    "Flail",
    "Flask",
    "Focus",
    "Gloves",
    "Helmet",
    "Jewel",
    "Map",
    "One Hand Axe",
    "One Hand Mace",
    "One Hand Sword",
    "Quiver",
    "Ring",
    "Sceptre",
    "Shield",
    "Spear",
    "Staff",
    "Talisman",
    "TowerAugment",
    "Two Hand Axe",
    "Two Hand Mace",
    "Two Hand Sword",
    "Wand",
    "Warstaff",
];
/// The categories left out: the currencies, the gems and the other items without base filters.
const LEFT_OUT_CATEGORIES: [&str; 14] = [
    "Breachstone",
    "BrequelFruit",
    "Currency",
    "ExpeditionLogbook",
    "Incubator",
    "MapFragment",
    "MiscMapItem",
    "Omen",
    "PinnacleKey",
    "QuestItem",
    "Relic",
    "SoulCore",
    "UncutSkillGem",
    "VaultKey",
];

pub(crate) type Result<T> = std::result::Result<T, Box<dyn Error>>;

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

/// An item of `renderer/public/data/<language>/items.ndjson`, with the fields the generator
/// reads.
#[derive(Deserialize)]
struct SourceItem {
    /// None for a few uniques without data.
    name: Option<String>,
    /// The English name.
    #[serde(rename = "refName")]
    ref_name: Option<String>,
    namespace: String,
    craftable: Option<SourceCraftable>,
}

#[derive(Deserialize)]
struct SourceCraftable {
    category: String,
}

/// A base as the application reads it (`src/game_data/bases.rs`): its name in the language
/// of the file, its category, and the tags that tell the modifiers it rolls.
#[derive(Serialize)]
struct Base {
    name: String,
    category: String,
    /// The domain of its modifiers, written when it isn't `item`.
    #[serde(skip_serializing_if = "modifiers::is_item")]
    domain: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tags: Vec<String>,
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
    let mut arguments = std::env::args().skip(1);
    let reference = arguments.next().unwrap_or_else(|| "master".to_owned());
    let repoe_reference = arguments.next().unwrap_or_else(|| "master".to_owned());
    let repoe_code_reference = arguments.next().unwrap_or_else(|| "master".to_owned());
    let agent = ureq::Agent::new_with_defaults();
    let commit = latest_commit(&agent, REPOSITORY, &reference)?;
    println!("Exiled Exchange 2, commit {}", commit.sha);
    let repoe_commit = latest_commit(&agent, REPOE, &repoe_reference)?;
    let repoe_version = get(
        &agent,
        &raw_url(REPOE, &repoe_commit.sha, "exported-version.txt"),
    )?;
    println!(
        "RePoE, commit {}, version {}",
        repoe_commit.sha,
        repoe_version.trim()
    );

    let output = output_directory();
    fs::create_dir_all(&output)?;
    let mut sources = Vec::new();
    for language in LANGUAGES {
        let source = get(
            &agent,
            &raw_url(
                REPOSITORY,
                &commit.sha,
                &format!("renderer/public/data/{language}/stats.ndjson"),
            ),
        )?;
        sources.push((
            language,
            reduce(&source).map_err(|error| format!("{language}: {error}"))?,
        ));
    }
    let english = sources
        .iter()
        .find(|(language, _)| *language == "en")
        .map(|(_, stats)| stats.as_slice())
        .ok_or("no English stats")?;
    let (tables, base_tags) = modifier_tables(&agent, &repoe_commit.sha, &templates(english))?;
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
                REPOSITORY,
                &commit.sha,
                &format!("renderer/public/data/{language}/items.ndjson"),
            ),
        )?;
        let bases =
            reduce_bases(&source, &base_tags).map_err(|error| format!("{language}: {error}"))?;
        println!("{language}: {} bases", bases.len());
        write_lines(&output.join(format!("bases-{language}.ndjson")), &bases)?;
    }
    if let [english, french] = ids_by_language.as_slice() {
        report_differences(english, french);
    }

    let license = get(&agent, &raw_url(REPOSITORY, &commit.sha, "LICENSE"))?;
    let repoe_code_commit = latest_commit(&agent, REPOE_CODE, &repoe_code_reference)?;
    println!("RePoE code, commit {}", repoe_code_commit.sha);
    let repoe_license = get(
        &agent,
        &raw_url(REPOE_CODE, &repoe_code_commit.sha, "LICENSE.md"),
    )?;
    fs::write(
        output.join("NOTICE.md"),
        notice(
            &Source {
                commit: &commit.sha,
                date: &commit.commit.committer.date,
                license: &license,
            },
            &Source {
                commit: &repoe_commit.sha,
                date: &repoe_commit.commit.committer.date,
                license: &repoe_license,
            },
            repoe_version.trim(),
            &repoe_code_commit.sha,
        ),
    )?;
    println!("Written to {}", output.canonicalize()?.display());
    Ok(())
}

/// The English templates of `stats`, the English stats of Exiled Exchange 2, by each of their
/// strings.
fn templates(stats: &[Stat]) -> modifiers::Templates {
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

/// The modifier tables of the commit `commit` of the data of RePoE, and the tags of the bases.
fn modifier_tables(
    agent: &ureq::Agent,
    commit: &str,
    templates: &modifiers::Templates,
) -> Result<(Vec<modifiers::Modifier>, modifiers::BaseTags)> {
    let repoe = |path: &str| get(agent, &raw_url(REPOE, commit, &format!("data/{path}")));
    modifiers::reduce(
        &serde_json::from_str(&repoe("mods.json")?)?,
        &serde_json::from_str(&repoe("base_items.json")?)?,
        &serde_json::from_str::<Vec<_>>(&repoe("stat_translations/stat_descriptions.json")?)?,
        &serde_json::from_str::<Vec<_>>(&repoe(
            "stat_translations/tablet_stat_descriptions.json",
        )?)?,
        templates,
        modifiers::CURRENCIES,
    )
}

/// `assets/game-data/` of the application.
fn output_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/game-data")
}

/// The commit `reference` (a branch or a commit) of the GitHub `repository`.
fn latest_commit(agent: &ureq::Agent, repository: &str, reference: &str) -> Result<Commit> {
    Ok(serde_json::from_str(&get(
        agent,
        &format!("https://api.github.com/repos/{repository}/commits/{reference}"),
    )?)?)
}

fn raw_url(repository: &str, commit: &str, path: &str) -> String {
    format!("https://raw.githubusercontent.com/{repository}/{commit}/{path}")
}

fn get(agent: &ureq::Agent, url: &str) -> Result<String> {
    read(agent, url).map_err(|error| format!("{url}: {error}").into())
}

fn read(agent: &ureq::Agent, url: &str) -> Result<String> {
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

/// The bases of an `items.ndjson`, of the categories the application searches, each name
/// once: a base listed in several categories takes the one most of its entries have
/// (`Runemastered Venerable Defender`, three times a shield and once a buckler), the first in
/// alphabetical order on a tie. Each takes the domain and the tags of its English name in
/// `spawns`.
fn reduce_bases(source: &str, spawns: &modifiers::BaseTags) -> Result<Vec<Base>> {
    let mut categories: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    let mut english_names = BTreeMap::new();
    for (index, line) in source.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let item: SourceItem =
            serde_json::from_str(line).map_err(|error| format!("line {}: {error}", index + 1))?;
        let (Some(name), Some(craftable)) = (item.name, item.craftable) else {
            continue;
        };
        if item.namespace != "ITEM" || LEFT_OUT_CATEGORIES.contains(&craftable.category.as_str()) {
            continue;
        }
        // A new category of the game is decided before it is embedded.
        if !CATEGORIES.contains(&craftable.category.as_str()) {
            return Err(format!("unknown category {} of {name}", craftable.category).into());
        }
        if let Some(english) = item.ref_name {
            english_names.insert(name.clone(), english);
        }
        *categories
            .entry(name)
            .or_default()
            .entry(craftable.category)
            .or_default() += 1;
    }
    let mut bases = Vec::new();
    for (name, counts) in categories {
        if counts.len() > 1 {
            println!("In several categories: {name} {counts:?}");
        }
        let category = counts
            .into_iter()
            .max_by(|(first, first_count), (second, second_count)| {
                first_count
                    .cmp(second_count)
                    .then_with(|| second.cmp(first))
            })
            .map(|(category, _)| category)
            .ok_or("a base without category")?;
        let english = english_names.get(&name).unwrap_or(&name);
        let (domain, tags) = spawns.get(english).map_or_else(
            || (modifiers::EQUIPMENT.to_owned(), Vec::new()),
            |spawn| (spawn.domain.clone(), spawn.tags.clone()),
        );
        // The application reads the domains of the modifiers kept only.
        if !modifiers::is_kept(&domain) {
            return Err(format!("unknown domain {domain} of {name}").into());
        }
        if tags.is_empty() {
            println!("Without tags: {name}");
        }
        bases.push(Base {
            name,
            category,
            domain,
            tags,
        });
    }
    Ok(bases)
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

/// A source of the data: its commit, the date of the commit, and its license.
struct Source<'a> {
    commit: &'a str,
    date: &'a str,
    license: &'a str,
}

/// The notice of the generated files; the license of `repoe` is that of its code, at the commit
/// `repoe_code`.
fn notice(
    exiled_exchange: &Source,
    repoe: &Source,
    repoe_version: &str,
    repoe_code: &str,
) -> String {
    format!(
        "# Game data notice

Generated by `tools/game-data` (`cargo run -p game-data`, see `docs/game-data.md`): don't edit these files by hand, generate them again.

`stats-en.ndjson` and `stats-fr.ndjson` are the stats of [Exiled Exchange 2](https://github.com/{REPOSITORY}) (`renderer/public/data/<language>/stats.ndjson`), commit `{}` of {}, reduced to the fields the application reads and without the groups of the Sanctum. `bases-en.ndjson` and `bases-fr.ndjson` are the bases of its items (`renderer/public/data/<language>/items.ndjson`), of the same commit: the name and the category of each base of equipment, jewel, flask, charm, waystone or tablet, with the tags of the base from RePoE.

`modifiers.ndjson` holds the tiers of the prefixes and suffixes of the equipment, the jewels, the flasks, the charms, the waystones and the tablets, from the data of the [RePoE fork](https://github.com/{REPOE}) (`data/mods.json`, `data/base_items.json`, `data/stat_translations/stat_descriptions.json` and `data/stat_translations/tablet_stat_descriptions.json`), commit `{}` of {}, export of the version {repoe_version} of the game, their lines written as the templates of Exiled Exchange 2. The bases of the modifiers only an essence adds, which RePoE doesn't give, are those of [Craft of Exile](https://www.craftofexile.com), checked on the trade site.

Their texts, ids and values come from the files of Path of Exile 2 and belong to Grinding Gear Games; they are used here in a free, non-commercial tool. The files of Exiled Exchange 2 are under the MIT license, whose notice follows:

```text
{}
```

The code of the RePoE fork ([{REPOE_CODE}](https://github.com/{REPOE_CODE}), commit `{repoe_code}`), which exports these data, is under the MIT license, whose notice follows:

```text
{}
```

This product isn't affiliated with or endorsed by Grinding Gear Games in any way.
",
        exiled_exchange.commit,
        exiled_exchange.date,
        repoe.commit,
        repoe.date,
        exiled_exchange.license.trim_end(),
        repoe.license.trim_end(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bases of `lines`, as name and category.
    fn bases(lines: &[&str]) -> Result<Vec<(String, String)>> {
        Ok(reduce_bases(&lines.join("\n"), &BTreeMap::new())?
            .into_iter()
            .map(|base| (base.name, base.category))
            .collect())
    }

    fn base(name: &str, category: &str) -> (String, String) {
        (name.to_owned(), category.to_owned())
    }

    #[test]
    fn a_base_of_a_domain_the_application_doesnt_know_stops_the_generator() {
        let spawn = |domain: &str| modifiers::BaseSpawn {
            domain: domain.to_owned(),
            tags: vec!["ring".to_owned()],
        };
        let ring =
            r#"{"name": "Gold Ring", "namespace": "ITEM", "craftable": {"category": "Ring"}}"#;
        let reduced = |domain| {
            reduce_bases(
                ring,
                &BTreeMap::from([("Gold Ring".to_owned(), spawn(domain))]),
            )
        };

        assert!(reduced("item").is_ok());
        assert_eq!(
            reduced("heist").err().map(|error| error.to_string()),
            Some("unknown domain heist of Gold Ring".to_owned())
        );
    }

    #[test]
    fn the_bases_of_the_searched_categories_are_kept_each_name_once() {
        let kept = bases(&[
            r#"{"name": "Mammoth Targe", "namespace": "ITEM", "craftable": {"category": "Shield"}}"#,
            r#"{"name": "Mammoth Targe", "namespace": "ITEM", "craftable": {"category": "Shield"}}"#,
            // The waystones and the tablets.
            r#"{"name": "Waystone (Tier 12)", "namespace": "ITEM", "craftable": {"category": "Map"}}"#,
            r#"{"name": "Irradiated Tablet", "namespace": "ITEM", "craftable": {"category": "TowerAugment"}}"#,
            // Another category, another namespace, no category, no name.
            r#"{"name": "Exalted Orb", "namespace": "ITEM", "craftable": {"category": "Currency"}}"#,
            r#"{"name": "Headhunter", "namespace": "UNIQUE", "craftable": {"category": "Belt"}}"#,
            r#"{"name": "Gold", "namespace": "ITEM"}"#,
            r#"{"namespace": "UNIQUE", "unique": {"base": "Ring"}}"#,
            "",
        ])
        .expect("read");

        assert_eq!(
            kept,
            [
                base("Irradiated Tablet", "TowerAugment"),
                base("Mammoth Targe", "Shield"),
                base("Waystone (Tier 12)", "Map"),
            ]
        );
    }

    #[test]
    fn a_base_in_several_categories_takes_the_one_most_of_its_entries_have() {
        let defender = |category: &str| {
            format!(
                r#"{{"name": "Venerable Defender", "namespace": "ITEM", "craftable": {{"category": "{category}"}}}}"#
            )
        };
        let most = bases(&[
            &defender("Shield"),
            &defender("Buckler"),
            &defender("Shield"),
        ]);
        // On a tie, the first in alphabetical order.
        let tie = bases(&[&defender("Shield"), &defender("Buckler")]);

        assert_eq!(most.expect("read"), [base("Venerable Defender", "Shield")]);
        assert_eq!(tie.expect("read"), [base("Venerable Defender", "Buckler")]);
    }

    #[test]
    fn a_new_category_stops_the_generator() {
        let new =
            bases(&[r#"{"name": "Idol", "namespace": "ITEM", "craftable": {"category": "Idol"}}"#]);

        assert_eq!(
            new.expect_err("a new category").to_string(),
            "unknown category Idol of Idol"
        );
    }
}
