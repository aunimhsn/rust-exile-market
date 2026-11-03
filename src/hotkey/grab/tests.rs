use super::*;

/// What `grab` does when the active window is `active`, the grab and the release giving
/// `result`, and how many times each was called.
fn follow(grab: &mut Grab, active: Active, result: Result<(), &str>) -> (Change, u32, u32) {
    let (mut grabs, mut releases) = (0, 0);
    let change = grab.follow(
        active,
        || {
            grabs += 1;
            result
        },
        || {
            releases += 1;
            result
        },
    );
    (change, grabs, releases)
}

#[test]
fn grabs_the_key_when_the_game_becomes_the_active_window() {
    let mut grab = Grab::default();

    assert_eq!(
        follow(&mut grab, Active::Game, Ok(())),
        (Change::Grabbed, 1, 0)
    );
}

#[test]
fn leaves_the_key_to_another_window_active_at_the_start() {
    let mut grab = Grab::default();

    assert_eq!(
        follow(&mut grab, Active::Other, Ok(())),
        (Change::Nothing, 0, 0)
    );
}

#[test]
fn grabs_the_key_once_while_the_game_stays_the_active_window() {
    let mut grab = Grab::default();
    follow(&mut grab, Active::Game, Ok(()));

    // A change of its title, or of another property followed.
    assert_eq!(
        follow(&mut grab, Active::Game, Ok(())),
        (Change::Nothing, 0, 0)
    );
}

#[test]
fn releases_the_key_when_another_window_becomes_the_active_window() {
    let mut grab = Grab::default();
    follow(&mut grab, Active::Game, Ok(()));

    assert_eq!(
        follow(&mut grab, Active::Other, Ok(())),
        (Change::Released, 0, 1)
    );
    assert_eq!(
        follow(&mut grab, Active::Other, Ok(())),
        (Change::Nothing, 0, 0)
    );
    assert_eq!(
        follow(&mut grab, Active::Game, Ok(())),
        (Change::Grabbed, 1, 0)
    );
}

#[test]
fn tries_a_key_taken_by_another_application_again_at_the_next_activation_of_the_game() {
    let mut grab = Grab::default();
    assert_eq!(
        follow(&mut grab, Active::Game, Err("taken")),
        (Change::Failed, 1, 0)
    );

    // Never grabbed: nothing to release.
    assert_eq!(
        follow(&mut grab, Active::Other, Ok(())),
        (Change::Nothing, 0, 0)
    );
    assert_eq!(
        follow(&mut grab, Active::Game, Err("taken")),
        (Change::Failed, 1, 0)
    );
    assert_eq!(
        follow(&mut grab, Active::Game, Ok(())),
        (Change::Grabbed, 1, 0)
    );
}

#[test]
fn keeps_the_key_as_it_is_while_the_desktop_tells_no_active_window() {
    let mut grab = Grab::default();
    assert_eq!(
        follow(&mut grab, Active::Unknown, Ok(())),
        (Change::Nothing, 0, 0)
    );
    follow(&mut grab, Active::Game, Ok(()));

    // The window over the game keeping the keyboard, on a desktop other than KWin.
    assert_eq!(
        follow(&mut grab, Active::Unknown, Ok(())),
        (Change::Nothing, 0, 0)
    );
    assert_eq!(
        follow(&mut grab, Active::Other, Ok(())),
        (Change::Released, 0, 1)
    );
}

#[test]
fn grabs_the_key_for_every_window_once_the_active_window_cant_be_followed() {
    let mut grab = Grab::default();
    assert_eq!(
        follow(&mut grab, Active::NotFollowed, Ok(())),
        (Change::Grabbed, 1, 0)
    );

    let mut grab = Grab::default();
    follow(&mut grab, Active::Game, Ok(()));
    // Already grabbed while the game was active: kept.
    assert_eq!(
        follow(&mut grab, Active::NotFollowed, Ok(())),
        (Change::Nothing, 0, 0)
    );

    let mut grab = Grab::default();
    follow(&mut grab, Active::Game, Err("taken"));
    follow(&mut grab, Active::Other, Ok(()));
    // Tried again after a failure.
    assert_eq!(
        follow(&mut grab, Active::NotFollowed, Err("taken")),
        (Change::Failed, 1, 0)
    );
}

/// What `follow` does when the foreground window told is `foreground`, the main window
/// `main_focused` or not, the grab and the release giving `result`, and how many times each
/// was called.
fn follow_foreground(
    follow: &mut ForegroundFollow,
    foreground: Foreground,
    main_focused: bool,
    result: Result<(), &str>,
) -> (Change, u32, u32) {
    follow.told(foreground);
    let (mut grabs, mut releases) = (0, 0);
    let change = follow.follow(
        main_focused,
        || {
            grabs += 1;
            result
        },
        || {
            releases += 1;
            result
        },
    );
    (change, grabs, releases)
}

#[test]
fn the_game_is_told_by_its_exact_title_out_of_the_file_explorer() {
    assert!(is_game_window("Path of Exile 2", "POEWindowClass"));
    // A page of a browser about the game.
    assert!(!is_game_window(
        "Path of Exile 2 - Wiki",
        "Chrome_WidgetWin_1"
    ));
    // The folder of the game.
    assert!(!is_game_window("Path of Exile 2", "CabinetWClass"));
}

#[test]
fn the_game_in_front_is_the_game_whether_the_main_window_has_the_keyboard_or_not() {
    assert_eq!(Foreground::Game.active(false), Active::Game);
    // The keyboard of the main window not told yet as lost.
    assert_eq!(Foreground::Game.active(true), Active::Game);
}

#[test]
fn the_window_over_the_game_in_front_keeps_the_key_as_it_is() {
    let mut follow = ForegroundFollow::default();
    follow_foreground(&mut follow, Foreground::Game, false, Ok(()));

    // In front after a click in it, or while one of its fields is edited.
    assert_eq!(
        follow_foreground(&mut follow, Foreground::Ours, false, Ok(())),
        (Change::Nothing, 0, 0)
    );
}

#[test]
fn the_main_window_in_front_releases_the_key_as_another_window() {
    let mut follow = ForegroundFollow::default();
    follow_foreground(&mut follow, Foreground::Game, false, Ok(()));

    // Alt+Tab to the main window: in front, then its keyboard told by egui.
    follow_foreground(&mut follow, Foreground::Ours, false, Ok(()));
    assert_eq!(
        follow_foreground(&mut follow, Foreground::Ours, true, Ok(())),
        (Change::Released, 0, 1)
    );
    assert_eq!(
        follow_foreground(&mut follow, Foreground::Other, false, Ok(())),
        (Change::Nothing, 0, 0)
    );
}

#[test]
fn no_foreground_window_for_a_moment_keeps_the_key_as_it_is() {
    let mut follow = ForegroundFollow::default();
    follow_foreground(&mut follow, Foreground::Game, false, Ok(()));

    assert_eq!(
        follow_foreground(&mut follow, Foreground::NoWindow, false, Ok(())),
        (Change::Nothing, 0, 0)
    );
    assert_eq!(
        follow_foreground(&mut follow, Foreground::Other, false, Ok(())),
        (Change::Released, 0, 1)
    );
}

#[test]
fn the_last_foreground_window_told_is_the_one_followed() {
    let mut follow = ForegroundFollow::default();
    // Several changes between two frames: the last one counts.
    follow.told(Foreground::Other);
    follow.told(Foreground::NoWindow);

    assert_eq!(
        follow_foreground(&mut follow, Foreground::Game, false, Ok(())),
        (Change::Grabbed, 1, 0)
    );
}

#[test]
fn follows_each_change_of_the_foreground_window_once() {
    let mut follow = ForegroundFollow::default();
    follow_foreground(&mut follow, Foreground::Game, false, Ok(()));

    // The next frames, the game still in front.
    assert_eq!(
        follow_foreground(&mut follow, Foreground::Game, false, Ok(())),
        (Change::Nothing, 0, 0)
    );
}

#[test]
fn tries_a_key_taken_by_another_application_again_at_the_next_change_not_at_each_frame() {
    let mut follow = ForegroundFollow::default();
    assert_eq!(
        follow_foreground(&mut follow, Foreground::Game, false, Err("taken")),
        (Change::Failed, 1, 0)
    );
    assert_eq!(
        follow_foreground(&mut follow, Foreground::Game, false, Err("taken")),
        (Change::Nothing, 0, 0)
    );

    follow_foreground(&mut follow, Foreground::Other, false, Ok(()));
    assert_eq!(
        follow_foreground(&mut follow, Foreground::Game, false, Ok(())),
        (Change::Grabbed, 1, 0)
    );
}

#[test]
fn grabs_the_key_for_every_window_once_the_foreground_window_is_lost() {
    let mut follow = ForegroundFollow::default();
    follow_foreground(&mut follow, Foreground::Other, false, Ok(()));

    follow.lose();
    assert_eq!(
        follow_foreground(&mut follow, Foreground::Other, false, Ok(())),
        (Change::Grabbed, 1, 0)
    );
    // Kept, whatever window comes to the front.
    assert_eq!(
        follow_foreground(&mut follow, Foreground::Ours, true, Ok(())),
        (Change::Nothing, 0, 0)
    );
}

/// What turning the hotkey `on` or off does, the grab and the release giving `result`.
fn turn(grab: &mut Grab, on: bool, result: Result<(), &str>) -> (Change, u32, u32) {
    let (mut grabs, mut releases) = (0, 0);
    let change = grab.turn(
        on,
        || {
            grabs += 1;
            result
        },
        || {
            releases += 1;
            result
        },
    );
    (change, grabs, releases)
}

#[test]
fn turned_off_the_key_is_released_at_once_and_never_grabbed() {
    let mut grab = Grab::default();
    follow(&mut grab, Active::Game, Ok(()));

    assert_eq!(turn(&mut grab, false, Ok(())), (Change::Released, 0, 1));
    follow(&mut grab, Active::Other, Ok(()));
    assert_eq!(
        follow(&mut grab, Active::Game, Ok(())),
        (Change::Nothing, 0, 0)
    );
}

#[test]
fn off_from_the_start_the_key_is_never_grabbed() {
    let mut grab = Grab::new(false, Shortcut::default());

    assert_eq!(
        follow(&mut grab, Active::Game, Ok(())),
        (Change::Nothing, 0, 0)
    );
    assert_eq!(
        follow(&mut grab, Active::NotFollowed, Ok(())),
        (Change::Nothing, 0, 0)
    );
}

#[test]
fn turned_on_while_the_game_is_active_the_key_is_grabbed_at_once() {
    let mut grab = Grab::new(false, Shortcut::default());
    follow(&mut grab, Active::Game, Ok(()));

    assert_eq!(turn(&mut grab, true, Ok(())), (Change::Grabbed, 1, 0));
}

#[test]
fn turned_on_while_another_window_is_active_the_key_waits_for_the_game() {
    let mut grab = Grab::new(false, Shortcut::default());
    follow(&mut grab, Active::Other, Ok(()));

    assert_eq!(turn(&mut grab, true, Ok(())), (Change::Nothing, 0, 0));
    assert_eq!(
        follow(&mut grab, Active::Game, Ok(())),
        (Change::Grabbed, 1, 0)
    );
    // Turned on again: nothing to do.
    assert_eq!(turn(&mut grab, true, Ok(())), (Change::Nothing, 0, 0));
}

#[test]
fn the_key_is_taken_until_it_is_grabbed_or_the_hotkey_turned_off() {
    let mut grab = Grab::default();
    assert!(!grab.taken());

    follow(&mut grab, Active::Game, Err("taken"));
    assert!(grab.taken());
    // Still taken in another window: the player is told until it works.
    follow(&mut grab, Active::Other, Ok(()));
    assert!(grab.taken());
    follow(&mut grab, Active::Game, Ok(()));
    assert!(!grab.taken());

    follow(&mut grab, Active::Other, Ok(()));
    follow(&mut grab, Active::Game, Err("taken"));
    // Never grabbed: nothing to release.
    assert_eq!(turn(&mut grab, false, Ok(())), (Change::Nothing, 0, 0));
    assert!(!grab.taken());
    // Turned on again in the game: tried at once, still taken.
    assert_eq!(turn(&mut grab, true, Err("taken")), (Change::Failed, 1, 0));
    assert!(grab.taken());
}

#[test]
fn the_key_grabbed_for_every_window_is_no_longer_taken() {
    let mut grab = Grab::default();
    follow(&mut grab, Active::Game, Err("taken"));

    assert_eq!(
        follow(&mut grab, Active::NotFollowed, Ok(())),
        (Change::Grabbed, 1, 0)
    );
    assert!(!grab.taken());
}

#[test]
fn turned_on_once_the_active_window_cant_be_followed_the_key_is_grabbed_for_every_window() {
    let mut grab = Grab::new(false, Shortcut::default());
    follow(&mut grab, Active::NotFollowed, Ok(()));

    assert_eq!(turn(&mut grab, true, Ok(())), (Change::Grabbed, 1, 0));
}

#[test]
fn turned_off_the_key_grabbed_for_every_window_is_released() {
    let mut grab = Grab::default();
    follow(&mut grab, Active::NotFollowed, Ok(()));

    assert_eq!(turn(&mut grab, false, Ok(())), (Change::Released, 0, 1));
    assert_eq!(
        follow(&mut grab, Active::NotFollowed, Ok(())),
        (Change::Nothing, 0, 0)
    );
}

#[test]
fn turned_on_again_after_a_failed_release_the_key_held_is_not_grabbed_twice() {
    let mut grab = Grab::default();
    follow(&mut grab, Active::Game, Ok(()));
    turn(&mut grab, false, Err("lost"));

    // Grabbed again, global-hotkey would answer that it is already registered, told as taken.
    assert_eq!(turn(&mut grab, true, Ok(())), (Change::Nothing, 0, 0));
    assert!(!grab.taken());
}

#[test]
fn turned_on_before_any_active_window_is_told_the_key_waits_for_the_game() {
    let mut grab = Grab::new(false, Shortcut::default());

    assert_eq!(turn(&mut grab, true, Ok(())), (Change::Nothing, 0, 0));
    assert_eq!(
        follow(&mut grab, Active::Game, Ok(())),
        (Change::Grabbed, 1, 0)
    );
}

#[test]
fn a_release_failed_when_turned_off_is_tried_again_at_the_next_change() {
    let mut grab = Grab::default();
    follow(&mut grab, Active::Game, Ok(()));

    assert_eq!(turn(&mut grab, false, Err("lost")), (Change::Failed, 0, 1));
    assert_eq!(
        follow(&mut grab, Active::Game, Ok(())),
        (Change::Released, 0, 1)
    );
}

#[test]
fn turned_on_in_front_of_the_game_on_windows_grabs_the_key_at_once() {
    let mut follow = ForegroundFollow::new(false, Shortcut::default());
    follow_foreground(&mut follow, Foreground::Game, false, Ok(()));

    assert_eq!(
        follow.turn(true, || Ok::<(), &str>(()), || Ok(())),
        Change::Grabbed
    );
    assert!(!follow.taken());
}

#[test]
fn tries_a_failed_release_again_at_the_next_change() {
    let mut grab = Grab::default();
    follow(&mut grab, Active::Game, Ok(()));

    assert_eq!(
        follow(&mut grab, Active::Other, Err("lost")),
        (Change::Failed, 0, 1)
    );
    assert_eq!(
        follow(&mut grab, Active::Other, Ok(())),
        (Change::Released, 0, 1)
    );
}

/// What replacing the keys with `keys` does, the grab of the new ones giving `on_grab` and the
/// release of the previous ones `on_release`.
fn change(
    grab: &mut Grab,
    keys: &str,
    on_grab: Result<(), &str>,
    on_release: Result<(), &str>,
) -> (Change, u32, u32) {
    let (mut grabs, mut releases) = (0, 0);
    let change = grab.change_keys(
        keys.parse().unwrap(),
        || {
            grabs += 1;
            on_grab
        },
        || {
            releases += 1;
            on_release
        },
    );
    (change, grabs, releases)
}

#[test]
fn new_keys_in_the_game_replace_the_previous_ones_at_once() {
    let mut grab = Grab::default();
    follow(&mut grab, Active::Game, Ok(()));

    assert_eq!(
        change(&mut grab, "Ctrl+G", Ok(()), Ok(())),
        (Change::Grabbed, 1, 1)
    );
    assert_eq!(grab.keys().to_string(), "Ctrl+G");
    // Released with the new keys when another window becomes the active window.
    assert_eq!(
        follow(&mut grab, Active::Other, Ok(())),
        (Change::Released, 0, 1)
    );
}

#[test]
fn new_keys_chosen_in_another_window_wait_for_the_game() {
    let mut grab = Grab::default();
    follow(&mut grab, Active::Game, Ok(()));
    follow(&mut grab, Active::Other, Ok(()));

    assert_eq!(
        change(&mut grab, "F3", Ok(()), Ok(())),
        (Change::Nothing, 0, 0)
    );
    assert_eq!(
        follow(&mut grab, Active::Game, Ok(())),
        (Change::Grabbed, 1, 0)
    );
}

#[test]
fn the_same_keys_change_nothing() {
    let mut grab = Grab::default();
    follow(&mut grab, Active::Game, Ok(()));

    assert_eq!(
        change(&mut grab, "Ctrl+D", Ok(()), Ok(())),
        (Change::Nothing, 0, 0)
    );
}

#[test]
fn new_keys_are_no_longer_taken_until_they_are_tried() {
    let mut grab = Grab::default();
    follow(&mut grab, Active::Game, Err("taken"));
    follow(&mut grab, Active::Other, Ok(()));
    assert!(grab.taken());

    change(&mut grab, "Alt+G", Ok(()), Ok(()));
    assert!(!grab.taken());
    assert_eq!(
        follow(&mut grab, Active::Game, Err("taken")),
        (Change::Failed, 1, 0)
    );
    assert!(grab.taken());
}

#[test]
fn new_keys_taken_in_the_game_say_so_at_once() {
    let mut grab = Grab::default();
    follow(&mut grab, Active::Game, Err("taken"));

    assert_eq!(
        change(&mut grab, "Ctrl+G", Err("taken"), Ok(())),
        (Change::Failed, 1, 0)
    );
    assert!(grab.taken());
}

#[test]
fn new_keys_of_a_hotkey_off_wait_for_it_to_be_turned_on() {
    let mut grab = Grab::new(false, Shortcut::default());
    follow(&mut grab, Active::Game, Ok(()));

    assert_eq!(
        change(&mut grab, "F4", Ok(()), Ok(())),
        (Change::Nothing, 0, 0)
    );
    assert_eq!(turn(&mut grab, true, Ok(())), (Change::Grabbed, 1, 0));
    assert_eq!(grab.keys().to_string(), "F4");
}

#[test]
fn previous_keys_that_cant_be_released_stay() {
    let mut grab = Grab::default();
    follow(&mut grab, Active::Game, Ok(()));

    assert_eq!(
        change(&mut grab, "Ctrl+G", Ok(()), Err("lost")),
        (Change::Failed, 0, 1)
    );
    assert_eq!(grab.keys().to_string(), "Ctrl+D");
    // Released at the next change, as any failed release.
    assert_eq!(
        follow(&mut grab, Active::Other, Ok(())),
        (Change::Released, 0, 1)
    );
}

#[test]
fn a_later_change_tries_the_previous_keys_that_couldnt_be_released_again() {
    let mut grab = Grab::default();
    follow(&mut grab, Active::Game, Ok(()));
    change(&mut grab, "Ctrl+G", Ok(()), Err("lost"));

    assert_eq!(
        change(&mut grab, "F3", Ok(()), Ok(())),
        (Change::Grabbed, 1, 1)
    );
    assert_eq!(grab.keys().to_string(), "F3");
}

#[test]
fn new_keys_once_the_active_window_cant_be_followed_are_grabbed_for_every_window() {
    let mut grab = Grab::default();
    follow(&mut grab, Active::NotFollowed, Ok(()));

    assert_eq!(
        change(&mut grab, "Alt+G", Ok(()), Ok(())),
        (Change::Grabbed, 1, 1)
    );
    // Held for every window: another window releases nothing.
    assert_eq!(
        follow(&mut grab, Active::NotFollowed, Ok(())),
        (Change::Nothing, 0, 0)
    );
}

#[test]
fn new_keys_while_no_active_window_is_told_wait_for_the_game() {
    let mut grab = Grab::default();
    follow(&mut grab, Active::Game, Ok(()));
    follow(&mut grab, Active::Unknown, Ok(()));

    // The previous keys released, the new ones not grabbed until the game is told.
    assert_eq!(
        change(&mut grab, "F4", Ok(()), Ok(())),
        (Change::Nothing, 0, 1)
    );
    assert_eq!(
        follow(&mut grab, Active::Game, Ok(())),
        (Change::Grabbed, 1, 0)
    );
}

#[test]
fn new_keys_of_a_hotkey_off_release_the_previous_ones_still_held() {
    let mut grab = Grab::default();
    follow(&mut grab, Active::Game, Ok(()));
    assert_eq!(turn(&mut grab, false, Err("lost")), (Change::Failed, 0, 1));

    assert_eq!(
        change(&mut grab, "Ctrl+G", Ok(()), Ok(())),
        (Change::Nothing, 0, 1)
    );
    assert_eq!(turn(&mut grab, true, Ok(())), (Change::Grabbed, 1, 0));
    assert_eq!(grab.keys().to_string(), "Ctrl+G");
}

/// What replacing the keys of `follow` with `keys` does, the grab and the release giving `Ok`.
fn change_foreground(follow: &mut ForegroundFollow, keys: &str) -> (Change, u32, u32) {
    let (mut grabs, mut releases) = (0, 0);
    let change = follow.change_keys(
        keys.parse().unwrap(),
        || {
            grabs += 1;
            Ok::<(), &str>(())
        },
        || {
            releases += 1;
            Ok(())
        },
    );
    (change, grabs, releases)
}

#[test]
fn new_keys_with_the_game_in_front_on_windows_replace_the_previous_ones_at_once() {
    let mut follow = ForegroundFollow::default();
    follow_foreground(&mut follow, Foreground::Game, false, Ok(()));

    assert_eq!(
        change_foreground(&mut follow, "Alt+G"),
        (Change::Grabbed, 1, 1)
    );
    assert_eq!(follow.keys().to_string(), "Alt+G");
}

#[test]
fn new_keys_as_the_foreground_window_is_lost_stay_held_for_every_window() {
    let mut follow = ForegroundFollow::default();
    follow_foreground(&mut follow, Foreground::Game, false, Ok(()));
    follow.lose();

    // Grabbed for the game, the last window followed, then kept for every window.
    assert_eq!(
        change_foreground(&mut follow, "Ctrl+G"),
        (Change::Grabbed, 1, 1)
    );
    assert_eq!(
        follow_foreground(&mut follow, Foreground::Other, false, Ok(())),
        (Change::Nothing, 0, 0)
    );
    assert_eq!(follow.keys().to_string(), "Ctrl+G");
}
