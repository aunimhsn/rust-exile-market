//! The application: the session, the settings and the clipboard watching, and the window
//! built from the components of [`crate::ui`].

use eframe::{App, CreationContext, Frame};
use egui::{Context, Key, Ui, ViewportCommand};

use crate::clipboard::{ClipboardEvent, ClipboardWatch};
use crate::i18n::{Message, Translator};
use crate::league::{League, LeagueMode};
use crate::session::Session;
use crate::settings::{SettingChange, Settings, SettingsFile};
use crate::ui::{
    self, EmptyState, Header, HeaderAction, RawItem, SettingsPanel, StatusBar, UnreadableItem,
    WarningBand,
};

/// What the window shows under the header.
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
    clipboard: ClipboardWatch,
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
        let ctx = creation.egui_ctx.clone();

        Self {
            // The language of the copied items never changes the interface: it only chooses
            // the item names and the trade site.
            i18n: Translator::new(settings.language().language()),
            // The leagues are downloaded from the price database in step 7; until then, the
            // current challenge league in softcore.
            session: Session::new(League::new("Forbidden Rites", LeagueMode::Softcore)),
            settings,
            settings_file,
            view: View::Item,
            clipboard: ClipboardWatch::start(move || ctx.request_repaint()),
        }
    }

    fn act_on(&mut self, action: HeaderAction) {
        match action {
            // The list of the leagues comes with the price data, in step 7.
            HeaderAction::Leagues => {}
            HeaderAction::Settings => self.view = self.view.toggled(),
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
        if self.view == View::Settings && ctx.input(|input| input.key_pressed(Key::Escape)) {
            self.view = View::Item;
        }
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut Frame) {
        // The panels on the edges come first, so that the central one takes what remains.
        let action = Header::new(self.session.league(), &self.i18n)
            .settings_open(self.view == View::Settings)
            .show(ui);
        if let Some(action) = action {
            self.act_on(action);
        }
        if !self.session.clipboard_readable() {
            WarningBand::new(self.i18n.text(Message::ClipboardUnreadable)).show(ui);
        }
        StatusBar::new(&self.i18n).show(ui);
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
