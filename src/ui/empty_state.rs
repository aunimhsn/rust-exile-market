//! The window before the first item: the logo and how to use the application.

use egui::{
    Align2, CentralPanel, Frame, Image, Rect, RichText, Sense, Stroke, StrokeKind, Ui, pos2, vec2,
};

use super::theme::{color, size, text};
use super::{images, widgets};
use crate::i18n::{Message, Translator};

const LOGO_WIDTH: f32 = 150.0;
const KEY_HEIGHT: f32 = 26.0;
const KEY_PADDING: f32 = 9.0;
const KEY_GAP: f32 = 8.0;
const SPACE: f32 = 14.0;
/// The keys to press in the game.
const KEYS: [&str; 2] = ["Ctrl", "C"];

pub struct EmptyState<'a> {
    i18n: &'a Translator,
}

impl<'a> EmptyState<'a> {
    #[must_use]
    pub fn new(i18n: &'a Translator) -> Self {
        Self { i18n }
    }

    pub fn show(self, ui: &mut Ui) {
        CentralPanel::no_frame()
            .frame(Frame::NONE.fill(color::BAND_B))
            .show(ui, |ui| {
                // Centered vertically with the height of the content at the previous frame:
                // egui only knows the size of what it has laid out.
                let height_id = ui.scope_id().with("content-height");
                let height = ui.data(|data| data.get_temp::<f32>(height_id));
                ui.add_space(((ui.available_height() - height.unwrap_or(0.0)) / 2.0).max(0.0));
                let content =
                    ui.vertical_centered(|ui| {
                        ui.add(Image::new(images::logo()).fit_to_exact_size(
                            images::LOGO_SIZE * (LOGO_WIDTH / images::LOGO_SIZE.x),
                        ));
                        ui.add_space(SPACE * 1.5);
                        self.hint(ui, Message::EmptyCopy);
                        ui.add_space(SPACE / 2.0);
                        Self::keys(ui);
                        ui.add_space(SPACE / 2.0);
                        self.hint(ui, Message::EmptyPrice);
                    });
                let content_height = content.response.rect.height();
                if height != Some(content_height) {
                    ui.data_mut(|data| data.insert_temp(height_id, content_height));
                    // The first frame isn't centered yet: draw the next one at once.
                    ui.ctx()
                        .request_discard("the height of the empty state changed");
                }
            });
    }

    fn hint(&self, ui: &mut Ui, message: Message) {
        ui.label(
            RichText::new(self.i18n.text(message))
                .font(text::body())
                .color(color::WEAK),
        );
    }

    /// The keys drawn as key caps, joined by "+".
    fn keys(ui: &mut Ui) {
        let caps: Vec<_> = KEYS
            .iter()
            .map(|&key| widgets::text_line(ui, key, text::keys()))
            .collect();
        let plus = widgets::text_line(ui, "+", text::keys());

        // Each key cap, and a "+" between two of them.
        let caps_width: f32 = caps
            .iter()
            .map(|cap| cap.size().x + 2.0 * KEY_PADDING)
            .sum();
        let separators_width: f32 = caps
            .iter()
            .skip(1)
            .map(|_| plus.size().x + 2.0 * KEY_GAP)
            .sum();
        let width = caps_width + separators_width;
        let (rect, _) = ui.allocate_exact_size(vec2(width, KEY_HEIGHT), Sense::hover());

        let painter = ui.painter();
        let mut left = rect.left();
        for (index, cap) in caps.into_iter().enumerate() {
            if index > 0 {
                left += KEY_GAP;
                painter.galley(
                    pos2(left, rect.center().y - plus.size().y / 2.0),
                    plus.clone(),
                    color::WEAK,
                );
                left += plus.size().x + KEY_GAP;
            }
            let key = Rect::from_min_size(
                pos2(left, rect.top()),
                vec2(cap.size().x + 2.0 * KEY_PADDING, KEY_HEIGHT),
            );
            painter.rect_filled(key, f32::from(size::RADIUS), color::CONTROL);
            painter.rect_stroke(
                key,
                f32::from(size::RADIUS),
                Stroke::new(1.0, color::CONTROL_LINE),
                StrokeKind::Inside,
            );
            painter.galley(
                Align2::CENTER_CENTER
                    .anchor_size(key.center(), cap.size())
                    .min,
                cap,
                color::TEXT,
            );
            left = key.right();
        }
    }
}
