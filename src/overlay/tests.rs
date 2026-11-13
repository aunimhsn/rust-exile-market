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
fn a_hotkey_without_target_closes_the_open_window() {
    let mut overlay = Overlay::default();
    let now = Instant::now();
    let later = now + Duration::from_secs(5);
    overlay.hotkey(Some(target(4557, 966)), Some(now), now);
    overlay.copied(Some(now));

    overlay.hotkey(None, Some(later), later);
    assert!(overlay.copied(Some(later)));
    assert_eq!(overlay.shown(), None);
}

#[test]
fn the_item_of_an_older_hotkey_closes_the_window_until_the_newer_one_arrives() {
    let mut overlay = Overlay::default();
    let first = Instant::now();
    let second = first + Duration::from_millis(400);
    overlay.hotkey(Some(target(4557, 966)), Some(first), first);
    overlay.copied(Some(first));
    overlay.hotkey(Some(target(4557, 966)), Some(second), first);
    overlay.hotkey(
        Some(target(2860, 500)),
        Some(second + Duration::from_millis(1)),
        second,
    );

    // The item of the second hotkey (the first arrives late) closes it, the third opens it.
    assert!(!overlay.copied(Some(second)));
    assert_eq!(overlay.shown(), None);
    assert!(overlay.copied(Some(second + Duration::from_millis(1))));
    assert_eq!(overlay.shown(), Some(target(2860, 500)));
}

#[test]
fn a_hotkey_without_copy_waits_for_nothing() {
    let mut overlay = Overlay::default();
    let now = Instant::now();
    overlay.hotkey(Some(target(4557, 966)), None, now);

    assert_eq!(overlay.expire(now), None);
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
fn each_hotkey_leaves_the_field_edited_once() {
    let mut overlay = Overlay::default();
    assert!(!overlay.take_leave_field());
    overlay.hotkey(None, None, Instant::now());
    assert!(overlay.take_leave_field());
    assert!(!overlay.take_leave_field());
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

#[test]
fn each_opening_has_its_own_number() {
    let mut overlay = Overlay::default();
    let now = Instant::now();
    assert_eq!(overlay.opened(), None);
    overlay.hotkey(Some(target(4557, 966)), Some(now), now);
    overlay.copied(Some(now));
    let first = overlay.opened();

    // Opened again on another item, without closing in between.
    let later = now + Duration::from_millis(500);
    overlay.hotkey(Some(target(3000, 400)), Some(later), later);
    overlay.copied(Some(later));

    assert!(first.is_some());
    assert!(overlay.opened().is_some());
    assert_ne!(overlay.opened(), first);
}

#[test]
fn esc_closes_it() {
    let mut overlay = Overlay::default();
    let now = Instant::now();
    overlay.hotkey(Some(target(4557, 966)), Some(now), now);
    overlay.copied(Some(now));

    overlay.close();

    assert_eq!(overlay.shown(), None);
    assert_eq!(overlay.opened(), None);
}
