//! The header: the emblem and the name on the left; the league and the settings, as flat
//! blocks glued to the right edge.

use egui::{Align, Frame, Image, Layout, Panel, Rect, Response, Sense, Ui, Vec2, pos2, vec2};

use super::theme::{LeagueModeColors, color, size, text};
use super::{images, widgets};
use crate::i18n::{Message, Translator};
use crate::league::{League, LeagueMode};

const EMBLEM_SIZE: f32 = 30.0;
const BRAND_GAP: f32 = 10.0;
const SETTINGS_ICON_SIZE: f32 = 17.0;
/// Space between the league block and the settings block.
const BLOCK_GAP: f32 = 1.0;
/// The least space kept between the brand and the league block.
const MIN_BRAND_SPACE: f32 = size::BAND_PADDING;

/// The inside of the league block.
mod league_block {
    pub const LINE_WIDTH: f32 = 3.0;
    pub const PADDING_LEFT: f32 = 14.0;
    pub const PADDING_RIGHT: f32 = 12.0;
    pub const GAP: f32 = 10.0;
    pub const LINE_GAP: f32 = 2.0;
    pub const CHEVRON_WIDTH: f32 = 9.0;
    /// Everything but the text, across the block.
    pub const FRAME_WIDTH: f32 = PADDING_LEFT + GAP + CHEVRON_WIDTH + PADDING_RIGHT;
}

/// The block of the header the player clicked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderAction {
    Leagues,
    Settings,
}

pub struct Header<'a> {
    league: &'a League,
    i18n: &'a Translator,
    settings_open: bool,
}

impl<'a> Header<'a> {
    #[must_use]
    pub fn new(league: &'a League, i18n: &'a Translator) -> Self {
        Self {
            league,
            i18n,
            settings_open: false,
        }
    }

    /// Lights the gear up while the settings are open.
    #[must_use]
    pub fn settings_open(mut self, open: bool) -> Self {
        self.settings_open = open;
        self
    }

    /// Draws the header; the block the player clicked, if any.
    pub fn show(self, ui: &mut Ui) -> Option<HeaderAction> {
        Panel::top("header")
            .exact_size(size::HEADER_HEIGHT)
            .frame(Frame::NONE.fill(color::BAND_A))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    ui.add_space(size::BAND_PADDING);
                    Self::brand(ui);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let settings = self.settings_block(ui).clicked();
                        ui.add_space(BLOCK_GAP);
                        let leagues = self.league_block(ui).clicked();
                        if settings {
                            Some(HeaderAction::Settings)
                        } else {
                            leagues.then_some(HeaderAction::Leagues)
                        }
                    })
                    .inner
                })
                .inner
            })
            .inner
    }

    /// The emblem and the name of the application.
    fn brand(ui: &mut Ui) {
        ui.add(
            Image::new(images::emblem())
                .fit_to_exact_size(Vec2::splat(EMBLEM_SIZE))
                .tint(color::ACCENT),
        );
        ui.add_space(BRAND_GAP);

        let name = widgets::text_line(ui, crate::APP_NAME, text::brand());
        let (rect, _) = ui.allocate_exact_size(name.size(), Sense::hover());
        // Optical alignment: the lowercase letters weigh low, so the name goes up 1 point.
        ui.painter()
            .galley(rect.min - vec2(0.0, 1.0), name, color::ACCENT);
    }

    /// The league name and its mode, on the color of the mode. A name too long for the
    /// window is cut with "…", and shown whole in the tooltip.
    fn league_block(&self, ui: &mut Ui) -> Response {
        use league_block::{
            CHEVRON_WIDTH, FRAME_WIDTH, LINE_GAP, LINE_WIDTH, PADDING_LEFT, PADDING_RIGHT,
        };

        let colors = LeagueModeColors::of(self.league.mode());
        let mode = match self.league.mode() {
            LeagueMode::Softcore => Message::LeagueModeSoftcore,
            LeagueMode::Hardcore => Message::LeagueModeHardcore,
        };
        let label = widgets::spaced_line(
            ui,
            self.i18n.text(mode).to_uppercase(),
            text::league_label(),
            text::LABEL_LETTER_SPACING,
        );
        let max_name_width = (ui.available_width() - MIN_BRAND_SPACE - FRAME_WIDTH).max(0.0);
        let name = widgets::truncated_line(ui, self.league.name(), text::body(), max_name_width);
        let name_is_cut = name.elided;

        let text_width = label.size().x.max(name.size().x);
        let response = widgets::flat_block(
            ui,
            vec2(FRAME_WIDTH + text_width, size::HEADER_HEIGHT),
            colors.background,
        );

        let rect = response.rect;
        let painter = ui.painter();
        painter.rect_filled(
            Rect::from_min_size(rect.min, vec2(LINE_WIDTH, rect.height())),
            0.0,
            colors.line,
        );
        let text_height = label.size().y + LINE_GAP + name.size().y;
        let left = rect.left() + PADDING_LEFT;
        let top = rect.center().y - text_height / 2.0;
        let name_top = top + label.size().y + LINE_GAP;
        painter.galley(pos2(left, top), label, colors.line);
        painter.galley(pos2(left, name_top), name, color::TEXT);
        widgets::chevron_down(
            painter,
            pos2(
                rect.right() - PADDING_RIGHT - CHEVRON_WIDTH / 2.0,
                rect.center().y,
            ),
            CHEVRON_WIDTH,
            color::WEAK,
        );

        let tooltip = self.i18n.text(Message::LeagueTooltip);
        response.on_hover_text(if name_is_cut {
            format!("{}\n{tooltip}", self.league.name())
        } else {
            tooltip
        })
    }

    /// The gear, which opens and closes the settings: on the accent while they are open.
    fn settings_block(&self, ui: &mut Ui) -> Response {
        let background = if self.settings_open {
            color::ACCENT
        } else {
            color::CONTROL
        };
        let response = widgets::flat_block(
            ui,
            vec2(size::ACTION_BLOCK, size::HEADER_HEIGHT),
            background,
        );
        let tint = if self.settings_open {
            color::ON_ACCENT
        } else if response.hovered() {
            color::TEXT
        } else {
            color::WEAK
        };
        Image::new(images::settings()).tint(tint).paint_at(
            ui,
            Rect::from_center_size(response.rect.center(), Vec2::splat(SETTINGS_ICON_SIZE)),
        );

        response.on_hover_text(self.i18n.text(Message::SettingsTooltip))
    }
}
