//! The tooltip of an offer of items: its item, its price, the difference of each roll from
//! the copied item, the seller status and the Copy whisper pill. It opens over a row under
//! the pointer, or under the row when there is no room above.
//!
//! When it opens and closes is decided apart from the drawing ([`TooltipState::update`]),
//! and tested.

use std::time::Duration;

use egui::{
    Align2, Area, Color32, CornerRadius, Frame, Id, Margin, Order, Pos2, Rect, Response, Sense,
    Stroke, Ui, Vec2, pos2, vec2,
};

use super::offers_block::{RowPrice, offer_price};
use super::price_block::orb;
use super::theme::{self, color, size, text};
use super::widgets;
use crate::filters::Property;
use crate::format;
use crate::i18n::{Message, NumberTag, Translator};
use crate::item::{Catalyst, Item};
use crate::roll_difference::{CopiedLines, Difference, Verdict};
use crate::trade::offers::{ItemOffer, ModifierKind, SellerStatus};

/// How long the tooltip stays open after the pointer left its row, to reach it.
const LINGER: f64 = 0.3;
/// How long the pill reads "Copied".
const COPIED_TIME: f64 = 2.0;
const WIDTH: f32 = 300.0;
const PADDING_X: i8 = 14;
const PADDING_Y: i8 = 12;
/// The space between the tooltip and its row, and from the right edge of the row.
const ROW_GAP: f32 = 6.0;
const RIGHT_INSET: f32 = 10.0;
/// The space above a group of modifiers, and between its line and its first modifier.
const GROUP_GAP: f32 = 8.0;
const LINE_GAP: f32 = 3.0;
/// The space between a modifier and its difference.
const DIFFERENCE_GAP: f32 = 10.0;
const META_GAP: f32 = 2.0;
const WARNING_GAP: f32 = 8.0;
const FOOT_GAP: f32 = 10.0;
const ORB: f32 = 18.0;
const ORB_GAP: f32 = 6.0;
const DOT_RADIUS: f32 = 3.5;
const DOT_GAP: f32 = 6.0;
const CHECK_SIZE: f32 = 11.0;
const CHECK_GAP: f32 = 5.0;
const PILL_PADDING: Vec2 = vec2(10.0, 4.0);

/// The tooltip open, from one frame to the next.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TooltipState {
    open: Option<Open>,
    /// The offer whose whisper was copied, and when, in seconds of egui.
    copied: Option<(String, f64)>,
}

#[derive(Debug, Clone, PartialEq)]
struct Open {
    /// The id of the offer.
    id: String,
    /// Opened by a click on its row: it stays until a click elsewhere or Escape.
    pinned: bool,
    /// When the pointer left the row and the tooltip; `None` while it is over them.
    left_at: Option<f64>,
}

/// What the pointer and the keyboard did during a frame.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Pointer<'a> {
    /// The id of the offer of the row under the pointer.
    pub hovered: Option<&'a str>,
    pub clicked: Option<&'a str>,
    /// Over the tooltip, or in the gap between it and its row, where the pointer passes on
    /// the edge of the next row to reach it.
    pub over_tooltip: bool,
    /// A click anywhere: on a row, in the tooltip, or elsewhere.
    pub any_click: bool,
    pub escape: bool,
    /// The time of egui, in seconds.
    pub time: f64,
}

impl TooltipState {
    /// The id of the offer whose tooltip is open.
    #[must_use]
    pub fn open(&self) -> Option<&str> {
        self.open.as_ref().map(|open| open.id.as_str())
    }

    /// Opens, pins or closes the tooltip after what the `pointer` did, among the offers
    /// whose ids are `offers`; how long before it closes by itself, to redraw then.
    pub fn update(&mut self, pointer: &Pointer, offers: &[&str]) -> Option<Duration> {
        // An offer gone with a new search, or another item.
        if self.open().is_some_and(|id| !offers.contains(&id)) {
            self.open = None;
        }
        if pointer.escape {
            self.open = None;
        } else if let Some(id) = pointer.clicked {
            self.open = Some(Open {
                id: id.to_owned(),
                pinned: true,
                left_at: None,
            });
        } else if pointer.any_click && !pointer.over_tooltip {
            self.open = None;
        } else if pointer.over_tooltip {
            if let Some(open) = &mut self.open {
                open.left_at = None;
            }
        } else if let Some(id) = pointer.hovered {
            match &mut self.open {
                Some(open) if open.pinned => {}
                Some(open) if open.id == id => open.left_at = None,
                _ => {
                    self.open = Some(Open {
                        id: id.to_owned(),
                        pinned: false,
                        left_at: None,
                    });
                }
            }
        } else if let Some(open) = &mut self.open
            && !open.pinned
        {
            let left_at = *open.left_at.get_or_insert(pointer.time);
            if pointer.time - left_at >= LINGER {
                self.open = None;
            }
        }
        let lingering = self
            .open
            .as_ref()
            .and_then(|open| open.left_at)
            .map(|left_at| LINGER - (pointer.time - left_at));
        let copied = self
            .copied
            .as_ref()
            .map(|(_, at)| COPIED_TIME - (pointer.time - at))
            .filter(|left| *left > 0.0);
        lingering
            .into_iter()
            .chain(copied)
            .reduce(f64::min)
            .map(|left| Duration::from_secs_f64(left.max(0.0)))
    }

    /// The whisper of the offer `id` was copied at `time`.
    pub fn copied(&mut self, id: &str, time: f64) {
        self.copied = Some((id.to_owned(), time));
    }

    /// Whether the pill of the offer `id` reads "Copied" at `time`.
    #[must_use]
    pub fn shows_copied(&self, id: &str, time: f64) -> bool {
        self.copied
            .as_ref()
            .is_some_and(|(copied, at)| copied == id && time - at < COPIED_TIME)
    }
}

/// What the tooltip shows of an offer.
pub struct OfferTooltip<'a> {
    pub offer: &'a ItemOffer,
    /// The copied item, whose quality of a catalyst the offer's is compared with.
    pub item: &'a Item,
    /// The kind of the quality of a catalyst searched: an offer of another kind isn't
    /// counted in the estimate.
    pub quality: Option<Catalyst>,
    /// The modifier lines of the copied item, to compare the rolls.
    pub lines: &'a CopiedLines,
    pub name_color: Color32,
    pub suspicious: bool,
    pub copied: bool,
    pub i18n: &'a Translator,
}

/// A line of the offer, with its difference from the copied item.
#[derive(Debug, Clone, PartialEq)]
struct Line {
    text: String,
    tint: Color32,
    difference: Option<Difference>,
}

/// The line of the quality of an offer, when the copied `item` has the quality of a
/// catalyst: gray with its difference; white at the maximum of the offer, or beyond it; red
/// without difference when of another kind.
fn quality_line(offer: &ItemOffer, item: &Item, i18n: &Translator) -> Option<Line> {
    let copied = item.quality_kind()?;
    let quality = offer.item.quality.as_ref()?;
    let value = format::number(f64::from(quality.value));
    let text = match &quality.kind {
        Some(kind) => i18n.filter_catalyst_quality(kind, &value),
        None => i18n.filter_property(Property::Quality, &value),
    };
    if copied
        .catalyst
        .is_some_and(|catalyst| offer.has_other_quality(catalyst))
    {
        return Some(Line {
            text,
            tint: color::FALL,
            difference: None,
        });
    }
    let tint = if quality.value >= offer.item.max_catalyst_quality() {
        color::TEXT
    } else {
        color::WEAK
    };
    let difference = item
        .quality()
        .map(|copied| f64::from(quality.value) - f64::from(copied))
        .filter(|difference| *difference != 0.0)
        .map(|value| Difference {
            value,
            verdict: if value > 0.0 {
                Verdict::Better
            } else {
                Verdict::Worse
            },
        });
    Some(Line {
        text,
        tint,
        difference,
    })
}

impl OfferTooltip<'_> {
    /// The id of the area of the tooltip of the offer `id`.
    #[must_use]
    pub fn area_id(id: &str) -> Id {
        Id::NULL.with("offer-tooltip").with(id)
    }

    /// Draws the tooltip over `row`, or under it when there is no room above `top`; whether
    /// Copy whisper was clicked.
    pub fn show(&self, ui: &Ui, row: Rect, top: f32) -> bool {
        // An area per offer, so that its height is known to place it: egui lays out a new
        // area once without showing it, so a tooltip shows one frame after it opens.
        let id = Self::area_id(&self.offer.id);
        let height = ui
            .ctx()
            .memory(|memory| memory.area_rect(id))
            .map_or(0.0, |rect| rect.height());
        let right = row.right() - RIGHT_INSET;
        let above = row.top() - ROW_GAP - height >= top;
        let (pivot, at) = if above {
            (Align2::RIGHT_BOTTOM, pos2(right, row.top() - ROW_GAP))
        } else {
            (Align2::RIGHT_TOP, pos2(right, row.bottom() + ROW_GAP))
        };
        let shown = Area::new(id)
            .order(Order::Foreground)
            .pivot(pivot)
            .fixed_pos(at)
            .show(ui.ctx(), |ui| {
                Frame::NONE
                    .fill(color::BUBBLE)
                    .corner_radius(CornerRadius::same(size::BUBBLE_RADIUS))
                    .shadow(theme::floating_shadow())
                    .inner_margin(Margin::symmetric(PADDING_X, PADDING_Y))
                    .show(ui, |ui| {
                        ui.set_width(WIDTH - 2.0 * f32::from(PADDING_X));
                        ui.spacing_mut().item_spacing = Vec2::ZERO;
                        self.content(ui)
                    })
                    .inner
            });
        shown.inner
    }

    fn content(&self, ui: &mut Ui) -> bool {
        let i18n = self.i18n;
        let item = &self.offer.item;
        self.head(ui);
        ui.add_space(META_GAP);
        let mut meta = vec![item.base_type.clone()];
        if let Some(level) = item.item_level {
            meta.push(i18n.tag(NumberTag::ItemLevel, level));
        }
        if item.corrupted {
            meta.push(i18n.text(Message::TagCorrupted));
        }
        text_block(ui, &meta.join(" · "), text::note(), color::WEAK);
        if self.suspicious {
            ui.add_space(WARNING_GAP);
            text_block(
                ui,
                &i18n.text(Message::OffersSuspicious),
                text::note(),
                color::WARNING,
            );
        }
        if self
            .quality
            .is_some_and(|catalyst| self.offer.has_other_quality(catalyst))
        {
            ui.add_space(WARNING_GAP);
            text_block(
                ui,
                &i18n.text(Message::OffersOtherQuality),
                text::note(),
                color::WEAK,
            );
        }
        if let Some(line) = quality_line(self.offer, self.item, i18n) {
            group(ui, &[line]);
        }
        for kind in [
            ModifierKind::Enchant,
            ModifierKind::Rune,
            ModifierKind::Implicit,
            ModifierKind::Explicit,
        ] {
            let lines = self.lines(kind);
            if !lines.is_empty() {
                group(ui, &lines);
            }
        }
        ui.add_space(FOOT_GAP);
        self.foot(ui)
    }

    /// The name on the left, the price with its orb on the right.
    fn head(&self, ui: &mut Ui) {
        let item = &self.offer.item;
        let price = self.offer.price.as_ref().map(offer_price);
        let price_width = price.as_ref().map_or(0.0, |price| price_size(ui, price).x);
        let width = ui.available_width();
        let name = widgets::truncated_line(
            ui,
            item.name.as_deref().unwrap_or(&item.base_type),
            text::tooltip_name(),
            (width - price_width - DIFFERENCE_GAP).max(0.0),
        );
        let height = name.size().y.max(ORB);
        let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        let painter = ui.painter();
        painter.galley(
            pos2(rect.left(), rect.center().y - name.size().y / 2.0),
            name,
            self.name_color,
        );
        if let Some(price) = price {
            paint_price(
                ui,
                &price,
                pos2(rect.right() - price_width, rect.center().y),
            );
        }
    }

    /// The modifier lines of `kind`, a line each, with their differences.
    fn lines(&self, kind: ModifierKind) -> Vec<Line> {
        self.offer
            .item
            .modifiers
            .iter()
            .filter(|modifier| modifier.kind == kind)
            .flat_map(|modifier| modifier.text.lines())
            .map(|line| Line {
                text: line.to_owned(),
                tint: color::MODIFIER,
                difference: self.lines.difference(line, kind),
            })
            .collect()
    }

    /// The seller status on the left; Copy whisper on the right, when the seller can answer.
    /// Whether the pill was clicked.
    fn foot(&self, ui: &mut Ui) -> bool {
        let i18n = self.i18n;
        let (status, dot) = match self.offer.seller {
            SellerStatus::Online => (Message::SellerOnline, color::RISE),
            SellerStatus::Away => (Message::SellerAway, color::WARNING),
            SellerStatus::Offline => (Message::SellerOffline, color::FALL),
        };
        let status = widgets::text_line(ui, i18n.text(status), text::note());
        let whisper = (self.offer.seller != SellerStatus::Offline).then(|| {
            if self.copied {
                i18n.text(Message::WhisperCopied)
            } else {
                i18n.text(Message::CopyWhisper)
            }
        });
        let pill = whisper
            .as_ref()
            .map(|label| widgets::text_line(ui, label.as_str(), text::pill()));
        let pill_size = pill.as_ref().map(|galley| {
            let check = if self.copied {
                CHECK_GAP + CHECK_SIZE
            } else {
                0.0
            };
            galley.size() + 2.0 * PILL_PADDING + vec2(check, 0.0)
        });
        let height = pill_size.map_or(status.size().y, |size| size.y.max(status.size().y));
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
        let center_y = rect.center().y;
        let painter = ui.painter();
        painter.circle_filled(pos2(rect.left() + DOT_RADIUS, center_y), DOT_RADIUS, dot);
        painter.galley(
            pos2(
                rect.left() + 2.0 * DOT_RADIUS + DOT_GAP,
                center_y - status.size().y / 2.0,
            ),
            status,
            color::WEAK,
        );
        let (Some(galley), Some(pill_size), Some(label)) = (pill, pill_size, whisper) else {
            return false;
        };
        let pill_rect = Rect::from_min_size(
            pos2(rect.right() - pill_size.x, center_y - pill_size.y / 2.0),
            pill_size,
        );
        let response = ui.interact(
            pill_rect,
            ui.scope_id().with("copy-whisper"),
            Sense::click(),
        );
        response
            .widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label.as_str()));
        self.paint_pill(ui, &response, galley);
        response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
    }

    /// Copy whisper on the color of the controls, lighter under the pointer; "Copied" and a
    /// check mark on the accent.
    fn paint_pill(&self, ui: &Ui, response: &Response, galley: std::sync::Arc<egui::Galley>) {
        let rect = response.rect;
        let lit = response.hovered() || response.has_focus();
        let (fill, tint) = if self.copied {
            (color::ACCENT, color::ON_ACCENT)
        } else if lit {
            (color::CONTROL_LINE, color::ACCENT)
        } else {
            (color::CONTROL, color::TEXT)
        };
        let painter = ui.painter();
        painter.rect_filled(rect, CornerRadius::same(size::RADIUS), fill);
        let text_left = rect.left() + PILL_PADDING.x;
        let text_width = galley.size().x;
        painter.galley(pos2(text_left, rect.top() + PILL_PADDING.y), galley, tint);
        if self.copied {
            widgets::check_mark(
                painter,
                pos2(
                    text_left + text_width + CHECK_GAP + CHECK_SIZE / 2.0,
                    rect.center().y,
                ),
                CHECK_SIZE,
                tint,
            );
        }
    }
}

/// The gap between the tooltip `area` and its `row`, as wide as both: the pointer crosses it
/// to reach the tooltip.
#[must_use]
pub fn reach(area: Rect, row: Rect) -> Rect {
    let (top, bottom) = if area.bottom() <= row.top() {
        (area.bottom(), row.top())
    } else {
        (row.bottom(), area.top())
    };
    Rect::from_min_max(
        pos2(area.left().min(row.left()), top),
        pos2(area.right().max(row.right()), bottom.max(top)),
    )
}

/// A text wrapped at the width of `ui`.
fn text_block(ui: &mut Ui, line: &str, font: egui::FontId, tint: Color32) {
    let galley = ui.painter().layout(
        line.to_owned(),
        font,
        Color32::PLACEHOLDER,
        ui.available_width(),
    );
    let (rect, _) =
        ui.allocate_exact_size(vec2(ui.available_width(), galley.size().y), Sense::hover());
    ui.painter().galley(rect.min, galley, tint);
}

/// A group of modifiers under a thin line, each with its difference on the right.
fn group(ui: &mut Ui, lines: &[Line]) {
    ui.add_space(GROUP_GAP);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().hline(
        rect.x_range(),
        rect.center().y,
        Stroke::new(1.0, color::CONTROL_LINE),
    );
    ui.add_space(GROUP_GAP);
    for (index, line) in lines.iter().enumerate() {
        if index > 0 {
            ui.add_space(LINE_GAP);
        }
        let width = ui.available_width();
        let delta = line.difference.map(|difference| {
            let tint = match difference.verdict {
                Verdict::Better => color::RISE,
                Verdict::Worse => color::FALL,
                Verdict::Unknown => color::WEAK,
            };
            (
                widgets::text_line(ui, signed(difference.value), text::difference()),
                tint,
            )
        });
        let delta_width = delta
            .as_ref()
            .map_or(0.0, |(galley, _)| galley.size().x + DIFFERENCE_GAP);
        let galley = ui.painter().layout(
            line.text.clone(),
            text::tooltip_line(),
            Color32::PLACEHOLDER,
            (width - delta_width).max(0.0),
        );
        let (rect, _) = ui.allocate_exact_size(vec2(width, galley.size().y), Sense::hover());
        let first_line = galley
            .rows
            .first()
            .map_or(galley.size().y, |row| row.rect().height());
        let painter = ui.painter();
        painter.galley(rect.min, galley, line.tint);
        if let Some((delta, tint)) = delta {
            let size = delta.size();
            painter.galley(
                pos2(
                    rect.right() - size.x,
                    rect.top() + (first_line - size.y) / 2.0,
                ),
                delta,
                tint,
            );
        }
    }
}

/// A difference with its sign, a real minus sign when negative: `+6`, `−7`, `+3.5`.
fn signed(value: f64) -> String {
    let sign = if value < 0.0 { '−' } else { '+' };
    format!("{sign}{}", crate::format::number(value.abs()))
}

/// The size of a price: its amount, then its orb or the name of its currency.
fn price_size(ui: &Ui, price: &RowPrice) -> Vec2 {
    let amount = widgets::text_line(ui, price.text(), text::list_amount());
    let orb = if price.currency.is_ok() {
        ORB_GAP + ORB
    } else {
        0.0
    };
    vec2(amount.size().x + orb, amount.size().y.max(ORB))
}

/// Paints `price` from `left_center`.
fn paint_price(ui: &Ui, price: &RowPrice, left_center: Pos2) {
    let amount = widgets::text_line(ui, price.text(), text::list_amount());
    let size = amount.size();
    ui.painter().galley(
        pos2(left_center.x, left_center.y - size.y / 2.0),
        amount,
        color::TEXT,
    );
    if let Ok(currency) = price.currency {
        let orb_rect = Rect::from_min_size(
            pos2(left_center.x + size.x + ORB_GAP, left_center.y - ORB / 2.0),
            Vec2::splat(ORB),
        );
        orb(currency, ORB).paint_at(ui, orb_rect);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OFFERS: [&str; 3] = ["a", "b", "c"];

    fn at(time: f64) -> Pointer<'static> {
        Pointer {
            time,
            ..Pointer::default()
        }
    }

    fn hovering(id: &'static str, time: f64) -> Pointer<'static> {
        Pointer {
            hovered: Some(id),
            ..at(time)
        }
    }

    /// An amulet offered with the quality of a catalyst of `catalyst`, `value` %, and its
    /// `modifiers` (`+20% to Maximum Quality`).
    fn amulet(value: u32, catalyst: Catalyst, words: &str, modifiers: &[&str]) -> ItemOffer {
        use crate::item::QualityKind;
        use crate::trade::offers::{OfferItem, OfferModifier, OfferQuality};

        ItemOffer {
            id: "a".to_owned(),
            price: None,
            seller: SellerStatus::Online,
            listed: None,
            whisper: String::new(),
            item: OfferItem {
                rarity: None,
                name: None,
                base_type: "Solar Amulet".to_owned(),
                item_level: None,
                corrupted: false,
                quality: Some(OfferQuality {
                    value,
                    kind: Some(QualityKind {
                        catalyst: Some(catalyst),
                        words: words.to_owned(),
                    }),
                }),
                modifiers: modifiers
                    .iter()
                    .map(|text| OfferModifier {
                        kind: ModifierKind::Explicit,
                        text: (*text).to_owned(),
                        stat: Some("explicit.stat_2039822488".to_owned()),
                    })
                    .collect(),
            },
        }
    }

    #[test]
    fn the_quality_of_an_offer_is_white_at_its_maximum_and_red_of_another_kind() {
        use crate::item::CopiedItem;

        let i18n = Translator::new(crate::language::Language::English);
        // The owner's amulet: 40 % of mana quality.
        let copied = Item::parse(
            &CopiedItem::from_text(include_str!(
                "../item/test-items/en-owner-rare-amulet-catalyst.txt"
            ))
            .unwrap(),
        )
        .unwrap();
        let essence = ["+20% to Maximum Quality"];
        let line = |offer: &ItemOffer| quality_line(offer, &copied, &i18n).unwrap();
        let difference = |value, verdict| Some(Difference { value, verdict });

        // Below its maximum of 40 %: gray, with its difference.
        let below = line(&amulet(30, Catalyst::Mana, "Mana Modifiers", &essence));
        assert_eq!(below.text, "Mana quality: 30%");
        assert_eq!(below.tint, color::WEAK);
        assert_eq!(below.difference, difference(-10.0, Verdict::Worse));
        // At its maximum: white.
        let at_maximum = line(&amulet(40, Catalyst::Mana, "Mana Modifiers", &essence));
        assert_eq!(at_maximum.tint, color::TEXT);
        assert_eq!(at_maximum.difference, None);
        // Left beyond its maximum once its essence was removed: white too.
        let beyond = line(&amulet(50, Catalyst::Mana, "Mana Modifiers", &[]));
        assert_eq!(beyond.tint, color::TEXT);
        assert_eq!(beyond.difference, difference(10.0, Verdict::Better));
        // Another kind: red, without difference.
        let attack = line(&amulet(50, Catalyst::Attack, "Attack Modifiers", &[]));
        assert_eq!(attack.text, "Attack quality: 50%");
        assert_eq!(attack.tint, color::FALL);
        assert_eq!(attack.difference, None);
    }

    #[test]
    fn an_item_without_the_quality_of_a_catalyst_has_no_quality_line() {
        use crate::item::CopiedItem;

        let i18n = Translator::new(crate::language::Language::English);
        let bow = Item::parse(
            &CopiedItem::from_text(include_str!("../item/test-items/en-rare-bow-fractured.txt"))
                .unwrap(),
        )
        .unwrap();

        assert_eq!(
            quality_line(
                &amulet(40, Catalyst::Mana, "Mana Modifiers", &[]),
                &bow,
                &i18n
            ),
            None
        );
    }

    #[test]
    fn hovering_a_row_opens_its_tooltip_which_lingers_after_the_pointer_leaves() {
        let mut state = TooltipState::default();

        state.update(&hovering("a", 0.0), &OFFERS);
        assert_eq!(state.open(), Some("a"));

        let redraw = state.update(&at(1.0), &OFFERS);
        assert_eq!(state.open(), Some("a"), "still open a moment");
        assert_eq!(redraw, Some(Duration::from_secs_f64(LINGER)));

        state.update(&at(1.0 + LINGER), &OFFERS);
        assert_eq!(state.open(), None);
    }

    #[test]
    fn the_tooltip_stays_while_the_pointer_is_inside_it() {
        let mut state = TooltipState::default();
        state.update(&hovering("a", 0.0), &OFFERS);
        state.update(&at(0.1), &OFFERS);

        let inside = Pointer {
            over_tooltip: true,
            ..at(5.0)
        };
        state.update(&inside, &OFFERS);
        state.update(&inside, &OFFERS);

        assert_eq!(state.open(), Some("a"));
    }

    #[test]
    fn hovering_another_row_moves_the_tooltip_at_once() {
        let mut state = TooltipState::default();
        state.update(&hovering("a", 0.0), &OFFERS);
        state.update(&at(0.1), &OFFERS);

        state.update(&hovering("b", 0.15), &OFFERS);

        assert_eq!(state.open(), Some("b"));
    }

    #[test]
    fn a_click_pins_the_tooltip_until_a_click_elsewhere_or_escape() {
        let mut state = TooltipState::default();
        let click = Pointer {
            clicked: Some("a"),
            any_click: true,
            ..hovering("a", 0.0)
        };
        state.update(&click, &OFFERS);

        state.update(&at(10.0), &OFFERS);
        state.update(&hovering("b", 11.0), &OFFERS);
        assert_eq!(
            state.open(),
            Some("a"),
            "pinned: neither leaving nor hovering"
        );

        let click_inside = Pointer {
            any_click: true,
            over_tooltip: true,
            ..at(12.0)
        };
        state.update(&click_inside, &OFFERS);
        assert_eq!(state.open(), Some("a"), "Copy whisper keeps it");

        let click_elsewhere = Pointer {
            any_click: true,
            ..at(13.0)
        };
        state.update(&click_elsewhere, &OFFERS);
        assert_eq!(state.open(), None);

        state.update(&click, &OFFERS);
        let escape = Pointer {
            escape: true,
            ..at(14.0)
        };
        state.update(&escape, &OFFERS);
        assert_eq!(state.open(), None);
    }

    #[test]
    fn the_pointer_reaches_the_tooltip_over_the_edge_of_the_next_row() {
        let mut state = TooltipState::default();
        state.update(&hovering("b", 0.0), &OFFERS);

        // Between the row and the tooltip, the pointer is on the edge of the row above.
        let gap = Pointer {
            over_tooltip: true,
            ..hovering("a", 0.05)
        };
        state.update(&gap, &OFFERS);
        assert_eq!(state.open(), Some("b"));

        let inside = Pointer {
            over_tooltip: true,
            ..at(0.1)
        };
        let redraw = state.update(&inside, &OFFERS);
        assert_eq!(state.open(), Some("b"));
        assert_eq!(redraw, None, "nothing to wait for");
    }

    #[test]
    fn coming_back_to_the_row_keeps_the_tooltip_open() {
        let mut state = TooltipState::default();
        state.update(&hovering("a", 0.0), &OFFERS);
        state.update(&at(0.1), &OFFERS);

        state.update(&hovering("a", 0.2), &OFFERS);
        state.update(&at(0.35), &OFFERS);

        assert_eq!(
            state.open(),
            Some("a"),
            "the wait starts again from when it left"
        );
    }

    #[test]
    fn a_click_on_another_row_pins_its_tooltip() {
        let mut state = TooltipState::default();
        let click = |id: &'static str, time| Pointer {
            clicked: Some(id),
            any_click: true,
            ..hovering(id, time)
        };
        state.update(&click("a", 0.0), &OFFERS);

        state.update(&click("b", 1.0), &OFFERS);
        state.update(&at(5.0), &OFFERS);

        assert_eq!(state.open(), Some("b"));
    }

    #[test]
    fn a_click_inside_an_unpinned_tooltip_keeps_it_open() {
        let mut state = TooltipState::default();
        state.update(&hovering("a", 0.0), &OFFERS);

        let click_inside = Pointer {
            any_click: true,
            over_tooltip: true,
            ..at(0.1)
        };
        state.update(&click_inside, &OFFERS);

        assert_eq!(state.open(), Some("a"));
    }

    #[test]
    fn the_tooltip_of_an_offer_gone_closes() {
        let mut state = TooltipState::default();
        state.update(&hovering("a", 0.0), &OFFERS);

        state.update(&at(0.0), &["d"]);

        assert_eq!(state.open(), None);
    }

    #[test]
    fn the_pill_reads_copied_for_two_seconds() {
        let mut state = TooltipState::default();
        state.copied("a", 1.0);

        assert!(state.shows_copied("a", 2.9));
        assert!(!state.shows_copied("b", 2.9));
        assert!(!state.shows_copied("a", 3.0));
        let redraw = state.update(&at(2.0), &OFFERS);
        assert_eq!(redraw, Some(Duration::from_secs_f64(1.0)));
    }

    #[test]
    fn the_gap_to_the_row_is_on_the_side_of_the_tooltip() {
        let row = Rect::from_min_max(pos2(0.0, 100.0), pos2(420.0, 128.0));
        let above = Rect::from_min_max(pos2(110.0, 20.0), pos2(410.0, 94.0));
        let below = Rect::from_min_max(pos2(110.0, 134.0), pos2(410.0, 200.0));

        assert_eq!(
            reach(above, row),
            Rect::from_min_max(pos2(0.0, 94.0), pos2(420.0, 100.0))
        );
        assert_eq!(
            reach(below, row),
            Rect::from_min_max(pos2(0.0, 128.0), pos2(420.0, 134.0))
        );
    }

    #[test]
    fn a_difference_is_written_with_its_sign() {
        assert_eq!(signed(6.0), "+6");
        assert_eq!(signed(-7.0), "−7");
        assert_eq!(signed(3.5), "+3.5");
    }
}
