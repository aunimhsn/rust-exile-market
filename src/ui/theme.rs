//! The Slate visual style: the tokens of `specs.md` (Visual style), applied to egui.
//!
//! No other module writes a color, a text style or a size shared by the style: they all use
//! these tokens. The components only keep the measures of their own layout.

use egui::{Color32, Context, CornerRadius, Shadow, Stroke, ThemePreference, Visuals};

use crate::league::LeagueMode;

/// The colors of the interface.
pub mod color {
    use egui::Color32;

    /// Title bar, price block, status bar.
    pub const BAND_A: Color32 = Color32::from_rgb(0x0f, 0x12, 0x16);
    /// Item block, offers block, settings.
    pub const BAND_B: Color32 = Color32::from_rgb(0x15, 0x1a, 0x20);
    /// Pills, inputs, check boxes, key caps.
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
    /// The warning band under the title bar: dark yellow, apart from the league colors.
    pub const WARNING_BAND: Color32 = Color32::from_rgb(0x8a, 0x6a, 0x10);
    pub const WARNING_TEXT: Color32 = Color32::from_rgb(0xf8, 0xed, 0xc2);
    /// The bubbles that open over the content: the tooltip of an offer, a new league.
    pub const BUBBLE: Color32 = Color32::from_rgb(0x20, 0x27, 0x2f);
    /// The emblem behind the empty state: white at 2.25 %, rounded to the nearest alpha.
    pub const WATERMARK: Color32 = Color32::from_rgba_premultiplied(6, 6, 6, 6);

    /// The item rarities, as in the game.
    pub const NORMAL: Color32 = Color32::from_rgb(0xd0, 0xd0, 0xd0);
    pub const MAGIC: Color32 = Color32::from_rgb(0x88, 0x88, 0xff);
    pub const RARE: Color32 = Color32::from_rgb(0xff, 0xff, 0x77);
    pub const UNIQUE: Color32 = Color32::from_rgb(0xc2, 0x6a, 0x2c);
    pub const CURRENCY: Color32 = Color32::from_rgb(0xb4, 0xa7, 0x87);
    pub const GEM: Color32 = Color32::from_rgb(0x1b, 0xa2, 0x9b);
    /// A modifier at its best tier, and the T1 tag: a vivid orange that stands out.
    pub const TIER_1: Color32 = Color32::from_rgb(0xff, 0x8c, 0x1a);
    /// The background of the hardcore leagues, also behind the Corrupted tag.
    pub const HARDCORE: Color32 = Color32::from_rgb(0x35, 0x14, 0x17);
    /// The Corrupted tag: white on the background of the hardcore leagues.
    pub const CORRUPTED_TAG: Color32 = HARDCORE;
    pub const CORRUPTED_TAG_TEXT: Color32 = Color32::WHITE;
    /// A price that rises or falls.
    pub const RISE: Color32 = Color32::from_rgb(0x5f, 0xcf, 0x80);
    pub const FALL: Color32 = Color32::from_rgb(0xe6, 0x6a, 0x5c);
    /// An uncertain price.
    pub const WARNING: Color32 = Color32::from_rgb(0xf0, 0xb8, 0x5a);
    /// Every other row of a list on band A: a light veil.
    pub const ROW_VEIL: Color32 = Color32::from_rgba_premultiplied(8, 8, 8, 8);
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

    /// The text of a bubble.
    #[must_use]
    pub fn bubble() -> FontId {
        fonts::text(12.5, Weight::Regular)
    }

    /// The name in the text of a bubble.
    #[must_use]
    pub fn bubble_strong() -> FontId {
        fonts::text(12.5, Weight::ExtraBold)
    }

    /// The label of a pill.
    #[must_use]
    pub fn pill() -> FontId {
        fonts::text(12.0, Weight::SemiBold)
    }

    /// The league name, in the title bar.
    #[must_use]
    pub fn league() -> FontId {
        fonts::text(13.0, Weight::Regular)
    }

    /// The extra space between the letters of the uppercase labels.
    pub const LABEL_LETTER_SPACING: f32 = 1.3;

    /// The name of an item, in its rarity color.
    #[must_use]
    pub fn item_name() -> FontId {
        fonts::text(20.0, Weight::ExtraBold)
    }

    /// The lines of the item block: its base type, its class, its summary.
    #[must_use]
    pub fn item_line() -> FontId {
        fonts::text(13.0, Weight::Regular)
    }

    /// A tag of the item block, and its requirements.
    #[must_use]
    pub fn tag() -> FontId {
        fonts::text(12.0, Weight::Regular)
    }

    /// An amount of the price block, beside its orb.
    #[must_use]
    pub fn price() -> FontId {
        fonts::numbers(19.0)
    }

    /// An amount of a list, e.g. the price of a possible unique.
    #[must_use]
    pub fn list_amount() -> FontId {
        fonts::numbers(13.0)
    }

    /// The keys to press, drawn as key caps.
    #[must_use]
    pub fn keys() -> FontId {
        fonts::numbers(13.5)
    }
}

/// The sizes of the interface, in points.
pub mod size {
    /// The title bar, in place of the one of the system.
    pub const TITLE_BAR_HEIGHT: f32 = 28.0;
    pub const STATUS_BAR_HEIGHT: f32 = 28.0;
    /// Left and right padding of the bands.
    pub const BAND_PADDING: f32 = 14.0;
    /// Rounding of the small information elements; the action blocks stay square.
    pub const RADIUS: u8 = 3;
    /// Width of the clickable zone of an icon of the title bar, drawn without a block.
    pub const ICON_ZONE: f32 = 36.0;
    /// Rounding of the bubbles that open over the content.
    pub const BUBBLE_RADIUS: u8 = 4;
    /// Width of the lines of the icons drawn with egui.
    pub const ICON_STROKE: f32 = 1.6;
}

/// The durations of the motions, in seconds.
pub mod motion {
    /// A list that opens or closes, and its chevron.
    pub const OPEN: f32 = 0.15;
    /// The bar of a download, from one step to the next.
    pub const PROGRESS: f32 = 0.3;
}

/// The shadow of what opens over the content: the bubbles, the list of the leagues.
#[must_use]
pub fn floating_shadow() -> Shadow {
    Shadow {
        offset: [0, 10],
        blur: 30,
        spread: 0,
        color: Color32::from_black_alpha(153),
    }
}

/// The colors of a league block, the only sign of its mode on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LeagueModeColors {
    pub background: Color32,
    /// The background under the pointer.
    pub hovered: Color32,
}

impl LeagueModeColors {
    #[must_use]
    pub fn of(mode: LeagueMode) -> Self {
        match mode {
            LeagueMode::Softcore => Self {
                background: Color32::from_rgb(0x13, 0x26, 0x3a),
                hovered: Color32::from_rgb(0x1a, 0x33, 0x50),
            },
            LeagueMode::Hardcore => Self {
                background: color::HARDCORE,
                hovered: Color32::from_rgb(0x4a, 0x1b, 0x20),
            },
        }
    }
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
    fn a_league_block_is_lighter_under_the_pointer() {
        for mode in [LeagueMode::Softcore, LeagueMode::Hardcore] {
            let colors = LeagueModeColors::of(mode);
            let (base, lit) = (colors.background, colors.hovered);

            assert!(
                lit.r() > base.r() && lit.g() > base.g() && lit.b() > base.b(),
                "{mode:?}"
            );
        }
    }
}
