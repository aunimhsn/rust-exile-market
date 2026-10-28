//! What the window shows when a copied item can't be read, also when it was copied without
//! its details: the owner's game always gives them (step 13k).

use egui::{CentralPanel, Frame, Label, RichText, Ui, vec2};

use super::theme::{color, size, text};
use crate::i18n::{Message, Translator};

/// The space above and under the messages.
const PADDING_Y: f32 = 12.0;
/// The space between the message and the hint.
const HINT_GAP: f32 = 9.0;

pub struct UnreadableItem<'a> {
    i18n: &'a Translator,
}

impl<'a> UnreadableItem<'a> {
    #[must_use]
    pub fn new(i18n: &'a Translator) -> Self {
        Self { i18n }
    }

    pub fn show(self, ui: &mut Ui) {
        CentralPanel::no_frame()
            .frame(
                Frame::NONE
                    .fill(color::BAND_B)
                    .inner_margin(vec2(size::BAND_PADDING, PADDING_Y)),
            )
            .show(ui, |ui| {
                ui.add(
                    Label::new(
                        RichText::new(self.i18n.text(Message::ItemUnreadable))
                            .font(text::message())
                            .color(color::TEXT),
                    )
                    .wrap(),
                );
                ui.add_space(HINT_GAP);
                ui.add(
                    Label::new(
                        RichText::new(self.i18n.text(Message::ItemUnreadableHint))
                            .font(text::body())
                            .color(color::WEAK),
                    )
                    .wrap(),
                );
            });
    }
}
