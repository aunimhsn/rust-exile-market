//! The links to the trade site, opened in the browser: the search for a copied item, in the
//! league shown and on the trade site of the language of the item, sellers online.
//!
//! The browser makes the search, not the application: these links are outside the limits of
//! the trade site API.

use serde_json::json;

use super::query::{self, ItemQuery, Query, RarityFilter};
use super::site;
use crate::http;
use crate::item::{Item, Rarity};
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
    /// Another item, by its base type, until its stats become filters (step 13).
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
                // A skill gem, or an item missing from the exchange.
                None => TradeTarget::Base { base_type },
            }
        }
        Rarity::Magic => TradeTarget::Base {
            base_type: names?.magic_base_type(language, base_type)?,
        },
        Rarity::Normal | Rarity::Rare => TradeTarget::Base { base_type },
    })
}

impl TradeTarget<'_> {
    /// The search of the target, without stat filters.
    #[must_use]
    pub fn query(&self) -> Query {
        let items = |name: Option<&str>, base_type: &str, rarity| {
            Query::Items(ItemQuery {
                name: name.map(str::to_owned),
                base_type: base_type.to_owned(),
                rarity,
                corrupted: None,
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

/// The link of the search of `target` in `league`, on the trade site of `language`.
#[must_use]
pub fn url(language: Language, league: &League, target: &TradeTarget) -> String {
    let (page, query) = match target.query() {
        Query::Exchange(id) => ("exchange", json!({ "exchange": query::exchange(&id) })),
        Query::Items(items) => ("search", items.to_json()),
    };
    format!(
        "{}/trade2/{page}/poe2/{}?q={}",
        site(language),
        http::encode(league.id()),
        http::encode(&query.to_string())
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::CopiedItem;
    use crate::item_names::tests::item_names;
    use serde_json::Value;

    fn parse(text: &str) -> Item {
        Item::parse(&CopiedItem::from_text(text).expect("an item")).expect("read")
    }

    /// The query of a link, decoded.
    fn query(url: &str) -> Value {
        let (_, encoded) = url.split_once("?q=").expect("a query");
        let mut bytes = Vec::new();
        let mut rest = encoded.as_bytes();
        while let [first, tail @ ..] = rest {
            if let (b'%', [high, low, tail @ ..]) = (first, tail) {
                let hex = std::str::from_utf8(&[*high, *low]).unwrap().to_owned();
                bytes.push(u8::from_str_radix(&hex, 16).unwrap());
                rest = tail;
            } else {
                bytes.push(*first);
                rest = tail;
            }
        }
        serde_json::from_slice(&bytes).expect("JSON")
    }

    #[test]
    fn a_unique_is_searched_by_its_name_and_base_type_on_the_site_of_its_language() {
        let crossbow = parse(include_str!("../item/test-items/fr-unique-crossbow.txt"));
        let target = target(&crossbow, None, None).expect("a target");

        let url = url(
            crossbow.language(),
            &League::new("HC Forbidden Rites"),
            &target,
        );

        assert!(url.starts_with(
            "https://fr.pathofexile.com/trade2/search/poe2/HC%20Forbidden%20Rites?q="
        ));
        assert_eq!(
            query(&url),
            json!({
                "query": {
                    "status": { "option": "online" },
                    "name": "Murmure des brumes",
                    "type": "Arbalète de fortune"
                },
                "sort": { "price": "asc" }
            })
        );
    }

    #[test]
    fn an_item_of_the_currency_exchange_opens_its_exchange_page() {
        let names = item_names();
        let stack = parse(include_str!("../item/test-items/fr-currency-stack.txt"));
        let target = target(&stack, Some(&names), None).expect("a target");

        assert_eq!(target, TradeTarget::Exchange { id: "transmute" });
        let url = url(Language::French, &League::new("Standard"), &target);
        assert!(url.starts_with("https://fr.pathofexile.com/trade2/exchange/poe2/Standard?q="));
        assert_eq!(
            query(&url),
            json!({
                "exchange": {
                    "status": { "option": "online" },
                    "have": ["exalted"],
                    "want": ["transmute"]
                }
            })
        );
    }

    #[test]
    fn the_other_items_are_searched_by_their_base_type() {
        let names = item_names();
        let mace = parse(include_str!("../item/test-items/en-magic-mace.txt"));
        let rare = parse("Item Class: Rings\nRarity: Rare\nWoe Loop\nRuby Ring");
        let unidentified =
            parse("Item Class: Jewels\nRarity: Unique\nEmerald\n--------\nUnidentified");

        assert_eq!(
            target(&mace, Some(&names), None),
            Some(TradeTarget::Base {
                base_type: "Temple Maul"
            })
        );
        assert_eq!(target(&mace, None, None), None, "its base type is unknown");
        assert_eq!(
            target(&rare, None, None),
            Some(TradeTarget::Base {
                base_type: "Ruby Ring"
            })
        );
        assert_eq!(
            target(&unidentified, None, None),
            Some(TradeTarget::UnidentifiedUnique {
                base_type: "Emerald"
            })
        );
        let url = url(
            Language::English,
            &League::new("Standard"),
            &TradeTarget::Base {
                base_type: "Ruby Ring",
            },
        );
        assert_eq!(
            query(&url)["query"]["filters"]["type_filters"]["filters"]["rarity"]["option"],
            "nonunique"
        );
    }

    #[test]
    fn an_english_item_of_the_exchange_is_found_in_the_prices_without_the_names() {
        use crate::prices::{CurrencyPrice, Price, Rates};
        let prices = PriceList {
            league: League::new("Standard"),
            updated: std::time::SystemTime::UNIX_EPOCH,
            rates: Rates {
                chaos: 75.0,
                divine: 750.0,
            },
            uniques: Vec::new(),
            currencies: vec![CurrencyPrice {
                name: "Exalted Orb".to_owned(),
                exchange_id: "exalted".to_owned(),
                price: Price {
                    exalted: 1.0,
                    quantity: 1000,
                    history: Vec::new(),
                },
            }],
        };
        let orb = parse("Item Class: Stackable Currency\nRarity: Currency\nExalted Orb");

        let target = target(&orb, None, Some(&prices)).expect("a target");

        assert_eq!(target, TradeTarget::Exchange { id: "exalted" });
        // The Exalted Orb itself is paid in Divine Orbs.
        let url = url(Language::English, &League::new("Standard"), &target);
        assert_eq!(query(&url)["exchange"]["have"], json!(["divine"]));
    }
}
