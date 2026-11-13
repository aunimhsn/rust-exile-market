//! The offers block, on band B under the price block: its title row, with a square action
//! block in the top right corner (search, the spinner of the search, the countdown to the
//! next search, then search again), and once they come, the offers of the trade site, the
//! cheapest first.
//!
//! The rows are decided apart from the drawing ([`item_rows`], [`exchange_rows`]), and
//! tested.

use std::f32::consts::PI;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use egui::text::{LayoutJob, TextFormat};
use egui::{
    Color32, CursorIcon, Galley, Id, Key, Modifiers, Pos2, Rect, Response, Role, Sense, Stroke, Ui,
    Vec2, WidgetInfo, pos2, vec2,
};

use super::item_block::rarity_color;
use super::offer_tooltip::{OfferTooltip, Pointer, TooltipState, reach};
use super::theme::{color, size, text};
use super::{icons, images, widgets};
use crate::clipboard;
use crate::currencies::{self, ExchangeCurrency};
use crate::estimate::Reference;
use crate::format::{self, Currency};
use crate::i18n::{Linked, Message, Translator};
use crate::item::{Catalyst, Item};
use crate::language::Language;
use crate::offers::{LONG_WAIT, Offers, OffersState};
use crate::roll_difference::CopiedLines;
use crate::trade::offers::{ExchangeOffer, ItemOffer, Price, SellerStatus, currency};
use crate::trade::query::SearchKind;
use crate::trade::search::Failure;

/// The space above and under the content of the band, and between the label and a message.
const PADDING_Y: f32 = 12.0;
/// The space between the label and the line under it.
const TITLE_GAP: f32 = 4.0;
/// The space between two messages.
const MESSAGE_GAP: f32 = 2.0;
/// The title row, and the square action block in its corner: search, wait, search again.
const HEAD_HEIGHT: f32 = 48.0;
const BLOCK_SIZE: f32 = HEAD_HEIGHT;
/// The space between the count and the action block, and before the count.
const COUNT_GAP: f32 = 12.0;
/// The space under the title row.
const HEAD_GAP: f32 = 6.0;
/// The icons of the search and refresh blocks, and the double chevron.
const SEARCH_ICON: f32 = 22.0;
const REFRESH_ICON: f32 = 18.0;
const MORE_ICON: f32 = 16.0;
/// The growth of the icons under the pointer, and the quarter turn of the refresh icon.
const SEARCH_GROWTH: f32 = 0.15;
const REFRESH_GROWTH: f32 = 0.12;
/// The length of the hover motions, in seconds.
const HOVER_TIME: f32 = 0.22;
/// The rows: the space above and under their text, and between their columns.
const ROW_PADDING_Y: f32 = 5.0;
const COLUMN_GAP: f32 = 12.0;
/// The column of the listing time of the currency exchange, right of the stock.
const LISTED_WIDTH: f32 = 90.0;
/// The space under the titles of the columns.
const COLUMNS_PADDING_BOTTOM: f32 = 4.0;
const ROW_ORB: f32 = 18.0;
const ORB_GAP: f32 = 6.0;
/// The seller status, before the listing time.
const DOT_RADIUS: f32 = 3.0;
const DOT_GAP: f32 = 6.0;
/// The "?" after a suspicious price.
const SUSPICIOUS_GAP: f32 = 2.0;
/// The icon of an offer of another kind of quality, faint.
const OTHER_QUALITY_ORB: u8 = 115;
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

/// The price of a row, in its currency.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct RowPrice {
    pub(super) amount: String,
    /// Its currency, drawn by its icon, its name after it in a row; `None` for a currency
    /// whose offers aren't shown (a waystone, or one the application doesn't know): the
    /// offers of items in it are left out, those of the currency exchange are paid in
    /// Exalted or Divine Orbs.
    pub(super) currency: Option<&'static ExchangeCurrency>,
}

impl RowPrice {
    /// The amount and the name of its currency in `language`, as a screen reader reads the
    /// row: `1 Vaal Orb`.
    pub(super) fn label(&self, language: Language) -> String {
        match self.currency {
            Some(currency) => format!("{} {}", self.amount, currency.name(language)),
            None => self.amount.clone(),
        }
    }
}

/// A row of the list.
#[derive(Debug, Clone, PartialEq)]
struct Row {
    /// `None` for an item listed without a price.
    price: Option<RowPrice>,
    /// Far below the reference price: usually bait or a mistake.
    suspicious: bool,
    /// The kind of the quality of a catalyst of an offer, when it isn't the kind searched:
    /// it isn't counted in the estimate.
    other_quality: Option<Catalyst>,
    /// The stock of an offer of the currency exchange.
    stock: Option<u64>,
    away: bool,
    listed: Option<Duration>,
}

/// `amount` of the currency `id`, as a row writes it: the other currencies than the three
/// of the prices with the decimals of the Divine Orbs.
fn row_price(amount: f64, id: &str) -> RowPrice {
    RowPrice {
        amount: format::amount(amount, currency(id).unwrap_or(Currency::Divine)),
        currency: currencies::shown(id),
    }
}

/// The price of an offer of items, as its row and its tooltip write it.
pub(super) fn offer_price(price: &Price) -> RowPrice {
    row_price(price.amount, &price.currency)
}

/// Whether one item for `amount` of the currency `id` is far below `reference`.
fn is_suspicious(amount: f64, id: &str, reference: Option<Reference>) -> bool {
    reference.is_some_and(|reference| reference.suspicious(amount, id))
}

fn listed_since(listed: Option<SystemTime>, now: SystemTime) -> Option<Duration> {
    listed.map(|listed| now.duration_since(listed).unwrap_or_default())
}

/// The rows of the offers of a search of items, at `now`; with the `searched_catalyst`, the
/// kind of the quality of a catalyst searched, the offers of another kind are marked, and
/// never suspicious: their market isn't the item's.
fn item_rows(
    offers: &[ItemOffer],
    reference: Option<Reference>,
    searched_catalyst: Option<Catalyst>,
    now: SystemTime,
) -> Vec<Row> {
    offers
        .iter()
        .map(|offer| {
            let other_quality =
                searched_catalyst.and_then(|catalyst| offer.other_quality(catalyst));
            Row {
                price: offer
                    .price
                    .as_ref()
                    .map(|price| row_price(price.amount, &price.currency)),
                suspicious: other_quality.is_none()
                    && offer
                        .price
                        .as_ref()
                        .is_some_and(|Price { amount, currency }| {
                            is_suspicious(*amount, currency, reference)
                        }),
                other_quality,
                stock: None,
                away: offer.seller == SellerStatus::Away,
                listed: listed_since(offer.listed, now),
            }
        })
        .collect()
}

/// The rows of the offers of the currency exchange, at `now`: the price of one item.
fn exchange_rows(
    offers: &[ExchangeOffer],
    reference: Option<Reference>,
    now: SystemTime,
) -> Vec<Row> {
    offers
        .iter()
        .map(|offer| {
            let unit = offer.unit_price();
            Row {
                price: Some(row_price(unit, &offer.pay.currency)),
                suspicious: is_suspicious(unit, &offer.pay.currency, reference),
                other_quality: None,
                stock: Some(offer.stock),
                away: offer.seller == SellerStatus::Away,
                listed: listed_since(offer.listed, now),
            }
        })
        .collect()
}

/// The action block of the band.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Block {
    /// The first search of the item.
    Search,
    /// Search again.
    Again,
    /// The time left before the next search; `None` once it is over while an answer is
    /// awaited.
    Waiting(Option<Duration>),
    /// The search is sent: the spinner, until its answer.
    Searching,
}

/// The block of a band whose action is `ready` once the `wait` is over and the offers aren't
/// `busy`.
fn block(ready: Block, wait: Duration, busy: bool) -> Block {
    if !wait.is_zero() {
        Block::Waiting(Some(wait))
    } else if busy {
        Block::Waiting(None)
    } else {
        ready
    }
}

pub struct OffersBlock<'a> {
    offers: &'a Offers,
    kind: SearchKind,
    reference: Option<Reference<'a>>,
    /// The copied item, and its lines, which the tooltips compare.
    item: &'a Item,
    lines: &'a CopiedLines,
    i18n: &'a Translator,
    /// The hint of a failed search opens the bug report: in the main window only, which has
    /// it.
    bug_report: bool,
}

impl<'a> OffersBlock<'a> {
    /// The block of the offers of a search of `kind` for `item`; `reference` tells the
    /// suspicious offers.
    #[must_use]
    pub fn new(
        offers: &'a Offers,
        kind: SearchKind,
        reference: Option<Reference<'a>>,
        item: &'a Item,
        lines: &'a CopiedLines,
        i18n: &'a Translator,
    ) -> Self {
        Self {
            offers,
            kind,
            reference,
            item,
            lines,
            i18n,
            bug_report: false,
        }
    }

    /// The words of the hint of a failed search open the bug report.
    #[must_use]
    pub fn bug_report(mut self, bug_report: bool) -> Self {
        self.bug_report = bug_report;
        self
    }

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
            OffersState::Idle => self.title_row(
                ui,
                (i18n.text(Message::OffersSearchHint), color::FAINT),
                block(Block::Search, wait, busy),
                None,
            ),
            &OffersState::Failed(failure) => {
                let (line, hint) = failure_lines(i18n, failure);
                self.title_row(ui, line, block(Block::Search, wait, busy), hint.as_ref())
            }
            OffersState::Limited => {
                let line = if wait.is_zero() {
                    (i18n.text(Message::OffersSearchHint), color::FAINT)
                } else {
                    (i18n.offers_limit(wait), color::WARNING)
                };
                self.title_row(ui, line, block(Block::Search, wait, busy), None)
            }
            OffersState::Searching => self.title_row(
                ui,
                (i18n.text(Message::OffersSearching), color::FAINT),
                Block::Searching,
                None,
            ),
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
                    exchange: false,
                    more: match (rest.is_empty(), *loading_more) {
                        (_, true) => More::Loading,
                        (false, false) => More::Chevron,
                        (true, false) => More::None,
                    },
                    more_failed: *more_failed,
                };
                self.results(ui, &list, *searched, wait)
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
                    exchange: true,
                    more: if shown.len() < offers.len() {
                        More::Chevron
                    } else {
                        More::None
                    },
                    more_failed: None,
                };
                self.results(ui, &list, *searched, wait)
            }
        }
    }

    /// Until the offers show, the band is only the title row: the label, `line` under it, and
    /// the action `block` in the corner; after a search that failed, what to do under the row
    /// (`hint`).
    fn title_row(
        &self,
        ui: &mut Ui,
        (line, line_color): (String, Color32),
        block: Block,
        hint: Option<&Linked>,
    ) -> Option<OffersAction> {
        let mut action = None;
        band(ui, |ui, width| {
            let head = Head {
                line,
                line_color,
                count: None,
                block,
            };
            if self.head(ui, &head) {
                action = Some(OffersAction::Search);
            }
            if let Some(hint) = hint {
                ui.add_space(HEAD_GAP);
                let tooltip = self.i18n.text(Message::BugReportTooltip);
                if linked_hint(ui, hint, width, self.bug_report, &tooltip) {
                    action = Some(OffersAction::ReportBug);
                }
                ui.add_space(PADDING_Y);
            }
        });
        action
    }

    /// The title row, then the list, or why it is empty.
    fn results(
        &self,
        ui: &mut Ui,
        list: &List,
        searched: SystemTime,
        wait: Duration,
    ) -> Option<OffersAction> {
        let i18n = self.i18n;
        let age = SystemTime::now()
            .duration_since(searched)
            .unwrap_or_default();
        // The age changes every minute.
        ui.ctx()
            .request_repaint_after(Duration::from_secs(60 - age.as_secs() % 60));
        let mut action = None;
        band(ui, |ui, width| {
            let count = (list.total > 0).then(|| {
                i18n.offers_count(
                    u64::try_from(list.rows.len()).unwrap_or(u64::MAX),
                    list.total,
                )
            });
            let head = Head {
                line: i18n.offers_searched(&i18n.age(age)),
                line_color: color::FAINT,
                count,
                block: block(Block::Again, wait, self.offers.busy()),
            };
            if self.head(ui, &head) {
                action = Some(OffersAction::Search);
            }
            ui.add_space(HEAD_GAP);
            let mut lines = Vec::new();
            if wait > LONG_WAIT {
                lines.push((i18n.offers_limit(wait), color::WARNING));
            }
            if no_offer(list.rows, list.more) {
                lines.push((i18n.text(Message::OffersNone), color::WEAK));
                lines.push((i18n.text(Message::OffersNoneHint), color::WEAK));
            }
            for (index, (line, tint)) in lines.iter().enumerate() {
                if index > 0 {
                    ui.add_space(MESSAGE_GAP);
                }
                let galley = widgets::wrapped_line(ui, line, text::item_line(), width);
                widgets::text_row(ui, galley, size::BAND_PADDING, *tint);
            }
            if !lines.is_empty() && !list.rows.is_empty() {
                ui.add_space(HEAD_GAP);
            }
            if !list.rows.is_empty() {
                self.list(ui, list);
            }
            if let Some(failure) = list.more_failed {
                ui.add_space(MORE_FAILED_GAP);
                let message = i18n.text(failure_message(failure));
                let galley = widgets::wrapped_line(ui, &message, text::note(), width);
                widgets::text_row(ui, galley, size::BAND_PADDING, color::WEAK);
            }
            match list.more {
                More::Chevron => {
                    ui.add_space(MORE_GAP);
                    if more(ui, i18n).clicked() {
                        action = Some(OffersAction::More);
                    }
                }
                More::Loading => {
                    ui.add_space(MORE_GAP);
                    let (rect, _) = ui.allocate_exact_size(
                        vec2(ui.available_width(), MORE_HEIGHT),
                        Sense::hover(),
                    );
                    let time = ui.input(|input| input.time);
                    icons::spinner(ui.painter(), rect.center() - vec2(0.0, MORE_RAISE), time);
                    ui.ctx().request_repaint();
                }
                More::None => ui.add_space(PADDING_Y),
            }
        });
        action
    }

    /// The label and the line under it, the count on the right, then the action block in
    /// the top right corner; whether the block was clicked.
    fn head(&self, ui: &mut Ui, head: &Head) -> bool {
        let (rect, _) =
            ui.allocate_exact_size(vec2(ui.available_width(), HEAD_HEIGHT), Sense::hover());
        let left = rect.left() + size::BAND_PADDING;
        let block_rect = Rect::from_min_max(
            pos2(rect.right() - BLOCK_SIZE, rect.top()),
            rect.right_bottom(),
        );
        let mut line_right = block_rect.left() - COUNT_GAP;
        if let Some(count) = &head.count {
            let galley = widgets::text_line(ui, count.as_str(), text::tag());
            let size = galley.size();
            line_right -= size.x + COUNT_GAP;
            ui.painter().galley(
                pos2(
                    block_rect.left() - COUNT_GAP - size.x,
                    rect.center().y - size.y / 2.0,
                ),
                galley,
                color::WEAK,
            );
        }
        let label = label(ui, self.i18n);
        // Cut with "…" in a window too narrow for it.
        let line = widgets::truncated_line(
            ui,
            head.line.as_str(),
            text::small(),
            (line_right - left).max(0.0),
        );
        let painter = ui.painter();
        let top = rect.center().y - (label.size().y + TITLE_GAP + line.size().y) / 2.0;
        painter.galley(pos2(left, top), label.clone(), color::WEAK);
        painter.galley(
            pos2(left, top + label.size().y + TITLE_GAP),
            line,
            head.line_color,
        );
        self.action_block(ui, block_rect, head.block)
    }

    /// Draws `block` in `rect`; whether it was clicked, and could be.
    fn action_block(&self, ui: &mut Ui, rect: Rect, block: Block) -> bool {
        let i18n = self.i18n;
        let clickable = matches!(block, Block::Search | Block::Again);
        let sense = if clickable {
            Sense::click()
        } else {
            Sense::hover()
        };
        let response = ui.interact(rect, ui.scope_id().with("offers-block"), sense);
        let tooltip = match block {
            Block::Search => i18n.text(Message::OffersSearchTooltip),
            Block::Again => i18n.text(Message::OffersAgainTooltip),
            Block::Waiting(Some(wait)) => i18n.offers_next_search_tooltip(wait),
            Block::Waiting(None) | Block::Searching => i18n.text(Message::OffersBusyTooltip),
        };
        let hover = ui.ctx().animate_bool_with_time_and_easing(
            response.id.with("hover"),
            clickable && response.hovered(),
            HOVER_TIME,
            overshoot,
        );
        let painter = ui.painter();
        match block {
            Block::Search => {
                painter.rect_filled(rect, 0.0, color::ACCENT);
                let size = SEARCH_ICON * (1.0 + SEARCH_GROWTH * hover);
                let square = Rect::from_center_size(rect.center(), Vec2::splat(size));
                icons::search(painter, square, color::ON_ACCENT);
            }
            Block::Again => {
                painter.rect_filled(rect, 0.0, color::ACCENT);
                let size = REFRESH_ICON * (1.0 + REFRESH_GROWTH * hover);
                let square = Rect::from_center_size(rect.center(), Vec2::splat(size));
                icons::refresh(painter, square, hover, color::ON_ACCENT);
            }
            Block::Searching => {
                painter.rect_filled(rect, 0.0, color::CONTROL);
                icons::spinner(painter, rect.center(), ui.input(|input| input.time));
                ui.ctx().request_repaint();
            }
            Block::Waiting(wait) => {
                painter.rect_filled(rect, 0.0, color::CONTROL);
                if let Some(wait) = wait {
                    let galley = widgets::text_line(ui, i18n.wait(wait), text::countdown());
                    let size = galley.size();
                    ui.painter()
                        .galley(rect.center() - size / 2.0, galley, color::WEAK);
                }
            }
        }
        response.widget_info(|| WidgetInfo::labeled(Role::Button, clickable, &tooltip));
        let response = response.on_hover_text(tooltip);
        clickable && response.on_hover_cursor(CursorIcon::PointingHand).clicked()
    }

    /// The titles of the columns, then the rows, the whole width of the band.
    fn list(&self, ui: &mut Ui, list: &List) {
        let i18n = self.i18n;
        let titles: Vec<String> = if list.exchange {
            vec![
                i18n.text(Message::OffersColumnPerUnit),
                i18n.text(Message::OffersColumnStock),
                i18n.text(Message::OffersColumnListed),
            ]
        } else {
            vec![
                i18n.text(Message::OffersColumnPrice),
                i18n.text(Message::OffersColumnListed),
            ]
        };
        let galleys: Vec<_> = titles
            .into_iter()
            .map(|title| {
                widgets::spaced_line(
                    ui,
                    title.to_uppercase(),
                    text::column(),
                    text::COLUMN_LETTER_SPACING,
                )
            })
            .collect();
        let height = ROW_PADDING_Y
            + galleys.first().map_or(0.0, |galley| galley.size().y)
            + COLUMNS_PADDING_BOTTOM;
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
        let columns = Columns::of(rect, list.exchange);
        let painter = ui.painter();
        let top = rect.top() + ROW_PADDING_Y;
        let mut cells = columns.right_edges().into_iter();
        for (index, galley) in galleys.into_iter().enumerate() {
            let left = if index == 0 {
                columns.left
            } else {
                cells.next().unwrap_or(columns.right) - galley.size().x
            };
            painter.galley(pos2(left, top), galley, color::FAINT);
        }

        // Only the offers of items have a tooltip, and light up under the pointer; without
        // the focus of the keyboard, which the rows wouldn't show.
        let sense = if list.items.is_some() {
            Sense::CLICK
        } else {
            Sense::hover()
        };
        let height = ROW_ORB + 2.0 * ROW_PADDING_Y;
        let responses: Vec<Response> = list
            .rows
            .iter()
            .map(|_| {
                ui.allocate_exact_size(vec2(ui.available_width(), height), sense)
                    .1
            })
            .collect();
        let open = list
            .items
            .and_then(|offers| self.tooltip(ui, offers, list.rows, &responses));
        for (index, (row, response)) in list.rows.iter().zip(&responses).enumerate() {
            let rect = response.rect;
            if list.items.is_some() {
                let language = self.item.language();
                let label = row
                    .price
                    .as_ref()
                    .map(|price| price.label(language))
                    .unwrap_or_default();
                response.widget_info(|| WidgetInfo::labeled(Role::Button, true, &label));
            }
            let highlighted = list.items.is_some() && (response.hovered() || open == Some(index));
            if highlighted {
                ui.painter().rect_filled(rect, 0.0, color::CONTROL_LINE);
            } else if index % 2 == 0 {
                ui.painter().rect_filled(rect, 0.0, color::ROW_ALT);
            }
            // The whole name of a currency cut, under the pointer; the offers of items have
            // the tooltip of their item instead.
            if self.row(ui, rect, row, list.exchange)
                && list.items.is_none()
                && let Some(price) = &row.price
            {
                response
                    .clone()
                    .on_hover_text(price.label(self.item.language()));
            }
        }
    }

    /// Opens, pins or closes the tooltip of the `offers` after what the pointer did over
    /// their rows, then draws it; the index of the offer whose tooltip is open.
    fn tooltip(
        &self,
        ui: &Ui,
        offers: &[ItemOffer],
        rows: &[Row],
        responses: &[Response],
    ) -> Option<usize> {
        let ctx = ui.ctx();
        // One per window: the main one and the one over the game each have their own.
        let state_id = Id::NULL.with(("offer-tooltip-state", ctx.viewport_id()));
        let mut state: TooltipState = ctx.data(|data| data.get_temp(state_id).unwrap_or_default());
        let ids: Vec<&str> = offers.iter().map(|offer| offer.id.as_str()).collect();
        let id_at = |index: Option<usize>| index.and_then(|index| ids.get(index).copied());
        // The tooltip as drawn at the last frame, and the gap to its row.
        let over_tooltip = state.open().is_some_and(|id| {
            let area = ctx.memory(|memory| memory.area_rect(OfferTooltip::area_id(id)));
            let row = ids
                .iter()
                .position(|other| *other == id)
                .and_then(|index| responses.get(index));
            match (area, row, ctx.pointer_latest_pos()) {
                (Some(area), Some(row), Some(pointer)) => {
                    area.contains(pointer) || reach(area, row.rect).contains(pointer)
                }
                _ => false,
            }
        });
        let (any_click, escape, time) = ui.input(|input| {
            (
                input.pointer.any_click(),
                input.key_pressed(Key::Escape),
                input.time,
            )
        });
        let pointer = Pointer {
            hovered: id_at(responses.iter().position(Response::hovered)),
            clicked: id_at(responses.iter().position(Response::clicked)),
            over_tooltip,
            any_click,
            escape,
            time,
        };
        // Esc closes the tooltip, and nothing under it (the window over the game on a single
        // screen, which Esc closes too).
        if escape && state.open().is_some() {
            ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape));
        }
        if let Some(redraw) = state.update(&pointer, &ids) {
            ctx.request_repaint_after(redraw);
        }
        let open = state
            .open()
            .and_then(|id| ids.iter().position(|other| *other == id));
        if let Some(index) = open
            && let (Some(offer), Some(row), Some(response)) =
                (offers.get(index), rows.get(index), responses.get(index))
        {
            let tooltip = OfferTooltip {
                offer,
                item: self.item,
                searched_catalyst: self.offers.searched_catalyst(self.item),
                lines: self.lines,
                name_color: rarity_color(offer.item.rarity.unwrap_or(self.item.rarity())),
                suspicious: row.suspicious,
                pill: state.pill(&offer.id, time),
                i18n: self.i18n,
            };
            if tooltip.show(ui, response.rect, ui.clip_rect().top()) {
                state.copy_result(&offer.id, time, clipboard::write_text(&offer.whisper));
                ctx.request_repaint();
            }
        }
        ctx.data_mut(|data| data.insert_temp(state_id, state));
        open
    }

    /// Draws `row` in `rect`: its stock and its listing time on the right, then its price
    /// before them; whether the name of its currency was cut.
    fn row(&self, ui: &Ui, rect: Rect, row: &Row, exchange: bool) -> bool {
        let columns = Columns::of(rect, exchange);
        let center_y = rect.center().y;
        let painter = ui.painter();
        let mut edges = columns.right_edges().into_iter();
        // The left of what is drawn on the right, before which the price stops.
        let mut used = columns.right;
        if let Some(stock) = row.stock {
            let galley = widgets::text_line(ui, stock.to_string(), text::body());
            let size = galley.size();
            let right = edges.next().unwrap_or(columns.right);
            used = used.min(right - size.x);
            painter.galley(
                pos2(right - size.x, center_y - size.y / 2.0),
                galley,
                color::TEXT,
            );
        }
        let right = edges.next().unwrap_or(columns.right);
        let listed = row
            .listed
            .map(|listed| self.i18n.age(listed))
            .unwrap_or_default();
        let galley = widgets::text_line(ui, listed, text::body());
        let size = galley.size();
        let left = right - size.x;
        painter.galley(pos2(left, center_y - size.y / 2.0), galley, color::TEXT);
        let dot = if row.away {
            color::WARNING
        } else {
            color::RISE
        };
        let dot_center = pos2(left - DOT_GAP - DOT_RADIUS, center_y);
        painter.circle_filled(dot_center, DOT_RADIUS, dot);
        used = used.min(dot_center.x - DOT_RADIUS);
        match &row.price {
            Some(price) => {
                let start = pos2(columns.left, center_y);
                self.row_price(ui, row, price, start, used - COLUMN_GAP)
            }
            None => false,
        }
    }

    /// Draws the price of `row` from `start`, on the left of the row at its middle, to
    /// `limit` at most: its amount, the icon of its currency, its name, then the kind of
    /// another quality or the "?" of a suspicious offer; whether the name was cut.
    fn row_price(&self, ui: &Ui, row: &Row, price: &RowPrice, start: Pos2, limit: f32) -> bool {
        let painter = ui.painter();
        let center_y = start.y;
        let faint = row.suspicious || row.other_quality.is_some();
        let galley = widgets::text_line(ui, price.amount.as_str(), text::list_amount());
        let size = galley.size();
        let tint = if faint { color::FAINT } else { color::TEXT };
        painter.galley(pos2(start.x, center_y - size.y / 2.0), galley, tint);
        if row.suspicious {
            painter.hline(
                start.x..=start.x + size.x,
                center_y,
                Stroke::new(1.0, color::FAINT),
            );
        }
        let mut right = start.x + size.x;
        // The kind of another quality, or the "?", after the name, which leaves them room. The
        // kind is written as its filter after a dot, "· Caster quality", so that it isn't read
        // as a name.
        let after = if let Some(catalyst) = row.other_quality {
            let mut job = LayoutJob::default();
            job.append(
                text::SEPARATOR,
                0.0,
                TextFormat::simple(text::note(), color::FAINT),
            );
            job.append(
                &self.i18n.offer_quality_kind(catalyst),
                0.0,
                TextFormat::simple(text::note(), color::WEAK),
            );
            Some((painter.layout_job(job), 0.0, color::WEAK))
        } else if row.suspicious {
            let mark = widgets::text_line(ui, "?", text::list_amount());
            Some((mark, ORB_GAP + SUSPICIOUS_GAP, color::WARNING))
        } else {
            None
        };
        let after_width = after
            .as_ref()
            .map_or(0.0, |(galley, gap, _)| gap + galley.size().x);
        let mut cut = false;
        if let Some(currency) = price.currency {
            let icon_rect = Rect::from_min_size(
                pos2(right + ORB_GAP, center_y - ROW_ORB / 2.0),
                Vec2::splat(ROW_ORB),
            );
            let icon = images::exchange_icon(currency);
            if row.other_quality.is_some() {
                icon.tint(Color32::from_white_alpha(OTHER_QUALITY_ORB))
            } else {
                icon
            }
            .paint_at(ui, icon_rect);
            let left = icon_rect.right() + ORB_GAP;
            let width = (limit - after_width - left).max(0.0);
            let name = currency.name(self.item.language());
            let galley = widgets::truncated_line(ui, name, text::note(), width);
            cut = galley.elided;
            let name_size = galley.size();
            let name_tint = if faint { color::FAINT } else { color::WEAK };
            painter.galley(pos2(left, center_y - name_size.y / 2.0), galley, name_tint);
            right = left + name_size.x;
        }
        if let Some((galley, gap, tint)) = after {
            let top = center_y - galley.size().y / 2.0;
            painter.galley(pos2(right + gap, top), galley, tint);
        }
        cut
    }
}

/// What the list shows.
struct List<'a> {
    rows: &'a [Row],
    /// The offers of items, whose rows open their tooltips.
    items: Option<&'a [ItemOffer]>,
    total: u64,
    /// The offers of the currency exchange, with their stock.
    exchange: bool,
    more: More,
    /// Why the next offers couldn't come.
    more_failed: Option<Failure>,
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

/// The title row of the offers.
struct Head {
    /// Under the label: the age of the offers, or what happens before they show.
    line: String,
    line_color: Color32,
    count: Option<String>,
    block: Block,
}

/// Where the columns of a row are.
struct Columns {
    left: f32,
    right: f32,
    exchange: bool,
}

impl Columns {
    fn of(row: Rect, exchange: bool) -> Self {
        Self {
            left: row.left() + size::BAND_PADDING,
            right: row.right() - size::BAND_PADDING,
            exchange,
        }
    }

    /// The right edges of the columns after the first one, whose text is aligned on the
    /// right.
    fn right_edges(&self) -> Vec<f32> {
        if self.exchange {
            vec![self.right - LISTED_WIDTH - COLUMN_GAP, self.right]
        } else {
            vec![self.right]
        }
    }
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

/// The line of the title row after a search that failed for `failure`, and what to do under
/// the row: nothing when trying again is enough.
fn failure_lines(i18n: &Translator, failure: Failure) -> ((String, Color32), Option<Linked>) {
    let line = i18n.text(failure_message(failure));
    match failure {
        Failure::Unreachable => ((line, color::WEAK), None),
        Failure::Refused => ((line, color::WARNING), Some(i18n.offers_refused_hint())),
        Failure::Unreadable => ((line, color::WARNING), Some(i18n.offers_unreadable_hint())),
    }
}

/// What a search, or the fetch of its next offers, that failed for `failure` says.
fn failure_message(failure: Failure) -> Message {
    match failure {
        Failure::Unreachable => Message::OffersUnreachable,
        Failure::Refused => Message::OffersRefused,
        Failure::Unreadable => Message::OffersUnreadable,
    }
}

/// What to do after a failed search, wrapped at `width`. When they are a `link` (the main
/// window), its words that open the bug report are in the color of the text, underlined under
/// the pointer and with the tooltip `tooltip`; elsewhere, they are plain. Whether they were
/// clicked.
fn linked_hint(ui: &mut Ui, hint: &Linked, width: f32, link: bool, tooltip: &str) -> bool {
    let galley = linked_galley(ui, hint, width, link);
    let (rect, _) =
        ui.allocate_exact_size(vec2(ui.available_width(), galley.size().y), Sense::hover());
    let origin = pos2(rect.left() + size::BAND_PADDING, rect.top());
    let words = hint.text.get(hint.link.clone()).unwrap_or_default();
    let mut clicked = false;
    let mut lit = false;
    let rects = if link {
        link_rects(&galley, hint)
    } else {
        Vec::new()
    };
    let id = ui.make_persistent_id("report-link");
    let rects: Vec<Rect> = rects
        .into_iter()
        .map(|part| part.translate(origin.to_vec2()))
        .collect();
    for (index, part) in rects.iter().enumerate() {
        // One stop of the keyboard for the whole link, on its first part.
        let sense = if index == 0 {
            Sense::click()
        } else {
            Sense::CLICK
        };
        let response = ui.interact(*part, id.with(index), sense);
        if index == 0 {
            response.widget_info(|| WidgetInfo::labeled(Role::Link, true, words));
        }
        let response = response.on_hover_text(tooltip);
        lit |= response.hovered() || response.has_focus();
        clicked |= response.clicked();
        if response.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
    }
    let painter = ui.painter();
    painter.galley(origin, galley, color::WEAK);
    if lit {
        for part in &rects {
            painter.hline(part.x_range(), part.bottom(), Stroke::new(1.0, color::TEXT));
        }
    }
    clicked
}

/// The hint laid out at `width`: in the weak color, its words of a `link` in the color of the
/// text.
fn linked_galley(ui: &Ui, hint: &Linked, width: f32, link: bool) -> Arc<Galley> {
    let font = text::item_line();
    let mut job = LayoutJob::default();
    let text = hint.text.as_str();
    let (start, end) = (hint.link.start, hint.link.end);
    for (part, tint) in [
        (text.get(..start), color::WEAK),
        (
            text.get(start..end),
            if link { color::TEXT } else { color::WEAK },
        ),
        (text.get(end..), color::WEAK),
    ] {
        if let Some(part) = part.filter(|part| !part.is_empty()) {
            job.append(part, 0.0, TextFormat::simple(font.clone(), tint));
        }
    }
    job.wrap.max_width = width;
    ui.painter().layout_job(job)
}

/// Where the words of the link of `hint` are in its `galley`, one rectangle per row they
/// are on (the words wrap like the rest of the sentence). The glyphs are counted as the
/// characters of the text, as `Galley::layout_from_cursor` of epaint counts them: a row keeps
/// the space it breaks at, and a newline has no glyph. To check again when egui changes.
fn link_rects(galley: &Galley, hint: &Linked) -> Vec<Rect> {
    let first = hint
        .text
        .get(..hint.link.start)
        .unwrap_or_default()
        .chars()
        .count();
    let last = first
        + hint
            .text
            .get(hint.link.clone())
            .unwrap_or_default()
            .chars()
            .count();
    let mut rects = Vec::new();
    let mut index = 0;
    for row in &galley.rows {
        let mut part: Option<Rect> = None;
        for glyph in &row.row.glyphs {
            if (first..last).contains(&index) && !glyph.chr.is_whitespace() {
                let glyph_rect = glyph.logical_rect().translate(row.pos.to_vec2());
                part = Some(part.map_or(glyph_rect, |part| part.union(glyph_rect)));
            }
            index += 1;
        }
        if row.ends_with_newline {
            index += 1;
        }
        rects.extend(part);
    }
    rects
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
