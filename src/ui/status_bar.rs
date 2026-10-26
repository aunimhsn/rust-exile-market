//! The status bar: the age of each data source, or, while the settings are open, the version
//! and the author, with a link to the author's site.

use egui::{
    Align, CursorIcon, Frame, Label, Layout, OpenUrl, Panel, RichText, Role, Sense, Stroke, Ui,
    WidgetInfo,
};

use std::time::Duration;

use super::theme::{color, size, text};
use super::widgets;
use crate::i18n::{Message, Translator};
use crate::session::PricesState;

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
enum Line {
    Sources(PricesState, TradeStatus),
    Author,
}

pub struct StatusBar<'a> {
    i18n: &'a Translator,
    line: Line,
}

impl<'a> StatusBar<'a> {
    /// The age of each data source.
    #[must_use]
    pub fn sources(i18n: &'a Translator, prices: PricesState, trade: TradeStatus) -> Self {
        Self {
            i18n,
            line: Line::Sources(prices, trade),
        }
    }

    /// The version and the author, while the settings are open.
    #[must_use]
    pub fn author(i18n: &'a Translator) -> Self {
        Self {
            i18n,
            line: Line::Author,
        }
    }

    pub fn show(self, ui: &mut Ui) {
        Panel::bottom("status-bar")
            .exact_size(size::STATUS_BAR_HEIGHT)
            .frame(Frame::NONE.fill(color::BAND_A))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    // The padding of the band on both sides.
                    ui.set_max_width(ui.available_width() - size::BAND_PADDING);
                    ui.add_space(size::BAND_PADDING);
                    match self.line {
                        Line::Sources(prices, trade) => self.sources_line(ui, prices, trade),
                        Line::Author => self.author_line(ui),
                    }
                });
            });
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
    /// under the pointer.
    fn author_line(&self, ui: &mut Ui) {
        let site = crate::AUTHOR_SITE_LABEL;
        let author = self
            .i18n
            .author_line(crate::APP_NAME, crate::VERSION, crate::AUTHOR, site);
        // The translations end with the site (checked by the tests of `i18n`).
        let Some(before) = author.strip_suffix(site) else {
            ui.add(Label::new(faint(author)).truncate());
            return;
        };
        ui.add(Label::new(faint(before)));

        let galley = widgets::text_line(ui, site, text::small());
        let (rect, link) = ui.allocate_exact_size(galley.size(), Sense::click());
        link.widget_info(|| WidgetInfo::labeled(Role::Link, true, site));
        let lit = link.hovered() || link.has_focus();
        let tint = if lit { color::TEXT } else { color::WEAK };
        let painter = ui.painter();
        painter.galley(rect.min, galley, tint);
        if lit {
            painter.hline(rect.x_range(), rect.bottom(), Stroke::new(1.0, tint));
        }
        if link.on_hover_cursor(CursorIcon::PointingHand).clicked() {
            ui.ctx().open_url(OpenUrl::new_tab(crate::AUTHOR_SITE));
        }
    }
}

fn faint(line: impl Into<String>) -> RichText {
    RichText::new(line).font(text::small()).color(color::FAINT)
}
