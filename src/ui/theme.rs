//! The Slate visual style: the tokens of `specs.md` (Visual style), applied to egui.
//!
//! No other module writes a color, a text style or a size shared by the style: they all use
//! these tokens. The components only keep the measures of their own layout.

use egui::style::{WidgetVisuals, Widgets};
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
    /// The implicit and explicit modifiers, as in the game.
    pub const MODIFIER: Color32 = Color32::from_rgb(0x8f, 0x8f, 0xff);
    /// The attributes, as in the game, in the requirements and on the defences they raise
    /// (armour, evasion, energy shield); a little apart from the rise and fall colors.
    pub const STRENGTH: Color32 = Color32::from_rgb(0xe0, 0x47, 0x3f);
    pub const DEXTERITY: Color32 = Color32::from_rgb(0x3f, 0xbf, 0x4f);
    pub const INTELLIGENCE: Color32 = Color32::from_rgb(0x5a, 0xa7, 0xf0);
    /// A modifier at its best tier: a vivid orange that stands out.
    pub const TIER_1: Color32 = Color32::from_rgb(0xff, 0x8c, 0x1a);
    /// The origin of a modifier, close to the colors of the game: the green of the dexterity
    /// for a desecrated one, a gold for a fractured one, a cyan for a crafted one.
    pub const DESECRATED: Color32 = DEXTERITY;
    pub const FRACTURED: Color32 = Color32::from_rgb(0xc9, 0xad, 0x7a);
    pub const CRAFTED: Color32 = Color32::from_rgb(0x7c, 0xc7, 0xd9);
    /// The Vaal modifier of a corrupted unique, in the corrupted red of the game.
    pub const VAAL: Color32 = Color32::from_rgb(0xe0, 0x42, 0x3a);
    /// The label of the Runic Ward of a runeforged base, a defence without attribute: a sky
    /// blue lighter than the energy shield blue, written next to it.
    pub const RUNIC_WARD: Color32 = Color32::from_rgb(0x8a, 0xcb, 0xf6);
    /// The band of an unrevealed modifier: 12 % of the desecrated green (`#3fbf4f`),
    /// premultiplied, as in the mock-up.
    pub const UNREVEALED_BAND: Color32 = Color32::from_rgba_premultiplied(8, 23, 9, 31);
    /// The background of the hardcore leagues, also behind the lit Corrupted tag.
    pub const HARDCORE: Color32 = Color32::from_rgb(0x35, 0x14, 0x17);
    /// A state tag lit as a filter: its text in a light color of its state on a dark
    /// background of the same hue, of the lightness and saturation of the hardcore red, and
    /// that background lighter under the pointer.
    pub const CORRUPTED_STATE: Color32 = Color32::from_rgb(0xff, 0x6b, 0x5e);
    pub const CORRUPTED_STATE_BG: Color32 = HARDCORE;
    pub const CORRUPTED_STATE_HOVER: Color32 = Color32::from_rgb(0x4a, 0x1b, 0x20);
    pub const FRACTURED_STATE: Color32 = FRACTURED;
    pub const FRACTURED_STATE_BG: Color32 = Color32::from_rgb(0x35, 0x29, 0x14);
    pub const FRACTURED_STATE_HOVER: Color32 = Color32::from_rgb(0x4a, 0x39, 0x1b);
    pub const SANCTIFIED_STATE: Color32 = Color32::from_rgb(0xec, 0xd9, 0x8a);
    pub const SANCTIFIED_STATE_BG: Color32 = Color32::from_rgb(0x35, 0x2f, 0x14);
    pub const SANCTIFIED_STATE_HOVER: Color32 = Color32::from_rgb(0x4a, 0x41, 0x1b);
    pub const MIRRORED_STATE: Color32 = Color32::from_rgb(0xa9, 0xc6, 0xff);
    pub const MIRRORED_STATE_BG: Color32 = Color32::from_rgb(0x14, 0x1f, 0x35);
    pub const MIRRORED_STATE_HOVER: Color32 = Color32::from_rgb(0x1b, 0x2b, 0x4a);
    /// A price that rises or falls.
    pub const RISE: Color32 = Color32::from_rgb(0x5f, 0xcf, 0x80);
    pub const FALL: Color32 = Color32::from_rgb(0xe6, 0x6a, 0x5c);
    /// An uncertain price.
    pub const WARNING: Color32 = Color32::from_rgb(0xf0, 0xb8, 0x5a);
    /// Every other row of the offers, on band B.
    pub const ROW_ALT: Color32 = Color32::from_rgb(0x1a, 0x20, 0x28);
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

    /// The lines of the item block: its base type, its class, its base values, its
    /// requirements.
    #[must_use]
    pub fn item_line() -> FontId {
        fonts::text(13.0, Weight::Regular)
    }

    /// The number of a base value of the item block, which stands out.
    #[must_use]
    pub fn base_value() -> FontId {
        fonts::text(13.0, Weight::ExtraBold)
    }

    /// A tag of the item block.
    #[must_use]
    pub fn tag() -> FontId {
        fonts::text(12.0, Weight::Regular)
    }

    /// The range of a roll, under its stat.
    #[must_use]
    pub fn roll() -> FontId {
        fonts::text(11.0, Weight::Regular)
    }

    /// The minimum and the maximum of a filter, in its range field: 1 point smaller than the
    /// other numbers of the stats, so that the text of the stats keeps its width.
    #[must_use]
    pub fn range_field() -> FontId {
        fonts::numbers(11.5)
    }

    /// A note under the stats, e.g. an unrevealed modifier, or under the invitation of the
    /// empty window.
    #[must_use]
    pub fn note() -> FontId {
        fonts::text(12.0, Weight::Regular)
    }

    /// The tier of a modifier, before its range.
    #[must_use]
    pub fn tier() -> FontId {
        fonts::text(11.0, Weight::ExtraBold)
    }

    /// The origin of a modifier, after the range of its tier.
    #[must_use]
    pub fn origin() -> FontId {
        fonts::text(11.0, Weight::SemiBold)
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

    /// The name of the item in the tooltip of an offer.
    #[must_use]
    pub fn tooltip_name() -> FontId {
        fonts::text(14.0, Weight::ExtraBold)
    }

    /// The tier of a waystone in the tooltip of an offer, after its base.
    #[must_use]
    pub fn tooltip_tier() -> FontId {
        fonts::text(12.0, Weight::ExtraBold)
    }

    /// The kind of another quality after the price of an offer, as the tier of a waystone.
    #[must_use]
    pub fn other_quality_kind() -> FontId {
        fonts::text(12.0, Weight::ExtraBold)
    }

    /// A modifier line in the tooltip of an offer.
    #[must_use]
    pub fn tooltip_line() -> FontId {
        fonts::text(12.5, Weight::Regular)
    }

    /// The difference of a roll from the copied item, in the tooltip of an offer.
    #[must_use]
    pub fn difference() -> FontId {
        fonts::numbers(12.0)
    }

    /// The uppercase titles of the columns of the offers.
    #[must_use]
    pub fn column() -> FontId {
        fonts::text(11.0, Weight::Regular)
    }

    /// The extra space between the letters of the titles of the columns.
    pub const COLUMN_LETTER_SPACING: f32 = 0.9;

    /// The countdown of the waiting block.
    #[must_use]
    pub fn countdown() -> FontId {
        fonts::text(13.0, Weight::SemiBold)
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
    /// Rounding of the scroll bar: its ends nearly square once it widens under the pointer
    /// (thin at rest, they are round whatever the radius).
    pub const SCROLL_BAR_RADIUS: u8 = 1;
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

    round_widgets(&mut visuals, size::RADIUS);
    let widgets = &mut visuals.widgets;
    for state in widget_states(widgets) {
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

/// Rounds the widgets of `visuals` by `radius`, in every state.
pub fn round_widgets(visuals: &mut Visuals, radius: u8) {
    let radius = CornerRadius::same(radius);
    for state in widget_states(&mut visuals.widgets) {
        state.corner_radius = radius;
    }
}

/// The looks of the widgets in each of their states.
fn widget_states(widgets: &mut Widgets) -> [&mut WidgetVisuals; 5] {
    [
        &mut widgets.noninteractive,
        &mut widgets.inactive,
        &mut widgets.hovered,
        &mut widgets.active,
        &mut widgets.open,
    ]
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
