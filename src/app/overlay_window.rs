//! The window over the game, drawn by the application beside the main one: the rules of its
//! frames, apart from the drawing of the item.

use egui::Key;

#[cfg(target_os = "windows")]
use crate::overlay;

/// Whether the interface must run while the main window is hidden: while the window over the
/// game shows, which eframe only draws with the main one. Not while the main window shows: the
/// command also asks for the next frame, which would redraw both windows at every frame.
pub(super) fn paint_while_hidden(overlay_shown: bool, main_visible: Option<bool>) -> bool {
    overlay_shown && main_visible == Some(false)
}

/// Leaves only Esc to the frame of the window over the game, before its content: the keys of the
/// game neither move the keyboard in it (Tab, the arrows, which egui takes in before the frame),
/// nor press what has it (Space, Enter), nor type.
pub(super) fn keep_only_esc(ui: &egui::Ui) {
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
pub(super) fn overlay_frame(ui: &egui::Ui, editing: bool) -> overlay::Frame {
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
pub(super) fn to_pixels(points: f32, scale: f32) -> i32 {
    (points * scale).ceil() as i32
}

/// `pixels` of the screen in points, for a screen of `scale` pixels per point.
#[expect(
    clippy::cast_precision_loss,
    reason = "a position of the screen, far within f32"
)]
pub(super) fn to_points(pixels: i32, scale: f32) -> f32 {
    pixels as f32 / scale
}

#[cfg(test)]
mod tests;
