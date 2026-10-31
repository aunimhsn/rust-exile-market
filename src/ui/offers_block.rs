//! The offers block, on band B under the price block: its title row, with a square action
//! block in the top right corner (search, the spinner of the search, the countdown to the
//! next search, then search again), and once they come, the offers of the trade site, the
//! cheapest first.
//!
//! The rows are decided apart from the drawing ([`item_rows`], [`exchange_rows`]), and
//! tested.

use std::f32::consts::{PI, TAU};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use egui::{
    Color32, CursorIcon, Galley, Id, Key, Painter, Pos2, Rect, Response, Role, Sense, Stroke, Ui,
    Vec2, WidgetInfo, pos2, vec2,
};

use super::item_block::rarity_color;
use super::offer_tooltip::{OfferTooltip, Pointer, TooltipState, reach};
use super::price_block::orb;
use super::theme::{color, size, text};
use super::widgets;
use crate::estimate::Reference;
use crate::format::{self, Currency};
use crate::i18n::{Message, Translator};
use crate::item::{Catalyst, Item};
use crate::offers::{LONG_WAIT, Offers, OffersState};
use crate::roll_difference::CopiedLines;
use crate::trade::offers::{ExchangeOffer, ItemOffer, Price, SellerStatus, currency};
use crate::trade::query::SearchKind;

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
/// The width of the strokes of the icons, on their grid of 16.
const ICON_STROKE: f32 = 1.8;
/// The growth of the icons under the pointer, and the quarter turn of the refresh icon.
const SEARCH_GROWTH: f32 = 0.15;
const REFRESH_GROWTH: f32 = 0.12;
/// The length of the hover motions, in seconds.
const HOVER_TIME: f32 = 0.22;
/// The spinner of the search.
const SPINNER_RADIUS: f32 = 6.0;
const SPINNER_STROKE: f32 = 2.0;
/// Turns per second.
const SPINNER_SPEED: f32 = 1.2;
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
/// The orb of an offer of another kind of quality, faint, and the space before its kind.
const OTHER_QUALITY_ORB: u8 = 115;
const OTHER_QUALITY_GAP: f32 = 4.0;
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
}

/// The price of a row, in its currency.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct RowPrice {
    amount: String,
    /// One of the three currencies of the prices, shown by its orb, or another one by its
    /// name on the trade site.
    pub(super) currency: Result<Currency, String>,
}

impl RowPrice {
    /// The amount, followed by the name of a currency without orb.
    pub(super) fn text(&self) -> String {
        match &self.currency {
            Ok(_) => self.amount.clone(),
            Err(name) => format!("{} {name}", self.amount),
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

/// `amount` of the currency `id`, as a row writes it.
fn row_price(amount: f64, id: &str) -> RowPrice {
    match currency(id) {
        Some(currency) => RowPrice {
            amount: format::amount(amount, currency),
            currency: Ok(currency),
        },
        None => RowPrice {
            amount: format::amount(amount, Currency::Divine),
            currency: Err(id.to_owned()),
        },
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

/// The rows of the offers of a search of items, at `now`; with the `quality` of a catalyst
/// searched, the offers of another kind are marked, and never suspicious: their market isn't
/// the item's.
fn item_rows(
    offers: &[ItemOffer],
    reference: Option<Reference>,
    quality: Option<Catalyst>,
    now: SystemTime,
) -> Vec<Row> {
    offers
        .iter()
        .map(|offer| {
            let other_quality = quality.and_then(|catalyst| offer.other_quality(catalyst));
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
    reference: Option<Reference>,
    /// The copied item, and its lines, which the tooltips compare.
    item: &'a Item,
    lines: &'a CopiedLines,
    i18n: &'a Translator,
}

impl<'a> OffersBlock<'a> {
    /// The block of the offers of a search of `kind` for `item`; `reference` tells the
    /// suspicious offers.
    #[must_use]
    pub fn new(
        offers: &'a Offers,
        kind: SearchKind,
        reference: Option<Reference>,
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
        }
    }

    /// The block of an unidentified unique, which can't be searched.
    pub fn identify(ui: &mut Ui, i18n: &Translator) {
        band(ui, |ui, width| {
            ui.add_space(PADDING_Y);
            let label = label(ui, i18n);
            let message = wrapped(ui, &i18n.text(Message::OffersIdentify), width);
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
            ),
            OffersState::Unreachable => self.title_row(
                ui,
                (i18n.text(Message::OffersUnreachable), color::WEAK),
                block(Block::Search, wait, busy),
            ),
            OffersState::Limited => {
                let line = if wait.is_zero() {
                    (i18n.text(Message::OffersSearchHint), color::FAINT)
                } else {
                    (i18n.offers_limit(wait), color::WARNING)
                };
                self.title_row(ui, line, block(Block::Search, wait, busy))
            }
            OffersState::Searching => self.title_row(
                ui,
                (i18n.text(Message::OffersSearching), color::FAINT),
                Block::Searching,
            ),
            OffersState::Items {
                offers,
                total,
                more,
                loading_more,
                more_failed,
                searched,
                ..
            } => {
                let rows = item_rows(offers, self.reference, self.offers.catalyst(self.item), now);
                let list = List {
                    rows: &rows,
                    items: Some(offers),
                    total: *total,
                    exchange: false,
                    more: match (*more, *loading_more) {
                        (_, true) => More::Loading,
                        (true, false) => More::Chevron,
                        (false, false) => More::None,
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
                    more_failed: false,
                };
                self.results(ui, &list, *searched, wait)
            }
        }
    }

    /// Until the offers show, the band is only the title row: the label, `line` under it, and
    /// the action `block` in the corner.
    fn title_row(
        &self,
        ui: &mut Ui,
        (line, line_color): (String, Color32),
        block: Block,
    ) -> Option<OffersAction> {
        let mut action = None;
        band(ui, |ui, _| {
            let head = Head {
                line,
                line_color,
                count: None,
                block,
            };
            if self.head(ui, &head) {
                action = Some(OffersAction::Search);
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
            if list.rows.is_empty() {
                lines.push((i18n.text(Message::OffersNone), color::WEAK));
                lines.push((i18n.text(Message::OffersNoneHint), color::WEAK));
            }
            for (index, (line, tint)) in lines.iter().enumerate() {
                if index > 0 {
                    ui.add_space(MESSAGE_GAP);
                }
                let galley = wrapped_with(ui, line, text::item_line(), width);
                let (rect, _) = ui.allocate_exact_size(
                    vec2(ui.available_width(), galley.size().y),
                    Sense::hover(),
                );
                ui.painter().galley(
                    pos2(rect.left() + size::BAND_PADDING, rect.top()),
                    galley,
                    *tint,
                );
            }
            if !lines.is_empty() && !list.rows.is_empty() {
                ui.add_space(HEAD_GAP);
            }
            if !list.rows.is_empty() {
                self.list(ui, list);
            }
            if list.more_failed {
                ui.add_space(MORE_FAILED_GAP);
                let galley = wrapped_with(
                    ui,
                    &i18n.text(Message::OffersUnreachable),
                    text::note(),
                    width,
                );
                let (rect, _) = ui.allocate_exact_size(
                    vec2(ui.available_width(), galley.size().y),
                    Sense::hover(),
                );
                ui.painter().galley(
                    pos2(rect.left() + size::BAND_PADDING, rect.top()),
                    galley,
                    color::WEAK,
                );
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
                    spinner(ui.painter(), rect.center() - vec2(0.0, MORE_RAISE), time);
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
                search_icon(
                    painter,
                    rect.center(),
                    SEARCH_ICON * (1.0 + SEARCH_GROWTH * hover),
                    color::ON_ACCENT,
                );
            }
            Block::Again => {
                painter.rect_filled(rect, 0.0, color::ACCENT);
                refresh_icon(
                    painter,
                    rect.center(),
                    REFRESH_ICON * (1.0 + REFRESH_GROWTH * hover),
                    hover,
                    color::ON_ACCENT,
                );
            }
            Block::Searching => {
                painter.rect_filled(rect, 0.0, color::CONTROL);
                spinner(painter, rect.center(), ui.input(|input| input.time));
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
                let label = row.price.as_ref().map(RowPrice::text).unwrap_or_default();
                response.widget_info(|| WidgetInfo::labeled(Role::Button, true, &label));
            }
            let highlighted = list.items.is_some() && (response.hovered() || open == Some(index));
            if highlighted {
                ui.painter().rect_filled(rect, 0.0, color::CONTROL_LINE);
            } else if index % 2 == 0 {
                ui.painter().rect_filled(rect, 0.0, color::ROW_ALT);
            }
            self.row(ui, rect, row, list.exchange);
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
        let state_id = Id::NULL.with("offer-tooltip-state");
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
                quality: self.offers.catalyst(self.item),
                lines: self.lines,
                name_color: rarity_color(offer.item.rarity.unwrap_or(self.item.rarity())),
                suspicious: row.suspicious,
                copied: state.shows_copied(&offer.id, time),
                i18n: self.i18n,
            };
            if tooltip.show(ui, response.rect, ui.clip_rect().top()) {
                ctx.copy_text(offer.whisper.clone());
                state.copied(&offer.id, time);
                ctx.request_repaint();
            }
        }
        ctx.data_mut(|data| data.insert_temp(state_id, state));
        open
    }

    /// Draws `row` in `rect`.
    fn row(&self, ui: &Ui, rect: Rect, row: &Row, exchange: bool) {
        let columns = Columns::of(rect, exchange);
        let center_y = rect.center().y;
        let painter = ui.painter();
        if let Some(price) = &row.price {
            let galley = widgets::text_line(ui, price.text(), text::list_amount());
            let size = galley.size();
            let top = center_y - size.y / 2.0;
            let tint = if row.suspicious || row.other_quality.is_some() {
                color::FAINT
            } else {
                color::TEXT
            };
            painter.galley(pos2(columns.left, top), galley, tint);
            if row.suspicious {
                painter.hline(
                    columns.left..=columns.left + size.x,
                    center_y,
                    Stroke::new(1.0, color::FAINT),
                );
            }
            let mut right = columns.left + size.x;
            if let Ok(currency) = price.currency {
                let orb_rect = Rect::from_min_size(
                    pos2(right + ORB_GAP, center_y - ROW_ORB / 2.0),
                    Vec2::splat(ROW_ORB),
                );
                let orb = orb(currency, ROW_ORB);
                if row.other_quality.is_some() {
                    orb.tint(Color32::from_white_alpha(OTHER_QUALITY_ORB))
                } else {
                    orb
                }
                .paint_at(ui, orb_rect);
                right = orb_rect.right();
            }
            if let Some(catalyst) = row.other_quality {
                let kind =
                    widgets::text_line(ui, self.i18n.offer_quality_kind(catalyst), text::note());
                let top = center_y - kind.size().y / 2.0;
                painter.galley(pos2(right + OTHER_QUALITY_GAP, top), kind, color::WEAK);
            }
            if row.suspicious {
                let mark = widgets::text_line(ui, "?", text::list_amount());
                let top = center_y - mark.size().y / 2.0;
                painter.galley(
                    pos2(right + ORB_GAP + SUSPICIOUS_GAP, top),
                    mark,
                    color::WARNING,
                );
            }
        }
        let mut edges = columns.right_edges().into_iter();
        if let Some(stock) = row.stock {
            let galley = widgets::text_line(ui, stock.to_string(), text::body());
            let size = galley.size();
            let right = edges.next().unwrap_or(columns.right);
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
        painter.circle_filled(pos2(left - DOT_GAP - DOT_RADIUS, center_y), DOT_RADIUS, dot);
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
    /// The next offers couldn't come.
    more_failed: bool,
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
    widgets::spaced_line(
        ui,
        i18n.text(Message::OffersLabel).to_uppercase(),
        text::label(),
        text::LABEL_LETTER_SPACING,
    )
}

/// A message, wrapped at `width`.
fn wrapped(ui: &Ui, message: &str, width: f32) -> Arc<Galley> {
    wrapped_with(ui, message, text::message(), width)
}

fn wrapped_with(ui: &Ui, line: &str, font: egui::FontId, width: f32) -> Arc<Galley> {
    ui.painter()
        .layout(line.to_owned(), font, Color32::PLACEHOLDER, width)
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
    double_chevron(ui.painter(), center, MORE_ICON, tint);
    response
        .on_hover_text(tooltip)
        .on_hover_cursor(CursorIcon::PointingHand)
}

/// An ease out with a slight overshoot, for the hover motions.
fn overshoot(t: f32) -> f32 {
    let eased = 1.0 - (1.0 - t).powi(3);
    eased + 0.12 * (PI * t).sin() * t
}

/// A point of an icon drawn on a grid of 16, `size` across, centered on `center`.
fn grid(center: Pos2, size: f32) -> impl Fn(f32, f32) -> Pos2 {
    let scale = size / 16.0;
    move |x, y| center + vec2(x - 8.0, y - 8.0) * scale
}

fn icon_stroke(size: f32, tint: Color32) -> Stroke {
    Stroke::new(ICON_STROKE * size / 16.0, tint)
}

/// The magnifying glass of the search block.
fn search_icon(painter: &Painter, center: Pos2, size: f32, tint: Color32) {
    let point = grid(center, size);
    let stroke = icon_stroke(size, tint);
    painter.circle_stroke(point(7.0, 7.0), 4.6 * size / 16.0, stroke);
    painter.line_segment([point(10.4, 10.4), point(14.0, 14.0)], stroke);
}

/// The refresh arrow, turned by `turn` quarters.
fn refresh_icon(painter: &Painter, center: Pos2, size: f32, turn: f32, tint: Color32) {
    let point = grid(center, size);
    let (sin, cos) = (turn * PI / 2.0).sin_cos();
    let rotate = |p: Pos2| {
        center + {
            let v = p - center;
            vec2(v.x * cos - v.y * sin, v.x * sin + v.y * cos)
        }
    };
    let stroke = icon_stroke(size, tint);
    // Three quarters and a half of a circle, from its right, clockwise.
    let arc: Vec<Pos2> = (0..=28)
        .map(|step| {
            #[expect(clippy::cast_precision_loss, reason = "a small step")]
            let angle = (step as f32 / 28.0) * TAU * 7.0 / 8.0;
            rotate(point(8.0 + 5.5 * angle.cos(), 8.0 + 5.5 * angle.sin()))
        })
        .collect();
    painter.line(arc, stroke);
    painter.line(
        vec![
            rotate(point(13.6, 2.6)),
            rotate(point(13.6, 5.9)),
            rotate(point(10.3, 5.9)),
        ],
        stroke,
    );
}

/// The double chevron pointing down.
fn double_chevron(painter: &Painter, center: Pos2, size: f32, tint: Color32) {
    let point = grid(center, size);
    let stroke = icon_stroke(size, tint);
    for top in [3.5, 8.5] {
        painter.line(
            vec![point(4.0, top), point(8.0, top + 4.0), point(12.0, top)],
            stroke,
        );
    }
}

/// A ring with a turning quarter, at `time` seconds.
fn spinner(painter: &Painter, center: Pos2, time: f64) {
    painter.circle_stroke(
        center,
        SPINNER_RADIUS,
        Stroke::new(SPINNER_STROKE, color::CONTROL_LINE),
    );
    #[expect(clippy::cast_possible_truncation, reason = "a fraction of a turn")]
    let start = ((time * f64::from(SPINNER_SPEED)).fract() as f32) * TAU;
    let arc: Vec<Pos2> = (0..=8)
        .map(|step| {
            #[expect(clippy::cast_precision_loss, reason = "a small step")]
            let angle = start + (step as f32 / 8.0) * TAU / 4.0;
            center + vec2(angle.cos(), angle.sin()) * SPINNER_RADIUS
        })
        .collect();
    painter.line(arc, Stroke::new(SPINNER_STROKE, color::ACCENT));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prices::Rates;
    use crate::trade::offers::OfferItem;

    const RATES: Rates = Rates {
        chaos: 75.0,
        divine: 750.0,
    };
    /// 12.4 Divine Orbs.
    const REFERENCE: Reference = Reference {
        exalted: 9300.0,
        rates: RATES,
    };

    fn offer(amount: f64, currency: &str, seller: SellerStatus) -> ItemOffer {
        ItemOffer {
            id: "a".to_owned(),
            price: Some(Price {
                amount,
                currency: currency.to_owned(),
            }),
            seller,
            listed: Some(SystemTime::UNIX_EPOCH),
            whisper: String::new(),
            item: OfferItem {
                rarity: None,
                name: Some("Headhunter".to_owned()),
                base_type: "Heavy Belt".to_owned(),
                item_level: None,
                corrupted: true,
                quality: None,
                modifiers: Vec::new(),
            },
        }
    }

    #[test]
    fn an_offer_under_a_quarter_of_the_market_price_is_suspicious() {
        let offers = [
            offer(6.0, "exalted", SellerStatus::Online),
            offer(1.0, "chaos", SellerStatus::Online),
            offer(3.0, "divine", SellerStatus::Online),
            offer(3.2, "divine", SellerStatus::Away),
        ];
        let now = SystemTime::UNIX_EPOCH + Duration::from_hours(2);

        let rows = item_rows(&offers, Some(REFERENCE), None, now);

        let suspicious: Vec<bool> = rows.iter().map(|row| row.suspicious).collect();
        assert_eq!(suspicious, [true, true, true, false]);
        assert_eq!(
            rows[3],
            Row {
                price: Some(RowPrice {
                    amount: "3.2".to_owned(),
                    currency: Ok(Currency::Divine),
                }),
                suspicious: false,
                other_quality: None,
                stock: None,
                away: true,
                listed: Some(Duration::from_hours(2)),
            }
        );
        assert!(
            item_rows(&offers, None, None, now)
                .iter()
                .all(|row| !row.suspicious),
            "without a market price, nothing tells"
        );
    }

    #[test]
    fn an_offer_of_another_kind_of_quality_is_marked_while_the_quality_is_searched() {
        use crate::item::QualityKind;
        use crate::trade::offers::OfferQuality;

        let of_kind = |amount, catalyst| {
            let mut offer = offer(amount, "divine", SellerStatus::Online);
            offer.item.quality = Some(OfferQuality {
                value: 40,
                kind: Some(QualityKind {
                    catalyst: Some(catalyst),
                    words: String::new(),
                }),
            });
            offer
        };
        // An attack amulet far below the market of the mana amulets: another market, not
        // bait.
        let offers = [
            of_kind(1.0, Catalyst::Attack),
            of_kind(12.0, Catalyst::Mana),
        ];
        let now = SystemTime::UNIX_EPOCH;

        let searched = item_rows(&offers, Some(REFERENCE), Some(Catalyst::Mana), now);
        let unsearched = item_rows(&offers, Some(REFERENCE), None, now);

        assert_eq!(searched[0].other_quality, Some(Catalyst::Attack));
        assert!(!searched[0].suspicious);
        assert_eq!(searched[1].other_quality, None);
        assert_eq!(unsearched[0].other_quality, None, "every offer counts");
        assert!(unsearched[0].suspicious);
    }

    #[test]
    fn another_currency_is_written_by_its_name_and_never_suspicious() {
        let rows = item_rows(
            &[offer(1.0, "annul", SellerStatus::Online)],
            Some(REFERENCE),
            None,
            SystemTime::UNIX_EPOCH,
        );

        assert_eq!(
            rows[0].price,
            Some(RowPrice {
                amount: "1".to_owned(),
                currency: Err("annul".to_owned()),
            })
        );
        assert!(!rows[0].suspicious);
    }

    #[test]
    fn an_offer_of_the_exchange_shows_the_price_of_one_item_and_its_stock() {
        let offer = ExchangeOffer {
            seller: SellerStatus::Online,
            listed: None,
            pay: Price {
                amount: 350.0,
                currency: "exalted".to_owned(),
            },
            get: Price {
                amount: 2.0,
                currency: "annul".to_owned(),
            },
            stock: 40,
            whisper: String::new(),
        };
        let reference = Reference {
            exalted: 182.0,
            rates: RATES,
        };

        let rows = exchange_rows(&[offer], Some(reference), SystemTime::UNIX_EPOCH);

        assert_eq!(
            rows,
            [Row {
                price: Some(RowPrice {
                    amount: "175".to_owned(),
                    currency: Ok(Currency::Exalted),
                }),
                suspicious: false,
                other_quality: None,
                stock: Some(40),
                away: false,
                listed: None,
            }]
        );
    }

    #[test]
    fn the_block_searches_only_once_the_wait_is_over_and_no_answer_is_awaited() {
        assert_eq!(block(Block::Again, Duration::ZERO, false), Block::Again);
        let eight = Duration::from_secs(8);
        assert_eq!(
            block(Block::Again, eight, false),
            Block::Waiting(Some(eight))
        );
        assert_eq!(
            block(Block::Again, eight, true),
            Block::Waiting(Some(eight)),
            "the countdown stays while more offers come"
        );
        assert_eq!(
            block(Block::Search, Duration::ZERO, true),
            Block::Waiting(None)
        );
    }
}
