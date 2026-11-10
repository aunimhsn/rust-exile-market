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
//! The draw is the same at every run (a xorshift generator from a fixed seed). An item has one
//! to three prefixes and as many suffixes, of other types, every tier its level allows as
//! likely, every value of a tier as likely. Its lines are written as the in-game trade writes
//! them: one line per stat, without affix, tier nor range, the values of two modifiers with the
//! same stat added up. The modifiers of the Genesis tree, of the essences and of the alloys
//! aren't drawn, nor a modifier one of whose lines no string of the stats writes back.
//!
//! What a real copy does and the draw doesn't: the order of the lines (by stat in the game, in
//! an order the data don't give: the draw tries the order of the modifiers, and a shuffled
//! one), the weights of the modifiers, and the items players list (the best tiers far more
//! often).

use super::*;

/// The bases the items are drawn on: weapons, armours of several attributes and jewellery.
const BASES: [&str; 14] = [
    "Warmonger Bow",
    "Gemini Crossbow",
    "Temple Maul",
    "Acrid Wand",
    "Voltaic Staff",
    "Wyrm Quarterstaff",
    "Divine Crown",
    "Ancestral Mail",
    "Soldier Cuirass",
    "Vaal Wraps",
    "Wrapped Sandals",
    "Gold Ring",
    "Wide Belt",
    "Veridical Chain",
];

/// The items drawn on each base, in each way of [`WAYS`].
const ITEMS_PER_BASE: usize = 400;

/// The level of the items: every tier of the tables rolls.
const ITEM_LEVEL: u32 = 82;

/// The ways the items are drawn: how many of their modifiers are desecrated, and whether their
/// lines are shuffled.
const WAYS: [(Desecrated, bool); 4] = [
    (Desecrated::None, false),
    (Desecrated::None, true),
    (Desecrated::Third, false),
    (Desecrated::All, false),
];

/// What the draw gave for each of the [`WAYS`], with the game data of RePoE `75885a0`
/// (2026-11-10): to record again when the data or the search change, after a look at what
/// moved. A wrong tier is shown to the player as sure: `other_modifier` is the count to keep
/// low.
const RECORDED: [Counts; 4] = [
    Counts {
        items: 5600,
        lines: 23920,
        summed_lines: 825,
        right: 21431,
        without_tier: 1581,
        other_tier: 73,
        other_modifier: 835,
        summed_other_modifier: 153,
        most_readings: 24,
        most_steps: 477,
    },
    Counts {
        items: 5600,
        lines: 23816,
        summed_lines: 844,
        right: 19987,
        without_tier: 2444,
        other_tier: 65,
        other_modifier: 1320,
        summed_other_modifier: 178,
        most_readings: 16,
        most_steps: 1025,
    },
    Counts {
        items: 5600,
        lines: 22656,
        summed_lines: 368,
        right: 21054,
        without_tier: 847,
        other_tier: 60,
        other_modifier: 695,
        summed_other_modifier: 51,
        most_readings: 12,
        most_steps: 415,
    },
    Counts {
        items: 5600,
        lines: 19506,
        summed_lines: 10,
        right: 19426,
        without_tier: 16,
        other_tier: 0,
        other_modifier: 64,
        summed_other_modifier: 7,
        most_readings: 8,
        most_steps: 52,
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

/// The modifiers of `tables` an item of [`ITEM_LEVEL`] can roll on `base`, desecrated or not,
/// whose lines the stats write back; and the types of those left out for that.
fn rolled<'a>(
    tables: &'a [Table],
    base: &BaseSpawn,
    desecrated: bool,
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
        let Some(worst) = tiers.last().filter(|tier| tier.level <= ITEM_LEVEL) else {
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
            unrevealed: false,
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

    /// Adds `rolled` in its tier `tier`, its values drawn.
    fn add(&mut self, draw: &mut Draw, rolled: &'a Rolled<'a>, tier: usize) {
        let origin =
            (rolled.table.domain == Domain::Desecrated).then_some(ModifierOrigin::Desecrated);
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

/// An item with one to three prefixes and as many suffixes drawn among `ordinary` and
/// `desecrated`, the share of the second as `share` says.
fn drawn<'a>(
    draw: &mut Draw,
    ordinary: &'a [Rolled<'a>],
    desecrated: &'a [Rolled<'a>],
    share: Desecrated,
) -> DrawnItem<'a> {
    let mut item = DrawnItem {
        modifiers: Vec::new(),
        lines: Vec::new(),
    };
    for affix in [TableAffix::Prefix, TableAffix::Suffix] {
        for _ in 0..=draw.below(3) {
            let from_desecrated = match share {
                Desecrated::None => false,
                Desecrated::Third => draw.below(3) == 0,
                Desecrated::All => true,
            };
            let pool: Vec<&Rolled<'_>> = if from_desecrated {
                desecrated
            } else {
                ordinary
            }
            .iter()
            .filter(|rolled| rolled.table.affix == affix)
            .collect();
            if pool.is_empty() {
                continue;
            }
            // A few tries: a type already on the item is drawn again.
            for _ in 0..20 {
                let rolled = pool[draw.below(pool.len())];
                let origin = from_desecrated.then_some(ModifierOrigin::Desecrated);
                if !item.takes(rolled, origin) {
                    continue;
                }
                let allowed = rolled
                    .tiers
                    .iter()
                    .filter(|tier| tier.level <= ITEM_LEVEL)
                    .count();
                let first = rolled.tiers.len() - allowed;
                let tier = first + draw.below(allowed);
                item.add(draw, rolled, tier);
                break;
            }
        }
    }
    item
}

/// What the draw counts, over the lines of the items.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Counts {
    items: usize,
    lines: usize,
    /// The lines that add up two modifiers or more, among `lines`.
    summed_lines: usize,
    /// The lines read as their modifiers, each in its tier (one of two neighbouring tiers for
    /// a line read so), a prefix or suffix in doubt counting when the true one is among them.
    right: usize,
    /// The lines left without tier.
    without_tier: usize,
    /// The lines read as their modifiers, one in another tier: a value in two tiers takes the
    /// worst.
    other_tier: usize,
    /// The lines read as other modifiers: a hybrid as pure modifiers, a sum as one modifier…
    other_modifier: usize,
    /// Those of `other_modifier` that add up two modifiers or more.
    summed_other_modifier: usize,
    /// The most readings looked at for an item, of [`MAX_READINGS`].
    most_readings: usize,
    /// The most steps of the search of the other readings for an item, of `MAX_STEPS`.
    most_steps: usize,
}

impl Counts {
    const ZERO: Self = Self {
        items: 0,
        lines: 0,
        summed_lines: 0,
        right: 0,
        without_tier: 0,
        other_tier: 0,
        other_modifier: 0,
        summed_other_modifier: 0,
        most_readings: 0,
        most_steps: 0,
    };
}

/// How a line was read against the modifiers it rolled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Read {
    Right,
    WithoutTier,
    OtherTier,
    OtherModifier,
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
    /// Whether `read` on the lines `lines` of the item is this modifier (its affix, its lines
    /// and its tiers on the base), and whether its tier is among those `read` may be.
    fn is(&self, affix: Affix, read: &ModifierTiers, lines: &[usize]) -> (bool, bool) {
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
        let same =
            Affix::from(self.rolled.table.affix) == affix && own == lines && tiers == read_tiers;
        let number = u32::try_from(self.tier + 1).expect("a tier number");
        (same, read.possible().any(|tier| tier.number == number))
    }
}

/// How the line `line` of `item` was read in `found`.
fn read_as(item: &DrawnItem<'_>, line: usize, found: &[FoundTiers]) -> Read {
    let Some(found) = found.iter().find(|found| found.modifiers.contains(&line)) else {
        return Read::WithoutTier;
    };
    let whole: Vec<usize> = found.modifiers.clone().collect();
    let added = found.added.as_ref().filter(|added| added.line == line);
    // Each way to read the line: the modifier of its lines, of either affix in doubt, and
    // the one added up on it.
    let truth: Vec<&TrueModifier<'_>> = item.lines[line]
        .modifiers
        .iter()
        .map(|&index| &item.modifiers[index])
        .collect();
    let mut best = Read::OtherModifier;
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
                if first.iter().all(|&(same, _)| same) {
                    first
                } else {
                    pair(other, one)
                }
            }
            _ => continue,
        };
        if read.iter().all(|&(same, tier)| same && tier) {
            return Read::Right;
        }
        if read.iter().all(|&(same, _)| same) {
            best = Read::OtherTier;
        }
    }
    best
}

/// The items of one way of the draw, read and counted; with the types left out of the draw.
fn measure(seed: u64, share: Desecrated, shuffled: bool) -> (Counts, Vec<&'static str>) {
    let mut draw = Draw(seed);
    let mut counts = Counts::ZERO;
    let mut left_out = Vec::new();
    for base_type in BASES {
        let base = bases::spawn(Language::English, base_type).expect(base_type);
        let (ordinary, unwritten) = rolled(&TABLES, base, false);
        left_out.extend(unwritten);
        let (desecrated, unwritten) = rolled(&TABLES, base, true);
        left_out.extend(unwritten);
        let trade_item = TradeItem {
            language: Language::English,
            base_type,
            base,
            item_level: Some(ITEM_LEVEL),
        };
        let mut read = 0;
        while read < ITEMS_PER_BASE {
            let mut item = drawn(&mut draw, &ordinary, &desecrated, share);
            if shuffled {
                item.shuffle(&mut draw);
            }
            // A sum no string writes (a line of two signs): another item.
            let Some(modifiers) = item.copied() else {
                continue;
            };
            read += 1;
            let limits = AffixLimits::new(3, 3);
            let (found, effort) = trade_tiers_in(&TABLES, &trade_item, &modifiers, limits, false);
            counts.items += 1;
            counts.most_readings = counts.most_readings.max(effort.readings);
            counts.most_steps = counts.most_steps.max(effort.steps);
            for (index, line) in item.lines.iter().enumerate() {
                let summed = line.modifiers.len() > 1;
                counts.lines += 1;
                counts.summed_lines += usize::from(summed);
                match read_as(&item, index, &found) {
                    Read::Right => counts.right += 1,
                    Read::WithoutTier => counts.without_tier += 1,
                    Read::OtherTier => counts.other_tier += 1,
                    Read::OtherModifier => {
                        counts.other_modifier += 1;
                        counts.summed_other_modifier += usize::from(summed);
                    }
                }
            }
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
    for (index, (share, shuffled)) in WAYS.into_iter().enumerate() {
        let seed = 0x9E37_79B9_7F4A_7C15 + u64::try_from(index).expect("an index");
        let (counts, left_out) = measure(seed, share, shuffled);
        println!("{share:?} desecrated, shuffled {shuffled}: {counts:#?}");
        println!("Not drawn, a line no string writes back: {left_out:?}");
        // Far from the bounds: past them, the lines of an item are left without tier.
        assert!(counts.most_readings < MAX_READINGS / 4, "{counts:?}");
        assert!(
            counts.most_steps < summed_readings::MAX_STEPS / 4,
            "{counts:?}"
        );
        measured.push(counts);
    }

    assert_eq!(measured, RECORDED);
}
