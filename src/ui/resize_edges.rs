//! The edges of the window, which resize it: without the frame of the system, the window draws
//! them itself, above everything else, so that the blocks along the edges don't take the drag.

use egui::viewport::ResizeDirection;
use egui::{Area, Context, CursorIcon, Id, Order, PointerButton, Pos2, Rect, Sense, pos2, vec2};

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
        for (index, (strip, side)) in strips(window).into_iter().enumerate() {
            Area::new(Id::NULL.with(("resize-edge", index)))
                .order(Order::Foreground)
                .fixed_pos(strip.min)
                .sense(Sense::hover())
                .show(ctx, |ui| {
                    let (_, response) = ui.allocate_exact_size(strip.size(), Sense::drag());
                    // egui gives the pointer to a strip a few points around it too: the
                    // direction comes from the strip it chose, the pointer only finds the
                    // corners.
                    let Some(pointer) = response.hover_pos().or(response.interact_pointer_pos())
                    else {
                        return;
                    };
                    let direction = direction_at(window, side, pointer);
                    if response.drag_started_by(PointerButton::Primary) {
                        started = Some(direction);
                    }
                    response.on_hover_cursor(cursor(direction));
                });
        }
        started
    }
}

/// The four edges of `window`, along its whole sides, with the side each one resizes.
fn strips(window: Rect) -> [(Rect, ResizeDirection); 4] {
    [
        (
            Rect::from_min_size(window.min, vec2(window.width(), EDGE)),
            ResizeDirection::North,
        ),
        (
            Rect::from_min_size(
                pos2(window.left(), window.bottom() - EDGE),
                vec2(window.width(), EDGE),
            ),
            ResizeDirection::South,
        ),
        (
            Rect::from_min_size(window.min, vec2(EDGE, window.height())),
            ResizeDirection::West,
        ),
        (
            Rect::from_min_size(
                pos2(window.right() - EDGE, window.top()),
                vec2(EDGE, window.height()),
            ),
            ResizeDirection::East,
        ),
    ]
}

/// The direction the edge of `side` resizes `window` from `pointer`: both ways near a
/// corner.
fn direction_at(window: Rect, side: ResizeDirection, pointer: Pos2) -> ResizeDirection {
    use ResizeDirection::{East, North, NorthEast, NorthWest, South, SouthEast, SouthWest, West};

    let near_west = pointer.x - window.left() < CORNER;
    let near_east = window.right() - pointer.x < CORNER;
    let near_north = pointer.y - window.top() < CORNER;
    let near_south = window.bottom() - pointer.y < CORNER;
    match side {
        North if near_west => NorthWest,
        North if near_east => NorthEast,
        South if near_west => SouthWest,
        South if near_east => SouthEast,
        West if near_north => NorthWest,
        West if near_south => SouthWest,
        East if near_north => NorthEast,
        East if near_south => SouthEast,
        side => side,
    }
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
    use ResizeDirection::{East, North, NorthEast, NorthWest, South, SouthWest, West};

    const WINDOW: Rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(420.0, 720.0));

    #[test]
    fn the_edges_resize_their_side() {
        assert_eq!(direction_at(WINDOW, North, pos2(200.0, 1.0)), North);
        assert_eq!(direction_at(WINDOW, South, pos2(200.0, 719.0)), South);
        assert_eq!(direction_at(WINDOW, West, pos2(1.0, 300.0)), West);
        assert_eq!(direction_at(WINDOW, East, pos2(419.0, 300.0)), East);
    }

    #[test]
    fn a_pointer_a_little_away_from_its_edge_still_resizes_its_side() {
        // egui gives the strip the pointer a few points under it too.
        assert_eq!(direction_at(WINDOW, North, pos2(200.0, 6.0)), North);
    }

    #[test]
    fn the_corners_resize_both_ways_along_both_sides() {
        assert_eq!(direction_at(WINDOW, North, pos2(410.0, 1.0)), NorthEast);
        assert_eq!(direction_at(WINDOW, East, pos2(419.0, 10.0)), NorthEast);
        assert_eq!(direction_at(WINDOW, West, pos2(1.0, 5.0)), NorthWest);
        assert_eq!(direction_at(WINDOW, South, pos2(2.0, 715.0)), SouthWest);
        assert_eq!(cursor(NorthEast), CursorIcon::ResizeNeSw);
    }

    #[test]
    fn the_strips_lie_along_their_side() {
        for (strip, side) in strips(WINDOW) {
            assert!(WINDOW.contains_rect(strip), "{side:?}");
            assert!(strip.width().min(strip.height()) <= EDGE, "{side:?}");
            assert_eq!(direction_at(WINDOW, side, strip.center()), side);
        }
    }
}
