//! The placements on the tables of the game: the numbers of a patch, to record again when the
//! game data are generated again.

use super::*;
use crate::game_data::bases;
use crate::game_data::lines::modifier_stats;
use crate::game_data::modifiers::fixtures::range;
use crate::game_data::modifiers::tables::TABLES;
use crate::item::{Modifier, ModifierKind, StatLine};
use crate::language::Language;

#[test]
fn only_the_tiers_the_item_level_allows_are_placed() {
    let modifiers = [Modifier {
        kind: ModifierKind::Explicit(None),
        origin: None,
        name: None,
        tier: None,
        tags: Vec::new(),
        is_unrevealed: false,
        lines: vec![StatLine {
            text: "175% increased Physical Damage".to_owned(),
            rolls: Vec::new(),
        }],
        doubt: None,
    }];
    let copied = Copied::of(
        &modifiers[0],
        &modifier_stats(Language::English, &modifiers[0], None),
    )
    .expect("a line of the game data");
    let lines: Vec<Line<'_>> = copied
        .lines
        .iter()
        .map(|copied| Line {
            modifier: 0,
            origin: None,
            copied: copied.clone(),
            values: 0..1,
            read_by_best: true,
        })
        .collect();
    let base = bases::spawn(Language::English, "Warmonger Bow").expect("a base");
    // The pure T1 `170–179` needs the item level 82.
    let pure_t1 = |item_level| {
        let tables = ItemTables {
            all: &TABLES,
            base,
            item_level: Some(item_level),
        };
        placements(&tables, &lines, &[175.0])
            .iter()
            .any(|placement| placement.shown.ranges == [range(170.0, 179.0)])
    };

    assert!(pure_t1(82));
    assert!(!pure_t1(81));
}
