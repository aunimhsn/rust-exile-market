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
use currencies::{Currency, CurrencyModifier, CurrencySpawn, currency_spawns};
pub use lines::Templates;
use lines::{Line, LineRanges, LineWriter, Translation, is_radius_jewel_tag};

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

impl SourceModifier {
    /// Whether it is a prefix or a suffix, the modifiers the tables hold.
    fn is_affix(&self) -> bool {
        ["prefix", "suffix"].contains(&self.generation_type.as_str())
    }

    /// The tags of the bases it rolls on, by its own spawn weights.
    fn rolling_tags(&self) -> impl Iterator<Item = &str> {
        self.spawn_weights
            .iter()
            .filter(|spawn_weight| spawn_weight.rolls())
            .map(|spawn_weight| spawn_weight.tag.as_str())
    }

    /// Whether it rolls on a Time-Lost jewel.
    fn rolls_on_radius_jewel(&self) -> bool {
        self.rolling_tags().any(is_radius_jewel_tag)
    }

    /// Its stat `id`, if it has it.
    fn stat(&self, id: &str) -> Option<&StatRange> {
        self.stats.iter().find(|stat| stat.id == id)
    }
}

/// The weight of a modifier on the bases with a tag: the first tag of the list a base has
/// tells whether the modifier rolls on it (above zero) or not.
#[derive(Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SpawnWeight {
    tag: String,
    weight: u32,
}

impl SpawnWeight {
    /// Whether the modifier rolls on the bases of its tag.
    fn rolls(&self) -> bool {
        self.weight > 0
    }
}

/// Written as the application reads it, `["bow", 1]`.
impl Serialize for SpawnWeight {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        (&self.tag, self.weight).serialize(serializer)
    }
}

/// A stat of a modifier and the range of its value.
#[derive(Deserialize)]
struct StatRange {
    id: String,
    min: i64,
    max: i64,
}

impl StatRange {
    /// Whether its value rolls, from one item to another.
    fn rolls(&self) -> bool {
        self.min != self.max
    }

    /// Whether its value is 0 on every item.
    fn is_zero(&self) -> bool {
        self.min == 0 && self.max == 0
    }
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
    /// Each tag and its weight.
    spawn: Vec<SpawnWeight>,
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

/// Whether `domain` is one of the bases kept, which the application knows.
pub fn is_kept_domain(domain: &str) -> bool {
    DOMAINS.contains(&domain)
}

/// Whether the tables read the modifiers of `domain`: those of the domains kept, and the
/// desecrated ones.
fn is_read_domain(domain: &str) -> bool {
    is_kept_domain(domain) || domain == DESECRATED
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

/// What the tiers of one table share: the modifiers of one type and affix, with the same
/// lines, that roll on the same bases, and where they come from when they aren't ordinary ones.
/// The tables are sorted by it, field by field.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct TableKey {
    kind: String,
    affix: String,
    domain: String,
    lines: Vec<Line>,
    spawn: Vec<SpawnWeight>,
    source: Option<Source>,
}

/// What tells the modifiers that roll on a base: their domain and the tags of the base.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BaseSpawn {
    pub domain: String,
    pub tags: Vec<String>,
}

/// The domain and the tags of each base, by its English name.
pub type BaseSpawns = BTreeMap<String, BaseSpawn>;

/// The files of the RePoE fork the tables are read from.
pub struct RepoeFiles {
    /// `mods.json`.
    pub modifiers: BTreeMap<String, SourceModifier>,
    /// `base_items.json`.
    pub bases: BTreeMap<String, SourceBase>,
    /// `stat_translations/stat_descriptions.json`.
    pub translations: Vec<Translation>,
    /// `stat_translations/tablet_stat_descriptions.json`, which writes the lines of the tablets
    /// first.
    pub tablet_translations: Vec<Translation>,
}

/// The modifier tables of `files`, those only a currency of `currencies` adds among them
/// ([`CURRENCIES`]), the lines of their stats written as the English templates of Exiled
/// Exchange 2 (`templates`), and the domain and the tags of the bases the tables read, by
/// English name of the base. What is left out or odd is noted in `report`.
pub fn reduce(
    files: &RepoeFiles,
    templates: &Templates,
    currencies: &[(Currency, &[CurrencyModifier])],
    report: &mut Report,
) -> Result<(Vec<Table>, BaseSpawns)> {
    let released: Vec<&SourceBase> = files
        .bases
        .values()
        .filter(|base| base.release_state == "released")
        .collect();
    let currencies = currency_spawns(&files.modifiers, currencies, &released)?;
    let writer = LineWriter::new(&files.translations, &files.tablet_translations, templates);
    let grouped = group_tiers(&files.modifiers, &writer, &currencies, report)?;

    let mut tables = Vec::new();
    let mut unknown_lines = BTreeSet::new();
    let mut unsigned_lines = BTreeSet::new();
    for (key, mut tiers) in grouped {
        let lord = lord(&key.spawn);
        let spawn = released_spawn(key.spawn, &key.domain, &released);
        // A modifier that rolls on no released base of its domains, e.g. one of Path of Exile
        // left in the files, or a desecrated one of the waystones.
        if !spawn.iter().any(SpawnWeight::rolls) {
            continue;
        }
        // A modifier without line, its values all 0 (`TrapThrowSpeed`), is never written.
        if key.lines.is_empty() {
            continue;
        }
        // A line without template can't be found, nor its modifier.
        let unknown: Vec<&Line> = key.lines.iter().filter(|line| !line.known).collect();
        if !unknown.is_empty() {
            unknown_lines.extend(unknown.iter().map(|line| line.template.clone()));
            continue;
        }
        unsigned_lines.extend(key.lines.iter().filter_map(|line| {
            let string = line.unsigned.clone()?;
            Some((string, line.template.clone()))
        }));
        // By level, then by ranges: of two tiers of one level, the higher ranges last, the better
        // tier for the application (`Upgrades Radius to Large` after `to Medium`, both of level
        // 1: its T1, as the game writes it).
        tiers.sort();
        tables.push(Table {
            kind: key.kind,
            affix: key.affix,
            domain: key.domain,
            lines: key.lines,
            spawn,
            source: key.source,
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

/// The lord of the Abyss whose modifier rolls with `spawn`, by the first of his tags that rolls.
fn lord(spawn: &[SpawnWeight]) -> Option<String> {
    spawn
        .iter()
        .filter(|spawn_weight| spawn_weight.rolls())
        .find_map(|spawn_weight| LORDS.iter().find(|&&(tag, _)| spawn_weight.tag == tag))
        .map(|&(_, name)| name.to_owned())
}

/// The tags of `spawn` that a base among `released` of the bases of `domain` has, in their order,
/// with their weight: only those matter, the first one a base has deciding.
fn released_spawn(
    spawn: Vec<SpawnWeight>,
    domain: &str,
    released: &[&SourceBase],
) -> Vec<SpawnWeight> {
    let tags_of_bases: BTreeSet<&String> = released
        .iter()
        .filter(|base| rolls_on(domain, &base.domain))
        .flat_map(|base| &base.tags)
        .collect();
    spawn
        .into_iter()
        .filter(|spawn_weight| tags_of_bases.contains(&spawn_weight.tag))
        .collect()
}

/// The tiers of the prefixes and suffixes of `modifiers` that roll (with the currency and the
/// spawn weights of `currencies` for those only a currency adds), grouped by table, their lines
/// written by `writer`. Notes in `report` those left out without an error, which a patch may
/// have brought: one only a currency adds that its lists don't have, and those of a domain the
/// application doesn't read.
fn group_tiers(
    modifiers: &BTreeMap<String, SourceModifier>,
    writer: &LineWriter<'_>,
    currencies: &HashMap<&str, CurrencySpawn>,
    report: &mut Report,
) -> Result<BTreeMap<TableKey, Vec<Tier>>> {
    let mut grouped: BTreeMap<TableKey, Vec<Tier>> = BTreeMap::new();
    let mut other_domains: BTreeMap<&str, usize> = BTreeMap::new();
    for (id, modifier) in modifiers {
        if !modifier.is_affix() {
            continue;
        }
        if !is_read_domain(&modifier.domain) {
            *other_domains.entry(&modifier.domain).or_default() += 1;
            continue;
        }
        let currency_spawn = currencies.get(id.as_str());
        if currency_spawn.is_none() && currencies::is_of_a_currency(id, modifier) {
            report.note(
                "Modifiers of a currency that ESSENCES and ALLOYS don't list, left out",
                id,
            );
        }
        let rolls = (currency_spawn.is_some() || !modifier.is_essence_only)
            && spawn_weights(modifier, currency_spawn)
                .iter()
                .any(SpawnWeight::rolls);
        if !rolls {
            continue;
        }
        let (key, tier) = table_tier(id, modifier, currency_spawn, writer)?;
        grouped.entry(key).or_default().push(tier);
    }
    for (domain, count) in other_domains {
        report.note(
            "Prefixes and suffixes of a domain the application doesn't read, left out",
            format!("{domain}: {count}"),
        );
    }
    Ok(grouped)
}

/// The spawn weights `modifier` rolls with: those of the currency that adds it
/// (`currency_spawn`), when it is a modifier only a currency adds, else its own.
fn spawn_weights<'m>(
    modifier: &'m SourceModifier,
    currency_spawn: Option<&'m CurrencySpawn>,
) -> &'m [SpawnWeight] {
    currency_spawn.map_or(&modifier.spawn_weights, |currency_spawn| {
        &currency_spawn.spawn
    })
}

/// The table of the modifier `id`, which rolls, and its tier, its lines written by `writer`; the
/// currency of `currency_spawn` adds it when it is a modifier only a currency adds.
fn table_tier(
    id: &str,
    modifier: &SourceModifier,
    currency_spawn: Option<&CurrencySpawn>,
    writer: &LineWriter<'_>,
) -> Result<(TableKey, Tier)> {
    // Its lines would be written within the words of the passive skills on the other bases.
    if modifier.rolls_on_radius_jewel()
        && modifier.rolling_tags().any(|tag| !is_radius_jewel_tag(tag))
    {
        return Err(
            format!("{id}: a modifier of the Time-Lost jewels that rolls on other bases").into(),
        );
    }
    let (lines, ranges) = writer
        .lines(modifier)
        .map_err(|error| format!("{id}: {error}"))?;
    if let Some(currency_spawn) = currency_spawn {
        currencies::check_lines(id, currency_spawn.currency, &lines)?;
    }
    let (spawn, genesis) = spawn_on_bases(spawn_weights(modifier, currency_spawn));
    let source = currency_spawn
        .map(|currency_spawn| Source::from(currency_spawn.currency))
        .or_else(|| genesis.then_some(Source::Genesis));
    let key = TableKey {
        kind: modifier.kind.clone(),
        affix: modifier.generation_type.clone(),
        domain: modifier.domain.clone(),
        lines,
        spawn,
        source,
    };
    let tier = Tier {
        // An essence or an alloy adds its modifier whatever the level of the item, below the
        // level RePoE writes (72 for the Perfect essences): a Cultist Bow of level 33 with the
        // `+2 to Level of all Attack Skills` of an essence, on the trade site on 2026-10-30, a
        // helmet of level 7 with an alloy of level 25 on 2026-10-31.
        level: if currency_spawn.is_some() {
            1
        } else {
            modifier.required_level
        },
        ranges,
    };
    Ok((key, tier))
}

/// The tags of `spawn` with a tag of the Genesis tree ([`GENESIS_TAGS`]) that rolls in place of
/// the tags of the bases it grows ([`GENESIS_BASES`]), with its weight, after the tags before it
/// (a tag written twice is decided by the first), and whether it had one. A tag of the tree at 0
/// is left out: on no base, it decides nothing.
fn spawn_on_bases(spawn: &[SpawnWeight]) -> (Vec<SpawnWeight>, bool) {
    let mut on_bases: Vec<SpawnWeight> = Vec::new();
    let mut genesis = false;
    for spawn_weight in spawn {
        if !GENESIS_TAGS.contains(&spawn_weight.tag.as_str()) {
            on_bases.push(spawn_weight.clone());
            continue;
        }
        if !spawn_weight.rolls() {
            continue;
        }
        genesis = true;
        for tag in GENESIS_BASES {
            if !on_bases.iter().any(|before| before.tag == tag) {
                on_bases.push(SpawnWeight {
                    tag: tag.to_owned(),
                    weight: spawn_weight.weight,
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
        .flat_map(|table| table.spawn.iter().map(|spawn_weight| &spawn_weight.tag))
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
