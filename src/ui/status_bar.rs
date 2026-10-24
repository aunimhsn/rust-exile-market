//! The status bar: the age of each data source.

use egui::{Align, Frame, Label, Layout, Panel, RichText, Ui};

use super::theme::{color, size, text};
use crate::i18n::{Message, Translator};

pub struct StatusBar<'a> {
    i18n: &'a Translator,
}

impl<'a> StatusBar<'a> {
    #[must_use]
    pub fn new(i18n: &'a Translator) -> Self {
        Self { i18n }
    }

    pub fn show(self, ui: &mut Ui) {
        Panel::bottom("status-bar")
            .exact_size(size::STATUS_BAR_HEIGHT)
            .frame(Frame::NONE.fill(color::BAND_A))
            .show(ui, |ui| {
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    // The padding of the band on both sides.
                    ui.set_max_width(ui.available_width() - size::BAND_PADDING);
                    ui.add_space(size::BAND_PADDING);
                    // Cut with "…" in a narrow window, and shown whole under the pointer.
                    ui.add(
                        Label::new(
                            RichText::new(self.text())
                                .font(text::small())
                                .color(color::FAINT),
                        )
                        .truncate(),
                    );
                });
            });
    }

    /// The age of the prices and of the last trade site search; none yet in this version.
    fn text(&self) -> String {
        self.i18n.status_bar(
            &self.i18n.text(Message::StatusPricesWaiting),
            &self.i18n.text(Message::StatusTradeIdle),
        )
    }
}
