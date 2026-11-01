//! The window over the game: the price beside the panel of the game the item is in, opened by
//! the price-check hotkey and closed by a click in the game (`specs.md`, Window over the game).
//!
//! This module decides where the window goes, and when it opens and closes, without the system:
//! the pointer and the game window come with the hotkey ([`crate::hotkey`]), the clicks from a
//! thread of each system ([`ClickWatch`]), and the application draws the window.

#[cfg(target_os = "linux")]
mod x11;

#[cfg(target_os = "linux")]
pub use self::x11::ClickWatch;

use std::time::{Duration, Instant};

use crate::hotkey::COPY_WAIT;

/// How long a hotkey waits for its item before it is forgotten: the clipboard watching takes an
/// item for its copy within [`COPY_WAIT`], then the item reaches the window at the next frame, up
/// to 200 ms later while the main window is minimized. Only for the log: the item of a hotkey
/// is known by the instant it carries.
const FORGOTTEN_AFTER: Duration = COPY_WAIT.saturating_add(Duration::from_secs(1));

/// The width of the window, in points: the one of the narrow layout.
pub const WIDTH: f32 = 420.0;
/// The width of a panel of the game (inventory, stash) for its height: 370 for 600, the ratio
/// Exiled Exchange 2 uses (`uiSidebarWidth` in `main/src/windowing/GameWindow.ts`).
const PANEL_WIDTH_PER_HEIGHT: (i32, i32) = (370, 600);

/// A point of the screen, in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

/// A rectangle of the screen, in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl ScreenRect {
    fn right(&self) -> i32 {
        self.x + self.width
    }

    fn bottom(&self) -> i32 {
        self.y + self.height
    }

    #[must_use]
    pub fn contains(&self, point: Point) -> bool {
        (self.x..self.right()).contains(&point.x) && (self.y..self.bottom()).contains(&point.y)
    }
}

/// Where the hotkey was pressed: the pointer, and the window of the game, which has the focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target {
    pub pointer: Point,
    pub game: ScreenRect,
}

impl Target {
    /// The place of the window, `width` wide, with its content `content_height` high (physical
    /// pixels): beside the panel of the game the item is in, which it never covers (left of the
    /// inventory when the pointer is on the right half of the game, right of the stash
    /// otherwise), its top level with the pointer, moved up as much as the game needs; at most
    /// the height of the game, its content then scrolls.
    #[must_use]
    pub fn place(&self, width: i32, content_height: i32) -> ScreenRect {
        let game = self.game;
        let (panel_width, per_height) = PANEL_WIDTH_PER_HEIGHT;
        let panel = game.height * panel_width / per_height;
        let height = content_height.min(game.height).max(1);
        let x = if self.pointer.x - game.x > game.width / 2 {
            game.right() - panel - width
        } else {
            game.x + panel
        };
        let y = self.pointer.y.min(game.bottom() - height).max(game.y);
        ScreenRect {
            x,
            y,
            width,
            height,
        }
    }
}

/// A hotkey whose item hasn't arrived yet.
#[derive(Debug, Clone, Copy)]
struct Pending {
    target: Option<Target>,
    copy: Option<Instant>,
    told: Instant,
}

/// When the window shows: from the item the hotkey copied until a click in the game.
#[derive(Debug, Default)]
pub struct Overlay {
    /// Where the window shows, while it is open.
    shown: Option<Target>,
    /// The last hotkey, until its item arrives: where it was pressed, when the system could
    /// tell it; when it prepared the clipboard for its copy, which its item carries, if it
    /// could; and when it was told.
    pending: Option<Pending>,
    /// Turned off for good: it would no longer close.
    off: bool,
}

impl Overlay {
    /// The hotkey, told at `now`, prepared the clipboard for its copy at `copy`: the window
    /// opens at `target` when its item arrives. Without target (the pointer or the game couldn't
    /// be read) or without copy (the clipboard couldn't be prepared), its item only shows in
    /// the main window; it replaces the hotkey before it all the same, whose item would
    /// otherwise open the window at the wrong place.
    pub fn hotkey(&mut self, target: Option<Target>, copy: Option<Instant>, now: Instant) {
        self.pending = Some(Pending {
            target: target.filter(|_| !self.off),
            copy,
            told: now,
        });
    }

    /// An item arrived, which the hotkey that prepared the clipboard at `hotkey` copied, if
    /// any (the clipboard watching tells it). The item of the last hotkey opens the window
    /// where it was pressed, or moves it there (`true`); any other item (copied with Ctrl+C)
    /// only shows in the main window, and closes the window over the game, which would show it
    /// beside an item it isn't.
    pub fn copied(&mut self, hotkey: Option<Instant>) -> bool {
        match self.pending {
            Some(Pending {
                target,
                copy: Some(copy),
                ..
            }) if hotkey == Some(copy) => {
                self.pending = None;
                // A hotkey pressed just before the window was turned off opens nothing either.
                if target.is_some() && !self.off {
                    self.shown = target;
                }
                true
            }
            _ => {
                self.shown = None;
                false
            }
        }
    }

    /// Closes the window and never opens it again, when the clicks that close it can no longer
    /// be read; `true` the first time.
    pub fn turn_off(&mut self) -> bool {
        self.shown = None;
        !std::mem::replace(&mut self.off, true)
    }

    /// Forgets a hotkey that copied nothing; while one is pending, the time left before it is
    /// forgotten.
    pub fn expire(&mut self, now: Instant) -> Option<Duration> {
        let pending = self.pending?;
        let left = FORGOTTEN_AFTER.checked_sub(now.saturating_duration_since(pending.told));
        if left.is_none() {
            self.pending = None;
            log::info!("No item came from the price-check hotkey (an empty cell?)");
        }
        left
    }

    /// A button of the mouse pressed at `point`: outside the `window` (in the game), it closes
    /// it; in it, the window takes the click.
    pub fn clicked(&mut self, point: Point, window: Option<ScreenRect>) {
        if self.shown.is_some() && !window.is_some_and(|window| window.contains(point)) {
            self.shown = None;
        }
    }

    /// Where the window shows, while it is open.
    #[must_use]
    pub fn shown(&self) -> Option<Target> {
        self.shown
    }
}

/// No window over the game on this system yet: no click to watch.
#[cfg(not(target_os = "linux"))]
pub struct ClickWatch;

#[cfg(not(target_os = "linux"))]
#[expect(
    clippy::unused_self,
    reason = "the interface of the click watch of Linux, without its state"
)]
impl ClickWatch {
    #[must_use]
    pub fn start(_wake: impl Fn() + Send + 'static) -> Self {
        Self
    }

    pub fn watch(&mut self, _watching: bool) {}

    /// No click, and nothing that stopped.
    pub fn receive(&mut self, _handle: impl FnMut(Point)) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The owner's game on 2026-11-01: 2560 × 1440, on the second screen, right of the first.
    const GAME: ScreenRect = ScreenRect {
        x: 2560,
        y: 0,
        width: 2560,
        height: 1440,
    };
    /// The panels of that game: 888 px wide.
    const PANEL: i32 = 888;

    fn target(x: i32, y: i32) -> Target {
        Target {
            pointer: Point { x, y },
            game: GAME,
        }
    }

    #[test]
    fn opens_left_of_the_inventory_for_an_item_on_the_right_half() {
        // The owner's pointer on 2026-11-01, on an item of the inventory.
        let place = target(4557, 966).place(420, 300);

        assert_eq!(place.x, 2560 + 2560 - PANEL - 420);
        assert_eq!(place.width, 420);
    }

    #[test]
    fn opens_right_of_the_stash_for_an_item_on_the_left_half() {
        let place = target(2560 + 300, 500).place(420, 300);

        assert_eq!(place.x, 2560 + PANEL);
    }

    #[test]
    fn the_middle_of_the_game_counts_as_its_left_half() {
        assert_eq!(target(2560 + 1280, 500).place(420, 300).x, 2560 + PANEL);
        assert_eq!(
            target(2560 + 1281, 500).place(420, 300).x,
            2560 + 2560 - PANEL - 420
        );
    }

    #[test]
    fn its_top_is_level_with_the_pointer() {
        let place = target(4400, 398).place(420, 600);

        assert_eq!((place.y, place.height), (398, 600));
    }

    #[test]
    fn moves_up_as_much_as_the_game_needs() {
        let place = target(4400, 1200).place(420, 600);

        assert_eq!((place.y, place.height), (1440 - 600, 600));
    }

    #[test]
    fn is_at_most_the_height_of_the_game() {
        let place = target(4400, 700).place(420, 2000);

        assert_eq!((place.y, place.height), (0, 1440));
    }

    #[test]
    fn stays_in_a_game_moved_down_and_never_above_it() {
        let game = ScreenRect { y: 100, ..GAME };
        let place = Target {
            pointer: Point { x: 4400, y: 50 },
            game,
        }
        .place(420, 600);

        assert_eq!(place.y, 100);
    }

    #[test]
    fn a_window_of_no_height_is_one_pixel_high() {
        assert_eq!(target(4400, 700).place(420, 0).height, 1);
    }

    #[test]
    fn contains_the_points_inside_it_only() {
        let rect = ScreenRect {
            x: 10,
            y: 20,
            width: 30,
            height: 40,
        };

        assert!(rect.contains(Point { x: 10, y: 20 }));
        assert!(rect.contains(Point { x: 39, y: 59 }));
        assert!(!rect.contains(Point { x: 40, y: 30 }));
        assert!(!rect.contains(Point { x: 20, y: 60 }));
        assert!(!rect.contains(Point { x: 9, y: 30 }));
    }

    #[test]
    fn opens_when_the_item_of_the_hotkey_arrives() {
        let mut overlay = Overlay::default();
        let now = Instant::now();

        overlay.hotkey(Some(target(4557, 966)), Some(now), now);
        assert_eq!(overlay.shown(), None);
        assert!(overlay.copied(Some(now)));
        assert_eq!(overlay.shown(), Some(target(4557, 966)));
    }

    #[test]
    fn an_item_copied_with_ctrl_c_opens_nothing() {
        let mut overlay = Overlay::default();

        assert!(!overlay.copied(None));
        assert_eq!(overlay.shown(), None);
    }

    #[test]
    fn an_item_copied_with_ctrl_c_closes_the_open_window() {
        let mut overlay = Overlay::default();
        let now = Instant::now();
        overlay.hotkey(Some(target(4557, 966)), Some(now), now);
        overlay.copied(Some(now));

        assert!(!overlay.copied(None));
        assert_eq!(overlay.shown(), None);
    }

    #[test]
    fn a_hotkey_without_copy_replaces_the_one_before() {
        let mut overlay = Overlay::default();
        let empty_cell = Instant::now();
        let now = empty_cell + Duration::from_millis(500);
        overlay.hotkey(Some(target(4400, 900)), Some(empty_cell), empty_cell);
        overlay.hotkey(Some(target(4557, 966)), None, now);

        // An item the clipboard watching still took for the copy of the empty cell.
        assert!(!overlay.copied(Some(empty_cell)));
        assert_eq!(overlay.shown(), None);
    }

    #[test]
    fn an_item_copied_with_ctrl_c_while_a_hotkey_waits_opens_nothing() {
        let mut overlay = Overlay::default();
        let now = Instant::now();
        overlay.hotkey(Some(target(4557, 966)), Some(now), now);

        assert!(!overlay.copied(None));
        assert_eq!(overlay.shown(), None);
        // The item of the hotkey may still come.
        assert!(overlay.copied(Some(now)));
        assert_eq!(overlay.shown(), Some(target(4557, 966)));
    }

    #[test]
    fn the_item_of_an_older_hotkey_opens_nothing() {
        let mut overlay = Overlay::default();
        let older = Instant::now();
        let now = older + Duration::from_secs(2);
        overlay.hotkey(Some(target(4557, 966)), Some(now), now);

        assert!(!overlay.copied(Some(older)));
        assert_eq!(overlay.shown(), None);
    }

    #[test]
    fn a_hotkey_that_copied_nothing_is_forgotten() {
        let mut overlay = Overlay::default();
        let now = Instant::now();
        overlay.hotkey(Some(target(4557, 966)), Some(now), now);

        assert_eq!(
            overlay.expire(now + Duration::from_millis(400)),
            Some(Duration::from_millis(1600))
        );
        assert_eq!(overlay.expire(now + Duration::from_millis(2100)), None);
        assert!(!overlay.copied(Some(now)));
        assert_eq!(overlay.shown(), None);
    }

    #[test]
    fn the_hotkey_on_another_item_moves_the_open_window() {
        let mut overlay = Overlay::default();
        let now = Instant::now();
        let later = now + Duration::from_secs(5);
        overlay.hotkey(Some(target(4557, 966)), Some(now), now);
        overlay.copied(Some(now));

        overlay.hotkey(Some(target(2860, 500)), Some(later), later);
        assert_eq!(overlay.shown(), Some(target(4557, 966)));
        overlay.copied(Some(later));
        assert_eq!(overlay.shown(), Some(target(2860, 500)));
    }

    #[test]
    fn the_hotkey_on_an_empty_cell_keeps_the_open_window() {
        let mut overlay = Overlay::default();
        let now = Instant::now();
        overlay.hotkey(Some(target(4557, 966)), Some(now), now);
        overlay.copied(Some(now));

        overlay.hotkey(
            Some(target(4400, 900)),
            Some(now + Duration::from_secs(5)),
            now + Duration::from_secs(5),
        );
        overlay.expire(now + Duration::from_secs(8));
        assert_eq!(overlay.shown(), Some(target(4557, 966)));
    }

    #[test]
    fn a_hotkey_without_target_copies_without_opening() {
        let mut overlay = Overlay::default();
        let now = Instant::now();
        overlay.hotkey(None, Some(now), now);

        assert!(overlay.copied(Some(now)));
        assert_eq!(overlay.shown(), None);
    }

    #[test]
    fn a_hotkey_without_target_keeps_the_open_window() {
        let mut overlay = Overlay::default();
        let now = Instant::now();
        let later = now + Duration::from_secs(5);
        overlay.hotkey(Some(target(4557, 966)), Some(now), now);
        overlay.copied(Some(now));

        overlay.hotkey(None, Some(later), later);
        assert!(overlay.copied(Some(later)));
        assert_eq!(overlay.shown(), Some(target(4557, 966)));
    }

    #[test]
    fn turned_off_it_closes_and_never_opens_again() {
        let mut overlay = Overlay::default();
        let now = Instant::now();
        overlay.hotkey(Some(target(4557, 966)), Some(now), now);
        overlay.copied(Some(now));

        assert!(overlay.turn_off());
        assert_eq!(overlay.shown(), None);
        assert!(!overlay.turn_off());
        overlay.hotkey(Some(target(4557, 966)), Some(now), now);
        // The item still counts as copied by the hotkey.
        assert!(overlay.copied(Some(now)));
        assert_eq!(overlay.shown(), None);
    }

    #[test]
    fn a_hotkey_pressed_just_before_it_was_turned_off_opens_nothing() {
        let mut overlay = Overlay::default();
        let now = Instant::now();
        overlay.hotkey(Some(target(4557, 966)), Some(now), now);
        overlay.turn_off();

        assert!(overlay.copied(Some(now)));
        assert_eq!(overlay.shown(), None);
    }

    #[test]
    fn a_click_in_the_game_closes_it_and_a_click_in_it_doesnt() {
        let mut overlay = Overlay::default();
        let now = Instant::now();
        overlay.hotkey(Some(target(4557, 966)), Some(now), now);
        overlay.copied(Some(now));
        let window = target(4557, 966).place(420, 300);

        overlay.clicked(
            Point {
                x: window.x + 5,
                y: window.y + 5,
            },
            Some(window),
        );
        assert!(overlay.shown().is_some());
        overlay.clicked(Point { x: 4557, y: 966 }, Some(window));
        assert_eq!(overlay.shown(), None);
    }

    #[test]
    fn a_click_before_the_window_was_drawn_closes_it() {
        let mut overlay = Overlay::default();
        let now = Instant::now();
        overlay.hotkey(Some(target(4557, 966)), Some(now), now);
        overlay.copied(Some(now));

        overlay.clicked(Point { x: 4000, y: 500 }, None);
        assert_eq!(overlay.shown(), None);
    }
}
