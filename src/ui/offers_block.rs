//! The offers block, on band B under the price block: its title row, with a square action
//! block in the top right corner (search, the spinner of the search, the countdown to the
//! next search, then search again), and once they come, the offers of the trade site, the
//! cheapest first.
//!
//! The `head` draws the title row; the `rows`, the list of the offers, whose rows are decided
//! apart from the drawing ([`item_rows`], [`exchange_rows`]), and tested.

mod head;
mod rows;

use std::f32::consts::PI;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use egui::{
    Color32, CursorIcon, Galley, Rect, Response, Role, Sense, Ui, Vec2, WidgetInfo, pos2, vec2,
};

use super::theme::{color, size, text};
use super::{icons, widgets};
use crate::i18n::{Message, Translator};
use crate::item::Item;
use crate::offers_state::{LONG_WAIT, Offers, OffersState};
use crate::price::estimate::Reference;
use crate::roll_difference::CopiedLines;
use crate::trade::offers::ItemOffer;
use crate::trade::query::SearchKind;
use crate::trade::search::Failure;
use head::{Block, Head, block, failure_lines, failure_message};
use rows::{Row, exchange_rows, item_rows};
pub(super) use rows::{RowPrice, offer_price};

/// The space above and under the content of the band, and between the label and a message.
const PADDING_Y: f32 = 12.0;

/// The space between the label and the line under it.
const TITLE_GAP: f32 = 4.0;

/// The space between two messages.
const MESSAGE_GAP: f32 = 2.0;

/// The space under the title row.
const HEAD_GAP: f32 = 6.0;

/// The length of the hover motions, in seconds.
const HOVER_TIME: f32 = 0.22;

/// The double chevron under the list.
const MORE_ICON: f32 = 16.0;

/// The double chevron under the list: its height, the space above it, how high it is
/// raised to look centered, and how far it slides down under the pointer.
const MORE_HEIGHT: f32 = 32.0;
const MORE_GAP: f32 = 6.0;
const MORE_RAISE: f32 = 3.0;
const MORE_SLIDE: f32 = 2.0;

/// The space above the message of the next offers that couldn't come.
const MORE_FAILED_GAP: f32 = 8.0;

/// What the player asks from the block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffersAction {
    Search,
    More,
    /// The words of the hint of a failed search that open the bug report.
    ReportBug,
}

/// The block of the offers of a search of `kind` for `item`.
pub struct OffersBlock<'a> {
    pub offers: &'a Offers,
    pub kind: SearchKind,
    /// The price that tells the suspicious offers.
    pub reference: Option<Reference<'a>>,
    /// The copied item, and its lines, which the tooltips compare.
    pub item: &'a Item,
    pub lines: &'a CopiedLines,
    /// The window has the bug report, which the hint of a failed search opens: the main
    /// window only.
    pub has_bug_report: bool,
    pub i18n: &'a Translator,
}

impl OffersBlock<'_> {
    /// The block of an unidentified unique, which can't be searched.
    pub fn identify(ui: &mut Ui, i18n: &Translator) {
        band(ui, |ui, width| {
            ui.add_space(PADDING_Y);
            let label = label(ui, i18n);
            let message = widgets::wrapped_line(
                ui,
                &i18n.text(Message::OffersIdentify),
                text::message(),
                width,
            );
            let height = label.size().y + TITLE_GAP * 2.0 + message.size().y;
            let (rect, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
            let painter = ui.painter();
            let left = rect.left() + size::BAND_PADDING;
            painter.galley(pos2(left, rect.top()), label.clone(), color::WEAK);
            let top = rect.top() + label.size().y + TITLE_GAP * 2.0;
            painter.galley(pos2(left, top), message, color::WEAK);
            ui.add_space(PADDING_Y);
        });
    }

    /// Draws the block, the whole width of `ui`.
    pub fn show(self, ui: &mut Ui) -> Option<OffersAction> {
        let now = SystemTime::now();
        let wait = self.offers.wait(self.kind, Instant::now());
        let busy = self.offers.busy();
        if !wait.is_zero() {
            // The countdown changes every second.
            let to_next_second = Duration::from_nanos(u64::from(wait.subsec_nanos()));
            ui.ctx().request_repaint_after(if to_next_second.is_zero() {
                Duration::from_secs(1)
            } else {
                to_next_second
            });
        }
        let i18n = self.i18n;
        match self.offers.state() {
            OffersState::Idle => {
                let line = (i18n.text(Message::OffersSearchHint), color::FAINT);
                self.title_row(ui, &Head::of(line, block(Block::Search, wait, busy)), None)
            }
            &OffersState::Failed(failure) => {
                let (line, hint) = failure_lines(i18n, failure);
                let head = Head::of(line, block(Block::Search, wait, busy));
                self.title_row(ui, &head, hint.as_ref())
            }
            OffersState::Limited => {
                let line = if wait.is_zero() {
                    (i18n.text(Message::OffersSearchHint), color::FAINT)
                } else {
                    (i18n.offers_limit(wait), color::WARNING)
                };
                self.title_row(ui, &Head::of(line, block(Block::Search, wait, busy)), None)
            }
            OffersState::Searching => {
                let line = (i18n.text(Message::OffersSearching), color::FAINT);
                self.title_row(ui, &Head::of(line, Block::Searching), None)
            }
            OffersState::Items {
                offers,
                total,
                rest,
                loading_more,
                more_failed,
                searched,
                ..
            } => {
                let rows = item_rows(
                    offers,
                    self.reference,
                    self.offers.searched_catalyst(self.item),
                    now,
                );
                let list = List {
                    rows: &rows,
                    items: Some(offers),
                    total: *total,
                    more: match (rest.is_empty(), *loading_more) {
                        (_, true) => More::Loading,
                        (false, false) => More::Chevron,
                        (true, false) => More::None,
                    },
                    more_failed: *more_failed,
                    searched: *searched,
                };
                self.results(ui, &list, wait)
            }
            OffersState::Exchange {
                offers,
                total,
                shown,
                searched,
            } => {
                let shown = offers.get(..*shown).unwrap_or(offers);
                let rows = exchange_rows(shown, self.reference, now);
                let list = List {
                    rows: &rows,
                    items: None,
                    total: *total,
                    more: if shown.len() < offers.len() {
                        More::Chevron
                    } else {
                        More::None
                    },
                    more_failed: None,
                    searched: *searched,
                };
                self.results(ui, &list, wait)
            }
        }
    }

    /// The title row, then the list, or why it is empty.
    fn results(&self, ui: &mut Ui, list: &List, wait: Duration) -> Option<OffersAction> {
        let age = SystemTime::now()
            .duration_since(list.searched)
            .unwrap_or_default();
        // The age changes every minute.
        ui.ctx()
            .request_repaint_after(Duration::from_secs(60 - age.as_secs() % 60));
        let mut action = None;
        band(ui, |ui, width| {
            if self.head(ui, &self.results_head(list, wait, age)) {
                action = Some(OffersAction::Search);
            }
            ui.add_space(HEAD_GAP);
            let messages = results_messages(self.i18n, list, wait);
            for (index, (message, tint)) in messages.iter().enumerate() {
                if index > 0 {
                    ui.add_space(MESSAGE_GAP);
                }
                let galley = widgets::wrapped_line(ui, message, text::item_line(), width);
                widgets::text_row(ui, galley, size::BAND_PADDING, *tint);
            }
            if !list.rows.is_empty() {
                if !messages.is_empty() {
                    ui.add_space(HEAD_GAP);
                }
                self.list(ui, list);
            }
            if let Some(failure) = list.more_failed {
                ui.add_space(MORE_FAILED_GAP);
                let message = self.i18n.text(failure_message(failure));
                let galley = widgets::wrapped_line(ui, &message, text::note(), width);
                widgets::text_row(ui, galley, size::BAND_PADDING, color::WEAK);
            }
            if more_row(ui, list.more, self.i18n) {
                action = Some(OffersAction::More);
            }
        });
        action
    }
}

/// The messages over the list of the offers, each in its color: the warning of a long wait
/// before the next search, and that no offer is left.
fn results_messages(i18n: &Translator, list: &List, wait: Duration) -> Vec<(String, Color32)> {
    let mut messages = Vec::new();
    if wait > LONG_WAIT {
        messages.push((i18n.offers_limit(wait), color::WARNING));
    }
    if no_offer(list.rows, list.more) {
        messages.push((i18n.text(Message::OffersNone), color::WEAK));
        messages.push((i18n.text(Message::OffersNoneHint), color::WEAK));
    }
    messages
}

/// Under the list: the chevron of the next offers, the spinner while they come, or the
/// space under the band; whether the chevron was clicked.
fn more_row(ui: &mut Ui, next: More, i18n: &Translator) -> bool {
    match next {
        More::Chevron => {
            ui.add_space(MORE_GAP);
            more(ui, i18n).clicked()
        }
        More::Loading => {
            ui.add_space(MORE_GAP);
            let (rect, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), MORE_HEIGHT), Sense::hover());
            let time = ui.input(|input| input.time);
            icons::spinner(ui.painter(), rect.center() - vec2(0.0, MORE_RAISE), time);
            ui.ctx().request_repaint();
            false
        }
        More::None => {
            ui.add_space(PADDING_Y);
            false
        }
    }
}

/// What the list shows.
struct List<'a> {
    rows: &'a [Row],
    /// The offers of items, whose rows open their tooltips; `None` for the offers of the
    /// currency exchange.
    items: Option<&'a [ItemOffer]>,
    total: u64,
    more: More,
    /// Why the next offers couldn't come.
    more_failed: Option<Failure>,
    /// When the search was sent.
    searched: SystemTime,
}

impl List<'_> {
    /// The search whose offers the list shows: of items, or of the currency exchange, with
    /// their stock.
    fn kind(&self) -> SearchKind {
        if self.items.is_some() {
            SearchKind::Items
        } else {
            SearchKind::Exchange
        }
    }
}

/// Whether the block says that no offer was found: none shown, and none left to fetch. A page
/// whose offers were all left out for their currency (the waystones) shows nothing above the
/// double chevron, which fetches the next ones.
fn no_offer(rows: &[Row], more: More) -> bool {
    rows.is_empty() && more == More::None
}

/// What is under the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum More {
    None,
    /// The double chevron, which shows more offers.
    Chevron,
    /// The spinner, while the next offers come.
    Loading,
}

/// Band B the whole width of `ui`, its content drawn by `add` with the width inside the
/// padding of the band.
fn band(ui: &mut Ui, add: impl FnOnce(&mut Ui, f32)) {
    egui::Frame::NONE.fill(color::BAND_B).show(ui, |ui| {
        let width = ui.available_width();
        ui.set_width(width);
        ui.spacing_mut().item_spacing = Vec2::ZERO;
        add(ui, (width - 2.0 * size::BAND_PADDING).max(0.0));
    });
}

/// The label of the band, "OFFERS".
fn label(ui: &Ui, i18n: &Translator) -> Arc<Galley> {
    widgets::small_title_line(ui, &i18n.text(Message::OffersLabel))
}

/// The double chevron under the list, the whole width of the band clickable.
fn more(ui: &mut Ui, i18n: &Translator) -> Response {
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), MORE_HEIGHT), Sense::click());
    let tooltip = i18n.text(Message::OffersMoreTooltip);
    response.widget_info(|| WidgetInfo::labeled(Role::Button, true, &tooltip));
    let lit = response.hovered();
    let slide = ui.ctx().animate_bool_with_time_and_easing(
        response.id.with("slide"),
        lit,
        HOVER_TIME,
        overshoot,
    );
    let tint = if lit { color::ACCENT } else { color::WEAK };
    let center = rect.center() + vec2(0.0, -MORE_RAISE + MORE_SLIDE * slide);
    let square = Rect::from_center_size(center, Vec2::splat(MORE_ICON));
    icons::double_chevron(ui.painter(), square, tint);
    response
        .on_hover_text(tooltip)
        .on_hover_cursor(CursorIcon::PointingHand)
}

/// An ease out with a slight overshoot, for the hover motions.
fn overshoot(t: f32) -> f32 {
    let eased = 1.0 - (1.0 - t).powi(3);
    eased + 0.12 * (PI * t).sin() * t
}

#[cfg(test)]
mod tests;
