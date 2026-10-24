//! The application: the session and the clipboard watching, and the window built from the
//! components of [`crate::ui`].

use eframe::{App, CreationContext, Frame};
use egui::{Context, Ui};

use crate::clipboard::ClipboardWatch;
use crate::i18n::{Message, Translator};
use crate::language::Language;
use crate::league::{League, LeagueMode};
use crate::session::Session;
use crate::ui::{self, EmptyState, Header, RawItem, StatusBar, UnreadableItem, WarningBand};

pub struct ExileMarketApp {
    i18n: Translator,
    session: Session,
    clipboard: ClipboardWatch,
}

impl ExileMarketApp {
    #[must_use]
    pub fn new(creation: &CreationContext<'_>) -> Self {
        ui::install(&creation.egui_ctx);
        let ctx = creation.egui_ctx.clone();

        Self {
            // The language of the copied items never changes the interface: it only chooses
            // the item names and the trade site.
            i18n: Translator::new(Language::of_system()),
            // The leagues are downloaded from the price database in step 6; until then, the
            // current challenge league in softcore.
            session: Session::new(League::new("Forbidden Rites", LeagueMode::Softcore)),
            clipboard: ClipboardWatch::start(move || ctx.request_repaint()),
        }
    }
}

impl App for ExileMarketApp {
    /// Takes in what happened since the last frame; `ui` only draws the session.
    fn logic(&mut self, _ctx: &Context, _frame: &mut Frame) {
        self.clipboard.receive(|event| self.session.apply(event));
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut Frame) {
        let session = &self.session;
        // The panels on the edges come first, so that the central one takes what remains.
        Header::new(session.league(), &self.i18n).show(ui);
        if !session.clipboard_readable() {
            WarningBand::new(self.i18n.text(Message::ClipboardUnreadable)).show(ui);
        }
        StatusBar::new(&self.i18n).show(ui);
        match session.item() {
            Some(shown) => match shown.parsed() {
                Ok(_) => RawItem::new(shown.copied()).show(ui),
                Err(_) => UnreadableItem::new(&self.i18n).show(ui),
            },
            None => EmptyState::new(&self.i18n).show(ui),
        }
    }
}
