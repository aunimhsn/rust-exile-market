//! The difference between a modifier line of an offer and the same line of the copied item,
//! e.g. `+48 to maximum Life` against `+55 to maximum Life`: −7, worse.
//!
//! Both lines are in the language of the item: its offers are searched on the trade site of
//! its language. Two lines are the same when they read the same with `#` in place of each
//! number; a line with several numbers (`Adds 5 to 82 Lightning Damage`) compares their
//! averages.

use crate::item::{Modifier, ModifierKind};
use crate::trade::offers::ModifierKind as OfferKind;

/// The words that make a line read the other way: "reduced Movement Speed" is worse when its
/// number is higher, "increased Mana Cost" too, and "reduced Mana Cost" is better.
const LESS_WORDS: [&str; 8] = [
    "reduced",
    "less",
    "réduit",
    "réduite",
    "réduits",
    "réduites",
    "en moins",
    "moins de",
];
/// What the player wants lower: costs, charges used, damage taken, life lost…
const BAD_WORDS: [&str; 12] = [
    "Cost",
    "cost",
    "Costs",
    "costs",
    "Charges used",
    "Damage taken",
    "Requirements",
    "Lose",
    "Coût",
    "coût",
    "subis",
    "Perd",
];
/// What the line acts on when the player isn't its subject: the better side of a number is
/// unknown (more life for the enemies, less damage on them…).
const OTHER_SUBJECTS: [&str; 6] = [
    "Enemies", "enemies", "Monsters", "monsters", "ennemis", "monstres",
];

/// Which side of a number is better.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Higher,
    Lower,
    Unknown,
}

/// The difference of an offer line from the copied item.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Difference {
    /// The offer minus the copied item, never zero.
    pub value: f64,
    /// `None` when the better side is unknown.
    pub better: Option<bool>,
}

/// A line read as its template (`+# to maximum Life`) and its numbers, with their signs.
#[derive(Debug, Clone, PartialEq)]
struct Template {
    text: String,
    numbers: Vec<f64>,
}

impl Template {
    fn of(line: &str) -> Self {
        let mut text = String::with_capacity(line.len());
        let mut numbers = Vec::new();
        let mut chars = line.chars().peekable();
        // A sign right before a number, not inside a word: `-10%`, but not `X-10`.
        let mut sign_allowed = true;
        while let Some(c) = chars.next() {
            let signed = matches!(c, '+' | '-') && sign_allowed;
            let starts_number =
                c.is_ascii_digit() || (signed && chars.peek().is_some_and(char::is_ascii_digit));
            if !starts_number {
                text.push(c);
                sign_allowed = !c.is_alphanumeric();
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
            text.push('#');
            sign_allowed = false;
            if let Ok(value) = number.parse() {
                numbers.push(value);
            }
        }
        Self { text, numbers }
    }

    fn average(&self) -> Option<f64> {
        let count = u32::try_from(self.numbers.len())
            .ok()
            .filter(|&count| count > 0)?;
        Some(self.numbers.iter().sum::<f64>() / f64::from(count))
    }
}

/// Which side of the numbers of `template` is better.
fn direction(template: &str) -> Direction {
    let has_any = |words: &[&str]| words.iter().any(|word| has_word(template, word));
    if has_any(&OTHER_SUBJECTS) {
        return Direction::Unknown;
    }
    let less = has_any(&LESS_WORDS);
    let bad = has_any(&BAD_WORDS);
    if less == bad {
        Direction::Higher
    } else {
        Direction::Lower
    }
}

/// Whether `text` holds `word` as whole words: "less" isn't in "unless".
fn has_word(text: &str, word: &str) -> bool {
    text.match_indices(word).any(|(start, _)| {
        let before = text[..start].chars().next_back();
        let after = text[start + word.len()..].chars().next();
        !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric)
    })
}

/// Whether a modifier of the copied item of `kind` is where the trade site lists the
/// modifiers of `offer`.
fn same_place(offer: OfferKind, kind: ModifierKind) -> bool {
    match offer {
        OfferKind::Enchant => kind == ModifierKind::Enchant,
        OfferKind::Rune => kind == ModifierKind::Rune,
        OfferKind::Implicit => kind == ModifierKind::Implicit,
        OfferKind::Explicit => matches!(kind, ModifierKind::Explicit(_)),
    }
}

/// The difference of the offer `line`, listed as `kind`, from the same line of the copied
/// `modifiers`; `None` when the copied item hasn't it, or has the same numbers.
#[must_use]
pub fn difference(line: &str, kind: OfferKind, modifiers: &[Modifier]) -> Option<Difference> {
    let offer = Template::of(line);
    let offer_average = offer.average()?;
    let copied_lines = |same: bool| {
        modifiers
            .iter()
            .filter(move |modifier| same_place(kind, modifier.kind) == same)
            .flat_map(|modifier| &modifier.lines)
    };
    // The same place first: an implicit and an explicit modifier can read the same.
    let copied = copied_lines(true)
        .chain(copied_lines(false))
        .map(|copied| Template::of(&copied.text))
        .find(|copied| copied.text == offer.text && copied.numbers.len() == offer.numbers.len())?;
    let value = offer_average - copied.average()?;
    // Rounded as the tooltip writes it: a difference that reads 0 isn't one.
    if (value * 100.0).round() == 0.0 {
        return None;
    }
    let better = match direction(&offer.text) {
        Direction::Higher => Some(value > 0.0),
        Direction::Lower => Some(value < 0.0),
        Direction::Unknown => None,
    };
    Some(Difference { value, better })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::StatLine;

    fn modifier(kind: ModifierKind, line: &str) -> Modifier {
        Modifier {
            kind,
            origin: None,
            name: None,
            tier: None,
            tags: Vec::new(),
            lines: vec![StatLine {
                text: line.to_owned(),
                rolls: Vec::new(),
            }],
        }
    }

    fn explicit(line: &str) -> Modifier {
        modifier(ModifierKind::Explicit(None), line)
    }

    #[test]
    fn a_line_is_read_as_its_template_and_its_signed_numbers() {
        assert_eq!(
            Template::of("-10% to Fire Resistance"),
            Template {
                text: "#% to Fire Resistance".to_owned(),
                numbers: vec![-10.0],
            }
        );
        assert_eq!(Template::of("+48 to maximum Life").numbers, [48.0]);
        assert_eq!(
            Template::of("Adds 5 to 82 Lightning Damage").text,
            "Adds # to # Lightning Damage"
        );
        assert_eq!(Template::of("0.5% of Life Regenerated").numbers, [0.5]);
        // A minus sign inside a word isn't a sign.
        assert_eq!(Template::of("Non-1 thing").numbers, [1.0]);
    }

    #[test]
    fn a_higher_roll_is_better_and_a_lower_one_worse() {
        let copied = [explicit("+55 to maximum Life"), explicit("+34 to Strength")];

        assert_eq!(
            difference("+48 to maximum Life", OfferKind::Explicit, &copied),
            Some(Difference {
                value: -7.0,
                better: Some(false),
            })
        );
        assert_eq!(
            difference("+40 to Strength", OfferKind::Explicit, &copied),
            Some(Difference {
                value: 6.0,
                better: Some(true),
            })
        );
    }

    #[test]
    fn a_line_with_several_numbers_compares_their_averages() {
        let copied = [explicit("Adds 3 to 76 Lightning Damage")];

        let difference = difference(
            "Adds 5 to 82 Lightning Damage",
            OfferKind::Explicit,
            &copied,
        );

        assert_eq!(
            difference,
            Some(Difference {
                value: 4.0,
                better: Some(true),
            })
        );
    }

    #[test]
    fn the_same_numbers_or_a_line_the_item_hasnt_make_no_difference() {
        let copied = [
            explicit("Has 2 Charm Slots"),
            explicit("When you kill a Rare monster, you gain its Modifiers for 60 seconds"),
        ];

        assert_eq!(
            difference("Has 2 Charm Slots", OfferKind::Explicit, &copied),
            None
        );
        assert_eq!(
            difference(
                "When you kill a Rare monster, you gain its Modifiers for 60 seconds",
                OfferKind::Explicit,
                &copied
            ),
            None
        );
        assert_eq!(
            difference("+40 to Strength", OfferKind::Explicit, &copied),
            None
        );
    }

    #[test]
    fn a_penalty_or_a_cost_is_better_lower() {
        let copied = [
            explicit("10% reduced Movement Speed"),
            explicit("20% increased Mana Cost of Skills"),
            explicit("15% reduced Mana Cost of Skills"),
            explicit("25% augmentation du Coût en Mana des Compétences"),
        ];
        let better = |line| {
            difference(line, OfferKind::Explicit, &copied).and_then(|difference| difference.better)
        };

        assert_eq!(better("5% reduced Movement Speed"), Some(true));
        assert_eq!(better("30% increased Mana Cost of Skills"), Some(false));
        assert_eq!(better("20% reduced Mana Cost of Skills"), Some(true));
        assert_eq!(
            better("20% augmentation du Coût en Mana des Compétences"),
            Some(true)
        );
    }

    #[test]
    fn a_word_counts_only_whole() {
        assert!(has_word("10% reduced Movement Speed", "reduced"));
        assert!(!has_word("Cannot be Stunned unless 10 Rage", "less"));
        assert_eq!(
            direction("Cannot be Stunned unless # Rage"),
            Direction::Higher
        );
    }

    #[test]
    fn a_line_about_the_enemies_has_no_better_side() {
        let copied = [explicit("Enemies in your Presence have 10% reduced Life")];

        assert_eq!(
            difference(
                "Enemies in your Presence have 15% reduced Life",
                OfferKind::Explicit,
                &copied
            ),
            Some(Difference {
                value: 5.0,
                better: None,
            })
        );
    }

    #[test]
    fn a_line_is_compared_with_the_same_place_first() {
        let copied = [
            explicit("+10 to Strength"),
            modifier(ModifierKind::Implicit, "+20 to Strength"),
        ];

        let difference = difference("+25 to Strength", OfferKind::Implicit, &copied);

        assert_eq!(difference.map(|difference| difference.value), Some(5.0));
    }
}
