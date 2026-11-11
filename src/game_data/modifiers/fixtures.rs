//! What the tests of the modifier tables share: the modifiers, the tables and the items they
//! are written with, and the readers of what the search found.

use crate::game_data::bases::{BaseSpawn, Domain};
use crate::game_data::lines::modifier_stats;
use crate::game_data::modifiers::ModifierTiers;
use crate::game_data::modifiers::lookup::modifier_tiers;
use crate::game_data::modifiers::trade_tiers::{AffixLimits, FoundTiers};
use crate::item::{
    Affix, CopiedItem, Item, Modifier, ModifierKind, ModifierOrigin, PartTier, Range, StatLine,
};
use crate::language::Language;

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

/// The tiers of `modifier` on `base_type`, in English.
pub(super) fn tiers(base_type: &str, modifier: &Modifier) -> Option<ModifierTiers> {
    let matches = modifier_stats(Language::English, modifier, None);
    modifier_tiers(Language::English, base_type, None, modifier, &matches)
}

/// A modifier of `affix` with `lines`, the game marking it desecrated.
pub(super) fn desecrated(affix: Affix, lines: &[&str]) -> Modifier {
    Modifier {
        origin: Some(ModifierOrigin::Desecrated),
        ..modifier(Some(affix), lines)
    }
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

/// The tier number of each affix a modifier can be.
pub(super) type AffixTiers = Vec<(Affix, u32)>;

/// The texts of the lines of each modifier read from its values, and the tier number of
/// each affix it can be.
pub(super) type LinesRead = Vec<(Vec<String>, AffixTiers)>;

/// The text of each line split, and the affix and tier number of its pure modifier.
pub(super) type Added = Vec<(String, Affix, u32)>;

/// The item copied as `text`, as the parser reads it.
pub(super) fn parse(text: &str) -> Item {
    Item::parse(&CopiedItem::from_text(text).expect("an item")).expect("read")
}

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
        let tier = modifier_tiers(item.language(), base_type, None, modifier, &matches)
            .and_then(|tiers| Some(tiers.tier()?.number));
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
