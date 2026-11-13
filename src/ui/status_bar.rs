//! The status bar: the age of each data source, or, while the settings are open, the version
//! and the author, with a link to the author's site (and to a newer version, when one is out).
//! In the window over the game, the league on its right, and while a field of it has the
//! keyboard, a line that says so in place of the ages.

use egui::{
    Align, CursorIcon, Frame, Label, Layout, OpenUrl, Panel, Rect, RichText, Role, Sense, Stroke,
    Ui, WidgetInfo, pos2,
};

use std::time::Duration;

use super::theme::{LeagueModeColors, color, size, text};
use super::widgets;
use crate::i18n::{Message, Translator};
use crate::league::League;
use crate::session::PricesState;
use crate::update::Release;

/// The state of the trade site in the status bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradeStatus {
    /// No search yet in this session.
    Idle,
    /// The age of the last search.
    Searched(Duration),
    /// A long wait before the next search.
    Waiting(Duration),
}

/// What the status bar shows.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Line<'a> {
    Sources(PricesState, TradeStatus),
    /// With the newer version, if one is out.
    Author(Option<&'a Release>),
    /// The keys go to the window over the game, not to the game.
    Keyboard,
    /// The window over the game is the active window on a single screen: the keys go to it.
    KeyboardSingleScreen,
}

/// The space on each side of the league name.
const LEAGUE_PADDING: f32 = 10.0;

pub struct StatusBar<'a> {
    i18n: &'a Translator,
    line: Line<'a>,
    league: Option<&'a League>,
}

impl<'a> StatusBar<'a> {
    /// The age of each data source.
    #[must_use]
    pub fn sources(i18n: &'a Translator, prices: PricesState, trade: TradeStatus) -> Self {
        Self {
            i18n,
            line: Line::Sources(prices, trade),
            league: None,
        }
    }

    /// With `league` on the right, on the color of its trade and the whole height of the bar,
    /// not clickable: in the window over the game, which has no title bar.
    #[must_use]
    pub fn league(mut self, league: Option<&'a League>) -> Self {
        self.league = league;
        self
    }

    /// In place of the ages of the sources while `held`: the window over the game has the
    /// keyboard, so that the game gets no keys, and Enter or Esc gives it back.
    #[must_use]
    pub fn keyboard(mut self, held: bool) -> Self {
        if held {
            self.line = Line::Keyboard;
        }
        self
    }

    /// In place of the ages of the sources while `active`: the window over the game is the active
    /// window on a single screen, so that the game gets no keys, and Esc or a click outside gives
    /// them back. Takes the place of the line of [`Self::keyboard`], which says less.
    #[must_use]
    pub fn single_screen(mut self, active: bool) -> Self {
        if active {
            self.line = Line::KeyboardSingleScreen;
        }
        self
    }

    /// The version and the author, while the settings are open, and `update`, a newer version
    /// out, as a link after the running one.
    #[must_use]
    pub fn author(i18n: &'a Translator, update: Option<&'a Release>) -> Self {
        Self {
            i18n,
            line: Line::Author(update),
            league: None,
        }
    }

    pub fn show(self, ui: &mut Ui) {
        Panel::bottom("status-bar")
            .exact_size(size::STATUS_BAR_HEIGHT)
            .frame(Frame::NONE.fill(color::BAND_A))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                let league_width = self
                    .league
                    .map_or(0.0, |league| Self::league_block(ui, league));
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    // The padding of the band on both sides, or before the league.
                    let right = if self.league.is_some() {
                        league_width + LEAGUE_PADDING
                    } else {
                        size::BAND_PADDING
                    };
                    ui.set_max_width(ui.available_width() - right);
                    ui.add_space(size::BAND_PADDING);
                    match self.line {
                        Line::Sources(prices, trade) => self.sources_line(ui, prices, trade),
                        Line::Author(update) => self.author_line(ui, update),
                        Line::Keyboard => self.keyboard_line(ui, Message::StatusKeyboard),
                        Line::KeyboardSingleScreen => {
                            self.keyboard_line(ui, Message::StatusKeyboardSingleScreen);
                        }
                    }
                });
            });
    }

    /// Where the keys go, in the text color: the game no longer gets the keys the player types.
    fn keyboard_line(&self, ui: &mut Ui, message: Message) {
        let line = self.i18n.text(message);
        ui.add(Label::new(small(line).color(color::TEXT)).truncate());
    }

    /// The league name in the text color on the color of its trade, at the right of the bar and
    /// its whole height, cut past half of it; its width.
    fn league_block(ui: &Ui, league: &League) -> f32 {
        let bar = ui.max_rect();
        // At most half of the bar, cut with "…": the ages of the sources keep the rest.
        let galley = widgets::truncated_line(ui, league.name(), text::small(), bar.width() / 2.0);
        let width = galley.size().x + 2.0 * LEAGUE_PADDING;
        let block = Rect::from_min_max(pos2(bar.right() - width, bar.top()), bar.right_bottom());
        let painter = ui.painter();
        painter.rect_filled(block, 0.0, LeagueModeColors::of(league.mode()).background);
        let top = block.center().y - galley.size().y / 2.0;
        painter.galley(
            pos2(block.left() + LEAGUE_PADDING, top),
            galley,
            color::TEXT,
        );
        width
    }

    /// The age of the prices and of the last trade site search, or the long wait before the
    /// next one.
    fn sources_line(&self, ui: &mut Ui, prices: PricesState, trade: TradeStatus) {
        let i18n = self.i18n;
        let prices = match prices {
            PricesState::Downloading { .. } => i18n.text(Message::StatusPricesDownloading),
            PricesState::Unavailable => i18n.text(Message::StatusPricesUnavailable),
            PricesState::Known {
                age,
                offline: false,
            } => i18n.status_prices(&i18n.age(age)),
            PricesState::Known { age, offline: true } => i18n.status_prices_offline(&i18n.age(age)),
        };
        let trade = match trade {
            TradeStatus::Idle => i18n.text(Message::StatusTradeIdle),
            TradeStatus::Searched(age) => i18n.status_trade(&i18n.age(age)),
            TradeStatus::Waiting(wait) => i18n.status_trade_wait(wait),
        };
        let line = i18n.status_bar(&prices, &trade);
        // Cut with "…" in a narrow window, and shown whole under the pointer.
        ui.add(Label::new(faint(line)).truncate());
    }

    /// The version and the author, then the site as a link: lighter, white and underlined
    /// under the pointer; `update`, a newer version, as a link of the same look after the
    /// running version.
    fn author_line(&self, ui: &mut Ui, update: Option<&Release>) {
        let site = crate::AUTHOR_SITE_LABEL;
        let author = self.i18n.author_line();
        // The translations start with the version and end with the site (checked by the tests
        // of `i18n`).
        let running = format!("{} {}", crate::APP_NAME, crate::VERSION);
        let Some(middle) = author
            .strip_prefix(&running)
            .and_then(|rest| rest.strip_suffix(site))
        else {
            ui.add(Label::new(faint(author)).truncate());
            return;
        };
        ui.add(Label::new(faint(running)));
        if let Some(release) = update {
            ui.add(Label::new(faint(text::SEPARATOR)));
            let label = self.i18n.status_update(&release.version.to_string());
            link(ui, &label, &release.page);
        }
        // In a narrow window, the middle is cut with "…" so that the site stays whole.
        let site_width = widgets::text_line(ui, site, text::small()).size().x;
        let room = (ui.available_width() - site_width).max(0.0);
        ui.scope(|ui| {
            ui.set_max_width(room);
            ui.add(Label::new(faint(middle)).truncate());
        });
        link(ui, site, crate::AUTHOR_SITE);
    }
}

/// `label` as a link to `url`: gray, lighter and underlined under the pointer.
fn link(ui: &mut Ui, label: &str, url: &str) {
    let galley = widgets::text_line(ui, label, text::small());
    let (rect, response) = ui.allocate_exact_size(galley.size(), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(Role::Link, true, label));
    let lit = response.hovered() || response.has_focus();
    let tint = if lit { color::TEXT } else { color::WEAK };
    let painter = ui.painter();
    painter.galley(rect.min, galley, tint);
    if lit {
        painter.hline(rect.x_range(), rect.bottom(), Stroke::new(1.0, tint));
    }
    if response.on_hover_cursor(CursorIcon::PointingHand).clicked() {
        ui.ctx().open_url(OpenUrl::new_tab(url));
    }
}

/// A line of the bar in its small face.
fn small(line: impl Into<String>) -> RichText {
    RichText::new(line).font(text::small())
}

/// A line of the bar in its small face, faint.
fn faint(line: impl Into<String>) -> RichText {
    small(line).color(color::FAINT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Context, RawInput, pos2, vec2};

    /// The width of the league block in a status bar of the narrow window.
    fn league_width(name: &str) -> (f32, f32) {
        let ctx = Context::default();
        crate::ui::install(&ctx);
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(420.0, 28.0))),
            ..Default::default()
        };
        let mut widths = (0.0, 0.0);
        crate::ui::fixtures::run_frame(&ctx, input, |ui| {
            widths = (
                StatusBar::league_block(ui, &League::new(name)),
                ui.max_rect().width(),
            );
        });
        widths
    }

    #[test]
    fn the_keyboard_held_takes_the_place_of_the_sources() {
        let i18n = Translator::new(crate::language::Language::English);
        let bar = |held| {
            StatusBar::sources(&i18n, PricesState::Unavailable, TradeStatus::Idle)
                .keyboard(held)
                .line
        };
        assert_eq!(bar(true), Line::Keyboard);
        assert_eq!(
            bar(false),
            Line::Sources(PricesState::Unavailable, TradeStatus::Idle)
        );
    }

    #[test]
    fn on_a_single_screen_the_line_says_esc_closes_even_while_a_field_is_edited() {
        let i18n = Translator::new(crate::language::Language::English);
        let bar = |held, active| {
            StatusBar::sources(&i18n, PricesState::Unavailable, TradeStatus::Idle)
                .keyboard(held)
                .single_screen(active)
                .line
        };
        assert_eq!(bar(false, true), Line::KeyboardSingleScreen);
        assert_eq!(bar(true, true), Line::KeyboardSingleScreen);
        assert_eq!(bar(true, false), Line::Keyboard);
    }

    #[test]
    fn a_short_league_name_takes_its_width_and_the_padding() {
        let (block, bar) = league_width("Standard");

        assert!(block > 2.0 * LEAGUE_PADDING);
        assert!(block < bar / 2.0);
    }

    /// The width of the line of the settings with a newer version, in a bar of `width`.
    fn author_line_width(width: f32) -> f32 {
        let ctx = Context::default();
        crate::ui::install(&ctx);
        let i18n = Translator::new(crate::language::Language::French);
        // A version of two digits, the longest of the next ones.
        let release = Release {
            version: crate::update::Version::parse("0.10.1").expect("a version"),
            page: String::new(),
        };
        let mut line = 0.0;
        crate::ui::fixtures::run_frame(&ctx, RawInput::default(), |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                ui.set_max_width(width);
                StatusBar::author(&i18n, Some(&release)).author_line(ui, Some(&release));
                line = ui.min_rect().width();
            });
        });
        line
    }

    #[test]
    fn the_line_of_the_settings_with_an_update_fits_the_narrow_window() {
        // The narrowest window, less the padding of the band on both sides.
        let width = crate::MIN_WINDOW_SIZE[0] - 2.0 * size::BAND_PADDING;

        let line = author_line_width(width);
        assert!(line <= width + 0.5, "{line} of {width}");
        // Narrower still, the middle is cut and the site stays.
        let narrow = author_line_width(250.0);
        assert!(narrow <= 250.5, "{narrow}");
    }

    #[test]
    fn a_long_league_name_is_cut_at_half_of_the_bar() {
        let (block, bar) =
            league_width("Forbidden Rites of the Very Long Name of a Private League With Friends");

        assert!(
            block <= bar / 2.0 + 2.0 * LEAGUE_PADDING + 1.0,
            "{block} of {bar}"
        );
    }
}
