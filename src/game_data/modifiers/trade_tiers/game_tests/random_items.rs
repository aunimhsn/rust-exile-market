//! The reading of the in-game trade measured on rare items drawn at random from the tables of
//! the game: how many lines get the tier they rolled, another one or none, and how far the
//! search goes against its bounds ([`MAX_READINGS`], `MAX_STEPS`). Too long for every run of
//! the tests, and bound to the patch: to run by hand after the game data are generated again,
//! or after a change of the search, and its numbers recorded again in [`RECORDED`]:
//!
//! ```text
//! cargo test --release random_items -- --ignored --nocapture
//! ```
//!
//! The draw is the same at every run (a xorshift generator from a fixed seed), and the same for
//! the ways that differ only by the order of the lines. An item has from one prefix to the most
//! its base can have (three, two on a jewel) and as many suffixes, of other types, every tier
//! its level allows as likely, every value of a tier as likely. Its lines are written as the
//! in-game trade writes them: one line per stat, without affix, tier nor range, the values of
//! two modifiers with the same stat added up. The modifiers of the Genesis tree, of the
//! essences and of the alloys aren't drawn, nor a modifier one of whose lines no string of the
//! stats writes back.
//!
//! What a real copy does and the draw doesn't: the order of the lines (by stat in the game, in
//! an order the data don't give: the draw tries the order of the modifiers, and a shuffled
//! one), the weights of the modifiers, the items players list (the best tiers far more often),
//! and a line no table explains (copied before a patch, or of a stat the tables don't know):
//! every line of the draw has a reading, so that its counts say nothing of two readings as good
//! that don't read the same lines. No item is sanctified, such an item finding no tier.

use super::*;
use crate::game_data::bases::{self, BaseSpawn, Domain};
use crate::game_data::modifiers::base_tiers::range_values;
use crate::game_data::modifiers::best_reading::MAX_READINGS;
use crate::game_data::modifiers::doubts;
use crate::game_data::modifiers::tables::{TableAffix, TableTier, first_of_its_modifier};
use crate::game_data::stats::Stats;
use crate::item::{Range, StatLine};

/// The bases the items are drawn on, each with the most prefixes, and suffixes, a rare item of
/// it can have: weapons, armours of several attributes and jewellery; then a jewel and a
/// cannon, where many desecrated lines can be an ordinary modifier or a lord's (26 templates
/// on a Diamond). A new base goes last: the draw of the others stays the same.
const BASES: [(&str, u32); 16] = [
    ("Warmonger Bow", 3),
    ("Gemini Crossbow", 3),
    ("Temple Maul", 3),
    ("Acrid Wand", 3),
    ("Voltaic Staff", 3),
    ("Wyrm Quarterstaff", 3),
    ("Divine Crown", 3),
    ("Ancestral Mail", 3),
    ("Soldier Cuirass", 3),
    ("Vaal Wraps", 3),
    ("Wrapped Sandals", 3),
    ("Gold Ring", 3),
    ("Wide Belt", 3),
    ("Veridical Chain", 3),
    ("Diamond", 2),
    ("Trarthan Cannon", 3),
];

/// The items drawn on each base, in each way of [`WAYS`]: 6,400 items a way.
const ITEMS_PER_BASE: usize = 400;

/// A way to draw the items.
#[derive(Debug, Clone, Copy)]
struct Way {
    /// How many of the modifiers the game marks desecrated.
    desecrated: Desecrated,
    /// Whether a modifier marked desecrated is, one time in two, an ordinary modifier the
    /// desecration revealed, which the game marks as well; a lord's otherwise.
    revealed: bool,
    /// Whether the lines are shuffled, rather than in the order of their modifiers.
    shuffled: bool,
    /// The level of the items: at 82, every tier of the tables rolls.
    item_level: u32,
}

const WAYS: [Way; 6] = [
    Way::PLAIN,
    Way {
        shuffled: true,
        ..Way::PLAIN
    },
    Way {
        desecrated: Desecrated::Third,
        ..Way::PLAIN
    },
    Way {
        desecrated: Desecrated::All,
        ..Way::PLAIN
    },
    // A line marked desecrated that an ordinary modifier and a lord's both hold.
    Way {
        desecrated: Desecrated::All,
        revealed: true,
        ..Way::PLAIN
    },
    // The best tiers above the level of the item.
    Way {
        item_level: 45,
        ..Way::PLAIN
    },
];

impl Way {
    /// No modifier desecrated, the lines in the order of their modifiers, every tier rolling.
    const PLAIN: Self = Self {
        desecrated: Desecrated::None,
        revealed: false,
        shuffled: false,
        item_level: 82,
    };
}

/// The seed of the draw of the items, and of the draw that shuffles their lines.
const SEED: u64 = 0x9E37_79B9_7F4A_7C15;
const SHUFFLE_SEED: u64 = 0xD1B5_4A32_D192_ED03;

/// What the draw gave for each of the [`WAYS`], with the game data of RePoE `75885a0`
/// (2026-11-11): to record again when the data or the search change, after a look at what
/// moved. A wrong tier is shown to the player as sure: `other_modifier` is the count to keep
/// low, and `wrong_items` the items it shows on. Before step 17e3e, which leaves without tier
/// the lines pure modifiers and their hybrid both hold, the first way read 858 lines as
/// another modifier, 656 of them lines of a hybrid read alone, on 559 items.
const RECORDED: [Counts; 6] = [
    Counts {
        items: 6400,
        wrong_items: 109,
        discarded: 0,
        lines: 26789,
        summed_lines: 871,
        right: 24137,
        either_affix: 198,
        without_tier: 2520,
        other_tier: 85,
        other_modifier: 47,
        summed_other_modifier: 47,
        hybrid_other_modifier: 0,
        most_readings: 24,
        most_steps: 995,
    },
    Counts {
        items: 6400,
        wrong_items: 105,
        discarded: 0,
        lines: 26789,
        summed_lines: 871,
        right: 22669,
        either_affix: 216,
        without_tier: 3994,
        other_tier: 82,
        other_modifier: 44,
        summed_other_modifier: 44,
        hybrid_other_modifier: 0,
        most_readings: 12,
        most_steps: 1810,
    },
    Counts {
        items: 6400,
        wrong_items: 60,
        discarded: 0,
        lines: 25638,
        summed_lines: 381,
        right: 23925,
        either_affix: 172,
        without_tier: 1648,
        other_tier: 43,
        other_modifier: 22,
        summed_other_modifier: 11,
        hybrid_other_modifier: 0,
        most_readings: 16,
        most_steps: 456,
    },
    Counts {
        items: 6400,
        wrong_items: 54,
        discarded: 0,
        lines: 23059,
        summed_lines: 42,
        right: 22705,
        either_affix: 75,
        without_tier: 300,
        other_tier: 0,
        other_modifier: 54,
        summed_other_modifier: 0,
        hybrid_other_modifier: 0,
        most_readings: 8,
        most_steps: 58,
    },
    Counts {
        items: 6400,
        wrong_items: 68,
        discarded: 0,
        lines: 25022,
        summed_lines: 258,
        right: 23743,
        either_affix: 153,
        without_tier: 1207,
        other_tier: 42,
        other_modifier: 30,
        summed_other_modifier: 9,
        hybrid_other_modifier: 0,
        most_readings: 16,
        most_steps: 536,
    },
    Counts {
        items: 6400,
        wrong_items: 42,
        discarded: 0,
        lines: 26716,
        summed_lines: 904,
        right: 24590,
        either_affix: 175,
        without_tier: 2068,
        other_tier: 24,
        other_modifier: 34,
        summed_other_modifier: 34,
        hybrid_other_modifier: 0,
        most_readings: 16,
        most_steps: 580,
    },
];

/// How many of the modifiers of an item are desecrated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Desecrated {
    None,
    /// One in three.
    Third,
    All,
}

/// A xorshift generator: the same draw at every run, without a dependency.
struct Draw(u64);

impl Draw {
    fn next(&mut self) -> u64 {
        let mut state = self.0;
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        self.0 = state;
        state
    }

    /// A number below `count`, which isn't zero.
    fn below(&mut self, count: usize) -> usize {
        let count = u64::try_from(count).expect("a count");
        usize::try_from(self.next() % count).expect("an index")
    }

    /// A value of the range `[min, max]` of a table, as the game shows it: a whole number, or
    /// one with a decimal when an end of the range has one.
    fn value(&mut self, [min, max]: [f64; 2]) -> f64 {
        let (low, high) = (min.min(max), min.max(max));
        let whole = |value: f64| value.fract().abs() < f64::EPSILON;
        let scale = if whole(low) && whole(high) { 1.0 } else { 10.0 };
        let mut values = Vec::new();
        let mut step = (low * scale).ceil();
        while step <= (high * scale).floor() {
            values.push(step / scale);
            step += 1.0;
        }
        if values.is_empty() {
            return (low * scale).round() / scale;
        }
        values[self.below(values.len())]
    }
}

/// A modifier of the tables on a base: its first table, and its tiers on the base, the best
/// first, numbered as `tiers_on_base` numbers them.
struct Rolled<'a> {
    table: &'a Table,
    tiers: Vec<&'a TableTier>,
}

/// The modifiers of `tables` an item of `item_level` can roll on `base`, desecrated or not,
/// whose lines the stats write back; and the types of those left out for that.
fn rolled<'a>(
    tables: &'a [Table],
    base: &BaseSpawn,
    desecrated: bool,
    item_level: u32,
) -> (Vec<Rolled<'a>>, Vec<&'a str>) {
    let mut rolled = Vec::new();
    let mut unwritten = Vec::new();
    for (index, table) in tables.iter().enumerate() {
        if table.source.is_some()
            || (table.domain == Domain::Desecrated) != desecrated
            || !first_of_its_modifier(tables, index)
        {
            continue;
        }
        let mut tiers: Vec<&TableTier> = tables
            .iter()
            .filter(|other| other.same_modifier(table) && other.rolls_on(base))
            .flat_map(|other| &other.tiers)
            .collect();
        tiers.sort_by(|tier, other| {
            tier.level.cmp(&other.level).then_with(|| {
                range_values(tier)
                    .partial_cmp(range_values(other))
                    .expect("no NaN")
            })
        });
        tiers.reverse();
        let Some(worst) = tiers.last().filter(|tier| tier.level <= item_level) else {
            continue;
        };
        let origin = desecrated.then_some(ModifierOrigin::Desecrated);
        let writes = table.lines.iter().zip(&worst.ranges).all(|(line, ranges)| {
            let values: Vec<f64> = ranges.iter().map(|&[min, _]| min).collect();
            written(&line.template, &values, origin).is_some()
        });
        if writes {
            rolled.push(Rolled { table, tiers });
        } else {
            unwritten.push(table.kind.as_str());
        }
    }
    (rolled, unwritten)
}

/// A number as the game writes it: `25`, `2.5`.
fn number(value: f64) -> String {
    format!("{}", value.abs())
}

/// The line of the English `template` with `values` (those of its stat, with its sign), as a
/// modifier of `origin` copied from the in-game trade: the first string of a stat of that
/// template which the application reads back as that template with these values, one that
/// writes them without a minus sign first (`20% reduced`, not `-20% increased`). None when no
/// string does.
fn written(template: &str, values: &[f64], origin: Option<ModifierOrigin>) -> Option<Modifier> {
    let strings = || {
        Stats::of(Language::English)
            .all()
            .filter(|stat| stat.reference == template)
            .flat_map(|stat| &stat.matchers)
    };
    let write = |matcher: &crate::game_data::stats::Matcher, signed: bool| {
        let sign = if matcher.negate { -1.0 } else { 1.0 };
        let hashes = matcher.string.matches('#').count();
        let text = if hashes == values.len() {
            let mut text = String::new();
            let mut parts = matcher.string.split('#');
            text.push_str(parts.next()?);
            for (part, value) in parts.zip(values) {
                let value = value * sign;
                if value < 0.0 {
                    if !signed {
                        return None;
                    }
                    text.push('-');
                }
                text.push_str(&number(value));
                text.push_str(part);
            }
            text
        } else if hashes == 0 && values.len() == 1 {
            // A string that gives its value: `Loads an additional bolt`.
            matcher.string.clone()
        } else {
            return None;
        };
        let modifier = Modifier {
            kind: ModifierKind::Explicit(None),
            origin,
            name: None,
            tier: None,
            tags: Vec::new(),
            is_unrevealed: false,
            lines: text.lines().map(StatLine::parse).collect(),
            doubt: None,
        };
        reads_back(&modifier, template, values).then_some(modifier)
    };
    [false, true]
        .into_iter()
        .find_map(|signed| strings().find_map(|matcher| write(matcher, signed)))
}

/// Whether the search reads `modifier` as one line of `template` with `values`.
fn reads_back(modifier: &Modifier, template: &str, values: &[f64]) -> bool {
    let matches = modifier_stats(Language::English, modifier, None);
    let Some(copied) = Copied::of(modifier, &matches) else {
        return false;
    };
    let [line] = copied.lines.as_slice() else {
        return false;
    };
    let read: Vec<f64> = if line.values.is_empty() {
        line.written.into_iter().collect()
    } else {
        line.values.clone()
    };
    line.is(template)
        && read.len() == values.len()
        && read
            .iter()
            .zip(values)
            .all(|(read, value)| (read - value).abs() < 1e-9)
}

/// A modifier drawn on an item.
struct TrueModifier<'a> {
    rolled: &'a Rolled<'a>,
    /// The index of its tier among those of `rolled`.
    tier: usize,
    /// The indexes among the lines of the item of its lines.
    lines: Vec<usize>,
}

/// A line of an item, as the in-game trade writes it.
struct ItemLine<'a> {
    template: &'a str,
    origin: Option<ModifierOrigin>,
    /// The sum of the values of its modifiers.
    values: Vec<f64>,
    /// The indexes among the modifiers of the item of those it adds up.
    modifiers: Vec<usize>,
}

/// An item drawn.
struct DrawnItem<'a> {
    modifiers: Vec<TrueModifier<'a>>,
    lines: Vec<ItemLine<'a>>,
}

impl<'a> DrawnItem<'a> {
    /// Whether `rolled` can be added: no modifier of its type yet, and none of its lines a
    /// line of the item it can't add up with (another origin, or no value to add).
    fn takes(&self, rolled: &Rolled<'_>, origin: Option<ModifierOrigin>) -> bool {
        let kind = self
            .modifiers
            .iter()
            .any(|modifier| modifier.rolled.table.kind == rolled.table.kind);
        let clash = rolled.table.lines.iter().any(|line| {
            self.lines.iter().any(|other| {
                other.template == line.template
                    && (other.origin != origin || other.values.is_empty())
            })
        });
        !kind && !clash
    }

    /// Adds `rolled` in its tier `tier`, marked `origin`, its values drawn.
    fn add(
        &mut self,
        draw: &mut Draw,
        rolled: &'a Rolled<'a>,
        tier: usize,
        origin: Option<ModifierOrigin>,
    ) {
        let index = self.modifiers.len();
        let mut lines = Vec::new();
        for (line, ranges) in rolled.table.lines.iter().zip(&rolled.tiers[tier].ranges) {
            let values: Vec<f64> = ranges.iter().map(|&range| draw.value(range)).collect();
            let same = self.lines.iter().position(|other| {
                other.template == line.template && other.values.len() == values.len()
            });
            if let Some(same) = same {
                let other = &mut self.lines[same];
                for (sum, value) in other.values.iter_mut().zip(&values) {
                    *sum += value;
                }
                other.modifiers.push(index);
                lines.push(same);
            } else {
                lines.push(self.lines.len());
                self.lines.push(ItemLine {
                    template: &line.template,
                    origin,
                    values,
                    modifiers: vec![index],
                });
            }
        }
        self.modifiers.push(TrueModifier {
            rolled,
            tier,
            lines,
        });
    }

    /// The lines in another order, drawn.
    fn shuffle(&mut self, draw: &mut Draw) {
        let mut order: Vec<usize> = (0..self.lines.len()).collect();
        for last in (1..order.len()).rev() {
            order.swap(last, draw.below(last + 1));
        }
        // `order[new]` is the old place of the line now at `new`.
        let mut lines: Vec<Option<ItemLine<'a>>> = self.lines.drain(..).map(Some).collect();
        self.lines = order
            .iter()
            .map(|&old| lines[old].take().expect("each line once"))
            .collect();
        for modifier in &mut self.modifiers {
            for line in &mut modifier.lines {
                *line = order
                    .iter()
                    .position(|old| old == line)
                    .expect("its new place");
            }
        }
    }

    /// The lines as the parser reads them from a copy of the in-game trade; none when a sum
    /// can't be written.
    fn copied(&self) -> Option<Vec<Modifier>> {
        self.lines
            .iter()
            .map(|line| written(line.template, &line.values, line.origin))
            .collect()
    }
}

/// An item with one to `most` prefixes and as many suffixes drawn among `ordinary` and
/// `desecrated` as `way` says.
fn drawn<'a>(
    draw: &mut Draw,
    ordinary: &'a [Rolled<'a>],
    desecrated: &'a [Rolled<'a>],
    way: Way,
    most: u32,
) -> DrawnItem<'a> {
    let most = usize::try_from(most).expect("a count");
    let mut item = DrawnItem {
        modifiers: Vec::new(),
        lines: Vec::new(),
    };
    for affix in [TableAffix::Prefix, TableAffix::Suffix] {
        for _ in 0..=draw.below(most) {
            let marked = match way.desecrated {
                Desecrated::None => false,
                Desecrated::Third => draw.below(3) == 0,
                Desecrated::All => true,
            };
            let of_a_lord = marked && !(way.revealed && draw.below(2) == 0);
            let pool: Vec<&Rolled<'_>> = if of_a_lord { desecrated } else { ordinary }
                .iter()
                .filter(|rolled| rolled.table.affix == affix)
                .collect();
            if pool.is_empty() {
                continue;
            }
            let origin = marked.then_some(ModifierOrigin::Desecrated);
            // A few tries: a type already on the item is drawn again.
            for _ in 0..20 {
                let rolled = pool[draw.below(pool.len())];
                if !item.takes(rolled, origin) {
                    continue;
                }
                let allowed = rolled
                    .tiers
                    .iter()
                    .filter(|tier| tier.level <= way.item_level)
                    .count();
                let first = rolled.tiers.len() - allowed;
                let tier = first + draw.below(allowed);
                item.add(draw, rolled, tier, origin);
                break;
            }
        }
    }
    item
}

/// What the draw counts, over the lines of the items.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct Counts {
    items: usize,
    /// The items with a line read in another tier or as another modifier, among `items`.
    wrong_items: usize,
    /// The items drawn again, a sum of theirs written by no string (a line of two signs).
    discarded: usize,
    lines: usize,
    /// The lines that add up two modifiers or more, among `lines`.
    summed_lines: usize,
    /// The lines read as their modifiers, each in its tier (one of two neighbouring tiers for
    /// a line read so), a prefix or suffix in doubt counting when the true one is among them.
    right: usize,
    /// Those of `right` whose affix stays in doubt, a prefix or a suffix, the room of the
    /// item leaving both.
    either_affix: usize,
    /// The lines left without tier.
    without_tier: usize,
    /// The lines read as their modifiers, one in another tier: a value in two tiers takes the
    /// worst.
    other_tier: usize,
    /// The lines read as other modifiers: a hybrid as pure modifiers, a sum as one modifier…
    other_modifier: usize,
    /// Those of `other_modifier` that add up two modifiers or more.
    summed_other_modifier: usize,
    /// Those of `other_modifier` that are a line of one hybrid read alone, as a modifier of
    /// that line only: none, the other readings leaving such a line without tier.
    hybrid_other_modifier: usize,
    /// The most readings looked at for an item, of [`MAX_READINGS`].
    most_readings: usize,
    /// The most steps of the search of the other readings for an item, of `MAX_STEPS`.
    most_steps: usize,
}

/// How a line was read against the modifiers it rolled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Read {
    Right,
    WithoutTier,
    OtherTier,
    OtherModifier,
}

/// Whether a modifier read is the one that rolled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Same {
    /// Another modifier: another affix, other lines or other tiers on the base.
    No,
    /// The modifier, in another tier than those read.
    OtherTier,
    /// The modifier, its tier among those read.
    Yes,
}

/// The level and the ranges of each tier, the ranges sorted: the same for the same modifier
/// whatever the order of its lines.
fn shape<'t>(tiers: impl Iterator<Item = (u32, Vec<Range>)> + 't) -> Vec<(u32, Vec<[f64; 2]>)> {
    tiers
        .map(|(level, ranges)| {
            let mut ranges: Vec<[f64; 2]> =
                ranges.iter().map(|range| [range.min, range.max]).collect();
            ranges.sort_by(|range, other| range.partial_cmp(other).expect("no NaN"));
            (level, ranges)
        })
        .collect()
}

impl TrueModifier<'_> {
    /// Whether `read` on the lines `lines` of the item is this modifier: its affix, its lines
    /// and its tiers on the base, then its tier among those `read` may be.
    fn is(&self, affix: Affix, read: &ModifierTiers, lines: &[usize]) -> Same {
        let mut own = self.lines.clone();
        own.sort_unstable();
        let tiers = shape(self.rolled.tiers.iter().map(|tier| {
            let ranges = tier.ranges.iter().flatten();
            let ranges = ranges.map(|&[min, max]| Range {
                min: min.min(max),
                max: min.max(max),
            });
            (tier.level, ranges.collect())
        }));
        let read_tiers = shape(
            read.tiers
                .iter()
                .map(|tier| (tier.level, tier.ranges.clone())),
        );
        let number = u32::try_from(self.tier + 1).expect("a tier number");
        if Affix::from(self.rolled.table.affix) != affix || own != lines || tiers != read_tiers {
            Same::No
        } else if read.possible().any(|tier| tier.number == number) {
            Same::Yes
        } else {
            Same::OtherTier
        }
    }
}

/// How the line `line` of `item` was read in `found`.
fn read_as(item: &DrawnItem<'_>, line: usize, found: &[FoundTiers]) -> Read {
    let Some(found) = found.iter().find(|found| found.modifiers.contains(&line)) else {
        return Read::WithoutTier;
    };
    let whole: Vec<usize> = found.modifiers.clone().collect();
    let added = found.added.as_ref().filter(|added| added.line == line);
    let truth: Vec<&TrueModifier<'_>> = item.lines[line]
        .modifiers
        .iter()
        .map(|&index| &item.modifiers[index])
        .collect();
    let mut best = Read::OtherModifier;
    // Each way to read the line: the modifier of its lines, of either affix in doubt, and
    // the one added up on it. A line of three modifiers or more is never read as them.
    for (affix, tiers) in &found.affixes {
        let read = match (truth.as_slice(), added) {
            ([one], None) => vec![one.is(*affix, tiers, &whole)],
            ([one, other], Some(added)) => {
                let alone = [line];
                let pair = |whole_one: &TrueModifier<'_>, added_one: &TrueModifier<'_>| {
                    vec![
                        whole_one.is(*affix, tiers, &whole),
                        added_one.is(added.affix, &added.tiers, &alone),
                    ]
                };
                let first = pair(one, other);
                if first.contains(&Same::No) {
                    pair(other, one)
                } else {
                    first
                }
            }
            _ => continue,
        };
        if read.iter().all(|&same| same == Same::Yes) {
            return Read::Right;
        }
        if !read.contains(&Same::No) {
            best = Read::OtherTier;
        }
    }
    best
}

/// Whether `found` reads the line `line` as a modifier that can be a prefix or a suffix.
fn either_affix(line: usize, found: &[FoundTiers]) -> bool {
    found
        .iter()
        .any(|found| found.modifiers.contains(&line) && found.affixes.len() > 1)
}

/// Whether the line `line` of `item` is a line of one hybrid that `found` reads alone, as a
/// modifier of that line only.
fn hybrid_read_alone(item: &DrawnItem<'_>, line: usize, found: &[FoundTiers]) -> bool {
    let [modifier] = item.lines[line].modifiers.as_slice() else {
        return false;
    };
    item.modifiers[*modifier].lines.len() > 1
        && found
            .iter()
            .any(|found| found.modifiers == (line..line + 1) && found.added.is_none())
}

/// The items of one way of the draw, read and counted; with the types left out of the draw.
fn measure(way: Way) -> (Counts, Vec<&'static str>) {
    // The lines shuffled by a draw of their own: the same items whatever their order.
    let mut draw = Draw(SEED);
    let mut shuffle = Draw(SHUFFLE_SEED);
    let mut counts = Counts::default();
    let mut left_out = Vec::new();
    for (base_type, most) in BASES {
        let base = bases::spawn(Language::English, base_type).expect(base_type);
        let (ordinary, unwritten) = rolled(&TABLES, base, false, way.item_level);
        left_out.extend(unwritten);
        let (desecrated, unwritten) = rolled(&TABLES, base, true, way.item_level);
        left_out.extend(unwritten);
        let trade_item = TradeItem {
            language: Language::English,
            base,
            item_level: Some(way.item_level),
        };
        let mut read = 0;
        while read < ITEMS_PER_BASE {
            let mut item = drawn(&mut draw, &ordinary, &desecrated, way, most);
            if way.shuffled {
                item.shuffle(&mut shuffle);
            }
            let Some(modifiers) = item.copied() else {
                counts.discarded += 1;
                // Over the bases of the way: a draw that keeps failing would never end.
                assert!(counts.discarded < ITEMS_PER_BASE, "{way:?}: {counts:?}");
                continue;
            };
            read += 1;
            let limits = AffixLimits::new(most, most);
            let (found, effort) = trade_tiers_in(&TABLES, &trade_item, &modifiers, limits);
            counts.items += 1;
            counts.most_readings = counts.most_readings.max(effort.readings);
            counts.most_steps = counts.most_steps.max(effort.steps);
            let mut wrong = false;
            for (index, line) in item.lines.iter().enumerate() {
                let summed = line.modifiers.len() > 1;
                counts.lines += 1;
                counts.summed_lines += usize::from(summed);
                match read_as(&item, index, &found) {
                    Read::Right => {
                        counts.right += 1;
                        counts.either_affix += usize::from(either_affix(index, &found));
                    }
                    Read::WithoutTier => counts.without_tier += 1,
                    Read::OtherTier => {
                        counts.other_tier += 1;
                        wrong = true;
                    }
                    Read::OtherModifier => {
                        counts.other_modifier += 1;
                        counts.summed_other_modifier += usize::from(summed);
                        counts.hybrid_other_modifier +=
                            usize::from(hybrid_read_alone(&item, index, &found));
                        wrong = true;
                    }
                }
            }
            counts.wrong_items += usize::from(wrong);
        }
    }
    left_out.sort_unstable();
    left_out.dedup();
    (counts, left_out)
}

#[test]
#[ignore = "long, run by hand after the game data are generated again: see the module"]
fn random_items_are_read_within_the_bounds_of_the_search() {
    let mut measured = Vec::new();
    for way in WAYS {
        let (counts, left_out) = measure(way);
        println!("{way:?}: {counts:#?}");
        println!("Not drawn, a line no string writes back: {left_out:?}");
        // Far from the bounds, past which every line of the item is left without tier.
        assert!(counts.most_readings < MAX_READINGS / 4, "{counts:?}");
        assert!(counts.most_steps < doubts::MAX_STEPS / 4, "{counts:?}");
        measured.push(counts);
    }

    assert_eq!(measured, RECORDED);
}
