//! The difference between a modifier line of an offer and the same line of the copied item,
//! e.g. `+48 to maximum Life` against `+55 to maximum Life`: −7, worse.
//!
//! Both lines are in the language of the item: its offers are searched on the trade site of
//! its language. Two lines are the same when they read the same with `#` in place of each
//! number, at the same place on the item (implicit, explicit…); a line with several numbers
//! (`Adds 5 to 82 Lightning Damage`) compares their averages.

use crate::item::{Modifier, ModifierKind};
use crate::trade::offers::ModifierKind as OfferKind;

/// The words that make a line read the other way, in lowercase: "reduced Movement Speed" is
/// worse when its number is higher, "increased Mana Cost" too, and "reduced Mana Cost" is
/// better.
const LESS_WORDS: &[&str] = &[
    "reduced",
    "less",
    "réduit",
    "réduite",
    "réduits",
    "réduites",
    "en moins",
    "moins de",
];
/// What the player wants lower, in lowercase: costs, charges used, damage taken, life lost,
/// what affects the player ("Duration of Bleeding on You")…
const BAD_WORDS: &[&str] = &[
    "cost",
    "costs",
    "charges used",
    "damage taken",
    "requirements",
    "lose",
    "on you",
    "coût",
    "subis",
    "perd",
    "vous affectant",
];
/// The words that turn what the player wants lower into what they want higher, in lowercase:
/// "Damage taken Recouped as Life", "Mana Cost Efficiency".
const GOOD_TURNS: &[&str] = &["recouped", "efficiency", "récupérés", "efficacité"];
/// What the line acts on when the player isn't its subject, in lowercase: the better side of a
/// number is unknown (more life for the enemies, less damage on them…).
const OTHER_SUBJECTS: &[&str] = &["enemies", "monsters", "ennemis", "monstres"];

/// Which side of a number is better.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Higher,
    Lower,
    Unknown,
}

/// Whether the offer is better than the copied item on a line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Better,
    Worse,
    /// The better side of the line is unknown.
    Unknown,
}

/// The difference of an offer line from the copied item.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Difference {
    /// The offer minus the copied item, never written 0.
    pub value: f64,
    pub verdict: Verdict,
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
}

/// The average of `numbers`; `None` without any.
fn average(numbers: impl ExactSizeIterator<Item = f64>) -> Option<f64> {
    let count = u32::try_from(numbers.len())
        .ok()
        .filter(|&count| count > 0)?;
    Some(numbers.sum::<f64>() / f64::from(count))
}

/// Which side of the numbers of `template` is better.
fn direction(template: &str) -> Direction {
    let template = template.to_lowercase();
    let has_any = |words: &[&str]| words.iter().any(|word| has_word(&template, word));
    if has_any(OTHER_SUBJECTS) {
        return Direction::Unknown;
    }
    let less = has_any(LESS_WORDS);
    let bad = has_any(BAD_WORDS) && !has_any(GOOD_TURNS);
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
/// modifiers of `offer`: the corruption adds enchantments, as the item block shows them.
fn same_place(offer: OfferKind, kind: ModifierKind) -> bool {
    match offer {
        OfferKind::Enchant => matches!(kind, ModifierKind::Enchant | ModifierKind::Corruption),
        OfferKind::Rune => kind == ModifierKind::Rune,
        OfferKind::Implicit => kind == ModifierKind::Implicit,
        OfferKind::Explicit => matches!(kind, ModifierKind::Explicit(_)),
    }
}

/// The difference of the offer `line`, listed as `kind`, from the same line of the copied
/// `modifiers`, at the same place; `None` when the copied item hasn't it there, or has the
/// same numbers.
#[must_use]
pub fn difference(line: &str, kind: OfferKind, modifiers: &[Modifier]) -> Option<Difference> {
    let offer = Template::of(line);
    // The numbers of the copied lines as the item parser read them.
    let copied = modifiers
        .iter()
        .filter(|modifier| same_place(kind, modifier.kind))
        .flat_map(|modifier| &modifier.lines)
        .find(|copied| {
            copied.rolls.len() == offer.numbers.len()
                && Template::of(&copied.text).text == offer.text
        })?;
    let value = average(offer.numbers.iter().copied())?
        - average(copied.rolls.iter().map(|roll| roll.value))?;
    // A difference written 0 isn't one.
    if crate::format::number(value.abs()) == "0" {
        return None;
    }
    let verdict = match (direction(&offer.text), value > 0.0) {
        (Direction::Higher, true) | (Direction::Lower, false) => Verdict::Better,
        (Direction::Higher, false) | (Direction::Lower, true) => Verdict::Worse,
        (Direction::Unknown, _) => Verdict::Unknown,
    };
    Some(Difference { value, verdict })
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
            lines: vec![StatLine::parse(line)],
        }
    }

    fn explicit(line: &str) -> Modifier {
        modifier(ModifierKind::Explicit(None), line)
    }

    fn verdict(line: &str, copied: &[Modifier]) -> Option<Verdict> {
        difference(line, OfferKind::Explicit, copied).map(|difference| difference.verdict)
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
                verdict: Verdict::Worse,
            })
        );
        assert_eq!(
            difference("+40 to Strength", OfferKind::Explicit, &copied),
            Some(Difference {
                value: 6.0,
                verdict: Verdict::Better,
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
                verdict: Verdict::Better,
            })
        );
    }

    #[test]
    fn a_sign_that_changes_is_compared_as_a_number() {
        let copied = [explicit("-10% to Fire Resistance")];

        assert_eq!(
            difference("+5% to Fire Resistance", OfferKind::Explicit, &copied),
            Some(Difference {
                value: 15.0,
                verdict: Verdict::Better,
            })
        );
    }

    #[test]
    fn the_same_numbers_or_a_line_the_item_hasnt_make_no_difference() {
        let copied = [
            explicit("Has 2 Charm Slots"),
            explicit("When you kill a Rare monster, you gain its Modifiers for 60 seconds"),
            explicit("Adds 3 to 76 Lightning Damage"),
        ];

        assert_eq!(verdict("Has 2 Charm Slots", &copied), None);
        assert_eq!(
            verdict(
                "When you kill a Rare monster, you gain its Modifiers for 60 seconds",
                &copied
            ),
            None
        );
        assert_eq!(verdict("+40 to Strength", &copied), None);
        assert_eq!(
            verdict("Adds 76 Lightning Damage", &copied),
            None,
            "another count of numbers"
        );
        assert_eq!(
            verdict("Has 2.001 Charm Slots", &copied),
            None,
            "a difference written 0"
        );
    }

    #[test]
    fn a_penalty_or_a_cost_is_better_lower() {
        let copied = [
            explicit("10% reduced Movement Speed"),
            explicit("20% increased Mana Cost of Skills"),
            explicit("15% reduced Mana Cost of Skills"),
        ];

        assert_eq!(
            verdict("5% reduced Movement Speed", &copied),
            Some(Verdict::Better)
        );
        assert_eq!(
            verdict("30% increased Mana Cost of Skills", &copied),
            Some(Verdict::Worse)
        );
        assert_eq!(
            verdict("20% reduced Mana Cost of Skills", &copied),
            Some(Verdict::Better)
        );
    }

    #[test]
    fn what_reduces_a_penalty_on_the_player_is_better_higher() {
        // The English line of fr-rare-body-armour.txt, and its French line as copied.
        let copied = [
            explicit("41% reduced Duration of Bleeding on You"),
            explicit("Réduit la durée des saignements vous affectant de 41 %."),
            explicit("8% of Damage taken Recouped as Life"),
            explicit("10% increased Mana Cost Efficiency"),
        ];

        assert_eq!(
            verdict("45% reduced Duration of Bleeding on You", &copied),
            Some(Verdict::Better)
        );
        assert_eq!(
            verdict(
                "Réduit la durée des saignements vous affectant de 45 %.",
                &copied
            ),
            Some(Verdict::Better)
        );
        assert_eq!(
            verdict("10% of Damage taken Recouped as Life", &copied),
            Some(Verdict::Better)
        );
        assert_eq!(
            verdict("12% increased Mana Cost Efficiency", &copied),
            Some(Verdict::Better)
        );
    }

    #[test]
    fn the_french_lines_are_read_whatever_their_capitals() {
        // Lines of fr-rare-body-armour.txt.
        let copied = [
            explicit("76% d'Augmentation du Bouclier d'énergie"),
            explicit("Réduit la durée des saignements vous affectant de 41 %."),
        ];

        assert_eq!(
            verdict("80% d'Augmentation du Bouclier d'énergie", &copied),
            Some(Verdict::Better)
        );
        assert_eq!(
            direction("Réduit la durée des saignements de # %."),
            Direction::Lower,
            "Réduit with its capital"
        );
        // "Réduction" also writes a bonus, the damage reduction: no word reads it the other way.
        assert_eq!(
            direction("#% de Réduction supplémentaire des Dégâts physiques"),
            Direction::Higher
        );
    }

    #[test]
    fn a_word_counts_only_whole() {
        assert!(has_word("10% reduced movement speed", "reduced"));
        assert!(!has_word("cannot be stunned unless 10 rage", "less"));
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
                verdict: Verdict::Unknown,
            })
        );
    }

    #[test]
    fn a_line_is_compared_only_at_the_same_place() {
        let copied = [
            explicit("+10 to Strength"),
            modifier(ModifierKind::Implicit, "+7% to Chaos Resistance"),
            modifier(ModifierKind::Corruption, "+2 to Level of all Skills"),
        ];

        let value = |line, kind| difference(line, kind, &copied).map(|difference| difference.value);

        assert_eq!(value("+25 to Strength", OfferKind::Explicit), Some(15.0));
        assert_eq!(value("+25 to Strength", OfferKind::Implicit), None);
        assert_eq!(
            value("+23% to Chaos Resistance", OfferKind::Explicit),
            None,
            "a suffix isn't the implicit of the base"
        );
        assert_eq!(
            value("+3 to Level of all Skills", OfferKind::Enchant),
            Some(1.0),
            "the corruption adds enchantments"
        );
    }
}
