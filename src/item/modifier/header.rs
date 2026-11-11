//! The header of a modifier in the advanced copy, e.g. `{ Prefix Modifier "Icy" (Tier: 8) —
//! Damage, Cold }`: its kind, its origin, its name, its tier and its tags.

use super::Modifier;
use crate::item::labels::Labels;

/// What a header line of the advanced copy tells.
#[derive(Debug, PartialEq)]
pub enum Header {
    Known(Box<Modifier>),
    /// A header whose kind isn't in the labels: its lines make a modifier of their own.
    Unknown,
}

impl Header {
    /// Reads a header line of the advanced copy, e.g.
    /// `{ Fractured Prefix Modifier "Electrocuting" (Tier: 2) — Damage, Lightning }`; `None`
    /// when the line isn't a header.
    pub fn parse(line: &str, labels: &Labels) -> Option<Self> {
        let body = line.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
        Some(
            Self::read(body, labels)
                .map_or(Self::Unknown, |modifier| Self::Known(Box::new(modifier))),
        )
    }

    /// The modifier a header tells from its `body`, between its braces; none when its kind
    /// or its origin isn't in the labels.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::{Affix, ModifierKind, ModifierOrigin};
    use crate::language::Language;

    fn header(line: &str, language: Language) -> Modifier {
        match Header::parse(line, Labels::of(language)) {
            Some(Header::Known(modifier)) => *modifier,
            other => panic!("{line}: {other:?}"),
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
}
