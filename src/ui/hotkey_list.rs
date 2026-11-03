//! The keys of the price-check hotkey in the settings: a split button, the keys as text on the
//! dark of the controls and the chevron in a white square, which opens the list of the keys the
//! player may choose (`specs.md`, Settings; mock-up section "Hotkey list (14c3b2)").

use egui::{
    Area, Context, CornerRadius, CursorIcon, Id, Order, Pos2, Rect, Response, Role, Sense, Ui,
    Vec2, WidgetInfo, pos2, vec2,
};

use super::keys;
use super::theme::{self, color, motion, size, text};
use super::widgets;
use crate::hotkey::Shortcut;
use crate::i18n::{Message, Translator};

/// The height of the button, the side of the square of its chevron: the height of a pill.
const BUTTON_HEIGHT: f32 = 22.0;
/// Left and right of the keys, in the button.
const BUTTON_PADDING: f32 = 10.0;
const CHEVRON_WIDTH: f32 = 8.0;

/// A row of the list.
const ROW_HEIGHT: f32 = 28.0;
/// Above the first row and under the last.
const LIST_PADDING: f32 = 4.0;
const ROW_PADDING_LEFT: f32 = 8.0;
const ROW_PADDING_RIGHT: f32 = 10.0;
/// The zone of the check, on the left of each row, and the space after it.
const CHECK_ZONE: f32 = 12.0;
const CHECK_WIDTH: f32 = 11.0;
const ROW_GAP: f32 = 8.0;
/// The narrowest list, so that a short row doesn't make it a sliver.
const LIST_MIN_WIDTH: f32 = 200.0;
/// The space between the button and the open list, which it slides down through.
const GAP_UNDER_BUTTON: f32 = 4.0;
/// The keys let the settings show through a little, like the leagues.
const OPACITY: f32 = 0.94;

/// The split button of the keys, `label` the keys joined, `on` whether the hotkey is on (its keys
/// faint while off), `open` whether the list is open (its chevron turned down). The whole button
/// opens the list.
pub fn button(ui: &mut Ui, label: &str, on: bool, open: bool, tooltip: &str) -> Response {
    let galley = widgets::text_line(ui, label, text::pill());
    let keys_width = galley.size().x + 2.0 * BUTTON_PADDING;
    let (rect, response) = ui.allocate_exact_size(
        vec2(keys_width + BUTTON_HEIGHT, BUTTON_HEIGHT),
        Sense::click(),
    );
    response.widget_info(|| WidgetInfo::labeled(Role::ComboBox, true, label));
    let (keys_rect, square) = rect.split_left_right_at_x(rect.left() + keys_width);
    let radius = size::RADIUS;
    let lit = response.hovered() || response.has_focus();
    let painter = ui.painter();
    painter.rect_filled(
        keys_rect,
        CornerRadius {
            nw: radius,
            sw: radius,
            ..CornerRadius::ZERO
        },
        if lit {
            color::KEYS_LIT
        } else {
            color::CONTROL_LINE
        },
    );
    painter.rect_filled(
        square,
        CornerRadius {
            ne: radius,
            se: radius,
            ..CornerRadius::ZERO
        },
        color::ACCENT,
    );
    painter.galley(
        pos2(
            keys_rect.left() + BUTTON_PADDING,
            keys_rect.center().y - galley.size().y / 2.0,
        ),
        galley,
        if on { color::TEXT } else { color::FAINT },
    );
    let turn = ui
        .ctx()
        .animate_bool_with_time(response.id.with("chevron"), open, motion::OPEN);
    widgets::chevron(
        ui.painter(),
        square.center(),
        CHEVRON_WIDTH,
        turn,
        color::ON_ACCENT,
    );
    response
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(tooltip)
}

/// What the player did with the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyListAction {
    Choose(Shortcut),
    /// Clicked outside the list.
    Close,
}

/// The list of [`Shortcut::CHOICES`], the default first, the keys of the hotkey checked.
pub struct HotkeyList<'a> {
    chosen: Shortcut,
    i18n: &'a Translator,
}

impl<'a> HotkeyList<'a> {
    #[must_use]
    pub fn new(chosen: Shortcut, i18n: &'a Translator) -> Self {
        Self { chosen, i18n }
    }

    /// Draws the list under `button`, its right edge on the right of the button, while it is
    /// `open` and while it closes; what the player did with it.
    pub fn show(&self, ctx: &Context, button: Rect, open: bool) -> Option<HotkeyListAction> {
        let shown = ctx.animate_bool_with_time(Id::NULL.with("hotkey-list"), open, motion::OPEN);
        if shown == 0.0 {
            return None;
        }
        // The keyboard starts on the chosen keys, when the list opens.
        let opened_id = Id::NULL.with("hotkey-list-opened");
        let just_opened = open && !ctx.data(|data| data.get_temp(opened_id).unwrap_or(false));
        ctx.data_mut(|data| data.insert_temp(opened_id, open));
        let rows = self.rows(ctx);
        let width = rows
            .iter()
            .map(|row| row.width)
            .fold(LIST_MIN_WIDTH, f32::max);
        #[expect(clippy::cast_precision_loss, reason = "a few keys")]
        let list_size = vec2(width, 2.0 * LIST_PADDING + rows.len() as f32 * ROW_HEIGHT);
        let top_left = pos2(
            button.right() - width,
            button.bottom() + GAP_UNDER_BUTTON * shown,
        );
        Area::new(Id::NULL.with("hotkey-list-area"))
            .order(Order::Foreground)
            .fixed_pos(top_left)
            .interactable(open)
            .show(ctx, |ui| {
                ui.multiply_opacity(shown);
                let list = Rect::from_min_size(ui.next_widget_position(), list_size);
                let radius = CornerRadius::same(size::RADIUS);
                ui.painter()
                    .add(theme::floating_shadow().as_shape(list, radius));
                ui.painter()
                    .rect_filled(list, radius, color::BUBBLE.gamma_multiply(OPACITY));
                let mut action = None;
                for (index, row) in rows.iter().enumerate() {
                    #[expect(clippy::cast_precision_loss, reason = "a few keys")]
                    let rect = Rect::from_min_size(
                        list.min + vec2(0.0, LIST_PADDING + index as f32 * ROW_HEIGHT),
                        vec2(width, ROW_HEIGHT),
                    );
                    let response = self.row(ui, rect, row);
                    if just_opened && row.shortcut == self.chosen {
                        response.request_focus();
                    }
                    if response.clicked() {
                        action = Some(HotkeyListAction::Choose(row.shortcut));
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
                action.or_else(|| (open && clicked_outside).then_some(HotkeyListAction::Close))
            })
            .inner
    }

    /// The rows of the list, with the width each needs.
    fn rows(&self, ctx: &Context) -> Vec<Row> {
        let default = self.i18n.text(Message::SettingsHotkeyDefault);
        Shortcut::CHOICES
            .into_iter()
            .enumerate()
            .map(|(index, shortcut)| {
                let label = keys::joined(&keys::named(&shortcut.keys(), self.i18n));
                let note = (index == 0).then(|| default.clone());
                let text_width = |value: &str, font| {
                    ctx.fonts_mut(|fonts| {
                        fonts
                            .layout_no_wrap(value.to_owned(), font, color::TEXT)
                            .size()
                            .x
                    })
                };
                let note_width = note
                    .as_deref()
                    .map_or(0.0, |note| ROW_GAP + text_width(note, text::small()));
                Row {
                    shortcut,
                    width: ROW_PADDING_LEFT
                        + CHECK_ZONE
                        + ROW_GAP
                        + text_width(&label, text::list_keys())
                        + note_width
                        + ROW_PADDING_RIGHT,
                    label,
                    note,
                }
            })
            .collect()
    }

    /// A row: the check of the chosen keys on the left, the keys, then "Default" on the right
    /// of the first; lighter under the pointer, and while it is the chosen one.
    fn row(&self, ui: &mut Ui, rect: Rect, row: &Row) -> Response {
        let response = ui.interact(rect, ui.scope_id().with(row.shortcut), Sense::click());
        let chosen = row.shortcut == self.chosen;
        let painter = ui.painter();
        if chosen || response.hovered() || response.has_focus() {
            painter.rect_filled(rect, CornerRadius::ZERO, color::ROW_LIT);
        }
        let check_center = pos2(
            rect.left() + ROW_PADDING_LEFT + CHECK_ZONE / 2.0,
            rect.center().y,
        );
        if chosen {
            widgets::check_mark(painter, check_center, CHECK_WIDTH, color::ACCENT);
        }
        let keys = widgets::text_line(ui, row.label.clone(), text::list_keys());
        let keys_left = rect.left() + ROW_PADDING_LEFT + CHECK_ZONE + ROW_GAP;
        painter.galley(centered(keys_left, rect, keys.size()), keys, color::TEXT);
        if let Some(note) = &row.note {
            let note = widgets::text_line(ui, note.clone(), text::small());
            let left = rect.right() - ROW_PADDING_RIGHT - note.size().x;
            painter.galley(centered(left, rect, note.size()), note, color::FAINT);
        }
        response.widget_info(|| WidgetInfo::selected(Role::Button, true, chosen, &row.label));
        response.on_hover_cursor(CursorIcon::PointingHand)
    }
}

/// A row to draw.
struct Row {
    shortcut: Shortcut,
    /// The keys joined, in the language of the interface.
    label: String,
    /// "Default", on the right of the default keys.
    note: Option<String>,
    /// The width it needs, so that the list grows with its longest row.
    width: f32,
}

/// Where to draw a text of `size` from `left`, centered in the height of `rect`.
fn centered(left: f32, rect: Rect, size: Vec2) -> Pos2 {
    pos2(left, rect.center().y - size.y / 2.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::language::Language;
    use egui::{Event, Modifiers, PointerButton, RawInput};

    /// The button, at the top left of the window.
    const BUTTON: Rect = Rect {
        min: pos2(200.0, 100.0),
        max: pos2(280.0, 122.0),
    };

    /// Draws the list alone, one frame per call, with a clock that goes past the animations.
    struct Window {
        ctx: Context,
        i18n: Translator,
        time: std::cell::Cell<f64>,
    }

    impl Window {
        fn new() -> Self {
            let ctx = Context::default();
            crate::ui::install(&ctx);
            Self {
                ctx,
                i18n: Translator::new(Language::English),
                time: std::cell::Cell::new(0.0),
            }
        }

        fn frame(&self, open: bool, events: Vec<Event>) -> Option<HotkeyListAction> {
            // Past the animation of a frame, and short of the longest click.
            self.time.set(self.time.get() + 0.2);
            let input = RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(420.0, 720.0))),
                time: Some(self.time.get()),
                events,
                ..Default::default()
            };
            let list = HotkeyList::new(Shortcut::default(), &self.i18n);
            let mut action = None;
            let mut output = self
                .ctx
                .run_ui(input, |ui| action = list.show(ui.ctx(), BUTTON, open));
            // epaint panics when the texture changes of a frame are dropped unapplied.
            output.textures_delta.clear();
            action
        }

        /// The actions of a click at `at`, the list `open`, after it finished moving.
        fn click(&self, open: bool, at: Pos2) -> Vec<Option<HotkeyListAction>> {
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

    /// The middle of the row `index` from the top, near the right edge of the list, which is the
    /// right edge of the button.
    fn row(index: f32) -> Pos2 {
        pos2(
            BUTTON.right() - 20.0,
            BUTTON.bottom() + GAP_UNDER_BUTTON + LIST_PADDING + (index + 0.5) * ROW_HEIGHT,
        )
    }

    #[test]
    fn a_click_on_keys_chooses_them() {
        let actions = Window::new().click(true, row(2.0));

        assert_eq!(
            actions.last(),
            Some(&Some(HotkeyListAction::Choose(Shortcut::CHOICES[2])))
        );
    }

    #[test]
    fn a_click_outside_closes_the_list() {
        let actions = Window::new().click(true, pos2(200.0, 600.0));

        assert_eq!(actions.last(), Some(&Some(HotkeyListAction::Close)));
    }

    #[test]
    fn a_closed_list_takes_no_click() {
        let actions = Window::new().click(false, row(0.0));

        assert_eq!(actions, [None, None, None]);
    }

    #[test]
    fn the_list_grows_with_its_longest_row_from_its_narrowest() {
        let window = Window::new();
        window.frame(true, vec![]);
        let rows = |language| {
            HotkeyList::new(Shortcut::default(), &Translator::new(language)).rows(&window.ctx)
        };

        let english = rows(Language::English);
        assert_eq!(english.len(), Shortcut::CHOICES.len());
        assert_eq!(english[0].note.as_deref(), Some("Default"));
        assert!(english[1..].iter().all(|row| row.note.is_none()));
        assert!(english.iter().all(|row| row.width < LIST_MIN_WIDTH));
        // "Par défaut" is longer than "Default".
        let french = rows(Language::French);
        assert!(french[0].width > english[0].width);
    }
}
