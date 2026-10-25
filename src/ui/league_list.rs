//! The list of the leagues, under the league block: one line per league, its softcore trade on
//! the left and its hardcore trade on the right, the permanent ones last. It fades in while it
//! slides down, and lets the item show through a little.

use egui::{
    Area, Context, CornerRadius, CursorIcon, Id, Order, Rect, Response, Role, Sense, Ui, Vec2,
    WidgetInfo, pos2, vec2,
};

use super::theme::{self, LeagueModeColors, color, motion, size, text};
use super::title_bar::LEAGUE_BLOCK_LEFT;
use super::widgets;
use crate::i18n::{Message, Translator};
use crate::league::{League, LeagueMode, LeagueRow};

/// A league in one mode.
const CELL: Vec2 = vec2(150.0, 28.0);
const CELL_PADDING_LEFT: f32 = 10.0;
const CELL_PADDING_RIGHT: f32 = 8.0;
/// The space between the title bar and the open list, which it slides down through.
const GAP_UNDER_BAR: f32 = 4.0;
/// The leagues let the item show through a little.
const OPACITY: f32 = 0.94;

/// What the player did with the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeagueListAction {
    Choose(League),
    /// Clicked outside the list.
    Close,
}

pub struct LeagueList<'a> {
    rows: &'a [LeagueRow],
    chosen: Option<&'a League>,
    i18n: &'a Translator,
}

impl<'a> LeagueList<'a> {
    #[must_use]
    pub fn new(rows: &'a [LeagueRow], chosen: Option<&'a League>, i18n: &'a Translator) -> Self {
        Self { rows, chosen, i18n }
    }

    /// How much of the list shows, from 0 (closed) to 1 (open), as it moves toward `open`.
    pub fn visibility(ctx: &Context, open: bool) -> f32 {
        ctx.animate_bool_with_time(Id::NULL.with("league-list"), open, motion::OPEN)
    }

    /// Draws the list while it is `open`, and while it closes; what the player did with it.
    pub fn show(&self, ctx: &Context, open: bool) -> Option<LeagueListAction> {
        let shown = Self::visibility(ctx, open);
        if shown == 0.0 {
            return None;
        }
        // The keyboard starts on the chosen league, when the list opens.
        let opened_id = Id::NULL.with("league-list-opened");
        let just_opened = open && !ctx.data(|data| data.get_temp(opened_id).unwrap_or(false));
        ctx.data_mut(|data| data.insert_temp(opened_id, open));
        let top = size::TITLE_BAR_HEIGHT + GAP_UNDER_BAR * shown;
        #[expect(clippy::cast_precision_loss, reason = "a few leagues")]
        let list_size = vec2(2.0 * CELL.x, self.rows.len() as f32 * CELL.y);
        Area::new(Id::NULL.with("league-list-area"))
            .order(Order::Foreground)
            .fixed_pos(pos2(LEAGUE_BLOCK_LEFT, top))
            .interactable(open)
            .show(ctx, |ui| {
                ui.multiply_opacity(shown);
                let list = Rect::from_min_size(ui.next_widget_position(), list_size);
                let radius = CornerRadius::same(size::RADIUS);
                ui.painter()
                    .add(theme::floating_shadow().as_shape(list, radius));
                let mut action = None;
                for (index, row) in self.rows.iter().enumerate() {
                    for (column, league) in [&row.softcore, &row.hardcore].into_iter().enumerate() {
                        let Some(league) = league else { continue };
                        #[expect(clippy::cast_precision_loss, reason = "a few leagues")]
                        let cell = Rect::from_min_size(
                            list.min + vec2(column as f32 * CELL.x, index as f32 * CELL.y),
                            CELL,
                        );
                        let corners = corners(index == 0, index + 1 == self.rows.len(), column);
                        let response = self.cell(ui, cell, corners, league);
                        if just_opened && self.chosen == Some(league) {
                            response.request_focus();
                        }
                        if response.clicked() {
                            action = Some(LeagueListAction::Choose(league.clone()));
                        }
                    }
                }
                ui.allocate_rect(list, Sense::hover());
                let clicked_outside = ui.input(|input| {
                    input.pointer.any_click()
                        && input
                            .pointer
                            .interact_pos()
                            .is_some_and(|pointer| !list.contains(pointer))
                });
                action.or_else(|| (open && clicked_outside).then_some(LeagueListAction::Close))
            })
            .inner
    }

    /// A league in its mode: lighter under the pointer, and while it is the chosen one.
    fn cell(&self, ui: &mut Ui, rect: Rect, corners: CornerRadius, league: &League) -> Response {
        let response = ui.interact(rect, ui.scope_id().with(league.id()), Sense::click());
        let colors = LeagueModeColors::of(league.mode());
        let lit = response.hovered() || response.has_focus() || self.chosen == Some(league);
        let background = if lit {
            colors.hovered
        } else {
            colors.background
        };
        let name = widgets::truncated_line(
            ui,
            league.name(),
            text::league(),
            CELL.x - CELL_PADDING_LEFT - CELL_PADDING_RIGHT,
        );
        let name_is_cut = name.elided;
        let painter = ui.painter();
        painter.rect_filled(rect, corners, background.gamma_multiply(OPACITY));
        painter.galley(
            pos2(
                rect.left() + CELL_PADDING_LEFT,
                rect.center().y - name.size().y / 2.0,
            ),
            name,
            color::TEXT,
        );

        let mode = self.i18n.text(match league.mode() {
            LeagueMode::Softcore => Message::LeagueModeSoftcore,
            LeagueMode::Hardcore => Message::LeagueModeHardcore,
        });
        let label = format!("{} · {mode}", league.name());
        response.widget_info(|| {
            WidgetInfo::selected(Role::Button, true, self.chosen == Some(league), &label)
        });
        response
            .on_hover_cursor(CursorIcon::PointingHand)
            .on_hover_text(if name_is_cut { label } else { mode })
    }
}

/// The rounded corners of a cell: the corners of the list.
fn corners(first_row: bool, last_row: bool, column: usize) -> CornerRadius {
    let round = |yes: bool| if yes { size::RADIUS } else { 0 };
    let (left, right) = (column == 0, column == 1);
    CornerRadius {
        nw: round(first_row && left),
        ne: round(first_row && right),
        sw: round(last_row && left),
        se: round(last_row && right),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::language::Language;
    use crate::league::Leagues;
    use egui::{Event, Modifiers, PointerButton, Pos2, RawInput};

    /// Draws the list alone, one frame per call, with a clock that goes past the animations.
    struct Window {
        ctx: Context,
        leagues: Leagues,
        chosen: League,
        i18n: Translator,
        time: std::cell::Cell<f64>,
    }

    impl Window {
        fn new() -> Self {
            let ctx = Context::default();
            crate::ui::install(&ctx);
            Self {
                ctx,
                leagues: Leagues::new(
                    ["Forbidden Rites", "HC Forbidden Rites", "Runes of Aldur"].map(League::new),
                ),
                chosen: League::new("Forbidden Rites"),
                i18n: Translator::new(Language::English),
                time: std::cell::Cell::new(0.0),
            }
        }

        fn frame(&self, open: bool, events: Vec<Event>) -> Option<LeagueListAction> {
            // Past the animation of a frame, and short of the longest click.
            self.time.set(self.time.get() + 0.2);
            let input = RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(420.0, 720.0))),
                time: Some(self.time.get()),
                events,
                ..Default::default()
            };
            let list = LeagueList::new(self.leagues.rows(), Some(&self.chosen), &self.i18n);
            let mut action = None;
            let mut output = self
                .ctx
                .run_ui(input, |ui| action = list.show(ui.ctx(), open));
            // epaint panics when the texture changes of a frame are dropped unapplied.
            output.textures_delta.clear();
            action
        }

        /// The actions of a click at `at`, the list `open`, after it finished moving.
        fn click(&self, open: bool, at: Pos2) -> Vec<Option<LeagueListAction>> {
            self.frame(open, vec![]);
            self.frame(open, vec![]);
            let button = |pressed| Event::PointerButton {
                pos: at,
                button: PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            };
            vec![
                self.frame(open, vec![Event::PointerMoved(at)]),
                self.frame(open, vec![button(true)]),
                self.frame(open, vec![button(false)]),
            ]
        }
    }

    /// The middle of a cell of the list: `row` from the top, `column` 0 softcore, 1 hardcore.
    fn cell(row: f32, column: f32) -> Pos2 {
        pos2(
            LEAGUE_BLOCK_LEFT + (column + 0.5) * CELL.x,
            size::TITLE_BAR_HEIGHT + GAP_UNDER_BAR + (row + 0.5) * CELL.y,
        )
    }

    #[test]
    fn a_click_on_a_league_chooses_it() {
        let actions = Window::new().click(true, cell(0.0, 1.0));

        assert_eq!(
            actions.last(),
            Some(&Some(LeagueListAction::Choose(League::new(
                "HC Forbidden Rites"
            ))))
        );
    }

    #[test]
    fn a_click_outside_closes_the_list() {
        let actions = Window::new().click(true, pos2(200.0, 600.0));

        assert_eq!(actions.last(), Some(&Some(LeagueListAction::Close)));
    }

    #[test]
    fn a_closed_list_takes_no_click() {
        let actions = Window::new().click(false, cell(0.0, 0.0));

        assert_eq!(actions, [None, None, None]);
    }

    #[test]
    fn a_league_without_hardcore_trade_leaves_its_half_empty() {
        // Runes of Aldur has no hardcore trade here: its right half takes no click, and the
        // click falls outside the leagues.
        let actions = Window::new().click(true, cell(1.0, 1.0));

        assert_ne!(
            actions.last(),
            Some(&Some(LeagueListAction::Choose(League::new(
                "Runes of Aldur"
            ))))
        );
    }

    #[test]
    fn only_the_corners_of_the_list_are_rounded() {
        assert_eq!(
            corners(true, false, 0),
            CornerRadius {
                nw: size::RADIUS,
                ..CornerRadius::ZERO
            }
        );
        assert_eq!(
            corners(false, true, 1),
            CornerRadius {
                se: size::RADIUS,
                ..CornerRadius::ZERO
            }
        );
        assert_eq!(corners(false, false, 0), CornerRadius::ZERO);
        // A list of one line rounds both corners of each side.
        assert_eq!(
            corners(true, true, 0),
            CornerRadius {
                nw: size::RADIUS,
                sw: size::RADIUS,
                ..CornerRadius::ZERO
            }
        );
    }
}
