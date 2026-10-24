//! The application: its state, and the window built from the components of [`crate::ui`].

use eframe::{App, CreationContext, Frame};
use egui::Ui;

use crate::i18n::{Language, Translator};
use crate::league::{League, LeagueMode};
use crate::ui::{self, EmptyState, Header, StatusBar};

pub struct ExileMarketApp {
    i18n: Translator,
    league: League,
}

impl ExileMarketApp {
    #[must_use]
    pub fn new(creation: &CreationContext<'_>) -> Self {
        ui::install(&creation.egui_ctx);

        Self {
            i18n: Translator::new(Language::default()),
            // The leagues are downloaded from the price database in step 6; until then, the
            // current challenge league in softcore.
            league: League::new("Forbidden Rites", LeagueMode::Softcore),
        }
    }
}

impl App for ExileMarketApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut Frame) {
        // The panels on the edges come first, so that the central one takes what remains.
        Header::new(&self.league, &self.i18n).show(ui);
        StatusBar::new(&self.i18n).show(ui);
        EmptyState::new(&self.i18n).show(ui);
    }
}
