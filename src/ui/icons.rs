//! The icons drawn by hand: the chevrons, the check mark, the cross, the magnifying glass,
//! the refresh arrow and the spinner.

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use egui::{Color32, Painter, Pos2, Rect, Stroke, Vec2, vec2};

use super::theme::{color, size};

/// The width of the strokes of the icons, on their grid of 16.
const GRID_STROKE: f32 = 1.8;
/// The spinner of the search.
const SPINNER_RADIUS: f32 = 6.0;
const SPINNER_STROKE: f32 = 2.0;
/// Turns per second.
const SPINNER_SPEED: f32 = 1.2;

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

/// A point of an icon drawn on a grid of 16 that fills `square`.
fn grid(square: Rect) -> impl Fn(f32, f32) -> Pos2 {
    let (center, scale) = (square.center(), square.width() / 16.0);
    move |x, y| center + vec2(x - 8.0, y - 8.0) * scale
}

/// The stroke of an icon drawn on a grid of 16 that fills `square`, thicker as it grows.
fn grid_stroke(square: Rect, tint: Color32) -> Stroke {
    Stroke::new(GRID_STROKE * square.width() / 16.0, tint)
}

/// The magnifying glass of the search block, filling `square`.
pub fn search(painter: &Painter, square: Rect, tint: Color32) {
    let point = grid(square);
    let stroke = grid_stroke(square, tint);
    painter.circle_stroke(point(7.0, 7.0), 4.6 * square.width() / 16.0, stroke);
    painter.line_segment([point(10.4, 10.4), point(14.0, 14.0)], stroke);
}

/// The refresh arrow, filling `square`, turned by `turn` quarters.
pub fn refresh(painter: &Painter, square: Rect, turn: f32, tint: Color32) {
    let point = grid(square);
    let center = square.center();
    let (sin, cos) = (turn * PI / 2.0).sin_cos();
    let rotate = |p: Pos2| {
        center + {
            let v = p - center;
            vec2(v.x * cos - v.y * sin, v.x * sin + v.y * cos)
        }
    };
    let stroke = grid_stroke(square, tint);
    // Three quarters and a half of a circle, from its right, clockwise.
    let arc: Vec<Pos2> = (0..=28)
        .map(|step| {
            #[expect(clippy::cast_precision_loss, reason = "a small step")]
            let angle = (step as f32 / 28.0) * TAU * 7.0 / 8.0;
            rotate(point(8.0 + 5.5 * angle.cos(), 8.0 + 5.5 * angle.sin()))
        })
        .collect();
    painter.line(arc, stroke);
    painter.line(
        vec![
            rotate(point(13.6, 2.6)),
            rotate(point(13.6, 5.9)),
            rotate(point(10.3, 5.9)),
        ],
        stroke,
    );
}

/// The double chevron pointing down, filling `square`.
pub fn double_chevron(painter: &Painter, square: Rect, tint: Color32) {
    let point = grid(square);
    let stroke = grid_stroke(square, tint);
    for top in [3.5, 8.5] {
        painter.line(
            vec![point(4.0, top), point(8.0, top + 4.0), point(12.0, top)],
            stroke,
        );
    }
}

/// A ring with a turning quarter, at `time` seconds.
pub fn spinner(painter: &Painter, center: Pos2, time: f64) {
    painter.circle_stroke(
        center,
        SPINNER_RADIUS,
        Stroke::new(SPINNER_STROKE, color::CONTROL_LINE),
    );
    #[expect(clippy::cast_possible_truncation, reason = "a fraction of a turn")]
    let start = ((time * f64::from(SPINNER_SPEED)).fract() as f32) * TAU;
    let arc: Vec<Pos2> = (0..=8)
        .map(|step| {
            #[expect(clippy::cast_precision_loss, reason = "a small step")]
            let angle = start + (step as f32 / 8.0) * TAU / 4.0;
            center + vec2(angle.cos(), angle.sin()) * SPINNER_RADIUS
        })
        .collect();
    painter.line(arc, Stroke::new(SPINNER_STROKE, color::ACCENT));
}
