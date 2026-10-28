//! What the window shows when a copied item can't be read, or was copied without its details.

use egui::{CentralPanel, Frame, Label, RichText, Ui, vec2};

use super::theme::{color, size, text};
use crate::i18n::{Message, Translator};
use crate::item::ParseError;

/// The space above and under the messages.
const PADDING_Y: f32 = 12.0;
/// The space between the message and the hint.
const HINT_GAP: f32 = 9.0;

pub struct UnreadableItem<'a> {
    i18n: &'a Translator,
    error: ParseError,
}

impl<'a> UnreadableItem<'a> {
    #[must_use]
    pub fn new(i18n: &'a Translator, error: ParseError) -> Self {
        Self { i18n, error }
    }

    pub fn show(self, ui: &mut Ui) {
        let (message, hint) = match self.error {
            ParseError::WithoutDetails => {
                (Message::ItemWithoutDetails, Message::ItemWithoutDetailsHint)
            }
            ParseError::MissingRarity | ParseError::UnknownRarity | ParseError::MissingBaseType => {
                (Message::ItemUnreadable, Message::ItemUnreadableHint)
            }
        };
        CentralPanel::no_frame()
            .frame(
                Frame::NONE
                    .fill(color::BAND_B)
                    .inner_margin(vec2(size::BAND_PADDING, PADDING_Y)),
            )
            .show(ui, |ui| {
                ui.add(
                    Label::new(
                        RichText::new(self.i18n.text(message))
                            .font(text::message())
                            .color(color::TEXT),
                    )
                    .wrap(),
                );
                ui.add_space(HINT_GAP);
                ui.add(
                    Label::new(
                        RichText::new(self.i18n.text(hint))
                            .font(text::body())
                            .color(color::WEAK),
                    )
                    .wrap(),
                );
            });
    }
}
