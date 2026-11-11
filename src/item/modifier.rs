//! The modifiers of an item: their kind, their origin, their tier and their lines, read under
//! their header in the advanced copy (`header`) or with the marker of a line without header
//! (`marker`: a rune, an enchantment, or any modifier copied from the trade of the game); the
//! values of their lines (`stat_line`).

mod header;
mod marker;
mod stat_line;

pub(super) use header::Header;
pub(super) use marker::Marker;
/// The values of a line, built by hand in the tests.
#[cfg(test)]
pub use stat_line::Roll;
pub use stat_line::{Range, StatLine};

use super::labels::Labels;

/// Where a modifier comes from on the item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModifierKind {
    Implicit,
    /// A prefix, a suffix, or a modifier of a unique (`None`, also when the copy doesn't say).
    Explicit(Option<Affix>),
    Enchant,
    /// Added by a rune or a soul core in a socket.
    Rune,
    /// Added by the corruption of the item.
    Corruption,
}

/// The two kinds of explicit modifiers of a magic or rare item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Affix {
    Prefix,
    Suffix,
}

/// How a modifier was added, when it isn't rolled by the item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModifierOrigin {
    Crafted,
    Desecrated,
    Fractured,
    /// The Vaal modifier of a corrupted unique, told by its header (`{ Vaal Unique Modifier }`)
    /// or its marker (` (mutated)`); "mutated" on the trade site.
    Vaal,
}

/// The currency that adds a modifier of its own, whose tier the game doesn't write: named after
/// the ranges of its line, so that a line without tier says why.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Currency {
    Essence,
    Alloy,
}

/// A lord of the Abyss whose modifier a desecration reveals, of one tier on its base: named
/// before the affix of a line that can be his or an ordinary modifier ("Amanamu Prefix"), so
/// that its row without tier number reads as "the only one", not as an unknown tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lord {
    Amanamu,
    Kurgal,
    Ulaman,
}

impl Lord {
    /// His name, the same in every language of the game.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Amanamu => "Amanamu",
            Self::Kurgal => "Kurgal",
            Self::Ulaman => "Ulaman",
        }
    }
}
/// The tier of a modifier for one affix it can be, found from its values on an item copied
/// from the in-game trade: a prefix of tier 1, 16–19.
#[derive(Debug, Clone, PartialEq)]
pub struct AffixTier {
    pub affix: Affix,
    /// 1 is the best.
    pub tier: u32,
    /// Whether the number of its tier isn't written: its modifier has one tier only on the base
    /// of the item, whose T1 would read as "the best" while it is "the only one", or a tier the
    /// game doesn't write (the Genesis tree).
    pub hides_number: bool,
    /// The range of each value its stats count, in their order, as the game writes it:
    /// without its sign; none for fixed values ([`Range::unless_fixed`]).
    pub ranges: Vec<Range>,
    /// The lord of the Abyss whose modifier it is, named before the affix.
    pub lord: Option<Lord>,
}

/// What the values of a modifier copied from the in-game trade leave in doubt about its tier.
#[derive(Debug, Clone, PartialEq)]
pub enum TierDoubt {
    /// The tier of each affix, the item having room for either (the `18% increased Rarity of
    /// Items found` of a helmet with a prefix and a suffix open): the modifier stays explicit,
    /// without affix nor tier.
    EitherAffix {
        prefix: AffixTier,
        suffix: AffixTier,
    },
    /// A line that adds up two modifiers of its stat (the `258% increased Physical Damage` of
    /// a bow: a pure 170–179 and the 75–79 of the hybrid with `+195 to Accuracy Rating`): the
    /// modifier holds the lines of the hybrid, or that line alone for two pure modifiers,
    /// without tier; of their affix, or explicit without affix for a prefix and a suffix.
    Split {
        /// The index among the lines of the modifier of the line that adds them up.
        line: usize,
        /// The modifier of every line: a pure modifier of that line, or a hybrid with the
        /// others; of two pure modifiers of a prefix and a suffix, the prefix.
        whole: PartTier,
        /// The pure modifier of that line only, of another type.
        added: PartTier,
    },
}

/// The tiers of a part of a line that adds up two modifiers of its stat, found from the value
/// of that line: a pure prefix of tier 1 or 2, 155–179.
#[derive(Debug, Clone, PartialEq)]
pub struct PartTier {
    pub affix: Affix,
    /// The tiers it may be, the best first: one, or two neighbouring tiers.
    pub tiers: Vec<u32>,
    /// Whether the number of its tiers isn't written: its modifier has one tier only on the base
    /// of the item, or a tier the game doesn't write (the Genesis tree, an essence, an alloy).
    pub hides_number: bool,
    /// The range over these tiers of each value of that line its stat counts, in their order,
    /// as the game writes it: without its sign.
    pub ranges: Vec<Range>,
    /// The currency that adds its modifier, an essence or an alloy, named after its ranges.
    pub currency: Option<Currency>,
}

/// A modifier, with one line or more.
#[derive(Debug, Clone, PartialEq)]
pub struct Modifier {
    pub kind: ModifierKind,
    pub origin: Option<ModifierOrigin>,
    /// The name of a prefix or suffix, e.g. `Crackling`.
    pub name: Option<String>,
    /// 1 is the best tier; in the advanced copy, or found from the values of an item copied
    /// from the in-game trade.
    pub tier: Option<u32>,
    /// The tags of the advanced copy, e.g. `Damage`, `Elemental`.
    pub tags: Vec<String>,
    /// A desecrated modifier not revealed yet: its affix is known, not its lines, so it has
    /// none.
    pub unrevealed: bool,
    pub lines: Vec<StatLine>,
    /// What its values leave in doubt about its tier, when it was found from them.
    pub doubt: Option<TierDoubt>,
}
impl Modifier {
    /// A modifier without header nor line yet.
    fn new(kind: ModifierKind, origin: Option<ModifierOrigin>) -> Self {
        Self {
            kind,
            origin,
            name: None,
            tier: None,
            tags: Vec::new(),
            unrevealed: false,
            lines: Vec::new(),
            doubt: None,
        }
    }

    /// A modifier of one line without header, of the `kind` its section tells, e.g.
    /// `18% increased Physical Damage (rune)`, and of the origin its marker tells, e.g.
    /// `+5% to Critical Hit Chance (fractured)` copied from the trade of the game.
    pub(super) fn marked(line: &str, kind: ModifierKind) -> Self {
        let (text, marker) = Marker::take(line);
        let mut modifier = Self::new(kind, marker.and_then(Marker::origin));
        modifier.lines.push(StatLine::parse(text));
        modifier
    }

    /// A modifier whose header is unknown: explicit, without affix, name nor tier.
    pub(super) fn unknown() -> Self {
        Self::new(ModifierKind::Explicit(None), None)
    }

    /// Adds a line under the header; its marker tells the origin when the header doesn't. The
    /// only line of a prefix or suffix not revealed yet (`Desecrated Prefix`, in `labels`)
    /// marks it unrevealed instead, and no line follows it.
    pub(super) fn push_line(&mut self, line: &str, labels: &Labels) {
        if self.unrevealed {
            return;
        }
        if let ModifierKind::Explicit(Some(affix)) = self.kind
            && self.lines.is_empty()
            && labels.unrevealed.contains(&(line.trim(), affix))
        {
            self.unrevealed = true;
            return;
        }
        let (text, marker) = Marker::take(line);
        if self.origin.is_none() {
            self.origin = marker.and_then(Marker::origin);
        }
        self.lines.push(StatLine::parse(text));
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::language::Language;

    #[test]
    fn a_marked_line_is_read_without_its_marker() {
        let rune = Modifier::marked("18% increased Physical Damage (rune)", ModifierKind::Rune);
        assert_eq!((rune.kind, rune.origin), (ModifierKind::Rune, None));
        assert_eq!(rune.lines[0].text, "18% increased Physical Damage");
    }

    #[test]
    fn a_marked_line_has_the_origin_of_its_marker() {
        let explicit = ModifierKind::Explicit(None);
        for (line, origin) in [
            (
                "+5% to Critical Hit Chance (fractured)",
                ModifierOrigin::Fractured,
            ),
            (
                "Adds 36 to 56 Physical Damage (desecrated)",
                ModifierOrigin::Desecrated,
            ),
            ("+12 to Strength (crafted)", ModifierOrigin::Crafted),
            (
                "14% increased Energy Shield Recovery rate (mutated)",
                ModifierOrigin::Vaal,
            ),
        ] {
            let modifier = Modifier::marked(line, explicit);
            assert_eq!((modifier.kind, modifier.origin), (explicit, Some(origin)));
            assert!(!modifier.lines[0].text.ends_with(')'), "{line}");
        }
        assert_eq!(
            Modifier::marked("+195 to Accuracy Rating", explicit).origin,
            None
        );
    }

    #[test]
    fn a_marker_tells_the_origin_of_a_line_under_a_header() {
        let mut crafted = Modifier::unknown();
        crafted.push_line("+12 to Strength (crafted)", Labels::of(Language::English));
        assert_eq!(crafted.origin, Some(ModifierOrigin::Crafted));
        assert_eq!(crafted.lines[0].text, "+12 to Strength");
    }

    #[test]
    fn the_only_line_of_an_unrevealed_modifier_matches_its_affix() {
        let labels = Labels::of(Language::English);
        let of_affix = |affix| Modifier::new(ModifierKind::Explicit(Some(affix)), None);
        let mut prefix = of_affix(Affix::Prefix);
        prefix.push_line("Desecrated Prefix", labels);
        assert!(prefix.unrevealed && prefix.lines.is_empty());
        // The words of the other affix, or under a line, are a line.
        let mut suffix = of_affix(Affix::Suffix);
        suffix.push_line("Desecrated Prefix", labels);
        let mut after = of_affix(Affix::Prefix);
        after.push_line("+12 to Strength", labels);
        after.push_line("Desecrated Prefix", labels);
        for modifier in [suffix, after] {
            assert!(!modifier.unrevealed);
            assert_eq!(
                modifier.lines.last().map(|line| line.text.as_str()),
                Some("Desecrated Prefix")
            );
        }
    }
}
