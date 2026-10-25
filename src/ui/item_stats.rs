//! The stats of the item, unfolded under the item block: its modifiers in the color of the
//! game, grouped, each with its roll. A unique shows a bar of where its value falls in its
//! range; a rare or magic item, its tier and the range of that tier (its bar over every tier
//! comes with the modifier tables, step 13).
//!
//! The rows are decided apart from the drawing ([`rows`]), and tested.

use egui::{Rect, RichText, Sense, Ui, Vec2, pos2, vec2};

use super::theme::{color, text};
use super::widgets;
use crate::format;
use crate::i18n::{Message, Translator};
use crate::item::{Affix, Item, Modifier, ModifierKind, Range, Rarity, StatLine};
use crate::rolls;

/// The space between two rows.
const ROW_GAP: f32 = 6.0;
/// The column on the left of each stat, for its filter (step 10b).
const GUTTER: f32 = 20.0;
/// The space between the gutter and the text, and between a bar and its range.
const GAP: f32 = 8.0;
/// The bar of a roll.
const BAR_SIZE: Vec2 = vec2(96.0, 4.0);
/// The space above the roll of a stat.
const ROLL_GAP: f32 = 3.0;
/// The space around a separator.
const SEPARATOR_MARGIN: f32 = 2.0;
/// The separator of the ranges of a stat with several rolls.
const RANGE_SEPARATOR: &str = " · ";

/// A small title of the stats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Group {
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
}

/// The roll of a stat.
#[derive(Debug, Clone, PartialEq)]
enum Roll {
    /// A unique: where its value falls in its ranges, from 0 to 1, and the ranges.
    Bar { share: f32, ranges: Vec<Range> },
    /// A rare or magic item: its tier, and the ranges of that tier.
    Tier { tier: u32, ranges: Vec<Range> },
}

/// A row of the stats.
#[derive(Debug, Clone, PartialEq)]
enum Row<'a> {
    Group(Group),
    /// Between the implicit and the explicit modifiers of a unique.
    Separator,
    Stat {
        text: &'a str,
        roll: Option<Roll>,
    },
}

/// The number of stats of `item`, for the pill that unfolds them.
#[must_use]
pub fn count(item: &Item) -> u32 {
    u32::try_from(item.modifiers().len()).unwrap_or(u32::MAX)
}

/// The rows of the stats of `item`; `unique_modifiers` are the modifiers of its unique in the
/// price database, the ranges of a unique copied without them.
fn rows<'a>(item: &'a Item, unique_modifiers: &[String]) -> Vec<Row<'a>> {
    let modifiers = item.modifiers();
    let implicit: Vec<&Modifier> = modifiers
        .iter()
        .filter(|modifier| !matches!(modifier.kind, ModifierKind::Explicit(_)))
        .collect();
    let explicit = |affix: Option<Affix>| -> Vec<&Modifier> {
        modifiers
            .iter()
            .filter(|modifier| modifier.kind == ModifierKind::Explicit(affix))
            .collect()
    };
    let mut rows = Vec::new();
    if item.rarity() == Rarity::Unique {
        let lines = |modifiers: &[&'a Modifier], rows: &mut Vec<Row<'a>>| {
            for line in modifiers.iter().flat_map(|modifier| &modifier.lines) {
                rows.push(Row::Stat {
                    text: &line.text,
                    roll: unique_roll(line, unique_modifiers),
                });
            }
        };
        let explicit: Vec<&Modifier> = modifiers
            .iter()
            .filter(|modifier| matches!(modifier.kind, ModifierKind::Explicit(_)))
            .collect();
        lines(&implicit, &mut rows);
        if !implicit.is_empty() && !explicit.is_empty() {
            rows.push(Row::Separator);
        }
        lines(&explicit, &mut rows);
        return rows;
    }
    let max = match item.rarity() {
        Rarity::Magic => 1,
        _ if item.is_jewel() => 2,
        _ => 3,
    };
    let groups = [
        (Group::Implicit, implicit),
        (
            Group::Prefixes { filled: 0, max },
            explicit(Some(Affix::Prefix)),
        ),
        (
            Group::Suffixes { filled: 0, max },
            explicit(Some(Affix::Suffix)),
        ),
        (Group::Explicit, explicit(None)),
    ];
    for (group, modifiers) in groups {
        if modifiers.is_empty() {
            continue;
        }
        let filled = u32::try_from(modifiers.len()).unwrap_or(u32::MAX);
        rows.push(Row::Group(match group {
            Group::Prefixes { max, .. } => Group::Prefixes { filled, max },
            Group::Suffixes { max, .. } => Group::Suffixes { filled, max },
            other => other,
        }));
        for modifier in modifiers {
            for (index, line) in modifier.lines.iter().enumerate() {
                // The tier is the one of the modifier, written once, with its first line.
                let roll = modifier.tier.filter(|_| index == 0).map(|tier| Roll::Tier {
                    tier,
                    ranges: line.rolls.iter().filter_map(|roll| roll.range).collect(),
                });
                rows.push(Row::Stat {
                    text: &line.text,
                    roll,
                });
            }
        }
    }
    rows
}

/// The roll of a line of a unique: its ranges from the advanced copy, or from the modifiers
/// of the price database; none for a line without a range.
fn unique_roll(line: &StatLine, unique_modifiers: &[String]) -> Option<Roll> {
    let copied: Vec<Range> = line.rolls.iter().filter_map(|roll| roll.range).collect();
    let ranges = if copied.is_empty() {
        rolls::unique_ranges(&line.text, unique_modifiers)?
    } else {
        copied
    };
    let shares: Vec<f64> = line
        .rolls
        .iter()
        .zip(&ranges)
        .map(|(roll, range)| share(roll.value.abs(), *range))
        .collect();
    if shares.is_empty() {
        return None;
    }
    #[expect(clippy::cast_precision_loss, reason = "a few rolls")]
    let average = shares.iter().sum::<f64>() / shares.len() as f64;
    #[expect(clippy::cast_possible_truncation, reason = "a share from 0 to 1")]
    let share = average as f32;
    Some(Roll::Bar { share, ranges })
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
pub fn show(ui: &mut Ui, item: &Item, unique_modifiers: &[String], i18n: &Translator) {
    for (index, row) in rows(item, unique_modifiers).into_iter().enumerate() {
        if index > 0 {
            ui.add_space(ROW_GAP);
        }
        match row {
            Row::Group(group) => group_title(ui, group, i18n),
            Row::Separator => {
                ui.add_space(SEPARATOR_MARGIN);
                let (rect, _) =
                    ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
                ui.painter().rect_filled(rect, 0.0, color::CONTROL_LINE);
                ui.add_space(SEPARATOR_MARGIN);
            }
            Row::Stat { text, roll } => stat(ui, text, roll.as_ref()),
        }
    }
}

/// A small title, with the fill of the prefixes and suffixes: lighter while one is open.
fn group_title(ui: &mut Ui, group: Group, i18n: &Translator) {
    let (message, fill) = match group {
        Group::Implicit => (Message::GroupImplicit, None),
        Group::Prefixes { filled, max } => (Message::GroupPrefixes, Some((filled, max))),
        Group::Suffixes { filled, max } => (Message::GroupSuffixes, Some((filled, max))),
        Group::Explicit => (Message::GroupExplicit, None),
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

/// A stat in the color of the modifiers, after the gutter of its filter, and its roll under
/// it.
fn stat(ui: &mut Ui, line: &str, roll: Option<&Roll>) {
    ui.horizontal_top(|ui| {
        ui.add_space(GUTTER + GAP);
        ui.vertical(|ui| {
            ui.add(
                egui::Label::new(
                    RichText::new(line)
                        .font(text::body())
                        .color(color::MODIFIER),
                )
                .wrap(),
            );
            if let Some(roll) = roll {
                ui.add_space(ROLL_GAP);
                roll_line(ui, roll);
            }
        });
    });
}

/// The bar of a unique and its ranges, or the tier of a rare and the ranges of that tier.
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

fn range_label(ui: &mut Ui, ranges: &[Range]) {
    let galley = widgets::text_line(ui, ranges_text(ranges), text::roll());
    let (rect, _) = ui.allocate_exact_size(galley.size(), Sense::hover());
    ui.painter()
        .galley(pos2(rect.left(), rect.top()), galley, color::FAINT);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::CopiedItem;

    fn parse(text: &str) -> Item {
        Item::parse(&CopiedItem::from_text(text).expect("an item")).expect("read")
    }

    /// The rows as texts, e.g. `Prefixes 2/3`, `-`, `+55 to maximum Life [40–60 0.75]`,
    /// `+3 to Level [T1 3–3]`.
    fn texts(item: &Item, unique_modifiers: &[String]) -> Vec<String> {
        rows(item, unique_modifiers)
            .into_iter()
            .map(|row| match row {
                Row::Group(Group::Implicit) => "Implicit".to_owned(),
                Row::Group(Group::Explicit) => "Explicit".to_owned(),
                Row::Group(Group::Prefixes { filled, max }) => format!("Prefixes {filled}/{max}"),
                Row::Group(Group::Suffixes { filled, max }) => format!("Suffixes {filled}/{max}"),
                Row::Separator => "-".to_owned(),
                Row::Stat { text, roll: None } => text.to_owned(),
                Row::Stat {
                    text,
                    roll: Some(Roll::Bar { share, ranges }),
                } => format!("{text} [{} {share:.2}]", ranges_text(&ranges)),
                Row::Stat {
                    text,
                    roll: Some(Roll::Tier { tier, ranges }),
                } => format!("{text} [T{tier} {}]", ranges_text(&ranges)),
            })
            .collect()
    }

    #[test]
    fn a_unique_copied_with_ctrl_c_gets_its_ranges_from_the_price_database() {
        let headhunter = parse(
            "Item Class: Belts\nRarity: Unique\nHeadhunter\nHeavy Belt\n--------\n\
             Item Level: 80\n--------\n26% increased Stun Threshold (implicit)\n--------\n\
             +55 to maximum Life\nWhen you kill a Rare monster, you gain its Modifiers for 60 seconds",
        );
        let modifiers = [
            "(20-30)% increased Stun Threshold".to_owned(),
            "+(40-60) to maximum Life".to_owned(),
        ];

        assert_eq!(
            texts(&headhunter, &modifiers),
            [
                "26% increased Stun Threshold [20–30 0.60]",
                "-",
                "+55 to maximum Life [40–60 0.75]",
                "When you kill a Rare monster, you gain its Modifiers for 60 seconds",
            ]
        );
    }

    #[test]
    fn a_unique_copied_with_ctrl_alt_c_has_its_own_ranges() {
        let focus = parse(include_str!("../item/test-items/en-unique-focus.txt"));

        let texts = texts(&focus, &[]);

        assert!(
            texts.iter().any(|text| text.contains('[')),
            "a roll with its range: {texts:?}"
        );
    }

    #[test]
    fn a_rare_shows_its_prefixes_and_suffixes_with_their_tiers() {
        let crossbow = parse(include_str!(
            "../item/test-items/en-rare-crossbow-all-modifiers.txt"
        ));

        let texts = texts(&crossbow, &[]);

        assert!(
            texts.iter().any(|text| text.starts_with("Prefixes ")),
            "{texts:?}"
        );
        assert!(
            texts.iter().any(|text| text.starts_with("Suffixes ")),
            "{texts:?}"
        );
        assert!(texts.iter().any(|text| text.contains("[T1 ")), "{texts:?}");
    }

    #[test]
    fn a_magic_item_has_one_prefix_and_one_suffix_at_most() {
        let mace = parse(include_str!("../item/test-items/en-magic-mace.txt"));

        assert_eq!(
            texts(&mace, &[]),
            [
                "Prefixes 1/1",
                "Adds 1 to 50 Lightning Damage [T7 1–4 · 46–66]",
                "Suffixes 1/1",
                "+8 to Strength [T8 5–8]",
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
