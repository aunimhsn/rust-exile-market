//! The price of a copied item among the prices of the price database. For now, the
//! fixed-name items: the items of the currency exchange and the uniques, found by their
//! English name, or through the names of the trade site for an item copied in French.

use crate::item::{Item, Rarity};
use crate::item_names::ItemNames;
use crate::language::Language;
use crate::prices::{CurrencyPrice, PriceList, UniquePrice};

/// The price of a fixed-name item.
#[derive(Debug, Clone, PartialEq)]
pub enum FixedPrice<'a> {
    Currency(&'a CurrencyPrice),
    Unique(&'a UniquePrice),
    /// An unidentified unique whose base type has several uniques: each of them.
    Uniques(Vec<&'a UniquePrice>),
}

/// The price of `item` among `prices`; `names` find the items copied in French. `None` for
/// an item without a fixed name, or missing from the prices, or copied in French before the
/// names are known.
#[must_use]
pub fn fixed_price<'a>(
    item: &Item,
    prices: &'a PriceList,
    names: Option<&ItemNames>,
) -> Option<FixedPrice<'a>> {
    let language = item.language();
    match item.rarity() {
        Rarity::Currency | Rarity::Gem => {
            currency(item.base_type(), language, prices, names).map(FixedPrice::Currency)
        }
        Rarity::Unique => match item.name() {
            Some(name) => {
                let english = match language {
                    Language::English => name,
                    Language::French => names?.english_unique(language, name)?,
                };
                prices.unique(english).map(FixedPrice::Unique)
            }
            None => unidentified(item.base_type(), language, prices, names),
        },
        Rarity::Normal | Rarity::Magic | Rarity::Rare => None,
    }
}

/// The item of the currency exchange named `name` in `language`.
fn currency<'a>(
    name: &str,
    language: Language,
    prices: &'a PriceList,
    names: Option<&ItemNames>,
) -> Option<&'a CurrencyPrice> {
    match language {
        // The price database has the English names of the trade site.
        Language::English => prices.currency_named(name),
        Language::French => prices.currency(names?.exchange_id(language, name)?),
    }
}

/// The uniques an unidentified item of the base type `base_type` can be.
fn unidentified<'a>(
    base_type: &str,
    language: Language,
    prices: &'a PriceList,
    names: Option<&ItemNames>,
) -> Option<FixedPrice<'a>> {
    let mut uniques: Vec<&UniquePrice> = match language {
        Language::English => prices.uniques_of_base(base_type),
        Language::French => names?
            .english_uniques_of_base(language, base_type)
            .into_iter()
            .filter_map(|name| prices.unique(name))
            .collect(),
    };
    match uniques.len() {
        0 => None,
        1 => uniques.pop().map(FixedPrice::Unique),
        _ => Some(FixedPrice::Uniques(uniques)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::CopiedItem;
    use crate::item_names::tests::item_names;
    use crate::league::League;
    use crate::prices::{Price, Rates};
    use std::time::SystemTime;

    fn price(exalted: f64) -> Price {
        Price {
            exalted,
            quantity: 10,
            history: Vec::new(),
        }
    }

    fn unique(name: &str, base_type: &str, exalted: f64) -> UniquePrice {
        UniquePrice {
            name: name.to_owned(),
            base_type: base_type.to_owned(),
            price: price(exalted),
        }
    }

    fn prices() -> PriceList {
        PriceList {
            league: League::new("Forbidden Rites"),
            updated: SystemTime::UNIX_EPOCH,
            rates: Rates {
                chaos: 75.0,
                divine: 750.0,
            },
            uniques: vec![
                unique("Grand Spectrum", "Emerald", 40.0),
                unique("Prism Guardian", "Emerald", 3.0),
                unique("Mist Whisper", "Makeshift Crossbow", 1.0),
            ],
            currencies: vec![
                CurrencyPrice {
                    name: "Divine Orb".to_owned(),
                    exchange_id: "divine".to_owned(),
                    price: price(750.0),
                },
                CurrencyPrice {
                    name: "Orb of Transmutation".to_owned(),
                    exchange_id: "transmute".to_owned(),
                    price: price(0.1),
                },
            ],
        }
    }

    /// The fixed price of the item copied as `text`, with the names of the trade site or not.
    fn found(text: &str, names: Option<&ItemNames>) -> Option<String> {
        let item = Item::parse(&CopiedItem::from_text(text).expect("an item")).expect("read");
        let prices = prices();
        fixed_price(&item, &prices, names).map(|price| match price {
            FixedPrice::Currency(currency) => currency.exchange_id.clone(),
            FixedPrice::Unique(unique) => unique.name.clone(),
            FixedPrice::Uniques(uniques) => uniques
                .iter()
                .map(|unique| unique.name.as_str())
                .collect::<Vec<_>>()
                .join(" or "),
        })
    }

    #[test]
    fn an_english_item_is_found_by_its_name_without_the_names_of_the_trade_site() {
        let orb = "Item Class: Stackable Currency\nRarity: Currency\nDivine Orb";
        let unique = "Item Class: Jewels\nRarity: Unique\nGrand Spectrum\nEmerald";

        assert_eq!(found(orb, None).as_deref(), Some("divine"));
        assert_eq!(found(unique, None).as_deref(), Some("Grand Spectrum"));
    }

    #[test]
    fn a_french_item_is_found_through_the_names_of_the_trade_site() {
        let names = item_names();
        let stack = include_str!("item/test-items/fr-currency-stack.txt");
        let crossbow = include_str!("item/test-items/fr-unique-crossbow.txt");

        assert_eq!(found(stack, Some(&names)).as_deref(), Some("transmute"));
        assert_eq!(
            found(crossbow, Some(&names)).as_deref(),
            Some("Mist Whisper")
        );
        assert_eq!(found(stack, None), None, "before the names arrive");
    }

    #[test]
    fn an_unidentified_unique_is_each_unique_of_its_base_type() {
        let names = item_names();
        let english = "Item Class: Jewels\nRarity: Unique\nEmerald\n--------\nUnidentified";
        let french = "Classe d'objet: Joyaux\nRareté: Unique\nÉmeraude\n--------\nNon identifié";
        let crossbow =
            "Item Class: Crossbows\nRarity: Unique\nMakeshift Crossbow\n--------\nUnidentified";

        assert_eq!(
            found(english, None).as_deref(),
            Some("Grand Spectrum or Prism Guardian")
        );
        assert_eq!(
            found(french, Some(&names)).as_deref(),
            Some("Grand Spectrum or Prism Guardian")
        );
        assert_eq!(found(crossbow, None).as_deref(), Some("Mist Whisper"));
    }

    #[test]
    fn an_item_without_a_fixed_name_or_a_price_has_no_fixed_price() {
        let rare = "Item Class: Rings\nRarity: Rare\nWoe Loop\nRuby Ring";
        let missing = "Item Class: Stackable Currency\nRarity: Currency\nMirror of Kalandra";

        assert_eq!(found(rare, None), None);
        assert_eq!(found(missing, None), None);
    }
}
