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

/// The range of each number of the copied `line`, in its order (`None` for a number written
/// in the text of the modifier, which doesn't roll), from the first of `modifiers` it
/// matches; `None` when it matches none.
#[must_use]
pub fn unique_ranges(line: &str, modifiers: &[String]) -> Option<Vec<Option<Range>>> {
    modifiers
        .iter()
        // A modifier can hold several lines.
        .flat_map(|modifier| modifier.lines())
        .find_map(|modifier| ranges(modifier.trim(), line))
}

/// The range of each number of `line` when it is `modifier` with a number in place of each
/// range.
fn ranges(modifier: &str, line: &str) -> Option<Vec<Option<Range>>> {
    let mut rest = line;
    let mut ranges = Vec::new();
    for part in parts(modifier) {
        match part {
            Part::Text(text) => {
                rest = rest.strip_prefix(text)?;
                // The numbers of the text of the modifier are the same on every item.
                ranges.extend((0..numbers_in(text)).map(|_| None));
            }
            Part::Range(range) => {
                rest = after_number(rest)?;
                ranges.push(Some(range));
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

/// The range written `min-max`, each with its sign, e.g. `15-35` or `-40--20`, of the absolute
/// values as the game shows them ([`Range::magnitude`]).
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
    Range {
        min: first,
        max: second,
    }
    .magnitude()
}

/// `text` after the number at its start, with its sign, e.g. `-12.5% reduced`.
fn after_number(text: &str) -> Option<&str> {
    let unsigned = text.strip_prefix(['-', '+']).unwrap_or(text);
    let length = unsigned
        .find(|character: char| !character.is_ascii_digit() && character != '.')
        .unwrap_or(unsigned.len());
    // A number has a digit: "." alone isn't one.
    unsigned[..length]
        .contains(|character: char| character.is_ascii_digit())
        .then(|| &unsigned[length..])
}

/// The number of numbers written in `text`, e.g. 1 in ` to 9 Physical Damage`.
fn numbers_in(text: &str) -> usize {
    text.split(|character: char| !character.is_ascii_digit() && character != '.')
        .filter(|part| part.contains(|character: char| character.is_ascii_digit()))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[expect(clippy::unnecessary_wraps, reason = "a range of the result")]
    fn range(min: f64, max: f64) -> Option<Range> {
        Some(Range { min, max })
    }

    fn modifiers(lines: &[&str]) -> Vec<String> {
        lines.iter().map(|&line| line.to_owned()).collect()
    }

    #[test]
    fn a_copied_line_gets_the_ranges_of_its_modifier() {
        let headhunter = modifiers(&[
            "(20-30)% increased Stun Threshold",
            "+(40-60) to maximum Life",
            "When you kill a Rare monster, you gain its Modifiers for 60 seconds",
        ]);

        assert_eq!(
            unique_ranges("+55 to maximum Life", &headhunter),
            Some(vec![range(40.0, 60.0)])
        );
        assert_eq!(unique_ranges("+55 to maximum Mana", &headhunter), None);
        // A modifier without a range: its number is written in its text.
        assert_eq!(
            unique_ranges(
                "When you kill a Rare monster, you gain its Modifiers for 60 seconds",
                &headhunter
            ),
            Some(vec![None])
        );
    }

    #[test]
    fn a_fixed_number_before_a_range_keeps_its_place() {
        let loreweave = modifiers(&[
            "Adds (8-12) to 9 Physical Damage to Attacks",
            "Adds 1 to (50-60) Lightning Damage",
        ]);

        assert_eq!(
            unique_ranges("Adds 10 to 9 Physical Damage to Attacks", &loreweave),
            Some(vec![range(8.0, 12.0), None])
        );
        assert_eq!(
            unique_ranges("Adds 1 to 55 Lightning Damage", &loreweave),
            Some(vec![None, range(50.0, 60.0)])
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
            Some(vec![range(2.0, 3.0), None])
        );
    }

    #[test]
    fn a_range_across_zero_and_a_lone_point_are_not_read_but_a_bound_at_zero_is() {
        let modifiers = modifiers(&["(-10-10) to Maximum Power Charges"]);

        assert_eq!(
            unique_ranges("5 to Maximum Power Charges", &modifiers),
            None
        );
        assert_eq!(after_number(". to"), None);
        assert_eq!(after_number("12.5% more"), Some("% more"));
    }

    #[test]
    fn parentheses_around_a_text_stay_in_the_text() {
        assert_eq!(
            parts("Gain (some) Rage (10-20) times"),
            [
                Part::Text("Gain (some)"),
                Part::Text(" Rage "),
                Part::Range(Range {
                    min: 10.0,
                    max: 20.0
                }),
                Part::Text(" times"),
            ]
        );
    }
}
