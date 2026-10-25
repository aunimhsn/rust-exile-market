//! The interface: the visual style and the components of the window.
//!
//! Each component borrows the data it shows and draws itself; the state lives in the
//! application ([`crate::app`]).

mod empty_state;
mod fonts;
mod images;
mod league_list;
mod new_league_bubble;
mod raw_item;
mod resize_edges;
mod settings_panel;
mod status_bar;
mod theme;
mod title_bar;
mod unreadable_item;
mod warning_band;
mod widgets;

pub use empty_state::{EmptyState, PriceDownload};
pub use league_list::{LeagueList, LeagueListAction};
pub use new_league_bubble::{BubbleAction, NewLeagueBubble};
pub use raw_item::RawItem;
pub use resize_edges::ResizeEdges;
pub use settings_panel::SettingsPanel;
pub use status_bar::StatusBar;
pub use title_bar::{TitleBar, TitleBarAction};
pub use unreadable_item::UnreadableItem;
pub use warning_band::WarningBand;

/// Sets up the image loaders, the fonts and the style, once, before the first frame.
pub fn install(ctx: &egui::Context) {
    egui_extras::install_image_loaders(ctx);
    fonts::install(ctx);
    theme::apply(ctx);
}
