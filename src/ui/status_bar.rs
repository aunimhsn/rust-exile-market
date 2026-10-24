//! The status bar: the age of each data source, or, while the settings are open, the version
//! and the author, with a link to the author's site.

use egui::{
    Align, CursorIcon, Frame, Label, Layout, OpenUrl, Panel, RichText, Role, Sense, Stroke, Ui,
    WidgetInfo,
};

use super::theme::{color, size, text};
use super::widgets;
use crate::i18n::{Message, Translator};

/// What the status bar shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Line {
    Sources,
    Author,
}

pub struct StatusBar<'a> {
    i18n: &'a Translator,
    line: Line,
}

impl<'a> StatusBar<'a> {
    /// The age of each data source.
    #[must_use]
    pub fn sources(i18n: &'a Translator) -> Self {
        Self {
            i18n,
            line: Line::Sources,
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
                        Line::Sources => self.sources_line(ui),
                        Line::Author => self.author_line(ui),
                    }
                });
            });
    }

    /// The age of the prices and of the last trade site search; none yet in this version.
    fn sources_line(&self, ui: &mut Ui) {
        let line = self.i18n.status_bar(
            &self.i18n.text(Message::StatusPricesWaiting),
            &self.i18n.text(Message::StatusTradeIdle),
        );
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
