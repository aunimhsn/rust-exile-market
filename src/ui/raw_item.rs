//! The text of the last copied item, as the game gave it: what the window shows until the
//! items are parsed and priced.

use egui::{CentralPanel, Frame, Label, RichText, ScrollArea, Ui, vec2};

use super::theme::{color, size, text};
use crate::item::CopiedItem;

pub struct RawItem<'a> {
    item: &'a CopiedItem,
}

impl<'a> RawItem<'a> {
    #[must_use]
    pub fn new(item: &'a CopiedItem) -> Self {
        Self { item }
    }

    pub fn show(self, ui: &mut Ui) {
        CentralPanel::no_frame()
            .frame(
                Frame::NONE
                    .fill(color::BAND_B)
                    .inner_margin(vec2(size::BAND_PADDING, size::BAND_PADDING)),
            )
            .show(ui, |ui| {
                ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                    ui.add(
                        Label::new(
                            RichText::new(self.item.text())
                                .font(text::raw_item())
                                .color(color::TEXT),
                        )
                        .wrap(),
                    );
                });
            });
    }
}
