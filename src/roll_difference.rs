//! The difference between a modifier line of an offer and the same line of the copied item,
//! e.g. `+48 to maximum Life` against `+55 to maximum Life`: −7, worse.
//!
//! Both lines are in the language of the item: its offers are searched on the trade site of
//! its language. Two lines are the same when they read the same with `#` in place of each
//! number, at the same place on the item (implicit, explicit…); a line with several numbers
//! (`Adds 5 to 82 Lightning Damage`) compares their averages. The lines of the copied item are
//! read once, when it is copied ([`CopiedLines`]).
//!
//! The better side of a line is read from its words: those of its English string when the
//! line is in another language and its stat has one (`#% de Réduction de l'Armure` is read
//! as `#% reduced Armour`), or else the English template of its stat on the trade site, its
//! own words otherwise. The words were classified with the owner
//! on 2026-10-26, on every string of the stats; the `better` value of Exiled Exchange 2 isn't
//! used, wrong on lines such as `Lose # Life when you use a Skill`.

use crate::filters::StatKey;
use crate::game_data::{self, TradeGroup, TradeStats};
use crate::item::{Modifier, ModifierKind};
use crate::language::Language;
use crate::trade::offers::ModifierKind as OfferKind;

/// The words that make a line read the other way, in lowercase: "reduced Movement Speed" is
/// worse when its number is higher, "increased Mana Cost" too, and "reduced Mana Cost" is
/// better.
const LESS_WORDS: &[&str] = &[
    "reduced",
    "less",
    "slower",
    "shorter",
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
    "charges per use",
    "damage taken",
    "requirements",
    "lose",
    "removes",
    "on you",
    "against you",
    "you take",
    "be inflicted",
    "penalty",
    "seconds to cooldown",
    // "Your Hits treat Fire Resistance as #% higher than actual value".
    "higher than actual value",
    "enemy stun threshold",
    // "Abyss Pits in Map are #% less likely to have no Rewards".
    "to have no",
    // The Breaches and the Delirium Fog of the maps: slower is better.
    "open and close",
    "dissipates",
    "coût",
    "subis",
    "perd",
    "vous affectant",
];
/// The words that turn what the player wants lower into what they want higher, in lowercase:
/// "Damage taken Recouped as Life", "Mana Cost Efficiency", "Gain Maximum Energy Shield equal
/// to #% of total Strength Requirements", the buffs on the player…
const GOOD_TURNS: &[&str] = &[
    "recouped",
    "recoup",
    "efficiency",
    "equal to",
    "cost no",
    "that cost",
    // "Skills gain #% of Damage as Chaos Damage per 3 Life Cost".
    "of damage as",
    "reflected",
    "debuffs on you expire",
    "per curse on you",
    "arcane surge on you",
    "archon buffs on you",
    "récupérés",
    "efficacité",
];
/// What the line acts on when the player isn't its subject, in lowercase: the better side of a
/// number is unknown (more life for the enemies, less damage on them…).
const OTHER_SUBJECTS: &[&str] = &["enemies", "monsters", "ennemis", "monstres"];
/// The conversions, in lowercase: their better side depends on the build.
const BUILD_DEPENDENT: &[&str] = &["taken as", "converted to", "requirements to"];

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
    if has_any(OTHER_SUBJECTS) || has_any(BUILD_DEPENDENT) {
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

/// A modifier line of the copied item, as the offer lines are compared with it.
#[derive(Debug)]
struct CopiedLine {
    /// The index of its modifier among the modifiers of the item, and of the line in it.
    modifier: usize,
    line: usize,
    kind: ModifierKind,
    template: String,
    /// The numbers of the line as the item parser read them.
    rolls: Vec<f64>,
    direction: Direction,
}

/// The modifier lines of the copied item, read once when it is copied: the tooltip of an
/// offer compares its lines with them each time it is drawn.
#[derive(Debug, Default)]
pub struct CopiedLines(Vec<CopiedLine>);

impl CopiedLines {
    /// The lines of `modifiers`, written in `language`; `trade` holds the stats of the trade
    /// site, once it gave them.
    #[must_use]
    pub fn new(language: Language, modifiers: &[Modifier], trade: Option<&TradeStats>) -> Self {
        let lines = modifiers.iter().enumerate().flat_map(|(index, modifier)| {
            let group = TradeGroup::of(modifier);
            modifier
                .lines
                .iter()
                .enumerate()
                .map(move |(line_index, line)| {
                    let template = Template::of(&line.text).text;
                    let words = game_data::english_string(language, &line.text, group, trade);
                    CopiedLine {
                        modifier: index,
                        line: line_index,
                        kind: modifier.kind,
                        direction: direction(words.unwrap_or(&template)),
                        template,
                        rolls: line.rolls.iter().map(|roll| roll.value).collect(),
                    }
                })
        });
        Self(lines.collect())
    }

    /// Whether the line `key` of the copied item is better lower, as "18% reduced Movement
    /// Speed"; not when its better side is unknown.
    #[must_use]
    pub fn better_lower(&self, key: StatKey) -> bool {
        self.0.iter().any(|copied| {
            copied.modifier == key.modifier
                && copied.line == key.line
                && copied.direction == Direction::Lower
        })
    }

    /// The difference of the offer `line`, listed as `kind`, from the same line of the
    /// copied item, at the same place; `None` when the copied item hasn't it there, or has
    /// the same numbers.
    #[must_use]
    pub fn difference(&self, line: &str, kind: OfferKind) -> Option<Difference> {
        let offer = Template::of(line);
        let copied = self.0.iter().find(|copied| {
            same_place(kind, copied.kind)
                && copied.rolls.len() == offer.numbers.len()
                && copied.template == offer.text
        })?;
        let value =
            average(offer.numbers.iter().copied())? - average(copied.rolls.iter().copied())?;
        // A difference written 0 isn't one.
        if crate::format::number(value.abs()) == "0" {
            return None;
        }
        let verdict = match (copied.direction, value > 0.0) {
            (Direction::Higher, true) | (Direction::Lower, false) => Verdict::Better,
            (Direction::Higher, false) | (Direction::Lower, true) => Verdict::Worse,
            (Direction::Unknown, _) => Verdict::Unknown,
        };
        Some(Difference { value, verdict })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::StatLine;

    /// The difference of the offer `line` from the copied `modifiers`, in `language`.
    fn difference(
        line: &str,
        kind: OfferKind,
        language: Language,
        modifiers: &[Modifier],
    ) -> Option<Difference> {
        CopiedLines::new(language, modifiers, None).difference(line, kind)
    }

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
        difference(line, OfferKind::Explicit, Language::English, copied)
            .map(|difference| difference.verdict)
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
            difference(
                "+48 to maximum Life",
                OfferKind::Explicit,
                Language::English,
                &copied
            ),
            Some(Difference {
                value: -7.0,
                verdict: Verdict::Worse,
            })
        );
        assert_eq!(
            difference(
                "+40 to Strength",
                OfferKind::Explicit,
                Language::English,
                &copied
            ),
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
            Language::English,
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
            difference(
                "+5% to Fire Resistance",
                OfferKind::Explicit,
                Language::English,
                &copied
            ),
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
    fn a_line_of_the_copied_item_tells_whether_it_is_better_lower() {
        let mut hybrid = explicit("+20 to Evasion Rating");
        hybrid
            .lines
            .push(StatLine::parse("10% increased Mana Cost of Skills"));
        let copied = [
            explicit("10% reduced Movement Speed"),
            hybrid,
            explicit("15% reduced Life of nearby Enemies"),
        ];
        let lines = CopiedLines::new(Language::English, &copied, None);
        let key = |modifier, line| StatKey { modifier, line };

        assert!(lines.better_lower(key(0, 0)));
        assert!(!lines.better_lower(key(1, 0)));
        assert!(lines.better_lower(key(1, 1)));
        // Its better side unknown: searched with a minimum, as any line.
        assert!(!lines.better_lower(key(2, 0)));
        assert!(!lines.better_lower(key(3, 0)));
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
                Language::English,
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

        let value = |line, kind| {
            difference(line, kind, Language::English, &copied).map(|difference| difference.value)
        };

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

    #[test]
    fn slower_and_shorter_read_like_reduced() {
        for (line, expected) in [
            ("#% slower Curse Activation", Direction::Lower),
            ("Leech Life #% slower", Direction::Lower),
            (
                "Your Heavy Stun buildup empties #% slower",
                Direction::Lower,
            ),
            ("#% faster Curse Activation", Direction::Higher),
            (
                "Skills have a #% shorter Perfect Timing window",
                Direction::Lower,
            ),
        ] {
            assert_eq!(direction(line), expected, "{line}");
        }
    }

    #[test]
    fn what_the_player_wants_lower_is_better_lower() {
        for (line, expected) in [
            ("#% increased Charges per use", Direction::Lower),
            ("#% reduced Charges per use", Direction::Higher),
            (
                "#% increased Movement Speed Penalty from using Skills while moving",
                Direction::Lower,
            ),
            (
                "Hits against you have #% reduced Critical Damage Bonus",
                Direction::Higher,
            ),
            (
                "Hits have #% increased Critical Hit Chance against you",
                Direction::Lower,
            ),
            (
                "#% chance to be inflicted with Bleeding when Hit",
                Direction::Lower,
            ),
            ("Skills have # seconds to Cooldown", Direction::Lower),
            ("You take #% of damage from Blocked Hits", Direction::Lower),
            (
                "Your Hits treat Fire Resistance as #% higher than actual value",
                Direction::Lower,
            ),
            (
                "Abyss Pits in Map are #% less likely to have no Rewards",
                Direction::Higher,
            ),
            ("#% reduced Enemy Stun Threshold", Direction::Higher),
            ("#% increased Enemy Stun Threshold", Direction::Lower),
            ("Removes # of your Life per Enemy Hit", Direction::Lower),
            (
                "Removes #% of Mana Recovered from Life when used",
                Direction::Lower,
            ),
            // The Breaches and the Delirium Fog of the maps last longer when slower.
            ("Breaches open and close #% slower", Direction::Higher),
            ("Breaches in Map open and close #% faster", Direction::Lower),
            ("Delirium Fog dissipates #% slower", Direction::Higher),
            (
                "Delirium Fog in Area dissipates #% faster",
                Direction::Lower,
            ),
        ] {
            assert_eq!(direction(line), expected, "{line}");
        }
    }

    #[test]
    fn some_words_turn_what_the_player_wants_lower_into_better_higher() {
        for (line, expected) in [
            (
                "You Recoup #% of Damage taken by your Offerings as Life",
                Direction::Higher,
            ),
            (
                "Gain Maximum Energy Shield equal to #% of total\nStrength Requirements of Equipped Armour Items",
                Direction::Higher,
            ),
            (
                "Favours Rerolled at Ritual Altars in Map have #% chance to cost no Tribute",
                Direction::Higher,
            ),
            (
                "Skills gain #% of Damage as Chaos Damage per 3 Life Cost",
                Direction::Higher,
            ),
            // Not a gain: the cost is what the player gets.
            (
                "Skills Gain #% of Mana Cost as Extra Life Cost",
                Direction::Lower,
            ),
            ("Debuffs on you expire #% faster", Direction::Higher),
            ("Debuffs on you expire #% slower", Direction::Lower),
            ("Bonded: Buffs on you expire #% faster", Direction::Lower),
            (
                "#% of Melee Physical Damage taken reflected to Attacker",
                Direction::Higher,
            ),
            ("#% increased Damage per Curse on you", Direction::Higher),
            ("#% reduced Totem Damage per Curse on you", Direction::Lower),
            (
                "#% increased effect of Arcane Surge on you",
                Direction::Higher,
            ),
            ("#% reduced effect of Archon Buffs on you", Direction::Lower),
            (
                "#% increased Spell Damage with Spells that cost Life",
                Direction::Higher,
            ),
        ] {
            assert_eq!(direction(line), expected, "{line}");
        }
    }

    #[test]
    fn a_conversion_has_no_better_side() {
        for line in [
            "#% of Physical Damage taken as Fire Damage",
            "#% of Elemental Damage Converted to Chaos Damage",
            "#% of Skill Mana Costs Converted to Life Costs",
            "Convert #% of Requirements to Dexterity",
        ] {
            assert_eq!(direction(line), Direction::Unknown, "{line}");
        }
    }

    #[test]
    fn a_french_line_is_read_with_the_words_of_its_english_string() {
        let copied = [
            explicit("10% de Réduction de l'Armure"),
            explicit("20% d'Augmentation des Charges par utilisation"),
        ];
        let verdict = |line| {
            difference(line, OfferKind::Explicit, Language::French, &copied)
                .map(|difference| difference.verdict)
        };

        // Read as `#% reduced Armour` and `#% increased Charges per use`.
        assert_eq!(
            verdict("15% de Réduction de l'Armure"),
            Some(Verdict::Worse)
        );
        assert_eq!(
            verdict("15% d'Augmentation des Charges par utilisation"),
            Some(Verdict::Better)
        );
        assert_eq!(
            game_data::english_string(
                Language::English,
                "10% reduced Armour",
                TradeGroup::Explicit,
                None
            ),
            None,
            "an English line keeps its own words"
        );
    }

    #[test]
    fn a_french_line_without_english_string_is_read_with_its_own_words() {
        let copied = [explicit("Réduit une chose inconnue de 10 %")];

        assert_eq!(
            difference(
                "Réduit une chose inconnue de 15 %",
                OfferKind::Explicit,
                Language::French,
                &copied
            )
            .map(|difference| difference.verdict),
            Some(Verdict::Worse)
        );
    }

    #[test]
    fn a_french_line_of_the_trade_site_only_is_read_with_its_english_template() {
        let trade = game_data::trade_stats();
        let copied = [explicit("10% de Réduction des Dégâts par patch")];
        let verdict = |trade| {
            CopiedLines::new(Language::French, &copied, trade)
                .difference("15% de Réduction des Dégâts par patch", OfferKind::Explicit)
                .map(|difference| difference.verdict)
        };

        // Read as `#% reduced Damage per Patch`.
        assert_eq!(verdict(Some(&trade)), Some(Verdict::Worse));
        // `Réduction` alone isn't a word that reads the other way.
        assert_eq!(verdict(None), Some(Verdict::Better));
    }

    /// The number of strings of the stats read higher, lower and unknown in `language`.
    fn counts(language: Language) -> (usize, usize, usize) {
        let mut counts = (0, 0, 0);
        for (string, english) in game_data::equipment_strings(language) {
            match direction(english.unwrap_or(string)) {
                Direction::Higher => counts.0 += 1,
                Direction::Lower => counts.1 += 1,
                Direction::Unknown => counts.2 += 1,
            }
        }
        counts
    }

    /// The words were classified with the owner on 2026-10-26, on every string of the stats
    /// that can be on equipment: a change of the words or of the game data changes these
    /// counts, and the lines read differently are to look at before updating them.
    #[test]
    fn every_string_of_the_stats_reads_as_classified() {
        assert_eq!(counts(Language::English), (1983, 599, 300));
        assert_eq!(counts(Language::French), (2008, 589, 304));

        // Two changes that make up for each other would keep the counts: the French strings
        // with an English string are counted too.
        let french = game_data::equipment_strings(Language::French);
        let twins = french
            .iter()
            .filter(|(_, english)| english.is_some())
            .count();
        assert_eq!((twins, french.len()), (2616, 2901));
    }
}
