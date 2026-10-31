//! The advanced copy of an item written as the in-game trade copies it (another player's
//! listing), for the tests: as the owner's copies of the in-game trade show it, without
//! headers nor ranges, each implicit, enchantment, crafted, fractured, desecrated or Vaal
//! modifier marked (` (fractured)`), the fractured and Vaal modifiers first and the
//! desecrated ones last. A modifier of several lines has its last line only marked, the
//! form the parser must also read (step 13k5b), the game possibly marking a stat written on
//! several lines so.

use super::labels::Labels;
use super::modifier::{Header, Marker};
use super::{CopiedItem, Modifier, ModifierKind, ModifierOrigin};

/// `text`, an advanced copy, as the in-game trade copies it; the marker of a fixed value
/// (` — Unscalable Value`) kept, whether the in-game trade writes it being unknown. None when
/// the item holds a modifier whose in-game trade copy isn't known: a corruption enhancement,
/// or a modifier not revealed yet.
pub fn as_in_game_trade(text: &str) -> Option<String> {
    let labels = Labels::of(CopiedItem::from_text(text)?.language());
    // The headers of the French texts put no-break spaces around the punctuation, as the
    // parser reads them.
    let text = text.replace(['\u{a0}', '\u{202f}'], " ");
    let mut sections = Vec::new();
    for section in text.split("\n--------\n") {
        if !section.lines().any(|line| line.starts_with('{')) {
            sections.push(section.to_owned());
            continue;
        }
        // Each modifier, read under its header, its lines without ranges.
        let mut modifiers: Vec<Modifier> = Vec::new();
        for line in section
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
        {
            if let Some(header) = Header::parse(line, labels) {
                let modifier = match header {
                    Header::Known(modifier) => *modifier,
                    Header::Unknown => Modifier::unknown(),
                };
                if modifier.kind == ModifierKind::Corruption {
                    return None;
                }
                modifiers.push(modifier);
            } else if let Some(modifier) = modifiers.last_mut() {
                modifier.push_line(line, labels);
                if modifier.unrevealed {
                    return None;
                }
            }
        }
        // The fractured and Vaal modifiers first, the desecrated ones last.
        modifiers.sort_by_key(|modifier| match modifier.origin {
            Some(ModifierOrigin::Fractured | ModifierOrigin::Vaal) => 0,
            Some(ModifierOrigin::Desecrated) => 2,
            _ => 1,
        });
        let mut lines = Vec::new();
        for modifier in &modifiers {
            let mut written: Vec<String> = modifier
                .lines
                .iter()
                .map(|line| line.text.clone())
                .collect();
            // Only the last line of a modifier is marked.
            if let (Some(marker), Some(last)) = (marker(modifier), written.last_mut()) {
                last.push_str(marker.suffix());
            }
            lines.append(&mut written);
        }
        sections.push(lines.join("\n"));
    }
    Some(sections.join("\n--------\n"))
}

/// The marker of the lines of `modifier`: none for a prefix, a suffix or a modifier of a
/// unique, which the in-game trade doesn't mark.
fn marker(modifier: &Modifier) -> Option<Marker> {
    match (modifier.kind, modifier.origin) {
        (ModifierKind::Implicit, _) => Some(Marker::Implicit),
        (ModifierKind::Enchant, _) => Some(Marker::Enchant),
        (ModifierKind::Rune, _) => Some(Marker::Rune),
        (_, Some(ModifierOrigin::Crafted)) => Some(Marker::Crafted),
        (_, Some(ModifierOrigin::Fractured)) => Some(Marker::Fractured),
        (_, Some(ModifierOrigin::Desecrated)) => Some(Marker::Desecrated),
        (_, Some(ModifierOrigin::Vaal)) => Some(Marker::Mutated),
        (ModifierKind::Explicit(_) | ModifierKind::Corruption, None) => None,
    }
}
