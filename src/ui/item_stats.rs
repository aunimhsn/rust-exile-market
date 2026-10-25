//! The stats of the item, unfolded under the item block: its modifiers in the color of the
//! game, grouped, the stats that vary first. A unique, or an implicit modifier of a rare,
//! shows a bar of where its value falls in its range; a prefix or a suffix, its tier and the
//! range of that tier (its bar over every tier comes with the modifier tables, step 13).
//!
//! A stat that is a filter starts after the column of its checkbox and keeps the column of
//! its minimum on the right (both come with step 10b); a stat of a unique that never varies
//! isn't a filter and starts at the left edge.
//!
//! The rows are decided apart from the drawing ([`rows`]), and tested.

use egui::text::{LayoutJob, TextFormat};
use egui::{Align, Label, Layout, Rect, RichText, Sense, Ui, Vec2, vec2};

use super::theme::{color, text};
use super::widgets;
use crate::format;
use crate::i18n::{Message, Translator};
use crate::item::{Affix, Item, Modifier, ModifierKind, Range, Rarity, StatLine};
use crate::prices::UniqueModifiers;
use crate::rolls;

/// The space between two rows.
const ROW_GAP: f32 = 6.0;
/// The space added above a small title, after the first.
const GROUP_GAP: f32 = 6.0;
/// The column on the left of a filter, for its checkbox (step 10b).
const GUTTER: f32 = 20.0;
/// The space between the columns of a stat, and between a bar and its range.
const GAP: f32 = 8.0;
/// The column on the right of a filter, for its minimum (step 10b): the text never runs
/// under it.
const MIN_COLUMN: f32 = 46.0;
/// The lines of the text of a stat, at most; the whole text is in its tooltip.
const MAX_LINES: usize = 2;
/// The bar of a roll.
const BAR_SIZE: Vec2 = vec2(96.0, 4.0);
/// The space above the roll of a stat.
const ROLL_GAP: f32 = 3.0;
/// The separator of the ranges of a stat with several rolls.
const RANGE_SEPARATOR: &str = " · ";

/// A small title of the stats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Group {
    /// The enchantments, and the modifiers added by the corruption.
    Enchant,
    Runes,
    Implicit,
    Prefixes {
        filled: u32,
        max: u32,
    },
    Suffixes {
        filled: u32,
        max: u32,
    },
    /// The explicit modifiers of a magic or rare item copied without their affixes.
    Explicit,
    /// The modifiers of a unique, no prefix nor suffix.
    Unique,
}

/// The roll of a stat.
#[derive(Debug, Clone, PartialEq)]
enum Roll {
    /// Where its value falls in its ranges, from 0 to 1, and the ranges.
    Bar { share: f32, ranges: Vec<Range> },
    /// A prefix or a suffix: its tier, and the ranges of that tier.
    Tier { tier: u32, ranges: Vec<Range> },
}

/// A stat: a line of a modifier.
#[derive(Debug, Clone, PartialEq)]
struct Stat<'a> {
    text: &'a str,
    /// A filter of the search, with its checkbox and its minimum.
    filter: bool,
    roll: Option<Roll>,
}

/// A row of the stats.
#[derive(Debug, Clone, PartialEq)]
enum Row<'a> {
    Group(Group),
    Stat(Stat<'a>),
}

/// Where the ranges of the rolls of a unique come from.
#[derive(Debug, Clone, Copy)]
enum RangeSource<'a> {
    /// The advanced copy: a line without a range never varies.
    Copied,
    /// The modifiers of the unique in the price database, of the kind of the line.
    Database(&'a [String]),
    /// Unknown, e.g. an item copied in French with Ctrl+C: a line with a number may vary.
    Unknown,
}

/// The number of stats of `item`, for the pill that unfolds them.
#[must_use]
pub fn count(item: &Item) -> u32 {
    len(item.modifiers())
}

fn len<T>(list: &[T]) -> u32 {
    u32::try_from(list.len()).unwrap_or(u32::MAX)
}

/// The rows of the stats of `item`; `unique_modifiers` are the modifiers of its unique in the
/// price database, the ranges of a unique copied without them.
fn rows<'a>(item: &'a Item, unique_modifiers: Option<&UniqueModifiers>) -> Vec<Row<'a>> {
    if item.rarity() == Rarity::Unique {
        unique_rows(item, unique_modifiers)
    } else {
        affix_rows(item)
    }
}

/// The rows of a unique: its enchantments, its runes, the implicit modifiers of its base,
/// then its own modifiers.
fn unique_rows<'a>(item: &'a Item, unique_modifiers: Option<&UniqueModifiers>) -> Vec<Row<'a>> {
    let copied = item
        .modifiers()
        .iter()
        .flat_map(|modifier| &modifier.lines)
        .any(|line| line.rolls.iter().any(|roll| roll.range.is_some()));
    let source = |database: fn(&UniqueModifiers) -> &[String]| match unique_modifiers {
        _ if copied => RangeSource::Copied,
        Some(modifiers) => RangeSource::Database(database(modifiers)),
        None => RangeSource::Unknown,
    };
    let implicit = source(|modifiers| &modifiers.implicit);
    let explicit = source(|modifiers| &modifiers.explicit);
    // A stat of a unique varies when it is a filter.
    let filter = |_: &StatLine, stat: &Stat<'_>| stat.filter;
    let mut rows = Vec::new();
    push_group(
        &mut rows,
        Group::Enchant,
        stats(item, is_enchant, added, filter),
    );
    push_group(&mut rows, Group::Runes, stats(item, is_rune, added, filter));
    push_group(
        &mut rows,
        Group::Implicit,
        stats(
            item,
            is_implicit,
            |line| unique_stat(line, implicit),
            filter,
        ),
    );
    push_group(
        &mut rows,
        Group::Unique,
        stats(
            item,
            |kind| matches!(kind, ModifierKind::Explicit(_)),
            |line| unique_stat(line, explicit),
            filter,
        ),
    );
    rows
}

/// The rows of a rare, magic or normal item: its enchantments, its runes, its implicit
/// modifiers, then its prefixes and suffixes.
fn affix_rows(item: &Item) -> Vec<Row<'_>> {
    let max = match item.rarity() {
        Rarity::Magic => 1,
        _ if item.is_jewel() => 2,
        _ => 3,
    };
    let line_varies = |line: &StatLine, _: &Stat<'_>| varies(line);
    let prefixes = affix_stats(item, Some(Affix::Prefix));
    let suffixes = affix_stats(item, Some(Affix::Suffix));
    let mut rows = Vec::new();
    push_group(
        &mut rows,
        Group::Enchant,
        stats(item, is_enchant, added, line_varies),
    );
    push_group(
        &mut rows,
        Group::Runes,
        stats(item, is_rune, added, line_varies),
    );
    push_group(
        &mut rows,
        Group::Implicit,
        stats(item, is_implicit, implicit_stat, line_varies),
    );
    push_group(
        &mut rows,
        Group::Prefixes {
            filled: prefixes.count,
            max,
        },
        prefixes.stats,
    );
    push_group(
        &mut rows,
        Group::Suffixes {
            filled: suffixes.count,
            max,
        },
        suffixes.stats,
    );
    push_group(&mut rows, Group::Explicit, affix_stats(item, None).stats);
    rows
}

/// A line of an enchantment or a rune: a filter, since the trade site searches them, without
/// a roll.
fn added(line: &StatLine) -> Stat<'_> {
    Stat {
        text: &line.text,
        filter: true,
        roll: None,
    }
}

/// A line of an implicit modifier of a rare, magic or normal item: a filter, rolled in the
/// range its base gives it.
fn implicit_stat(line: &StatLine) -> Stat<'_> {
    Stat {
        text: &line.text,
        filter: true,
        roll: bar(copied_rolls(line)),
    }
}

/// The number of modifiers of an affix, and their stats.
struct AffixStats<'a> {
    count: u32,
    stats: Vec<Stat<'a>>,
}

/// The explicit modifiers of `affix` (`None`: copied without their affixes), those with a
/// line that varies first, each with its tier written once, with its first line.
fn affix_stats(item: &Item, affix: Option<Affix>) -> AffixStats<'_> {
    let mut modifiers: Vec<&Modifier> = item
        .modifiers()
        .iter()
        .filter(|modifier| modifier.kind == ModifierKind::Explicit(affix))
        .collect();
    modifiers.sort_by_key(|modifier| !modifier.lines.iter().any(varies));
    let stats = modifiers
        .iter()
        .flat_map(|modifier| {
            modifier.lines.iter().enumerate().map(|(index, line)| Stat {
                text: &line.text,
                filter: true,
                roll: modifier.tier.filter(|_| index == 0).map(|tier| Roll::Tier {
                    tier,
                    ranges: copied_rolls(line)
                        .into_iter()
                        .map(|(_, range)| range)
                        .collect(),
                }),
            })
        })
        .collect();
    AffixStats {
        count: len(&modifiers),
        stats,
    }
}

/// Adds the title of `group` and its stats, when it has some.
fn push_group<'a>(rows: &mut Vec<Row<'a>>, group: Group, stats: Vec<Stat<'a>>) {
    if !stats.is_empty() {
        rows.push(Row::Group(group));
        rows.extend(stats.into_iter().map(Row::Stat));
    }
}

/// The stats of the modifiers of `item` whose kind passes `keep`, made by `stat`: the
/// modifiers with a stat that `varies` first, each with its lines together.
fn stats<'a>(
    item: &'a Item,
    keep: impl Fn(ModifierKind) -> bool,
    stat: impl Fn(&'a StatLine) -> Stat<'a>,
    varies: impl Fn(&StatLine, &Stat<'_>) -> bool,
) -> Vec<Stat<'a>> {
    let mut modifiers: Vec<(bool, Vec<Stat<'a>>)> = item
        .modifiers()
        .iter()
        .filter(|modifier| keep(modifier.kind))
        .map(|modifier| {
            let stats: Vec<Stat<'a>> = modifier.lines.iter().map(&stat).collect();
            let fixed = !modifier
                .lines
                .iter()
                .zip(&stats)
                .any(|(line, stat)| varies(line, stat));
            (fixed, stats)
        })
        .collect();
    modifiers.sort_by_key(|&(fixed, _)| fixed);
    modifiers.into_iter().flat_map(|(_, stats)| stats).collect()
}

/// A line with a number whose range isn't known, or is wider than one value: not `Loads
/// an additional bolt`, nor `5(5-5)% increased Movement Speed`.
fn varies(line: &StatLine) -> bool {
    line.rolls
        .iter()
        .any(|roll| roll.range.is_none_or(|range| range.max > range.min))
}

fn is_enchant(kind: ModifierKind) -> bool {
    matches!(kind, ModifierKind::Enchant | ModifierKind::Corruption)
}

fn is_rune(kind: ModifierKind) -> bool {
    kind == ModifierKind::Rune
}

fn is_implicit(kind: ModifierKind) -> bool {
    kind == ModifierKind::Implicit
}

/// A line of a unique: a filter with its bar when it varies, at the left edge otherwise.
fn unique_stat<'a>(line: &'a StatLine, source: RangeSource<'_>) -> Stat<'a> {
    let roll = match source {
        RangeSource::Copied => bar(copied_rolls(line)),
        RangeSource::Database(modifiers) => bar(database_rolls(line, modifiers)),
        RangeSource::Unknown => None,
    };
    let filter = match source {
        RangeSource::Unknown => !line.rolls.is_empty(),
        RangeSource::Copied | RangeSource::Database(_) => roll.is_some(),
    };
    Stat {
        text: &line.text,
        filter,
        roll,
    }
}

/// The values of a line with the ranges of the advanced copy, without their sign.
fn copied_rolls(line: &StatLine) -> Vec<(f64, Range)> {
    line.rolls
        .iter()
        .filter_map(|roll| Some((roll.value, roll.range?.magnitude())))
        .collect()
}

/// The values of a line copied with Ctrl+C, with the ranges of the first of `modifiers` it
/// matches.
fn database_rolls(line: &StatLine, modifiers: &[String]) -> Vec<(f64, Range)> {
    match rolls::unique_ranges(&line.text, modifiers) {
        // One range or none for each number of the line.
        Some(ranges) if ranges.len() == line.rolls.len() => line
            .rolls
            .iter()
            .zip(ranges)
            .filter_map(|(roll, range)| Some((roll.value, range?)))
            .collect(),
        _ => Vec::new(),
    }
}

/// The bar of `rolls`, those whose range is wider than one value: the average of where each
/// value falls in its range; none when no roll varies.
fn bar(rolls: Vec<(f64, Range)>) -> Option<Roll> {
    let rolls: Vec<(f64, Range)> = rolls
        .into_iter()
        .filter(|(_, range)| range.max > range.min)
        .collect();
    if rolls.is_empty() {
        return None;
    }
    let shares: f64 = rolls
        .iter()
        .map(|&(value, range)| share(value.abs(), range))
        .sum();
    #[expect(clippy::cast_precision_loss, reason = "a few rolls")]
    let average = shares / rolls.len() as f64;
    #[expect(clippy::cast_possible_truncation, reason = "a share from 0 to 1")]
    let share = average as f32;
    Some(Roll::Bar {
        share,
        ranges: rolls.into_iter().map(|(_, range)| range).collect(),
    })
}

/// Where `value` falls in `range`, from 0 to 1; 1 for a range of one value.
fn share(value: f64, range: Range) -> f64 {
    if range.max <= range.min {
        return 1.0;
    }
    ((value - range.min) / (range.max - range.min)).clamp(0.0, 1.0)
}

/// The ranges of a stat, e.g. `40–60`, or `8–12 · 16–22` for several rolls.
fn ranges_text(ranges: &[Range]) -> String {
    ranges
        .iter()
        .map(|range| {
            format!(
                "{}–{}",
                format::number(range.min),
                format::number(range.max)
            )
        })
        .collect::<Vec<_>>()
        .join(RANGE_SEPARATOR)
}

/// Draws the stats of `item`, in the item block.
pub fn show(
    ui: &mut Ui,
    item: &Item,
    unique_modifiers: Option<&UniqueModifiers>,
    i18n: &Translator,
) {
    for (index, row) in rows(item, unique_modifiers).into_iter().enumerate() {
        if index > 0 {
            let group_gap = if matches!(row, Row::Group(_)) {
                GROUP_GAP
            } else {
                0.0
            };
            ui.add_space(ROW_GAP + group_gap);
        }
        match row {
            Row::Group(group) => group_title(ui, group, i18n),
            Row::Stat(stat) => stat_row(ui, &stat),
        }
    }
}

/// A small title, with the fill of the prefixes and suffixes: lighter while one is open.
fn group_title(ui: &mut Ui, group: Group, i18n: &Translator) {
    let (message, fill) = match group {
        Group::Enchant => (Message::GroupEnchant, None),
        Group::Runes => (Message::GroupRunes, None),
        Group::Implicit => (Message::GroupImplicit, None),
        Group::Prefixes { filled, max } => (Message::GroupPrefixes, Some((filled, max))),
        Group::Suffixes { filled, max } => (Message::GroupSuffixes, Some((filled, max))),
        Group::Explicit => (Message::GroupExplicit, None),
        Group::Unique => (Message::GroupUnique, None),
    };
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        let title = widgets::spaced_line(
            ui,
            i18n.text(message).to_uppercase(),
            text::label(),
            text::LABEL_LETTER_SPACING,
        );
        let (rect, _) = ui.allocate_exact_size(title.size(), Sense::hover());
        ui.painter().galley(rect.min, title, color::FAINT);
        if let Some((filled, max)) = fill {
            let tint = if filled < max {
                color::ACCENT
            } else {
                color::WEAK
            };
            ui.label(
                RichText::new(i18n.affix_fill(filled, max))
                    .font(text::label())
                    .color(tint),
            );
        }
    });
}

/// A stat in the color of the modifiers, on two lines at most, and its roll under it; a
/// filter between the columns of its checkbox and its minimum.
fn stat_row(ui: &mut Ui, stat: &Stat<'_>) {
    ui.horizontal_top(|ui| {
        let mut width = ui.available_width();
        if stat.filter {
            ui.add_space(GUTTER + GAP);
            width -= GUTTER + GAP + GAP + MIN_COLUMN;
        }
        ui.allocate_ui_with_layout(
            vec2(width.max(0.0), 0.0),
            Layout::top_down(Align::Min),
            |ui| {
                ui.add(Label::new(clamped(stat.text)).wrap());
                if let Some(roll) = &stat.roll {
                    ui.add_space(ROLL_GAP);
                    roll_line(ui, roll);
                }
            },
        );
    });
}

/// The text of a stat, cut with "…" past its last line; the label shows the whole text in
/// its tooltip.
fn clamped(text: &str) -> LayoutJob {
    let mut job = LayoutJob::single_section(
        text.to_owned(),
        TextFormat::simple(text::body(), color::MODIFIER),
    );
    job.wrap.max_rows = MAX_LINES;
    job.wrap.overflow_character = Some('…');
    job
}

/// The bar of a roll and its ranges, or the tier of a prefix or suffix and the ranges of
/// that tier.
fn roll_line(ui: &mut Ui, roll: &Roll) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = GAP;
        match roll {
            Roll::Bar { share, ranges } => {
                let (rect, _) = ui.allocate_exact_size(BAR_SIZE, Sense::hover());
                let radius = BAR_SIZE.y / 2.0;
                let painter = ui.painter();
                painter.rect_filled(rect, radius, color::CONTROL_LINE);
                let fill = Rect::from_min_size(
                    rect.min,
                    vec2(rect.width() * share.clamp(0.0, 1.0), rect.height()),
                );
                painter.rect_filled(fill, radius, color::ACCENT);
                range_label(ui, ranges);
            }
            Roll::Tier { tier, ranges } => {
                let tint = match tier {
                    1 => color::TIER_1,
                    2 | 3 => color::ACCENT,
                    _ => color::FAINT,
                };
                ui.label(
                    RichText::new(format!("T{tier}"))
                        .font(text::tier())
                        .color(tint),
                );
                if !ranges.is_empty() {
                    range_label(ui, ranges);
                }
            }
        }
    });
}

/// The ranges of a roll, in faint text.
fn range_label(ui: &mut Ui, ranges: &[Range]) {
    let galley = widgets::text_line(ui, ranges_text(ranges), text::roll());
    let (rect, _) = ui.allocate_exact_size(galley.size(), Sense::hover());
    ui.painter().galley(rect.min, galley, color::FAINT);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::CopiedItem;

    fn parse(text: &str) -> Item {
        Item::parse(&CopiedItem::from_text(text).expect("an item")).expect("read")
    }

    fn database(implicit: &[&str], explicit: &[&str]) -> UniqueModifiers {
        let owned = |lines: &[&str]| lines.iter().map(|&line| line.to_owned()).collect();
        UniqueModifiers {
            implicit: owned(implicit),
            explicit: owned(explicit),
        }
    }

    /// The rows as texts: a title (`Prefixes 2/3`), or a stat, `□ ` first for a filter, with
    /// its roll (`□ +55 to maximum Life [40–60 0.75]`, `□ +3 to Level [T1 3–3]`).
    fn texts(item: &Item, unique_modifiers: Option<&UniqueModifiers>) -> Vec<String> {
        rows(item, unique_modifiers)
            .into_iter()
            .map(|row| match row {
                Row::Group(Group::Prefixes { filled, max }) => format!("Prefixes {filled}/{max}"),
                Row::Group(Group::Suffixes { filled, max }) => format!("Suffixes {filled}/{max}"),
                Row::Group(group) => format!("{group:?}"),
                Row::Stat(Stat { text, filter, roll }) => {
                    let check = if filter { "□ " } else { "" };
                    let roll = match roll {
                        None => String::new(),
                        Some(Roll::Bar { share, ranges }) => {
                            format!(" [{} {share:.2}]", ranges_text(&ranges))
                        }
                        Some(Roll::Tier { tier, ranges }) => {
                            format!(" [T{tier} {}]", ranges_text(&ranges))
                        }
                    };
                    format!("{check}{text}{roll}")
                }
            })
            .collect()
    }

    #[test]
    fn a_unique_copied_with_ctrl_c_gets_its_ranges_from_the_price_database() {
        let headhunter = parse(
            "Item Class: Belts\nRarity: Unique\nHeadhunter\nHeavy Belt\n--------\n\
             Item Level: 80\n--------\n26% increased Stun Threshold (implicit)\n--------\n\
             When you kill a Rare monster, you gain its Modifiers for 60 seconds\n\
             +55 to maximum Life",
        );
        let modifiers = database(
            &["(20-30)% increased Stun Threshold"],
            &[
                "+(40-60) to maximum Life",
                "When you kill a Rare monster, you gain its Modifiers for 60 seconds",
            ],
        );

        assert_eq!(
            texts(&headhunter, Some(&modifiers)),
            [
                "Implicit",
                "□ 26% increased Stun Threshold [20–30 0.60]",
                "Unique",
                // The stats that vary first.
                "□ +55 to maximum Life [40–60 0.75]",
                "When you kill a Rare monster, you gain its Modifiers for 60 seconds",
            ]
        );
    }

    #[test]
    fn an_implicit_and_an_explicit_of_the_same_stat_keep_their_own_ranges() {
        let ring = parse(
            "Item Class: Rings\nRarity: Unique\nVenom Loop\nAmethyst Ring\n--------\n\
             Item Level: 80\n--------\n\
             +10% to Chaos Resistance (implicit)\n--------\n+20% to Chaos Resistance\n\
             Adds 1 to 55 Lightning Damage to Attacks",
        );
        let modifiers = database(
            &["+(7-13)% to Chaos Resistance"],
            &[
                "+(17-23)% to Chaos Resistance",
                "Adds 1 to (50-60) Lightning Damage to Attacks",
            ],
        );

        assert_eq!(
            texts(&ring, Some(&modifiers)),
            [
                "Implicit",
                "□ +10% to Chaos Resistance [7–13 0.50]",
                "Unique",
                "□ +20% to Chaos Resistance [17–23 0.50]",
                // The fixed number before the roll keeps its place.
                "□ Adds 1 to 55 Lightning Damage to Attacks [50–60 0.50]",
            ]
        );
    }

    #[test]
    fn a_unique_copied_with_ctrl_alt_c_has_its_own_ranges() {
        let focus = parse(include_str!("../item/test-items/en-unique-focus.txt"));

        assert_eq!(
            texts(&focus, None),
            [
                "Unique",
                "□ 56% increased Energy Shield [50–70 0.30]",
                "□ +26% to Lightning Resistance [20–30 0.60]",
                "40% increased Mana Regeneration Rate while stationary",
                "+5% to Maximum Lightning Resistance",
                "40% increased Mana Regeneration Rate",
            ]
        );
    }

    #[test]
    fn a_negative_range_of_the_advanced_copy_is_without_its_sign() {
        let ring = parse(
            "Item Class: Rings\nRarity: Unique\nCold Loop\nRuby Ring\n--------\n\
             { Unique Modifier — Elemental, Fire, Resistance }\n-15(-20--10)% to Fire Resistance",
        );

        assert_eq!(
            texts(&ring, None),
            ["Unique", "□ -15% to Fire Resistance [10–20 0.50]"]
        );
    }

    #[test]
    fn a_french_unique_copied_with_ctrl_c_has_its_numbers_as_filters_without_bar() {
        let crossbow = parse(
            "Classe d'objet: Arbalètes\nRareté: Unique\nMurmure des brumes\nArbalète de fortune\n\
             --------\nNiveau de l'objet: 78\n--------\nLes Attaques Ricochent sur les ennemis\n\
             36% d'Augmentation d'Accumulation de Gel",
        );

        assert_eq!(
            texts(&crossbow, None),
            [
                "Unique",
                "□ 36% d'Augmentation d'Accumulation de Gel",
                "Les Attaques Ricochent sur les ennemis",
            ]
        );
    }

    #[test]
    fn a_rare_shows_its_enchantment_runes_implicit_prefixes_and_suffixes() {
        let crossbow = parse(include_str!(
            "../item/test-items/en-rare-crossbow-all-modifiers.txt"
        ));

        assert_eq!(
            texts(&crossbow, None),
            [
                "Enchant",
                "□ 45% increased Elemental Damage with Attacks",
                "Runes",
                "□ 18% increased Physical Damage",
                "□ Gain 30 Mana per enemy killed",
                "Implicit",
                "□ Loads an additional bolt",
                "Prefixes 3/3",
                "□ Adds 10 to 273 Lightning Damage [T2 1–16 · 239–300]",
                "□ Adds 24 to 51 Physical Damage [T3 23–35 · 39–59]",
                "□ 109% increased Elemental Damage with Attacks [T2 100–119]",
                "Suffixes 3/3",
                "□ Leeches 7.59% of Physical Damage as Mana [T1 7–7.9]",
                "□ Gain 21 Mana per enemy killed [T3 21–27]",
                "□ 18% increased Attack Speed [T1 17–19]",
            ]
        );
    }

    #[test]
    fn a_rare_copied_with_ctrl_c_has_its_explicit_modifiers_together() {
        let ring = parse(include_str!("../item/test-items/en-plain-rare-ring.txt"));

        assert_eq!(
            texts(&ring, None),
            [
                "Implicit",
                "□ +8% to all Elemental Resistances",
                "Explicit",
                "□ +143 to Evasion Rating",
                "□ +12 to Strength",
            ]
        );
    }

    #[test]
    fn a_fixed_implicit_of_a_rare_is_a_filter_without_bar_after_those_that_vary() {
        let boots = parse(
            "Item Class: Boots\nRarity: Rare\nGale Tread\nLattice Sandals\n--------\n\
             { Implicit Modifier }\n5(5-5)% increased Movement Speed\n\
             { Implicit Modifier }\n+12(10-15) to Dexterity\n--------\n\
             { Prefix Modifier \"Hale\" (Tier: 3) — Life }\n+86(85-99) to maximum Life\n\
             { Prefix Modifier \"Sturdy\" (Tier: 5) — Defences }\n\
             +20(15-25) to Evasion Rating\n+10(8-12)% increased Evasion Rating",
        );

        assert_eq!(
            texts(&boots, None),
            [
                "Implicit",
                "□ +12 to Dexterity [10–15 0.40]",
                "□ 5% increased Movement Speed",
                "Prefixes 2/3",
                "□ +86 to maximum Life [T3 85–99]",
                // The tier of a modifier of two lines, written once.
                "□ +20 to Evasion Rating [T5 15–25]",
                "□ +10% increased Evasion Rating",
            ]
        );
    }

    #[test]
    fn a_jewel_has_two_prefixes_and_two_suffixes_at_most() {
        let english = parse(
            "Item Class: Jewels\nRarity: Rare\nGlow Spark\nRuby\n--------\n\
             { Prefix Modifier \"Hale\" (Tier: 2) — Life }\n+12(10-14) to maximum Life",
        );
        let french = parse(
            "Classe d'objet: Joyaux\nRareté: Rare\nÉclat\nRubis\n--------\n\
             { Préfixe : « Sain » (Palier : 2) — Vie }\n+12(10-14) de Vie maximale",
        );

        assert!(english.is_jewel());
        assert_eq!(
            texts(&english, None),
            ["Prefixes 1/2", "□ +12 to maximum Life [T2 10–14]"]
        );
        assert_eq!(
            texts(&french, None),
            ["Prefixes 1/2", "□ +12 de Vie maximale [T2 10–14]"]
        );
    }

    #[test]
    fn a_magic_item_has_one_prefix_and_one_suffix_at_most() {
        let mace = parse(include_str!("../item/test-items/en-magic-mace.txt"));

        assert_eq!(
            texts(&mace, None),
            [
                "Prefixes 1/1",
                "□ Adds 1 to 50 Lightning Damage [T7 1–4 · 46–66]",
                "Suffixes 1/1",
                "□ +8 to Strength [T8 5–8]",
            ]
        );
    }

    #[test]
    fn a_range_of_one_value_is_full() {
        assert!((share(5.0, Range { min: 5.0, max: 5.0 }) - 1.0).abs() < 1e-9);
        assert!(
            (share(
                30.0,
                Range {
                    min: 20.0,
                    max: 40.0
                }
            ) - 0.5)
                .abs()
                < 1e-9
        );
    }
}
