//! The leagues over the window: the list the player chooses one in, the bubble of a new league,
//! and the bubble of an update, which the player closes for the league of the moment.

use egui::{Context, OpenUrl};

use super::open_lists::OpenList;
use super::{ExileMarketApp, View};
use crate::league::League;
use crate::ui::{Bubble, BubbleAction, LeagueList, LeagueListAction, TitleBarAction};
use crate::update::{LeagueOfTheMoment, Release};

impl ExileMarketApp {
    /// Shows `league`, chosen by the player, and keeps it in the settings. Choosing answers
    /// the bubble of a new league too: it doesn't come back for that league.
    fn choose_league(&mut self, league: League) {
        self.lists.close(OpenList::Leagues);
        let mut changed = self.settings.set_league(&league);
        if let Some(offered) = self.session.close_new_league() {
            changed |= self.settings.close_new_league(&offered);
        }
        if changed {
            self.settings_file.save(&self.settings);
        }
        // The offers shown were searched in the previous league.
        if self.session.league() != Some(&league) {
            self.forget_offers();
        }
        self.session.choose_league(league);
        self.market.price(self.session.league());
    }

    /// Closes the bubble of the new league for good.
    fn close_new_league(&mut self) {
        if let Some(league) = self.session.close_new_league()
            && self.settings.close_new_league(&league)
        {
            self.settings_file.save(&self.settings);
        }
    }

    /// The newer release to offer in a bubble: not once its page was opened in this session,
    /// nor while the player closed its bubble during the league of the moment.
    fn offered_release(&self) -> Option<&Release> {
        let league = LeagueOfTheMoment::of(self.session.leagues());
        self.release.as_ref().filter(|release| {
            !self.release_opened
                && !self
                    .settings
                    .closed_update()
                    .is_some_and(|closed| closed.hides(release, league))
        })
    }

    /// Closes the bubble of the update for that version during the league of the moment.
    fn close_update(&mut self) {
        let Some(release) = &self.release else {
            return;
        };
        let league = LeagueOfTheMoment::of(self.session.leagues());
        if self.settings.close_update(release, league) {
            self.settings_file.save(&self.settings);
        }
    }

    /// Gives an update closed before the leagues were known the league of the moment, once
    /// known.
    pub(super) fn settle_closed_update(&mut self) {
        let league = LeagueOfTheMoment::of(self.session.leagues());
        if self.settings.settle_closed_update(league) {
            self.settings_file.save(&self.settings);
        }
    }

    /// Draws the list of the leagues, and the bubbles of a new league and of an update, over
    /// the window.
    pub(super) fn league_overlays(
        &mut self,
        ctx: &Context,
        title_bar_action: Option<TitleBarAction>,
    ) {
        if let Some(leagues) = self.session.leagues() {
            let list = LeagueList::new(leagues.rows(), self.session.league(), &self.i18n);
            match list.show(ctx, self.lists.is_open(OpenList::Leagues)) {
                Some(LeagueListAction::Choose(league)) => self.choose_league(league),
                // The click on the league block that opened the list isn't outside it.
                Some(LeagueListAction::Close)
                    if title_bar_action != Some(TitleBarAction::Leagues) =>
                {
                    self.lists.close(OpenList::Leagues);
                }
                Some(LeagueListAction::Close) | None => {}
            }
        }
        // Once the list is gone: the bubble would show through it while it fades out.
        if LeagueList::visibility(ctx, self.lists.is_open(OpenList::Leagues)) > 0.0
            || self.view != View::Item
        {
            return;
        }
        let mut bottom = None;
        if let Some(league) = self.session.new_league().cloned() {
            let shown = Bubble::new_league(&league, &self.i18n).show(ctx);
            bottom = Some(shown.bottom);
            match shown.action {
                Some(BubbleAction::Pill) => self.choose_league(league),
                Some(BubbleAction::Close) => self.close_new_league(),
                None => {}
            }
        }
        // Under the bubble of a new league, aligned on its left.
        let Some(release) = self.offered_release().cloned() else {
            return;
        };
        let mut bubble = Bubble::update(release.version, &self.i18n);
        if let Some(bottom) = bottom {
            bubble = bubble.below(bottom);
        }
        match bubble.show(ctx).action {
            Some(BubbleAction::Pill) => {
                ctx.open_url(OpenUrl::new_tab(&release.page));
                self.release_opened = true;
            }
            Some(BubbleAction::Close) => self.close_update(),
            None => {}
        }
    }
}
