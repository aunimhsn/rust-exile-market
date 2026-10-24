//! The title bar, in place of the one of the system: the emblem, the league block, then the
//! settings, minimize and close icons on the right. Its empty space moves the window.

use egui::{
    Align, Color32, CursorIcon, Frame, Image, Layout, Painter, Panel, PointerButton, Pos2, Rect,
    Response, Role, Sense, Stroke, StrokeKind, Ui, Vec2, WidgetInfo, pos2, vec2,
};

use super::theme::{LeagueModeColors, color, size, text};
use super::{images, widgets};
use crate::i18n::{Message, Translator};
use crate::league::{League, LeagueMode};

const PADDING_LEFT: f32 = 12.0;
const EMBLEM_SIZE: f32 = 16.0;
const EMBLEM_GAP: f32 = 10.0;
/// The ring around the emblem with the keyboard focus: the pointer never shows it.
const EMBLEM_FOCUS_MARGIN: f32 = 3.0;
const SETTINGS_ICON_SIZE: f32 = 15.0;
/// The width of the minimize line and of the close cross.
const WINDOW_MARK_SIZE: f32 = 9.0;

/// The inside of the league block.
mod league_block {
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
    /// Clicked the emblem, to go back to the item.
    Home,
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

/// The icons on the right, from the right edge.
const ICONS: [Icon; 3] = [Icon::Close, Icon::Minimize, Icon::Settings];

impl Icon {
    fn action(self) -> TitleBarAction {
        match self {
            Self::Settings => TitleBarAction::Settings,
            Self::Minimize => TitleBarAction::Minimize,
            Self::Close => TitleBarAction::Close,
        }
    }

    fn tooltip(self) -> Message {
        match self {
            Self::Settings => Message::SettingsTooltip,
            Self::Minimize => Message::MinimizeTooltip,
            Self::Close => Message::CloseTooltip,
        }
    }
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
    pub fn show(&self, ui: &mut Ui) -> Option<TitleBarAction> {
        Panel::top("title-bar")
            .exact_size(size::TITLE_BAR_HEIGHT)
            .frame(Frame::NONE.fill(color::BAND_A))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                // Under everything else. Sensing clicks too, it only starts to drag once the
                // pointer moves, so that a press on a block or an icon above stays a click.
                let bar = ui.interact(
                    ui.max_rect(),
                    ui.scope_id().with("move"),
                    Sense::click_and_drag(),
                );
                let mut action = None;
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    if self.emblem(ui).clicked() {
                        action = Some(TitleBarAction::Home);
                    }
                    #[expect(clippy::cast_precision_loss, reason = "three icons")]
                    let icons_width = ICONS.len() as f32 * size::ICON_ZONE;
                    if self.league_block(ui, icons_width).clicked() {
                        action = Some(TitleBarAction::Leagues);
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        for icon in ICONS {
                            if self.icon(ui, icon).clicked() {
                                action = Some(icon.action());
                            }
                        }
                    });
                });
                action.or_else(|| {
                    bar.drag_started_by(PointerButton::Primary)
                        .then_some(TitleBarAction::Move)
                })
            })
            .inner
    }

    /// The emblem, which goes back to the item, like the logo of a site to its home page;
    /// clickable from the left edge to the league block, across the height of the bar.
    fn emblem(&self, ui: &mut Ui) -> Response {
        let (rect, response) = ui.allocate_exact_size(
            vec2(
                PADDING_LEFT + EMBLEM_SIZE + EMBLEM_GAP,
                size::TITLE_BAR_HEIGHT,
            ),
            Sense::click(),
        );
        let tooltip = self.i18n.text(Message::HomeTooltip);
        response.widget_info(|| WidgetInfo::labeled(Role::Button, true, &tooltip));
        let emblem = Rect::from_center_size(
            pos2(
                rect.left() + PADDING_LEFT + EMBLEM_SIZE / 2.0,
                rect.center().y,
            ),
            Vec2::splat(EMBLEM_SIZE),
        );
        Image::new(images::emblem())
            .tint(color::ACCENT)
            .paint_at(ui, emblem);
        if response.has_focus() {
            ui.painter().rect_stroke(
                emblem.expand(EMBLEM_FOCUS_MARGIN),
                f32::from(size::RADIUS),
                Stroke::new(1.0, color::WEAK),
                StrokeKind::Outside,
            );
        }
        response
            .on_hover_cursor(CursorIcon::PointingHand)
            .on_hover_text(tooltip)
    }

    /// The league name on the color of its mode, the only sign of the mode on screen; its
    /// tooltip says it. A name too long for the window is cut with "…", and shown whole in
    /// the tooltip.
    fn league_block(&self, ui: &mut Ui, room_on_the_right: f32) -> Response {
        use league_block::{CHEVRON_WIDTH, FRAME_WIDTH, GAP, PADDING_LEFT};

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
        // The mode isn't written on screen: the screen readers read it with the name.
        response.widget_info(|| {
            WidgetInfo::labeled(
                Role::Button,
                true,
                format!("{} · {tooltip}", self.league.name()),
            )
        });
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
        let tooltip = self.i18n.text(icon.tooltip());
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::language::Language;
    use egui::{Context, Event, Modifiers, RawInput};

    const WINDOW: Rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(420.0, 720.0));
    /// The middle of the Close icon, at the right edge.
    const CLOSE: Pos2 = pos2(420.0 - size::ICON_ZONE / 2.0, size::TITLE_BAR_HEIGHT / 2.0);
    /// The empty space of the bar, between the league block and the icons.
    const EMPTY: Pos2 = pos2(250.0, size::TITLE_BAR_HEIGHT / 2.0);

    /// Draws a window with only the title bar, one frame per call.
    struct Window {
        ctx: Context,
        league: League,
        i18n: Translator,
    }

    impl Window {
        fn new() -> Self {
            let ctx = Context::default();
            crate::ui::install(&ctx);
            let window = Self {
                ctx,
                league: League::new("Forbidden Rites", LeagueMode::Softcore),
                i18n: Translator::new(Language::English),
            };
            // The first frame lays the bar out.
            window.frame(vec![]);
            window
        }

        /// One frame with `events`; what the title bar returned.
        fn frame(&self, events: Vec<Event>) -> Option<TitleBarAction> {
            let input = RawInput {
                screen_rect: Some(WINDOW),
                events,
                ..Default::default()
            };
            let bar = TitleBar::new(&self.league, &self.i18n);
            let mut action = None;
            let mut output = self.ctx.run_ui(input, |ui| action = bar.show(ui));
            // epaint panics when the texture changes of a frame are dropped unapplied.
            output.textures_delta.clear();
            action
        }

        /// What each frame returned: pointer over `at`, press, move by `moved`, release.
        fn press_and_release(&self, at: Pos2, moved: Vec2) -> Vec<Option<TitleBarAction>> {
            let button = |pressed| Event::PointerButton {
                pos: at + if pressed { Vec2::ZERO } else { moved },
                button: PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            };
            vec![
                self.frame(vec![Event::PointerMoved(at)]),
                self.frame(vec![button(true)]),
                self.frame(vec![Event::PointerMoved(at + moved)]),
                self.frame(vec![button(false)]),
            ]
        }
    }

    #[test]
    fn a_click_on_close_closes_without_moving_the_window() {
        let actions = Window::new().press_and_release(CLOSE, Vec2::ZERO);

        assert_eq!(actions, [None, None, None, Some(TitleBarAction::Close)]);
    }

    #[test]
    fn dragging_the_empty_space_moves_the_window() {
        let actions = Window::new().press_and_release(EMPTY, vec2(30.0, 0.0));

        assert!(actions.contains(&Some(TitleBarAction::Move)), "{actions:?}");
    }

    #[test]
    fn a_click_on_the_empty_space_does_nothing() {
        let actions = Window::new().press_and_release(EMPTY, Vec2::ZERO);

        assert_eq!(actions, [None; 4]);
    }

    #[test]
    fn a_click_on_the_emblem_goes_home() {
        let emblem = pos2(
            PADDING_LEFT + EMBLEM_SIZE / 2.0,
            size::TITLE_BAR_HEIGHT / 2.0,
        );
        let actions = Window::new().press_and_release(emblem, Vec2::ZERO);

        assert_eq!(actions, [None, None, None, Some(TitleBarAction::Home)]);
    }

    #[test]
    fn a_click_between_the_emblem_and_the_league_goes_home_too() {
        let gap = pos2(
            PADDING_LEFT + EMBLEM_SIZE + EMBLEM_GAP / 2.0,
            size::TITLE_BAR_HEIGHT / 2.0,
        );
        let actions = Window::new().press_and_release(gap, Vec2::ZERO);

        assert_eq!(actions.last(), Some(&Some(TitleBarAction::Home)));
    }

    #[test]
    fn dragging_the_emblem_moves_the_window_without_going_home() {
        let emblem = pos2(
            PADDING_LEFT + EMBLEM_SIZE / 2.0,
            size::TITLE_BAR_HEIGHT / 2.0,
        );
        let actions = Window::new().press_and_release(emblem, vec2(30.0, 0.0));

        assert!(actions.contains(&Some(TitleBarAction::Move)), "{actions:?}");
        assert!(
            !actions.contains(&Some(TitleBarAction::Home)),
            "{actions:?}"
        );
    }

    #[test]
    fn a_click_on_the_league_opens_the_leagues() {
        let league = pos2(60.0, size::TITLE_BAR_HEIGHT / 2.0);
        let actions = Window::new().press_and_release(league, Vec2::ZERO);

        assert_eq!(actions.last(), Some(&Some(TitleBarAction::Leagues)));
    }
}
