//! The tooltip of an offer of items: its item, its price, the difference of each roll from
//! the copied item, the seller status and the Copy whisper pill. It opens over a row under
//! the pointer, or under the row when there is no room above.
//!
//! When it opens and closes is decided apart from the drawing ([`TooltipState::update`]),
//! and tested.

use std::time::Duration;

use egui::text::{LayoutJob, TextFormat};
use egui::{
    Align2, Area, Color32, CornerRadius, Frame, Id, Margin, Order, Pos2, Rect, Response, Sense,
    Stroke, Ui, Vec2, pos2, vec2,
};

use super::item_block::{
    SEPARATOR, gem_level_color, requirement_parts, requirements_job, waystone_tier_color,
};
use super::item_stats::property_value;
use super::offers_block::{RowPrice, offer_price};
use super::price_block::orb;
use super::theme::{self, color, size, text};
use super::widgets;
use crate::filters::Property;
use crate::format;
use crate::game_data::{self, Category};
use crate::i18n::{Message, NumberTag, Translator};
use crate::item::{
    self, Catalyst, Item, MAX_GEM_SOCKETS, TOP_GEM_QUALITY, WaystoneProperty, WaystoneValue,
};
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
    pub searched_catalyst: Option<Catalyst>,
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
/// catalyst, of a known kind or not. Of another known kind than the item's: red, without
/// difference. Else, with its difference from the item: white at the maximum of the offer (20
/// %, more with its modifiers `+#% to Maximum Quality`) or beyond it, gray below it. The
/// quality of the base, or a kind the application doesn't know, is never of another kind.
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
        .and_then(|copied| higher_better(f64::from(quality.value), f64::from(copied)));
    Some(Line {
        text,
        tint,
        difference,
    })
}

/// The lines of the properties of an offer of waystone, when the copied `item` is a
/// waystone, in the order of the game: each with its difference from the item, gray; a
/// property of the item that the offer lacks, faint at 0, with its loss, unless a property of
/// the offer couldn't be read: it may be that one, under another name. Every property is
/// better higher: a buyer pays for the bonuses of the map.
fn waystone_lines(offer: &ItemOffer, item: &Item, i18n: &Translator) -> Vec<Line> {
    if !item.is_waystone() {
        return Vec::new();
    }
    let copied = item.waystone_properties();
    let value_of = |values: &[WaystoneValue], property| {
        values
            .iter()
            .find(|found| found.property == property)
            .map(|found| found.value)
    };
    WaystoneProperty::ALL
        .into_iter()
        .filter_map(|property| {
            let offered = value_of(&offer.item.waystone_properties, property);
            let copied = value_of(&copied, property);
            if offered.is_none() && (copied.is_none() || offer.item.waystone_unread) {
                return None;
            }
            let value = offered.unwrap_or(0);
            let text = i18n.filter_property(
                Property::Waystone(property),
                &property_value(Property::Waystone(property), f64::from(value)),
            );
            Some(Line {
                text,
                tint: if offered.is_some() {
                    color::WEAK
                } else {
                    color::FAINT
                },
                difference: higher_better(f64::from(value), f64::from(copied.unwrap_or(0))),
            })
        })
        .collect()
}

/// The difference of `value` from the `copied` one, of a property better higher: none when
/// they are equal.
fn higher_better(value: f64, copied: f64) -> Option<Difference> {
    let difference = value - copied;
    (difference != 0.0).then_some(Difference {
        value: difference,
        verdict: if difference > 0.0 {
            Verdict::Better
        } else {
            Verdict::Worse
        },
    })
}

/// The lines of the quality and the sockets of an offer of skill gem, when the copied `item`
/// is a skill gem, written like their filters, each with its difference from the item, better
/// higher: gray, white at their top (a quality of 20 % or more, 5 sockets). A quality of the
/// item that the offer lacks is written faint, at 0, with its loss; sockets the trade site
/// doesn't give aren't written, nor their difference when the copy doesn't give the item's
/// (every gem has 2, never 0). The level is under the name ([`meta`]).
fn gem_lines(offer: &ItemOffer, item: &Item, i18n: &Translator) -> Vec<Line> {
    if !item.is_skill_gem() {
        return Vec::new();
    }
    let offered = &offer.item;
    let quality = offered.quality.as_ref().map(|quality| quality.value);
    let quality = (quality.is_some() || item.quality().is_some()).then(|| {
        let value = quality.unwrap_or(0);
        Line {
            text: i18n.filter_property(Property::Quality, &format::number(f64::from(value))),
            tint: match quality {
                None => color::FAINT,
                Some(value) if value >= TOP_GEM_QUALITY => color::TEXT,
                Some(_) => color::WEAK,
            },
            difference: higher_better(f64::from(value), f64::from(item.quality().unwrap_or(0))),
        }
    });
    let sockets = offered.gem_sockets.map(|sockets| Line {
        text: i18n.filter_property(Property::GemSockets, &format::number(f64::from(sockets))),
        tint: if sockets >= MAX_GEM_SOCKETS {
            color::TEXT
        } else {
            color::WEAK
        },
        difference: item
            .sockets()
            .and_then(|copied| higher_better(f64::from(sockets), f64::from(copied))),
    });
    quality.into_iter().chain(sockets).collect()
}

/// The requirements of an offer of skill gem, weapon or armour, when the copied `item` is one,
/// wrapped at `width`: "Requires Level 70 · 67 Dex · 67 Int", each attribute in its color,
/// without difference: the level of the character is unknown. None for another item, whose
/// requirements matter less, or an offer without any.
fn requirements(
    offer: &ItemOffer,
    item: &Item,
    i18n: &Translator,
    width: f32,
) -> Option<LayoutJob> {
    // A magic item is known by its whole name, its affixes with it: the base of the offer,
    // the same category, tells it then.
    let category = game_data::category(item.language(), item.base_type())
        .or_else(|| game_data::category(item.language(), &offer.item.base_type));
    let equipped = item.is_skill_gem() || category.is_some_and(Category::is_weapon_or_armour);
    if !equipped {
        return None;
    }
    let parts = requirement_parts(offer.item.requirements, i18n)?;
    let mut job = requirements_job(&parts, i18n, &text::note());
    job.wrap.max_width = width;
    Some(job)
}

/// The base type of the offer, its item level and its corruption, wrapped at `width`. An
/// offer of waystone, when the copied `item` is one, writes its tier short, in its color, as
/// the item block does: "Waystone · T12". An offer of skill gem, when the copied `item` is one,
/// writes the item class of the item in place of its base type, the name of the gem again,
/// and its level in its color: "Skill Gems · Level 16".
fn meta(offer: &ItemOffer, item: &Item, i18n: &Translator, width: f32) -> LayoutJob {
    let offered = &offer.item;
    let font = text::note();
    let mut job = LayoutJob::default();
    job.wrap.max_width = width;
    let mut append = |text: &str, font: &egui::FontId, tint| {
        job.append(text, 0.0, TextFormat::simple(font.clone(), tint));
    };
    let waystone = if item.is_waystone() {
        item::base_and_tier(item.language(), &offered.base_type)
    } else {
        None
    };
    let gem_level = offered.gem_level.filter(|_| item.is_skill_gem());
    if let Some((name, tier)) = waystone {
        append(name, &font, color::WEAK);
        append(SEPARATOR, &font, color::WEAK);
        append(
            &format!("T{tier}"),
            &text::tooltip_tier(),
            waystone_tier_color(tier),
        );
    } else if let Some(level) = gem_level {
        append(item.class(), &font, color::WEAK);
        append(SEPARATOR, &font, color::WEAK);
        append(
            &i18n.gem_level(level),
            &text::tooltip_tier(),
            gem_level_color(level),
        );
    } else {
        append(&offered.base_type, &font, color::WEAK);
    }
    let mut rest = Vec::new();
    if let Some(level) = offered.item_level {
        rest.push(i18n.tag(NumberTag::ItemLevel, level));
    }
    if offered.corrupted {
        rest.push(i18n.text(Message::TagCorrupted));
    }
    for part in rest {
        append(SEPARATOR, &font, color::WEAK);
        append(&part, &font, color::WEAK);
    }
    job
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
        self.head(ui);
        ui.add_space(META_GAP);
        let galley =
            ui.painter()
                .layout_job(meta(self.offer, self.item, i18n, ui.available_width()));
        let (rect, _) =
            ui.allocate_exact_size(vec2(ui.available_width(), galley.size().y), Sense::hover());
        ui.painter().galley(rect.min, galley, color::WEAK);
        if let Some(job) = requirements(self.offer, self.item, i18n, ui.available_width()) {
            ui.add_space(META_GAP);
            let galley = ui.painter().layout_job(job);
            let (rect, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), galley.size().y), Sense::hover());
            ui.painter().galley(rect.min, galley, color::WEAK);
        }
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
            .searched_catalyst
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
        for properties in [
            waystone_lines(self.offer, self.item, i18n),
            gem_lines(self.offer, self.item, i18n),
        ] {
            if !properties.is_empty() {
                group(ui, &properties);
            }
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
mod tests;
