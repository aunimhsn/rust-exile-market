//! The values of the lines `Label: value`: the properties (the quality, the level of a gem, the
//! properties of a waystone and the damage of a weapon among them), the requirements, the
//! sockets, the item level and the stack.

use super::{Field, ParseWarning};
use crate::item::labels::{Labels, marker};
use crate::item::weapon::Weapon;
use crate::item::{Item, Property, QualityKind, Rarity, Requirements, Stack, gem, waystone};
use crate::language::Language;

/// The lines `Name: value` of the properties section, and the damage of a weapon among them.
pub(super) fn read_properties(item: &mut Item, lines: &[&str], labels: &Labels) {
    for line in lines {
        read_property(item, line, labels);
    }
    item.weapon = Weapon::read(&item.properties, labels);
}

/// One line `Name: value` of the properties section, kept among the properties: the quality, the
/// level of a gem and a property of a waystone read on the way, or a warning for one that can't
/// be read.
fn read_property(item: &mut Item, line: &str, labels: &Labels) {
    let (line, is_augmented) = match line.strip_suffix(marker::AUGMENTED) {
        Some(line) => (line, true),
        None => (line, false),
    };
    if let Some(quality) = QualityLine::read(line, labels) {
        fill_quality(item, quality);
    }
    let (name, value) = line.split_once(':').unwrap_or((line, ""));
    let (name, value) = (name.trim(), value.trim());
    if item.rarity == Rarity::Gem && name == labels.gem_level {
        item.gem_level = gem::read_gem_level(value);
        if item.gem_level.is_none() {
            item.warnings
                .push(ParseWarning::InvalidValue(Field::GemLevel));
        }
    }
    if item.class == labels.waystones
        && let Err(warning) = waystone::read_property(labels, name, value)
    {
        item.warnings.push(warning);
    }
    item.properties.push(Property {
        name: name.to_owned(),
        value: value.to_owned(),
        is_augmented,
    });
}

/// The quality of `item`, from its quality line: a warning for a value that isn't a number, or
/// for the kind of a catalyst not in the labels, kept with the words of the game.
fn fill_quality(item: &mut Item, quality: QualityLine<'_>) {
    item.quality = quality.value.and_then(digits);
    if item.quality.is_none() {
        item.warnings
            .push(ParseWarning::InvalidValue(Field::Quality));
    } else if let Some(kind) = quality.kind {
        if kind.catalyst.is_none() {
            item.warnings.push(ParseWarning::UnknownQualityKind);
        }
        item.quality_kind = Some(kind);
    }
}

/// `Requires: Level 78, 89 Str, 163 (unmet) Dex`, and whether every part was read.
pub(super) fn read_requirements(line: &str, labels: &Labels) -> (Requirements, bool) {
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
pub(super) fn count_sockets(line: &str, labels: &Labels) -> u32 {
    let count = line
        .strip_prefix(labels.sockets)
        .unwrap_or_default()
        .split_whitespace()
        .count();
    u32::try_from(count).unwrap_or(u32::MAX)
}

/// `Stack Size: 40/40`, with or without thousands separators.
pub(super) fn read_stack(line: &str, labels: &Labels) -> Option<Stack> {
    let (size, max) = line.strip_prefix(labels.stack_size)?.split_once('/')?;
    Some(Stack {
        size: digits(size)?,
        max: digits(max)?,
    })
}

/// The quality line, `Quality: +20%` or the quality of a catalyst, `Quality (Mana Modifiers):
/// +40%`.
struct QualityLine<'a> {
    /// The kind of a catalyst; none for the quality of the base.
    kind: Option<QualityKind>,
    /// The text of the value; none for a quality of a catalyst whose words don't close.
    value: Option<&'a str>,
}

impl<'a> QualityLine<'a> {
    /// The quality `line` in the words of `labels`; none for another line.
    fn read(line: &'a str, labels: &Labels) -> Option<Self> {
        if let Some(value) = line.strip_prefix(labels.quality) {
            return Some(Self {
                kind: None,
                value: Some(value),
            });
        }
        let (open, close) = labels.catalyst_quality;
        let Some((words, value)) = line.strip_prefix(open)?.split_once(close) else {
            return Some(Self {
                kind: None,
                value: None,
            });
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
        Some(Self {
            kind: Some(kind),
            value: Some(value),
        })
    }
}

/// The quality of a property written as the game writes it in `language`, `Quality (Mana
/// Modifiers): +40%` or `Quality: +20%`: its kind, none for the quality of the base, and its
/// value, none when it isn't a whole percentage; `None` for another property.
#[must_use]
pub fn read_quality(line: &str, language: Language) -> Option<(Option<QualityKind>, Option<u32>)> {
    let quality = QualityLine::read(line, Labels::of(language))?;
    Some((quality.kind, quality.value.and_then(whole_percent)))
}

/// A whole percentage written `+20%` or `20%`; `None` for another text, a decimal or a
/// negative one.
#[must_use]
pub fn whole_percent(text: &str) -> Option<u32> {
    let text = text.trim();
    text.strip_prefix('+')
        .unwrap_or(text)
        .strip_suffix('%')?
        .parse()
        .ok()
}

/// The number after a label, e.g. `Item Level: 80` or `Quality: +20%`.
pub(super) fn number_after(line: &str, label: &str) -> Option<u32> {
    digits(line.strip_prefix(label)?)
}

/// The number made of all the digits of a text: `+20%` gives 20, `1,234` gives 1234; `None`
/// without digits or when too large.
fn digits(text: &str) -> Option<u32> {
    let digits: String = text.chars().filter(char::is_ascii_digit).collect();
    digits.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::Catalyst;

    #[test]
    fn reads_the_small_parts() {
        assert_eq!(digits("+20%"), Some(20));
        assert_eq!(digits("1 234"), Some(1234));
        assert_eq!(digits("none"), None);
        assert_eq!(digits("99999999999"), None);

        let labels = Labels::of(Language::English);
        assert_eq!(count_sockets("Sockets: S S S", labels), 3);
        assert_eq!(count_sockets("Sockets: ", labels), 0);
        assert_eq!(
            read_requirements("Requires: 113 Intelligence", labels),
            (
                Requirements {
                    intelligence: Some(113),
                    ..Requirements::default()
                },
                true
            )
        );
    }

    #[test]
    fn a_quality_is_read_as_a_whole_percentage_only() {
        assert_eq!(whole_percent("+20%"), Some(20));
        assert_eq!(whole_percent(" 20% "), Some(20));
        assert_eq!(whole_percent("+12.5%"), None);
        assert_eq!(whole_percent("-10%"), None);
        assert_eq!(whole_percent("+20"), None);
        assert_eq!(
            read_quality("Qualité (Modificateurs de Mana): +50%", Language::French),
            Some((
                Some(QualityKind {
                    catalyst: Some(Catalyst::Mana),
                    words: "Modificateurs de Mana".to_owned(),
                }),
                Some(50)
            ))
        );
        assert_eq!(
            read_quality("Quality: +12.5%", Language::English),
            Some((None, None)),
            "a quality, unreadable"
        );
        assert_eq!(read_quality("Spirit: 100", Language::English), None);
    }
}
