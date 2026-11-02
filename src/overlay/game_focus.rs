//! The focus given back to the game after a click in the window over the game, on Windows
//! (`specs.md`, Window over the game, Focus).
//!
//! winit can't make a window that is never activated (`WS_EX_NOACTIVATE`), and `unsafe` is
//! forbidden in the application: a click in the window over the game activates it, and the game
//! loses the keyboard. The window then gives the focus back to the game, as Exiled Exchange 2
//! does (`focusTarget` of electron-overlay-window), once the buttons are released, so that the
//! window still gets the release of its click (a checkbox, Search). A click that opens a link
//! (Check on trade) leaves the foreground to the browser: Windows lets the browser come to the
//! front only while this application has the foreground.
//!
//! Without the system, so that the rules are tested on Linux too: the windows come from a
//! [`Desktop`].

/// The windows of the desktop, or fake ones in the tests.
pub trait Desktop {
    type Window;
    /// The foreground window, when it isn't a window of this application.
    fn foreground_not_ours(&mut self) -> Option<Self::Window>;
    /// Brings `window` to the foreground; `false` when it is gone, or the system refused.
    fn activate(&mut self, window: &Self::Window) -> bool;
}

/// What a frame of the window over the game did with the focus, for the tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gave {
    /// Nothing to give back: the window doesn't have the focus, or gave it back already, or a
    /// button is still held in it.
    Nothing,
    /// Given back to the game.
    Given,
    /// Left to the browser, which the click opened.
    Left,
    /// The system refused it, or the game is gone, or was never seen.
    Failed,
}

/// Gives the focus back to the game each time a click in the window over the game takes it.
pub struct GameFocus<D: Desktop> {
    desktop: D,
    /// The foreground window while the window over the game showed without the focus: the game,
    /// where the hotkey was pressed.
    game: Option<D::Window>,
    /// The window over the game took the focus, and it was given back already (or failed).
    handled: bool,
}

impl<D: Desktop> GameFocus<D> {
    pub fn new(desktop: D) -> Self {
        Self {
            desktop,
            game: None,
            handled: false,
        }
    }

    /// At each frame of the window over the game, once its content is drawn: `focused`,
    /// whether it has the focus, none while it is hidden; `pressed`, whether a button of the
    /// mouse is held in it; `opens_link`, whether the frame opens a link in the browser.
    pub fn follow(&mut self, focused: Option<bool>, pressed: bool, opens_link: bool) -> Gave {
        match focused {
            None => {
                self.handled = false;
                Gave::Nothing
            }
            Some(false) => {
                self.handled = false;
                if let Some(window) = self.desktop.foreground_not_ours() {
                    self.game = Some(window);
                }
                Gave::Nothing
            }
            Some(true) if self.handled || pressed => Gave::Nothing,
            Some(true) if opens_link => {
                self.handled = true;
                Gave::Left
            }
            Some(true) => {
                self.handled = true;
                let given = self
                    .game
                    .as_ref()
                    .is_some_and(|game| self.desktop.activate(game));
                if given {
                    log::info!("The window over the game gave the focus back to the game");
                    Gave::Given
                } else {
                    log::warn!(
                        "The window over the game can't give the focus back to the game: a click \
                         in the game takes it back"
                    );
                    Gave::Failed
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A desktop whose foreground window the test sets, `0` for a window of this application.
    #[derive(Default)]
    struct FakeDesktop {
        foreground: u32,
        activated: Vec<u32>,
        refuses: bool,
    }

    impl Desktop for FakeDesktop {
        type Window = u32;

        fn foreground_not_ours(&mut self) -> Option<u32> {
            (self.foreground != 0).then_some(self.foreground)
        }

        fn activate(&mut self, window: &u32) -> bool {
            if self.refuses {
                return false;
            }
            self.activated.push(*window);
            self.foreground = *window;
            true
        }
    }

    const GAME: u32 = 7;
    const OURS: u32 = 0;

    /// The window over the game opened over the game, which has the focus.
    fn opened() -> GameFocus<FakeDesktop> {
        let mut focus = GameFocus::new(FakeDesktop {
            foreground: GAME,
            ..FakeDesktop::default()
        });
        assert_eq!(focus.follow(Some(false), false, false), Gave::Nothing);
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

        assert_eq!(focus.follow(Some(true), true, false), Gave::Nothing);
        assert_eq!(focus.follow(Some(true), false, false), Gave::Given);
        assert_eq!(focus.desktop.activated, [GAME]);
    }

    #[test]
    fn gives_it_back_once_per_click() {
        let mut focus = opened();
        click(&mut focus);
        focus.follow(Some(true), false, false);
        // The frames before the system tells the window it lost the focus.
        assert_eq!(focus.follow(Some(true), false, false), Gave::Nothing);
        assert_eq!(focus.follow(Some(false), false, false), Gave::Nothing);

        click(&mut focus);
        assert_eq!(focus.follow(Some(true), false, false), Gave::Given);
        assert_eq!(focus.desktop.activated, [GAME, GAME]);
    }

    #[test]
    fn a_window_of_this_application_is_never_taken_for_the_game() {
        let mut focus = opened();
        // The main window brought to the front, then a click in the window over the game.
        focus.desktop.foreground = OURS;
        focus.follow(Some(false), false, false);

        assert_eq!(focus.follow(Some(true), false, false), Gave::Given);
        assert_eq!(focus.desktop.activated, [GAME]);
    }

    #[test]
    fn gives_it_to_the_last_window_that_had_it() {
        let mut focus = opened();
        // Alt+Tab to a browser while the window over the game stays open.
        focus.desktop.foreground = 9;
        focus.follow(Some(false), false, false);
        click(&mut focus);

        assert_eq!(focus.follow(Some(true), false, false), Gave::Given);
        assert_eq!(focus.desktop.activated, [9]);
    }

    #[test]
    fn without_game_seen_nothing_is_given() {
        let mut focus = GameFocus::new(FakeDesktop::default());

        assert_eq!(focus.follow(Some(true), false, false), Gave::Failed);
        assert_eq!(focus.follow(Some(true), false, false), Gave::Nothing);
    }

    #[test]
    fn a_refusal_is_tried_again_at_the_next_click_only() {
        let mut focus = opened();
        focus.desktop.refuses = true;
        click(&mut focus);

        assert_eq!(focus.follow(Some(true), false, false), Gave::Failed);
        assert_eq!(focus.follow(Some(true), false, false), Gave::Nothing);
        focus.desktop.refuses = false;
        // A click in the game gave it the focus back; the next click in the window takes it.
        focus.desktop.foreground = GAME;
        focus.follow(Some(false), false, false);
        click(&mut focus);
        assert_eq!(focus.follow(Some(true), false, false), Gave::Given);
    }

    #[test]
    fn hidden_it_gives_nothing_and_shown_again_it_gives_it_back() {
        let mut focus = opened();
        click(&mut focus);
        focus.follow(Some(true), false, false);

        assert_eq!(focus.follow(None, false, false), Gave::Nothing);
        // Opened again by the hotkey, then clicked before any frame saw it without the focus.
        assert_eq!(focus.follow(Some(true), false, false), Gave::Given);
    }

    #[test]
    fn a_click_that_opens_a_link_leaves_the_focus_to_the_browser() {
        let mut focus = opened();
        click(&mut focus);

        assert_eq!(focus.follow(Some(true), false, true), Gave::Left);
        assert_eq!(focus.follow(Some(true), false, false), Gave::Nothing);
        assert_eq!(focus.desktop.activated, [] as [u32; 0]);
        // Back in the game, the next click in the window gives the focus back again.
        focus.desktop.foreground = GAME;
        focus.follow(Some(false), false, false);
        click(&mut focus);
        assert_eq!(focus.follow(Some(true), false, false), Gave::Given);
    }
}
