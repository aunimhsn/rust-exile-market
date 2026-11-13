//! The blocks of the item shown, one under the other, and what they are drawn from: the
//! filters of its search, its estimated price and the reference of its suspicious offers.

use egui::{OpenUrl, Ui};

use crate::filters::{Filter, FilterKey, Filters};
use crate::i18n::Translator;
use crate::item::Item;
use crate::league::League;
use crate::offers_state::{Offers, OffersState};
use crate::price::Rates;
use crate::price::estimate::{Estimate, Reference, Worth};
use crate::price::fixed::{FixedPrice, NoFixedPrice};
use crate::session::{PricesState, Session, ShownItem};
use crate::trade::query::{SearchFilters, SearchKind};
use crate::ui::{
    ItemBlock, ItemBlockAction, LineFacts, OffersAction, OffersBlock, PriceBlock, ShownLeague,
    StatChecks, Stats,
};

/// The filters of the search of `item`, the item `shown`: as the player set them, or else
/// by default.
pub(super) fn search_filters(
    session: &Session,
    shown: &ShownItem,
    item: &Item,
    filters: &Filters,
) -> SearchFilters {
    let (_, search) = every_filter(session, shown, item, filters);
    search
}

/// What the application knows of the modifier lines of the item `shown`.
fn line_facts(shown: &ShownItem) -> LineFacts<'_> {
    LineFacts {
        copied_lines: shown.lines(),
        line_stats: shown.stats(),
        tier_rolls: shown.tiers(),
    }
}

/// Every filter of `item`, the item `shown`, checked or not, as the player set them or else
/// by default, and the filters of its search they make ([`search_filters`]).
fn every_filter(
    session: &Session,
    shown: &ShownItem,
    item: &Item,
    filters: &Filters,
) -> (Vec<(FilterKey, Filter)>, SearchFilters) {
    let stats = Stats::new(item, line_facts(shown), session.filtered_base_type(item));
    let all = stats.filters(filters);
    let search = SearchFilters {
        states: filters.lit_states_of(item).collect(),
        ..shown.stats().search_filters(&all)
    };
    (all, search)
}

/// The blocks of the item shown, one under the other: the item with its filters, its price,
/// and its offers when it can be searched (`kind`).
pub(super) struct ItemView<'a> {
    pub(super) session: &'a Session,
    pub(super) shown: &'a ShownItem,
    pub(super) item: &'a Item,
    pub(super) offers: &'a Offers,
    pub(super) filters: &'a mut Filters,
    pub(super) stats_open: bool,
    pub(super) prices: PricesState,
    pub(super) kind: Option<SearchKind>,
    /// The window has the bug report, which the hint of a failed search opens: the main
    /// window only.
    pub(super) has_bug_report: bool,
    pub(super) i18n: &'a Translator,
}

impl ItemView<'_> {
    /// Draws the blocks; what the player did in the item block and in the offers block. The
    /// Check on trade pill opens its link here.
    pub(super) fn show(self, ui: &mut Ui) -> (Option<ItemBlockAction>, Option<OffersAction>) {
        let Self {
            session,
            shown,
            item,
            offers,
            filters,
            stats_open,
            prices,
            kind,
            has_bug_report,
            i18n,
        } = self;
        let fixed_price = session.fixed_price(item);
        let worth = session.worth();
        let estimate = shown_estimate(offers, item, &fixed_price, worth);
        // Without a fixed price, the estimate tells the suspicious offers.
        let reference =
            reference(&fixed_price, worth).or(estimate.map(|estimate| estimate.reference));
        let action = ItemBlock::new(item, line_facts(shown), filters, i18n)
            // The pill shows only when its link can be opened: the league and the search known.
            .tradable(session.tradable(item))
            .stats(stats_open, session.filtered_base_type(item))
            .show(ui);
        // With the filters as the player just left them.
        let (all, searched) = every_filter(session, shown, item, filters);
        let checks = StatChecks::of(&searched, shown.stats(), &all);
        if action == Some(ItemBlockAction::CheckOnTrade)
            && let Some(url) = session.trade_url(item, searched)
        {
            ui.ctx().open_url(OpenUrl::new_tab(url));
        }
        PriceBlock {
            item,
            price: fixed_price,
            estimate,
            checks,
            league: ShownLeague {
                name: session.league().map(League::name),
                prices,
            },
            i18n,
        }
        .show(ui);
        let offers_action = if let Some(kind) = kind {
            OffersBlock {
                offers,
                kind,
                reference,
                item,
                lines: shown.lines(),
                has_bug_report,
                i18n,
            }
            .show(ui)
        } else {
            if session.must_identify(item) {
                OffersBlock::identify(ui, i18n);
            }
            None
        };
        (action, offers_action)
    }
}

/// The price of one item that tells a suspicious offer: the price of a fixed-name item, with
/// the `worth` of the currencies of its prices; none for an item of the currency exchange
/// whose price stands on too few trades, which would tell honest offers. Both come from the
/// prices of the league shown: the rates beside the fixed price are those of `worth`, which
/// also prices the other currencies.
fn reference<'a>(
    fixed_price: &Result<(FixedPrice, Rates), NoFixedPrice>,
    worth: Option<Worth<'a>>,
) -> Option<Reference<'a>> {
    let exalted = match fixed_price {
        Ok((FixedPrice::Currency(currency), _)) => currency.trusted_exalted()?,
        Ok((FixedPrice::Unique(unique), _)) => unique.price.exalted,
        Ok((FixedPrice::Uniques(_), _)) | Err(_) => return None,
    };
    Some(Reference {
        exalted,
        worth: worth?,
    })
}

/// The estimated price of `item`, the item shown, without a fixed price, from the `offers` of
/// its search and the `worth` of the currencies with the prices of the league; none for a rare
/// or magic item whose search compared none of its stats, whose offers are any items of its
/// base: their price says nothing of its own.
fn shown_estimate<'a>(
    offers: &Offers,
    item: &Item,
    fixed_price: &Result<(FixedPrice, Rates), NoFixedPrice>,
    worth: Option<Worth<'a>>,
) -> Option<Estimate<'a>> {
    let Err(NoFixedPrice::NotFixedName) = fixed_price else {
        return None;
    };
    if item.is_priced_by_stats() && !offers.compared_stats() {
        return None;
    }
    let searched_catalyst = offers.searched_catalyst(item);
    let OffersState::Items(items) = offers.state() else {
        return None;
    };
    // The next offers loaded don't move it.
    Estimate::of(
        items.offers.get(..items.first_page)?,
        items.total,
        worth?,
        searched_catalyst,
    )
}

#[cfg(test)]
mod tests;
