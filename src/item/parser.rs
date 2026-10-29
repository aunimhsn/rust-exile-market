//! Reads a copied item, in three phases: recognize each section from its first line, decide
//! what the sections without label are, then fill the item.
//!
//! A copied item is a list of sections separated by `--------`: the header (class, rarity,
//! names), the properties, the requirements, the sockets, the item level, the enchantments,
//! the runes, the implicit modifiers, the granted skills, the explicit modifiers, the states,
//! and texts that aren't read (flavour text, descriptions, note).

use std::fmt;

use super::labels::{Labels, marker};
use super::modifier::{Header, Marker, Modifier, ModifierKind};
use super::weapon::{self, DamageKind, Weapon};
use super::{CopiedItem, Item, ItemState, Property, Rarity, Requirements, Stack};
use crate::language::Language;

const SECTION_SEPARATOR: &str = "--------";

/// Why a copied item couldn't be read at all. It never holds the text of the item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    MissingRarity,
    UnknownRarity,
    MissingBaseType,
    /// An item whose implicit or explicit modifiers were copied without the `{ … }` headers of
    /// the advanced description, which give their tiers and ranges.
    WithoutDetails,
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::MissingRarity => "no rarity line",
            Self::UnknownRarity => "unknown rarity",
            Self::MissingBaseType => "no base type",
            Self::WithoutDetails => "modifiers copied without the advanced description",
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
    /// Modifiers without header, all of the kind their marker tells (` (rune)`…).
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
    for section in resolve_plain_sections(recognized, item.rarity)? {
        fill(&mut item, section, labels);
    }
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
        sockets: None,
        requirements: Requirements::default(),
        granted_skills: Vec::new(),
        states: Vec::new(),
        properties: Vec::new(),
        weapon: None,
        modifiers: Vec::new(),
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
/// plain sections (flavour text, descriptions) aren't read, but for the lines of a unique
/// written without header ([`unheaded_section`]).
///
/// The advanced copy writes a `{ … }` header before each implicit and explicit modifier: an
/// item with implicit lines without one, or without any header but with a plain section after
/// the item level that reads as the explicit modifiers of an identified item, was copied
/// without the details.
fn resolve_plain_sections(
    recognized: Vec<Recognized<'_>>,
    rarity: Rarity,
) -> Result<Vec<Section<'_>>, ParseError> {
    let headed = is_headed(&recognized);
    let explicit_expected =
        !headed && rarity.has_explicit_modifiers() && !is_unidentified(&recognized);
    let unheaded = unheaded_section(&recognized, rarity);
    let mut after_item_level = false;

    recognized
        .into_iter()
        .enumerate()
        .map(|(index, recognized)| match recognized {
            Recognized::Section(Section::Marked(_, ModifierKind::Implicit)) => {
                Err(ParseError::WithoutDetails)
            }
            Recognized::Section(section) => {
                after_item_level |= matches!(section, Section::ItemLevel(_));
                Ok(section)
            }
            Recognized::Plain(lines) if index == 0 => Ok(Section::Properties(lines)),
            // A modifier has a number in nearly every line, unlike a description: a magic
            // item whose only modifier was removed keeps its description, read with both
            // copies, and isn't taken for a plain copy.
            Recognized::Plain(lines)
                if explicit_expected
                    && after_item_level
                    && lines.iter().any(|line| has_digit(line)) =>
            {
                Err(ParseError::WithoutDetails)
            }
            // After the arm above: a unique copied without the details (from the trade of the
            // game, step 13k2e) has the shape of the lines without header, and its lines with
            // numbers must be refused, not read as lines that never vary.
            Recognized::Plain(lines) if Some(index) == unheaded => {
                Ok(Section::Marked(lines, ModifierKind::Explicit(None)))
            }
            Recognized::Plain(_) => Ok(Section::Unread),
        })
        .collect()
}

/// Whether one of the sections has a `{ … }` header.
fn is_headed(recognized: &[Recognized<'_>]) -> bool {
    recognized
        .iter()
        .any(|r| matches!(r, Recognized::Section(Section::Headed(_))))
}

fn is_unidentified(recognized: &[Recognized<'_>]) -> bool {
    recognized.iter().any(|r| {
        matches!(r, Recognized::Section(Section::States(states)) if states.contains(&ItemState::Unidentified))
    })
}

/// The index of the lines of an identified unique written without header, the same on every
/// copy (`Reflects opposite Ring` of Kalandra's Touch): when the copy has no `{ … }` header
/// at all, its first plain section after the item level, if another plain section, its
/// flavour text, follows it; a description or a flavour text alone is never taken for it.
fn unheaded_section(recognized: &[Recognized<'_>], rarity: Rarity) -> Option<usize> {
    if rarity != Rarity::Unique || is_headed(recognized) || is_unidentified(recognized) {
        return None;
    }
    let item_level = recognized
        .iter()
        .position(|r| matches!(r, Recognized::Section(Section::ItemLevel(_))))?;
    let mut plain = recognized
        .iter()
        .enumerate()
        .skip(item_level + 1)
        .filter(|(_, r)| matches!(r, Recognized::Plain(_)))
        .map(|(index, _)| index);
    let first = plain.next()?;
    plain.next().map(|_| first)
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

/// The modifiers of a section with headers: each header, then its lines.
fn read_headed(item: &mut Item, lines: &[&str], labels: &Labels) {
    // The lines before the first header, if any, make a modifier of their own.
    let mut current: Option<Modifier> = None;
    for line in lines {
        match Header::parse(line, labels) {
            Some(header) => {
                item.modifiers.extend(current.take());
                current = Some(match header {
                    Header::Known(modifier) => modifier,
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
        if line.starts_with(labels.quality) {
            item.quality = number_after(line, labels.quality);
            if item.quality.is_none() {
                item.warnings
                    .push(ParseWarning::InvalidValue(Field::Quality));
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
