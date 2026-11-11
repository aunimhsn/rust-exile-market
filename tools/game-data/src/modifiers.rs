//! The modifier tables, from the RePoE fork (`mods.json`, `base_items.json`,
//! `stat_translations/stat_descriptions.json` and
//! `stat_translations/tablet_stat_descriptions.json`): the tiers of each prefix and suffix that
//! rolls on the equipment, the jewels, the flasks, the charms, the waystones and the tablets, also
//! those a desecration reveals (the lords of the Abyss), those of the Genesis tree of the Breach
//! and those only an essence or an alloy of Verisium adds, and the tags of each base that tell
//! where a modifier rolls.

mod currencies;
#[cfg(test)]
mod fixtures;
mod lines;

use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde::{Deserialize, Serialize};

use crate::Result;
use crate::report::Report;
pub use currencies::CURRENCIES;
use currencies::{CURRENCY_IDS, CurrencyModifier, currency_spawns};
pub use lines::Templates;
use lines::{
    Line, LineRanges, Translation, by_stat, is_radius_jewel_tag, lines, rolls_on_radius_jewel,
};

/// The domain of the modifiers of the equipment, which the application takes when none is
/// written.
pub const EQUIPMENT: &str = "item";

/// The domain of the modifiers of the jewels.
const JEWELS: &str = "misc";

/// The domain of the modifiers a desecration reveals on the equipment and the jewels: those of
/// the lords of the Abyss (Amanamu, Kurgal, Ulaman) and of the Genesis tree, of one tier each.
const DESECRATED: &str = "desecrated";

/// The tags of the lords of the Abyss, after `default`, on no base, and the name of each.
const LORDS: [(&str, &str); 3] = [
    ("amanamu_mod", "Amanamu"),
    ("kurgal_mod", "Kurgal"),
    ("ulaman_mod", "Ulaman"),
];

/// The tags the Genesis tree of the Breach gives the items it grows, which no base has: its
/// desecrated modifiers (`breach_desecration`) and those of the minions (`genesis_tree_minion`,
/// of several tiers, which the game writes without tier).
const GENESIS_TAGS: [&str; 2] = ["breach_desecration", "genesis_tree_minion"];

/// The tags of the bases the Genesis tree grows, the rings, the amulets and the belts (seen on
/// the trade site on 2026-10-30): a modifier of a tag of [`GENESIS_TAGS`] rolls on them, but on
/// those its tags before rule out (`ring` and `belt` at 0 for one of the amulets).
const GENESIS_BASES: [&str; 3] = ["ring", "amulet", "belt"];

/// The domains of the bases kept, and of their modifiers: the equipment, the jewels, the flasks
/// and the charms, the waystones (`area`) and the tablets; each one a domain the application
/// knows (`Domain` in `src/game_data/bases.rs`), as is [`DESECRATED`], the domain of modifiers
/// only.
const DOMAINS: [&str; 5] = [EQUIPMENT, JEWELS, "flask", "area", TABLETS];

/// The domain of the modifiers of the tablets, which the game writes as the translations of
/// `tablet_stat_descriptions.json` write them (`30% increased Gold found in Map`, not `… in
/// this Area`), before those of `stat_descriptions.json`.
const TABLETS: &str = "tablet";

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
    stats: Vec<StatRange>,
}

/// The weight of a modifier on the bases with a tag: the first tag of the list a base has
/// tells whether the modifier rolls on it (above zero) or not.
#[derive(Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SpawnWeight {
    tag: String,
    weight: u32,
}

/// A stat of a modifier and the range of its value.
#[derive(Deserialize)]
struct StatRange {
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

/// The tiers of a type of modifier that roll on the same bases, as the application reads them
/// (`src/game_data/modifiers.rs`).
#[derive(Serialize)]
pub struct Table {
    #[serde(rename = "type")]
    kind: String,
    affix: String,
    /// The domain of the bases it rolls on, written when it isn't `item`: a tag of every base,
    /// `default`, rolls on the bases of its domain only (the charges of the flasks); a
    /// desecrated modifier rolls on the equipment and the jewels.
    #[serde(skip_serializing_if = "is_equipment")]
    domain: String,
    lines: Vec<Line>,
    /// Each tag and its weight, as `["bow", 1]`.
    spawn: Vec<(String, u32)>,
    /// Where the modifier comes from when it isn't an ordinary one, written when it isn't: a
    /// modifier of its own, whose tiers the game doesn't write.
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<Source>,
    /// The lord of the Abyss whose modifier it is, written when it is one ("Amanamu"): named
    /// by the application before the affix of a line that can be his or an ordinary modifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    lord: Option<String>,
    tiers: Vec<Tier>,
}

/// Where a modifier that isn't an ordinary one comes from.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    /// The Genesis tree of the Breach, on the rings, the amulets and the belts it grows.
    Genesis,
    /// An essence ([`ESSENCES`](currencies::ESSENCES)).
    Essence,
    /// An alloy of Verisium ([`ALLOYS`](currencies::ALLOYS)).
    Alloy,
}

impl Source {
    /// The currency, with its article, in the errors of its list.
    fn currency(self) -> &'static str {
        match self {
            Self::Genesis => "the Genesis tree",
            Self::Essence => "an essence",
            Self::Alloy => "an alloy",
        }
    }
}

/// Whether `domain` is one of the bases kept, which the application knows.
pub fn is_kept_domain(domain: &str) -> bool {
    DOMAINS.contains(&domain)
}

/// Whether a modifier of `domain` rolls on the bases of `base_domain`: those of its domain, the
/// equipment and the jewels for a desecrated one (`Domain::rolls_on` in
/// `src/game_data/bases.rs`).
fn rolls_on(domain: &str, base_domain: &str) -> bool {
    domain == base_domain || (domain == DESECRATED && [EQUIPMENT, JEWELS].contains(&base_domain))
}

/// Whether `domain` is that of the equipment, left unwritten.
pub fn is_equipment(domain: &String) -> bool {
    domain == EQUIPMENT
}

#[derive(Serialize, PartialEq, Eq, PartialOrd, Ord)]
struct Tier {
    level: u32,
    /// For each line, the range of each of its values, as the game shows them (per second for
    /// a value stored per minute), with the sign of the stat.
    ranges: Vec<LineRanges>,
}

/// The modifiers of one type and affix, with the same lines, that roll on the same bases, and
/// where they come from when they aren't ordinary ones.
type Key = (
    String,
    String,
    String,
    Vec<Line>,
    Vec<SpawnWeight>,
    Option<Source>,
);

/// What tells the modifiers that roll on a base: their domain and the tags of the base.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BaseSpawn {
    pub domain: String,
    pub tags: Vec<String>,
}

/// The domain and the tags of each base, by its English name.
pub type BaseSpawns = BTreeMap<String, BaseSpawn>;

/// The modifier tables of `modifiers`, those only a currency of `currencies` adds among them
/// ([`CURRENCIES`]), the lines of their stats written as the English templates of Exiled
/// Exchange 2 (by `tablet_translations` first for the tablets), and the tags of the bases that
/// the tables read, by English name of the base.
#[cfg(test)]
pub fn reduce(
    modifiers: &BTreeMap<String, SourceModifier>,
    bases: &BTreeMap<String, SourceBase>,
    translations: &[Translation],
    tablet_translations: &[Translation],
    templates: &Templates,
    currencies: &[(Source, &[CurrencyModifier])],
) -> Result<(Vec<Table>, BaseSpawns)> {
    reduce_with_report(
        modifiers,
        bases,
        translations,
        tablet_translations,
        templates,
        currencies,
        &mut Report::default(),
    )
}

/// [`reduce`], with what is left out or odd noted in `report`.
pub fn reduce_with_report(
    modifiers: &BTreeMap<String, SourceModifier>,
    bases: &BTreeMap<String, SourceBase>,
    translations: &[Translation],
    tablet_translations: &[Translation],
    templates: &Templates,
    currencies: &[(Source, &[CurrencyModifier])],
    report: &mut Report,
) -> Result<(Vec<Table>, BaseSpawns)> {
    let released: Vec<&SourceBase> = bases
        .values()
        .filter(|base| base.release_state == "released")
        .collect();
    let currencies = currency_spawns(modifiers, currencies, &released)?;
    let grouped = group_tiers(
        modifiers,
        &by_stat(&[translations]),
        &by_stat(&[tablet_translations, translations]),
        templates,
        &currencies,
        report,
    )?;

    let mut tables = Vec::new();
    let mut unknown_lines = BTreeSet::new();
    let mut unsigned_lines = BTreeSet::new();
    for ((kind, affix, domain, lines, spawn, source), mut tiers) in grouped {
        let lord = spawn
            .iter()
            .filter(|weight| weight.weight > 0)
            .find_map(|weight| LORDS.iter().find(|&&(tag, _)| weight.tag == tag))
            .map(|&(_, name)| name.to_owned());
        let tags_of_bases: BTreeSet<&String> = released
            .iter()
            .filter(|base| rolls_on(&domain, &base.domain))
            .flat_map(|base| &base.tags)
            .collect();
        // Only the tags of the bases matter, in their order: the first one a base has decides.
        let spawn: Vec<(String, u32)> = spawn
            .into_iter()
            .filter(|spawn| tags_of_bases.contains(&spawn.tag))
            .map(|spawn| (spawn.tag, spawn.weight))
            .collect();
        // A modifier that rolls on no released base of its domains, e.g. one of Path of Exile
        // left in the files, or a desecrated one of the waystones.
        if !spawn.iter().any(|&(_, weight)| weight > 0) {
            continue;
        }
        // A modifier without line, its values all 0 (`TrapThrowSpeed`), is never written.
        if lines.is_empty() {
            continue;
        }
        // A line without template can't be found, nor its modifier.
        let unknown: Vec<&Line> = lines.iter().filter(|line| !line.known).collect();
        if !unknown.is_empty() {
            unknown_lines.extend(unknown.iter().map(|line| line.template.clone()));
            continue;
        }
        unsigned_lines.extend(lines.iter().filter_map(|line| {
            let string = line.unsigned.clone()?;
            Some((string, line.template.clone()))
        }));
        // By level, then by ranges: of two tiers of one level, the higher ranges last, the better
        // tier for the application (`Upgrades Radius to Large` after `to Medium`, both of level
        // 1: its T1, as the game writes it).
        tiers.sort();
        tables.push(Table {
            kind,
            affix,
            domain,
            lines,
            spawn,
            source,
            lord,
            tiers,
        });
    }
    for line in unknown_lines {
        report.note(
            "Modifier lines without template, their modifier left out",
            line,
        );
    }
    for (string, template) in unsigned_lines {
        report.note(
            "Modifier lines known without the + of their number",
            format!("{string} as {template}"),
        );
    }
    // The ordinary tables first, then those of the Genesis tree, of the essences and of the
    // alloys, then the desecrated ones: of two tables that hold a copied modifier as well, the
    // application takes the first of the file, the ordinary one (`+2 to Level of all Minion
    // Skills` of an amulet, a T2 of its own or a minion modifier of the Genesis tree), but for a
    // header written without tier, which takes one whose tier the game doesn't write (`find` in
    // the application).
    tables.sort_by_key(|table| (table.domain == DESECRATED, table.source));
    let base_spawns = base_spawns(&released, &tables);
    Ok((tables, base_spawns))
}

/// Notes in `report` a prefix or suffix the tables leave out without an error, which a patch
/// may have brought: one only a currency adds (`is_essence_only`, or named like those of
/// [`ESSENCES`](currencies::ESSENCES) and [`ALLOYS`](currencies::ALLOYS)) that these lists
/// don't have, `listed` telling whether they do; and, counted in `other_domains`, one of a
/// domain the application doesn't read.
fn note_left_out<'a>(
    id: &str,
    modifier: &'a SourceModifier,
    listed: bool,
    other_domains: &mut BTreeMap<&'a str, usize>,
    report: &mut Report,
) {
    if !["prefix", "suffix"].contains(&modifier.generation_type.as_str()) {
        return;
    }
    if !(is_kept_domain(&modifier.domain) || modifier.domain == DESECRATED) {
        *other_domains.entry(&modifier.domain).or_default() += 1;
        return;
    }
    let of_a_currency =
        modifier.is_essence_only || CURRENCY_IDS.iter().any(|start| id.starts_with(start));
    if of_a_currency && !listed {
        report.note(
            "Modifiers of a currency that ESSENCES and ALLOYS don't list, left out",
            id,
        );
    }
}

/// The tiers of `modifiers` that roll (with the currency and the spawn weights of `currencies`
/// for those only a currency adds), grouped by type, affix, domain, lines, bases and source,
/// their lines written by the translations of `by_stat`, those of the tablets by
/// `tablet_by_stat`.
fn group_tiers(
    modifiers: &BTreeMap<String, SourceModifier>,
    by_stat: &HashMap<&str, &Translation>,
    tablet_by_stat: &HashMap<&str, &Translation>,
    templates: &Templates,
    currencies: &HashMap<&str, (Source, Vec<SpawnWeight>)>,
    report: &mut Report,
) -> Result<BTreeMap<Key, Vec<Tier>>> {
    let mut grouped: BTreeMap<Key, Vec<Tier>> = BTreeMap::new();
    let mut other_domains: BTreeMap<&str, usize> = BTreeMap::new();
    for (id, modifier) in modifiers {
        let currency = currencies.get(id.as_str());
        note_left_out(id, modifier, currency.is_some(), &mut other_domains, report);
        let spawn_weights = currency.map_or(&modifier.spawn_weights, |(_, spawn)| spawn);
        let rolls = (is_kept_domain(&modifier.domain) || modifier.domain == DESECRATED)
            && ["prefix", "suffix"].contains(&modifier.generation_type.as_str())
            && (currency.is_some() || !modifier.is_essence_only)
            && spawn_weights.iter().any(|spawn| spawn.weight > 0);
        if !rolls {
            continue;
        }
        // Its lines would be written within the words of the passive skills on the other bases.
        if rolls_on_radius_jewel(modifier)
            && modifier
                .spawn_weights
                .iter()
                .any(|spawn| spawn.weight > 0 && !is_radius_jewel_tag(&spawn.tag))
        {
            return Err(format!(
                "{id}: a modifier of the Time-Lost jewels that rolls on other bases"
            )
            .into());
        }
        let by_stat = if modifier.domain == TABLETS {
            tablet_by_stat
        } else {
            by_stat
        };
        let (lines, ranges) =
            lines(modifier, by_stat, templates).map_err(|error| format!("{id}: {error}"))?;
        // Left out, it would be read as no modifier, without a word.
        if let Some((source, _)) = currency {
            if let Some(line) = lines.iter().find(|line| !line.known) {
                return Err(format!(
                    "{id}: a modifier of {} with a line without template, {}",
                    source.currency(),
                    line.template
                )
                .into());
            }
            if lines.is_empty() {
                return Err(format!(
                    "{id}: a modifier of {} without line, its values all 0",
                    source.currency()
                )
                .into());
            }
        }
        let (spawn, genesis) = spawn_on_bases(spawn_weights);
        let source = match currency {
            Some(&(source, _)) => Some(source),
            None => genesis.then_some(Source::Genesis),
        };
        grouped
            .entry((
                modifier.kind.clone(),
                modifier.generation_type.clone(),
                modifier.domain.clone(),
                lines,
                spawn,
                source,
            ))
            .or_default()
            .push(Tier {
                // An essence or an alloy adds its modifier whatever the level of the item,
                // below the level RePoE writes (72 for the Perfect essences): a Cultist Bow of
                // level 33 with the `+2 to Level of all Attack Skills` of an essence, on the
                // trade site on 2026-10-30, a helmet of level 7 with an alloy of level 25 on
                // 2026-10-31.
                level: if currency.is_some() {
                    1
                } else {
                    modifier.required_level
                },
                ranges,
            });
    }
    for (domain, count) in other_domains {
        report.note(
            "Prefixes and suffixes of a domain the application doesn't read, left out",
            format!("{domain}: {count}"),
        );
    }
    Ok(grouped)
}

/// The tags of `spawn` with a tag of the Genesis tree ([`GENESIS_TAGS`]) that rolls in place of
/// the tags of the bases it grows ([`GENESIS_BASES`]), with its weight, after the tags before it
/// (a tag written twice is decided by the first), and whether it had one. A tag of the tree at 0
/// is left out: on no base, it decides nothing.
fn spawn_on_bases(spawn: &[SpawnWeight]) -> (Vec<SpawnWeight>, bool) {
    let mut on_bases: Vec<SpawnWeight> = Vec::new();
    let mut genesis = false;
    for weight in spawn {
        if !GENESIS_TAGS.contains(&weight.tag.as_str()) {
            on_bases.push(weight.clone());
            continue;
        }
        if weight.weight == 0 {
            continue;
        }
        genesis = true;
        for tag in GENESIS_BASES {
            if !on_bases.iter().any(|before| before.tag == tag) {
                on_bases.push(SpawnWeight {
                    tag: tag.to_owned(),
                    weight: weight.weight,
                });
            }
        }
    }
    (on_bases, genesis)
}

/// The domain of each base among `released` and its tags that tell where a modifier of
/// `tables` rolls, by its name.
fn base_spawns(released: &[&SourceBase], tables: &[Table]) -> BaseSpawns {
    let tags_used: BTreeSet<&String> = tables
        .iter()
        .flat_map(|table| table.spawn.iter().map(|(tag, _)| tag))
        .collect();
    let mut base_spawns = BaseSpawns::new();
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
        match base_spawns.get_mut(&base.name) {
            Some(known) if *known != spawn => known.tags.clear(),
            Some(_) => {}
            None => {
                base_spawns.insert(base.name.clone(), spawn);
            }
        }
    }
    base_spawns
}

#[cfg(test)]
mod tests;
