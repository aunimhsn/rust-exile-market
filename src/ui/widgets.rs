//! Small drawing blocks shared by the components.
//!
//! The lines of text are laid out without a color: the caller gives it when drawing them
//! (`painter.galley(position, galley, color)`), in one place.

use std::sync::Arc;

use egui::text::{LayoutJob, TextFormat, TextWrapping};
use egui::{
    Color32, CornerRadius, CursorIcon, FontId, Galley, Painter, Pos2, Response, Sense, Stroke, Ui,
    Vec2, vec2,
};

use super::theme::{self, size};

/// A flat, square block the given size, that can be clicked: its background is one shade
/// lighter under the pointer. The caller draws the content inside `response.rect`.
pub fn flat_block(ui: &mut Ui, size: Vec2, background: Color32) -> Response {
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let fill = if response.hovered() {
        theme::hovered(background)
    } else {
        background
    };
    ui.painter().rect_filled(rect, CornerRadius::ZERO, fill);
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

/// A chevron pointing down, centered on `center`.
pub fn chevron_down(painter: &Painter, center: Pos2, width: f32, color: Color32) {
    let half_width = width / 2.0;
    let half_height = width / 4.0;
    painter.line(
        vec![
            center + vec2(-half_width, -half_height),
            center + vec2(0.0, half_height),
            center + vec2(half_width, -half_height),
        ],
        Stroke::new(size::ICON_STROKE, color),
    );
}
