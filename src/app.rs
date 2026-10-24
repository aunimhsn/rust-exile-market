//! The application: the session, the settings, the clipboard watching and the price
//! database, and the window built from the components of [`crate::ui`].

use eframe::{App, CreationContext, Frame};
use egui::{Context, Key, Ui, ViewportCommand};

use crate::clipboard::{ClipboardEvent, ClipboardWatch};
use crate::i18n::{Message, Translator};
use crate::league::League;
use crate::market::MarketWatch;
use crate::session::{LeagueChange, Session};
use crate::settings::{SettingChange, Settings, SettingsFile};
use crate::ui::{
    self, BubbleAction, EmptyState, LeagueList, LeagueListAction, NewLeagueBubble, RawItem,
    ResizeEdges, SettingsPanel, StatusBar, TitleBar, TitleBarAction, UnreadableItem, WarningBand,
};

/// What the window shows under the title bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum View {
    /// The copied item, or the empty state before the first one.
    Item,
    Settings,
}

impl View {
    /// The view after a click on the gear: it opens the settings, and closes them.
    fn toggled(self) -> Self {
        match self {
            Self::Item => Self::Settings,
            Self::Settings => Self::Item,
        }
    }

    /// The view after `event`: a copied item closes the settings, so that the player sees it.
    fn after(self, event: &ClipboardEvent) -> Self {
        match event {
            ClipboardEvent::Copied(_) => Self::Item,
            ClipboardEvent::Unreadable | ClipboardEvent::Readable => self,
        }
    }
}

pub struct ExileMarketApp {
    i18n: Translator,
    session: Session,
    settings: Settings,
    settings_file: SettingsFile,
    view: View,
    /// The list of the leagues is open over the view.
    leagues_open: bool,
    clipboard: ClipboardWatch,
    market: MarketWatch,
}

impl ExileMarketApp {
    /// The application with the settings read from `settings_file`, which the window
    /// already applies.
    #[must_use]
    pub fn new(
        creation: &CreationContext<'_>,
        settings: Settings,
        settings_file: SettingsFile,
    ) -> Self {
        ui::install(&creation.egui_ctx);
        let clipboard_ctx = creation.egui_ctx.clone();
        let market_ctx = creation.egui_ctx.clone();

        Self {
            // The language of the copied items never changes the interface: it only chooses
            // the item names and the trade site.
            i18n: Translator::new(settings.language().language()),
            // The league of the last session, until the leagues arrive.
            session: Session::new(settings.league()),
            settings,
            settings_file,
            view: View::Item,
            leagues_open: false,
            clipboard: ClipboardWatch::start(move || clipboard_ctx.request_repaint()),
            market: MarketWatch::start(move || market_ctx.request_repaint()),
        }
    }

    fn act_on(&mut self, ctx: &Context, action: TitleBarAction) {
        match action {
            TitleBarAction::Move => ctx.send_viewport_cmd(ViewportCommand::StartDrag),
            TitleBarAction::Home => {
                self.view = View::Item;
                self.leagues_open = false;
            }
            TitleBarAction::Leagues => {
                self.leagues_open = !self.leagues_open && self.session.leagues().is_some();
            }
            TitleBarAction::Settings => {
                self.view = self.view.toggled();
                self.leagues_open = false;
            }
            TitleBarAction::Minimize => ctx.send_viewport_cmd(ViewportCommand::Minimized(true)),
            TitleBarAction::Close => ctx.send_viewport_cmd(ViewportCommand::Close),
        }
    }

    /// Shows `league`, chosen by the player, and keeps it in the settings.
    fn choose_league(&mut self, league: League) {
        self.leagues_open = false;
        if self.settings.set_league(&league) {
            self.settings_file.save(&self.settings);
        }
        self.session.choose_league(league);
    }

    /// Closes the bubble of the new league for good.
    fn close_new_league(&mut self) {
        if let Some(league) = self.session.close_new_league()
            && self.settings.close_new_league(&league)
        {
            self.settings_file.save(&self.settings);
        }
    }

    /// Draws the list of the leagues and the bubble of a new league over the window.
    fn league_overlays(&mut self, ctx: &Context, title_bar_action: Option<TitleBarAction>) {
        if let Some(leagues) = self.session.leagues() {
            let list = LeagueList::new(leagues.rows(), self.session.league(), &self.i18n);
            match list.show(ctx, self.leagues_open) {
                Some(LeagueListAction::Choose(league)) => self.choose_league(league),
                // The click on the league block that opened the list isn't outside it.
                Some(LeagueListAction::Close)
                    if title_bar_action != Some(TitleBarAction::Leagues) =>
                {
                    self.leagues_open = false;
                }
                Some(LeagueListAction::Close) | None => {}
            }
        }
        if self.leagues_open || self.view != View::Item {
            return;
        }
        let Some(league) = self.session.new_league().cloned() else {
            return;
        };
        match NewLeagueBubble::new(&league, &self.i18n).show(ctx) {
            Some(BubbleAction::Switch) => self.choose_league(league),
            Some(BubbleAction::Close) => self.close_new_league(),
            None => {}
        }
    }

    /// Applies a change of the settings at once, and saves them.
    fn change_setting(&mut self, ctx: &Context, change: SettingChange) {
        if !self.settings.apply(change) {
            return;
        }
        match change {
            SettingChange::Language(language) => {
                self.i18n = Translator::new(language.language());
            }
            SettingChange::AlwaysOnTop(_) => {
                ctx.send_viewport_cmd(ViewportCommand::WindowLevel(self.settings.window_level()));
            }
        }
        self.settings_file.save(&self.settings);
        // The bands above the settings were drawn with the previous values.
        ctx.request_repaint();
    }

    fn item(&self, ui: &mut Ui) {
        match self.session.item() {
            Some(shown) => match shown.parsed() {
                Ok(_) => RawItem::new(shown.copied()).show(ui),
                Err(_) => UnreadableItem::new(&self.i18n).show(ui),
            },
            None => EmptyState::new(&self.i18n).show(ui),
        }
    }
}

impl App for ExileMarketApp {
    /// Takes in what happened since the last frame; `ui` draws the session, and takes in the
    /// clicks of the components it draws.
    fn logic(&mut self, ctx: &Context, _frame: &mut Frame) {
        self.clipboard.receive(|event| {
            self.view = self.view.after(&event);
            self.session.apply(event);
        });
        let mut shown = None;
        self.market.receive(|event| {
            let closed = self.settings.closed_new_leagues();
            if let Some(LeagueChange::Shown(league)) = self.session.apply_market(event, closed) {
                shown = Some(league);
            }
        });
        if let Some(league) = shown
            && self.settings.set_league(&league)
        {
            self.settings_file.save(&self.settings);
        }
        // Escape closes what is on top: the list of the leagues, then the settings.
        if ctx.input(|input| input.key_pressed(Key::Escape)) {
            if self.leagues_open {
                self.leagues_open = false;
            } else if self.view == View::Settings {
                self.view = View::Item;
            }
        }
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut Frame) {
        let ctx = ui.ctx().clone();
        if let Some(direction) = ResizeEdges::show(&ctx) {
            ctx.send_viewport_cmd(ViewportCommand::BeginResize(direction));
        }
        // The panels on the edges come first, so that the central one takes what remains.
        let action = TitleBar::new(self.session.league(), &self.i18n)
            .settings_open(self.view == View::Settings)
            .leagues_open(self.leagues_open)
            .show(ui);
        if let Some(action) = action {
            self.act_on(&ctx, action);
        }
        self.league_overlays(&ctx, action);
        if !self.session.clipboard_readable() {
            WarningBand::new(self.i18n.text(Message::ClipboardUnreadable)).show(ui);
        }
        match self.view {
            View::Item => StatusBar::sources(&self.i18n).show(ui),
            View::Settings => StatusBar::author(&self.i18n).show(ui),
        }
        match self.view {
            View::Item => self.item(ui),
            View::Settings => {
                if let Some(change) = SettingsPanel::new(&self.settings, &self.i18n).show(ui) {
                    self.change_setting(ui.ctx(), change);
                }
            }
        }
    }

    /// eframe only keeps the size and the position of the window: the state of the widgets
    /// starts again at each launch.
    fn persist_egui_memory(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::CopiedItem;

    #[test]
    fn the_gear_opens_the_settings_then_closes_them() {
        assert_eq!(View::Item.toggled(), View::Settings);
        assert_eq!(View::Settings.toggled(), View::Item);
    }

    #[test]
    fn a_copied_item_closes_the_settings() {
        let copied = ClipboardEvent::Copied(
            CopiedItem::from_text("Item Class: Rings\nRarity: Rare\nWoe Loop\nRuby Ring").unwrap(),
        );

        assert_eq!(View::Settings.after(&copied), View::Item);
        assert_eq!(View::Item.after(&copied), View::Item);
    }

    #[test]
    fn the_state_of_the_clipboard_keeps_the_view() {
        for event in [ClipboardEvent::Unreadable, ClipboardEvent::Readable] {
            assert_eq!(View::Settings.after(&event), View::Settings);
            assert_eq!(View::Item.after(&event), View::Item);
        }
    }
}
