use super::*;

/// A desktop whose foreground window the test sets, `0` for a window of this application.
#[derive(Default)]
struct FakeDesktop {
    foreground: u32,
    game: Option<u32>,
    activated: Vec<u32>,
    refuses: bool,
}

impl Desktop for FakeDesktop {
    fn foreground_is_ours(&mut self) -> bool {
        self.foreground == OURS
    }

    fn remember_game(&mut self) {
        if self.foreground != OURS {
            self.game = Some(self.foreground);
        }
    }

    fn activate_game(&mut self) -> bool {
        let Some(game) = self.game.filter(|_| !self.refuses) else {
            return false;
        };
        self.activated.push(game);
        self.foreground = game;
        true
    }
}

const GAME: u32 = 7;
const OURS: u32 = 0;

/// A frame of the window hidden, shown without the focus, or with it, nothing held nor
/// edited.
const HIDDEN: Frame = Frame::HIDDEN;
const UNFOCUSED: Frame = Frame {
    focused: Some(false),
    ..HIDDEN
};
const FOCUSED: Frame = Frame {
    focused: Some(true),
    ..HIDDEN
};

/// The window over the game opened over the game, which has the focus.
fn opened() -> GameFocus<FakeDesktop> {
    let mut focus = GameFocus::new(FakeDesktop {
        foreground: GAME,
        ..FakeDesktop::default()
    });
    assert_eq!(focus.follow(UNFOCUSED), Gave::Nothing);
    focus
}

/// A click in the window over the game, which takes the focus.
fn click(focus: &mut GameFocus<FakeDesktop>) {
    focus.desktop.foreground = OURS;
}

#[test]
fn gives_the_focus_back_to_the_game_once_the_click_is_released() {
    let mut focus = opened();
    click(&mut focus);

    assert_eq!(
        focus.follow(Frame {
            pressed: true,
            ..FOCUSED
        }),
        Gave::Nothing
    );
    assert_eq!(focus.follow(FOCUSED), Gave::Given);
    assert_eq!(focus.desktop.activated, [GAME]);
}

#[test]
fn gives_it_back_once_per_click() {
    let mut focus = opened();
    click(&mut focus);
    focus.follow(FOCUSED);
    // The frames before the system tells the window it lost the focus.
    assert_eq!(focus.follow(FOCUSED), Gave::Nothing);
    assert_eq!(focus.follow(UNFOCUSED), Gave::Nothing);

    click(&mut focus);
    assert_eq!(focus.follow(FOCUSED), Gave::Given);
    assert_eq!(focus.desktop.activated, [GAME, GAME]);
}

#[test]
fn a_window_of_this_application_is_never_taken_for_the_game() {
    let mut focus = opened();
    // The main window brought to the front, then a click in the window over the game.
    focus.desktop.foreground = OURS;
    focus.follow(UNFOCUSED);

    assert_eq!(focus.follow(FOCUSED), Gave::Given);
    assert_eq!(focus.desktop.activated, [GAME]);
}

#[test]
fn gives_it_to_the_last_window_that_had_it() {
    let mut focus = opened();
    // Alt+Tab to a browser while the window over the game stays open.
    focus.desktop.foreground = 9;
    focus.follow(UNFOCUSED);
    click(&mut focus);

    assert_eq!(focus.follow(FOCUSED), Gave::Given);
    assert_eq!(focus.desktop.activated, [9]);
}

#[test]
fn without_game_seen_nothing_is_given() {
    let mut focus = GameFocus::new(FakeDesktop::default());

    assert_eq!(focus.follow(FOCUSED), Gave::Failed);
    assert_eq!(focus.follow(FOCUSED), Gave::Nothing);
}

#[test]
fn a_refusal_is_tried_again_at_the_next_click_only() {
    let mut focus = opened();
    focus.desktop.refuses = true;
    click(&mut focus);

    assert_eq!(focus.follow(FOCUSED), Gave::Failed);
    assert_eq!(focus.follow(FOCUSED), Gave::Nothing);
    focus.desktop.refuses = false;
    // A click in the game gave it the focus back; the next click in the window takes it.
    focus.desktop.foreground = GAME;
    focus.follow(UNFOCUSED);
    click(&mut focus);
    assert_eq!(focus.follow(FOCUSED), Gave::Given);
}

#[test]
fn hidden_it_gives_nothing_and_shown_again_it_gives_it_back() {
    let mut focus = opened();
    click(&mut focus);
    focus.follow(FOCUSED);

    assert_eq!(focus.follow(HIDDEN), Gave::Nothing);
    // Opened again by the hotkey, then clicked before any frame saw it without the focus.
    assert_eq!(focus.follow(FOCUSED), Gave::Given);
}

#[test]
fn keeps_the_focus_while_a_field_is_edited_and_gives_it_back_once_left() {
    let mut focus = opened();
    click(&mut focus);
    let editing = Frame {
        editing: true,
        ..FOCUSED
    };

    assert_eq!(
        focus.follow(Frame {
            pressed: true,
            ..editing
        }),
        Gave::Nothing
    );
    // The number typed, then Enter or Esc.
    assert_eq!(focus.follow(editing), Gave::Nothing);
    assert_eq!(focus.follow(editing), Gave::Nothing);
    assert_eq!(focus.desktop.activated, [] as [u32; 0]);
    assert_eq!(focus.follow(FOCUSED), Gave::Given);
    assert_eq!(focus.desktop.activated, [GAME]);
}

#[test]
fn a_click_elsewhere_in_the_window_gives_it_back_once_released() {
    let mut focus = opened();
    click(&mut focus);
    focus.follow(Frame {
        editing: true,
        ..FOCUSED
    });

    // A checkbox pressed, which leaves the field.
    assert_eq!(
        focus.follow(Frame {
            pressed: true,
            ..FOCUSED
        }),
        Gave::Nothing
    );
    assert_eq!(focus.follow(FOCUSED), Gave::Given);
}

#[test]
fn a_field_edited_then_left_for_another_window_gives_nothing() {
    let mut focus = opened();
    click(&mut focus);
    focus.follow(Frame {
        editing: true,
        ..FOCUSED
    });

    // Alt+Tab to a browser: it keeps the focus.
    focus.desktop.foreground = 9;
    assert_eq!(focus.follow(UNFOCUSED), Gave::Nothing);
    assert_eq!(focus.desktop.activated, [] as [u32; 0]);
}

/// A field edited after a click in it: the window has the keyboard.
fn editing(focus: &mut GameFocus<FakeDesktop>) -> bool {
    click(focus);
    let frame = Frame {
        editing: true,
        ..FOCUSED
    };
    focus.follow(frame);
    frame.holds_keyboard()
}

#[test]
fn holds_the_keyboard_while_focused_and_editing_only() {
    let editing = Frame {
        editing: true,
        ..FOCUSED
    };

    assert!(editing.holds_keyboard());
    assert!(!FOCUSED.holds_keyboard());
    assert!(
        !Frame {
            focused: Some(false),
            ..editing
        }
        .holds_keyboard()
    );
    assert!(
        !Frame {
            focused: None,
            ..editing
        }
        .holds_keyboard()
    );
}

#[test]
fn the_hotkey_gives_the_focus_back_to_the_game_while_a_field_is_edited() {
    let mut focus = opened();
    let held = editing(&mut focus);

    assert_eq!(
        give_keyboard_back(&mut focus.desktop, held),
        KeyboardBack::Given
    );
    assert_eq!(focus.desktop.activated, [GAME]);
    // The window lost the focus, then a click in it takes it, given back once released.
    assert_eq!(focus.follow(UNFOCUSED), Gave::Nothing);
    click(&mut focus);
    assert_eq!(focus.follow(FOCUSED), Gave::Given);
    assert_eq!(focus.desktop.activated, [GAME, GAME]);
}

#[test]
fn the_hotkey_without_field_edited_gives_nothing() {
    let mut focus = opened();

    assert_eq!(
        give_keyboard_back(&mut focus.desktop, false),
        KeyboardBack::NotHeld
    );
    assert_eq!(focus.desktop.activated, [] as [u32; 0]);
}

#[test]
fn the_hotkey_leaves_the_focus_to_another_window_that_took_it() {
    let mut focus = opened();
    let held = editing(&mut focus);
    // Alt+Tab to a browser, before a frame saw it.
    focus.desktop.foreground = 9;

    assert_eq!(
        give_keyboard_back(&mut focus.desktop, held),
        KeyboardBack::NotHeld
    );
    assert_eq!(focus.desktop.activated, [] as [u32; 0]);
}

#[test]
fn the_hotkey_tells_a_refusal() {
    let mut focus = opened();
    let held = editing(&mut focus);
    focus.desktop.refuses = true;

    assert_eq!(
        give_keyboard_back(&mut focus.desktop, held),
        KeyboardBack::Refused
    );
}

#[test]
fn a_click_that_opens_a_link_leaves_the_focus_to_the_browser() {
    let mut focus = opened();
    click(&mut focus);

    assert_eq!(
        focus.follow(Frame {
            opens_link: true,
            ..FOCUSED
        }),
        Gave::Left
    );
    assert_eq!(focus.follow(FOCUSED), Gave::Nothing);
    assert_eq!(focus.desktop.activated, [] as [u32; 0]);
    // Back in the game, the next click in the window gives the focus back again.
    focus.desktop.foreground = GAME;
    focus.follow(UNFOCUSED);
    click(&mut focus);
    assert_eq!(focus.follow(FOCUSED), Gave::Given);
}
