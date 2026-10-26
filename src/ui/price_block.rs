//! The price block, on band A under the item block: the price of a fixed-name item in the
//! three currencies, with its stack, its offers and its trend; the possible uniques of an
//! unidentified item; or, without a price, why.
//!
//! What the block shows is decided apart from the drawing ([`view`]), and tested.

use egui::{
    Align, Frame, Image, ImageSource, Label, Layout, Rect, RichText, Sense, Ui, Vec2, pos2, vec2,
};

use super::images;
use super::theme::{color, size, text};
use super::widgets;
use crate::format::{self, Currency};
use crate::i18n::{Message, Translator};
use crate::item::{Item, Rarity};
use crate::prices::{Direction, Price, Rates};
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
/// The space between an orb and its amount, beside the amounts and in a list.
const ORB_GAP: f32 = 7.0;
const LIST_ORB_GAP: f32 = 6.0;
/// The space above the list of the possible uniques.
const LIST_GAP: f32 = 6.0;
/// The space above and under the text of a row of the list, and its height.
const ROW_PADDING_Y: f32 = 4.0;
const ROW_HEIGHT: f32 = LIST_ORB_SIZE + 2.0 * ROW_PADDING_Y;
/// The least space between the name of a possible unique and its price.
const NAME_GAP: f32 = 12.0;
/// Fewer offers make the price of a unique uncertain.
const ENOUGH_OFFERS: u32 = 5;
/// The separator of the amounts of the unit price.
const SEPARATOR: &str = " · ";
/// The currencies of the amounts, in their order.
const CURRENCIES: [Currency; 3] = [Currency::Divine, Currency::Chaos, Currency::Exalted];
/// Under this many Divine or Chaos Orbs, an amount is too small to matter and isn't shown.
const SMALL_AMOUNT: f64 = 0.1;

/// What the price block shows.
#[derive(Debug, Clone, PartialEq)]
enum PriceView<'a> {
    /// The price of a fixed-name item; of a unique, with its offers.
    Priced {
        price: &'a Price,
        rates: Rates,
        unique: bool,
    },
    /// The uniques an unidentified item can be.
    Candidates {
        candidates: &'a [Candidate<'a>],
        rates: Rates,
    },
    /// Why there is no price.
    Notice(Notice<'a>),
}

/// Why the price block shows no price.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Notice<'a> {
    /// A rare, magic or normal item, priced from similar offers.
    NoMarketPrice(Rarity),
    /// The prices of the league are downloading; the league once it is known.
    PricesDownloading(Option<&'a str>),
    PricesUnavailable,
    FrenchNamesDownloading,
    /// Missing from the complete prices of the league.
    NoPriceFound(&'a str),
}

/// What the block shows for the fixed `price` of an item of `rarity`, given the state of the
/// `prices` of the `league` shown.
fn view<'a>(
    price: &'a Result<(FixedPrice<'a>, Rates), NoFixedPrice>,
    rarity: Rarity,
    prices: PricesState,
    league: Option<&'a str>,
) -> PriceView<'a> {
    let downloading = match prices {
        PricesState::Unavailable => Notice::PricesUnavailable,
        PricesState::Downloading { .. } | PricesState::Known { .. } => {
            Notice::PricesDownloading(league)
        }
    };
    match price {
        Ok((FixedPrice::Currency(currency), rates)) => PriceView::Priced {
            price: &currency.price,
            rates: *rates,
            unique: false,
        },
        Ok((FixedPrice::Unique(unique), rates)) => PriceView::Priced {
            price: &unique.price,
            rates: *rates,
            unique: true,
        },
        Ok((FixedPrice::Uniques(candidates), rates)) => PriceView::Candidates {
            candidates,
            rates: *rates,
        },
        Err(NoFixedPrice::NotFixedName) => PriceView::Notice(Notice::NoMarketPrice(rarity)),
        Err(NoFixedPrice::NamesUnknown) => PriceView::Notice(Notice::FrenchNamesDownloading),
        Err(NoFixedPrice::PricesUnknown) => PriceView::Notice(downloading),
        Err(NoFixedPrice::NotPriced) => PriceView::Notice(match (prices, league) {
            // Complete prices: the item isn't in them.
            (PricesState::Known { .. }, Some(league)) => Notice::NoPriceFound(league),
            // Its category may still come.
            _ => downloading,
        }),
    }
}

pub struct PriceBlock<'a> {
    item: &'a Item,
    price: Result<(FixedPrice<'a>, Rates), NoFixedPrice>,
    prices: PricesState,
    league: Option<&'a str>,
    i18n: &'a Translator,
}

impl<'a> PriceBlock<'a> {
    /// The block of `item`: its fixed `price` with the rates of its prices, or why it has
    /// none, given the state of the `prices` of the `league` shown.
    #[must_use]
    pub fn new(
        item: &'a Item,
        price: Result<(FixedPrice<'a>, Rates), NoFixedPrice>,
        prices: PricesState,
        league: Option<&'a str>,
        i18n: &'a Translator,
    ) -> Self {
        Self {
            item,
            price,
            prices,
            league,
            i18n,
        }
    }

    /// Draws the block, the whole width of `ui`.
    pub fn show(self, ui: &mut Ui) {
        let view = view(&self.price, self.item.rarity(), self.prices, self.league);
        Frame::NONE
            .fill(color::BAND_A)
            .inner_margin(vec2(size::BAND_PADDING, PADDING_Y))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                self.title_row(ui, &view);
                ui.add_space(TITLE_GAP);
                match view {
                    PriceView::Priced {
                        price,
                        rates,
                        unique,
                    } => self.priced(ui, price, rates, unique),
                    PriceView::Candidates { candidates, rates } => {
                        self.candidates(ui, candidates, rates);
                    }
                    PriceView::Notice(notice) => self.notice(ui, notice),
                }
            });
    }

    /// The title, and on its right the offers of a unique or the size of a stack.
    fn title_row(&self, ui: &mut Ui, view: &PriceView) {
        let label = widgets::spaced_line(
            ui,
            self.i18n.text(Message::PriceLabel).to_uppercase(),
            text::label(),
            text::LABEL_LETTER_SPACING,
        );
        let caption = match *view {
            PriceView::Priced {
                price,
                unique: true,
                ..
            } => Some(self.i18n.offers(price.quantity)),
            PriceView::Priced { unique: false, .. } => {
                self.stack_size().map(|size| self.i18n.stack_of(size))
            }
            PriceView::Candidates { .. } | PriceView::Notice(_) => None,
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
    /// then the unit price, the trend and the warning of a unique with few offers.
    fn priced(&self, ui: &mut Ui, price: &Price, rates: Rates, unique: bool) {
        let stack = self.stack_size();
        amounts(ui, price.exalted * f64::from(stack.unwrap_or(1)), rates);
        if stack.is_some() {
            ui.add_space(LINE_GAP);
            line(
                ui,
                &self.i18n.unit_price(&unit_amounts(price.exalted, rates)),
                color::WEAK,
            );
        }
        if let Some(trend) = price.trend() {
            let direction = trend.direction();
            let tint = match direction {
                Direction::Rise => color::RISE,
                Direction::Fall => color::FALL,
                Direction::Stable => color::WEAK,
            };
            let text = self
                .i18n
                .trend(direction, rounded_percent(trend.change.abs()), trend.days);
            ui.add_space(LINE_GAP);
            line(ui, &text, tint);
        }
        if unique && price.quantity < ENOUGH_OFFERS {
            ui.add_space(LINE_GAP);
            line(ui, &self.i18n.uncertain(price.quantity), color::WARNING);
        }
    }

    /// The uniques an unidentified item can be, each with its price in its own currency; a
    /// name too long is cut with "…", and shown whole under the pointer.
    fn candidates(&self, ui: &mut Ui, candidates: &[Candidate], rates: Rates) {
        message(ui, &self.i18n.text(Message::IdentifyToPrice), color::TEXT);
        ui.add_space(LINE_GAP);
        line(ui, &self.i18n.text(Message::PossibleUniques), color::WEAK);
        ui.add_space(LIST_GAP);
        let language = self.item.language();
        for (index, candidate) in candidates.iter().enumerate() {
            let (rect, response) =
                ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::hover());
            // The rows go to the edges of the band.
            let row = rect.expand2(vec2(size::BAND_PADDING, 0.0));
            if index % 2 == 0 {
                ui.painter().rect_filled(row, 0.0, color::ROW_VEIL);
            }
            let price_width = self.candidate_price(ui, rect, candidate, rates);
            let name = &candidate.names.of(language).name;
            let name_width = (rect.width() - price_width - NAME_GAP).max(0.0);
            let galley = widgets::truncated_line(ui, name.as_str(), text::body(), name_width);
            let cut = galley.elided;
            let top = rect.center().y - galley.size().y / 2.0;
            ui.painter()
                .galley(pos2(rect.left(), top), galley, color::UNIQUE);
            if cut {
                response.on_hover_text(name.as_str());
            }
        }
    }

    /// Draws the price of a possible unique at the right of its row; its width.
    fn candidate_price(&self, ui: &Ui, row: Rect, candidate: &Candidate, rates: Rates) -> f32 {
        let painter = ui.painter();
        let Some(unique) = candidate.price else {
            let label = self.i18n.text(Message::NoOffers);
            let galley = widgets::text_line(ui, label, text::tag());
            let size = galley.size();
            painter.galley(
                pos2(row.right() - size.x, row.center().y - size.y / 2.0),
                galley,
                color::WEAK,
            );
            return size.x;
        };
        let (amount, currency) = list_price(unique.price.exalted, rates);
        let orb_rect = Rect::from_center_size(
            pos2(row.right() - LIST_ORB_SIZE / 2.0, row.center().y),
            Vec2::splat(LIST_ORB_SIZE),
        );
        orb(currency, LIST_ORB_SIZE).paint_at(ui, orb_rect);
        let galley = widgets::text_line(ui, amount, text::list_amount());
        let size = galley.size();
        let left = orb_rect.left() - LIST_ORB_GAP - size.x;
        painter.galley(
            pos2(left, row.center().y - size.y / 2.0),
            galley,
            color::TEXT,
        );
        row.right() - left
    }

    /// Why the item has no price.
    fn notice(&self, ui: &mut Ui, notice: Notice) {
        let i18n = self.i18n;
        let first = match notice {
            Notice::NoMarketPrice(rarity) => i18n.text(match rarity {
                Rarity::Magic => Message::NoMarketPriceMagic,
                Rarity::Normal => Message::NoMarketPriceNormal,
                Rarity::Rare | Rarity::Unique | Rarity::Currency | Rarity::Gem => {
                    Message::NoMarketPriceRare
                }
            }),
            Notice::PricesDownloading(Some(league)) => i18n.prices_downloading(league),
            Notice::PricesDownloading(None) => i18n.text(Message::PricesDownloadingNoLeague),
            Notice::PricesUnavailable => i18n.text(Message::PricesUnavailable),
            Notice::FrenchNamesDownloading => i18n.text(Message::FrenchNamesDownloading),
            Notice::NoPriceFound(league) => i18n.no_price_found(league),
        };
        message(ui, &first, color::TEXT);
        if let Notice::NoMarketPrice(_) = notice {
            ui.add_space(MESSAGE_GAP);
            message(ui, &i18n.text(Message::SearchToEstimate), color::WEAK);
        }
    }
}

/// The amounts in the three currencies, side by side, each with its orb, a third of the width
/// each. An amount too small to matter is left out, and the currencies after it move to the
/// left.
fn amounts(ui: &mut Ui, exalted: f64, rates: Rates) {
    let width = ui.available_width() / 3.0;
    ui.horizontal(|ui| {
        for currency in CURRENCIES {
            let Some((amount, _)) = shown_amount(exalted, currency, rates) else {
                continue;
            };
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

/// The unit price of a stack on one line, e.g. `0.27 c · 20 ex`, without the amounts too
/// small to matter.
fn unit_amounts(exalted: f64, rates: Rates) -> String {
    CURRENCIES
        .into_iter()
        .filter_map(|currency| shown_amount(exalted, currency, rates))
        .map(|(amount, abbreviation)| format!("{amount} {abbreviation}"))
        .collect::<Vec<_>>()
        .join(SEPARATOR)
}

/// `exalted` Exalted Orbs in `currency`, written as [`in_currency`] does, when worth showing:
/// always in Exalted Orbs, the smallest currency; in Divine or Chaos Orbs, when the amount as
/// written, rounded to two decimals, reaches [`SMALL_AMOUNT`].
fn shown_amount(exalted: f64, currency: Currency, rates: Rates) -> Option<(String, &'static str)> {
    let written = (value_in(exalted, currency, rates) * 100.0).round() / 100.0;
    (currency == Currency::Exalted || written >= SMALL_AMOUNT)
        .then(|| in_currency(exalted, currency, rates))
}

/// `exalted` Exalted Orbs in `currency`, written, with the abbreviation of the currency, the
/// same in every language.
fn in_currency(exalted: f64, currency: Currency, rates: Rates) -> (String, &'static str) {
    let abbreviation = match currency {
        Currency::Divine => "div",
        Currency::Chaos => "c",
        Currency::Exalted => "ex",
    };
    (
        format::amount(value_in(exalted, currency, rates), currency),
        abbreviation,
    )
}

/// `exalted` Exalted Orbs in `currency`.
fn value_in(exalted: f64, currency: Currency, rates: Rates) -> f64 {
    match currency {
        Currency::Divine => exalted / rates.divine,
        Currency::Chaos => exalted / rates.chaos,
        Currency::Exalted => exalted,
    }
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
pub(super) fn orb(currency: Currency, size: f32) -> Image<'static> {
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
    use std::time::Duration;

    const RATES: Rates = Rates {
        chaos: 75.0,
        divine: 750.0,
    };
    const KNOWN: PricesState = PricesState::Known {
        age: Duration::ZERO,
        offline: false,
    };
    const DOWNLOADING: PricesState = PricesState::Downloading { progress: 0.5 };

    fn notice<'a>(
        price: &'a Result<(FixedPrice<'a>, Rates), NoFixedPrice>,
        prices: PricesState,
        league: Option<&'a str>,
    ) -> PriceView<'a> {
        view(price, Rarity::Currency, prices, league)
    }

    #[test]
    fn a_rare_item_has_no_market_price() {
        let price = Err(NoFixedPrice::NotFixedName);

        assert_eq!(
            view(&price, Rarity::Magic, KNOWN, Some("Standard")),
            PriceView::Notice(Notice::NoMarketPrice(Rarity::Magic))
        );
    }

    #[test]
    fn an_item_missing_from_the_complete_prices_has_no_price_found() {
        let price = Err(NoFixedPrice::NotPriced);

        assert_eq!(
            notice(&price, KNOWN, Some("Standard")),
            PriceView::Notice(Notice::NoPriceFound("Standard"))
        );
        // Its category may still come.
        assert_eq!(
            notice(&price, DOWNLOADING, Some("Standard")),
            PriceView::Notice(Notice::PricesDownloading(Some("Standard")))
        );
        assert_eq!(
            notice(&price, PricesState::Unavailable, Some("Standard")),
            PriceView::Notice(Notice::PricesUnavailable)
        );
    }

    #[test]
    fn before_the_prices_the_block_says_they_are_coming() {
        let price = Err(NoFixedPrice::PricesUnknown);

        assert_eq!(
            notice(&price, DOWNLOADING, None),
            PriceView::Notice(Notice::PricesDownloading(None))
        );
        assert_eq!(
            notice(&price, PricesState::Unavailable, None),
            PriceView::Notice(Notice::PricesUnavailable)
        );
        assert_eq!(
            notice(&Err(NoFixedPrice::NamesUnknown), KNOWN, Some("Standard")),
            PriceView::Notice(Notice::FrenchNamesDownloading)
        );
    }

    #[test]
    fn the_price_is_written_in_each_currency() {
        assert_eq!(in_currency(9000.0, Currency::Divine, RATES).0, "12");
        assert_eq!(in_currency(9000.0, Currency::Chaos, RATES).0, "120");
        assert_eq!(in_currency(9000.0, Currency::Exalted, RATES).0, "9K");
    }

    #[test]
    fn an_amount_under_a_tenth_of_divine_or_chaos_orb_is_hidden() {
        let shown =
            |exalted, currency| shown_amount(exalted, currency, RATES).map(|(amount, _)| amount);
        // 0.1 div, 1 c: all shown.
        assert_eq!(shown(75.0, Currency::Divine).as_deref(), Some("0.1"));
        // 0.08 div, 0.8 c: the Divine Orbs hidden.
        assert_eq!(shown(60.0, Currency::Divine), None);
        assert_eq!(shown(60.0, Currency::Chaos).as_deref(), Some("0.8"));
        // 0.0013 div, 0.013 c: only the Exalted Orbs.
        assert_eq!(shown(1.0, Currency::Divine), None);
        assert_eq!(shown(1.0, Currency::Chaos), None);
        assert_eq!(shown(1.0, Currency::Exalted).as_deref(), Some("1"));
        assert_eq!(shown(0.004, Currency::Exalted).as_deref(), Some("<0.01"));
    }

    #[test]
    fn the_threshold_reads_the_amount_as_written() {
        // 0.0953 div, written 0.1: shown; 0.0947 div, written 0.09: hidden.
        assert!(shown_amount(71.5, Currency::Divine, RATES).is_some());
        assert!(shown_amount(71.0, Currency::Divine, RATES).is_none());
    }

    #[test]
    fn the_unit_price_leaves_out_the_small_amounts() {
        assert_eq!(unit_amounts(20.0, RATES), "0.27 c · 20 ex");
        assert_eq!(unit_amounts(187.5, RATES), "0.25 div · 2.5 c · 188 ex");
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
