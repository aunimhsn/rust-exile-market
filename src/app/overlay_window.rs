//! The window over the game, drawn by the application beside the main one: when it shows and
//! where, the clicks that close it, the keyboard it takes, and the rules of its frames; the item
//! in it is drawn by the application.

use std::time::Instant;

use egui::{
    Context, Key, Modifiers, Ui, ViewportBuilder, ViewportCommand, ViewportId, WindowLevel,
    X11WindowType, pos2, vec2,
};

use super::desk::{Desk, DeskRequest, DrawnItem};
use crate::filters::Window;
use crate::hotkey::PriceCheckHotkey;
use crate::overlay::{self, Activation, ClickWatch, KeyboardFocus, Overlay, ScreenRect};
use crate::ui;

/// Where the keys go while the window over the game shows, which its status bar says.
#[derive(Debug, Clone, Copy)]
pub(super) struct Keys {
    /// The window took the keyboard, while one of its fields is edited.
    pub(super) held: bool,
    /// The window is the active window, on a single screen.
    pub(super) single_screen: bool,
}

/// The window over the game: when it shows and where, the clicks in the game that close it, and
/// the keyboard it takes from the game.
pub(super) struct OverlayWindow {
    /// When it shows, and where.
    overlay: Overlay,
    /// The clicks in the game, which close it.
    clicks: ClickWatch,
    /// Where it was drawn, in physical pixels, while it is open.
    place: Option<ScreenRect>,
    /// The height of its content at the last frame, in points; none until it is laid out once.
    content_height: Option<f32>,
    /// The keyboard, given to it while one of its fields is edited.
    keyboard: KeyboardFocus,
    /// It is made the active window while the game holds the pointer (a single screen, on
    /// Linux).
    activation: Activation,
}

impl OverlayWindow {
    /// The window, closed, with the thread of the clicks; `wake` is called after each click.
    pub(super) fn start(wake: impl Fn() + Send + 'static) -> Self {
        Self {
            overlay: Overlay::default(),
            clicks: ClickWatch::start(wake),
            place: None,
            content_height: None,
            keyboard: KeyboardFocus::new(),
            activation: Activation::new(),
        }
    }

    /// Takes in the presses of the hotkey, then the clicks, at `now`: each press is told before
    /// its keys, so before its item, and the clicks, pressed before the items not taken in yet,
    /// close the window as it was, not the one these items open.
    pub(super) fn receive(&mut self, hotkey: &mut PriceCheckHotkey, now: Instant) {
        let mut pressed = false;
        hotkey.receive(|press| {
            self.overlay.hotkey(press.target, press.copy, now);
            pressed = true;
        });
        if pressed {
            self.activation.hotkey();
        }
        let place = self.place;
        let clicks_read = self
            .clicks
            .receive(|point| self.overlay.clicked(point, place));
        // Without its clicks, the window couldn't close: it no longer opens.
        if !clicks_read && self.overlay.turn_off() {
            log::warn!(
                "The window over the game is turned off: the clicks that close it can't be read"
            );
        }
    }

    /// An item arrived, which the hotkey that prepared the clipboard at `hotkey` copied, if any:
    /// the window opens on the item of the last hotkey, and closes on any other.
    pub(super) fn copied(&mut self, hotkey: Option<Instant>) {
        let was_open = self.overlay.shown().is_some();
        // A window that opens is laid out once before it shows; an open one keeps its height
        // until the next frame, without blinking.
        if self.overlay.copied(hotkey) && !was_open {
            self.content_height = None;
        }
    }

    /// Follows the window once the items are taken in, at `now`: forgets a hotkey that copied
    /// nothing, makes the window the active one while the game holds the pointer, watches the
    /// clicks while it is open, and asks for the frames it waits for.
    pub(super) fn follow(&mut self, ctx: &Context, hotkey: &PriceCheckHotkey, now: Instant) {
        if let Some(left) = self.overlay.expire(now) {
            ctx.request_repaint_after(left);
        }
        // Asked again soon while it waits for the window to show, the desktop to activate it, or
        // the game to hold the pointer. A press on its way, not received yet, is pending too: its
        // copy makes the game the active window before it is told.
        let waiting = self.overlay.waiting();
        if let Some(after) = self.activation.follow(
            self.overlay.opened(),
            || waiting || hotkey.on_its_way(),
            now,
        ) {
            ctx.request_repaint_after(after);
        }
        let shown = self.overlay.shown().is_some();
        self.clicks.watch(shown);
        let main_visible = ctx.input(|input| input.viewport().visible());
        if paint_while_hidden(shown, main_visible) {
            ctx.send_viewport_cmd(ViewportCommand::RequestPaintWhileHidden);
        }
    }

    /// Draws the window at every frame, shown while it is open: `content` draws its status bar
    /// and the item of `desk` on it, without the title bar, beside the panel of the game the item
    /// is in, as high as its content. What the item asks of the application, if anything.
    ///
    /// Two rules keep it from a panic of egui ("the user callback was never called"), since
    /// eframe 0.37 can't create a window while the main one is minimized (it then paints outside
    /// of its event loop): it is called at every frame of `ui`, so that the window, created
    /// hidden with the first frame, is never destroyed; and the fields of its builder that
    /// eframe can only set by creating the window again (its type, override-redirect, active,
    /// taskbar, decorations) never change. While one of its fields is edited, it has the
    /// keyboard through X11 (`SetInputFocus`, [`KeyboardFocus`]), not through `with_active`.
    /// On Windows, always on top, and shown without activation; a click in it activates it all
    /// the same, and it gives the focus back to the game, unless the click opens a link, the
    /// browser then coming to the front, or goes to a field, which keeps it until it is left.
    pub(super) fn show(
        &mut self,
        ctx: &Context,
        desk: &mut Desk,
        mut content: impl FnMut(&mut Ui, &mut Desk, Keys) -> DrawnItem,
    ) -> Option<DeskRequest> {
        // No window over the game on macOS, where Path of Exile 2 has no client.
        if !cfg!(any(target_os = "linux", target_os = "windows")) {
            return None;
        }
        let target = self.overlay.shown();
        let id = ViewportId::from_hash_of("window-over-the-game");
        // The pointer and the game come in the physical pixels of the screen; the points of the
        // window are scaled by its own screen, which egui-winit converts them back with (on
        // Windows, each screen has its own scale), the main window's until it exists, and by
        // the zoom of the player (Ctrl + plus).
        let native = ctx
            .input_for(id, |input| input.viewport().native_pixels_per_point)
            .or_else(|| ctx.input(|input| input.viewport().native_pixels_per_point));
        let scale = native.unwrap_or(1.0) * ctx.zoom_factor();
        let width = to_pixels(overlay::WIDTH, scale);
        let content_height = self
            .content_height
            .map(|height| height + ui::STATUS_BAR_HEIGHT);
        let place = target.map(|target| {
            target.place(
                width,
                content_height.map_or(target.game.height, |height| to_pixels(height, scale)),
            )
        });
        self.place = place;
        let mut builder = ViewportBuilder::default()
            .with_title(crate::APP_NAME)
            // Out of the hands of the window manager: above the game, which keeps the focus.
            .with_override_redirect(true)
            .with_window_type(X11WindowType::Tooltip)
            .with_decorations(false)
            .with_active(false)
            .with_taskbar(false)
            .with_resizable(false)
            // Shown once its content is laid out, at its height.
            .with_visible(place.is_some() && content_height.is_some());
        // Windows has no override-redirect: the window stays above the game by its level.
        if cfg!(target_os = "windows") {
            builder = builder.with_window_level(WindowLevel::AlwaysOnTop);
        }
        // Hidden where it was: it doesn't move before it hides.
        if let Some(place) = place {
            builder = builder
                .with_position(pos2(to_points(place.x, scale), to_points(place.y, scale)))
                .with_inner_size(vec2(overlay::WIDTH, to_points(place.height, scale)));
        }
        let held = self.keyboard.held();
        // Drawn at every frame, hidden while it is closed (see above).
        let drawn = ctx.show_viewport_immediate(id, builder, |ui, _| {
            // Closed, the hotkey gave the keyboard back to the game, or the window lost it
            // (Alt+Tab, which egui doesn't take for leaving the field; on Windows, whatever
            // took the focus, even while no field had it; only a window that had it loses it;
            // on a single screen, as soon as it is no longer the active window, before winit
            // tells it):
            // no field is edited, so that the status bar doesn't say the keys go to the window,
            // and a window that opens again doesn't take the keyboard. The number typed is
            // settled, as leaving its field does.
            let left = self.overlay.take_leave_field();
            let lost = self.activation.lost()
                || ui.input(|input| {
                    input
                        .events
                        .iter()
                        .any(|event| matches!(event, egui::Event::WindowFocused(false)))
                });
            if (place.is_none() || left || lost) && ui.ctx().text_edit_focused() {
                desk.settle_edited(Window::OverTheGame);
                ui.ctx().memory_mut(egui::Memory::stop_text_input);
            }
            #[cfg(target_os = "windows")]
            if place.is_none() {
                self.keyboard.give_back(overlay::Frame::HIDDEN);
            }
            place?;
            // On a single screen, where the window is the active one, the keys are the game's
            // (Tab its map, Space its dodge, 1 to 5 its flasks and charms) unless a field is
            // edited: only Esc does something, below, so that they neither move the keyboard
            // in the window nor press its buttons nor type in its fields.
            if self.activation.escape_closes() {
                keep_only_esc(ui);
            }
            let keys = Keys {
                held: self.keyboard.held(),
                single_screen: self.activation.holds(),
            };
            // Its own ids: the scroll and the panels of the main window are its own.
            let drawn = ui
                .push_id("window-over-the-game", |ui| content(ui, desk, keys))
                .inner;
            let editing = ui.ctx().text_edit_focused();
            // On a single screen, where the window is the active one, Esc closes it, after what
            // is on top of it (the tooltip of an offer, which takes the key), and unless it
            // leaves a field: edited at the last frame, since egui takes the keyboard away at
            // Esc before this one.
            if self.activation.escape_closes()
                && ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape))
            {
                self.activation.escape();
                self.overlay.close();
                ui.ctx().request_repaint();
            }
            // A click in it took the focus of the game: given back once it is released,
            // after the content, which may open a link with this click; kept while a field is
            // edited.
            #[cfg(target_os = "windows")]
            self.keyboard.give_back(overlay_frame(ui, editing));
            Some((drawn, editing))
        });
        let editing = drawn.as_ref().is_some_and(|(_, editing)| *editing);
        self.activation.editing(editing);
        if !editing {
            desk.forget_edited(Window::OverTheGame);
        }
        // The status bar, drawn before the fields, says it at the next frame: on Windows, the
        // window took the keyboard during the frame already. On a single screen, the window
        // already has the keyboard while it is the active window.
        if self.keyboard.follow(editing && !self.activation.holds()) != held {
            ctx.request_repaint();
        }
        let (drawn, _) = drawn?;
        if self.content_height != Some(drawn.height) {
            self.content_height = Some(drawn.height);
            ctx.request_repaint();
        }
        drawn.request
    }
}

/// Whether the interface must run while the main window is hidden: while the window over the
/// game shows, which eframe only draws with the main one. Not while the main window shows: the
/// command also asks for the next frame, which would redraw both windows at every frame.
fn paint_while_hidden(overlay_shown: bool, main_visible: Option<bool>) -> bool {
    overlay_shown && main_visible == Some(false)
}

/// Leaves only Esc to the frame of the window over the game, before its content: the keys of the
/// game neither move the keyboard in it (Tab, the arrows, which egui takes in before the frame),
/// nor press what has it (Space, Enter), nor type.
fn keep_only_esc(ui: &Ui) {
    ui.ctx()
        .memory_mut(|memory| memory.move_focus(egui::FocusDirection::None));
    ui.input_mut(|input| {
        input.events.retain(|event| match event {
            egui::Event::Key { key, .. } => *key == Key::Escape,
            egui::Event::Text(_) | egui::Event::Paste(_) | egui::Event::Copy | egui::Event::Cut => {
                false
            }
            _ => true,
        });
    });
}

/// What a frame of the window over the game saw once its content is drawn, for the focus given
/// back to the game on Windows: the content may open a link with the click (Check on trade), the
/// browser then coming to the front instead; `editing`, whether one of its fields is edited.
#[cfg(target_os = "windows")]
fn overlay_frame(ui: &Ui, editing: bool) -> overlay::Frame {
    let (focused, pressed) = ui.input(|input| {
        (
            input.viewport().focused == Some(true),
            input.pointer.any_down(),
        )
    });
    let opens_link = ui.ctx().output(|output| {
        output
            .commands
            .iter()
            .any(|command| matches!(command, egui::OutputCommand::OpenUrl(_)))
    });
    overlay::Frame {
        focused: Some(focused),
        pressed,
        opens_link,
        editing,
    }
}

/// `points` in the physical pixels of a screen of `scale` pixels per point.
#[expect(
    clippy::cast_possible_truncation,
    reason = "a size of the screen, far within i32"
)]
fn to_pixels(points: f32, scale: f32) -> i32 {
    (points * scale).ceil() as i32
}

/// `pixels` of the screen in points, for a screen of `scale` pixels per point.
#[expect(
    clippy::cast_precision_loss,
    reason = "a position of the screen, far within f32"
)]
fn to_points(pixels: i32, scale: f32) -> f32 {
    pixels as f32 / scale
}

#[cfg(test)]
mod tests;
