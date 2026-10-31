//! The advanced copy of an item written as the in-game trade copies it (another player's
//! listing), for the tests: as the owner's copies of the in-game trade show it, without
//! headers nor ranges, each implicit, enchantment, crafted, fractured, desecrated or Vaal
//! modifier marked (` (fractured)`), the fractured and Vaal modifiers first and the
//! desecrated ones last. A modifier of several lines has its last line only marked, the
//! form the parser must also read (step 13k5b), the game possibly marking a stat written on
//! several lines so.

use super::CopiedItem;
use super::labels::Labels;

/// The words of the header of a corruption enhancement, in English and French, whose in-game
/// trade copy isn't known.
const CORRUPTION: [&str; 2] = ["Corruption Enhancement", "Amélioration de Corruption"];

/// The marker of the lines of a header, from its words, in English and French.
const MARKERS: [(&[&str], &str); 6] = [
    (&["Implicit"], " (implicit)"),
    (&["Enhancement", "Amélioration"], " (enchant)"),
    (&["Crafted", "Artisanat"], " (crafted)"),
    (&["Fractured", "Fissuré"], " (fractured)"),
    (&["Desecrated", "Profané"], " (desecrated)"),
    (&["Vaal", "vaal"], " (mutated)"),
];

/// A modifier of the advanced copy: the marker of its lines, then its lines without ranges.
struct Written {
    marker: Option<&'static str>,
    lines: Vec<String>,
}

impl Written {
    /// Its place in the in-game trade copy: the fractured and Vaal modifiers first, the
    /// desecrated ones last.
    fn rank(&self) -> u8 {
        match self.marker {
            Some(" (fractured)" | " (mutated)") => 0,
            Some(" (desecrated)") => 2,
            _ => 1,
        }
    }
}

/// `text`, an advanced copy, as the in-game trade copies it; the marker of a fixed value
/// (` — Unscalable Value`) kept, whether the in-game trade writes it being unknown. None when
/// the item holds a modifier whose in-game trade copy isn't known: a corruption enhancement,
/// or a modifier not revealed yet.
pub fn as_in_game_trade(text: &str) -> Option<String> {
    let labels = Labels::of(CopiedItem::from_text(text)?.language());
    let mut sections = Vec::new();
    for section in text.split("\n--------\n") {
        if !section.lines().any(|line| line.starts_with('{')) {
            sections.push(section.to_owned());
            continue;
        }
        let mut modifiers: Vec<Written> = Vec::new();
        for line in section.lines() {
            if line.starts_with('{') {
                if CORRUPTION.iter().any(|words| line.contains(words)) {
                    return None;
                }
                modifiers.push(Written {
                    marker: header_marker(line),
                    lines: Vec::new(),
                });
            } else if labels
                .unrevealed
                .iter()
                .any(|&(unrevealed, _)| unrevealed == line.trim())
            {
                return None;
            } else if let Some(modifier) = modifiers.last_mut() {
                modifier.lines.push(without_ranges(line));
            }
        }
        modifiers.sort_by_key(Written::rank);
        let mut lines = Vec::new();
        for Written {
            marker,
            lines: mut written,
        } in modifiers
        {
            // Only the last line of a modifier is marked.
            if let (Some(marker), Some(last)) = (marker, written.last_mut()) {
                last.push_str(marker);
            }
            lines.append(&mut written);
        }
        sections.push(lines.join("\n"));
    }
    Some(sections.join("\n--------\n"))
}

/// The marker of the lines under `header`: none for a prefix, a suffix or a modifier of a
/// unique, which the in-game trade doesn't mark.
fn header_marker(header: &str) -> Option<&'static str> {
    MARKERS
        .iter()
        .find(|(words, _)| words.iter().any(|word| header.contains(word)))
        .map(|&(_, marker)| marker)
}

/// `line` without the ranges of its values: `Adds 12(11-17) to 26(20-30) Physical Damage`
/// written `Adds 12 to 26 Physical Damage`.
fn without_ranges(line: &str) -> String {
    let mut written = String::new();
    let mut in_range = false;
    for c in line.chars() {
        match c {
            '(' if written.ends_with(|c: char| c.is_ascii_digit()) => in_range = true,
            ')' if in_range => in_range = false,
            _ if in_range => {}
            _ => written.push(c),
        }
    }
    written
}
