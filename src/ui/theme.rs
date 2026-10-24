//! The Slate visual style: the tokens of `specs.md` (Visual style), applied to egui.
//!
//! No other module writes a color, a text style or a size shared by the style: they all use
//! these tokens. The components only keep the measures of their own layout.

use egui::{Color32, Context, CornerRadius, Stroke, ThemePreference, Visuals};

use crate::league::LeagueMode;

/// The colors of the interface.
pub mod color {
    use egui::Color32;

    /// Header, price block, status bar.
    pub const BAND_A: Color32 = Color32::from_rgb(0x0f, 0x12, 0x16);
    /// Item block, offers block, settings.
    pub const BAND_B: Color32 = Color32::from_rgb(0x15, 0x1a, 0x20);
    /// Pills, inputs, the gear block.
    pub const CONTROL: Color32 = Color32::from_rgb(0x1d, 0x24, 0x2d);
    /// Input borders, separators, hover of the controls.
    pub const CONTROL_LINE: Color32 = Color32::from_rgb(0x2a, 0x34, 0x40);
    pub const TEXT: Color32 = Color32::from_rgb(0xe3, 0xe8, 0xef);
    pub const WEAK: Color32 = Color32::from_rgb(0x86, 0x92, 0xa3);
    pub const FAINT: Color32 = Color32::from_rgb(0x55, 0x60, 0x70);
    /// The main action: white, so that the colors keep their meaning.
    pub const ACCENT: Color32 = Color32::from_rgb(0xf2, 0xf5, 0xf9);
    /// The icons and marks drawn on the accent.
    pub const ON_ACCENT: Color32 = BAND_A;
    /// The warning band under the header: dark yellow, apart from the league colors.
    pub const WARNING_BAND: Color32 = Color32::from_rgb(0x8a, 0x6a, 0x10);
    pub const WARNING_TEXT: Color32 = Color32::from_rgb(0xf8, 0xed, 0xc2);
}

/// The text styles: the typography of `specs.md`, at the sizes of the mock-up.
pub mod text {
    use egui::FontId;

    use crate::ui::fonts::{self, Weight};

    /// The body text: hints, the league name.
    #[must_use]
    pub fn body() -> FontId {
        fonts::text(13.5, Weight::Regular)
    }

    /// A message in place of the content, e.g. an item that can't be read.
    #[must_use]
    pub fn message() -> FontId {
        fonts::text(14.0, Weight::Regular)
    }

    /// The small texts: the status bar.
    #[must_use]
    pub fn small() -> FontId {
        fonts::text(11.5, Weight::Regular)
    }

    /// The warning band.
    #[must_use]
    pub fn warning() -> FontId {
        fonts::text(12.0, Weight::SemiBold)
    }

    /// The raw text of a copied item, until it is parsed.
    #[must_use]
    pub fn raw_item() -> FontId {
        fonts::numbers(12.0)
    }

    /// The title of a panel, e.g. the settings.
    #[must_use]
    pub fn title() -> FontId {
        fonts::text(18.0, Weight::ExtraBold)
    }

    /// The uppercase label above a group of choices.
    #[must_use]
    pub fn label() -> FontId {
        fonts::text(11.0, Weight::SemiBold)
    }

    /// The name of the application, next to the emblem.
    #[must_use]
    pub fn brand() -> FontId {
        fonts::text(15.0, Weight::ExtraBold)
    }

    /// The uppercase label above the league name, smaller than the other labels so that the
    /// two lines fit in the header.
    #[must_use]
    pub fn league_label() -> FontId {
        fonts::text(9.5, Weight::ExtraBold)
    }

    /// The extra space between the letters of the uppercase labels.
    pub const LABEL_LETTER_SPACING: f32 = 1.3;

    /// The keys to press, drawn as key caps.
    #[must_use]
    pub fn keys() -> FontId {
        fonts::numbers(13.5)
    }
}

/// The sizes of the interface, in points.
pub mod size {
    pub const HEADER_HEIGHT: f32 = 48.0;
    pub const STATUS_BAR_HEIGHT: f32 = 28.0;
    /// Left and right padding of the bands.
    pub const BAND_PADDING: f32 = 14.0;
    /// Rounding of the small information elements; the action blocks stay square.
    pub const RADIUS: u8 = 3;
    /// Width of a square action block (gear, search, refresh).
    pub const ACTION_BLOCK: f32 = 48.0;
    /// Width of the lines of the icons drawn with egui.
    pub const ICON_STROKE: f32 = 1.6;
}

/// The colors of a league block, which say its mode at a glance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LeagueModeColors {
    pub background: Color32,
    /// The left line and the label.
    pub line: Color32,
}

impl LeagueModeColors {
    #[must_use]
    pub fn of(mode: LeagueMode) -> Self {
        match mode {
            LeagueMode::Softcore => Self {
                background: Color32::from_rgb(0x13, 0x26, 0x3a),
                line: Color32::from_rgb(0x6a, 0xa7, 0xde),
            },
            LeagueMode::Hardcore => Self {
                background: Color32::from_rgb(0x35, 0x14, 0x17),
                line: Color32::from_rgb(0xe0, 0x70, 0x6a),
            },
        }
    }
}

/// The background of a block under the pointer: one shade lighter.
#[must_use]
pub fn hovered(background: Color32) -> Color32 {
    background.lerp_to_gamma(color::ACCENT, 0.06)
}

/// Applies the style to every egui widget: dark, flat, with the colors above.
pub fn apply(ctx: &Context) {
    ctx.set_theme(ThemePreference::Dark);

    let mut visuals = Visuals::dark();
    visuals.panel_fill = color::BAND_B;
    visuals.window_fill = color::BAND_B;
    visuals.extreme_bg_color = color::BAND_A;
    visuals.faint_bg_color = color::CONTROL;
    visuals.override_text_color = Some(color::TEXT);
    visuals.selection.bg_fill = color::CONTROL_LINE;
    visuals.selection.stroke = Stroke::new(1.0, color::ACCENT);

    let radius = CornerRadius::same(size::RADIUS);
    let widgets = &mut visuals.widgets;
    for state in [
        &mut widgets.noninteractive,
        &mut widgets.inactive,
        &mut widgets.hovered,
        &mut widgets.active,
        &mut widgets.open,
    ] {
        state.corner_radius = radius;
        state.bg_stroke = Stroke::NONE;
    }
    widgets.inactive.weak_bg_fill = color::CONTROL;
    widgets.inactive.bg_fill = color::CONTROL;
    widgets.hovered.weak_bg_fill = color::CONTROL_LINE;
    widgets.hovered.bg_fill = color::CONTROL_LINE;
    widgets.active.weak_bg_fill = color::CONTROL_LINE;
    widgets.active.bg_fill = color::CONTROL_LINE;

    ctx.set_visuals_of(egui::Theme::Dark, visuals);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_league_mode_has_its_own_colors() {
        assert_ne!(
            LeagueModeColors::of(LeagueMode::Softcore),
            LeagueModeColors::of(LeagueMode::Hardcore)
        );
    }

    #[test]
    fn a_hovered_block_is_lighter() {
        let base = color::CONTROL;
        let lighter = hovered(base);

        assert!(lighter.r() > base.r() && lighter.g() > base.g() && lighter.b() > base.b());
    }
}
