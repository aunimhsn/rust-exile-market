//! The search for a copied item on the trade site: what it looks for, and its query, sent
//! by the application within the limits of the trade site API, or opened in the browser as a
//! link, in the league shown and on the trade site of the language of the item, sellers
//! online. The browser makes the search of a link: the links are outside those limits.

use serde_json::json;

use super::host;
use super::query::{self, ItemQuery, Query, RarityFilter, SearchFilters, StateFilters};
use crate::game_data;
use crate::http;
use crate::item::{Item, ItemState, Rarity};
use crate::item_names::ItemNames;
use crate::language::Language;
use crate::league::League;
use crate::prices::PriceList;

/// What a link searches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TradeTarget<'a> {
    /// A unique, by its name and base type.
    Unique { name: &'a str, base_type: &'a str },
    /// An unidentified unique, by its base type.
    UnidentifiedUnique { base_type: &'a str },
    /// An item of the currency exchange, on its exchange page.
    Exchange { id: &'a str },
    /// Another item, by its base type and its filters.
    Base { base_type: &'a str },
}

/// What the link of `item` searches; `names` and `prices` give the id of an item of the
/// currency exchange and the base type of a magic item. `None` for a magic item whose base
/// type isn't known.
#[must_use]
pub fn target<'a>(
    item: &'a Item,
    names: Option<&'a ItemNames>,
    prices: Option<&'a PriceList>,
) -> Option<TradeTarget<'a>> {
    let language = item.language();
    let base_type = item.base_type();
    Some(match item.rarity() {
        Rarity::Unique => match item.name() {
            Some(name) => TradeTarget::Unique { name, base_type },
            None => TradeTarget::UnidentifiedUnique { base_type },
        },
        // Never on the exchange: priced from its similar offers ([`Item::is_skill_gem`]).
        Rarity::Gem if item.is_skill_gem() => TradeTarget::Base { base_type },
        Rarity::Currency | Rarity::Gem => {
            let id = names
                .and_then(|names| names.exchange_id(language, base_type))
                .or_else(|| match language {
                    Language::English => prices
                        .and_then(|prices| prices.currency_named(base_type))
                        .map(|currency| currency.exchange_id.as_str()),
                    // The prices only know the English names: a French name needs the names
                    // of the trade site.
                    Language::French => None,
                });
            match id {
                Some(id) => TradeTarget::Exchange { id },
                // A gem without level missing from the exchange, or another item missing
                // from it.
                None => TradeTarget::Base { base_type },
            }
        }
        Rarity::Magic => TradeTarget::Base {
            base_type: names?.magic_base_type(language, base_type)?,
        },
        Rarity::Normal | Rarity::Rare => TradeTarget::Base { base_type },
    })
}

impl<'a> TradeTarget<'a> {
    /// The base type the search of another item looks for, among its filters; none for a
    /// unique, searched by its name, and an item of the currency exchange.
    #[must_use]
    pub fn base_type(&self) -> Option<&'a str> {
        match *self {
            Self::Base { base_type } => Some(base_type),
            Self::Unique { .. } | Self::UnidentifiedUnique { .. } | Self::Exchange { .. } => None,
        }
    }

    /// The search of the target, without filters.
    #[must_use]
    pub fn query(&self) -> Query {
        let items = |name: Option<&str>, base_type: &str, rarity| {
            Query::Items(ItemQuery {
                name: name.map(str::to_owned),
                base_type: Some(base_type.to_owned()),
                category: None,
                rarity,
                states: StateFilters::default(),
                properties: Vec::new(),
                stats: Vec::new(),
            })
        };
        match *self {
            TradeTarget::Exchange { id } => Query::Exchange(id.to_owned()),
            TradeTarget::Unique { name, base_type } => items(Some(name), base_type, None),
            TradeTarget::UnidentifiedUnique { base_type } => {
                items(None, base_type, Some(RarityFilter::Unique))
            }
            TradeTarget::Base { base_type } => {
                items(None, base_type, Some(RarityFilter::NotUnique))
            }
        }
    }
}

/// The search of `item` for `target`: its states as [`states`] says, a unique with the stats
/// of its `filters` the player checked, the other items with all their filters, the category
/// of their base type when the player unchecked it (a bow among all the bows); any base type
/// for a base the game data doesn't know, when another filter narrows the search (never a
/// search of every item of the league).
#[must_use]
pub fn query(item: &Item, target: &TradeTarget, filters: SearchFilters) -> Query {
    let mut query = target.query();
    if let Query::Items(items) = &mut query {
        items.states = states(item, &filters.states);
        match *target {
            TradeTarget::Unique { .. } => items.stats = filters.stats,
            TradeTarget::Base { base_type } => {
                if filters.any_base {
                    if let Some(category) = searched_category(item, base_type) {
                        items.base_type = None;
                        items.category = Some(category);
                    } else if filters.narrows() {
                        log::info!(
                            "No trade site category for the base type {base_type}: \
                             searched among any base"
                        );
                        items.base_type = None;
                    } else {
                        log::info!(
                            "No trade site category for the base type {base_type}, \
                             and nothing else narrows the search: searched by its base type"
                        );
                    }
                }
                items.properties = filters.properties;
                items.stats = filters.stats;
            }
            TradeTarget::UnidentifiedUnique { .. } | TradeTarget::Exchange { .. } => {}
        }
    }
    query
}

/// The states searched for `item`, whose state tags the player lit (`lit_tags`): a lit tag,
/// the offers of that state only; a tag left gray, the offers with or without it. A clean item
/// is compared with offers that aren't corrupted, and a clean normal, magic or rare item, which
/// can still be crafted, with offers neither sanctified nor mirrored either, like Exiled
/// Exchange 2.
fn states(item: &Item, lit_tags: &[ItemState]) -> StateFilters {
    let only = |state| (item.has_state(state) && lit_tags.contains(&state)).then_some(true);
    let crafted = craftable(item).then_some(false);
    StateFilters {
        corrupted: only(ItemState::Corrupted).or((!item.is_corrupted()).then_some(false)),
        twice_corrupted: only(ItemState::TwiceCorrupted),
        sanctified: only(ItemState::Sanctified).or(crafted),
        fractured: only(ItemState::Fractured),
        mirrored: only(ItemState::Mirrored).or(crafted),
    }
}

/// `item` is a normal, magic or rare item that can still be crafted: neither corrupted,
/// sanctified nor mirrored.
fn craftable(item: &Item) -> bool {
    matches!(item.rarity(), Rarity::Normal | Rarity::Magic | Rarity::Rare)
        && !item.is_corrupted()
        && !item.has_state(ItemState::Sanctified)
        && !item.has_state(ItemState::Mirrored)
}

/// The category of the trade site searched in place of `base_type`, the base type of `item`
/// its search looks for, when the player unchecks it (`weapon.bow`): a bow among all the bows;
/// none for a base the game data doesn't know (see [`query`] for its search).
#[must_use]
pub fn searched_category(item: &Item, base_type: &str) -> Option<&'static str> {
    game_data::trade_category(item.language(), base_type)
}

/// The link of the search `query` in `league`, on the trade site of `language`.
#[must_use]
pub fn url(language: Language, league: &League, query: &Query) -> String {
    let (page, query) = match query.clone() {
        Query::Exchange(id) => ("exchange", json!({ "exchange": query::exchange(&id) })),
        Query::Items(items) => ("search", items.to_json()),
    };
    format!(
        "{}/trade2/{page}/poe2/{}?q={}",
        host(language),
        http::encode(league.id()),
        http::encode(&query.to_string())
    )
}

#[cfg(test)]
mod tests;
