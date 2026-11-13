//! Small drawing blocks shared by the components.
//!
//! The lines of text are laid out without a color: the caller gives it when drawing them
//! (`painter.galley(position, galley, color)`), in one place.

use std::f32::consts::FRAC_PI_2;
use std::sync::Arc;

use egui::text::{LayoutJob, TextFormat, TextWrapping};
use egui::{
    Color32, CornerRadius, CursorIcon, FontId, Galley, Painter, Pos2, Rect, Response, Role, Sense,
    Stroke, StrokeKind, Ui, Vec2, WidgetInfo, pos2, vec2,
};

use super::theme::{color, size, text};
use crate::i18n::CutSentence;

/// The mark in front of a choice: a radio button among exclusive choices, a check box for a
/// choice of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    Radio { selected: bool },
    Check { checked: bool },
}

/// The measures of a choice.
mod choice {
    /// The square the mark is drawn in, the same for both marks so that the texts line up.
    pub const MARK_SIZE: f32 = 15.0;
    pub const MARK_GAP: f32 = 8.0;
    pub const RADIO_RADIUS: f32 = 5.5;
    pub const RADIO_STROKE: f32 = 2.0;
    pub const RADIO_DOT_RADIUS: f32 = 2.5;
    pub const CHECK_SIZE: f32 = MARK_SIZE;
    pub const CHECK_STROKE: f32 = 1.5;
    pub const CHECK_MARK_WIDTH: f32 = 9.0;
    /// The "not allowed" sign, drawn on the 16 px grid of its icon in the mock-up.
    pub const SIGN_GRID: f32 = 16.0;
    pub const SIGN_RADIUS: f32 = 5.8;
    pub const SIGN_STROKE: f32 = 2.2;
    /// How far the bar of the sign reaches from its center, across and down.
    pub const SIGN_BAR: f32 = 4.1;
}

impl Mark {
    fn is_on(self) -> bool {
        match self {
            Self::Radio { selected } => selected,
            Self::Check { checked } => checked,
        }
    }

    fn paint(self, painter: &Painter, center: Pos2, hovered: bool) {
        use choice::{
            CHECK_MARK_WIDTH, CHECK_SIZE, CHECK_STROKE, RADIO_DOT_RADIUS, RADIO_RADIUS,
            RADIO_STROKE,
        };

        match self {
            Self::Radio { selected } => {
                let line = if selected {
                    color::ACCENT
                } else if hovered {
                    color::WEAK
                } else {
                    color::FAINT
                };
                painter.circle_stroke(center, RADIO_RADIUS, Stroke::new(RADIO_STROKE, line));
                if selected {
                    painter.circle_filled(center, RADIO_DOT_RADIUS, color::ACCENT);
                }
            }
            Self::Check { checked } => {
                let rect = Rect::from_center_size(center, Vec2::splat(CHECK_SIZE));
                let radius = CornerRadius::same(size::RADIUS);
                if checked {
                    painter.rect_filled(rect, radius, color::ACCENT);
                    check_mark(painter, center, CHECK_MARK_WIDTH, color::ON_ACCENT);
                } else {
                    let line = if hovered { color::TEXT } else { color::WEAK };
                    painter.rect(
                        rect,
                        radius,
                        color::CONTROL,
                        Stroke::new(CHECK_STROKE, line),
                        StrokeKind::Inside,
                    );
                }
            }
        }
    }
}

/// Where the text of a choice starts, after its mark.
pub const CHOICE_TEXT_LEFT: f32 = choice::MARK_SIZE + choice::MARK_GAP;

/// A choice that can be clicked: its mark, then its text.
pub fn choice(ui: &mut Ui, mark: Mark, label: &str) -> Response {
    let galley = text_line(ui, label, text::body());
    choice_galley(ui, mark, galley, label)
}

/// A choice whose text, `galley`, has parts of several styles (a name in bold); `label`, its
/// text, for the screen readers.
pub fn choice_galley(ui: &mut Ui, mark: Mark, galley: Arc<Galley>, label: &str) -> Response {
    use choice::{MARK_GAP, MARK_SIZE};

    let size = vec2(
        MARK_SIZE + MARK_GAP + galley.size().x,
        MARK_SIZE.max(galley.size().y),
    );
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let role = match mark {
        Mark::Radio { .. } => Role::RadioButton,
        Mark::Check { .. } => Role::CheckBox,
    };
    response.widget_info(|| WidgetInfo::selected(role, true, mark.is_on(), label));

    let painter = ui.painter();
    mark.paint(
        painter,
        pos2(rect.left() + MARK_SIZE / 2.0, rect.center().y),
        response.hovered() || response.has_focus(),
    );
    let text_top = rect.center().y - galley.size().y / 2.0;
    painter.galley(
        pos2(rect.left() + MARK_SIZE + MARK_GAP, text_top),
        galley,
        color::TEXT,
    );
    response.on_hover_cursor(CursorIcon::PointingHand)
}

/// A checkbox without text, its mark at the left of a zone of `size`, centered across its
/// height; `label` tells what it checks.
pub fn check_box(ui: &mut Ui, checked: bool, size: Vec2, label: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    response.widget_info(|| WidgetInfo::selected(Role::CheckBox, true, checked, label));
    Mark::Check { checked }.paint(
        ui.painter(),
        pos2(rect.left() + choice::MARK_SIZE / 2.0, rect.center().y),
        response.hovered() || response.has_focus(),
    );
    response.on_hover_cursor(CursorIcon::PointingHand)
}

/// The "not allowed" sign, a circle struck through in the warning color, in place of a
/// checkbox: at the left of a zone of `size`, centered across its height.
pub fn not_allowed_sign(ui: &mut Ui, size: Vec2) -> Response {
    use choice::{MARK_SIZE, SIGN_BAR, SIGN_GRID, SIGN_RADIUS, SIGN_STROKE};

    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    let center = pos2(rect.left() + MARK_SIZE / 2.0, rect.center().y);
    let scale = MARK_SIZE / SIGN_GRID;
    let stroke = Stroke::new(SIGN_STROKE * scale, color::WARNING);
    let painter = ui.painter();
    painter.circle_stroke(center, SIGN_RADIUS * scale, stroke);
    let bar = SIGN_BAR * scale;
    painter.line_segment([center + vec2(-bar, bar), center + vec2(bar, -bar)], stroke);
    response
}

/// What lies under a floating list shows through it a little: the item under the leagues, the
/// settings under the keys.
pub const FLOATING_LIST_OPACITY: f32 = 0.94;

/// A list floating over the window, in the foreground: the leagues, the keys of the hotkey.
pub struct FloatingList {
    pub id: egui::Id,
    pub top_left: Pos2,
    pub size: Vec2,
    /// How much of it shows, from 0 (closed) to 1 (open), as it opens and closes.
    pub shown: f32,
    /// Whether it is open, not closing: only then do its rows take the pointer, and a click
    /// outside closes it.
    pub is_open: bool,
}

impl FloatingList {
    /// Draws the list, its shadow then its rows, which `add` draws in its rect; what the player
    /// did: what `add` gives, else `close` at a click outside the open list.
    pub fn show<A>(
        self,
        ctx: &egui::Context,
        close: A,
        add: impl FnOnce(&mut Ui, Rect) -> Option<A>,
    ) -> Option<A> {
        egui::Area::new(self.id)
            .order(egui::Order::Foreground)
            .fixed_pos(self.top_left)
            .interactable(self.is_open)
            .show(ctx, |ui| {
                ui.multiply_opacity(self.shown);
                let list = Rect::from_min_size(ui.next_widget_position(), self.size);
                ui.painter().add(
                    super::theme::floating_shadow()
                        .as_shape(list, CornerRadius::same(size::RADIUS)),
                );
                let action = add(ui, list);
                ui.allocate_rect(list, Sense::hover());
                action.or_else(|| (self.is_open && clicked_outside(ui, list)).then_some(close))
            })
            .inner
    }
}

/// Whether a click of this frame fell outside `rect`.
fn clicked_outside(ui: &Ui, rect: Rect) -> bool {
    ui.input(|input| {
        input.pointer.any_click()
            && input
                .pointer
                .interact_pos()
                .is_some_and(|pointer| !rect.contains(pointer))
    })
}

/// A line of text.
#[must_use]
pub fn text_line(ui: &Ui, text: impl Into<String>, font: FontId) -> Arc<Galley> {
    line(
        ui,
        text,
        TextFormat::simple(font, Color32::PLACEHOLDER),
        None,
    )
}

/// A line of text cut with "…" when it is wider than `max_width`.
#[must_use]
pub fn truncated_line(
    ui: &Ui,
    text: impl Into<String>,
    font: FontId,
    max_width: f32,
) -> Arc<Galley> {
    line(
        ui,
        text,
        TextFormat::simple(font, Color32::PLACEHOLDER),
        Some(max_width),
    )
}

/// A text wrapped at `max_width`.
#[must_use]
pub fn wrapped_line(ui: &Ui, text: &str, font: FontId, max_width: f32) -> Arc<Galley> {
    ui.painter()
        .layout(text.to_owned(), font, Color32::PLACEHOLDER, max_width)
}

/// `text` wrapped at the width of `ui`, on a row across it, in `tint`.
pub fn wrapped_text(ui: &mut Ui, text: &str, font: FontId, tint: Color32) {
    let galley = wrapped_line(ui, text, font, ui.available_width());
    text_row(ui, galley, 0.0, tint);
}

/// A row across the width of `ui`, as tall as `galley`, which it draws `inset` from its left
/// in `tint`.
pub fn text_row(ui: &mut Ui, galley: Arc<Galley>, inset: f32, tint: Color32) {
    let (rect, _) =
        ui.allocate_exact_size(vec2(ui.available_width(), galley.size().y), Sense::hover());
    ui.painter()
        .galley(pos2(rect.left() + inset, rect.top()), galley, tint);
}

/// A line of text with extra space between its letters, for the small uppercase labels.
#[must_use]
pub fn spaced_line(
    ui: &Ui,
    text: impl Into<String>,
    font: FontId,
    letter_spacing: f32,
) -> Arc<Galley> {
    let format = TextFormat {
        extra_letter_spacing: letter_spacing,
        ..TextFormat::simple(font, Color32::PLACEHOLDER)
    };
    line(ui, text, format, None)
}

/// A small title, `title` in capitals spaced in the font of the labels: "PRICE", "OFFERS".
#[must_use]
pub fn small_title_line(ui: &Ui, title: &str) -> Arc<Galley> {
    spaced_line(
        ui,
        title.to_uppercase(),
        text::label(),
        text::LABEL_LETTER_SPACING,
    )
}

/// A small title where the layout places it, in `tint`.
pub fn small_title(ui: &mut Ui, title: &str, tint: Color32) {
    let galley = small_title_line(ui, title);
    let (rect, _) = ui.allocate_exact_size(galley.size(), Sense::hover());
    ui.painter().galley(rect.min, galley, tint);
}

fn line(
    ui: &Ui,
    text: impl Into<String>,
    format: TextFormat,
    max_width: Option<f32>,
) -> Arc<Galley> {
    let mut job = LayoutJob::single_section(text.into(), format);
    job.wrap = max_width.map_or_else(TextWrapping::no_max_width, TextWrapping::truncate_at_width);
    ui.painter().layout_job(job)
}

/// `sentence` with `argument` in its place, in the format `apart`, the rest in `regular`: the
/// name of a league in bold in its bubble.
#[must_use]
pub fn sentence_with(
    sentence: &CutSentence,
    argument: &str,
    regular: &TextFormat,
    apart: TextFormat,
) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.append(&sentence.before, 0.0, regular.clone());
    job.append(argument, 0.0, apart);
    job.append(&sentence.after, 0.0, regular.clone());
    job
}

/// A chevron centered on `center`, `width` across its opening: pointing right at `turn` 0,
/// down at 1, and turned in between, for its animation.
pub fn chevron(painter: &Painter, center: Pos2, width: f32, turn: f32, color: Color32) {
    let (sin, cos) = (turn * FRAC_PI_2).sin_cos();
    let rotate = |point: Vec2| vec2(point.x * cos - point.y * sin, point.x * sin + point.y * cos);
    let half_width = width / 2.0;
    let half_depth = width / 4.0;
    painter.line(
        [
            vec2(-half_depth, -half_width),
            vec2(half_depth, 0.0),
            vec2(-half_depth, half_width),
        ]
        .map(|point| center + rotate(point))
        .to_vec(),
        Stroke::new(size::ICON_STROKE, color),
    );
}

/// A check mark, centered on `center`.
pub fn check_mark(painter: &Painter, center: Pos2, width: f32, color: Color32) {
    // The "check" icon of Lucide, from its 24 point grid: 16 points across.
    let scale = width / 16.0;
    painter.line(
        vec![
            center + vec2(-8.0, 0.5) * scale,
            center + vec2(-3.0, 5.5) * scale,
            center + vec2(8.0, -5.5) * scale,
        ],
        Stroke::new(size::ICON_STROKE, color),
    );
}

/// A cross `width` across, centered on `center`: the close icons.
pub fn cross(painter: &Painter, center: Pos2, width: f32, color: Color32) {
    let half = width / 2.0;
    let stroke = Stroke::new(size::ICON_STROKE, color);
    painter.line_segment(
        [center - Vec2::splat(half), center + Vec2::splat(half)],
        stroke,
    );
    painter.line_segment(
        [center + vec2(-half, half), center + vec2(half, -half)],
        stroke,
    );
}

/// The size of the pill of `label`, to make room for it.
#[must_use]
pub fn pill_size(ui: &Ui, label: &str) -> Vec2 {
    text_line(ui, label, text::pill()).size() + 2.0 * PILL_PADDING
}

/// A pill on the accent, for the main action of a bubble.
pub fn pill(ui: &mut Ui, label: &str) -> Response {
    let galley = text_line(ui, label, text::pill());
    let (rect, response) =
        ui.allocate_exact_size(galley.size() + 2.0 * PILL_PADDING, Sense::click());
    response.widget_info(|| WidgetInfo::labeled(Role::Button, true, label));
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(size::RADIUS), color::ACCENT);
    painter.galley(rect.min + PILL_PADDING, galley, color::ON_ACCENT);
    response.on_hover_cursor(CursorIcon::PointingHand)
}

/// A pill on the color of the controls, for a secondary action: lighter, with the text of
/// the accent, under the pointer.
pub fn control_pill(ui: &mut Ui, label: &str) -> Response {
    tinted_control_pill(ui, label, color::TEXT, color::ACCENT)
}

/// A pill on the color of the controls whose action failed, in red: a click tries again.
pub fn failed_pill(ui: &mut Ui, label: &str) -> Response {
    tinted_control_pill(ui, label, color::FALL, color::FALL)
}

/// A pill on the color of the controls, lighter under the pointer, with the text in `tint`, or
/// in `lit_tint` under the pointer.
fn tinted_control_pill(ui: &mut Ui, label: &str, tint: Color32, lit_tint: Color32) -> Response {
    let galley = text_line(ui, label, text::pill());
    let (rect, response) =
        ui.allocate_exact_size(galley.size() + 2.0 * PILL_PADDING, Sense::click());
    response.widget_info(|| WidgetInfo::labeled(Role::Button, true, label));
    let lit = response.hovered() || response.has_focus();
    let (fill, tint) = if lit {
        (color::CONTROL_LINE, lit_tint)
    } else {
        (color::CONTROL, tint)
    };
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(size::RADIUS), fill);
    painter.galley(rect.min + PILL_PADDING, galley, tint);
    response.on_hover_cursor(CursorIcon::PointingHand)
}

/// The space around the text of a pill.
const PILL_PADDING: Vec2 = vec2(10.0, 4.0);

/// The check mark of a "Copied" pill, as in the mock-up.
mod copied {
    pub const CHECK_SIZE: f32 = 11.0;
    /// The space between the check mark and the word.
    pub const CHECK_GAP: f32 = 5.0;
}

/// The size of a "Copied" pill: its word and its check mark.
#[must_use]
pub fn copied_pill_size(ui: &Ui, label: &str) -> Vec2 {
    pill_size(ui, label) + vec2(copied::CHECK_SIZE + copied::CHECK_GAP, 0.0)
}

/// "Copied ✓" on the accent, like the main action: the whisper copied, which a click copies
/// again.
pub fn copied_pill(ui: &mut Ui, label: &str) -> Response {
    use copied::{CHECK_GAP, CHECK_SIZE};

    let galley = text_line(ui, label, text::pill());
    let (rect, response) = ui.allocate_exact_size(copied_pill_size(ui, label), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(Role::Button, true, label));
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(size::RADIUS), color::ACCENT);
    let check_left = rect.left() + PILL_PADDING.x + galley.size().x + CHECK_GAP;
    painter.galley(rect.min + PILL_PADDING, galley, color::ON_ACCENT);
    check_mark(
        painter,
        pos2(check_left + CHECK_SIZE / 2.0, rect.center().y),
        CHECK_SIZE,
        color::ON_ACCENT,
    );
    response.on_hover_cursor(CursorIcon::PointingHand)
}

/// "✓ Copied" on the color of the controls, in the weak color, in place of the pill of a
/// secondary action: done, nothing to click.
pub fn copied_control_pill(ui: &mut Ui, label: &str) {
    use copied::{CHECK_GAP, CHECK_SIZE};

    let galley = text_line(ui, label, text::pill());
    let (rect, response) = ui.allocate_exact_size(copied_pill_size(ui, label), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, label));
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(size::RADIUS), color::CONTROL);
    let check_left = rect.left() + PILL_PADDING.x;
    check_mark(
        painter,
        pos2(check_left + CHECK_SIZE / 2.0, rect.center().y),
        CHECK_SIZE,
        color::WEAK,
    );
    painter.galley(
        pos2(
            check_left + CHECK_SIZE + CHECK_GAP,
            rect.top() + PILL_PADDING.y,
        ),
        galley,
        color::WEAK,
    );
}

/// The measures of two pills joined.
mod segmented {
    use egui::{Vec2, vec2};

    /// The space between the dark of the controls and the pills.
    pub const INSET: f32 = 2.0;
    /// The space around the text of a pill.
    pub const PADDING: Vec2 = vec2(9.0, 3.0);
}

/// The slide of the white pill of two pills joined, kept from one frame to the next.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Slide {
    /// Where it started: 0 under the first pill, 1 under the second, or between.
    from: f32,
    /// Whether it goes to the second pill.
    to_second: bool,
    /// When it started, in seconds of `egui::InputState::time`; minus infinity for a pill placed
    /// at once.
    start: f64,
}

impl Slide {
    /// The white pill placed at once under the second pill, or the first.
    fn placed(second: bool) -> Self {
        Self {
            from: f32::from(u8::from(second)),
            to_second: second,
            start: f64::NEG_INFINITY,
        }
    }

    /// The slide toward the second pill, or the first, at `now`: the same one when it already
    /// goes there, else a new one from where the pill stands.
    fn toward(self, to_second: bool, now: f64) -> Self {
        if self.to_second == to_second {
            self
        } else {
            Self {
                from: self.at(now),
                to_second,
                start: now,
            }
        }
    }

    /// Where the white pill stands at `now`.
    fn at(self, now: f64) -> f32 {
        // Any duration, infinite for a pill placed at once: clamped after, so no precision
        // lost that matters.
        #[expect(clippy::cast_possible_truncation)]
        let elapsed = (now - self.start) as f32;
        let to = f32::from(u8::from(self.to_second));
        slide_at(self.from, to, elapsed / super::theme::motion::SLIDE)
    }

    /// Whether it is still on its way at `now`, told by the time: never for a pill placed at
    /// once.
    fn sliding(self, now: f64) -> bool {
        now - self.start < f64::from(super::theme::motion::SLIDE)
    }
}

/// Where a slide from `from` to `to` stands after `progress` of its time (0 to 1, clamped), slowing
/// down at its end.
fn slide_at(from: f32, to: f32, progress: f32) -> f32 {
    from + (to - from) * super::theme::motion::slowing(progress.clamp(0.0, 1.0))
}

/// Two pills joined on the dark of the controls, `labels`, the one of `chosen` (0 or 1) white
/// like the main action; at a click, the white pill slides to the other one (`motion::SLIDE`)
/// from where it stands, even in the middle of a slide, and is placed at once the first time
/// `id` is drawn. `spoken` is what a screen reader says of each pill ("Log: This session"). The
/// pill clicked, if another one.
pub fn segmented(
    ui: &mut Ui,
    id: egui::Id,
    labels: [&str; 2],
    spoken: &[String; 2],
    chosen: usize,
) -> Option<usize> {
    use segmented::{INSET, PADDING};
    let galleys = labels.map(|label| text_line(ui, label, text::small_strong()));
    let sizes = galleys
        .each_ref()
        .map(|galley| galley.size() + 2.0 * PADDING);
    let height = sizes[0].y.max(sizes[1].y);
    let (track, _) = ui.allocate_exact_size(
        vec2(sizes[0].x + sizes[1].x + 2.0 * INSET, height + 2.0 * INSET),
        Sense::hover(),
    );
    let pills = [
        Rect::from_min_size(track.min + vec2(INSET, INSET), vec2(sizes[0].x, height)),
        Rect::from_min_size(
            track.min + vec2(INSET + sizes[0].x, INSET),
            vec2(sizes[1].x, height),
        ),
    ];
    let responses = [0, 1].map(|index| {
        let response = ui.interact(pills[index], id.with(index), Sense::click());
        response.widget_info(|| {
            WidgetInfo::selected(Role::RadioButton, true, index == chosen, &spoken[index])
        });
        response
    });

    let to_second = chosen == 1;
    let now = ui.input(|input| input.time);
    let slide = ui.data_mut(|data| {
        let slide = data.get_temp_mut_or_insert_with(id, || Slide::placed(to_second));
        *slide = slide.toward(to_second, now);
        *slide
    });
    if slide.sliding(now) {
        ui.ctx().request_repaint();
    }
    // 0 under the first pill, 1 under the second.
    let t = slide.at(now);

    let painter = ui.painter();
    let radius = CornerRadius::same(size::RADIUS);
    painter.rect_filled(track, radius, color::CONTROL);
    let white = Rect::from_min_max(
        pills[0].min.lerp(pills[1].min, t),
        pills[0].max.lerp(pills[1].max, t),
    );
    // Dimmed a little when the pill chosen has the focus of the keyboard.
    let fill = if responses[chosen.min(1)].has_focus() {
        color::ACCENT_DIMMED
    } else {
        color::ACCENT
    };
    painter.rect_filled(white, radius, fill);

    let mut clicked = None;
    for (index, (galley, response)) in galleys.into_iter().zip(responses).enumerate() {
        // How much of the white pill is under this one: its text dark on it, and fading back.
        let under = if index == 1 { t } else { 1.0 - t };
        let rest = if response.hovered() || response.has_focus() {
            color::TEXT
        } else {
            color::WEAK
        };
        let tint = rest.lerp_to_gamma(color::ON_ACCENT, under);
        ui.painter()
            .galley(pills[index].min + PADDING, galley, tint);
        if index != chosen && response.on_hover_cursor(CursorIcon::PointingHand).clicked() {
            clicked = Some(index);
        }
    }
    clicked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_argument_is_set_apart_at_its_place_even_when_the_sentence_writes_it_before() {
        let sentence = CutSentence {
            before: "Join a: ".to_owned(),
            after: ".".to_owned(),
        };
        let regular = TextFormat::simple(text::bubble(), color::TEXT);
        let apart = TextFormat::simple(text::bubble_strong(), color::TEXT);

        let job = sentence_with(&sentence, "a", &regular, apart.clone());

        let sections: Vec<(&str, bool)> = job
            .sections
            .iter()
            .map(|section| {
                let range = section.byte_range.start.0..section.byte_range.end.0;
                (&job.text[range], section.format == apart)
            })
            .collect();
        assert_eq!(sections, [("Join a: ", false), ("a", true), (".", false)]);
    }

    #[test]
    fn the_white_pill_is_placed_at_once_the_first_time() {
        let placed = Slide::placed(true);

        assert!((placed.at(0.0) - 1.0).abs() < f32::EPSILON);
        assert!(!placed.sliding(0.0));
    }

    #[test]
    fn the_same_target_keeps_the_slide() {
        let going = Slide::placed(false).toward(true, 10.0);

        assert_eq!(going.toward(true, 10.1), going, "a frame drawn again");
        assert!(going.sliding(10.1));
        assert!(!going.sliding(10.3), "over after its time");
    }

    #[test]
    fn a_slide_turned_back_starts_from_where_the_pill_stands() {
        let going = Slide::placed(false).toward(true, 10.0);
        let midway = going.at(10.066);
        let back = going.toward(false, 10.066);

        assert!(midway > 0.5 && midway < 1.0, "{midway}");
        assert!((back.at(10.066) - midway).abs() < f32::EPSILON, "no jump");
        assert!(back.at(10.1) < midway, "on its way back");
        assert!(back.at(11.0).abs() < f32::EPSILON, "arrived");
    }

    #[test]
    fn a_slide_ends_on_its_target() {
        assert!((slide_at(0.3, 1.0, 1.0) - 1.0).abs() < f32::EPSILON);
        assert!(
            (slide_at(0.3, 1.0, 7.0) - 1.0).abs() < f32::EPSILON,
            "past its time"
        );
        assert!((slide_at(0.3, 1.0, 0.0) - 0.3).abs() < f32::EPSILON);
    }
}
