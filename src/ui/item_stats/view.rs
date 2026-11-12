//! The drawing of the rows of the stats: the small titles, the rows with their checkbox, text
//! and range field, the tooltips, the bars and the tiers under a line.

use egui::text::{LayoutJob, TextFormat};
use egui::{
    Align, Align2, Area, Color32, CornerRadius, Frame, Id, Label, Layout, Margin, Order, Pos2,
    Rect, Response, RichText, Sense, Stroke, TextEdit, Ui, Vec2, ViewportId, pos2, vec2,
};

use super::Stats;
use super::model::{
    Group, Limit, ModifierTier, Origin, Row, Stat, StatFilter, StatRoll, StatText, TierRow,
    WordTip, radius_word, tier_word,
};
use super::rolls::ranges_text;
use crate::filters::{Filter, FilterKey, Filters, Half, Window};
use crate::format;
use crate::i18n::{Message, Translator};
use crate::item::{Currency, MAX_WAYSTONE_TIER, ModifierOrigin, RadiusSkills, Range};
use crate::ui::item_block::waystone_tier_color;
use crate::ui::number_field::NumberField;
use crate::ui::theme::{self, color, size, text};
use crate::ui::widgets;

/// The space between two rows.
const ROW_GAP: f32 = 6.0;

/// The space added above a small title, after the first.
const GROUP_GAP: f32 = 6.0;

/// The space between a small title and the fill of its prefixes or suffixes.
const TITLE_GAP: f32 = 6.0;

/// The column on the left of a filter, for its checkbox.
const CHECKBOX_COLUMN: f32 = 20.0;

/// The space between the columns of a stat, and between the parts of its roll.
const COLUMN_GAP: f32 = 8.0;

/// The column on the right of a filter, for its range field: the text never runs under it.
const RANGE_COLUMN: f32 = 62.0;

/// The column of a range field of four characters a side.
const WIDE_RANGE_COLUMN: f32 = 76.0;

/// A half of a range field, for three characters, and for four.
const HALF_WIDTH: f32 = 24.0;
const WIDE_HALF_WIDTH: f32 = 31.0;

/// The space around the halves of a range field.
const RANGE_PADDING: Margin = Margin::symmetric(2, 1);

/// The range field raised by 3 px on the line of its checkbox: taller than the checkbox, it
/// looked too low (tuned by eye by the owner on the mock-up: 1 px in step 13h, 3 px in step
/// 13k4a, a whole pixel so that every row is the same). The row keeps its height.
const RANGE_RAISE: Margin = Margin {
    left: 0,
    right: 0,
    top: -3,
    bottom: 3,
};

/// The space between a range field and its tooltip, under it.
const TOOLTIP_GAP: f32 = 8.0;

/// The space around the text of the tooltip of a range field, and between its lines.
const TOOLTIP_PADDING: Margin = Margin::symmetric(8, 5);
const TOOLTIP_LINE_GAP: f32 = 3.0;

/// The space above a small line of a tooltip, the tiers of the rolls right under them.
const TOOLTIP_SMALL_LINE_GAP: f32 = 1.0;

/// How far the tooltip of the "not allowed" sign starts left of the sign.
const UNSEARCHABLE_TOOLTIP_SHIFT: f32 = 4.0;

/// The space between the rows of several tiers under a line.
const TIER_ROW_GAP: f32 = 1.0;

/// The widest tooltip of a word after the ranges of a tier, wrapped past it: the tooltip of
/// "Hybrid" names a line of the item, which can be long.
const WORD_TIP_WIDTH: f32 = 230.0;

/// The lines of the text of a stat, at most; the whole text is in its tooltip.
const MAX_LINES: usize = 4;

/// The bar of a roll.
const BAR_SIZE: Vec2 = vec2(96.0, 4.0);

/// The space above the roll of a stat.
const ROLL_GAP: f32 = 3.0;

/// The space between the range of a tier and the origin of its modifier.
const ORIGIN_GAP: f32 = 6.0;

/// The space around the tag of the radius of a line of a Time-Lost jewel, inside its block.
const RADIUS_TAG_PADDING: Vec2 = vec2(5.0, 0.0);

/// The inner margin of the band of an unrevealed modifier, around its words.
const UNREVEALED_PADDING: Margin = Margin::symmetric(6, 3);

/// The band of an unrevealed modifier reaches 6 px out of the column of the stats, so that its
/// words stay at the left edge.
const UNREVEALED_OUTSET: Margin = Margin::symmetric(-6, 0);

impl Stats<'_> {
    /// Draws the stats, in the item block, with the filters the player set.
    pub fn show(self, ui: &mut Ui, filters: &mut Filters, i18n: &Translator) {
        // "min – max" on the right of the first title, above the range fields.
        let mut shows_range_title = self.rows.iter().any(|row| {
            matches!(
                row,
                Row::Stat(Stat {
                    filter: Some(StatFilter { limit: Some(_), .. }),
                    ..
                })
            )
        });
        for (index, row) in self.rows.into_iter().enumerate() {
            if index > 0 {
                let group_gap = if matches!(row, Row::Group(_)) {
                    GROUP_GAP
                } else {
                    0.0
                };
                ui.add_space(ROW_GAP + group_gap);
            }
            match row {
                Row::Group(group) => {
                    let shows_range = std::mem::take(&mut shows_range_title);
                    ui.horizontal(|ui| {
                        group_title(ui, group, i18n);
                        if shows_range {
                            range_title(ui, i18n);
                        }
                    });
                }
                Row::Stat(stat) => stat_row(ui, &stat, filters, i18n),
                Row::UnrevealedNote => note(ui, i18n.text(Message::UnrevealedNote)),
                Row::TradeCopyNote => note(ui, i18n.text(Message::TradeCopyNote)),
                Row::UniqueRollsNote => note(ui, i18n.text(Message::UniqueRollsNote)),
            }
        }
    }
}

/// A note under the stats, in faint text, wrapped.
fn note(ui: &mut Ui, words: String) {
    ui.label(RichText::new(words).font(text::note()).color(color::FAINT));
}

/// A small title, in a row, with the fill of the prefixes and suffixes: lighter while one is
/// open, the count alone without most.
fn group_title(ui: &mut Ui, group: Group, i18n: &Translator) {
    let (message, fill) = match group {
        Group::Item => (Message::GroupItem, None),
        Group::Weapon => (Message::GroupWeapon, None),
        Group::Waystone => (Message::GroupWaystone, None),
        Group::Gem => (Message::GroupGem, None),
        Group::Pseudo => (Message::GroupPseudo, None),
        Group::Enchant => (Message::GroupEnchant, None),
        Group::Runes => (Message::GroupRunes, None),
        Group::Implicit => (Message::GroupImplicit, None),
        Group::Prefixes { filled, max } => (Message::GroupPrefixes, Some((filled, max))),
        Group::Suffixes { filled, max } => (Message::GroupSuffixes, Some((filled, max))),
        Group::Explicit => (Message::GroupExplicit, None),
        Group::Unique => (Message::GroupUnique, None),
    };
    ui.spacing_mut().item_spacing.x = TITLE_GAP;
    small_title(ui, message, i18n);
    if let Some((filled, max)) = fill {
        let (words, tint) = match max {
            Some(max) if filled < max => (i18n.affix_fill(filled, max), color::ACCENT),
            Some(max) => (i18n.affix_fill(filled, max), color::WEAK),
            None => (filled.to_string(), color::WEAK),
        };
        ui.label(RichText::new(words).font(text::label()).color(tint));
    }
}

/// "min – max" on the right of the row of a title, above the range fields of the filters.
fn range_title(ui: &mut Ui, i18n: &Translator) {
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        small_title(ui, Message::FilterRange, i18n);
    });
}

/// The words of `message` in capitals, spaced, faint.
fn small_title(ui: &mut Ui, message: Message, i18n: &Translator) {
    let title = widgets::spaced_line(
        ui,
        i18n.text(message).to_uppercase(),
        text::label(),
        text::LABEL_LETTER_SPACING,
    );
    let (rect, _) = ui.allocate_exact_size(title.size(), Sense::hover());
    ui.painter().galley(rect.min, title, color::FAINT);
}

/// A stat in the color of the modifiers, on four lines at most, and its roll under it; a
/// filter between its checkbox and the column of its range field.
fn stat_row(ui: &mut Ui, stat: &Stat<'_>, filters: &mut Filters, i18n: &Translator) {
    if let StatText::Unrevealed(_) = stat.text {
        unrevealed_row(ui, &stat.text.text(i18n));
        return;
    }
    start_on_whole_pixel(ui);
    ui.horizontal_top(|ui| {
        if stat.is_unsearchable {
            unsearchable_row(ui, stat, i18n);
        } else {
            filter_row(ui, stat, filters, i18n);
        }
    });
}

/// The row starts on a whole pixel, so that its checkbox and its range field round alike on
/// every row.
fn start_on_whole_pixel(ui: &mut Ui) {
    let top = ui.cursor().top();
    let pixels = ui.pixels_per_point();
    ui.add_space((top * pixels).ceil() / pixels - top);
}

/// A line the trade site can't search: the "not allowed" sign in place of its checkbox, its
/// tooltip under the pointer, and the column of a range field kept empty.
fn unsearchable_row(ui: &mut Ui, stat: &Stat<'_>, i18n: &Translator) {
    let width = ui.available_width();
    let sign = widgets::not_allowed_sign(ui, vec2(CHECKBOX_COLUMN, line_height(ui)));
    if sign.hovered() {
        unsearchable_tooltip(ui, stat.key, sign.rect, i18n);
    }
    ui.add_space(COLUMN_GAP);
    let (column, _) = range_widths(None);
    stat_text(ui, stat, None, text_width(width, column), i18n);
}

/// A stat with its filter, the checkbox on the first line of its text, and its range field,
/// which the `filters` the player set fill; the item class searched in place of an unchecked
/// base type, under it. A stat without filter is its text alone, the whole row across.
fn filter_row(ui: &mut Ui, stat: &Stat<'_>, filters: &mut Filters, i18n: &Translator) {
    let width = ui.available_width();
    let Some(filter) = stat.filter else {
        stat_text(ui, stat, None, width, i18n);
        return;
    };
    let mut choice = filters.get(stat.key, || filter.default_choice());
    let before = choice.clone();
    let label = stat.text.label(i18n);
    if widgets::check_box(
        ui,
        choice.checked,
        vec2(CHECKBOX_COLUMN, line_height(ui)),
        &label,
    )
    .clicked()
    {
        choice.checked = !choice.checked;
    }
    ui.add_space(COLUMN_GAP);
    let note = match stat.text {
        StatText::BaseType {
            class: Some(class), ..
        } if !choice.checked => Some(i18n.filter_item_class(class)),
        _ => None,
    };
    let (column, _) = range_widths(filter.limit);
    stat_text(ui, stat, note.as_deref(), text_width(width, column), i18n);
    ui.add_space(COLUMN_GAP);
    let edited = filter
        .limit
        .and_then(|limit| range_field(ui, stat.key, &mut choice, limit, i18n));
    if choice != before {
        filters.set(stat.key, choice);
    }
    // Settled if its field disappears before the player leaves it.
    if let Some(half) = edited {
        filters.edit(window_of(ui), stat.key, half);
    }
}

/// The height of a line of the text of a stat, the height of its checkbox.
fn line_height(ui: &Ui) -> f32 {
    ui.fonts_mut(|fonts| fonts.row_height(&text::body()))
}

/// The width of the text of a stat in a row `width` across, between the column of the checkbox
/// and the range `column`.
fn text_width(width: f32, column: f32) -> f32 {
    (width - CHECKBOX_COLUMN - COLUMN_GAP - COLUMN_GAP - column).max(0.0)
}

/// The column of the range field of `limit` and the width of each of its halves: wider for
/// four characters a side.
fn range_widths(limit: Option<Limit>) -> (f32, f32) {
    if limit.is_some_and(Limit::is_wide) {
        (WIDE_RANGE_COLUMN, WIDE_HALF_WIDTH)
    } else {
        (RANGE_COLUMN, HALF_WIDTH)
    }
}

/// The window `ui` draws in.
fn window_of(ui: &Ui) -> Window {
    if ui.ctx().viewport_id() == ViewportId::ROOT {
        Window::Main
    } else {
        Window::OverTheGame
    }
}

/// A modifier not revealed yet: its words in the desecrated green, in the face of the modifier
/// lines, from the left edge, on a faint band of that green, like the row of the game.
fn unrevealed_row(ui: &mut Ui, words: &str) {
    Frame::new()
        .fill(color::UNREVEALED_BAND)
        .corner_radius(size::RADIUS)
        .inner_margin(UNREVEALED_PADDING)
        .outer_margin(UNREVEALED_OUTSET)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new(words)
                    .font(text::body())
                    .color(color::DESECRATED),
            );
        });
}

/// The text of a stat and its roll, the tag of the passive skills of its radius after it (alone
/// under a line without roll), or a `note` in faint text under it, `width` across.
fn stat_text(ui: &mut Ui, stat: &Stat, note: Option<&str>, width: f32, i18n: &Translator) {
    ui.allocate_ui_with_layout(vec2(width, 0.0), Layout::top_down(Align::Min), |ui| {
        ui.set_width(width);
        let response = ui.add(Label::new(clamped(&stat.text.text(i18n))).wrap());
        if let Some(tip) = stat.text.tip() {
            response.on_hover_ui(|ui| {
                ui.set_max_width(WORD_TIP_WIDTH);
                ui.add(Label::new(i18n.text(tip)).wrap());
            });
        }
        let skills = stat.text.radius_skills();
        if let Some(roll) = &stat.roll {
            ui.add_space(ROLL_GAP);
            roll_line(ui, roll, skills, i18n);
        } else if let Some(skills) = skills {
            ui.add_space(ROLL_GAP);
            roll_row(ui, |ui| radius_tag(ui, skills, i18n));
        }
        if let Some(note) = note {
            ui.add_space(ROLL_GAP);
            let galley = widgets::truncated_line(ui, note, text::roll(), width);
            let (rect, _) = ui.allocate_exact_size(galley.size(), Sense::hover());
            ui.painter().galley(rect.min, galley, color::FAINT);
        }
    });
}

/// The range field of a filter, in its column: its minimum, a dash, its maximum, each half a
/// number only, brought back within the range of the stat when the player leaves it; its
/// tooltip under the pointer; the half the player is in, if any.
fn range_field(
    ui: &mut Ui,
    key: FilterKey,
    choice: &mut Filter,
    limit: Limit,
    i18n: &Translator,
) -> Option<Half> {
    let halves = [Half::Minimum, Half::Maximum];
    let ids = halves.map(|half| ui.make_persistent_id(("stat-range", key, half)));
    let focused = ui.memory(|memory| ids.iter().any(|&id| memory.has_focus(id)));
    let line_color = if focused {
        color::WEAK
    } else {
        color::CONTROL_LINE
    };
    let (column, half_width) = range_widths(Some(limit));
    let stroke = Stroke::new(1.0, line_color);
    let mut edited = None;
    let dash_width =
        column - 2.0 * (f32::from(RANGE_PADDING.left) + stroke.width) - 2.0 * half_width;
    let field = Frame::NONE
        .fill(color::CONTROL)
        .stroke(stroke)
        .corner_radius(CornerRadius::same(size::RADIUS))
        .inner_margin(RANGE_PADDING)
        .outer_margin(RANGE_RAISE)
        .show(ui, |ui| {
            // The halves and the dash one line of numbers high, the dash in the middle.
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.spacing_mut().interact_size.y = 0.0;
            let height = ui.fonts_mut(|fonts| fonts.row_height(&text::range_field()));
            for (index, (half, id)) in halves.into_iter().zip(ids).enumerate() {
                if index > 0 {
                    range_dash(ui, vec2(dash_width, height));
                }
                let mut text = NumberField {
                    text: choice.text_mut(half),
                    negative: limit.allows_minus_sign(),
                };
                if range_half(ui, id, &mut text, half_width).lost_focus() {
                    choice.settle(half);
                }
                // Read in egui, as `Context::text_edit_focused`: the window may not have the
                // keyboard of the system yet.
                if ui.memory(|memory| memory.has_focus(id)) {
                    edited = Some(half);
                }
            }
        });
    // The field as it is drawn, without the margin that raises it.
    let rect = field.response.rect - RANGE_RAISE;
    if ui.rect_contains_pointer(rect) {
        field_tooltip(ui, key, rect, limit, i18n);
    }
    edited
}

/// The dash between the halves of a range field, faint, in the middle of its `size`.
fn range_dash(ui: &mut Ui, size: Vec2) {
    let galley = widgets::text_line(ui, "–", text::range_field());
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let at = rect.center() - galley.size() / 2.0;
    ui.painter().galley(at, galley, color::FAINT);
}

/// A half of a range field, of the id `id`, `width` across, editing its number `text`.
fn range_half(ui: &mut Ui, id: Id, text: &mut NumberField, width: f32) -> Response {
    ui.add(
        TextEdit::singleline(text)
            .id(id)
            .font(text::range_field())
            .text_color(color::TEXT)
            // The caret of an empty half in its middle.
            .align(Align2::CENTER_CENTER)
            .frame(Frame::NONE)
            .desired_width(width),
    )
}

/// A line of a tooltip: its text, its color, and whether it is small, like the ranges under
/// the stats.
#[derive(Debug, Clone, PartialEq)]
struct TooltipLine {
    text: String,
    tint: Color32,
    is_small: bool,
}

impl TooltipLine {
    fn new(text: String, tint: Color32) -> Self {
        Self {
            text,
            tint,
            is_small: false,
        }
    }
}

/// The lines of the tooltip of the range field of `limit`: the rolls it accepts, for a prefix
/// or suffix their tiers right under them, small and faint ("T13–T1"), then "Lower is better"
/// in gray for a line better lower.
fn tooltip_lines(limit: Limit, i18n: &Translator) -> Vec<TooltipLine> {
    let rolls = limit.rolls.map(|rolls| {
        TooltipLine::new(
            i18n.filter_rolls(&format::number(rolls.min), &format::number(rolls.max)),
            color::TEXT,
        )
    });
    let tiers = limit.rolls.and(limit.tiers).map(|tiers| TooltipLine {
        text: format!("T{}–T{}", tiers.lowest, tiers.highest),
        tint: color::FAINT,
        is_small: true,
    });
    let lower = (limit.half == Half::Maximum)
        .then(|| TooltipLine::new(i18n.text(Message::FilterLowerBetter), color::WEAK));
    [rolls, tiers, lower].into_iter().flatten().collect()
}

/// The color of a tier: tier 1 in a vivid orange, tiers 2 and 3 in white, the others faint.
fn tier_color(tier: u32) -> Color32 {
    match tier {
        1 => color::TIER_1,
        2 | 3 => color::ACCENT,
        _ => color::FAINT,
    }
}

/// The tooltip of a range field in `field`: the rolls it accepts, then "Lower is better" in
/// gray for a line better lower; none when it has nothing to say. Under the field, their
/// right edges together, so that it stays in the window.
fn field_tooltip(ui: &Ui, key: FilterKey, field: Rect, limit: Limit, i18n: &Translator) {
    let lines = tooltip_lines(limit, i18n);
    if lines.is_empty() {
        return;
    }
    let at = pos2(field.right(), field.bottom() + TOOLTIP_GAP);
    tooltip(
        ui,
        Id::NULL.with(("stat-range-tooltip", key)),
        at,
        Align2::RIGHT_TOP,
        lines,
    );
}

/// The tooltip of the "not allowed" sign of a line the trade site can't search, in `sign`:
/// two sentences, one per line. Under the sign, opening to the right since the sign is on the
/// left edge.
fn unsearchable_tooltip(ui: &Ui, key: FilterKey, sign: Rect, i18n: &Translator) {
    let lines = [Message::FilterUnsearchable, Message::FilterUnsearchableHint]
        .map(|message| TooltipLine::new(i18n.text(message), color::TEXT));
    let at = pos2(
        sign.left() - UNSEARCHABLE_TOOLTIP_SHIFT,
        sign.bottom() + TOOLTIP_GAP,
    );
    tooltip(
        ui,
        Id::NULL.with(("unsearchable-tooltip", key)),
        at,
        Align2::LEFT_TOP,
        lines.into(),
    );
}

/// A tooltip of `lines` in their colors, its corner `pivot` at `at`.
fn tooltip(ui: &Ui, id: Id, at: Pos2, pivot: Align2, lines: Vec<TooltipLine>) {
    Area::new(id)
        .order(Order::Tooltip)
        .interactable(false)
        .pivot(pivot)
        .fixed_pos(at)
        .show(ui.ctx(), |ui| {
            Frame::NONE
                .fill(color::BUBBLE)
                .corner_radius(CornerRadius::same(size::BUBBLE_RADIUS))
                .shadow(theme::floating_shadow())
                .inner_margin(TOOLTIP_PADDING)
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = Vec2::ZERO;
                    for (index, line) in lines.into_iter().enumerate() {
                        if index > 0 {
                            let gap = if line.is_small {
                                TOOLTIP_SMALL_LINE_GAP
                            } else {
                                TOOLTIP_LINE_GAP
                            };
                            ui.add_space(gap);
                        }
                        let font = if line.is_small {
                            text::roll()
                        } else {
                            text::note()
                        };
                        let galley = widgets::text_line(ui, line.text, font);
                        let (rect, _) = ui.allocate_exact_size(galley.size(), Sense::hover());
                        ui.painter().galley(rect.min, galley, line.tint);
                    }
                });
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

/// The bar of a roll and its ranges; for a line of a prefix or suffix, its bar over every
/// tier, then under the last line of its modifier its tier, the ranges of that tier and its
/// origin; or the tiers of a line that can be a prefix or a suffix, one under the other, then
/// its origin on a row of its own. Last, the tag of the passive `skills` of the radius of a line
/// of a Time-Lost jewel.
fn roll_line(ui: &mut Ui, roll: &StatRoll, skills: Option<RadiusSkills>, i18n: &Translator) {
    let ending = |origin| RollEnding { origin, skills };
    match roll {
        StatRoll::Bar {
            share,
            ranges,
            origin,
        } => roll_row(ui, |ui| {
            roll_bar(ui, *share, color::ACCENT);
            range_label(ui, ranges);
            ending(*origin).show(ui, i18n);
        }),
        StatRoll::Affix {
            share,
            tier,
            origin,
        } => roll_row(ui, |ui| {
            if let Some(share) = share {
                roll_bar(ui, *share, color::ACCENT);
            }
            if let Some(tier) = tier {
                tier_row(ui, tier, i18n);
            }
            ending(*origin).show(ui, i18n);
        }),
        StatRoll::Tiers { rows, origin } => tier_rows(ui, rows, ending(*origin), i18n),
        StatRoll::WaystoneTier(tier) => waystone_tier_roll(ui, *tier),
    }
}

/// What ends the roll of a line: the origin of its modifier, then the tag of the passive skills
/// of its radius.
#[derive(Clone, Copy)]
struct RollEnding {
    origin: Option<Origin>,
    skills: Option<RadiusSkills>,
}

impl RollEnding {
    /// Draws the origin, then the tag, those there are.
    fn show(self, ui: &mut Ui, i18n: &Translator) {
        if let Some(origin) = self.origin {
            origin_word(ui, origin, i18n);
        }
        if let Some(skills) = self.skills {
            radius_tag(ui, skills, i18n);
        }
    }
}

/// The tiers of a line that can be a prefix or a suffix, or that adds up two modifiers, one
/// under the other, then the `ending` of the line on a row of its own: the line has it
/// whichever tier it is.
fn tier_rows(ui: &mut Ui, rows: &[TierRow], ending: RollEnding, i18n: &Translator) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = TIER_ROW_GAP;
        for row in rows {
            roll_row(ui, |ui| tier_row(ui, row, i18n));
        }
        if ending.origin.is_some() || ending.skills.is_some() {
            roll_row(ui, |ui| ending.show(ui, i18n));
        }
    });
}

/// The tier of a waystone: a bar of where it falls over every tier, and "T" with its number,
/// both in its color.
fn waystone_tier_roll(ui: &mut Ui, tier: u32) {
    roll_row(ui, |ui| {
        let tint = waystone_tier_color(tier);
        roll_bar(ui, waystone_tier_share(tier), tint);
        ui.label(
            RichText::new(format!("T{tier}"))
                .font(text::tier())
                .color(tint),
        );
    });
}

/// Where the tier of a waystone falls over every tier, from 0 for tier 1 to 1 for the highest.
fn waystone_tier_share(tier: u32) -> f32 {
    let share = f64::from(tier.saturating_sub(1)) / f64::from(MAX_WAYSTONE_TIER - 1);
    #[expect(clippy::cast_possible_truncation, reason = "a share from 0 to 1")]
    let share = share.clamp(0.0, 1.0) as f32;
    share
}

/// A tier and its ranges ([`tier_label`]), then the word after them in gray, with its
/// tooltip wrapped past [`WORD_TIP_WIDTH`].
fn tier_row(ui: &mut Ui, row: &TierRow, i18n: &Translator) {
    let word = tier_word(row, i18n);
    if !row.is_tier_in_word {
        tier_label(ui, &row.tier, row.better);
    }
    if let Some(word) = word {
        let response = small_word(ui, word, color::WEAK);
        if let Some(tip) = &row.tip {
            let text = match tip {
                WordTip::Text(message) => i18n.text(*message),
                WordTip::HybridLines(lines) => i18n.hybrid_with(lines),
                WordTip::HybridPart { ranges, line } => {
                    i18n.hybrid_part(&ranges_text(ranges), line)
                }
            };
            response.on_hover_ui(|ui| {
                ui.set_max_width(WORD_TIP_WIDTH);
                ui.add(Label::new(text).wrap());
            });
        }
    }
    if let Some(currency) = row.currency {
        origin_word(ui, Origin::Currency(currency), i18n);
    }
}

/// One row of a roll, its parts `COLUMN_GAP` apart.
fn roll_row(ui: &mut Ui, parts: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = COLUMN_GAP;
        parts(ui);
    });
}

/// A tier, `T1` in its color, and its ranges (the ranges alone for a modifier of one tier only);
/// or two neighbouring tiers, the `better` and `tier`, `T1/T2` in the color of the worse, and
/// the ranges of both.
fn tier_label(ui: &mut Ui, tier: &ModifierTier, better: Option<u32>) {
    if let Some(number) = tier.number {
        let label = match better {
            Some(better) => format!("T{better}/T{number}"),
            None => format!("T{number}"),
        };
        ui.label(
            RichText::new(label)
                .font(text::tier())
                .color(tier_color(number)),
        );
    }
    if !tier.ranges.is_empty() {
        range_label(ui, &tier.ranges);
    }
}

/// The word of an origin, small, in its color: the last item of the line of a roll.
fn origin_word(ui: &mut Ui, origin: Origin, i18n: &Translator) {
    let (message, tint) = origin_label(origin);
    small_word(ui, i18n.text(message), tint);
}

/// A small word in `tint`: an origin, after the ranges of a tier or on a row of its own, or the
/// affix of a tier among several ("Prefix") in gray, after its ranges; `ORIGIN_GAP` after what
/// comes before it.
fn small_word(ui: &mut Ui, word: String, tint: Color32) -> Response {
    space_before(ui, ORIGIN_GAP);
    ui.label(RichText::new(word).font(text::origin()).color(tint))
}

/// The tag of the passive `skills` a line of a Time-Lost jewel is granted to, small, gray on a
/// block: "Small", `COLUMN_GAP` after what comes before it.
fn radius_tag(ui: &mut Ui, skills: RadiusSkills, i18n: &Translator) {
    space_before(ui, COLUMN_GAP);
    let galley = widgets::text_line(ui, i18n.text(radius_word(skills)), text::origin());
    let (rect, _) =
        ui.allocate_exact_size(galley.size() + 2.0 * RADIUS_TAG_PADDING, Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(size::RADIUS), color::CONTROL);
    painter.galley(rect.min + RADIUS_TAG_PADDING, galley, color::WEAK);
}

/// Makes the space between what the row of `ui` already holds and the next item `gap`: egui puts
/// the spacing of the row after an item, so the item before set it. Nothing on an empty row.
fn space_before(ui: &mut Ui, gap: f32) {
    if ui.min_rect().width() > 0.0 {
        ui.add_space(gap - ui.spacing().item_spacing.x);
    }
}

/// The word of an origin, and its color: that of the crafted modifiers for a currency, as the
/// trade site files them.
fn origin_label(origin: Origin) -> (Message, Color32) {
    match origin {
        Origin::Written(ModifierOrigin::Desecrated) => {
            (Message::OriginDesecrated, color::DESECRATED)
        }
        Origin::Written(ModifierOrigin::Fractured) => (Message::OriginFractured, color::FRACTURED),
        Origin::Written(ModifierOrigin::Crafted) => (Message::OriginCrafted, color::CRAFTED),
        Origin::Written(ModifierOrigin::Vaal) => (Message::OriginVaal, color::VAAL),
        Origin::Currency(Currency::Essence) => (Message::OriginEssence, color::CRAFTED),
        Origin::Currency(Currency::Alloy) => (Message::OriginAlloy, color::CRAFTED),
    }
}

/// The bar of a roll, filled in `tint` up to where the roll falls in its range, `share`, also on
/// a line better lower, so that it reads with the range written next to it.
fn roll_bar(ui: &mut Ui, share: f32, tint: Color32) {
    let (rect, _) = ui.allocate_exact_size(BAR_SIZE, Sense::hover());
    let radius = BAR_SIZE.y / 2.0;
    let painter = ui.painter();
    painter.rect_filled(rect, radius, color::CONTROL_LINE);
    let fill = Rect::from_min_size(
        rect.min,
        vec2(rect.width() * share.clamp(0.0, 1.0), rect.height()),
    );
    painter.rect_filled(fill, radius, tint);
}

/// The ranges of a roll, in faint text.
fn range_label(ui: &mut Ui, ranges: &[Range]) {
    let galley = widgets::text_line(ui, ranges_text(ranges), text::roll());
    let (rect, _) = ui.allocate_exact_size(galley.size(), Sense::hover());
    ui.painter().galley(rect.min, galley, color::FAINT);
}

#[cfg(test)]
mod tests;
