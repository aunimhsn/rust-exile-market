//! The window before the first item, over the emblem as a faint watermark, which takes no
//! height: how to use the application, or, while no price of the league is known, the
//! download of the prices.

use egui::text::{LayoutJob, TextFormat};
use egui::{
    Align2, CentralPanel, Frame, Image, Rect, RichText, Sense, Stroke, StrokeKind, Ui, Vec2, pos2,
    vec2,
};

use super::theme::{color, motion, size, text};
use super::{images, widgets};
use crate::i18n::{Message, Translator};

const WATERMARK_SIZE: f32 = 190.0;
const KEY_HEIGHT: f32 = 26.0;
const KEY_PADDING: f32 = 9.0;
const KEY_GAP: f32 = 8.0;
const SPACE: f32 = 14.0;
/// The keys to press in the game.
const KEYS: [&str; 2] = ["Ctrl", "C"];
const BAR_SIZE: Vec2 = vec2(220.0, 4.0);
/// The space between the lines of the download.
const DOWNLOAD_GAP: f32 = 10.0;

/// The download of the prices of a league.
#[derive(Debug, Clone, Copy)]
pub struct PriceDownload<'a> {
    /// The name of the league.
    pub league: &'a str,
    /// The share downloaded, from 0 to 1.
    pub progress: f32,
    /// The sentence under the bar, drawn at random.
    pub quote: usize,
}

pub struct EmptyState<'a> {
    i18n: &'a Translator,
    download: Option<PriceDownload<'a>>,
}

impl<'a> EmptyState<'a> {
    #[must_use]
    pub fn new(i18n: &'a Translator) -> Self {
        Self {
            i18n,
            download: None,
        }
    }

    /// Shows the download of the prices in place of the invitation to copy an item, which
    /// would be wrong while no price is known.
    #[must_use]
    pub fn downloading(mut self, download: PriceDownload<'a>) -> Self {
        self.download = Some(download);
        self
    }

    pub fn show(self, ui: &mut Ui) {
        CentralPanel::no_frame()
            .frame(Frame::NONE.fill(color::BAND_B))
            .show(ui, |ui| {
                Image::new(images::emblem())
                    .tint(color::WATERMARK)
                    .paint_at(
                        ui,
                        Rect::from_center_size(ui.max_rect().center(), Vec2::splat(WATERMARK_SIZE)),
                    );
                // Centered vertically with the height of the content at the previous frame:
                // egui only knows the size of what it has laid out.
                let height_id = ui.scope_id().with("content-height");
                let height = ui.data(|data| data.get_temp::<f32>(height_id));
                ui.add_space(((ui.available_height() - height.unwrap_or(0.0)) / 2.0).max(0.0));
                let content = ui.vertical_centered(|ui| {
                    if let Some(download) = self.download {
                        self.download(ui, download);
                    } else {
                        self.hint(ui, Message::EmptyCopy);
                        ui.add_space(SPACE / 2.0);
                        Self::keys(ui);
                        ui.add_space(SPACE / 2.0);
                        self.hint(ui, Message::EmptyPrice);
                    }
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

    /// The league whose prices are downloaded, in the color of the text, the bar, then a
    /// sentence.
    fn download(&self, ui: &mut Ui, download: PriceDownload<'_>) {
        let title = self.i18n.download_title(download.league);
        let mut job = LayoutJob::default();
        let mut append = |part: &str, tint| {
            job.append(part, 0.0, TextFormat::simple(text::body(), tint));
        };
        match title.split_once(download.league) {
            Some((before, after)) => {
                append(before, color::WEAK);
                append(download.league, color::TEXT);
                append(after, color::WEAK);
            }
            None => append(&title, color::WEAK),
        }
        ui.label(job);
        ui.add_space(DOWNLOAD_GAP);

        let (rect, _) = ui.allocate_exact_size(BAR_SIZE, Sense::hover());
        // From one step to the next without a jump: a step is a request, every second.
        let progress = ui.ctx().animate_value_with_time(
            // Per league: the bar of another league starts from zero.
            ui.scope_id().with(("download-progress", download.league)),
            download.progress.clamp(0.0, 1.0),
            motion::PROGRESS,
        );
        let radius = BAR_SIZE.y / 2.0;
        let painter = ui.painter();
        painter.rect_filled(rect, radius, color::CONTROL);
        painter.rect_filled(
            Rect::from_min_size(rect.min, vec2(rect.width() * progress, rect.height())),
            radius,
            color::TEXT,
        );
        ui.add_space(DOWNLOAD_GAP);

        ui.label(
            RichText::new(self.i18n.download_quote(download.quote))
                .font(text::small())
                .color(color::FAINT),
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
