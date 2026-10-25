//! The ranges of the rolls of a unique copied without them (Ctrl+C): each copied line is
//! matched with the modifiers of its unique in the price database, e.g. `28% increased Flask
//! Life Recovery rate` with `(15-35)% increased Flask Life Recovery rate`.
//!
//! The price database writes its modifiers in English: an item copied in French gets its
//! ranges from the advanced copy only.

use crate::item::Range;

/// A part of a modifier of the price database: a text, or the range of a roll.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Part<'a> {
    Text(&'a str),
    Range(Range),
}

/// The ranges of the rolls of the copied `line`, from the first modifier of `modifiers` it
/// matches; none when it matches none, or a modifier without a range.
#[must_use]
pub fn unique_ranges(line: &str, modifiers: &[String]) -> Option<Vec<Range>> {
    modifiers
        .iter()
        // A modifier can hold several lines.
        .flat_map(|modifier| modifier.lines())
        .find_map(|modifier| ranges(modifier.trim(), line))
        .filter(|ranges| !ranges.is_empty())
}

/// The ranges of `modifier` when `line` is that modifier with a number in place of each
/// range.
fn ranges(modifier: &str, line: &str) -> Option<Vec<Range>> {
    let mut rest = line;
    let mut ranges = Vec::new();
    for part in parts(modifier) {
        match part {
            Part::Text(text) => rest = rest.strip_prefix(text)?,
            Part::Range(range) => {
                rest = after_number(rest)?;
                ranges.push(range);
            }
        }
    }
    // The game writes a plural the database doesn't: "Has 2 Charm Slots" for "Has (1-3)
    // Charm Slot".
    (rest.is_empty() || rest == "s").then_some(ranges)
}

/// The parts of `modifier`: the ranges in parentheses, e.g. `(15-35)` or `(-40--20)`, and
/// the texts between them.
fn parts(modifier: &str) -> Vec<Part<'_>> {
    let mut parts = Vec::new();
    let mut rest = modifier;
    while let Some(open) = rest.find('(') {
        let Some(close) = rest[open..].find(')').map(|close| open + close) else {
            break;
        };
        match range(&rest[open + 1..close]) {
            Some(range) => {
                parts.push(Part::Text(&rest[..open]));
                parts.push(Part::Range(range));
            }
            // Parentheses around a text: part of the text.
            None => parts.push(Part::Text(&rest[..=close])),
        }
        rest = &rest[close + 1..];
    }
    parts.push(Part::Text(rest));
    parts.retain(|part| *part != Part::Text(""));
    parts
}

/// The range written `min-max`, each with its sign, e.g. `15-35` or `-40--20`. The game shows
/// a negative roll without its sign ("20% reduced"): the range is of the absolute values.
fn range(text: &str) -> Option<Range> {
    // The separator is the first minus sign after a digit.
    let separator = text
        .char_indices()
        .skip(1)
        .find(|&(index, character)| {
            character == '-'
                && text[..index]
                    .chars()
                    .next_back()
                    .is_some_and(|before| before.is_ascii_digit())
        })
        .map(|(index, _)| index)?;
    let first: f64 = text[..separator].parse().ok()?;
    let second: f64 = text[separator + 1..].parse().ok()?;
    let (first, second) = (first.abs(), second.abs());
    Some(Range {
        min: first.min(second),
        max: first.max(second),
    })
}

/// `text` after the number at its start, with its sign, e.g. `-12.5% reduced`.
fn after_number(text: &str) -> Option<&str> {
    let unsigned = text.strip_prefix(['-', '+']).unwrap_or(text);
    let digits = unsigned
        .find(|character: char| !character.is_ascii_digit() && character != '.')
        .unwrap_or(unsigned.len());
    (digits > 0).then(|| &unsigned[digits..])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn range(min: f64, max: f64) -> Range {
        Range { min, max }
    }

    fn modifiers(lines: &[&str]) -> Vec<String> {
        lines.iter().map(|&line| line.to_owned()).collect()
    }

    #[test]
    fn a_copied_line_gets_the_ranges_of_its_modifier() {
        let headhunter = modifiers(&[
            "(20-30)% increased Stun Threshold",
            "+(40-60) to maximum Life",
            "+(20-40) to Strength",
            "When you kill a Rare monster, you gain its Modifiers for 60 seconds",
        ]);

        assert_eq!(
            unique_ranges("+55 to maximum Life", &headhunter),
            Some(vec![range(40.0, 60.0)])
        );
        assert_eq!(
            unique_ranges("26% increased Stun Threshold", &headhunter),
            Some(vec![range(20.0, 30.0)])
        );
        assert_eq!(unique_ranges("+55 to maximum Mana", &headhunter), None);
        // A modifier without a range has no roll.
        assert_eq!(
            unique_ranges(
                "When you kill a Rare monster, you gain its Modifiers for 60 seconds",
                &headhunter
            ),
            None
        );
    }

    #[test]
    fn several_rolls_negative_ranges_and_plurals_are_matched() {
        let modifiers = modifiers(&[
            "Adds (8-12) to (16-22) Fire Damage",
            "(-40--20)% reduced Cooldown Recovery Rate",
            "Has (1-3) Charm Slot",
            "Gain (2-3) Explosive Rhythm every 3 times you use a Grenade Skill\n Remove all Explosive Rhythm on reaching 10",
        ]);

        assert_eq!(
            unique_ranges("Adds 10 to 20 Fire Damage", &modifiers),
            Some(vec![range(8.0, 12.0), range(16.0, 22.0)])
        );
        assert_eq!(
            unique_ranges("30% reduced Cooldown Recovery Rate", &modifiers),
            Some(vec![range(20.0, 40.0)])
        );
        assert_eq!(
            unique_ranges("Has 2 Charm Slots", &modifiers),
            Some(vec![range(1.0, 3.0)])
        );
        assert_eq!(
            unique_ranges(
                "Gain 3 Explosive Rhythm every 3 times you use a Grenade Skill",
                &modifiers
            ),
            Some(vec![range(2.0, 3.0)])
        );
    }

    #[test]
    fn parentheses_around_a_text_stay_in_the_text() {
        assert_eq!(
            parts("Gain (some) Rage (10-20) times"),
            [
                Part::Text("Gain (some)"),
                Part::Text(" Rage "),
                Part::Range(range(10.0, 20.0)),
                Part::Text(" times"),
            ]
        );
    }
}
