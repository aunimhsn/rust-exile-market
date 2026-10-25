//! The warning band under the title bar: what doesn't work, one sentence per warning.

use egui::{Frame, Label, Panel, RichText, Ui, vec2};

use super::theme::{color, size, text};

/// The space above and under the text.
const PADDING_Y: f32 = 10.0;
/// The space between two warnings.
const GAP: f32 = 4.0;

pub struct WarningBand {
    warnings: Vec<String>,
}

impl WarningBand {
    #[must_use]
    pub fn new(warnings: Vec<String>) -> Self {
        Self { warnings }
    }

    /// Shown after the title bar, so that it sits right under it; nothing without a warning.
    pub fn show(self, ui: &mut Ui) {
        if self.warnings.is_empty() {
            return;
        }
        Panel::top("warning-band")
            .resizable(false)
            .frame(
                Frame::NONE
                    .fill(color::WARNING_BAND)
                    .inner_margin(vec2(size::BAND_PADDING, PADDING_Y)),
            )
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = GAP;
                for warning in self.warnings {
                    ui.add(
                        Label::new(
                            RichText::new(warning)
                                .font(text::warning())
                                .color(color::WARNING_TEXT),
                        )
                        .wrap(),
                    );
                }
            });
    }
}
