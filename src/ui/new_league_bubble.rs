//! The bubble of a new league, under the league block, for a player on a permanent league
//! once a new league has started: its name, a pill to switch to it, and a cross to close it.

use egui::text::{LayoutJob, TextFormat};
use egui::{
    Align, Area, Context, CornerRadius, CursorIcon, Frame, Id, Layout, Margin, Order, Role, Sense,
    Ui, Vec2, WidgetInfo, pos2, vec2,
};

use super::theme::{self, color, size, text};
use super::title_bar::LEAGUE_BLOCK_LEFT;
use super::widgets;
use crate::i18n::{Message, Translator};
use crate::league::League;

const WIDTH: f32 = 330.0;
/// The bubble starts a little left of the league block, 4 px under the title bar like the
/// list of the leagues.
const LEFT: f32 = LEAGUE_BLOCK_LEFT - 8.0;
const GAP_UNDER_BAR: f32 = 4.0;
const PADDING_X: i8 = 14;
const PADDING_Y: i8 = 10;
const CONTENT_GAP: f32 = 10.0;
const CLOSE_ZONE: f32 = 18.0;
const CLOSE_SIZE: f32 = 7.0;

/// What the player did with the bubble.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BubbleAction {
    Switch,
    Close,
}

pub struct NewLeagueBubble<'a> {
    league: &'a League,
    i18n: &'a Translator,
}

impl<'a> NewLeagueBubble<'a> {
    #[must_use]
    pub fn new(league: &'a League, i18n: &'a Translator) -> Self {
        Self { league, i18n }
    }

    pub fn show(&self, ctx: &Context) -> Option<BubbleAction> {
        Area::new(Id::NULL.with("new-league-bubble"))
            .order(Order::Foreground)
            .fixed_pos(pos2(LEFT, size::TITLE_BAR_HEIGHT + GAP_UNDER_BAR))
            .show(ctx, |ui| {
                let frame = Frame::NONE
                    .fill(color::BUBBLE)
                    .corner_radius(CornerRadius::same(size::BUBBLE_RADIUS))
                    .shadow(theme::floating_shadow())
                    .inner_margin(Margin::symmetric(PADDING_X, PADDING_Y));
                frame
                    .show(ui, |ui| {
                        ui.set_width(WIDTH - 2.0 * f32::from(PADDING_X));
                        ui.spacing_mut().item_spacing = vec2(CONTENT_GAP, 0.0);
                        ui.with_layout(Layout::left_to_right(Align::Center), |ui| self.content(ui))
                            .inner
                    })
                    .inner
            })
            .inner
    }

    fn content(&self, ui: &mut Ui) -> Option<BubbleAction> {
        // A long league name wraps, so that the pill and the cross stay in the bubble.
        let switch = self.i18n.text(Message::NewLeagueSwitch);
        let text_width = ui.available_width()
            - widgets::pill_size(ui, &switch).x
            - CLOSE_ZONE
            - 2.0 * CONTENT_GAP;
        let mut job = self.text();
        job.wrap.max_width = text_width.max(0.0);
        ui.label(ui.painter().layout_job(job));
        let mut action = None;
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let (rect, close) = ui.allocate_exact_size(Vec2::splat(CLOSE_ZONE), Sense::click());
            let tooltip = self.i18n.text(Message::CloseTooltip);
            close.widget_info(|| WidgetInfo::labeled(Role::Button, true, &tooltip));
            let lit = close.hovered() || close.has_focus();
            widgets::cross(
                ui.painter(),
                rect.center(),
                CLOSE_SIZE,
                if lit { color::TEXT } else { color::WEAK },
            );
            if close
                .on_hover_cursor(CursorIcon::PointingHand)
                .on_hover_text(tooltip)
                .clicked()
            {
                action = Some(BubbleAction::Close);
            }
            if widgets::pill(ui, &switch).clicked() {
                action = Some(BubbleAction::Switch);
            }
        });
        action
    }

    /// "Rise of the Abyssal has started.", the name in bold.
    fn text(&self) -> LayoutJob {
        let name = self.league.name();
        let sentence = self.i18n.new_league(name);
        let regular = TextFormat::simple(text::bubble(), color::TEXT);
        let strong = TextFormat::simple(text::bubble_strong(), color::TEXT);
        let mut job = LayoutJob::default();
        match sentence.split_once(name) {
            Some((before, after)) => {
                job.append(before, 0.0, regular.clone());
                job.append(name, 0.0, strong);
                job.append(after, 0.0, regular);
            }
            None => job.append(&sentence, 0.0, regular),
        }
        job
    }
}
