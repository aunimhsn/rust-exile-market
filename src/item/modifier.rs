//! The modifiers of an item: their header in the advanced copy, the marker of a line without
//! header (a rune, an enchantment, or any modifier copied from the trade of the game), and the
//! values of their lines.

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

/// The marker at the end of a modifier line, in English in every language, e.g. ` (rune)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Marker {
    Implicit,
    Enchant,
    Rune,
    Crafted,
    Fractured,
    Desecrated,
    /// The Vaal modifier of a corrupted unique copied from the trade of the game.
    Mutated,
}

impl Marker {
    const ALL: [Self; 7] = [
        Self::Implicit,
        Self::Enchant,
        Self::Rune,
        Self::Crafted,
        Self::Fractured,
        Self::Desecrated,
        Self::Mutated,
    ];

    fn suffix(self) -> &'static str {
        match self {
            Self::Implicit => " (implicit)",
            Self::Enchant => " (enchant)",
            Self::Rune => " (rune)",
            Self::Crafted => " (crafted)",
            Self::Fractured => " (fractured)",
            Self::Desecrated => " (desecrated)",
            Self::Mutated => " (mutated)",
        }
    }

    /// A line without its marker, and the marker found.
    pub fn take(line: &str) -> (&str, Option<Self>) {
        Self::ALL
            .into_iter()
            .find_map(|marker| Some((line.strip_suffix(marker.suffix())?, Some(marker))))
            .unwrap_or((line, None))
    }

    /// The kind of the modifiers that the marker alone tells, e.g. a rune.
    pub fn kind(self) -> Option<ModifierKind> {
        match self {
            Self::Implicit => Some(ModifierKind::Implicit),
            Self::Enchant => Some(ModifierKind::Enchant),
            Self::Rune => Some(ModifierKind::Rune),
            Self::Crafted | Self::Fractured | Self::Desecrated | Self::Mutated => None,
        }
    }

    pub fn origin(self) -> Option<ModifierOrigin> {
        match self {
            Self::Crafted => Some(ModifierOrigin::Crafted),
            Self::Fractured => Some(ModifierOrigin::Fractured),
            Self::Desecrated => Some(ModifierOrigin::Desecrated),
            Self::Mutated => Some(ModifierOrigin::Vaal),
            Self::Implicit | Self::Enchant | Self::Rune => None,
        }
    }
}

/// The range of a tier, from its lowest to its highest value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Range {
    pub min: f64,
    pub max: f64,
}

impl Range {
    /// `value` brought back within the range; never panics, unlike `f64::clamp`, on a range
    /// written from its highest value or with a NaN end.
    #[must_use]
    pub fn clamp(self, value: f64) -> f64 {
        value.max(self.min).min(self.max)
    }

    /// The range widened to hold `value`, e.g. the value of a legacy unique, rolled in a
    /// range that changed since.
    #[must_use]
    pub fn holding(self, value: f64) -> Self {
        Self {
            min: self.min.min(value),
            max: self.max.max(value),
        }
    }

    /// The range from the lowest to the highest value of both.
    #[must_use]
    pub fn spanning(self, other: Self) -> Self {
        self.holding(other.min).holding(other.max)
    }

    /// The range of the absolute values: the game shows a negative roll without its sign
    /// ("20% reduced"); none for a range across zero, which needs its sign.
    #[must_use]
    pub fn magnitude(self) -> Option<Self> {
        if self.min * self.max < 0.0 {
            return None;
        }
        let (first, second) = (self.min.abs(), self.max.abs());
        Some(Self {
            min: first.min(second),
            max: first.max(second),
        })
    }

    /// The ranges of `rolls`, values without their sign each with its range, as written under
    /// a line: none when each value is fixed, its range of one value being the value, as the
    /// advanced copy most often writes it, without range ("T2", not "T2 2"; owner's choice on
    /// 2026-10-30); a range of one value other than the value stays, `29(30)%` of a sanctified
    /// item ("T2 30").
    #[must_use]
    pub fn unless_fixed(rolls: impl IntoIterator<Item = (f64, Self)>) -> Vec<Self> {
        let rolls: Vec<(f64, Self)> = rolls.into_iter().collect();
        let fixed = rolls.iter().all(|(value, range)| {
            (range.max - range.min).abs() < f64::EPSILON
                && (value.abs() - range.min).abs() < f64::EPSILON
        });
        if fixed {
            return Vec::new();
        }
        rolls.into_iter().map(|(_, range)| range).collect()
    }
}

/// A value of a line, with the range of its tier when the item was copied with its advanced
/// description, e.g. `10(1-16)`, or when its tier was found from the values of an item copied
/// from the in-game trade.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Roll {
    pub value: f64,
    pub range: Option<Range>,
}

/// A line of a modifier: its text without the ranges and the markers, and its values.
#[derive(Debug, Clone, PartialEq)]
pub struct StatLine {
    /// E.g. `Adds 10 to 273 Lightning Damage`.
    pub text: String,
    pub rolls: Vec<Roll>,
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
    /// of the item, or a tier the game doesn't write (the Genesis tree).
    pub hides_number: bool,
    /// The range over these tiers of each value of that line its stat counts, in their order,
    /// as the game writes it: without its sign.
    pub ranges: Vec<Range>,
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

/// What a header line of the advanced copy tells.
#[derive(Debug, PartialEq)]
pub(super) enum Header {
    Known(Box<Modifier>),
    /// A header whose kind isn't in the labels: its lines make a modifier of their own.
    Unknown,
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

impl Header {
    /// Reads a header line of the advanced copy, e.g.
    /// `{ Fractured Prefix Modifier "Electrocuting" (Tier: 2) — Damage, Lightning }`; `None`
    /// when the line isn't a header.
    pub(super) fn parse(line: &str, labels: &Labels) -> Option<Self> {
        let body = line.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
        Some(
            Self::read(body, labels)
                .map_or(Self::Unknown, |modifier| Self::Known(Box::new(modifier))),
        )
    }

    fn read(body: &str, labels: &Labels) -> Option<Modifier> {
        let (main, tags) = match body.split_once(" — ") {
            Some((main, tags)) => (main, tags.split(", ").map(str::to_owned).collect()),
            None => (body, Vec::new()),
        };
        let (main, tier) = take_tier(main, labels.tier);
        let (main, name) = take_name(main);
        let main = main.trim().trim_end_matches(':').trim();

        let header_kind = labels
            .modifier_kinds
            .iter()
            .find(|header_kind| main.ends_with(header_kind.label))?;
        let origin_words = main.strip_suffix(header_kind.label)?.trim();
        let origin = if origin_words.is_empty() {
            header_kind.origin
        } else {
            Some(
                labels
                    .modifier_origins
                    .iter()
                    .find(|(label, _)| *label == origin_words)?
                    .1,
            )
        };
        Some(Modifier {
            name,
            tier,
            tags,
            ..Modifier::new(header_kind.kind, origin)
        })
    }
}

/// The text before the tier, and the tier: `Prefix Modifier "Icy" (Tier: 8)` gives
/// `Prefix Modifier "Icy"` and 8.
fn take_tier<'a>(main: &'a str, tier_label: &str) -> (&'a str, Option<u32>) {
    let Some((before, after)) = main.split_once(&format!("({tier_label}")) else {
        return (main, None);
    };
    let tier = after
        .split_once(')')
        .and_then(|(tier, _)| tier.trim().parse().ok());
    (before, tier)
}

/// The text before the name, and the name between quotes (`"Icy"` or `« d'Amanamu »`).
fn take_name(main: &str) -> (&str, Option<String>) {
    for (open, close) in [('"', '"'), ('«', '»')] {
        if let Some((before, rest)) = main.split_once(open)
            && let Some((name, _)) = rest.split_once(close)
        {
            return (before, Some(name.trim().to_owned()));
        }
    }
    (main, None)
}

impl StatLine {
    /// Reads the values of a line, and removes their ranges from its text:
    /// `Adds 10(1-16) to 273(239-300) Lightning Damage` gives `Adds 10 to 273 Lightning
    /// Damage` and the values 10 and 273 with their ranges.
    pub(crate) fn parse(line: &str) -> Self {
        let mut text = String::with_capacity(line.len());
        let mut rolls = Vec::new();
        let mut chars = line.chars().peekable();
        let mut previous: Option<char> = None;

        while let Some(c) = chars.next() {
            if !c.is_ascii_digit() {
                text.push(c);
                previous = Some(c);
                continue;
            }
            let mut number = String::from(c);
            while let Some(&next) = chars.peek() {
                if next.is_ascii_digit() || next == '.' {
                    number.push(next);
                    chars.next();
                } else {
                    break;
                }
            }
            text.push_str(&number);
            // A minus sign right before the number, not inside a word: `-10% to…`.
            let negative = previous == Some('-') && !text_before_sign_is_word(&text, &number);
            previous = number.chars().last();
            let Ok(value) = number.parse::<f64>() else {
                continue;
            };
            let value = if negative { -value } else { value };
            let range = if chars.peek() == Some(&'(') {
                read_range(&mut chars)
            } else {
                None
            };
            rolls.push(Roll { value, range });
        }
        Self { text, rolls }
    }
}

/// Whether the character before the minus sign of `number`, at the end of `text`, is a
/// letter or a digit (`X-10` is not a negative number).
fn text_before_sign_is_word(text: &str, number: &str) -> bool {
    text.strip_suffix(number)
        .and_then(|before| before.strip_suffix('-'))
        .and_then(|before| before.chars().last())
        .is_some_and(char::is_alphanumeric)
}

/// Reads `(min-max)` after a value, if it is a range, or `(value)`, a range of one value
/// (`29(30)% reduced Attribute Requirements` of a sanctified item): closed, with digits, points
/// and minus signs only. The characters are only consumed when it is one.
fn read_range(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> Option<Range> {
    let mut lookahead = chars.clone();
    lookahead.next(); // (
    let mut inside = String::new();
    loop {
        match lookahead.next()? {
            ')' => break,
            c if c.is_ascii_digit() || c == '.' || c == '-' => inside.push(c),
            _ => return None,
        }
    }
    let (low, high) = split_range(&inside).unwrap_or((&inside, &inside));
    let (low, high) = (low.parse::<f64>().ok()?, high.parse::<f64>().ok()?);
    *chars = lookahead;
    Some(Range {
        min: low.min(high),
        max: low.max(high),
    })
}

/// Splits `1-16` or `-10--5` at the minus sign between the two numbers.
fn split_range(inside: &str) -> Option<(&str, &str)> {
    let start = usize::from(inside.starts_with('-'));
    let (index, _) = inside.char_indices().skip(start).find(|&(_, c)| c == '-')?;
    let low = inside.get(..index)?;
    let high = inside.get(index + 1..)?;
    Some((low, high))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::language::Language;

    fn roll(value: f64, range: Option<(f64, f64)>) -> Roll {
        Roll {
            value,
            range: range.map(|(min, max)| Range { min, max }),
        }
    }

    #[test]
    fn a_range_spanning_another_goes_from_the_lowest_to_the_highest_of_both() {
        let range = |min, max| Range { min, max };
        // Apart, in either order.
        assert_eq!(range(1.0, 4.0).spanning(range(7.0, 9.0)), range(1.0, 9.0));
        assert_eq!(range(7.0, 9.0).spanning(range(1.0, 4.0)), range(1.0, 9.0));
        // One within the other.
        assert_eq!(range(1.0, 9.0).spanning(range(3.0, 4.0)), range(1.0, 9.0));
        assert_eq!(range(3.0, 4.0).spanning(range(1.0, 9.0)), range(1.0, 9.0));
    }

    fn header(line: &str, language: Language) -> Modifier {
        match Header::parse(line, Labels::of(language)) {
            Some(Header::Known(modifier)) => *modifier,
            other => panic!("{line}: {other:?}"),
        }
    }

    #[test]
    fn reads_the_values_and_their_ranges() {
        let line = StatLine::parse("Adds 10(1-16) to 273(239-300) Lightning Damage");

        assert_eq!(line.text, "Adds 10 to 273 Lightning Damage");
        assert_eq!(
            line.rolls,
            [
                roll(10.0, Some((1.0, 16.0))),
                roll(273.0, Some((239.0, 300.0)))
            ]
        );
    }

    #[test]
    fn reads_decimal_negative_and_reversed_values() {
        let line = StatLine::parse("Leeches 7.59(7-7.9)% of Physical Damage as Mana");
        assert_eq!(line.rolls, [roll(7.59, Some((7.0, 7.9)))]);

        let line = StatLine::parse("-10% to Cold Resistance");
        assert_eq!(line.rolls, [roll(-10.0, None)]);

        let line = StatLine::parse("Adds 1(-10--5) to X-10");
        assert_eq!(
            line.rolls,
            [roll(1.0, Some((-10.0, -5.0))), roll(10.0, None)]
        );

        let line =
            StatLine::parse("Réduit la durée des saignements vous affectant de 41(45-41) %.");
        assert_eq!(
            line.text,
            "Réduit la durée des saignements vous affectant de 41 %."
        );
        assert_eq!(line.rolls, [roll(41.0, Some((41.0, 45.0)))]);
    }

    #[test]
    fn reads_a_range_of_one_value() {
        let line = StatLine::parse("29(30)% reduced Attribute Requirements");
        assert_eq!(line.text, "29% reduced Attribute Requirements");
        assert_eq!(line.rolls, [roll(29.0, Some((30.0, 30.0)))]);

        let line = StatLine::parse("0.5(0.6)% of Physical Attack Damage Leeched as Life");
        assert_eq!(line.text, "0.5% of Physical Attack Damage Leeched as Life");
        assert_eq!(line.rolls, [roll(0.5, Some((0.6, 0.6)))]);
    }

    #[test]
    fn keeps_the_parentheses_that_are_not_ranges() {
        let line = StatLine::parse("Adds 5 (cold) and 3(a-b)");
        assert_eq!(line.text, "Adds 5 (cold) and 3(a-b)");
        assert_eq!(line.rolls, [roll(5.0, None), roll(3.0, None)]);

        // Only a closed range of digits, points and minus signs: what `f64` reads besides
        // (`nan`, `1e3`, `+5`) stays in the text.
        for text in [
            "4() a",
            "6(x) a",
            "7(nan) a",
            "8(inf) a",
            "9(1e3) a",
            "1(+5) a",
            "2(5 - 6) a",
            "3(10",
        ] {
            let line = StatLine::parse(text);
            assert_eq!(line.text, text);
            assert_eq!(
                line.rolls.first().and_then(|roll| roll.range),
                None,
                "{text}"
            );
        }
    }

    #[test]
    fn reads_the_headers_in_english() {
        let modifier = header(
            "{ Fractured Prefix Modifier \"Electrocuting\" (Tier: 2) — Damage, Lightning }",
            Language::English,
        );

        assert_eq!(modifier.kind, ModifierKind::Explicit(Some(Affix::Prefix)));
        assert_eq!(modifier.origin, Some(ModifierOrigin::Fractured));
        assert_eq!(modifier.name.as_deref(), Some("Electrocuting"));
        assert_eq!(modifier.tier, Some(2));
        assert_eq!(modifier.tags, ["Damage", "Lightning"]);

        let implicit = header("{ Implicit Modifier }", Language::English);
        assert_eq!(implicit.kind, ModifierKind::Implicit);
        assert_eq!((implicit.tier, implicit.tags.len()), (None, 0));
    }

    #[test]
    fn reads_the_headers_in_french() {
        let modifier = header(
            "{ Profané Préfixe : « d'Amanamu » (Palier : 1) — Dégâts, Chaos }",
            Language::French,
        );

        assert_eq!(modifier.kind, ModifierKind::Explicit(Some(Affix::Prefix)));
        assert_eq!(modifier.origin, Some(ModifierOrigin::Desecrated));
        assert_eq!(modifier.name.as_deref(), Some("d'Amanamu"));
        assert_eq!(modifier.tier, Some(1));

        let unique = header("{ Modificateur d'Objet unique }", Language::French);
        assert_eq!(unique.kind, ModifierKind::Explicit(None));
    }

    #[test]
    fn tells_the_unknown_headers_and_the_lines_that_are_not_headers() {
        let labels = Labels::of(Language::English);

        assert_eq!(
            Header::parse("{ Strange Modifier }", labels),
            Some(Header::Unknown)
        );
        assert_eq!(Header::parse("Adds 1 to 2 Fire Damage", labels), None);
    }

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
        let mut prefix = header(r#"{ Prefix Modifier "Veiled" }"#, Language::English);
        prefix.push_line("Desecrated Prefix", labels);
        assert!(prefix.unrevealed && prefix.lines.is_empty());
        // The words of the other affix, or under a line, are a line.
        let mut suffix = header(r#"{ Suffix Modifier "of the Veil" }"#, Language::English);
        suffix.push_line("Desecrated Prefix", labels);
        let mut after = header(r#"{ Prefix Modifier "Veiled" }"#, Language::English);
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
