//! The edges of the window, which resize it: without the frame of the system, the window draws
//! them itself, above everything else, so that the blocks along the edges don't take the drag.

use egui::viewport::ResizeDirection;
use egui::{Area, Context, CursorIcon, Id, Order, Pos2, Rect, Sense, pos2, vec2};

/// The thickness of an edge, inside the window.
const EDGE: f32 = 4.0;
/// The length of a corner along each side, which resizes both ways.
const CORNER: f32 = 12.0;

pub struct ResizeEdges;

impl ResizeEdges {
    /// Draws the edges of the window; the direction the player started to drag, if any.
    pub fn show(ctx: &Context) -> Option<ResizeDirection> {
        let window = ctx.content_rect();
        let mut started = None;
        for (index, strip) in strips(window).into_iter().enumerate() {
            Area::new(Id::NULL.with(("resize-edge", index)))
                .order(Order::Foreground)
                .fixed_pos(strip.min)
                .sense(Sense::hover())
                .show(ctx, |ui| {
                    let (_, response) = ui.allocate_exact_size(strip.size(), Sense::drag());
                    let Some(direction) = response
                        .hover_pos()
                        .or(response.interact_pointer_pos())
                        .and_then(|pointer| direction_at(window, pointer))
                    else {
                        return;
                    };
                    if response.drag_started() {
                        started = Some(direction);
                    }
                    response.on_hover_cursor(cursor(direction));
                });
        }
        started
    }
}

/// The four edges of `window`, along its whole sides.
fn strips(window: Rect) -> [Rect; 4] {
    [
        Rect::from_min_size(window.min, vec2(window.width(), EDGE)),
        Rect::from_min_size(
            pos2(window.left(), window.bottom() - EDGE),
            vec2(window.width(), EDGE),
        ),
        Rect::from_min_size(window.min, vec2(EDGE, window.height())),
        Rect::from_min_size(
            pos2(window.right() - EDGE, window.top()),
            vec2(EDGE, window.height()),
        ),
    ]
}

/// The direction an edge resizes `window` from `pointer`: both ways near a corner; `None`
/// away from the edges.
fn direction_at(window: Rect, pointer: Pos2) -> Option<ResizeDirection> {
    let near = |distance: f32, reach: f32| (0.0..reach).contains(&distance);
    let (top, bottom) = (pointer.y - window.top(), window.bottom() - pointer.y);
    let (left, right) = (pointer.x - window.left(), window.right() - pointer.x);
    let on_edge = near(top, EDGE) || near(bottom, EDGE) || near(left, EDGE) || near(right, EDGE);
    if !on_edge {
        return None;
    }
    let north = near(top, CORNER);
    let south = near(bottom, CORNER);
    let west = near(left, CORNER);
    let east = near(right, CORNER);
    Some(match (north, south, west, east) {
        (true, _, true, _) => ResizeDirection::NorthWest,
        (true, _, _, true) => ResizeDirection::NorthEast,
        (_, true, true, _) => ResizeDirection::SouthWest,
        (_, true, _, true) => ResizeDirection::SouthEast,
        _ if near(top, EDGE) => ResizeDirection::North,
        _ if near(bottom, EDGE) => ResizeDirection::South,
        _ if near(left, EDGE) => ResizeDirection::West,
        _ => ResizeDirection::East,
    })
}

fn cursor(direction: ResizeDirection) -> CursorIcon {
    match direction {
        ResizeDirection::North | ResizeDirection::South => CursorIcon::ResizeVertical,
        ResizeDirection::East | ResizeDirection::West => CursorIcon::ResizeHorizontal,
        ResizeDirection::NorthEast | ResizeDirection::SouthWest => CursorIcon::ResizeNeSw,
        ResizeDirection::NorthWest | ResizeDirection::SouthEast => CursorIcon::ResizeNwSe,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WINDOW: Rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(420.0, 720.0));

    #[test]
    fn the_edges_resize_their_side() {
        assert_eq!(
            direction_at(WINDOW, pos2(200.0, 1.0)),
            Some(ResizeDirection::North)
        );
        assert_eq!(
            direction_at(WINDOW, pos2(200.0, 719.0)),
            Some(ResizeDirection::South)
        );
        assert_eq!(
            direction_at(WINDOW, pos2(1.0, 300.0)),
            Some(ResizeDirection::West)
        );
        assert_eq!(
            direction_at(WINDOW, pos2(419.0, 300.0)),
            Some(ResizeDirection::East)
        );
    }

    #[test]
    fn the_corners_resize_both_ways_along_both_sides() {
        // Along the top and along the right side, near the corner.
        assert_eq!(
            direction_at(WINDOW, pos2(410.0, 1.0)),
            Some(ResizeDirection::NorthEast)
        );
        assert_eq!(
            direction_at(WINDOW, pos2(419.0, 10.0)),
            Some(ResizeDirection::NorthEast)
        );
        assert_eq!(
            direction_at(WINDOW, pos2(2.0, 715.0)),
            Some(ResizeDirection::SouthWest)
        );
        assert_eq!(cursor(ResizeDirection::NorthEast), CursorIcon::ResizeNeSw);
    }

    #[test]
    fn inside_the_window_nothing_resizes() {
        assert_eq!(direction_at(WINDOW, pos2(200.0, 300.0)), None);
        // Near a corner, but away from the edges.
        assert_eq!(direction_at(WINDOW, pos2(410.0, 10.0)), None);
    }

    #[test]
    fn the_strips_cover_the_edges() {
        for strip in strips(WINDOW) {
            assert!(WINDOW.contains_rect(strip));
            assert!(strip.width().min(strip.height()) <= EDGE);
        }
    }
}
