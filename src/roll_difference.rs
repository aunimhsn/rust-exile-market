//! The difference between a modifier line of an offer and the same line of the copied item,
//! e.g. `+48 to maximum Life` against `+55 to maximum Life`: −7, worse.
//!
//! Both lines are in the language of the item: its offers are searched on the trade site of
//! its language. Two lines are the same when they read the same with `#` in place of each
//! number, at the same place on the item (implicit, explicit…); a line with several numbers
//! (`Adds 5 to 82 Lightning Damage`) compares their averages, of the numbers its stat counts
//! only (not the 3 of `20% chance to gain Onslaught for 3 seconds`). The lines of the copied
//! item are read once, when it is copied ([`CopiedLines`]).
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
use crate::trade::stat_filters::LineStats;

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
/// number is unknown (more life for the enemies, less damage on them…); the singular too, as
/// the waystones write it (`#% more Monster Life`, `Monster have #% increased …`).
const OTHER_SUBJECTS: &[&str] = &[
    "enemies", "monsters", "monster", "ennemis", "monstres", "monstre",
];
/// The monsters the player hits, in lowercase: they aren't the subject of the line (`Critical
/// Hits ignore Enemy Monster Elemental Resistances`).
const PLAYER_TARGETS: &[&str] = &["enemy monster", "enemy monsters"];
/// The rewards of the monsters of a map, in lowercase: more is better, though the line is
/// about monsters (`#% increased number of Monster Packs`, `Map has #% increased Monster
/// Rarity` of the tablets; owner's choice on 2026-10-31: more monsters make a map harder at
/// first, but they are a bonus in the end).
const REWARDS: &[&str] = &[
    "monster packs",
    "pack of monsters",
    "packs of monsters",
    "additional packs",
    "additional waves",
    "pack size",
    "monster rarity",
    "monster density",
    "monster markers",
    "increased magic monsters",
    "reduced magic monsters",
    "number of rare monsters",
    "increased rare monsters",
    "reduced rare monsters",
    "additional rare monsters",
    "rare expedition monsters",
    "will be rare monsters",
    "slaying rare monsters",
    "duplicate runic monsters",
    "unique monster to the map",
    "additional monsters",
    "increased monsters",
    "reduced monsters",
    "at least magic",
    "to be magic",
    "to be rare",
    "additional modifier",
    "additional modifiers",
    "rare modifier",
    "rare modifiers",
    "abyssal modifier",
    "abyssal modifiers",
    "dropped by",
    "drop",
    "experience",
    "tribute",
    "reward progress",
    // The French lines without English twin (their strings don't line up).
    "cédés par",
    "au moins magiques",
];
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
    // A French reward without English twin written with "de Réduction", a word that also
    // writes a bonus, keeps an unknown side: no word reads it lower.
    let reward = has_any(REWARDS) && !has_word(&template, "de réduction");
    let other_subject = has_any(OTHER_SUBJECTS) && !has_any(PLAYER_TARGETS) && !reward;
    if other_subject || has_any(BUILD_DEPENDENT) {
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
    /// The indexes of the numbers its stat counts, when known; the others are written by the
    /// string of the stat, the same on every item.
    counted: Option<Vec<usize>>,
    direction: Direction,
}

/// The modifier lines of the copied item, read once when it is copied: the tooltip of an
/// offer compares its lines with them each time it is drawn.
#[derive(Debug, Default)]
pub struct CopiedLines(Vec<CopiedLine>);

impl CopiedLines {
    /// The lines of `modifiers`, written in `language`, whose `stats` tell the numbers each
    /// stat counts; `trade` holds the stats of the trade site, once it gave them.
    #[must_use]
    pub fn new(
        language: Language,
        modifiers: &[Modifier],
        stats: &LineStats,
        trade: Option<&TradeStats>,
    ) -> Self {
        let lines = modifiers.iter().enumerate().flat_map(|(index, modifier)| {
            let group = TradeGroup::of(modifier);
            modifier
                .lines
                .iter()
                .enumerate()
                .map(move |(line_index, line)| {
                    let key = StatKey {
                        modifier: index,
                        line: line_index,
                    };
                    let template = Template::of(&line.text).text;
                    let words = game_data::english_string(language, &line.text, group, trade);
                    CopiedLine {
                        modifier: index,
                        line: line_index,
                        kind: modifier.kind,
                        direction: direction(words.unwrap_or(&template)),
                        template,
                        rolls: line.rolls.iter().map(|roll| roll.value).collect(),
                        counted: stats.numbers(key).map(<[usize]>::to_vec),
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
        let counted = |numbers: &[f64]| -> Vec<f64> {
            match &copied.counted {
                Some(counted) => counted
                    .iter()
                    .filter_map(|&number| numbers.get(number).copied())
                    .collect(),
                None => numbers.to_vec(),
            }
        };
        let value = average(counted(&offer.numbers).into_iter())?
            - average(counted(&copied.rolls).into_iter())?;
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
mod tests;
