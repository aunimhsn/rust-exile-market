//! The modifier tables, from the RePoE fork (`mods.json`, `base_items.json` and
//! `stat_translations/stat_descriptions.json`): the tiers of each prefix and suffix that rolls
//! on the equipment, the jewels, the flasks and the charms, also those a desecration reveals
//! (the lords of the Abyss), those of the Genesis tree of the Breach and those only an essence
//! adds, and the tags of each base that tell where a modifier rolls.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde::{Deserialize, Serialize};

use crate::Result;

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

/// A modifier only an essence adds, by its id in `mods.json`, and the tags of the bases it rolls
/// on, as `(tag, weight)`.
pub type Essence = (&'static str, &'static [(&'static str, u32)]);

/// The modifiers only an essence adds, of the domain of the equipment, and the tags of the bases
/// each one rolls on, as `(tag, weight)`, `default` at 0 after them: RePoE writes these
/// modifiers with `default` at 0 alone, and doesn't export the essences of Path of Exile 2.
/// From Craft of Exile (`essences` of its `poec_data.json`, 2026-10-30: the item classes of each
/// modifier), checked on the trade site the same day (the bows take the values of the
/// one-handed weapons, `+2 to Level of all Attack Skills`; the `+3` of the bows of Standard is
/// that of an older patch). The ordinary essences add ordinary modifiers, with their tiers.
/// Left out: the notable of the Essence of Delirium on a body armour (`EssenceGrantedPassive`,
/// `Allocates #`), whose value is the hash of the notable, not a roll.
pub const ESSENCES: [Essence; 33] = [
    ("EssenceAbyssPrefix", &[("default", 1)]),
    ("EssenceAbyssSuffix", &[("default", 1)]),
    (
        "EssenceAttackSkillLevel1H1",
        &[("bow", 1), ("one_hand_weapon", 1)],
    ),
    (
        "EssenceAttackSkillLevel2H1",
        &[("bow", 0), ("two_hand_weapon", 1)],
    ),
    ("EssenceAuraEffect1", &[("sceptre", 1)]),
    ("EssenceBreach", &[("amulet", 1), ("ring", 1)]),
    ("EssenceColdRecoupLife1", &[("helmet", 1)]),
    ("EssenceCorruptForTwoEnchantments1", &[("belt", 1)]),
    (
        "EssenceDamageasExtraCold1",
        &[("bow", 1), ("one_hand_weapon", 1)],
    ),
    (
        "EssenceDamageasExtraCold2H",
        &[("bow", 0), ("two_hand_weapon", 1)],
    ),
    (
        "EssenceDamageasExtraFire1",
        &[("bow", 1), ("one_hand_weapon", 1)],
    ),
    (
        "EssenceDamageasExtraFire2H",
        &[("bow", 0), ("two_hand_weapon", 1)],
    ),
    (
        "EssenceDamageasExtraLightning1",
        &[("bow", 1), ("one_hand_weapon", 1)],
    ),
    (
        "EssenceDamageasExtraLightning2H",
        &[("bow", 0), ("two_hand_weapon", 1)],
    ),
    (
        "EssenceDamageasExtraPhysical1",
        &[("bow", 1), ("one_hand_weapon", 1)],
    ),
    (
        "EssenceDamageasExtraPhysical2H",
        &[("bow", 0), ("two_hand_weapon", 1)],
    ),
    ("EssenceFireRecoupLife1", &[("belt", 1)]),
    ("EssenceGlobalDefences1", &[("amulet", 1)]),
    ("EssenceGoldDropped1", &[("gloves", 1)]),
    ("EssenceIncreasedLifePercent1", &[("body_armour", 1)]),
    ("EssenceIncreasedManaPercent1", &[("ring", 1)]),
    ("EssenceLightningRecoupLife1", &[("gloves", 1)]),
    (
        "EssenceLocalRuneAndSoulCoreEffect1",
        &[("boots", 1), ("gloves", 1)],
    ),
    ("EssenceManaCostReduction", &[("focus", 1), ("wand", 1)]),
    ("EssenceManaCostReduction2H", &[("staff", 1)]),
    ("EssenceOnslaughtonKill1", &[("weapon", 1)]),
    ("EssencePercentDexterity1", &[("amulet", 1)]),
    ("EssencePercentIntelligence1", &[("amulet", 1)]),
    ("EssencePercentStrength1", &[("amulet", 1)]),
    ("EssencePhysicalDamageTakenAsChaos1", &[("body_armour", 1)]),
    (
        "EssenceReducedCriticalDamageAgainstYou1",
        &[("body_armour", 1)],
    ),
    ("EssenceSpellSkillLevel1H1", &[("wand", 1)]),
    ("EssenceSpellSkillLevel2H1", &[("staff", 1)]),
];

/// The domains of the bases kept, and of their modifiers: the equipment, the jewels, the flasks
/// and the charms; each one a domain the application knows (`Domain` in
/// `src/game_data/bases.rs`), as is [`DESECRATED`], the domain of modifiers only.
const DOMAINS: [&str; 3] = [EQUIPMENT, JEWELS, "flask"];

/// Whether `tag` is one of the Time-Lost jewels, whose modifiers the game writes within the
/// words of [`RADIUS_GRANTS`].
fn radius_jewel(tag: &str) -> bool {
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
#[derive(Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
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
    /// `default`, rolls on the bases of its domain only (the charges of the flasks); a
    /// desecrated modifier rolls on the equipment and the jewels.
    #[serde(skip_serializing_if = "is_item")]
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
enum Source {
    /// The Genesis tree of the Breach, on the rings, the amulets and the belts it grows.
    Genesis,
    /// An essence ([`ESSENCES`]).
    Essence,
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

/// Whether `domain` is one of the bases kept, which the application knows.
pub fn is_kept(domain: &str) -> bool {
    DOMAINS.contains(&domain)
}

/// Whether a modifier of `domain` rolls on the bases of `base_domain`: those of its domain, the
/// equipment and the jewels for a desecrated one (`Domain::rolls_on` in
/// `src/game_data/bases.rs`).
fn rolls_on(domain: &str, base_domain: &str) -> bool {
    domain == base_domain || (domain == DESECRATED && [EQUIPMENT, JEWELS].contains(&base_domain))
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
pub type BaseTags = BTreeMap<String, BaseSpawn>;

/// The English template of Exiled Exchange 2 (its `ref`) of each of its English strings, and of
/// each template itself: `Loads # additional bolts` is `Loads an additional bolt`.
pub type Templates = BTreeMap<String, String>;

/// The modifier tables of `modifiers`, those of `essences` among them ([`ESSENCES`]), the lines
/// of their stats written as the English templates of Exiled Exchange 2, and the tags of the
/// bases that the tables read, by English name of the base.
pub fn reduce(
    modifiers: &BTreeMap<String, SourceModifier>,
    bases: &BTreeMap<String, SourceBase>,
    translations: &[Translation],
    templates: &Templates,
    essences: &[Essence],
) -> Result<(Vec<Modifier>, BaseTags)> {
    let mut by_stat: HashMap<&str, usize> = HashMap::new();
    for (index, translation) in translations.iter().enumerate() {
        for id in &translation.ids {
            by_stat.entry(id.as_str()).or_insert(index);
        }
    }
    let essences = essence_spawns(modifiers, essences)?;
    let mut grouped: BTreeMap<Key, Vec<Tier>> = BTreeMap::new();
    for (id, modifier) in modifiers {
        let essence = essences.get(id.as_str());
        let spawn_weights = essence.unwrap_or(&modifier.spawn_weights);
        let rolls = (is_kept(&modifier.domain) || modifier.domain == DESECRATED)
            && ["prefix", "suffix"].contains(&modifier.generation_type.as_str())
            && (essence.is_some() || !modifier.is_essence_only)
            && spawn_weights.iter().any(|spawn| spawn.weight > 0);
        if !rolls {
            continue;
        }
        // Its lines would be written within the words of the passive skills on the other bases.
        if on_radius_jewel(modifier)
            && modifier
                .spawn_weights
                .iter()
                .any(|spawn| spawn.weight > 0 && !radius_jewel(&spawn.tag))
        {
            return Err(format!(
                "{id}: a modifier of the Time-Lost jewels that rolls on other bases"
            )
            .into());
        }
        let (lines, ranges) = lines(modifier, translations, &by_stat, templates)
            .map_err(|error| format!("{id}: {error}"))?;
        let (spawn, genesis) = spawn_on_bases(spawn_weights);
        let source = if essence.is_some() {
            Some(Source::Essence)
        } else {
            genesis.then_some(Source::Genesis)
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
                level: modifier.required_level,
                ranges,
            });
    }

    let released: Vec<&SourceBase> = bases
        .values()
        .filter(|base| base.release_state == "released")
        .collect();
    let mut tables = Vec::new();
    let mut unknown_lines = BTreeSet::new();
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
        // A line without template can't be found, nor its modifier.
        let unknown: Vec<&Line> = lines.iter().filter(|line| !line.known).collect();
        if !unknown.is_empty() {
            unknown_lines.extend(unknown.iter().map(|line| line.template.clone()));
            continue;
        }
        // By level, then by ranges: of two tiers of one level, the higher ranges last, the better
        // tier for the application (`Upgrades Radius to Large` after `to Medium`, both of level
        // 1: its T1, as the game writes it).
        tiers.sort();
        tables.push(Modifier {
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
        println!("Modifier line without template, its modifier left out: {line}");
    }
    // The ordinary tables first, then those of the Genesis tree and of the essences, then the
    // desecrated ones: of two tables that hold a copied modifier as well, the application takes
    // the first of the file, the ordinary one (`+2 to Level of all Minion Skills` of an amulet, a
    // T2 of its own or a minion modifier of the Genesis tree).
    tables.sort_by_key(|table| (table.domain == DESECRATED, table.source));
    let base_tags = base_tags(&released, &tables);
    Ok((tables, base_tags))
}

/// The spawn weights of each modifier of `essences` by its id, `default` at 0 after its own
/// tags. An error when one isn't a prefix or a suffix of the equipment among `modifiers`, or
/// rolls without an essence: the list is to be checked again, after a patch of the game.
fn essence_spawns(
    modifiers: &BTreeMap<String, SourceModifier>,
    essences: &[Essence],
) -> Result<HashMap<&'static str, Vec<SpawnWeight>>> {
    let mut spawns = HashMap::new();
    for &(id, tags) in essences {
        let modifier = modifiers
            .get(id)
            .ok_or_else(|| format!("{id}: a modifier of an essence that RePoE doesn't have"))?;
        if modifier.domain != EQUIPMENT
            || !["prefix", "suffix"].contains(&modifier.generation_type.as_str())
            || modifier.spawn_weights.iter().any(|spawn| spawn.weight > 0)
        {
            return Err(format!(
                "{id}: a modifier of an essence that isn't one of the equipment rolled by an \
                 essence only"
            )
            .into());
        }
        let weight = |(tag, weight): (&str, u32)| SpawnWeight {
            tag: tag.to_owned(),
            weight,
        };
        // Every base for a modifier of `default` already.
        let default = tags
            .iter()
            .all(|&(tag, _)| tag != "default")
            .then_some(("default", 0));
        let spawn = tags.iter().copied().chain(default).map(weight).collect();
        spawns.insert(id, spawn);
    }
    Ok(spawns)
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
            // A stat the game doesn't write, e.g. `dummy_stat_display_nothing`; one that rolls
            // is written by a translation RePoE doesn't have (`minion_area_of_effect_+%` of a
            // hybrid of the Abyss on the jewels), whose line can't be found.
            if stat.min != stat.max {
                lines.push(Line {
                    template: stat.id.clone(),
                    divisor: 1,
                    known: false,
                });
                ranges.push(Vec::new());
            }
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
    let Some(template) = known_template(modifier, &strings, templates)?.cloned() else {
        // Without template, the line can't be found: written as the game writes it, its string
        // with the most numbers; one of the Time-Lost jewels, which the game writes within the
        // words of the passive skills that grant it, saying so.
        let jewel = if on_radius_jewel(modifier) {
            " (Time-Lost jewels: not known within the words of the passive skills, with its number)"
        } else {
            ""
        };
        let line = Line {
            template: format!("{}{jewel}", template(&general.string)),
            divisor: 1,
            known: false,
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
    let without_number: Vec<String> = translation
        .english
        .iter()
        .filter(|variant| placeholders(&variant.string).is_empty())
        .map(|variant| self::template(&variant.string))
        .collect();
    let written = known_template(modifier, &without_number, templates)?
        .is_some_and(|known| *known == template);
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

/// The template of Exiled Exchange 2 of a line of `modifier` written one of `strings` (templates
/// of the game, [`template`]), the first it knows. On a Time-Lost jewel, the line within the
/// words of [`RADIUS_GRANTS`] of which it knows one kind only (`Small Passive Skills in Radius
/// also grant #% increased Elemental Damage`), with its number when a string writes one: known
/// without its number only (`Small Passive Skills in Radius also grant Attack Hits apply
/// Incision`, while the game writes `… 4% chance for Attack Hits to apply Incision`, seen on the
/// trade site on 2026-10-30), the line can't be found, and has none. A line about the radius
/// itself ([`RADIUS_LINES`]) is written alone; another line known within neither has none. Of
/// a desecrated modifier, which it knows within both (`#% increased maximum Life`), the
/// notables: the trade site has a desecrated stat for them only, as for every other desecrated
/// modifier of these jewels (checked on 2026-10-30).
fn known_template<'a>(
    modifier: &SourceModifier,
    strings: &[String],
    templates: &'a Templates,
) -> Result<Option<&'a String>> {
    let alone = || strings.iter().find_map(|string| templates.get(string));
    if !on_radius_jewel(modifier) {
        return Ok(alone());
    }
    let within = |words: &str, string: &String| templates.get(&format!("{words}{string}"));
    let granted: Vec<&str> = RADIUS_GRANTS
        .into_iter()
        .filter(|words| strings.iter().any(|string| within(words, string).is_some()))
        .collect();
    let words = match granted.as_slice() {
        [] => {
            return Ok(alone().filter(|template| RADIUS_LINES.contains(&template.as_str())));
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
        .find_map(|string| within(words, string)))
}

/// Whether `modifier` rolls on a Time-Lost jewel.
fn on_radius_jewel(modifier: &SourceModifier) -> bool {
    modifier
        .spawn_weights
        .iter()
        .any(|spawn| spawn.weight > 0 && radius_jewel(&spawn.tag))
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
            &[],
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

    #[test]
    fn a_desecrated_table_rolls_on_the_equipment_and_the_jewels_after_the_ordinary_ones() {
        let bases = r#"{
            "Ring": {"domain": "item", "name": "Ring", "release_state": "released", "tags": ["ring", "default"]},
            "Ruby": {"domain": "misc", "name": "Ruby", "release_state": "released", "tags": ["strjewel", "default"]},
            "Waystone": {"domain": "area", "name": "Waystone", "release_state": "released", "tags": ["map", "default"]}
        }"#;
        let desecrated = |spawn: &str| {
            modifier(
                "Leech",
                65,
                spawn,
                r#"[{"id": "life_leech_+%", "min": 12, "max": 20}]"#,
            )
            .replace(r#""domain": "item""#, r#""domain": "desecrated""#)
        };
        let modifiers = format!(
            // The tag of a lord comes after `default`, on no base.
            r#"{{"Abyss": {}, "Lightless": {}, "MapAbyss": {}, "Ordinary": {}}}"#,
            desecrated(
                r#"[{"tag": "ring", "weight": 1}, {"tag": "default", "weight": 0}, {"tag": "kurgal_mod", "weight": 1}]"#
            ),
            // A tag of a lord at 0 names no lord.
            desecrated(
                r#"[{"tag": "strjewel", "weight": 1}, {"tag": "default", "weight": 0}, {"tag": "ulaman_mod", "weight": 0}]"#
            ),
            desecrated(r#"[{"tag": "map", "weight": 1}, {"tag": "default", "weight": 0}]"#),
            modifier(
                "Leech",
                1,
                r#"[{"tag": "ring", "weight": 1}, {"tag": "default", "weight": 0}]"#,
                r#"[{"id": "life_leech_+%", "min": 5, "max": 10}]"#
            ),
        );
        let translations = r#"[{"ids": ["life_leech_+%"], "English": [{"string": "{0}% increased amount of Life Leeched", "index_handlers": [[]]}], "hidden": null}]"#;

        let (tables, tags) = tables(
            &modifiers,
            bases,
            translations,
            &[(
                "#% increased amount of Life Leeched",
                "#% increased amount of Life Leeched",
            )],
        );

        let spawns: Vec<_> = tables
            .as_array()
            .expect("tables")
            .iter()
            .map(|table| (table.get("domain").cloned(), table["spawn"].clone()))
            .collect();
        let lords: Vec<_> = tables
            .as_array()
            .expect("tables")
            .iter()
            .map(|table| table.get("lord").cloned())
            .collect();
        // The name of the lord, of the first table only.
        assert_eq!(lords, [None, Some(serde_json::json!("Kurgal")), None]);
        let desecrated = Some(serde_json::json!("desecrated"));
        // Without the tag of the lord, nor the table of the waystones, a base of another
        // domain.
        assert_eq!(
            spawns,
            [
                (None, serde_json::json!([["ring", 1], ["default", 0]])),
                (
                    desecrated.clone(),
                    serde_json::json!([["ring", 1], ["default", 0]])
                ),
                (
                    desecrated,
                    serde_json::json!([["strjewel", 1], ["default", 0]])
                ),
            ]
        );
        assert_eq!(
            tags.get("Ruby").map(|spawn| spawn.tags.clone()),
            Some(vec!["strjewel".to_owned(), "default".to_owned()])
        );
    }

    #[test]
    fn a_modifier_of_the_genesis_tree_rolls_on_the_rings_the_amulets_and_the_belts_it_grows() {
        let bases = r#"{
            "Ring": {"domain": "item", "name": "Ring", "release_state": "released", "tags": ["ring", "default"]},
            "Amulet": {"domain": "item", "name": "Amulet", "release_state": "released", "tags": ["amulet", "default"]},
            "Belt": {"domain": "item", "name": "Belt", "release_state": "released", "tags": ["belt", "default"]}
        }"#;
        let life = |level: u32, min: u32, max: u32, spawn: &str| {
            modifier(
                "MinionLife",
                level,
                spawn,
                &format!(r#"[{{"id": "minion_maximum_life_+%", "min": {min}, "max": {max}}}]"#),
            )
        };
        let tree =
            r#"[{"tag": "genesis_tree_minion", "weight": 1}, {"tag": "default", "weight": 0}]"#;
        let modifiers = format!(
            r#"{{"MinionLifeRing2": {}, "MinionLifeRing1": {}, "Amulet": {}, "Exposure": {}, "Sceptre": {}}}"#,
            life(29, 11, 14, tree),
            life(18, 7, 10, tree),
            // Ruled out of the rings and the belts before the tag of the tree.
            modifier(
                "MinionLife",
                1,
                r#"[{"tag": "ring", "weight": 0}, {"tag": "belt", "weight": 0}, {"tag": "genesis_tree_minion", "weight": 1}, {"tag": "default", "weight": 0}]"#,
                r#"[{"id": "minion_maximum_life_+%", "min": 1, "max": 2}]"#
            ),
            modifier(
                "Exposure",
                1,
                r#"[{"tag": "amulet", "weight": 0}, {"tag": "belt", "weight": 0}, {"tag": "breach_desecration", "weight": 1}, {"tag": "default", "weight": 0}]"#,
                r#"[{"id": "exposure_effect_+%", "min": 25, "max": 35}]"#
            )
            .replace(r#""domain": "item""#, r#""domain": "desecrated""#),
            // An ordinary one, with a tag of the tree at 0 that decides nothing.
            life(
                2,
                21,
                25,
                r#"[{"tag": "genesis_tree_minion", "weight": 0}, {"tag": "ring", "weight": 1}, {"tag": "default", "weight": 0}]"#
            ),
        );
        let translations = r#"[
            {"ids": ["minion_maximum_life_+%"], "English": [{"string": "Minions have {0}% increased maximum Life", "index_handlers": [[]]}], "hidden": null},
            {"ids": ["exposure_effect_+%"], "English": [{"string": "{0}% increased Exposure Effect", "index_handlers": [[]]}], "hidden": null}
        ]"#;

        let (tables, tags) = tables(
            &modifiers,
            bases,
            translations,
            &[
                (
                    "Minions have #% increased maximum Life",
                    "Minions have #% increased maximum Life",
                ),
                (
                    "#% increased Exposure Effect",
                    "#% increased Exposure Effect",
                ),
            ],
        );

        let summary: Vec<_> = tables
            .as_array()
            .expect("tables")
            .iter()
            .map(|table| {
                let levels: Vec<_> = table["tiers"]
                    .as_array()
                    .expect("tiers")
                    .iter()
                    .map(|tier| tier["level"].clone())
                    .collect();
                serde_json::json!([table["type"], table["spawn"], table.get("source"), levels])
            })
            .collect();
        // The ordinary table first, then those of the tree, the desecrated one last; the tiers
        // of the tree on the same bases one table.
        assert_eq!(
            summary,
            [
                serde_json::json!(["MinionLife", [["ring", 1], ["default", 0]], null, [2]]),
                serde_json::json!([
                    "MinionLife",
                    [["ring", 0], ["belt", 0], ["amulet", 1], ["default", 0]],
                    "genesis",
                    [1]
                ]),
                serde_json::json!([
                    "MinionLife",
                    [["ring", 1], ["amulet", 1], ["belt", 1], ["default", 0]],
                    "genesis",
                    [18, 29]
                ]),
                serde_json::json!([
                    "Exposure",
                    [["amulet", 0], ["belt", 0], ["ring", 1], ["default", 0]],
                    "genesis",
                    [1]
                ]),
            ]
        );
        assert_eq!(
            tags.get("Belt").map(|spawn| spawn.tags.clone()),
            Some(vec!["belt".to_owned(), "default".to_owned()])
        );
    }

    #[test]
    fn the_tags_of_the_genesis_tree_give_way_to_the_bases_it_grows_the_first_tag_deciding() {
        let spawn = |tags: &[(&str, u32)]| -> Vec<SpawnWeight> {
            tags.iter()
                .map(|&(tag, weight)| SpawnWeight {
                    tag: tag.to_owned(),
                    weight,
                })
                .collect()
        };

        // A tag of a base after the tag of the tree: the tree's weight, written first, decides;
        // the second tag of the tree adds no base twice.
        assert_eq!(
            spawn_on_bases(&spawn(&[
                ("breach_desecration", 1),
                ("ring", 0),
                ("genesis_tree_minion", 2),
                ("default", 0),
            ])),
            (
                spawn(&[
                    ("ring", 1),
                    ("amulet", 1),
                    ("belt", 1),
                    ("ring", 0),
                    ("default", 0),
                ]),
                true
            )
        );
        // A tag of the tree at 0 left out, the others as they are.
        let ordinary = spawn(&[("genesis_tree_minion", 0), ("ring", 1), ("default", 0)]);
        assert_eq!(
            spawn_on_bases(&ordinary),
            (spawn(&[("ring", 1), ("default", 0)]), false)
        );
    }

    /// The tables from `modifiers` of `mods.json` on the bases of [`BASES`], those of `essences`
    /// among them, each with a line `{0} to Level of all Spell Skills`.
    fn essence_tables(modifiers: &str, essences: &[Essence]) -> Result<serde_json::Value> {
        let translations = r#"[{"ids": ["spell_skill_gem_level_+"], "English": [{"string": "{0:+d} to Level of all Spell Skills", "index_handlers": [[]]}], "hidden": null}]"#;
        let template = "# to Level of all Spell Skills";
        let (tables, _) = reduce(
            &serde_json::from_str(modifiers).expect("modifiers"),
            &serde_json::from_str(BASES).expect("bases"),
            &serde_json::from_str::<Vec<_>>(translations).expect("translations"),
            &[(template.to_owned(), template.to_owned())].into(),
            essences,
        )?;
        Ok(serde_json::to_value(tables).expect("written"))
    }

    /// A modifier of `kind` of `+value to Level of all Spell Skills`, rolling where `spawn`
    /// says.
    fn spell_levels(kind: &str, level: u32, spawn: &str, value: u32) -> String {
        modifier(
            kind,
            level,
            spawn,
            &format!(r#"[{{"id": "spell_skill_gem_level_+", "min": {value}, "max": {value}}}]"#),
        )
    }

    #[test]
    fn a_modifier_of_an_essence_rolls_on_the_bases_of_its_list_in_a_table_of_its_own() {
        let nowhere = r#"[{"tag": "default", "weight": 0}]"#;
        let modifiers = format!(
            r#"{{"Ordinary1": {}, "Ordinary2": {}, "Essence": {}, "Unlisted": {}}}"#,
            spell_levels(
                "SpellLevel",
                1,
                r#"[{"tag": "bow", "weight": 1}, {"tag": "default", "weight": 0}]"#,
                2
            ),
            spell_levels(
                "SpellLevel",
                40,
                r#"[{"tag": "bow", "weight": 1}, {"tag": "default", "weight": 0}]"#,
                3
            ),
            // Of the same type and value as the ordinary T1, with `default` at 0 alone.
            spell_levels("SpellLevel", 72, nowhere, 3),
            // Another one of the files, which the list doesn't name: on no base.
            spell_levels("Other", 72, nowhere, 5),
        );

        let tables = essence_tables(&modifiers, &[("Essence", &[("bow", 1)])]).expect("tables");

        let summary: Vec<_> = tables
            .as_array()
            .expect("tables")
            .iter()
            .map(|table| {
                let levels: Vec<_> = table["tiers"]
                    .as_array()
                    .expect("tiers")
                    .iter()
                    .map(|tier| tier["level"].clone())
                    .collect();
                serde_json::json!([table["type"], table["spawn"], table.get("source"), levels])
            })
            .collect();
        // The ordinary table first, the essence's apart, after it.
        assert_eq!(
            summary,
            [
                serde_json::json!(["SpellLevel", [["bow", 1], ["default", 0]], null, [1, 40]]),
                serde_json::json!(["SpellLevel", [["bow", 1], ["default", 0]], "essence", [72]]),
            ]
        );
    }

    #[test]
    fn a_modifier_of_an_essence_missing_or_rolling_without_it_stops_the_generator() {
        let nowhere = r#"[{"tag": "default", "weight": 0}]"#;
        let missing = essence_tables(
            &format!(r#"{{"Essence": {}}}"#, spell_levels("Kind", 72, nowhere, 3)),
            &[("Renamed", &[("bow", 1)])],
        );
        assert_eq!(
            missing.err().map(|error| error.to_string()),
            Some("Renamed: a modifier of an essence that RePoE doesn't have".to_owned())
        );

        // Rolling on a base of its own after a patch: the list is to be checked again.
        let rolling = essence_tables(
            &format!(
                r#"{{"Essence": {}}}"#,
                spell_levels(
                    "Kind",
                    72,
                    r#"[{"tag": "bow", "weight": 1}, {"tag": "default", "weight": 0}]"#,
                    3
                )
            ),
            &[("Essence", &[("bow", 1)])],
        );
        assert_eq!(
            rolling.err().map(|error| error.to_string()),
            Some(
                "Essence: a modifier of an essence that isn't one of the equipment rolled by an \
                 essence only"
                    .to_owned()
            )
        );
    }

    #[test]
    fn the_modifiers_of_the_essences_roll_on_the_bases_their_tags_say() {
        // The tags of the bases of Path of Exile 2, as `bases-en.ndjson` writes them.
        let bow = ["bow", "ranged", "two_hand_weapon", "weapon", "default"];
        let crossbow = [
            "cannon",
            "crossbow",
            "ranged",
            "two_hand_weapon",
            "weapon",
            "default",
        ];
        let spear = ["one_hand_weapon", "spear", "weapon", "default"];
        let wand = ["wand", "default"];
        let spawn = |id: &str| {
            ESSENCES
                .iter()
                .find(|&&(essence, _)| essence == id)
                .map(|&(_, tags)| tags)
                .expect("an essence")
        };
        let rolls = |id: &str, base: &[&str]| {
            spawn(id)
                .iter()
                .find(|(tag, _)| base.contains(tag))
                .is_some_and(|&(_, weight)| weight > 0)
        };

        // A bow takes the values of the one-handed weapons (trade site, 2026-10-30).
        assert!(rolls("EssenceAttackSkillLevel1H1", &bow));
        assert!(!rolls("EssenceAttackSkillLevel2H1", &bow));
        assert!(rolls("EssenceAttackSkillLevel1H1", &spear));
        assert!(rolls("EssenceAttackSkillLevel2H1", &crossbow));
        assert!(!rolls("EssenceAttackSkillLevel1H1", &crossbow));
        // A martial weapon only.
        assert!(rolls("EssenceOnslaughtonKill1", &bow));
        assert!(!rolls("EssenceOnslaughtonKill1", &wand));
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
            r#"{{"Kept": {kept}, "Monster": {}, "Unique": {}, "Essence": {}, "Nowhere": {}}}"#,
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
            &[],
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
    fn a_modifier_with_a_stat_that_rolls_without_translation_is_left_out() {
        let ring = r#"[{"tag": "ring", "weight": 1}]"#;
        let modifiers = format!(
            r#"{{"Hybrid": {}, "Flag": {}}}"#,
            modifier(
                "MinionAreaAndLife",
                1,
                ring,
                r#"[{"id": "minion_area_of_effect_+%", "min": 3, "max": 6}, {"id": "minion_maximum_life_+%", "min": 4, "max": 8}]"#
            ),
            modifier(
                "MinionLife",
                1,
                ring,
                r#"[{"id": "dummy_stat_display_nothing", "min": 1, "max": 1}, {"id": "minion_maximum_life_+%", "min": 5, "max": 15}]"#
            ),
        );
        let translations = r#"[{"ids": ["minion_maximum_life_+%"], "English": [{"string": "Minions have {0}% increased maximum Life", "index_handlers": [[]]}], "hidden": null}]"#;

        let (tables, _) = tables(
            &modifiers,
            BASES,
            translations,
            &[(
                "Minions have #% increased maximum Life",
                "Minions have #% increased maximum Life",
            )],
        );

        // A stat without translation that doesn't roll is left out of its lines only.
        let types: Vec<_> = tables
            .as_array()
            .expect("tables")
            .iter()
            .map(|table| table["type"].clone())
            .collect();
        assert_eq!(types, [serde_json::json!("MinionLife")]);
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
            &[],
        )
        .err()
        .map(|error| error.to_string());

        assert_eq!(
            error.as_deref(),
            Some("Thorns: {0} to {1} Thorns damage has several values and a string without number")
        );
    }

    /// The templates of the lines of the tables of the Time-Lost modifiers of `stats` (each
    /// written `{0}% increased <stat>`), desecrated or not, on a Time-Lost Ruby, Exiled Exchange 2
    /// knowing `templates`.
    fn time_lost_templates(
        stats: &[&str],
        desecrated: bool,
        templates: &[&str],
    ) -> Result<Vec<serde_json::Value>> {
        let spawn =
            r#"[{"tag": "str_radius_jewel", "weight": 1}, {"tag": "default", "weight": 0}]"#;
        time_lost_tables(stats, spawn, desecrated, templates)
    }

    /// [`time_lost_templates`] with the modifiers rolling where `spawn` says.
    fn time_lost_tables(
        stats: &[&str],
        spawn: &str,
        desecrated: bool,
        templates: &[&str],
    ) -> Result<Vec<serde_json::Value>> {
        let bases = r#"{"TimeLostRuby": {"domain": "misc", "name": "Time-Lost Ruby", "release_state": "released", "tags": ["jewel", "str_radius_jewel", "radius_jewel", "default"]}}"#;
        let modifiers: Vec<String> = stats
            .iter()
            .map(|stat| {
                let modifier = modifier(
                    stat,
                    1,
                    spawn,
                    &format!(r#"[{{"id": "{stat}", "min": 1, "max": 2}}]"#),
                )
                .replace(
                    r#""domain": "item""#,
                    if desecrated {
                        r#""domain": "desecrated""#
                    } else {
                        r#""domain": "misc""#
                    },
                );
                format!(r#""{stat}": {modifier}"#)
            })
            .collect();
        let translations: Vec<String> = stats
            .iter()
            .map(|stat| {
                format!(
                    r#"{{"ids": ["{stat}"], "English": [{{"string": "{{0}}% increased {stat}", "index_handlers": [[]]}}], "hidden": null}}"#
                )
            })
            .collect();
        let (tables, _) = reduce(
            &serde_json::from_str(&format!("{{{}}}", modifiers.join(", "))).expect("modifiers"),
            &serde_json::from_str(bases).expect("bases"),
            &serde_json::from_str::<Vec<_>>(&format!("[{}]", translations.join(", ")))
                .expect("translations"),
            &templates
                .iter()
                .map(|&template| (template.to_owned(), template.to_owned()))
                .collect(),
            &[],
        )?;
        Ok(serde_json::to_value(tables)
            .expect("written")
            .as_array()
            .expect("tables")
            .iter()
            .map(|table| table["lines"][0]["template"].clone())
            .collect())
    }

    #[test]
    fn a_line_of_a_time_lost_jewel_is_written_within_the_words_of_the_passive_skills_it_grants() {
        // The small passive skills or the notables, whichever Exiled Exchange 2 knows, also when
        // it knows the line alone; a line about the radius itself alone when it knows it only,
        // another line left out (`Light Radius`, an ordinary modifier of other bases).
        let templates = time_lost_templates(
            &[
                "Armour",
                "Evasion",
                "Effect of Small Passive Skills in Radius",
                "Life",
                "Light Radius",
            ],
            false,
            &[
                "#% increased Armour",
                "Small Passive Skills in Radius also grant #% increased Armour",
                "Notable Passive Skills in Radius also grant #% increased Evasion",
                "#% increased Effect of Small Passive Skills in Radius",
                "#% increased Life",
                "#% increased Light Radius",
            ],
        )
        .expect("tables");

        assert_eq!(
            templates,
            [
                "Small Passive Skills in Radius also grant #% increased Armour",
                "#% increased Effect of Small Passive Skills in Radius",
                "Notable Passive Skills in Radius also grant #% increased Evasion",
            ]
        );
    }

    #[test]
    fn a_line_of_a_time_lost_jewel_known_without_its_number_only_is_left_out() {
        // The game writes `… 4% chance for Attack Hits to apply Incision`, which Exiled Exchange 2
        // knows alone only, within the words without number (`value` 100).
        let modifiers = format!(
            r#"{{"Incision": {}}}"#,
            modifier(
                "Incision",
                1,
                r#"[{"tag": "str_radius_jewel", "weight": 1}, {"tag": "default", "weight": 0}]"#,
                r#"[{"id": "incision_chance", "min": 3, "max": 5}]"#
            )
            .replace(r#""domain": "item""#, r#""domain": "misc""#)
        );
        let bases = r#"{"TimeLostRuby": {"domain": "misc", "name": "Time-Lost Ruby", "release_state": "released", "tags": ["str_radius_jewel", "radius_jewel", "default"]}}"#;
        let translations = r#"[{"ids": ["incision_chance"], "English": [
            {"string": "Attack Hits apply Incision", "index_handlers": [[]]},
            {"string": "{0}% chance for Attack Hits to apply Incision", "index_handlers": [[]]}
        ], "hidden": null}]"#;

        let (tables, _) = tables(
            &modifiers,
            bases,
            translations,
            &[
                ("Attack Hits apply Incision", "Attack Hits apply Incision"),
                (
                    "#% chance for Attack Hits to apply Incision",
                    "Attack Hits apply Incision",
                ),
                (
                    "Small Passive Skills in Radius also grant Attack Hits apply Incision",
                    "Small Passive Skills in Radius also grant Attack Hits apply Incision",
                ),
            ],
        );

        assert_eq!(tables, serde_json::json!([]));
    }

    #[test]
    fn a_modifier_of_the_time_lost_jewels_rolling_on_other_bases_stops_the_generator() {
        let spawn = r#"[{"tag": "str_radius_jewel", "weight": 1}, {"tag": "ring", "weight": 1}, {"tag": "default", "weight": 0}]"#;

        let error = time_lost_tables(
            &["Armour"],
            spawn,
            false,
            &["Small Passive Skills in Radius also grant #% increased Armour"],
        )
        .err()
        .map(|error| error.to_string());

        assert_eq!(
            error.as_deref(),
            Some("Armour: a modifier of the Time-Lost jewels that rolls on other bases")
        );
    }

    #[test]
    fn a_line_of_a_time_lost_jewel_known_for_both_kinds_of_passive_skills_is_a_notable_if_desecrated()
     {
        let both = [
            "Small Passive Skills in Radius also grant #% increased Life",
            "Notable Passive Skills in Radius also grant #% increased Life",
        ];

        let desecrated = time_lost_templates(&["Life"], true, &both).expect("tables");
        let ordinary = time_lost_templates(&["Life"], false, &both)
            .err()
            .map(|error| error.to_string());

        assert_eq!(
            desecrated,
            ["Notable Passive Skills in Radius also grant #% increased Life"]
        );
        assert_eq!(
            ordinary.as_deref(),
            Some(
                "Life: #% increased Life is known within the words of both kinds of passive skills"
            )
        );
    }

    #[test]
    fn a_line_of_a_time_lost_jewel_known_for_both_kinds_by_two_strings_stops_the_generator() {
        // Its number under the small passive skills, its string without number under the
        // notables.
        let modifiers = format!(
            r#"{{"Daze": {}}}"#,
            modifier(
                "Daze",
                1,
                r#"[{"tag": "str_radius_jewel", "weight": 1}, {"tag": "default", "weight": 0}]"#,
                r#"[{"id": "daze_chance", "min": 1, "max": 2}]"#
            )
            .replace(r#""domain": "item""#, r#""domain": "misc""#)
        );
        let bases = r#"{"TimeLostRuby": {"domain": "misc", "name": "Time-Lost Ruby", "release_state": "released", "tags": ["str_radius_jewel", "radius_jewel", "default"]}}"#;
        let translations = r#"[{"ids": ["daze_chance"], "English": [
            {"string": "Dazes on Hit", "index_handlers": [[]]},
            {"string": "{0}% chance to Daze on Hit", "index_handlers": [[]]}
        ], "hidden": null}]"#;
        let templates: Templates = [
            "Small Passive Skills in Radius also grant #% chance to Daze on Hit",
            "Notable Passive Skills in Radius also grant Dazes on Hit",
        ]
        .into_iter()
        .map(|template| (template.to_owned(), template.to_owned()))
        .collect();

        let error = reduce(
            &serde_json::from_str(&modifiers).expect("modifiers"),
            &serde_json::from_str(bases).expect("bases"),
            &serde_json::from_str::<Vec<_>>(translations).expect("translations"),
            &templates,
            &[],
        )
        .err()
        .map(|error| error.to_string());

        assert_eq!(
            error.as_deref(),
            Some(
                "Daze: #% chance to Daze on Hit is known within the words of both kinds of passive skills"
            )
        );
    }

    #[test]
    fn a_string_without_number_of_another_template_gives_no_divisor() {
        // `Rage loss starts 1 second later` is the string of another stat: the line of
        // `Rage loss starts # seconds later` keeps its values as they are shown, undivided.
        let delay = one_table(
            &[r#"[{"id": "rage_delay_ms", "min": 1000, "max": 2000}]"#],
            r#"{"ids": ["rage_delay_ms"], "English": [
                {"string": "Rage loss starts {0} seconds later", "index_handlers": [["milliseconds_to_seconds"]]},
                {"string": "Rage loss starts 1 second later", "index_handlers": [["milliseconds_to_seconds"]]}
            ], "hidden": null}"#,
            &[
                (
                    "Rage loss starts # seconds later",
                    "Rage loss starts # seconds later",
                ),
                (
                    "Rage loss starts 1 second later",
                    "Inherent Rage loss starts 1 second later",
                ),
            ],
        );

        assert_eq!(
            delay,
            serde_json::json!([
                [{"template": "Rage loss starts # seconds later"}],
                [{"level": 0, "ranges": [[[1, 2]]]}]
            ])
        );
    }
}
