//! The price of a copied item among the prices of the price database. For now, the
//! fixed-name items: the items of the currency exchange and the uniques, found by their
//! English name, or through the names of the trade site for an item copied in French.

use crate::item::{Item, Rarity};
use crate::item_names::{ItemNames, UniqueNames};
use crate::language::Language;
use crate::prices::{CurrencyPrice, PriceList, UniquePrice};

/// The price of a fixed-name item.
#[derive(Debug, Clone, PartialEq)]
pub enum FixedPrice<'a> {
    Currency(&'a CurrencyPrice),
    Unique(&'a UniquePrice),
    /// An unidentified unique whose base type has several uniques: each of them, priced or
    /// not.
    Uniques(Vec<Candidate<'a>>),
}

/// A unique an unidentified item can be: its names in both languages, and its price when
/// the price database has one.
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate<'a> {
    pub names: &'a UniqueNames,
    pub price: Option<&'a UniquePrice>,
}

/// Why an item has no fixed price.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoFixedPrice {
    /// A rare, magic or normal item: its price is estimated from similar offers.
    NotFixedName,
    /// The prices of the league aren't known yet.
    PricesUnknown,
    /// Missing from the prices: no offer lately, a name the prices don't know, or a
    /// category not downloaded yet.
    NotPriced,
    /// The item needs the names of the trade site, which aren't known yet: an item copied in
    /// French, or an unidentified unique.
    NamesUnknown,
}

/// The price of `item` among `prices`, none before they arrive; `names` find the items
/// copied in French, and the uniques an unidentified item can be.
pub fn fixed_price<'a>(
    item: &Item,
    prices: Option<&'a PriceList>,
    names: Option<&'a ItemNames>,
) -> Result<FixedPrice<'a>, NoFixedPrice> {
    let rarity = item.rarity();
    if matches!(rarity, Rarity::Normal | Rarity::Magic | Rarity::Rare) {
        return Err(NoFixedPrice::NotFixedName);
    }
    let prices = prices.ok_or(NoFixedPrice::PricesUnknown)?;
    let names = || names.ok_or(NoFixedPrice::NamesUnknown);
    let language = item.language();
    let name = item.base_type();
    match (rarity, item.name()) {
        // The lineage support gems are gems of the currency exchange; the other gems aren't
        // found there.
        (Rarity::Currency | Rarity::Gem, _) => {
            let currency = match language {
                // The price database has the English names of the trade site.
                Language::English => prices.currency_named(name),
                Language::French => names()?
                    .exchange_id(language, name)
                    .and_then(|id| prices.currency(id)),
            };
            currency
                .map(FixedPrice::Currency)
                .ok_or(NoFixedPrice::NotPriced)
        }
        (_, Some(unique)) => {
            let english = match language {
                Language::English => Some(unique),
                Language::French => names()?.english_unique(language, unique),
            };
            english
                .and_then(|english| prices.unique(english))
                .map(FixedPrice::Unique)
                .ok_or(NoFixedPrice::NotPriced)
        }
        (_, None) => unidentified(name, language, prices, names()?),
    }
}

/// The uniques an unidentified item of the base type `base_type` can be. They come from the
/// names of the trade site, in both languages: the price database only has the uniques
/// listed lately, and a base with two uniques, one of them priced, must not show the price
/// of the other one.
fn unidentified<'a>(
    base_type: &str,
    language: Language,
    prices: &'a PriceList,
    names: &'a ItemNames,
) -> Result<FixedPrice<'a>, NoFixedPrice> {
    let candidates: Vec<Candidate<'a>> = names
        .uniques_of_base(language, base_type)
        .into_iter()
        .map(|unique| Candidate {
            names: unique,
            price: prices.unique(&unique.english.name),
        })
        .collect();
    match candidates.as_slice() {
        [] => Err(NoFixedPrice::NotPriced),
        [only] => only
            .price
            .map(FixedPrice::Unique)
            .ok_or(NoFixedPrice::NotPriced),
        [_, _, ..] => Ok(FixedPrice::Uniques(candidates)),
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

    fn currency(name: &str, exchange_id: &str, exalted: f64) -> CurrencyPrice {
        CurrencyPrice {
            name: name.to_owned(),
            exchange_id: exchange_id.to_owned(),
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
                currency("Divine Orb", "divine", 750.0),
                currency("Orb of Transmutation", "transmute", 0.1),
                currency("Garukhan's Resolve", "garukhans-resolve", 12.0),
            ],
        }
    }

    /// The fixed price of the item copied as `text` among `prices`, with the names of the
    /// trade site or not: what it is, in the language of the item, with "?" for a unique
    /// without a price.
    fn found_in(
        prices: &PriceList,
        text: &str,
        names: Option<&ItemNames>,
    ) -> Result<String, NoFixedPrice> {
        let item = Item::parse(&CopiedItem::from_text(text).expect("an item")).expect("read");
        fixed_price(&item, Some(prices), names).map(|price| match price {
            FixedPrice::Currency(currency) => currency.exchange_id.clone(),
            FixedPrice::Unique(unique) => unique.name.clone(),
            FixedPrice::Uniques(candidates) => candidates
                .iter()
                .map(|candidate| {
                    let name = &candidate.names.of(item.language()).name;
                    match candidate.price {
                        Some(_) => name.clone(),
                        None => format!("{name}?"),
                    }
                })
                .collect::<Vec<_>>()
                .join(" or "),
        })
    }

    fn found(text: &str, names: Option<&ItemNames>) -> Result<String, NoFixedPrice> {
        found_in(&prices(), text, names)
    }

    #[expect(
        clippy::unnecessary_wraps,
        reason = "compared with what `found` returns"
    )]
    fn ok(found: &str) -> Result<String, NoFixedPrice> {
        Ok(found.to_owned())
    }

    #[test]
    fn an_english_item_is_found_by_its_name_without_the_names_of_the_trade_site() {
        let orb = "Item Class: Stackable Currency\nRarity: Currency\nDivine Orb";
        let unique = "Item Class: Jewels\nRarity: Unique\nGrand Spectrum\nEmerald";

        assert_eq!(found(orb, None), ok("divine"));
        assert_eq!(found(unique, None), ok("Grand Spectrum"));
    }

    #[test]
    fn a_french_item_is_found_through_the_names_of_the_trade_site() {
        let names = item_names();
        let stack = include_str!("item/test-items/fr-currency-stack.txt");
        let crossbow = include_str!("item/test-items/fr-unique-crossbow.txt");

        assert_eq!(found(stack, Some(&names)), ok("transmute"));
        assert_eq!(found(crossbow, Some(&names)), ok("Mist Whisper"));
        assert_eq!(found(stack, None), Err(NoFixedPrice::NamesUnknown));
    }

    #[test]
    fn an_unidentified_unique_is_each_unique_of_its_base_type() {
        let names = item_names();
        let english = "Item Class: Jewels\nRarity: Unique\nEmerald\n--------\nUnidentified";
        let french = "Classe d'objet: Joyaux\nRareté: Unique\nÉmeraude\n--------\nNon identifié";
        let crossbow =
            "Item Class: Crossbows\nRarity: Unique\nMakeshift Crossbow\n--------\nUnidentified";

        assert_eq!(
            found(english, Some(&names)),
            ok("Grand Spectrum or Prism Guardian")
        );
        // In the language of the item.
        assert_eq!(
            found(french, Some(&names)),
            ok("Grand spectre or Gardien prismatique")
        );
        assert_eq!(found(crossbow, Some(&names)), ok("Mist Whisper"));
        // The price database alone doesn't know every unique of a base.
        assert_eq!(found(english, None), Err(NoFixedPrice::NamesUnknown));
    }

    #[test]
    fn an_unidentified_unique_whose_base_has_an_unpriced_unique_isnt_priced_as_the_other() {
        let names = item_names();
        let mut prices = prices();
        prices
            .uniques
            .retain(|unique| unique.name != "Prism Guardian");
        let emerald = "Item Class: Jewels\nRarity: Unique\nEmerald\n--------\nUnidentified";

        assert_eq!(
            found_in(&prices, emerald, Some(&names)),
            ok("Grand Spectrum or Prism Guardian?")
        );
    }

    #[test]
    fn a_lineage_gem_is_found_on_the_currency_exchange_and_a_skill_gem_isnt() {
        let lineage = "Item Class: Support Gems\nRarity: Gem\nGarukhan's Resolve";
        let skill = "Item Class: Skill Gems\nRarity: Gem\nFireball";

        assert_eq!(found(lineage, None), ok("garukhans-resolve"));
        assert_eq!(found(skill, None), Err(NoFixedPrice::NotPriced));
    }

    #[test]
    fn before_the_prices_arrive_a_fixed_name_item_isnt_priced_yet() {
        let orb = "Item Class: Stackable Currency\nRarity: Currency\nDivine Orb";
        let rare = "Item Class: Rings\nRarity: Rare\nWoe Loop\nRuby Ring";
        let parse = |text| Item::parse(&CopiedItem::from_text(text).unwrap()).unwrap();

        assert_eq!(
            fixed_price(&parse(orb), None, None),
            Err(NoFixedPrice::PricesUnknown)
        );
        assert_eq!(
            fixed_price(&parse(rare), None, None),
            Err(NoFixedPrice::NotFixedName)
        );
    }

    #[test]
    fn an_item_without_a_fixed_name_or_a_price_has_no_fixed_price() {
        let rare = "Item Class: Rings\nRarity: Rare\nWoe Loop\nRuby Ring";
        let missing = "Item Class: Stackable Currency\nRarity: Currency\nMirror of Kalandra";

        assert_eq!(found(rare, None), Err(NoFixedPrice::NotFixedName));
        assert_eq!(found(missing, None), Err(NoFixedPrice::NotPriced));
    }
}
