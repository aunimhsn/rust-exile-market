//! The window before the first item, over the emblem as a faint watermark, which takes no
//! height: how to use the application, or, while no price of the league is known, the
//! download of the prices.

use egui::text::TextFormat;
use egui::{CentralPanel, Frame, Image, Rect, RichText, Sense, Ui, Vec2, vec2};

use super::images;
use super::keys::{self, Caps, Emphasis};
use super::theme::{color, motion, text};
use super::widgets;
use crate::hotkey::{HotkeyState, Shortcut};
use crate::i18n::{Message, Translator};

const WATERMARK_SIZE: f32 = 190.0;
const KEY_HEIGHT: f32 = 26.0;
const KEY_PADDING: f32 = 9.0;
const KEY_GAP: f32 = 8.0;
const SPACE: f32 = 14.0;
/// The space above the note on the display mode, and its width (the mock-up).
const NOTE_GAP: f32 = 18.0;
const NOTE_WIDTH: f32 = 330.0;
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
    hotkey: HotkeyState,
    /// The keys of the hotkey.
    keys: Shortcut,
    download: Option<PriceDownload<'a>>,
}

impl<'a> EmptyState<'a> {
    /// The invitation to copy with the price-check hotkey, its `keys`, when it copies (`hotkey`),
    /// otherwise with the copy of the game, so that it never shows a key that does nothing.
    #[must_use]
    pub fn new(i18n: &'a Translator, hotkey: HotkeyState, keys: Shortcut) -> Self {
        Self {
            i18n,
            hotkey,
            keys,
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
                        self.keys(ui);
                        ui.add_space(SPACE / 2.0);
                        self.hint(ui, Message::EmptyPrice);
                        if self.hotkey == HotkeyState::Taken {
                            ui.add_space(NOTE_GAP - ui.spacing().item_spacing.y);
                            self.taken(ui);
                        }
                        ui.add_space(NOTE_GAP - ui.spacing().item_spacing.y);
                        self.note(ui);
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

    /// The display mode the game needs, in faint text on a few centered lines.
    fn note(&self, ui: &mut Ui) {
        ui.scope(|ui| {
            ui.set_max_width(NOTE_WIDTH);
            ui.label(
                RichText::new(self.i18n.text(Message::EmptyDisplayMode))
                    .font(text::note())
                    .color(color::WEAK),
            );
        });
    }

    /// The key of the hotkey taken by another application, in the color of the warnings, its
    /// keys in bold, then on a line of its own where to choose another.
    fn taken(&self, ui: &mut Ui) {
        let hotkey = keys::joined(&keys::named(&self.keys.keys(), self.i18n));
        let mut job = keys::two_lines(
            &self.i18n.hotkey_taken(&hotkey),
            &self.i18n.text(Message::HotkeyTakenChoose),
            &Emphasis {
                strong: &[&hotkey],
                regular: TextFormat::simple(text::note(), color::WARNING),
                bold: TextFormat::simple(text::note_strong(), color::WARNING),
            },
        );
        job.halign = egui::Align::Center;
        ui.scope(|ui| {
            ui.set_max_width(NOTE_WIDTH);
            ui.label(job);
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
        ui.label(widgets::sentence_with(
            &self.i18n.download_title(),
            download.league,
            &TextFormat::simple(text::body(), color::WEAK),
            TextFormat::simple(text::body(), color::TEXT),
        ));
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

    /// The keys to press in the game, drawn as key caps: the price-check hotkey while it
    /// copies, otherwise the copy of the game (macOS, which has no hotkey; the hotkey turned off
    /// or its key taken).
    fn keys(&self, ui: &mut Ui) {
        let pressed = keys::named(&invited_keys(self.hotkey, self.keys), self.i18n);
        let caps = Caps {
            font: text::keys(),
            height: KEY_HEIGHT,
            padding: KEY_PADDING,
            gap: KEY_GAP,
        };
        keys::caps(ui, &pressed, &caps);
    }
}

/// The keys the empty window invites to press: the price-check hotkey while it copies, otherwise
/// the copy of the game, so that it never shows a key that does nothing.
fn invited_keys(hotkey: HotkeyState, keys: Shortcut) -> Vec<&'static str> {
    match hotkey {
        HotkeyState::Listening => keys.keys(),
        HotkeyState::Off | HotkeyState::Taken | HotkeyState::Unavailable => keys::COPY.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_invitation_never_shows_a_key_that_does_nothing() {
        let alt_d = "Alt+D".parse().unwrap();
        assert_eq!(
            invited_keys(HotkeyState::Listening, Shortcut::default()),
            ["Ctrl", "D"]
        );
        assert_eq!(invited_keys(HotkeyState::Listening, alt_d), ["Alt", "D"]);
        for state in [
            HotkeyState::Off,
            HotkeyState::Taken,
            HotkeyState::Unavailable,
        ] {
            assert_eq!(invited_keys(state, alt_d), ["Ctrl", "C"], "{state:?}");
        }
    }
}
