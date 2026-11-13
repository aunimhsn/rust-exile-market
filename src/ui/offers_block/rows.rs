//! The list of the offers: the titles of its columns, then a row per offer, its price, its stock
//! and its listing time, and the tooltip of an offer of items under the pointer.

use std::sync::Arc;
use std::time::{Duration, SystemTime};

use egui::text::{LayoutJob, TextFormat};
use egui::{
    Color32, Galley, Id, Key, Modifiers, Pos2, Rangef, Rect, Response, Role, Sense, Stroke, Ui,
    Vec2, WidgetInfo, pos2, vec2,
};

use super::{List, OffersBlock};
use crate::clipboard;
use crate::exchange_currencies::{self, ExchangeCurrency};
use crate::format;
use crate::i18n::{Message, Translator};
use crate::item::Catalyst;
use crate::language::Language;
use crate::price::Orb;
use crate::price::estimate::Reference;
use crate::trade::offers::{ExchangeOffer, ItemOffer, Price, SellerStatus};
use crate::trade::query::SearchKind;
use crate::ui::item_block::rarity_color;
use crate::ui::offer_tooltip::{OfferTooltip, Pointer, TooltipState, reach};
use crate::ui::theme::{color, size, text};
use crate::ui::{images, widgets};

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

/// The price of a row, in its currency.
#[derive(Debug, Clone, PartialEq)]
pub struct RowPrice {
    pub amount: String,
    /// Its currency, drawn by its icon, its name after it in a row; `None` for a currency
    /// whose offers aren't shown (a waystone, or one the application doesn't know): the
    /// offers of items in it are left out, those of the currency exchange are paid in
    /// Exalted or Divine Orbs.
    pub currency: Option<&'static ExchangeCurrency>,
}

impl RowPrice {
    /// The amount and the name of its currency in `language`, as a screen reader reads the
    /// row: `1 Vaal Orb`.
    pub fn label(&self, language: Language) -> String {
        match self.currency {
            Some(currency) => format!("{} {}", self.amount, currency.name(language)),
            None => self.amount.clone(),
        }
    }
}

/// A row of the list.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Row {
    /// `None` for an item listed without a price.
    price: Option<RowPrice>,
    /// Far below the reference price: usually bait or a mistake.
    is_suspicious: bool,
    /// The kind of the quality of a catalyst of an offer, when it isn't the kind searched:
    /// it isn't counted in the estimate.
    other_quality: Option<Catalyst>,
    /// The stock of an offer of the currency exchange.
    stock: Option<u64>,
    is_away: bool,
    listed: Option<Duration>,
}

impl Row {
    /// Whether its price is drawn faint: a suspicious offer, or one of another quality.
    fn is_faint(&self) -> bool {
        self.is_suspicious || self.other_quality.is_some()
    }
}

/// `amount` of the currency `id`, as a row writes it: the other currencies than the three
/// of the prices with the decimals of the Divine Orbs.
fn row_price(amount: f64, id: &str) -> RowPrice {
    RowPrice {
        amount: format::amount(amount, Orb::of(id).unwrap_or(Orb::Divine)),
        currency: exchange_currencies::shown(id),
    }
}

/// The price of an offer of items, as its row and its tooltip write it.
pub fn offer_price(price: &Price) -> RowPrice {
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
pub(super) fn item_rows(
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
                is_suspicious: other_quality.is_none()
                    && offer
                        .price
                        .as_ref()
                        .is_some_and(|Price { amount, currency }| {
                            is_suspicious(*amount, currency, reference)
                        }),
                other_quality,
                stock: None,
                is_away: offer.seller == SellerStatus::Away,
                listed: listed_since(offer.listed, now),
            }
        })
        .collect()
}

/// The rows of the offers of the currency exchange, at `now`: the price of one item.
pub(super) fn exchange_rows(
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
                is_suspicious: is_suspicious(unit, &offer.pay.currency, reference),
                other_quality: None,
                stock: Some(offer.stock),
                is_away: offer.seller == SellerStatus::Away,
                listed: listed_since(offer.listed, now),
            }
        })
        .collect()
}

/// What follows the name of the currency of an offer, drawn after it.
struct AfterName {
    galley: Arc<Galley>,
    /// The space between the name and it.
    gap: f32,
    tint: Color32,
}

/// Draws the `amount` of the price of `row` from `start`, at the middle of its row, struck
/// through on a suspicious offer; its width.
fn row_amount(ui: &Ui, row: &Row, amount: &str, start: Pos2) -> f32 {
    let galley = widgets::text_line(ui, amount, text::list_amount());
    let size = galley.size();
    let tint = if row.is_faint() {
        color::FAINT
    } else {
        color::TEXT
    };
    let painter = ui.painter();
    painter.galley(pos2(start.x, start.y - size.y / 2.0), galley, tint);
    if row.is_suspicious {
        painter.hline(
            start.x..=start.x + size.x,
            start.y,
            Stroke::new(1.0, color::FAINT),
        );
    }
    size.x
}

/// The titles of the columns of the offers of a search of `kind`, from left to right.
fn column_titles(i18n: &Translator, kind: SearchKind) -> Vec<String> {
    match kind {
        SearchKind::Exchange => vec![
            i18n.text(Message::OffersColumnPerUnit),
            i18n.text(Message::OffersColumnStock),
            i18n.text(Message::OffersColumnListed),
        ],
        SearchKind::Items => vec![
            i18n.text(Message::OffersColumnPrice),
            i18n.text(Message::OffersColumnListed),
        ],
    }
}

/// What the pointer and the keyboard did over the rows of the offers of `ids`, their
/// `responses`, and over the tooltip `open`, if one is.
fn pointer<'a>(
    ui: &Ui,
    open: Option<&str>,
    ids: &[&'a str],
    responses: &[Response],
) -> Pointer<'a> {
    let ctx = ui.ctx();
    let id_at = |index: Option<usize>| index.and_then(|index| ids.get(index).copied());
    // The tooltip as drawn at the last frame, and the gap to its row.
    let over_tooltip = open.is_some_and(|id| {
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
    Pointer {
        hovered: id_at(responses.iter().position(Response::hovered)),
        clicked: id_at(responses.iter().position(Response::clicked)),
        over_tooltip,
        any_click,
        escape,
        time,
    }
}

/// Where the columns are across a row, the titles or an offer.
struct Columns {
    left: f32,
    right: f32,
    /// The top and the bottom of the row, whose texts are centered across it.
    y_range: Rangef,
    /// The search of the offers: those of the currency exchange have a column of their stock.
    kind: SearchKind,
}

impl Columns {
    /// The columns across `row`, the titles or an offer, for a search of `kind`.
    fn of(row: Rect, kind: SearchKind) -> Self {
        Self {
            left: row.left() + size::BAND_PADDING,
            right: row.right() - size::BAND_PADDING,
            y_range: row.y_range(),
            kind,
        }
    }

    /// The right edges of the columns after the first one, whose text is aligned on the
    /// right.
    fn right_edges(&self) -> Vec<f32> {
        match self.kind {
            SearchKind::Exchange => vec![self.right - LISTED_WIDTH - COLUMN_GAP, self.right],
            SearchKind::Items => vec![self.right],
        }
    }
}

impl OffersBlock<'_> {
    /// The titles of the columns, then the rows, the whole width of the band.
    pub(super) fn list(&self, ui: &mut Ui, list: &List) {
        let kind = list.kind();
        self.titles_row(ui, kind);

        // Only the offers of items have a tooltip, and light up under the pointer; without
        // the focus of the keyboard, which the rows wouldn't show.
        let has_tooltips = kind == SearchKind::Items;
        let sense = if has_tooltips {
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
        let open = self.tooltip(ui, list, &responses);
        for (index, (row, response)) in list.rows.iter().zip(&responses).enumerate() {
            let rect = response.rect;
            if has_tooltips {
                let language = self.item.language();
                let label = row
                    .price
                    .as_ref()
                    .map(|price| price.label(language))
                    .unwrap_or_default();
                response.widget_info(|| WidgetInfo::labeled(Role::Button, true, &label));
            }
            let highlighted = has_tooltips && (response.hovered() || open == Some(index));
            if highlighted {
                ui.painter().rect_filled(rect, 0.0, color::CONTROL_LINE);
            } else if index % 2 == 0 {
                ui.painter().rect_filled(rect, 0.0, color::ROW_ALT);
            }
            // The whole name of a currency cut, under the pointer; the offers of items have
            // the tooltip of their item instead.
            if self.row(ui, &Columns::of(rect, kind), row)
                && !has_tooltips
                && let Some(price) = &row.price
            {
                response
                    .clone()
                    .on_hover_text(price.label(self.item.language()));
            }
        }
    }

    /// The titles of the columns of a search of `kind`, each over its column.
    fn titles_row(&self, ui: &mut Ui, kind: SearchKind) {
        let galleys: Vec<_> = column_titles(self.i18n, kind)
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
        let columns = Columns::of(rect, kind);
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
    }

    /// Opens, pins or closes the tooltip of an offer of items of `list` after what the
    /// pointer did over their rows (`responses`), then draws it; the index of the offer whose
    /// tooltip is open. The offers of the currency exchange have none.
    fn tooltip(&self, ui: &Ui, list: &List, responses: &[Response]) -> Option<usize> {
        let offers = list.items?;
        let ctx = ui.ctx();
        // One per window: the main one and the one over the game each have their own.
        let state_id = Id::NULL.with(("offer-tooltip-state", ctx.viewport_id()));
        let mut state: TooltipState = ctx.data(|data| data.get_temp(state_id).unwrap_or_default());
        let ids: Vec<&str> = offers.iter().map(|offer| offer.id.as_str()).collect();
        let pointer = pointer(ui, state.open(), &ids, responses);
        let time = pointer.time;
        // Esc closes the tooltip, and nothing under it (the window over the game on a single
        // screen, which Esc closes too).
        if pointer.escape && state.open().is_some() {
            ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape));
        }
        if let Some(redraw) = state.update(&pointer, &ids) {
            ctx.request_repaint_after(redraw);
        }
        let open = state
            .open()
            .and_then(|id| ids.iter().position(|other| *other == id));
        if let Some(index) = open
            && let (Some(offer), Some(row), Some(response)) = (
                offers.get(index),
                list.rows.get(index),
                responses.get(index),
            )
        {
            let tooltip = OfferTooltip {
                offer,
                item: self.item,
                searched_catalyst: self.offers.searched_catalyst(self.item),
                lines: self.lines,
                name_color: rarity_color(offer.item.rarity.unwrap_or(self.item.rarity())),
                is_suspicious: row.is_suspicious,
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

    /// Draws `row` in its `columns`: its stock and its listing time on the right, then its
    /// price before them; whether the name of its currency was cut.
    fn row(&self, ui: &Ui, columns: &Columns, row: &Row) -> bool {
        let center_y = columns.y_range.center();
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
        let dot = if row.is_away {
            color::WARNING
        } else {
            color::RISE
        };
        let dot_center = pos2(left - DOT_GAP - DOT_RADIUS, center_y);
        painter.circle_filled(dot_center, DOT_RADIUS, dot);
        used = used.min(dot_center.x - DOT_RADIUS);
        // A narrow row leaves the price no room: an empty one, never an inverted rectangle.
        let right = (used - COLUMN_GAP).max(columns.left);
        let room = Rect::from_x_y_ranges(columns.left..=right, columns.y_range);
        self.row_price(ui, row, room)
    }

    /// Draws the price of `row` in the `room` left for it, from its left at its middle: its
    /// amount, the icon of its currency, its name, then the kind of another quality or the "?"
    /// of a suspicious offer; whether the name was cut. An item listed without a price draws
    /// nothing.
    fn row_price(&self, ui: &Ui, row: &Row, room: Rect) -> bool {
        let Some(price) = &row.price else {
            return false;
        };
        let painter = ui.painter();
        let start = room.left_center();
        let center_y = start.y;
        let mut right = start.x + row_amount(ui, row, &price.amount, start);
        // What follows the name of the currency: the name is cut so that it fits.
        let after = self.after_name(ui, row);
        let after_width = after
            .as_ref()
            .map_or(0.0, |after| after.gap + after.galley.size().x);
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
            let width = (room.right() - after_width - left).max(0.0);
            let name = currency.name(self.item.language());
            let galley = widgets::truncated_line(ui, name, text::note(), width);
            cut = galley.elided;
            let name_size = galley.size();
            let name_tint = if row.is_faint() {
                color::FAINT
            } else {
                color::WEAK
            };
            painter.galley(pos2(left, center_y - name_size.y / 2.0), galley, name_tint);
            right = left + name_size.x;
        }
        if let Some(after) = after {
            let top = center_y - after.galley.size().y / 2.0;
            painter.galley(pos2(right + after.gap, top), after.galley, after.tint);
        }
        cut
    }

    /// What follows the name of the currency of `row`: the kind of another quality, written
    /// as its filter after a dot, "· Caster quality", so that it isn't read as a name; or else
    /// the "?" of a suspicious offer.
    fn after_name(&self, ui: &Ui, row: &Row) -> Option<AfterName> {
        if let Some(catalyst) = row.other_quality {
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
            Some(AfterName {
                galley: ui.painter().layout_job(job),
                gap: 0.0,
                tint: color::WEAK,
            })
        } else if row.is_suspicious {
            Some(AfterName {
                galley: widgets::text_line(ui, "?", text::list_amount()),
                gap: ORB_GAP + SUSPICIOUS_GAP,
                tint: color::WARNING,
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests;
