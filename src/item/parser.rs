//! Reads a copied item, in four phases: recognize each section from its first line, decide
//! what the sections without label are, fill the item, then join the lines of one stat
//! written without header.
//!
//! A copied item is a list of sections separated by `--------`: the header (class, rarity,
//! names), the properties, the requirements, the sockets, the item level, the enchantments,
//! the runes, the implicit modifiers, the granted skills, the explicit modifiers, the states,
//! and texts that aren't read (flavour text, descriptions, note).

use std::fmt;

use super::labels::{Labels, marker};
use super::modifier::{Header, Marker, Modifier, ModifierKind};
use super::weapon::{self, DamageKind, Weapon};
use super::{CopiedItem, Item, ItemState, Property, QualityKind, Rarity, Requirements, Stack};
use crate::game_data;
use crate::language::Language;

const SECTION_SEPARATOR: &str = "--------";

/// Why a copied item couldn't be read at all. It never holds the text of the item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    MissingRarity,
    UnknownRarity,
    MissingBaseType,
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::MissingRarity => "no rarity line",
            Self::UnknownRarity => "unknown rarity",
            Self::MissingBaseType => "no base type",
        })
    }
}

impl std::error::Error for ParseError {}

/// A part of the item that couldn't be read: the item is read without it. It never holds the
/// text of the item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseWarning {
    /// A line whose value can't be read: not a number where one is expected, or an
    /// attribute of the requirements that isn't in the labels.
    InvalidValue(Field),
    /// A modifier header whose kind isn't in the labels: the modifier is kept without its
    /// kind, name nor tier.
    UnknownModifierHeader,
    /// Modifier lines before the first header of their section: they are kept as a
    /// modifier without kind, name nor tier.
    ModifierWithoutHeader,
    /// The quality of a catalyst of a kind not in the labels: it is kept with the words of
    /// the game.
    UnknownQualityKind,
}

/// A line of the item with a value to read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    ItemLevel,
    Stack,
    Quality,
    Requirements,
}

impl fmt::Display for Field {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ItemLevel => "item level",
            Self::Stack => "stack size",
            Self::Quality => "quality",
            Self::Requirements => "requirements",
        })
    }
}

impl fmt::Display for ParseWarning {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidValue(field) => write!(formatter, "invalid {field}"),
            Self::UnknownModifierHeader => formatter.write_str("unknown modifier header"),
            Self::ModifierWithoutHeader => formatter.write_str("modifier lines without header"),
            Self::UnknownQualityKind => formatter.write_str("unknown kind of quality"),
        }
    }
}

/// A section after phase 1: recognized from its lines, or plain lines without label nor
/// marker, which phase 2 resolves.
enum Recognized<'a> {
    Section(Section<'a>),
    Plain(Vec<&'a str>),
}

/// A section of the item, once known.
#[derive(Debug)]
enum Section<'a> {
    Properties(Vec<&'a str>),
    Requirements(&'a str),
    Sockets(&'a str),
    ItemLevel(&'a str),
    Stack(&'a str),
    GrantedSkills(Vec<&'a str>),
    States(Vec<ItemState>),
    /// Modifiers with the headers of the advanced copy.
    Headed(Vec<&'a str>),
    /// Modifiers without header, all of one kind, which their marker tells (` (rune)`…) or
    /// phase 2 finds; each line of the origin its own marker tells (` (fractured)`…).
    Marked(Vec<&'a str>, ModifierKind),
    /// Flavour text, descriptions or note.
    Unread,
}

pub(super) fn parse(copied: &CopiedItem) -> Result<Item, ParseError> {
    let labels = Labels::of(copied.language());
    // The French texts put no-break spaces around the punctuation: plain spaces for the parser.
    let text = copied.text().replace(['\u{a0}', '\u{202f}'], " ");
    let mut sections = split_sections(&text);

    let header = sections.next().unwrap_or_default();
    let mut item = read_header(&header, copied.language(), labels)?;

    let recognized: Vec<Recognized<'_>> = sections.map(|lines| recognize(lines, labels)).collect();
    let with_headers = is_headed(&recognized);
    for section in resolve_plain_sections(recognized, item.rarity) {
        fill(&mut item, section, labels);
    }
    join_written_stats(&mut item.modifiers, copied.language(), with_headers);
    // An item copied from the in-game trade: its explicit lines without header.
    let explicit = item
        .modifiers
        .iter()
        .any(|modifier| matches!(modifier.kind, ModifierKind::Explicit(_)));
    item.tiers_from_values =
        !with_headers && explicit && matches!(item.rarity, Rarity::Magic | Rarity::Rare);
    item.rolls_from_prices = !with_headers && explicit && item.rarity == Rarity::Unique;
    Ok(item)
}

/// The non-empty lines of each section, trimmed: the game writes a space before some names.
fn split_sections(text: &str) -> impl Iterator<Item = Vec<&str>> {
    text.split(SECTION_SEPARATOR)
        .map(|section| {
            section
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .collect::<Vec<_>>()
        })
        .filter(|lines| !lines.is_empty())
}

/// The class, the rarity and the names.
fn read_header(lines: &[&str], language: Language, labels: &Labels) -> Result<Item, ParseError> {
    let mut lines = lines.iter().copied();
    let class = lines
        .next()
        .and_then(|line| line.strip_prefix(labels.item_class))
        .unwrap_or_default()
        .to_owned();
    let rarity_name = lines
        .next()
        .and_then(|line| line.strip_prefix(labels.rarity))
        .ok_or(ParseError::MissingRarity)?;
    let rarity = labels
        .rarities
        .iter()
        .find(|(name, _)| *name == rarity_name)
        .map(|&(_, rarity)| rarity)
        .ok_or(ParseError::UnknownRarity)?;

    let names: Vec<&str> = lines.collect();
    let (name, full_base_type) = match (rarity, names.as_slice()) {
        (Rarity::Rare | Rarity::Unique, [name, base_type, ..]) => (Some(*name), *base_type),
        (_, [base_type, ..]) => (None, *base_type),
        (_, []) => return Err(ParseError::MissingBaseType),
    };
    let (base_type, exceptional) = base_type_without_qualities(full_base_type, labels);

    Ok(Item {
        language,
        class,
        rarity,
        name: name.map(str::to_owned),
        base_type: base_type.to_owned(),
        written_base_type: full_base_type.to_owned(),
        exceptional,
        stack: None,
        item_level: None,
        quality: None,
        quality_kind: None,
        sockets: None,
        requirements: Requirements::default(),
        granted_skills: Vec::new(),
        states: Vec::new(),
        properties: Vec::new(),
        weapon: None,
        modifiers: Vec::new(),
        tiers_from_values: false,
        rolls_from_prices: false,
        warnings: Vec::new(),
    })
}

/// The base type without "Exceptional" and "Superior", in any order, and whether it was
/// exceptional.
fn base_type_without_qualities<'a>(mut base_type: &'a str, labels: &Labels) -> (&'a str, bool) {
    let mut exceptional = false;
    // Each word found makes the name shorter: the loop ends.
    loop {
        if let Some(rest) = labels
            .exceptional
            .iter()
            .find_map(|word| word.strip(base_type))
        {
            exceptional = true;
            base_type = rest;
        } else if let Some(rest) = labels
            .superior
            .iter()
            .find_map(|word| word.strip(base_type))
        {
            base_type = rest;
        } else {
            return (base_type, exceptional);
        }
    }
}

/// Phase 1: what a section is, from its lines.
fn recognize<'a>(lines: Vec<&'a str>, labels: &Labels) -> Recognized<'a> {
    let first = lines.first().copied().unwrap_or_default();
    let section = if first.starts_with(labels.requires) {
        Section::Requirements(first)
    } else if first.starts_with(labels.sockets) {
        Section::Sockets(first)
    } else if first.starts_with(labels.item_level) {
        Section::ItemLevel(first)
    } else if first.starts_with(labels.stack_size) {
        Section::Stack(first)
    } else if first.starts_with(labels.grants_skill) {
        Section::GrantedSkills(lines)
    } else if labels.note.is_some_and(|note| first.starts_with(note)) {
        Section::Unread
    } else if let Some(states) = lines.iter().map(|line| state(line, labels)).collect() {
        Section::States(states)
    } else if lines.iter().any(|line| line.starts_with('{')) {
        Section::Headed(lines)
    } else if let (_, Some(marker)) = Marker::take(first)
        && let Some(kind) = marker.kind()
        && lines
            .iter()
            .all(|line| Marker::take(line).1 == Some(marker))
    {
        Section::Marked(lines, kind)
    } else {
        return Recognized::Plain(lines);
    };
    Recognized::Section(section)
}

fn has_digit(line: &str) -> bool {
    line.chars().any(|c| c.is_ascii_digit())
}

fn state(line: &str, labels: &Labels) -> Option<ItemState> {
    labels.states.iter().find_map(|&(label, state)| {
        let matches = if state == ItemState::Unidentified {
            // Possibly followed by its tier: `Unidentified (Tier 3)`.
            line.starts_with(label)
        } else {
            line == label
        };
        matches.then_some(state)
    })
}

/// Phase 2: the first plain section, right after the header, is the properties; the other
/// plain sections (flavour text, descriptions) aren't read, but for the explicit modifiers of
/// an item copied without any header ([`explicit_section`]).
fn resolve_plain_sections(recognized: Vec<Recognized<'_>>, rarity: Rarity) -> Vec<Section<'_>> {
    let explicit = explicit_section(&recognized, rarity);
    recognized
        .into_iter()
        .enumerate()
        .map(|(index, recognized)| match recognized {
            Recognized::Section(section) => section,
            Recognized::Plain(lines) if index == 0 => Section::Properties(lines),
            Recognized::Plain(lines) if Some(index) == explicit => {
                Section::Marked(lines, ModifierKind::Explicit(None))
            }
            Recognized::Plain(_) => Section::Unread,
        })
        .collect()
}

/// Whether one of the sections has a `{ … }` header.
fn is_headed(recognized: &[Recognized<'_>]) -> bool {
    recognized
        .iter()
        .any(|r| matches!(r, Recognized::Section(Section::Headed(_))))
}

/// Whether the states of the item say it is unidentified.
fn is_unidentified(recognized: &[Recognized<'_>]) -> bool {
    recognized.iter().any(|r| {
        matches!(r, Recognized::Section(Section::States(states)) if states.contains(&ItemState::Unidentified))
    })
}

/// The index of the explicit modifiers of an identified item copied without any `{ … }`
/// header, its first plain section after the item level. Two copies have none: an item copied
/// from the trade of the game (another player's listing), its lines without affix, tier nor
/// range, those of a fractured, desecrated or Vaal modifier marked (` (fractured)`), and a
/// unique whose lines have no header in either copy, the same on every copy (`Reflects
/// opposite Ring` of Kalandra's Touch).
///
/// A modifier has a number in nearly every line, unlike a description: a magic item whose only
/// modifier was removed keeps its description, which isn't taken for it. The lines without
/// number of a unique are taken when another plain section, its flavour text, follows them; a
/// flavour text alone never is.
fn explicit_section(recognized: &[Recognized<'_>], rarity: Rarity) -> Option<usize> {
    if !rarity.has_explicit_modifiers() || is_headed(recognized) || is_unidentified(recognized) {
        return None;
    }
    let item_level = recognized
        .iter()
        .position(|r| matches!(r, Recognized::Section(Section::ItemLevel(_))))?;
    let mut plain = recognized
        .iter()
        .enumerate()
        .skip(item_level + 1)
        .filter_map(|(index, r)| match r {
            Recognized::Plain(lines) => Some((index, lines)),
            Recognized::Section(_) => None,
        });
    let (first, lines) = plain.next()?;
    let numbered = lines.iter().any(|line| has_digit(line));
    (numbered || (rarity == Rarity::Unique && plain.next().is_some())).then_some(first)
}

/// Phase 3: what a section adds to the item.
fn fill(item: &mut Item, section: Section<'_>, labels: &Labels) {
    match section {
        Section::Properties(lines) => read_properties(item, &lines, labels),
        Section::Requirements(line) => {
            let (requirements, complete) = read_requirements(line, labels);
            item.requirements = requirements;
            if !complete {
                item.warnings
                    .push(ParseWarning::InvalidValue(Field::Requirements));
            }
        }
        Section::Sockets(line) => item.sockets = Some(count_sockets(line, labels)),
        Section::ItemLevel(line) => {
            item.item_level = number_after(line, labels.item_level);
            if item.item_level.is_none() {
                item.warnings
                    .push(ParseWarning::InvalidValue(Field::ItemLevel));
            }
        }
        Section::Stack(line) => {
            item.stack = read_stack(line, labels);
            if item.stack.is_none() {
                item.warnings.push(ParseWarning::InvalidValue(Field::Stack));
            }
        }
        Section::GrantedSkills(lines) => item.granted_skills.extend(
            lines
                .iter()
                .filter_map(|line| line.strip_prefix(labels.grants_skill))
                .map(str::to_owned),
        ),
        Section::States(states) => item.states.extend(states),
        Section::Headed(lines) => read_headed(item, &lines, labels),
        Section::Marked(lines, kind) => item
            .modifiers
            .extend(lines.iter().map(|line| Modifier::marked(line, kind))),
        Section::Unread => {}
    }
}

/// Phase 4: the modifiers of one line written without header that make one stat written on
/// several lines (`Inflict Anaemia on Hit` then `Anaemia allows 2 Corrupted Blood debuffs to
/// be inflicted on enemies`, desecrated) joined into one, as under a header of the advanced
/// copy, when the game data finds that stat on their lines together: consecutive, of the same
/// kind, with the origin of the last line, whose marker may be the only one of the stat. The
/// runes and the enchantments never have a header; the implicit and explicit modifiers only in
/// a copy without any (`with_headers` false).
fn join_written_stats(modifiers: &mut Vec<Modifier>, language: Language, with_headers: bool) {
    let without_header = |modifier: &Modifier| {
        modifier.lines.len() == 1
            && match modifier.kind {
                ModifierKind::Rune | ModifierKind::Enchant => true,
                ModifierKind::Implicit | ModifierKind::Explicit(_) => !with_headers,
                ModifierKind::Corruption => false,
            }
    };
    let mut first = 0;
    while let Some(start) = modifiers.get(first) {
        let run = modifiers
            .iter()
            .skip(first)
            .take_while(|modifier| without_header(modifier) && modifier.kind == start.kind)
            .count();
        let joined = (2..=run).rev().find_map(|count| {
            let window = || modifiers.iter().skip(first).take(count);
            let origin = modifiers.get(first + count - 1)?.origin;
            // The lines before the last: without marker, or with its own.
            if window().any(|modifier| modifier.origin.is_some_and(|o| Some(o) != origin)) {
                return None;
            }
            let candidate = Modifier {
                lines: window()
                    .flat_map(|modifier| modifier.lines.iter().cloned())
                    .collect(),
                origin,
                ..start.clone()
            };
            let stats = game_data::modifier_stats(language, &candidate, None);
            matches!(stats.as_slice(), [found] if found.stat.is_some())
                .then_some((count, candidate))
        });
        if let Some((count, candidate)) = joined {
            modifiers.splice(first..first + count, [candidate]);
        }
        first += 1;
    }
}

/// The modifiers of a section with headers: each header, then its lines.
fn read_headed(item: &mut Item, lines: &[&str], labels: &Labels) {
    // The lines before the first header, if any, make a modifier of their own.
    let mut current: Option<Modifier> = None;
    for line in lines {
        match Header::parse(line, labels) {
            Some(header) => {
                item.modifiers.extend(current.take());
                current = Some(match header {
                    Header::Known(modifier) => *modifier,
                    Header::Unknown => {
                        item.warnings.push(ParseWarning::UnknownModifierHeader);
                        Modifier::unknown()
                    }
                });
            }
            None => current
                .get_or_insert_with(|| {
                    item.warnings.push(ParseWarning::ModifierWithoutHeader);
                    Modifier::unknown()
                })
                // The marker of a fixed value is too long for the narrow layout.
                .push_line(&line.replacen(labels.fixed_value, "", 1), labels),
        }
    }
    item.modifiers.extend(current);
}

/// The lines `Name: value` of the properties section, and the quality and the damage of a
/// weapon among them.
fn read_properties(item: &mut Item, lines: &[&str], labels: &Labels) {
    let mut damage = Vec::new();
    let mut attacks_per_second = None;
    for line in lines {
        let (line, augmented) = match line.strip_suffix(marker::AUGMENTED) {
            Some(line) => (line, true),
            None => (*line, false),
        };
        if let Some((kind, value)) = quality(line, labels) {
            item.quality = digits(value);
            if item.quality.is_none() {
                item.warnings
                    .push(ParseWarning::InvalidValue(Field::Quality));
            } else if let Some(kind) = kind {
                if kind.catalyst.is_none() {
                    item.warnings.push(ParseWarning::UnknownQualityKind);
                }
                item.quality_kind = Some(kind);
            }
        }
        let (name, value) = line.split_once(':').unwrap_or((line, ""));
        let (name, value) = (name.trim(), value.trim());
        if name == labels.attacks_per_second {
            attacks_per_second = value.parse::<f64>().ok();
        } else if let Some(ranges) = weapon::damage_ranges(value) {
            // A physical range has no marker: only the physical damage line has one.
            let physical = ranges
                .iter()
                .any(|range| range.kind == DamageKind::Physical);
            if !physical || name == labels.physical_damage {
                damage.extend(ranges);
            }
        }
        item.properties.push(Property {
            name: name.to_owned(),
            value: value.to_owned(),
            augmented,
        });
    }
    if let Some(attacks_per_second) = attacks_per_second
        && !damage.is_empty()
    {
        item.weapon = Some(Weapon {
            damage,
            attacks_per_second,
        });
    }
}

/// `Requires: Level 78, 89 Str, 163 (unmet) Dex`, and whether every part was read.
fn read_requirements(line: &str, labels: &Labels) -> (Requirements, bool) {
    let mut requirements = Requirements::default();
    let mut complete = true;
    for part in line
        .strip_prefix(labels.requires)
        .unwrap_or_default()
        .split(',')
    {
        let part = part
            .replace(marker::UNMET, "")
            .replace(marker::AUGMENTED, "");
        complete &= read_requirement(part.trim(), labels, &mut requirements).is_some();
    }
    (requirements, complete)
}

/// One part of the requirements: `Level 78` or `89 Str`.
fn read_requirement(part: &str, labels: &Labels, requirements: &mut Requirements) -> Option<()> {
    if let Some(level) = part.strip_prefix(labels.level) {
        requirements.level = Some(digits(level)?);
        return Some(());
    }
    let (value, attribute) = part.split_once(' ')?;
    let value = Some(digits(value)?);
    let attribute = attribute.trim();
    let target = if labels.strength.contains(&attribute) {
        &mut requirements.strength
    } else if labels.dexterity.contains(&attribute) {
        &mut requirements.dexterity
    } else if labels.intelligence.contains(&attribute) {
        &mut requirements.intelligence
    } else {
        return None;
    };
    *target = value;
    Some(())
}

/// `Sockets: S S` gives 2.
fn count_sockets(line: &str, labels: &Labels) -> u32 {
    let count = line
        .strip_prefix(labels.sockets)
        .unwrap_or_default()
        .split_whitespace()
        .count();
    u32::try_from(count).unwrap_or(u32::MAX)
}

/// `Stack Size: 40/40`, with or without thousands separators.
fn read_stack(line: &str, labels: &Labels) -> Option<Stack> {
    let (size, max) = line.strip_prefix(labels.stack_size)?.split_once('/')?;
    Some(Stack {
        size: digits(size)?,
        max: digits(max)?,
    })
}

/// The quality line, `Quality: +20%` or the quality of a catalyst, `Quality (Mana Modifiers):
/// +40%`: the kind of a catalyst, and the text of the value, empty for a quality of a catalyst
/// whose words don't close.
fn quality<'a>(line: &'a str, labels: &Labels) -> Option<(Option<QualityKind>, &'a str)> {
    if let Some(value) = line.strip_prefix(labels.quality) {
        return Some((None, value));
    }
    let (open, close) = labels.catalyst_quality;
    let Some((words, value)) = line.strip_prefix(open)?.split_once(close) else {
        return Some((None, ""));
    };
    let catalyst = labels
        .catalysts
        .iter()
        .find(|&&(known, _)| known == words)
        .map(|&(_, catalyst)| catalyst);
    let kind = QualityKind {
        catalyst,
        words: words.to_owned(),
    };
    Some((Some(kind), value))
}

/// The quality of a property written as the game writes it in `language`, `Quality (Mana
/// Modifiers): +40%` or `Quality: +20%`: its kind, none for the quality of the base, and its
/// value, none when it can't be read; `None` for another property.
#[must_use]
pub fn read_quality(line: &str, language: Language) -> Option<(Option<QualityKind>, Option<u32>)> {
    let (kind, value) = quality(line, Labels::of(language))?;
    Some((kind, digits(value)))
}

/// The number after a label, e.g. `Item Level: 80` or `Quality: +20%`.
fn number_after(line: &str, label: &str) -> Option<u32> {
    digits(line.strip_prefix(label)?)
}

/// The number made of all the digits of a text: `+20%` gives 20, `1,234` gives 1234; `None`
/// without digits or when too large.
fn digits(text: &str) -> Option<u32> {
    let digits: String = text.chars().filter(char::is_ascii_digit).collect();
    digits.parse().ok()
}

#[cfg(test)]
mod tests;
