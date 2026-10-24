//! The title bar, in place of the one of the system: the emblem, the league block, then the
//! settings, minimize and close icons on the right. Its empty space moves the window.

use egui::{
    Align, Color32, CursorIcon, Frame, Image, Layout, Painter, Panel, Pos2, Rect, Response, Role,
    Sense, Stroke, Ui, Vec2, WidgetInfo, pos2, vec2,
};

use super::theme::{LeagueModeColors, color, size, text};
use super::{images, widgets};
use crate::i18n::{Message, Translator};
use crate::league::{League, LeagueMode};

const PADDING_LEFT: f32 = 12.0;
const EMBLEM_SIZE: f32 = 16.0;
const EMBLEM_GAP: f32 = 10.0;
const SETTINGS_ICON_SIZE: f32 = 15.0;
/// The width of the minimize line and of the close cross.
const WINDOW_MARK_SIZE: f32 = 9.0;

/// The inside of the league block.
mod league_block {
    pub const LINE_WIDTH: f32 = 2.0;
    pub const PADDING_LEFT: f32 = 10.0;
    pub const PADDING_RIGHT: f32 = 8.0;
    pub const GAP: f32 = 6.0;
    pub const CHEVRON_WIDTH: f32 = 8.0;
    /// Everything but the name, across the block.
    pub const FRAME_WIDTH: f32 = PADDING_LEFT + GAP + CHEVRON_WIDTH + PADDING_RIGHT;
}

/// What the player did in the title bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TitleBarAction {
    /// Started to drag its empty space, to move the window.
    Move,
    Leagues,
    Settings,
    Minimize,
    Close,
}

/// An icon on the right of the title bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Icon {
    Settings,
    Minimize,
    Close,
}

pub struct TitleBar<'a> {
    league: &'a League,
    i18n: &'a Translator,
    settings_open: bool,
}

impl<'a> TitleBar<'a> {
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

    /// Draws the title bar; what the player did in it, if anything.
    pub fn show(self, ui: &mut Ui) -> Option<TitleBarAction> {
        Panel::top("title-bar")
            .exact_size(size::TITLE_BAR_HEIGHT)
            .frame(Frame::NONE.fill(color::BAND_A))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                // Under everything else: the blocks drawn over it take their own clicks.
                let bar = ui.interact(ui.max_rect(), ui.scope_id().with("move"), Sense::drag());
                let mut action = None;
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    ui.add_space(PADDING_LEFT);
                    ui.add(
                        Image::new(images::emblem())
                            .fit_to_exact_size(Vec2::splat(EMBLEM_SIZE))
                            .tint(color::ACCENT),
                    );
                    ui.add_space(EMBLEM_GAP);
                    let icons_width = 3.0 * size::ICON_ZONE;
                    if self.league_block(ui, icons_width).clicked() {
                        action = Some(TitleBarAction::Leagues);
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        for (icon, clicked) in [
                            (Icon::Close, TitleBarAction::Close),
                            (Icon::Minimize, TitleBarAction::Minimize),
                            (Icon::Settings, TitleBarAction::Settings),
                        ] {
                            if self.icon(ui, icon).clicked() {
                                action = Some(clicked);
                            }
                        }
                    });
                });
                action.or_else(|| bar.drag_started().then_some(TitleBarAction::Move))
            })
            .inner
    }

    /// The league name on the color of its mode, the only sign of the mode on screen; its
    /// tooltip says it. A name too long for the window is cut with "…", and shown whole in
    /// the tooltip.
    fn league_block(&self, ui: &mut Ui, room_on_the_right: f32) -> Response {
        use league_block::{CHEVRON_WIDTH, FRAME_WIDTH, GAP, LINE_WIDTH, PADDING_LEFT};

        let colors = LeagueModeColors::of(self.league.mode());
        let max_name_width = (ui.available_width() - room_on_the_right - FRAME_WIDTH).max(0.0);
        let name = widgets::truncated_line(ui, self.league.name(), text::league(), max_name_width);
        let name_is_cut = name.elided;

        let (rect, response) = ui.allocate_exact_size(
            vec2(FRAME_WIDTH + name.size().x, size::TITLE_BAR_HEIGHT),
            Sense::click(),
        );
        let lit = response.hovered() || response.has_focus();
        let painter = ui.painter();
        painter.rect_filled(
            rect,
            0.0,
            if lit {
                colors.hovered
            } else {
                colors.background
            },
        );
        painter.rect_filled(
            Rect::from_min_size(rect.min, vec2(LINE_WIDTH, rect.height())),
            0.0,
            colors.line,
        );
        let name_left = rect.left() + PADDING_LEFT;
        let name_width = name.size().x;
        painter.galley(
            pos2(name_left, rect.center().y - name.size().y / 2.0),
            name,
            color::TEXT,
        );
        widgets::chevron_down(
            painter,
            pos2(
                name_left + name_width + GAP + CHEVRON_WIDTH / 2.0,
                rect.center().y,
            ),
            CHEVRON_WIDTH,
            color::TEXT,
        );

        let mode = self.i18n.text(match self.league.mode() {
            LeagueMode::Softcore => Message::LeagueModeSoftcore,
            LeagueMode::Hardcore => Message::LeagueModeHardcore,
        });
        let tooltip = self.i18n.league_tooltip(&mode);
        response
            .on_hover_cursor(CursorIcon::PointingHand)
            .on_hover_text(if name_is_cut {
                format!("{}\n{tooltip}", self.league.name())
            } else {
                tooltip
            })
    }

    /// An icon without a block, clickable on its zone across the height of the bar; white
    /// under the pointer, and the gear while the settings are open.
    fn icon(&self, ui: &mut Ui, icon: Icon) -> Response {
        let (rect, response) = ui.allocate_exact_size(
            vec2(size::ICON_ZONE, size::TITLE_BAR_HEIGHT),
            Sense::click(),
        );
        let on = icon == Icon::Settings && self.settings_open;
        let tooltip = self.i18n.text(match icon {
            Icon::Settings => Message::SettingsTooltip,
            Icon::Minimize => Message::MinimizeTooltip,
            Icon::Close => Message::CloseTooltip,
        });
        response.widget_info(|| match icon {
            Icon::Settings => WidgetInfo::selected(Role::Button, true, on, &tooltip),
            Icon::Minimize | Icon::Close => WidgetInfo::labeled(Role::Button, true, &tooltip),
        });

        let tint = if on || response.hovered() || response.has_focus() {
            color::ACCENT
        } else {
            color::WEAK
        };
        let center = rect.center();
        match icon {
            Icon::Settings => Image::new(images::settings()).tint(tint).paint_at(
                ui,
                Rect::from_center_size(center, Vec2::splat(SETTINGS_ICON_SIZE)),
            ),
            Icon::Minimize => minimize_mark(ui.painter(), center, tint),
            Icon::Close => close_mark(ui.painter(), center, tint),
        }
        response
            .on_hover_cursor(CursorIcon::PointingHand)
            .on_hover_text(tooltip)
    }
}

/// A short horizontal line.
fn minimize_mark(painter: &Painter, center: Pos2, color: Color32) {
    let half = WINDOW_MARK_SIZE / 2.0;
    painter.line_segment(
        [center - vec2(half, 0.0), center + vec2(half, 0.0)],
        Stroke::new(size::ICON_STROKE, color),
    );
}

/// A cross, a little smaller than the minimize line so that both look the same size.
fn close_mark(painter: &Painter, center: Pos2, color: Color32) {
    let half = WINDOW_MARK_SIZE * 0.4;
    let stroke = Stroke::new(size::ICON_STROKE, color);
    painter.line_segment(
        [center - Vec2::splat(half), center + Vec2::splat(half)],
        stroke,
    );
    painter.line_segment(
        [center + vec2(-half, half), center + vec2(half, -half)],
        stroke,
    );
}
