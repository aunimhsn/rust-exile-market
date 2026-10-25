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

/// A choice that can be clicked: its mark, then its text.
pub fn choice(ui: &mut Ui, mark: Mark, label: &str) -> Response {
    use choice::{MARK_GAP, MARK_SIZE};

    let galley = text_line(ui, label, text::body());
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
    let galley = text_line(ui, label, text::pill());
    let (rect, response) =
        ui.allocate_exact_size(galley.size() + 2.0 * PILL_PADDING, Sense::click());
    response.widget_info(|| WidgetInfo::labeled(Role::Button, true, label));
    let lit = response.hovered() || response.has_focus();
    let (fill, tint) = if lit {
        (color::CONTROL_LINE, color::ACCENT)
    } else {
        (color::CONTROL, color::TEXT)
    };
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(size::RADIUS), fill);
    painter.galley(rect.min + PILL_PADDING, galley, tint);
    response.on_hover_cursor(CursorIcon::PointingHand)
}

/// The space around the text of a pill.
const PILL_PADDING: Vec2 = vec2(10.0, 4.0);
