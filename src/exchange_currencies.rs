//! The items of the currency exchange of the trade site, the currencies an offer may be
//! priced in: their icon and their names in English and French, embedded from
//! `assets/exchange/` (taken by `tools/exchange-icons`, see `docs/images.md`), so that they
//! show without a download.

use std::collections::HashMap;
use std::sync::LazyLock;

use serde::Deserialize;

use crate::language::Language;

include!(concat!(env!("OUT_DIR"), "/exchange_icons.rs"));

/// The names and groups of the icons, one item per line.
const INDEX: &str = include_str!("../assets/exchange/currencies.ndjson");
/// The group of the waystones, whose offers are left out: a waystone isn't stackable, a price
/// in waystones pollutes the market.
const WAYSTONES: &str = "Waystones";

/// An item of the currency exchange.
#[derive(Debug, PartialEq, Eq)]
pub struct ExchangeCurrency {
    /// Its id on the trade site, e.g. `vaal`.
    pub id: String,
    /// Its group on the trade site, e.g. `Currency`, `Waystones`.
    group: String,
    english: String,
    french: String,
    /// Its icon, a PNG.
    pub icon: &'static [u8],
}

impl ExchangeCurrency {
    /// Its name in `language`, e.g. `Vaal Orb`, `Orbe vaal`.
    #[must_use]
    pub fn name(&self, language: Language) -> &str {
        match language {
            Language::English => &self.english,
            Language::French => &self.french,
        }
    }
}

/// A line of `currencies.ndjson`.
#[derive(Deserialize)]
struct Entry {
    id: String,
    group: String,
    english: String,
    french: String,
}

static CURRENCIES: LazyLock<HashMap<String, ExchangeCurrency>> = LazyLock::new(|| read(INDEX));

/// The items of `index` that have an icon, by id.
fn read(index: &str) -> HashMap<String, ExchangeCurrency> {
    index
        .lines()
        .filter_map(|line| {
            let entry: Entry =
                serde_json::from_str(line).expect("assets/exchange/currencies.ndjson is valid");
            let icon = icon(&entry.id)?;
            Some((
                entry.id.clone(),
                ExchangeCurrency {
                    id: entry.id,
                    group: entry.group,
                    english: entry.english,
                    french: entry.french,
                    icon,
                },
            ))
        })
        .collect()
}

/// The icon of the item `id`.
fn icon(id: &str) -> Option<&'static [u8]> {
    ICONS
        .binary_search_by(|(other, _)| (*other).cmp(id))
        .ok()
        .map(|index| ICONS[index].1)
}

/// Whether the application knows the item `id`, a waystone among them.
#[must_use]
pub fn known(id: &str) -> bool {
    CURRENCIES.contains_key(id)
}

/// The currency `id` of a price, when the application shows its offers: one it knows, never
/// a waystone (owner's choice on 2026-11-01, see specifications, Offers block).
#[must_use]
pub fn shown(id: &str) -> Option<&'static ExchangeCurrency> {
    CURRENCIES
        .get(id)
        .filter(|currency| currency.group != WAYSTONES)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_item_of_the_index_has_its_icon_and_every_icon_its_item() {
        assert_eq!(INDEX.lines().count(), ICONS.len());
        assert_eq!(CURRENCIES.len(), ICONS.len());
        assert!(ICONS.windows(2).all(|pair| pair[0].0 < pair[1].0));
        for (_, icon) in ICONS {
            assert!(icon.starts_with(b"\x89PNG"));
        }
    }

    #[test]
    fn the_orbs_of_the_prices_are_shown_with_their_names() {
        for (id, english, french) in [
            ("exalted", "Exalted Orb", "Orbe exalté"),
            ("divine", "Divine Orb", "Orbe divin"),
            ("chaos", "Chaos Orb", "Orbe du chaos"),
            ("vaal", "Vaal Orb", "Orbe vaal"),
        ] {
            let currency = shown(id).unwrap();
            assert_eq!(currency.name(Language::English), english);
            assert_eq!(currency.name(Language::French), french);
        }
    }

    #[test]
    fn an_id_with_an_accent_finds_its_icon() {
        // Written by build.rs from the name of its file, the path most likely to break.
        let currency = shown("legacy-of-mjölner").unwrap();
        assert!(currency.icon.starts_with(b"\x89PNG"));
        assert_eq!(currency.name(Language::English), "Legacy of Mjölner");
    }

    #[test]
    fn the_names_have_no_space_around_them() {
        for currency in CURRENCIES.values() {
            for language in Language::ALL {
                let name = currency.name(language);
                assert!(!name.is_empty() && name.trim() == name, "{name:?}");
            }
        }
    }

    #[test]
    fn a_waystone_or_an_unknown_currency_isnt_shown() {
        assert!(known("waystone-12"));
        assert!(!known("not-a-currency"));
        assert_eq!(shown("waystone-12"), None);
        assert_eq!(shown("not-a-currency"), None);
    }

    #[test]
    fn an_item_of_the_index_without_icon_is_unknown() {
        let index = concat!(
            r#"{"id":"vaal","group":"Currency","english":"Vaal Orb","french":"Orbe vaal"}"#,
            "\n",
            r#"{"id":"new-orb","group":"Currency","english":"New Orb","french":"Nouvel orbe"}"#,
        );
        let currencies = read(index);
        assert!(currencies.contains_key("vaal"));
        assert!(!currencies.contains_key("new-orb"));
    }
}
