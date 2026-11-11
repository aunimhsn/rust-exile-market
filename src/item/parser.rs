//! Reads a copied item, in four phases: recognize each section from its first line, decide
//! what the sections without label are, fill the item, then join the lines of one stat
//! written without header.
//!
//! A copied item is a list of sections separated by `--------`: the first one (class, rarity,
//! names), the properties, the requirements, the sockets, the item level, the enchantments,
//! the runes, the implicit modifiers, the granted skills, the explicit modifiers, the states,
//! and texts that aren't read (flavour text, descriptions, note).

mod errors;
mod values;

use super::labels::Labels;
use super::modifier::{Header, Marker, Modifier, ModifierKind, StatLine};
use super::{CopiedItem, Item, ItemState, Rarity, Requirements, RollsSource};
use crate::game_data;
use crate::language::Language;

pub use errors::{Field, ParseError, ParseWarning};
use values::{count_sockets, number_after, read_properties, read_requirements, read_stack};
pub use values::{read_quality, whole_percent};

/// The line between two sections of a copied item.
const SECTION_SEPARATOR: &str = "--------";

/// A section after phase 1: recognized from its lines, or plain lines without label nor
/// marker, which phase 2 resolves.
enum Recognized<'a> {
    Section(Section<'a>),
    Plain(Vec<&'a str>),
}

/// Whether the copy writes the `{ … }` headers of the modifiers, as the advanced copy does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Headers {
    Written,
    Absent,
}

impl Headers {
    /// How the sections write the headers of the modifiers: `Written` when one of them has a
    /// `{ … }` header.
    fn of(recognized: &[Recognized<'_>]) -> Self {
        let headed = recognized
            .iter()
            .any(|r| matches!(r, Recognized::Section(Section::Headed(_))));
        if headed { Self::Written } else { Self::Absent }
    }
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
    /// Modifiers without header, all of one kind, which the marker of one line at least tells
    /// (` (rune)`…, [`marked_kind`]) or phase 2 finds; each line of the origin its own marker
    /// tells (` (fractured)`…).
    Marked(Vec<&'a str>, ModifierKind),
    /// Lines without header whose markers of a kind [`marked_kind`] can't tell: not read, and
    /// never taken for the explicit modifiers.
    MixedMarkers,
    /// Flavour text, descriptions or note.
    Unread,
}

/// Reads `copied` ([`Item::parse`]).
pub(super) fn parse(copied: &CopiedItem) -> Result<Item, ParseError> {
    let labels = Labels::of(copied.language());
    // The French texts put no-break spaces around the punctuation: plain spaces for the parser.
    // The marker of a fixed value, too long for the narrow layout, is left out of every line
    // before the other markers are read, whatever their order.
    let text = copied
        .text()
        .replace(['\u{a0}', '\u{202f}'], " ")
        .replace(labels.fixed_value, "");
    let mut sections = split_sections(&text);

    let first = sections.next().unwrap_or_default();
    let mut item = read_first_section(&first, copied.language(), labels)?;

    let recognized: Vec<Recognized<'_>> = sections.map(|lines| recognize(lines, labels)).collect();
    let headers = Headers::of(&recognized);
    for section in resolve_plain_sections(recognized, item.rarity, copied.language(), headers) {
        fill(&mut item, section, labels);
    }
    join_written_stats(&mut item.modifiers, copied.language(), headers);
    item.rolls_source = rolls_source(&item, headers);
    Ok(item)
}

/// Where the rolls of the modifiers of `item` come from, by its `headers`: an item copied
/// without them, its explicit lines among its modifiers, was copied from the in-game trade.
fn rolls_source(item: &Item, headers: Headers) -> RollsSource {
    let has_explicit = item
        .modifiers
        .iter()
        .any(|modifier| matches!(modifier.kind, ModifierKind::Explicit(_)));
    match headers {
        Headers::Written => RollsSource::Headers,
        Headers::Absent if !has_explicit => RollsSource::Nowhere,
        Headers::Absent => match item.rarity {
            Rarity::Magic | Rarity::Rare => RollsSource::Values,
            Rarity::Unique => RollsSource::Prices,
            Rarity::Normal | Rarity::Currency | Rarity::Gem => RollsSource::Nowhere,
        },
    }
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

/// The first section: the class, the rarity and the names.
fn read_first_section(
    lines: &[&str],
    language: Language,
    labels: &Labels,
) -> Result<Item, ParseError> {
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
        gem_level: None,
        quality: None,
        quality_kind: None,
        sockets: None,
        requirements: Requirements::default(),
        granted_skills: Vec::new(),
        states: Vec::new(),
        properties: Vec::new(),
        weapon: None,
        modifiers: Vec::new(),
        rolls_source: RollsSource::Nowhere,
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
    } else if let Some(kind) = marked_kind(&lines) {
        Section::Marked(lines, kind)
    } else if lines.iter().any(|line| has_kind_marker(line)) {
        Section::MixedMarkers
    } else {
        return Recognized::Plain(lines);
    };
    Recognized::Section(section)
}

/// The kind of the modifiers of a section without header that a marker of a kind tells
/// (` (implicit)`, ` (rune)`, ` (enchant)`): one line marked so at least, on any line, every
/// other line marked the same or not at all, as the in-game trade may mark a modifier of
/// several lines on its last line only (`Adds Irradiated to a Map` then `10 uses remaining
/// (implicit)`). An unmarked line anywhere is of that kind too: a section of these kinds never
/// holds the explicit modifiers, so that it never takes their place.
fn marked_kind(lines: &[&str]) -> Option<ModifierKind> {
    let mut markers = lines.iter().filter_map(|line| Marker::take(line).1);
    let marker = markers.next()?;
    let kind = marker.kind()?;
    markers.all(|other| other == marker).then_some(kind)
}

/// Whether a line has a marker of a kind, which no explicit modifier has.
fn has_kind_marker(line: &str) -> bool {
    Marker::take(line).1.and_then(Marker::kind).is_some()
}

/// Whether every line of a section copied without header is a stat of the game data, read as
/// explicit modifiers: the only modifier of a magic waystone has no number (`Area has patches
/// of Ignited Ground`), unlike nearly every other, and no description is a stat.
fn is_known_stats(lines: &[&str], language: Language) -> bool {
    let mut modifier = Modifier::unknown();
    modifier.lines.extend(
        lines
            .iter()
            .map(|line| StatLine::parse(Marker::take(line).0)),
    );
    game_data::modifier_stats(language, &modifier, None)
        .iter()
        .all(|found| found.stat.is_some())
}

/// Whether `line` holds a digit.
fn has_digit(line: &str) -> bool {
    line.chars().any(|c| c.is_ascii_digit())
}

/// The state a line at the end of the item tells, if it is one.
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

/// Phase 2: the first plain section, right after the first section, is the properties; the other
/// plain sections (flavour text, descriptions) aren't read, but for the explicit modifiers of
/// an item copied without any header ([`explicit_section`]).
fn resolve_plain_sections(
    recognized: Vec<Recognized<'_>>,
    rarity: Rarity,
    language: Language,
    headers: Headers,
) -> Vec<Section<'_>> {
    let explicit = explicit_section(&recognized, rarity, language, headers);
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

/// Whether the states of the item say it is unidentified.
fn is_unidentified(recognized: &[Recognized<'_>]) -> bool {
    recognized.iter().any(|r| {
        matches!(r, Recognized::Section(Section::States(states)) if states.contains(&ItemState::Unidentified))
    })
}

/// The index of the explicit modifiers of an identified item copied without any `{ … }`
/// header, its first plain section after the item level: a section with a marker of a kind
/// is never plain ([`Section::Marked`], [`Section::MixedMarkers`]). Two copies have none: an item copied from the trade of the game (another player's
/// listing), its lines without affix, tier nor range, those of a fractured, desecrated or Vaal
/// modifier marked (` (fractured)`), and a unique whose lines have no header in either copy,
/// the same on every copy (`Reflects opposite Ring` of Kalandra's Touch).
///
/// A modifier has a number in nearly every line, unlike a description: a magic item whose only
/// modifier was removed keeps its description, which isn't taken for it. Lines without number
/// are taken when each is a stat of the game data ([`is_known_stats`]), or, for a unique, when
/// another plain section, its flavour text, follows them; a flavour text alone never is.
fn explicit_section(
    recognized: &[Recognized<'_>],
    rarity: Rarity,
    language: Language,
    headers: Headers,
) -> Option<usize> {
    if !rarity.has_explicit_modifiers()
        || headers == Headers::Written
        || is_unidentified(recognized)
    {
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
    (numbered
        || is_known_stats(lines, language)
        || (rarity == Rarity::Unique && plain.next().is_some()))
    .then_some(first)
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
        Section::MixedMarkers => item.warnings.push(ParseWarning::MixedMarkers),
        Section::Unread => {}
    }
}

/// Phase 4: the modifiers of one line written without header that make one stat written on
/// several lines (`Inflict Anaemia on Hit` then `Anaemia allows 2 Corrupted Blood debuffs to
/// be inflicted on enemies`, desecrated) joined into one, as under a header of the advanced
/// copy, when the game data finds that stat on their lines together: consecutive, of the same
/// kind, with the origin of the last line, whose marker may be the only one of the stat. The
/// runes and the enchantments never have a header; the implicit and explicit modifiers only in
/// a copy without any (`headers` absent).
fn join_written_stats(modifiers: &mut Vec<Modifier>, language: Language, headers: Headers) {
    let without_header = |modifier: &Modifier| {
        modifier.lines.len() == 1
            && match modifier.kind {
                ModifierKind::Rune | ModifierKind::Enchant => true,
                ModifierKind::Implicit | ModifierKind::Explicit(_) => headers == Headers::Absent,
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
        // No longer than a stat of the game data: a long run of lines that make no stat (a
        // text written to look like an item) is then read once, not once per length.
        let most = run.min(game_data::max_stat_lines(language));
        let joined = (2..=most).rev().find_map(|count| {
            let window = modifiers.get(first..first + count)?;
            Some((count, joined_modifier(window, language)?))
        });
        if let Some((count, modifier)) = joined {
            modifiers.splice(first..first + count, [modifier]);
        }
        first += 1;
    }
}

/// The modifiers of one line of `window` joined into one, of the kind of the first and the origin
/// of the last, when the game data find one stat on their lines together; none when a line before
/// the last has the marker of another origin.
fn joined_modifier(window: &[Modifier], language: Language) -> Option<Modifier> {
    let origin = window.last()?.origin;
    // The lines before the last: without marker, or with its own.
    if window
        .iter()
        .any(|modifier| modifier.origin.is_some_and(|o| Some(o) != origin))
    {
        return None;
    }
    let joined = Modifier {
        lines: window
            .iter()
            .flat_map(|modifier| modifier.lines.iter().cloned())
            .collect(),
        origin,
        ..window.first()?.clone()
    };
    let stats = game_data::modifier_stats(language, &joined, None);
    matches!(stats.as_slice(), [found] if found.stat.is_some()).then_some(joined)
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
                .push_line(line, labels),
        }
    }
    item.modifiers.extend(current);
}

#[cfg(test)]
mod tests;
