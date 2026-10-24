//! The warning band under the title bar: what doesn't work, in one sentence.

use egui::{Frame, Label, Panel, RichText, Ui, vec2};

use super::theme::{color, size, text};

/// The space above and under the text.
const PADDING_Y: f32 = 10.0;

pub struct WarningBand {
    text: String,
}

impl WarningBand {
    #[must_use]
    pub fn new(text: String) -> Self {
        Self { text }
    }

    /// Shown after the title bar, so that it sits right under it.
    pub fn show(self, ui: &mut Ui) {
        Panel::top("warning-band")
            .resizable(false)
            .frame(
                Frame::NONE
                    .fill(color::WARNING_BAND)
                    .inner_margin(vec2(size::BAND_PADDING, PADDING_Y)),
            )
            .show(ui, |ui| {
                ui.add(
                    Label::new(
                        RichText::new(self.text)
                            .font(text::warning())
                            .color(color::WARNING_TEXT),
                    )
                    .wrap(),
                );
            });
    }
}
