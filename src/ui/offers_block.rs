//! The offers block, on band B under the price block: the search block before the first
//! search of the item, then the offers of the trade site, the cheapest first, with the
//! waiting block in its top right corner, which counts down to the next search and then
//! searches again.
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
use super::offer_tooltip::{OfferTooltip, Pointer, TooltipState};
use super::price_block::orb;
use super::theme::{color, size, text};
use super::widgets;
use crate::format::{self, Currency};
use crate::i18n::{Message, Translator};
use crate::item::Item;
use crate::offers::{LONG_WAIT, Offers, OffersState};
use crate::prices::Rates;
use crate::trade::offers::{ExchangeOffer, ItemOffer, Price, SellerStatus};
use crate::trade::query::SearchKind;

/// The space above and under the content of the band, and between the label and a message.
const PADDING_Y: f32 = 12.0;
/// The space between the label and the line under it.
const TITLE_GAP: f32 = 4.0;
/// The space between the label and the message of the search.
const SEARCHING_GAP: f32 = 18.0;
/// The space between two messages.
const MESSAGE_GAP: f32 = 2.0;
/// The action blocks: the search block, the height of its band, and the waiting block, the
/// height of the title row.
const BLOCK_WIDTH: f32 = 56.0;
const HEAD_HEIGHT: f32 = 48.0;
/// The space between the count and the waiting block.
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
const SPINNER_GAP: f32 = 10.0;
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
/// The double chevron under the list: its height, the space above it, how high it is
/// raised to look centered, and how far it slides down under the pointer.
const MORE_HEIGHT: f32 = 32.0;
const MORE_GAP: f32 = 6.0;
const MORE_RAISE: f32 = 3.0;
const MORE_SLIDE: f32 = 2.0;
/// The space above the message of the next offers that couldn't come.
const MORE_FAILED_GAP: f32 = 8.0;
/// An offer under this share of the reference price is suspicious.
const SUSPICIOUS_SHARE: f64 = 0.25;

/// What the player asks from the block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffersAction {
    Search,
    More,
}

/// The price of one item that tells a suspicious offer: the price of the price database, in
/// Exalted Orbs, with the rates of its currencies.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reference {
    pub exalted: f64,
    pub rates: Rates,
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
    /// The stock of an offer of the currency exchange.
    stock: Option<u64>,
    away: bool,
    listed: Option<Duration>,
}

/// The currency of the prices of the trade site `id`.
fn currency(id: &str) -> Option<Currency> {
    match id {
        "exalted" => Some(Currency::Exalted),
        "divine" => Some(Currency::Divine),
        "chaos" => Some(Currency::Chaos),
        _ => None,
    }
}

/// `amount` of the currency `id`, in Exalted Orbs; `None` for a currency without a rate.
fn in_exalted(amount: f64, id: &str, rates: Rates) -> Option<f64> {
    Some(match currency(id)? {
        Currency::Exalted => amount,
        Currency::Divine => amount * rates.divine,
        Currency::Chaos => amount * rates.chaos,
    })
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
    reference.is_some_and(|reference| {
        in_exalted(amount, id, reference.rates)
            .is_some_and(|exalted| exalted < reference.exalted * SUSPICIOUS_SHARE)
    })
}

fn listed_since(listed: Option<SystemTime>, now: SystemTime) -> Option<Duration> {
    listed.map(|listed| now.duration_since(listed).unwrap_or_default())
}

/// The rows of the offers of a search of items, at `now`.
fn item_rows(offers: &[ItemOffer], reference: Option<Reference>, now: SystemTime) -> Vec<Row> {
    offers
        .iter()
        .map(|offer| Row {
            price: offer
                .price
                .as_ref()
                .map(|price| row_price(price.amount, &price.currency)),
            suspicious: offer
                .price
                .as_ref()
                .is_some_and(|Price { amount, currency }| {
                    is_suspicious(*amount, currency, reference)
                }),
            stock: None,
            away: offer.seller == SellerStatus::Away,
            listed: listed_since(offer.listed, now),
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
    /// The copied item, whose rolls the tooltips compare.
    item: &'a Item,
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
        i18n: &'a Translator,
    ) -> Self {
        Self {
            offers,
            kind,
            reference,
            item,
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
            OffersState::Idle => self.search_band(
                ui,
                &[(i18n.text(Message::OffersSearchHint), color::WEAK)],
                block(Block::Search, wait, busy),
            ),
            OffersState::Unreachable => self.search_band(
                ui,
                &[(i18n.text(Message::OffersUnreachable), color::TEXT)],
                block(Block::Search, wait, busy),
            ),
            OffersState::Limited => {
                let line = if wait.is_zero() {
                    (i18n.text(Message::OffersSearchHint), color::WEAK)
                } else {
                    (i18n.offers_limit(wait), color::WARNING)
                };
                self.search_band(ui, &[line], block(Block::Search, wait, busy))
            }
            OffersState::Searching => {
                self.searching(ui);
                None
            }
            OffersState::Items {
                offers,
                total,
                more,
                loading_more,
                more_failed,
                searched,
            } => {
                let rows = item_rows(offers, self.reference, now);
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

    /// The label and `lines` beside an action block glued to the right edge, the height of
    /// the band.
    fn search_band(
        &self,
        ui: &mut Ui,
        lines: &[(String, Color32)],
        block: Block,
    ) -> Option<OffersAction> {
        let mut action = None;
        band(ui, |ui, width| {
            let text_width = (width - BLOCK_WIDTH - size::BAND_PADDING).max(0.0);
            let label = label(ui, self.i18n);
            let galleys: Vec<_> = lines
                .iter()
                .map(|(line, tint)| (wrapped_with(ui, line, text::item_line(), text_width), *tint))
                .collect();
            let lines_height: f32 = galleys.iter().map(|(galley, _)| galley.size().y).sum();
            let height = PADDING_Y * 2.0 + label.size().y + TITLE_GAP + lines_height;
            let (rect, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
            let painter = ui.painter();
            let left = rect.left() + size::BAND_PADDING;
            painter.galley(
                pos2(left, rect.top() + PADDING_Y),
                label.clone(),
                color::WEAK,
            );
            let mut top = rect.top() + PADDING_Y + label.size().y + TITLE_GAP;
            for (galley, tint) in galleys {
                let height = galley.size().y;
                painter.galley(pos2(left, top), galley, tint);
                top += height;
            }
            let block_rect = Rect::from_min_max(
                pos2(rect.right() - BLOCK_WIDTH, rect.top()),
                rect.right_bottom(),
            );
            if self.action_block(ui, block_rect, block) {
                action = Some(OffersAction::Search);
            }
        });
        action
    }

    /// The label, then the spinner and "Searching the trade site…".
    fn searching(&self, ui: &mut Ui) {
        band(ui, |ui, _| {
            ui.add_space(PADDING_Y);
            let label = label(ui, self.i18n);
            let message = widgets::text_line(
                ui,
                self.i18n.text(Message::OffersSearching),
                text::item_line(),
            );
            let line_height = message.size().y.max(SPINNER_RADIUS * 2.0);
            let height = label.size().y + SEARCHING_GAP + line_height;
            let (rect, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
            let painter = ui.painter();
            let left = rect.left() + size::BAND_PADDING;
            painter.galley(pos2(left, rect.top()), label.clone(), color::WEAK);
            let center_y = rect.bottom() - line_height / 2.0;
            let center = pos2(left + SPINNER_RADIUS, center_y);
            let time = ui.input(|input| input.time);
            spinner(painter, center, time);
            painter.galley(
                pos2(
                    center.x + SPINNER_RADIUS + SPINNER_GAP,
                    center_y - message.size().y / 2.0,
                ),
                message,
                color::TEXT,
            );
            ui.ctx().request_repaint();
            ui.add_space(PADDING_Y);
        });
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
                age: i18n.offers_searched(&i18n.age(age)),
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

    /// The label and the age of the offers, the count on the right, then the waiting block
    /// in the top right corner; whether the block was clicked.
    fn head(&self, ui: &mut Ui, head: &Head) -> bool {
        let (rect, _) =
            ui.allocate_exact_size(vec2(ui.available_width(), HEAD_HEIGHT), Sense::hover());
        let label = label(ui, self.i18n);
        let age = widgets::text_line(ui, head.age.as_str(), text::small());
        let painter = ui.painter();
        let left = rect.left() + size::BAND_PADDING;
        let top = rect.center().y - (label.size().y + TITLE_GAP + age.size().y) / 2.0;
        painter.galley(pos2(left, top), label.clone(), color::WEAK);
        painter.galley(
            pos2(left, top + label.size().y + TITLE_GAP),
            age,
            color::FAINT,
        );
        let block_rect = Rect::from_min_max(
            pos2(rect.right() - BLOCK_WIDTH, rect.top()),
            rect.right_bottom(),
        );
        if let Some(count) = &head.count {
            let galley = widgets::text_line(ui, count.as_str(), text::tag());
            let size = galley.size();
            ui.painter().galley(
                pos2(
                    block_rect.left() - COUNT_GAP - size.x,
                    rect.center().y - size.y / 2.0,
                ),
                galley,
                color::WEAK,
            );
        }
        self.action_block(ui, block_rect, head.block)
    }

    /// Draws `block` in `rect`; whether it was clicked, and could be.
    fn action_block(&self, ui: &mut Ui, rect: Rect, block: Block) -> bool {
        let i18n = self.i18n;
        let clickable = !matches!(block, Block::Waiting(_));
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
            Block::Waiting(None) => i18n.text(Message::OffersBusyTooltip),
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

        // Only the offers of items have a tooltip, and light up under the pointer.
        let sense = if list.items.is_some() {
            Sense::click()
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
        let id_of = |response: Option<&Response>| {
            response.and_then(|response| {
                let index = responses.iter().position(|other| other.id == response.id)?;
                ids.get(index).copied()
            })
        };
        let over_tooltip = state.open().is_some_and(|id| {
            let area = ctx.memory(|memory| memory.area_rect(OfferTooltip::area_id(id)));
            let pointer = ctx.pointer_latest_pos();
            area.zip(pointer)
                .is_some_and(|(area, pointer)| area.contains(pointer))
        });
        let (any_click, escape, time) = ui.input(|input| {
            (
                input.pointer.any_click(),
                input.key_pressed(Key::Escape),
                input.time,
            )
        });
        let pointer = Pointer {
            hovered: id_of(responses.iter().find(|response| response.hovered())),
            clicked: id_of(responses.iter().find(|response| response.clicked())),
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
                modifiers: self.item.modifiers(),
                name_color: rarity_color(self.item.rarity()),
                suspicious: row.suspicious,
                copied: state.shows_copied(&offer.id, time),
                i18n: self.i18n,
            };
            let (copy, _) = tooltip.show(ui, response.rect, ui.clip_rect().top());
            if copy {
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
            let tint = if row.suspicious {
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
                orb(currency, ROW_ORB).paint_at(ui, orb_rect);
                right = orb_rect.right();
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
    age: String,
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
                name: Some("Headhunter".to_owned()),
                base_type: "Heavy Belt".to_owned(),
                item_level: None,
                corrupted: true,
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

        let rows = item_rows(&offers, Some(REFERENCE), now);

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
                stock: None,
                away: true,
                listed: Some(Duration::from_hours(2)),
            }
        );
        assert!(
            item_rows(&offers, None, now)
                .iter()
                .all(|row| !row.suspicious),
            "without a market price, nothing tells"
        );
    }

    #[test]
    fn another_currency_is_written_by_its_name_and_never_suspicious() {
        let rows = item_rows(
            &[offer(1.0, "annul", SellerStatus::Online)],
            Some(REFERENCE),
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
