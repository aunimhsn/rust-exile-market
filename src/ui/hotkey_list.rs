//! The keys of the price-check hotkey in the settings: a split button, the keys as text on the
//! dark of the controls and the chevron in a white square, which opens the list of the keys the
//! player may choose (`specs.md`, Settings; mock-up section "Hotkey list (14c3b2)").

use egui::{
    Context, CornerRadius, CursorIcon, FocusDirection, Id, Key, Modifiers, Pos2, Rect, Response,
    Role, Sense, Ui, Vec2, WidgetInfo, pos2, vec2,
};

use super::theme::{color, motion, size, text};
use super::widgets::{self, FloatingList};
use super::{icons, keys};
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

/// The animation of the list, which opens and closes.
fn shown_id() -> Id {
    Id::NULL.with("hotkey-list")
}

/// The animation of the chevron of the button, which turns down while the list is open.
fn chevron_id() -> Id {
    Id::NULL.with("hotkey-list-chevron")
}

/// What the list remembers from the last frame it was drawn ([`Seen`]).
fn seen_id() -> Id {
    Id::NULL.with("hotkey-list-seen")
}

/// What the list was in the last frame it was drawn.
#[derive(Debug, Clone, Copy, Default)]
struct Seen {
    open: bool,
    /// One of its rows had the keyboard, given back to the button when the list closes.
    row_focused: bool,
}

/// The list no longer drawn, the settings left while it was open or closing: it forgets its
/// animation and that of the chevron, so that it doesn't fade out when the settings open again,
/// and that it was open, so that the keyboard starts on the chosen keys again.
pub fn forget(ctx: &Context) {
    // No time: egui goes straight to the end.
    ctx.animate_bool_with_time(shown_id(), false, 0.0);
    ctx.animate_bool_with_time(chevron_id(), false, 0.0);
    ctx.data_mut(|data| data.remove::<Seen>(seen_id()));
}

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
        .animate_bool_with_time(chevron_id(), open, motion::OPEN);
    icons::chevron(
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
    /// `open` and while it closes; what the player did with it. Opened from the keyboard (the
    /// button had it), the keyboard starts on the chosen keys, or on the first row when none is
    /// chosen (keys written by hand); the arrows go from row to row, and the keyboard goes back
    /// to the button when the list closes. Opened with the mouse, no row takes the keyboard, so
    /// that the button isn't lit as under the pointer once the list closes.
    pub fn show(&self, ctx: &Context, button: &Response, open: bool) -> Option<HotkeyListAction> {
        let shown = ctx.animate_bool_with_time(shown_id(), open, motion::OPEN);
        let seen: Seen = ctx.data(|data| data.get_temp(seen_id()).unwrap_or_default());
        let by_keyboard = open && !seen.open && button.has_focus();
        // From the last frame: Escape takes the keyboard away before this one (egui), and a row
        // chosen with Enter was clicked in the last one. Also when a slow frame skips the closing.
        if !open && seen.open && seen.row_focused {
            button.request_focus();
        }
        if shown == 0.0 {
            if seen.open {
                ctx.data_mut(|data| data.insert_temp(seen_id(), Seen::default()));
            }
            return None;
        }
        let button = button.rect;
        let rows = self.rows(ctx);
        let width = list_width(&rows);
        let start = rows
            .iter()
            .position(|row| row.shortcut == self.chosen)
            .unwrap_or(0);
        #[expect(clippy::cast_precision_loss, reason = "a few keys")]
        let list_size = vec2(width, 2.0 * LIST_PADDING + rows.len() as f32 * ROW_HEIGHT);
        let top_left = pos2(
            button.right() - width,
            button.bottom() + GAP_UNDER_BUTTON * shown,
        );
        let floating = FloatingList {
            id: Id::NULL.with("hotkey-list-area"),
            top_left,
            size: list_size,
            shown,
            is_open: open,
        };
        floating.show(ctx, HotkeyListAction::Close, |ui, list| {
            ui.painter().rect_filled(
                list,
                CornerRadius::same(size::RADIUS),
                color::LIST.gamma_multiply(widgets::FLOATING_LIST_OPACITY),
            );
            let mut action = None;
            let mut focused = None;
            let mut responses = Vec::with_capacity(rows.len());
            for (index, row) in rows.iter().enumerate() {
                #[expect(clippy::cast_precision_loss, reason = "a few keys")]
                let rect = Rect::from_min_size(
                    list.min + vec2(0.0, LIST_PADDING + index as f32 * ROW_HEIGHT),
                    vec2(width, ROW_HEIGHT),
                );
                let response = self.row(ui, rect, row);
                if by_keyboard && index == start {
                    response.request_focus();
                }
                if response.clicked() {
                    action = Some(HotkeyListAction::Choose(row.shortcut));
                }
                if response.has_focus() {
                    focused = Some(index);
                }
                responses.push(response);
            }
            if open && let Some(index) = focused {
                arrows(ui, &responses, index);
            }
            ctx.data_mut(|data| {
                data.insert_temp(
                    seen_id(),
                    Seen {
                        open,
                        row_focused: focused.is_some(),
                    },
                );
            });
            action
        })
    }

    /// The rows of the list, with the width each needs.
    fn rows(&self, ctx: &Context) -> Vec<Row> {
        let default = self.i18n.text(Message::SettingsHotkeyDefault);
        let text_width = |value: &str, font| {
            ctx.fonts_mut(|fonts| {
                fonts
                    .layout_no_wrap(value.to_owned(), font, color::TEXT)
                    .size()
                    .x
            })
        };
        Shortcut::CHOICES
            .into_iter()
            .enumerate()
            .map(|(index, shortcut)| {
                let label = keys::joined(&keys::named(&shortcut.keys(), self.i18n));
                let note = (index == 0).then(|| default.clone());
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
            icons::check_mark(painter, check_center, CHECK_WIDTH, color::ACCENT);
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

/// The arrows up and down move the keyboard from the row `focused` of `rows` to the one above or
/// under it, stopping at the first and the last; no arrow leaves the list, which would reach a
/// setting hidden under it.
fn arrows(ui: &Ui, rows: &[Response], focused: usize) {
    let (up, down, side) = ui.input_mut(|input| {
        (
            input.consume_key(Modifiers::NONE, Key::ArrowUp),
            input.consume_key(Modifiers::NONE, Key::ArrowDown),
            input.consume_key(Modifiers::NONE, Key::ArrowLeft)
                | input.consume_key(Modifiers::NONE, Key::ArrowRight),
        )
    });
    if !(up || down || side) {
        return;
    }
    // egui would move the keyboard to the nearest widget of the whole window.
    ui.memory_mut(|memory| memory.move_focus(FocusDirection::None));
    let next = if down {
        (focused + 1).min(rows.len() - 1)
    } else if up {
        focused.saturating_sub(1)
    } else {
        focused
    };
    rows[next].request_focus();
}

/// The width of the list: its longest row, or its narrowest width.
fn list_width(rows: &[Row]) -> f32 {
    rows.iter()
        .map(|row| row.width)
        .fold(LIST_MIN_WIDTH, f32::max)
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
    use egui::{Event, PointerButton, RawInput};

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
        /// The keys of the hotkey.
        chosen: Shortcut,
    }

    impl Window {
        fn new() -> Self {
            Self::with(Shortcut::default())
        }

        /// The keys of the hotkey `chosen`.
        fn with(chosen: Shortcut) -> Self {
            let ctx = Context::default();
            crate::ui::install(&ctx);
            Self {
                ctx,
                i18n: Translator::new(Language::English),
                time: std::cell::Cell::new(0.0),
                chosen,
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
            let list = HotkeyList::new(self.chosen, &self.i18n);
            let mut action = None;
            crate::ui::fixtures::run_frame(&self.ctx, input, |ui| {
                let button = ui.interact(BUTTON, button_id(), Sense::click());
                action = list.show(ui.ctx(), &button, open);
            });
            action
        }

        /// The action of `keys` pressed one per frame, the list opened from the keyboard (the
        /// button having it).
        fn keys(&self, keys: &[Key]) -> Option<HotkeyListAction> {
            self.frame(false, vec![]);
            self.ctx
                .memory_mut(|memory| memory.request_focus(button_id()));
            self.frame(true, vec![]);
            self.frame(true, vec![]);
            keys.iter()
                .map(|&key| self.frame(true, vec![press(key)]))
                .last()
                .flatten()
        }

        /// Whether the button has the keyboard.
        fn button_focused(&self) -> bool {
            self.ctx.memory(|memory| memory.has_focus(button_id()))
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

    fn button_id() -> Id {
        Id::NULL.with("button")
    }

    fn press(key: Key) -> Event {
        Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
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
    fn the_keyboard_starts_on_the_chosen_keys_and_the_arrows_go_from_row_to_row() {
        let window = Window::new();
        assert_eq!(
            window.keys(&[Key::Enter]),
            Some(HotkeyListAction::Choose(Shortcut::CHOICES[0]))
        );

        let window = Window::new();
        assert_eq!(
            window.keys(&[Key::ArrowDown, Key::ArrowDown, Key::Enter]),
            Some(HotkeyListAction::Choose(Shortcut::CHOICES[2]))
        );
    }

    #[test]
    fn the_arrows_stop_at_the_first_and_the_last_rows() {
        let window = Window::new();
        assert_eq!(
            window.keys(&[Key::ArrowUp, Key::ArrowLeft, Key::Enter]),
            Some(HotkeyListAction::Choose(Shortcut::CHOICES[0]))
        );

        let window = Window::new();
        let mut keys = vec![Key::ArrowDown; Shortcut::CHOICES.len() + 2];
        keys.push(Key::Enter);
        assert_eq!(
            window.keys(&keys),
            Some(HotkeyListAction::Choose(*Shortcut::CHOICES.last().unwrap()))
        );
    }

    #[test]
    fn keys_written_by_hand_start_the_keyboard_on_the_first_row() {
        let window = Window::with("Ctrl+Shift+D".parse().unwrap());
        assert_eq!(
            window.keys(&[Key::ArrowUp, Key::Enter]),
            Some(HotkeyListAction::Choose(Shortcut::CHOICES[0]))
        );
    }

    #[test]
    fn escape_gives_the_keyboard_back_to_the_button() {
        let window = Window::new();
        window.keys(&[Key::ArrowDown]);
        assert!(!window.button_focused());

        // egui takes the keyboard away at Escape, before the list closes in that same frame.
        window.frame(false, vec![press(Key::Escape)]);
        assert!(window.button_focused());
    }

    #[test]
    fn keys_chosen_with_enter_give_the_keyboard_back_to_the_button() {
        let window = Window::new();
        assert_eq!(
            window.keys(&[Key::ArrowDown, Key::Enter]),
            Some(HotkeyListAction::Choose(Shortcut::CHOICES[1]))
        );

        window.frame(false, vec![]);
        assert!(window.button_focused());
    }

    #[test]
    fn a_list_opened_with_the_mouse_leaves_the_button_unlit_once_closed() {
        let window = Window::new();
        // A click on the chosen keys, which the pointer leaves the keyboard on.
        let actions = window.click(true, row(0.0));
        assert_eq!(
            actions.last(),
            Some(&Some(HotkeyListAction::Choose(Shortcut::CHOICES[0])))
        );

        window.frame(false, vec![]);
        assert!(!window.button_focused());
    }

    #[test]
    fn a_list_forgotten_open_doesnt_fade_out_and_starts_on_the_chosen_keys_again() {
        let window = Window::new();
        // Opened from the keyboard: a row has it.
        window.keys(&[]);
        // The chevron turned down, as the button draws it over an open list.
        window.ctx.animate_bool_with_time(chevron_id(), true, 0.0);
        forget(&window.ctx);

        // Read without moving: no time passes in an endless animation.
        let value = |id| window.ctx.animate_bool_with_time(id, true, f32::INFINITY);
        assert_eq!(value(shown_id()), 0.0);
        assert_eq!(value(chevron_id()), 0.0);
        // Forgotten open: the keyboard doesn't go back to the button, which would stay lit.
        window.frame(false, vec![]);
        assert!(!window.button_focused());
        // Opened again: the keyboard on the chosen keys, Enter chooses them.
        assert_eq!(
            window.keys(&[Key::Enter]),
            Some(HotkeyListAction::Choose(Shortcut::CHOICES[0]))
        );
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
        assert_eq!(list_width(&english), LIST_MIN_WIDTH);
        // "Par défaut" is longer than "Default".
        let french = rows(Language::French);
        assert!(french[0].width > english[0].width);
        let mut long = rows(Language::French);
        long[0].width = LIST_MIN_WIDTH + 30.0;
        assert_eq!(list_width(&long), LIST_MIN_WIDTH + 30.0);
    }
}
