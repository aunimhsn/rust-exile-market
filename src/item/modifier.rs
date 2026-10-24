//! The modifiers of an item: their header in the advanced copy, their marker in the plain
//! copy, and the values of their lines.

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
}

impl Marker {
    const ALL: [Self; 6] = [
        Self::Implicit,
        Self::Enchant,
        Self::Rune,
        Self::Crafted,
        Self::Fractured,
        Self::Desecrated,
    ];

    fn suffix(self) -> &'static str {
        match self {
            Self::Implicit => " (implicit)",
            Self::Enchant => " (enchant)",
            Self::Rune => " (rune)",
            Self::Crafted => " (crafted)",
            Self::Fractured => " (fractured)",
            Self::Desecrated => " (desecrated)",
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
            Self::Crafted | Self::Fractured | Self::Desecrated => None,
        }
    }

    pub fn origin(self) -> Option<ModifierOrigin> {
        match self {
            Self::Crafted => Some(ModifierOrigin::Crafted),
            Self::Fractured => Some(ModifierOrigin::Fractured),
            Self::Desecrated => Some(ModifierOrigin::Desecrated),
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

/// A value of a line, with the range of its tier when the item was copied with its advanced
/// description, e.g. `10(1-16)`.
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

/// A modifier, with one line or more.
#[derive(Debug, Clone, PartialEq)]
pub struct Modifier {
    pub kind: ModifierKind,
    pub origin: Option<ModifierOrigin>,
    /// The name of a prefix or suffix, e.g. `Crackling`.
    pub name: Option<String>,
    /// 1 is the best tier; only in the advanced copy.
    pub tier: Option<u32>,
    /// The tags of the advanced copy, e.g. `Damage`, `Elemental`.
    pub tags: Vec<String>,
    pub lines: Vec<StatLine>,
}

/// What a header line of the advanced copy tells.
#[derive(Debug, PartialEq)]
pub(super) enum Header {
    Known(Modifier),
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
            lines: Vec::new(),
        }
    }

    /// A modifier of one line without header, e.g. `+15% to Fire Resistance (crafted)`; its
    /// marker tells its kind, or else `kind`.
    pub(super) fn plain(line: &str, kind: ModifierKind) -> Self {
        let (text, marker) = Marker::take(line);
        let mut modifier = Self::new(
            marker.and_then(Marker::kind).unwrap_or(kind),
            marker.and_then(Marker::origin),
        );
        modifier.lines.push(StatLine::parse(text));
        modifier
    }

    /// A modifier whose header is unknown: explicit, without affix, name nor tier.
    pub(super) fn unknown() -> Self {
        Self::new(ModifierKind::Explicit(None), None)
    }

    /// Adds a line under the header; its marker tells the origin when the header doesn't.
    pub(super) fn push_line(&mut self, line: &str) {
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
        Some(Self::read(body, labels).map_or(Self::Unknown, Self::Known))
    }

    fn read(body: &str, labels: &Labels) -> Option<Modifier> {
        let (main, tags) = match body.split_once(" — ") {
            Some((main, tags)) => (main, tags.split(", ").map(str::to_owned).collect()),
            None => (body, Vec::new()),
        };
        let (main, tier) = take_tier(main, labels.tier);
        let (main, name) = take_name(main);
        let main = main.trim().trim_end_matches(':').trim();

        let &(kind_label, kind) = labels
            .modifier_kinds
            .iter()
            .find(|(label, _)| main.ends_with(label))?;
        let origin_words = main.strip_suffix(kind_label)?.trim();
        let origin = if origin_words.is_empty() {
            None
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
            ..Modifier::new(kind, origin)
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
    pub(super) fn parse(line: &str) -> Self {
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

/// Reads `(min-max)` after a value, if it is a range; the characters are only consumed
/// when it is one.
fn read_range(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> Option<Range> {
    let mut lookahead = chars.clone();
    lookahead.next(); // (
    let mut inside = String::new();
    for c in lookahead.by_ref() {
        if c == ')' {
            break;
        }
        inside.push(c);
    }
    let (low, high) = split_range(&inside)?;
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

    fn header(line: &str, language: Language) -> Modifier {
        match Header::parse(line, Labels::of(language)) {
            Some(Header::Known(modifier)) => modifier,
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
    fn keeps_the_parentheses_that_are_not_ranges() {
        let line = StatLine::parse("Adds 5 (cold) and 3(a-b)");

        assert_eq!(line.text, "Adds 5 (cold) and 3(a-b)");
        assert_eq!(line.rolls, [roll(5.0, None), roll(3.0, None)]);
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
    fn a_marker_tells_the_kind_or_the_origin_of_a_plain_line() {
        let rune = Modifier::plain(
            "18% increased Physical Damage (rune)",
            ModifierKind::Implicit,
        );
        assert_eq!(rune.kind, ModifierKind::Rune);
        assert_eq!(rune.lines[0].text, "18% increased Physical Damage");

        let crafted = Modifier::plain("+12 to Strength (crafted)", ModifierKind::Explicit(None));
        assert_eq!(
            (crafted.kind, crafted.origin),
            (ModifierKind::Explicit(None), Some(ModifierOrigin::Crafted))
        );
    }
}
