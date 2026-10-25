//! The price block, on band A under the item block: the price of a fixed-name item in the
//! three currencies, with its stack, its offers and its trend; the possible uniques of an
//! unidentified item; or, without a price, why.

use egui::{
    Align, Frame, Image, ImageSource, Label, Layout, Rect, RichText, Sense, Ui, UiBuilder, Vec2,
    vec2,
};

use super::images;
use super::theme::{color, size, text};
use super::widgets;
use crate::format::{self, Currency};
use crate::i18n::{Message, Translator, TrendDirection};
use crate::item::{Item, Rarity};
use crate::prices::{Price, Rates};
use crate::pricing::{Candidate, FixedPrice, NoFixedPrice};
use crate::session::PricesState;

/// The space above and under the block.
const PADDING_Y: f32 = 12.0;
/// The space under the title row.
const TITLE_GAP: f32 = 8.0;
/// The space above the lines under the amounts.
const LINE_GAP: f32 = 9.0;
/// The space between two messages.
const MESSAGE_GAP: f32 = 2.0;
/// The orbs beside the amounts, and in a list.
const ORB_SIZE: f32 = 24.0;
const LIST_ORB_SIZE: f32 = 18.0;
/// The space between an orb and its amount.
const ORB_GAP: f32 = 7.0;
/// The space above the list of the possible uniques, and around the text of its rows.
const LIST_GAP: f32 = 6.0;
const ROW_PADDING: Vec2 = vec2(size::BAND_PADDING, 4.0);
/// Fewer offers make the price of a unique uncertain.
const ENOUGH_OFFERS: u32 = 5;
/// The separator of the amounts of the unit price.
const SEPARATOR: &str = " · ";

pub struct PriceBlock<'a> {
    item: &'a Item,
    price: Result<FixedPrice<'a>, NoFixedPrice>,
    rates: Option<Rates>,
    prices: PricesState,
    league: &'a str,
    i18n: &'a Translator,
}

impl<'a> PriceBlock<'a> {
    /// The block of `item`: its fixed `price` with the `rates` of the league, or why it has
    /// none, given the state of the prices of `league`.
    #[must_use]
    pub fn new(
        item: &'a Item,
        price: Result<FixedPrice<'a>, NoFixedPrice>,
        rates: Option<Rates>,
        prices: PricesState,
        league: &'a str,
        i18n: &'a Translator,
    ) -> Self {
        Self {
            item,
            price,
            rates,
            prices,
            league,
            i18n,
        }
    }

    /// Draws the block, the whole width of `ui`.
    pub fn show(self, ui: &mut Ui) {
        Frame::NONE
            .fill(color::BAND_A)
            .inner_margin(vec2(size::BAND_PADDING, PADDING_Y))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                self.title_row(ui);
                ui.add_space(TITLE_GAP);
                match (&self.price, self.rates) {
                    (Ok(FixedPrice::Currency(currency)), Some(rates)) => {
                        self.priced(ui, &currency.price, rates);
                    }
                    (Ok(FixedPrice::Unique(unique)), Some(rates)) => {
                        self.priced(ui, &unique.price, rates);
                    }
                    (Ok(FixedPrice::Uniques(candidates)), Some(rates)) => {
                        self.candidates(ui, candidates, rates);
                    }
                    (Ok(_), None) | (Err(_), _) => self.messages(ui),
                }
            });
    }

    /// The title, and on its right the offers of a unique or the size of a stack.
    fn title_row(&self, ui: &mut Ui) {
        let label = widgets::spaced_line(
            ui,
            self.i18n.text(Message::PriceLabel).to_uppercase(),
            text::label(),
            text::LABEL_LETTER_SPACING,
        );
        let caption = match &self.price {
            Ok(FixedPrice::Unique(unique)) => Some(self.i18n.offers(unique.price.quantity)),
            Ok(FixedPrice::Currency(_)) => self.stack_size().map(|size| self.i18n.stack_of(size)),
            _ => None,
        };
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(label.size(), Sense::hover());
            ui.painter().galley(rect.min, label, color::WEAK);
            if let Some(caption) = caption {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new(caption).font(text::tag()).color(color::WEAK));
                });
            }
        });
    }

    /// The size of a copied stack of more than one item.
    fn stack_size(&self) -> Option<u32> {
        self.item
            .stack()
            .map(|stack| stack.size)
            .filter(|&size| size > 1)
    }

    /// The price in the three currencies, the value of the whole stack when there is one,
    /// then the unit price, the trend and the warning of a price with few offers.
    fn priced(&self, ui: &mut Ui, price: &Price, rates: Rates) {
        let stack = self.stack_size();
        let total = price.exalted * f64::from(stack.unwrap_or(1));
        amounts(ui, total, rates);
        if stack.is_some() {
            let unit = [Currency::Divine, Currency::Chaos, Currency::Exalted]
                .map(|currency| {
                    let (amount, abbreviation) = in_currency(price.exalted, currency, rates);
                    format!("{amount} {abbreviation}")
                })
                .join(SEPARATOR);
            ui.add_space(LINE_GAP);
            line(ui, &self.i18n.unit_price(&unit), color::WEAK);
        }
        if let Some(trend) = price.trend() {
            let percent = rounded_percent(trend.change.abs());
            let days = u32::try_from(trend.days).unwrap_or(u32::MAX);
            let (direction, tint) = match trend.change {
                _ if trend.is_stable() => (TrendDirection::Stable, color::WEAK),
                change if change > 0.0 => (TrendDirection::Rise, color::RISE),
                _ => (TrendDirection::Fall, color::FALL),
            };
            ui.add_space(LINE_GAP);
            line(ui, &self.i18n.trend(direction, percent, days), tint);
        }
        let unique = matches!(self.price, Ok(FixedPrice::Unique(_)));
        if unique && price.quantity < ENOUGH_OFFERS {
            ui.add_space(LINE_GAP);
            line(ui, &self.i18n.uncertain(price.quantity), color::WARNING);
        }
    }

    /// The uniques an unidentified item can be, each with its price in its own currency.
    fn candidates(&self, ui: &mut Ui, candidates: &[Candidate], rates: Rates) {
        message(ui, &self.i18n.text(Message::IdentifyToPrice), color::TEXT);
        ui.add_space(LINE_GAP);
        line(ui, &self.i18n.text(Message::PossibleUniques), color::WEAK);
        ui.add_space(LIST_GAP);
        let language = self.item.language();
        for (index, candidate) in candidates.iter().enumerate() {
            let height = LIST_ORB_SIZE + 2.0 * ROW_PADDING.y;
            // The rows go to the edges of the band.
            let (row, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
            let row = row.expand2(vec2(size::BAND_PADDING, 0.0));
            if index % 2 == 0 {
                ui.painter().rect_filled(row, 0.0, color::ROW_VEIL);
            }
            let inner = row.shrink2(ROW_PADDING);
            let name = &candidate.names.of(language).name;
            let left = Rect::from_min_size(inner.min, vec2(inner.width() * 0.6, inner.height()));
            ui.scope_builder(
                UiBuilder::new()
                    .max_rect(left)
                    .layout(Layout::left_to_right(Align::Center)),
                |ui| {
                    ui.add(
                        Label::new(RichText::new(name).font(text::body()).color(color::UNIQUE))
                            .truncate(),
                    );
                },
            );
            let right = Rect::from_min_max(inner.center_top(), inner.max);
            ui.scope_builder(UiBuilder::new().max_rect(right), |ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = ORB_GAP - 1.0;
                    match candidate.price {
                        Some(unique) => {
                            let (amount, currency) = list_price(unique.price.exalted, rates);
                            ui.add(orb(currency, LIST_ORB_SIZE));
                            ui.label(
                                RichText::new(amount)
                                    .font(text::list_amount())
                                    .color(color::TEXT),
                            );
                        }
                        None => {
                            ui.label(
                                RichText::new(self.i18n.text(Message::NoOffers))
                                    .font(text::tag())
                                    .color(color::WEAK),
                            );
                        }
                    }
                });
            });
        }
    }

    /// Why the item has no price: a rare item is estimated from offers, a fixed-name item
    /// waits for its prices or isn't in them.
    fn messages(&self, ui: &mut Ui) {
        let i18n = self.i18n;
        let lines: Vec<String> = match self.price {
            Err(NoFixedPrice::NotFixedName) => {
                let first = match self.item.rarity() {
                    Rarity::Magic => Message::NoMarketPriceMagic,
                    Rarity::Normal => Message::NoMarketPriceNormal,
                    _ => Message::NoMarketPriceRare,
                };
                vec![i18n.text(first), i18n.text(Message::SearchToEstimate)]
            }
            Err(NoFixedPrice::NamesUnknown) => vec![i18n.text(Message::FrenchNamesDownloading)],
            Ok(_) | Err(NoFixedPrice::NotPriced) => vec![match self.prices {
                PricesState::Downloading { .. } => i18n.prices_downloading(self.league),
                PricesState::Unavailable => i18n.text(Message::PricesUnavailable),
                PricesState::Known { .. } => i18n.no_price_found(self.league),
            }],
        };
        for (index, text) in lines.iter().enumerate() {
            if index > 0 {
                ui.add_space(MESSAGE_GAP);
            }
            let tint = if index == 0 { color::TEXT } else { color::WEAK };
            message(ui, text, tint);
        }
    }
}

/// The amounts in the three currencies, side by side, each with its orb.
fn amounts(ui: &mut Ui, exalted: f64, rates: Rates) {
    let width = ui.available_width() / 3.0;
    ui.horizontal(|ui| {
        for currency in [Currency::Divine, Currency::Chaos, Currency::Exalted] {
            let (amount, _) = in_currency(exalted, currency, rates);
            ui.allocate_ui_with_layout(
                vec2(width, ORB_SIZE),
                Layout::left_to_right(Align::Center),
                |ui| {
                    // A third of the width each, whatever the length of the amount.
                    ui.set_min_width(width);
                    ui.spacing_mut().item_spacing.x = ORB_GAP;
                    ui.add(orb(currency, ORB_SIZE));
                    ui.label(RichText::new(amount).font(text::price()).color(color::TEXT));
                },
            );
        }
    });
}

/// `exalted` Exalted Orbs in `currency`, written, with the abbreviation of the currency.
fn in_currency(exalted: f64, currency: Currency, rates: Rates) -> (String, &'static str) {
    let (value, abbreviation) = match currency {
        Currency::Divine => (exalted / rates.divine, "div"),
        Currency::Chaos => (exalted / rates.chaos, "c"),
        Currency::Exalted => (exalted, "ex"),
    };
    (format::amount(value, currency), abbreviation)
}

/// The price of a row of a list, in one currency: Divine Orbs from one Divine Orb, Exalted
/// Orbs under it.
fn list_price(exalted: f64, rates: Rates) -> (String, Currency) {
    let currency = if exalted >= rates.divine {
        Currency::Divine
    } else {
        Currency::Exalted
    };
    (in_currency(exalted, currency, rates).0, currency)
}

/// The orb of `currency`, `size` across.
fn orb(currency: Currency, size: f32) -> Image<'static> {
    let source: ImageSource<'static> = match currency {
        Currency::Divine => images::divine_orb(),
        Currency::Chaos => images::chaos_orb(),
        Currency::Exalted => images::exalted_orb(),
    };
    Image::new(source).fit_to_exact_size(Vec2::splat(size))
}

/// A change in percent, rounded, e.g. 4 for `0.04`.
fn rounded_percent(change: f64) -> u32 {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a positive change, capped"
    )]
    let percent = (change * 100.0).round().clamp(0.0, 1e6) as u32;
    percent
}

/// A line under the amounts, in `tint`.
fn line(ui: &mut Ui, text: &str, tint: egui::Color32) {
    ui.add(Label::new(RichText::new(text).font(text::item_line()).color(tint)).wrap());
}

/// A message in place of the price, in `tint`.
fn message(ui: &mut Ui, text: &str, tint: egui::Color32) {
    ui.add(Label::new(RichText::new(text).font(text::message()).color(tint)).wrap());
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATES: Rates = Rates {
        chaos: 75.0,
        divine: 750.0,
    };

    #[test]
    fn the_price_is_written_in_each_currency() {
        assert_eq!(in_currency(9000.0, Currency::Divine, RATES).0, "12");
        assert_eq!(in_currency(9000.0, Currency::Chaos, RATES).0, "120");
        assert_eq!(in_currency(9000.0, Currency::Exalted, RATES).0, "9K");
    }

    #[test]
    fn a_row_of_a_list_is_in_divine_orbs_from_one_divine_orb() {
        assert_eq!(
            list_price(3000.0, RATES),
            ("4".to_owned(), Currency::Divine)
        );
        assert_eq!(
            list_price(640.0, RATES),
            ("640".to_owned(), Currency::Exalted)
        );
    }

    #[test]
    fn a_change_is_rounded_to_a_whole_percent() {
        assert_eq!(rounded_percent(0.044), 4);
        assert_eq!(rounded_percent(0.106), 11);
    }
}
