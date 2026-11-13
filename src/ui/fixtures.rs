//! What the tests of the interface share.

/// Runs one frame of `ctx` without a screen, drawn by `run` from `input`; what it drew.
pub fn run_frame(
    ctx: &egui::Context,
    input: egui::RawInput,
    run: impl FnMut(&mut egui::Ui),
) -> egui::FullOutput {
    let mut output = ctx.run_ui(input, run);
    // epaint panics when the texture changes of a frame are dropped unapplied.
    output.textures_delta.clear();
    output
}
