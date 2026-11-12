//! What the tests of the modifier tables share, in three parts: the modifiers and the bases
//! written by hand; the readers of what the search found; then what reads the test items and
//! the tables of the game, for the `game_tests.rs` only: a `tests.rs` takes none of it, so
//! that it passes whatever the patch of the game.

use crate::game_data::bases::{BaseSpawn, Domain};
use crate::game_data::lines::modifier_stats;
use crate::game_data::modifiers::ModifierTiers;
use crate::game_data::modifiers::item_tables::ItemBase;
use crate::game_data::modifiers::lookup::modifier_tiers;
use crate::game_data::modifiers::tables::{Table, read};
use crate::game_data::modifiers::trade_tiers::{AffixLimits, FoundTiers, TradeItem};
use crate::item::{
    Affix, CopiedItem, Item, ItemState, Modifier, ModifierKind, ModifierOrigin, PartTier, Range,
    StatLine,
};
use crate::language::Language;

// Written by hand.

/// The range from `min` to `max`.
pub(super) fn range(min: f64, max: f64) -> Range {
    Range { min, max }
}

/// A base of the equipment with `tags`.
pub(super) fn equipment(tags: &[&str]) -> BaseSpawn {
    BaseSpawn {
        domain: Domain::Item,
        tags: tags.iter().map(|&tag| tag.to_owned()).collect(),
    }
}

/// A modifier of `affix` with `lines`, without the ranges of the advanced copy: found by
/// the values the stats read in the text.
pub(super) fn modifier(affix: Option<Affix>, lines: &[&str]) -> Modifier {
    Modifier {
        kind: ModifierKind::Explicit(affix),
        origin: None,
        name: None,
        tier: None,
        tags: Vec::new(),
        is_unrevealed: false,
        lines: lines
            .iter()
            .map(|text| StatLine {
                text: (*text).to_owned(),
                rolls: Vec::new(),
            })
            .collect(),
        doubt: None,
    }
}

/// A modifier of `affix` with `lines`, the game marking it desecrated.
pub(super) fn desecrated(affix: Affix, lines: &[&str]) -> Modifier {
    Modifier {
        origin: Some(ModifierOrigin::Desecrated),
        ..modifier(Some(affix), lines)
    }
}

/// Modifiers of one line each, as the in-game trade writes them, with their values.
pub(super) fn trade_lines(lines: &[&str]) -> Vec<Modifier> {
    lines
        .iter()
        .map(|line| Modifier {
            lines: vec![StatLine::parse(line)],
            ..modifier(None, &[])
        })
        .collect()
}

/// A part of a split line of `affix`, in `tiers`, over `ranges`.
pub(super) fn part(affix: Affix, tiers: &[u32], ranges: &[(f64, f64)]) -> PartTier {
    PartTier {
        affix,
        tiers: tiers.to_vec(),
        hides_number: false,
        ranges: ranges.iter().map(|&(min, max)| range(min, max)).collect(),
        currency: None,
    }
}

/// A table of one or two lines that rolls on the helmets: its type, its affix, the template
/// of each line, and for each tier its level and the range of each line.
pub(super) fn table(
    kind: &str,
    affix: &str,
    templates: &[&str],
    tiers: &[(u32, &[(u32, u32)])],
) -> String {
    let lines: Vec<String> = templates
        .iter()
        .map(|template| format!(r#"{{"template": "{template}"}}"#))
        .collect();
    let tiers: Vec<String> = tiers
        .iter()
        .map(|(level, ranges)| {
            let ranges: Vec<String> = ranges
                .iter()
                .map(|(min, max)| format!("[[{min}, {max}]]"))
                .collect();
            format!(r#"{{"level": {level}, "ranges": [{}]}}"#, ranges.join(", "))
        })
        .collect();
    format!(
        r#"{{"type": "{kind}", "affix": "{affix}", "lines": [{}], "spawn": [["helmet", 1]], "tiers": [{}]}}"#,
        lines.join(", "),
        tiers.join(", ")
    )
}

/// The tables written line by line in `lines` ([`table`]), every one of them read.
pub(super) fn written(lines: &[String]) -> Vec<Table> {
    let tables = read(&lines.join("\n"));
    assert_eq!(tables.len(), lines.len());
    tables
}

pub(super) const LIFE: &str = "# to maximum Life";
pub(super) const MANA: &str = "# to maximum Mana";
pub(super) const ENERGY_SHIELD: &str = "#% increased Energy Shield";
pub(super) const RARITY: &str = "#% increased Rarity of Items found";

// The readers of what the search found.

/// The tier number of each affix a modifier can be.
pub(super) type AffixTiers = Vec<(Affix, u32)>;

/// The texts of the lines of each modifier read from its values, and the tier number of
/// each affix it can be.
pub(super) type LinesRead = Vec<(Vec<String>, AffixTiers)>;

/// The text of each line split, and the affix and tier number of its pure modifier.
pub(super) type Added = Vec<(String, Affix, u32)>;

/// For each modifier read from its values, the texts of its lines and the tier number of
/// each affix it can be.
pub(super) fn lines_read(modifiers: &[Modifier], found: &[FoundTiers]) -> LinesRead {
    found
        .iter()
        .map(|found| {
            let lines = modifiers[found.modifiers.clone()]
                .iter()
                .flat_map(|modifier| &modifier.lines)
                .map(|line| line.text.clone())
                .collect();
            let affixes = found
                .affixes
                .iter()
                .map(|(affix, tiers)| (*affix, tiers.tier().expect("its tier").number))
                .collect();
            (lines, affixes)
        })
        .collect()
}

/// For each line read as two modifiers, its text and the affix and tier number of the
/// other one, a pure modifier of the line.
pub(super) fn added(modifiers: &[Modifier], found: &[FoundTiers]) -> Added {
    found
        .iter()
        .filter_map(|found| {
            let added = found.added.as_ref()?;
            let text = modifiers.get(added.line)?.lines.first()?.text.clone();
            Some((text, added.affix, added.tiers.tier()?.number))
        })
        .collect()
}

/// For each modifier read from its values, how many lines it has and the tier number of
/// each affix it can be: the same in every language.
pub(super) fn summary(modifiers: &[Modifier], found: &[FoundTiers]) -> Vec<(usize, AffixTiers)> {
    lines_read(modifiers, found)
        .into_iter()
        .map(|(lines, affixes)| (lines.len(), affixes))
        .collect()
}

// What reads the test items and the tables of the game: for the `game_tests.rs` only.

/// The tiers of `modifier` on `base_type`, in English.
pub(super) fn tiers(base_type: &str, modifier: &Modifier) -> Option<ModifierTiers> {
    let matches = modifier_stats(Language::English, modifier, None);
    let base = ItemBase {
        language: Language::English,
        base_type,
        item_level: None,
    };
    modifier_tiers(base, modifier, &matches)
}

/// The tiara and the bow copied by the owner from the in-game trade, English and French.
pub(super) const TIARA_EN: &str =
    include_str!("../../item/test-items/en-owner-trade-rare-helmet-fractured.txt");
pub(super) const TIARA_FR: &str =
    include_str!("../../item/test-items/fr-trade-rare-helmet-fractured.txt");
pub(super) const BOW_EN: &str =
    include_str!("../../item/test-items/en-owner-trade-rare-bow-mirrored.txt");
pub(super) const BOW_FR: &str =
    include_str!("../../item/test-items/fr-trade-rare-bow-mirrored.txt");

/// The item copied as `text`, as the parser reads it.
pub(super) fn parse(text: &str) -> Item {
    Item::parse(&CopiedItem::from_text(text).expect("an item")).expect("read")
}

/// The rare and magic items of the advanced copy, with the base type of a magic one.
pub(super) const ADVANCED_COPIES: [(&str, Option<&str>); 16] = [
    (
        include_str!("../../item/test-items/en-rare-crossbow-all-modifiers.txt"),
        None,
    ),
    (
        include_str!("../../item/test-items/en-rare-bow-fractured.txt"),
        None,
    ),
    (
        include_str!("../../item/test-items/en-magic-mace.txt"),
        Some("Temple Maul"),
    ),
    (
        include_str!("../../item/test-items/fr-rare-staff.txt"),
        None,
    ),
    (
        include_str!("../../item/test-items/fr-rare-body-armour.txt"),
        None,
    ),
    (
        include_str!("../../item/test-items/en-owner-rare-sceptre-sanctified.txt"),
        None,
    ),
    (
        include_str!("../../item/test-items/fr-rare-sceptre-sanctified.txt"),
        None,
    ),
    (
        include_str!("../../item/test-items/en-owner-rare-helmet-runeforged.txt"),
        None,
    ),
    (
        include_str!("../../item/test-items/fr-rare-helmet-runeforged.txt"),
        None,
    ),
    (
        include_str!("../../item/test-items/en-owner-rare-focus-unrevealed.txt"),
        None,
    ),
    (
        include_str!("../../item/test-items/fr-rare-focus-unrevealed.txt"),
        None,
    ),
    (
        include_str!("../../item/test-items/en-owner-rare-ring-breach.txt"),
        None,
    ),
    (
        include_str!("../../item/test-items/fr-rare-ring-breach.txt"),
        None,
    ),
    (
        include_str!("../../item/test-items/en-owner-rare-jewel-time-lost.txt"),
        None,
    ),
    (
        include_str!("../../item/test-items/fr-rare-jewel-time-lost.txt"),
        None,
    ),
    (
        include_str!("../../item/test-items/fr-rare-jewel-time-lost-large.txt"),
        None,
    ),
];

/// An item of `base_type` and `item_level` written in English, copied from the in-game trade
/// with at most `limits`, not sanctified.
pub(super) fn english_item(base_type: &str, item_level: u32, limits: AffixLimits) -> TradeItem<'_> {
    TradeItem {
        base: ItemBase {
            language: Language::English,
            base_type,
            item_level: Some(item_level),
        },
        limits,
        is_sanctified: false,
    }
}

/// The test item `item` as copied from the in-game trade, of `base_type` (its own, or the one
/// the names of the trade site tell for a magic item), with at most `limits`.
pub(super) fn trade_item<'a>(
    item: &'a Item,
    base_type: &'a str,
    limits: AffixLimits,
) -> TradeItem<'a> {
    TradeItem {
        base: ItemBase {
            language: item.language(),
            base_type,
            item_level: item.item_level(),
        },
        limits,
        is_sanctified: item.has_state(ItemState::Sanctified),
    }
}

/// The prefixes and suffixes `item` can have, as the application gives them
/// (`LineStats::affix_limits`): 2 of each on a rare jewel.
pub(super) fn affix_limits(item: &Item) -> AffixLimits {
    crate::trade::stat_filters::LineStats::new(item, item.modifiers(), None).affix_limits(item)
}

/// A modifier of the advanced copy written as the in-game trade writes it.
pub(super) struct Written {
    /// The indexes of the modifiers its lines make, one per stat.
    pub(super) modifiers: std::ops::Range<usize>,
    pub(super) affix: Affix,
    /// Its tier, found by the ranges of the copy.
    pub(super) tier: Option<u32>,
}

/// The explicit modifiers of an advanced copy as the in-game trade writes them, one per
/// stat without header nor range, and what the copy wrote of each.
pub(super) fn as_in_game_trade(item: &Item, base_type: &str) -> (Vec<Modifier>, Vec<Written>) {
    let mut modifiers = Vec::new();
    let mut written = Vec::new();
    for modifier in item.modifiers() {
        let ModifierKind::Explicit(Some(affix)) = modifier.kind else {
            modifiers.push(modifier.clone());
            continue;
        };
        if modifier.is_unrevealed {
            modifiers.push(modifier.clone());
            continue;
        }
        let matches = modifier_stats(item.language(), modifier, None);
        let base = ItemBase {
            language: item.language(),
            base_type,
            item_level: None,
        };
        let tier =
            modifier_tiers(base, modifier, &matches).and_then(|tiers| Some(tiers.tier()?.number));
        let start = modifiers.len();
        for line_match in &matches {
            let mut lines = modifier.lines[line_match.lines.clone()].to_vec();
            for roll in lines.iter_mut().flat_map(|line| &mut line.rolls) {
                roll.range = None;
            }
            modifiers.push(Modifier {
                kind: ModifierKind::Explicit(None),
                origin: modifier.origin,
                name: None,
                tier: None,
                tags: Vec::new(),
                is_unrevealed: false,
                lines,
                doubt: None,
            });
        }
        written.push(Written {
            modifiers: start..modifiers.len(),
            affix,
            tier,
        });
    }
    (modifiers, written)
}
