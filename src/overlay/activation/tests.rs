use std::cell::RefCell;
use std::rc::Rc;

use super::*;

const GAME: Window = 10;
const OVERLAY: Window = 20;
const HELPER: Window = 30;
const MAIN: Window = 40;

/// What the fake desktop did, and how it answers.
#[derive(Debug, Default)]
struct Desk {
    /// Wine holds the pointer.
    grabbed: bool,
    /// The window Wine holds the pointer in is above the game.
    pointer_window: bool,
    /// The window over the game, once shown.
    overlay: Option<Window>,
    active: Option<Window>,
    /// The desktop doesn't activate the helper when asked.
    refuses: bool,
    /// The helper, while it is shown.
    helper: Option<Window>,
    focus: Option<Window>,
    activated: Vec<Window>,
    /// The call that fails, by its name.
    fails: Option<&'static str>,
    /// How many times the window Wine holds the pointer in was looked for.
    looks: usize,
    /// How many connections were made.
    connections: usize,
}

#[derive(Clone)]
struct FakeScreen(Rc<RefCell<Desk>>);

impl FakeScreen {
    fn call(&self, name: &'static str) -> Result<()> {
        if self.0.borrow().fails == Some(name) {
            return Err(format!("{name} failed").into());
        }
        Ok(())
    }
}

impl Screen for FakeScreen {
    fn pointer_grabbed(&mut self) -> Result<bool> {
        self.call("pointer_grabbed")?;
        Ok(self.0.borrow().grabbed)
    }
    fn pointer_window_above(&mut self, game: Window) -> Result<bool> {
        self.call("pointer_window_above")?;
        let mut desk = self.0.borrow_mut();
        desk.looks += 1;
        Ok(game == GAME && desk.pointer_window)
    }
    fn overlay(&mut self) -> Result<Option<Window>> {
        self.call("overlay")?;
        Ok(self.0.borrow().overlay)
    }
    fn active(&mut self) -> Result<Option<Window>> {
        self.call("active")?;
        Ok(self.0.borrow().active)
    }
    fn game_behind(&mut self, window: Window) -> Result<Option<Window>> {
        self.call("game_behind")?;
        let desk = self.0.borrow();
        Ok((window == HELPER && desk.helper == Some(HELPER)).then_some(GAME))
    }
    fn show_helper(&mut self, overlay: Window, game: Window) -> Result<Window> {
        self.call("show_helper")?;
        assert_eq!((overlay, game), (OVERLAY, GAME));
        self.0.borrow_mut().helper = Some(HELPER);
        Ok(HELPER)
    }
    fn hide_helper(&mut self) -> Result<()> {
        self.call("hide_helper")?;
        self.0.borrow_mut().helper = None;
        Ok(())
    }
    fn activate(&mut self, window: Window) -> Result<()> {
        self.call("activate")?;
        let mut desk = self.0.borrow_mut();
        desk.activated.push(window);
        if !(window == HELPER && desk.refuses) {
            desk.active = Some(window);
        }
        Ok(())
    }
    fn set_focus(&mut self, window: Window) -> Result<()> {
        self.call("set_focus")?;
        self.0.borrow_mut().focus = Some(window);
        Ok(())
    }
}

thread_local! {
    /// The fake desktop of the test, for the connection, a function that can't capture it.
    static DESK: RefCell<Option<FakeScreen>> = const { RefCell::new(None) };
}

type Desktop = Rc<RefCell<Desk>>;

/// An activation on a fake desktop where the game is active, holding the pointer (its window
/// above the game) or not.
fn activation(grabbed: bool) -> (Activation<FakeScreen>, Desktop) {
    let desk = Rc::new(RefCell::new(Desk {
        grabbed,
        pointer_window: grabbed,
        active: Some(GAME),
        ..Desk::default()
    }));
    DESK.with(|cell| *cell.borrow_mut() = Some(FakeScreen(Rc::clone(&desk))));
    let activation = Activation::with(|| {
        let screen = DESK.with(|cell| cell.borrow().clone().expect("a fake desktop"));
        screen.0.borrow_mut().connections += 1;
        Ok(screen)
    });
    (activation, desk)
}

/// Opened while the game holds the pointer, and followed until it holds the keyboard.
fn opened_active() -> (Activation<FakeScreen>, Desktop, Instant) {
    let (mut activation, desk) = activation(true);
    desk.borrow_mut().overlay = Some(OVERLAY);
    let now = Instant::now();
    activation.follow(Some(1), || false, now);
    activation.follow(Some(1), || false, now);
    assert!(activation.holds());
    (activation, desk, now)
}

fn later(now: Instant, millis: u64) -> Instant {
    now + Duration::from_millis(millis)
}

#[test]
fn on_two_screens_the_window_stays_out_of_the_way() {
    let (mut activation, desk) = activation(false);
    let now = Instant::now();
    desk.borrow_mut().overlay = Some(OVERLAY);

    // Looked for a while, then left alone.
    assert_eq!(activation.follow(Some(1), || false, now), Some(WATCH_POLL));
    assert_eq!(
        activation.follow(Some(1), || false, now + WATCH + WATCH_POLL),
        None
    );

    assert!(!activation.holds());
    let desk = desk.borrow();
    assert_eq!(desk.activated, Vec::<Window>::new());
    assert_eq!(desk.focus, None);
    assert_eq!(desk.active, Some(GAME));
}

#[test]
fn while_the_game_holds_the_pointer_the_window_becomes_the_active_one() {
    let (mut activation, desk) = activation(true);
    let now = Instant::now();

    // Not shown yet: it waits for the window, and wants the next frame soon.
    assert_eq!(activation.follow(Some(1), || false, now), Some(STEP_POLL));
    assert!(!activation.holds());
    desk.borrow_mut().overlay = Some(OVERLAY);
    // Shown: the helper is asked for, then the window takes the keyboard from it.
    assert_eq!(activation.follow(Some(1), || false, now), Some(STEP_POLL));
    assert_eq!(activation.follow(Some(1), || false, now), None);

    assert!(activation.holds());
    let desk = desk.borrow();
    assert_eq!(desk.helper, Some(HELPER));
    assert_eq!(desk.active, Some(HELPER));
    assert_eq!(desk.focus, Some(OVERLAY));
}

#[test]
fn the_game_holding_the_pointer_a_moment_after_the_window_opened_makes_it_active() {
    let (mut activation, desk) = activation(false);
    desk.borrow_mut().overlay = Some(OVERLAY);
    let now = Instant::now();
    activation.follow(Some(1), || false, now);
    {
        let mut desk = desk.borrow_mut();
        desk.grabbed = true;
        desk.pointer_window = true;
    }

    activation.follow(Some(1), || false, later(now, 250));
    activation.follow(Some(1), || false, later(now, 280));
    activation.follow(Some(1), || false, later(now, 310));

    assert!(activation.holds());
    assert_eq!(desk.borrow().active, Some(HELPER));
}

#[test]
fn a_button_held_or_another_grab_without_the_window_of_wine_changes_nothing() {
    let (mut activation, desk) = activation(false);
    desk.borrow_mut().overlay = Some(OVERLAY);
    let now = Instant::now();
    activation.follow(Some(1), || false, now);
    // The pointer held, but not by Wine for the game (no window of its own above the game).
    desk.borrow_mut().grabbed = true;

    activation.follow(Some(1), || false, later(now, 250));

    assert!(!activation.holds());
    assert_eq!(desk.borrow().activated, Vec::<Window>::new());
}

#[test]
fn closed_with_esc_the_game_is_the_active_window_again() {
    let (mut activation, desk, now) = opened_active();

    activation.escape();
    activation.follow(None, || false, now);

    assert!(!activation.holds());
    let desk = desk.borrow();
    assert_eq!(desk.active, Some(GAME));
    assert_eq!(desk.helper, None);
}

#[test]
fn closed_by_a_click_the_desktop_chooses_even_when_it_is_late() {
    let (mut activation, desk, now) = opened_active();

    // The desktop hasn't activated the window clicked yet: the helper still is the active one.
    activation.follow(None, || false, now);

    let desk = desk.borrow();
    assert_eq!(desk.activated, [HELPER]);
    assert_eq!(desk.helper, None);
}

#[test]
fn esc_after_another_window_became_active_leaves_it_there() {
    let (mut activation, desk, now) = opened_active();
    desk.borrow_mut().active = Some(MAIN);

    activation.escape();
    activation.follow(None, || false, now);

    let desk = desk.borrow();
    assert_eq!(desk.active, Some(MAIN));
    assert_eq!(desk.activated, [HELPER]);
}

/// The copy of a hotkey pressed while the window is the active one: the game made the active
/// window again, Wine not holding the pointer yet, a frame comes (the window lost the keyboard)
/// before the window receives the press.
fn copy_on_its_way(activation: &mut Activation<FakeScreen>, desk: &Desktop, now: Instant) {
    {
        let mut desk = desk.borrow_mut();
        desk.active = Some(GAME);
        desk.grabbed = false;
        desk.pointer_window = false;
    }
    activation.follow(Some(1), || true, now);
    assert!(activation.holds());
    activation.hotkey();
    activation.follow(Some(1), || true, now);
}

#[test]
fn the_hotkey_opens_it_again_as_the_active_window_without_the_pointer_held() {
    let (mut activation, desk, now) = opened_active();
    copy_on_its_way(&mut activation, &desk, now);
    assert_eq!(desk.borrow().helper, Some(HELPER));

    // Its item.
    activation.follow(Some(2), || false, now);
    activation.follow(Some(2), || false, now);

    assert!(activation.holds());
    let desk = desk.borrow();
    assert_eq!(desk.active, Some(HELPER));
    assert_eq!(desk.activated, [HELPER, HELPER]);
}

#[test]
fn opened_again_while_it_holds_the_keyboard_it_keeps_it() {
    let (mut activation, desk, now) = opened_active();
    // The press seen too late to carry: the window still has the keyboard.
    desk.borrow_mut().grabbed = false;

    activation.follow(Some(2), || false, now);
    activation.follow(Some(2), || false, now);

    assert!(activation.holds());
}

#[test]
fn the_hotkey_on_an_empty_cell_makes_the_window_active_again() {
    let (mut activation, desk, now) = opened_active();
    copy_on_its_way(&mut activation, &desk, now);

    // The hotkey forgotten: no item came.
    activation.follow(Some(1), || false, later(now, 2000));
    activation.follow(Some(1), || false, later(now, 2030));
    activation.follow(Some(1), || false, later(now, 2060));

    assert!(activation.holds());
    assert_eq!(desk.borrow().active, Some(HELPER));
    // The next hotkey doesn't take the empty cell for an opening of its own.
    assert!(!activation.carry);
}

#[test]
fn a_failure_after_the_hotkey_hides_the_helper_and_gives_the_game_back() {
    let (mut activation, desk, now) = opened_active();
    // The copy couldn't make the game active: the helper still is.
    activation.follow(Some(1), || true, now);
    activation.hotkey();
    desk.borrow_mut().fails = Some("show_helper");

    activation.follow(Some(2), || false, now);

    assert!(!activation.holds());
    let desk = desk.borrow();
    assert_eq!(desk.helper, None);
    assert_eq!(desk.active, Some(GAME));
}

#[test]
fn a_hotkey_on_an_empty_cell_while_the_window_becomes_active_is_forgotten() {
    let (mut activation, desk) = activation(true);
    let now = Instant::now();
    activation.follow(Some(1), || false, now);
    activation.hotkey();
    activation.follow(Some(1), || true, now);
    // No item came.
    activation.follow(Some(1), || false, later(now, 100));
    desk.borrow_mut().overlay = Some(OVERLAY);
    activation.follow(Some(1), || false, later(now, 130));
    activation.follow(Some(1), || false, later(now, 160));
    assert!(activation.holds());

    // Alt+Tab: the window doesn't take the window chosen back.
    desk.borrow_mut().active = Some(MAIN);
    activation.follow(Some(1), || false, later(now, 190));

    assert!(!activation.holds());
    let desk = desk.borrow();
    assert_eq!(desk.activated, [HELPER]);
    assert_eq!(desk.active, Some(MAIN));
}

#[test]
fn a_copy_on_its_way_keeps_the_window_from_taking_its_keys() {
    let (mut activation, desk) = activation(false);
    desk.borrow_mut().overlay = Some(OVERLAY);
    let now = Instant::now();
    activation.follow(Some(1), || false, now);
    {
        let mut desk = desk.borrow_mut();
        desk.grabbed = true;
        desk.pointer_window = true;
    }

    activation.follow(Some(1), || true, later(now, 250));
    assert_eq!(desk.borrow().activated, Vec::<Window>::new());

    // Sent: the window becomes the active one.
    activation.follow(Some(1), || false, later(now, 500));
    activation.follow(Some(1), || false, later(now, 530));
    activation.follow(Some(1), || false, later(now, 560));
    assert!(activation.holds());
}

#[test]
fn the_window_of_wine_is_looked_for_four_times_a_second_whatever_the_frames() {
    let (mut activation, desk) = activation(false);
    desk.borrow_mut().overlay = Some(OVERLAY);
    let now = Instant::now();
    activation.follow(Some(1), || false, now);
    assert_eq!(desk.borrow().looks, 1);

    // The hover of the window on two screens, a frame at each move.
    for millis in (10..250).step_by(10) {
        assert!(
            activation
                .follow(Some(1), || false, later(now, millis))
                .is_some()
        );
    }
    assert_eq!(desk.borrow().looks, 1);

    activation.follow(Some(1), || false, later(now, 250));
    assert_eq!(desk.borrow().looks, 2);
}

#[test]
fn after_a_failure_the_next_opening_connects_again() {
    let (mut activation, desk) = activation(true);
    {
        let mut desk = desk.borrow_mut();
        desk.overlay = Some(OVERLAY);
        desk.fails = Some("show_helper");
    }
    let now = Instant::now();
    activation.follow(Some(1), || false, now);
    activation.follow(None, || false, now);
    desk.borrow_mut().fails = None;

    activation.follow(Some(2), || false, now);
    activation.follow(Some(2), || false, now);

    assert!(activation.holds());
    assert_eq!(desk.borrow().connections, 2);
}

#[test]
fn closing_without_ever_opening_connects_to_nothing() {
    let (activation, desk) = activation(true);

    drop(activation);

    assert_eq!(desk.borrow().connections, 0);
}

#[test]
fn the_hotkey_on_two_screens_asks_again() {
    let (mut activation, desk) = activation(false);
    desk.borrow_mut().overlay = Some(OVERLAY);
    let now = Instant::now();
    activation.follow(Some(1), || false, now);
    activation.hotkey();

    activation.follow(Some(2), || false, now);

    assert!(!activation.holds());
    assert_eq!(desk.borrow().activated, Vec::<Window>::new());
}

#[test]
fn another_window_made_active_lets_the_keys_go_there() {
    let (mut activation, desk, now) = opened_active();
    // Alt+Tab.
    desk.borrow_mut().active = Some(MAIN);

    assert_eq!(activation.follow(Some(1), || false, now), Some(WATCH_POLL));

    assert!(!activation.holds());
    let desk = desk.borrow();
    assert_eq!(desk.helper, None);
    assert_eq!(desk.active, Some(MAIN));
    assert_eq!(desk.activated, [HELPER]);
}

#[test]
fn back_in_the_game_the_window_becomes_the_active_one_again() {
    let (mut activation, desk, now) = opened_active();
    desk.borrow_mut().active = Some(MAIN);
    activation.follow(Some(1), || false, now);
    // Back in the game, which holds the pointer again.
    {
        let mut desk = desk.borrow_mut();
        desk.active = Some(GAME);
        desk.pointer_window = true;
    }

    activation.follow(Some(1), || false, later(now, 1000));
    activation.follow(Some(1), || false, later(now, 1030));
    activation.follow(Some(1), || false, later(now, 1060));

    assert!(activation.holds());
}

#[test]
fn a_desktop_that_refuses_leaves_the_window_as_on_two_screens() {
    let (mut activation, desk) = activation(true);
    {
        let mut desk = desk.borrow_mut();
        desk.overlay = Some(OVERLAY);
        desk.refuses = true;
    }
    let now = Instant::now();
    activation.follow(Some(1), || false, now);
    assert_eq!(activation.follow(Some(1), || false, now), Some(STEP_POLL));

    assert_eq!(
        activation.follow(Some(1), || false, now + TIMEOUT + STEP_POLL),
        None
    );

    assert!(!activation.holds());
    let desk = desk.borrow();
    assert_eq!(desk.helper, None);
    assert_eq!(desk.focus, None);
}

#[test]
fn a_window_that_never_shows_is_given_up() {
    let (mut activation, desk) = activation(true);
    let now = Instant::now();
    assert_eq!(activation.follow(Some(1), || false, now), Some(STEP_POLL));

    assert_eq!(
        activation.follow(Some(1), || false, now + TIMEOUT + STEP_POLL),
        None
    );

    assert_eq!(desk.borrow().activated, Vec::<Window>::new());
}

#[test]
fn closed_before_it_shows_nothing_is_activated() {
    let (mut activation, desk) = activation(true);
    let now = Instant::now();
    activation.follow(Some(1), || false, now);

    activation.follow(None, || false, now);
    desk.borrow_mut().overlay = Some(OVERLAY);
    activation.follow(None, || false, now);

    assert_eq!(desk.borrow().activated, Vec::<Window>::new());
}

#[test]
fn a_failure_while_the_helper_is_active_gives_the_game_back() {
    let (mut activation, desk) = activation(true);
    desk.borrow_mut().overlay = Some(OVERLAY);
    let now = Instant::now();
    activation.follow(Some(1), || false, now);
    // The keyboard can't be given to the window, once the helper is active.
    desk.borrow_mut().fails = Some("set_focus");

    assert_eq!(activation.follow(Some(1), || false, now), None);

    assert!(!activation.holds());
    let desk = desk.borrow();
    assert_eq!(desk.activated, [HELPER, GAME]);
    assert_eq!(desk.helper, None);
}

#[test]
fn a_failure_before_the_helper_leaves_the_window_as_on_two_screens() {
    let (mut activation, desk) = activation(true);
    {
        let mut desk = desk.borrow_mut();
        desk.overlay = Some(OVERLAY);
        desk.fails = Some("show_helper");
    }

    assert_eq!(activation.follow(Some(1), || false, Instant::now()), None);

    assert!(!activation.holds());
    assert_eq!(desk.borrow().activated, Vec::<Window>::new());
}

#[test]
fn a_failure_to_read_the_pointer_opens_it_as_on_two_screens() {
    let (mut activation, desk) = activation(true);
    {
        let mut desk = desk.borrow_mut();
        desk.overlay = Some(OVERLAY);
        desk.fails = Some("pointer_grabbed");
    }

    assert_eq!(activation.follow(Some(1), || false, Instant::now()), None);

    assert!(!activation.holds());
}

#[test]
fn the_application_closing_gives_the_game_back() {
    let (activation, desk, _) = opened_active();

    drop(activation);

    let desk = desk.borrow();
    assert_eq!(desk.active, Some(GAME));
    assert_eq!(desk.helper, None);
}

#[test]
fn esc_closes_the_window_unless_it_leaves_a_field() {
    let (mut activation, _desk, _) = opened_active();
    assert!(activation.escape_closes());

    // A field edited at the last frame: egui already took the keyboard away at this Esc.
    activation.editing(true);
    assert!(!activation.escape_closes());
    activation.editing(false);
    assert!(activation.escape_closes());
}

#[test]
fn on_two_screens_esc_is_left_to_the_game() {
    let (mut activation, desk) = activation(false);
    desk.borrow_mut().overlay = Some(OVERLAY);
    activation.follow(Some(1), || false, Instant::now());

    assert!(!activation.escape_closes());
}
