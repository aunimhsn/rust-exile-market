//! The interface: the visual style and the components of the window.
//!
//! Each component borrows the data it shows and draws itself; the state lives in the
//! application ([`crate::app`]).

mod empty_state;
mod fonts;
mod header;
mod images;
mod raw_item;
mod status_bar;
mod theme;
mod unreadable_item;
mod warning_band;
mod widgets;

pub use empty_state::EmptyState;
pub use header::Header;
pub use raw_item::RawItem;
pub use status_bar::StatusBar;
pub use unreadable_item::UnreadableItem;
pub use warning_band::WarningBand;

/// Sets up the image loaders, the fonts and the style, once, before the first frame.
pub fn install(ctx: &egui::Context) {
    egui_extras::install_image_loaders(ctx);
    fonts::install(ctx);
    theme::apply(ctx);
}
