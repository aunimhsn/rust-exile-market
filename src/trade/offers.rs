//! The offers of the trade site, read from its answers: the listed items of a search, and the
//! offers of the currency exchange.

use std::collections::BTreeMap;
use std::time::{Duration, SystemTime};

use serde::Deserialize;
use serde::de::{self, Deserializer};
use serde_json::Value;

use crate::item::Rarity;

/// The status of a seller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SellerStatus {
    Online,
    Away,
    Offline,
}

/// A price asked, e.g. 3 `divine`.
#[derive(Debug, Clone, PartialEq)]
pub struct Price {
    pub amount: f64,
    /// The id of the currency on the trade site (`exalted`, `divine`, `chaos`…).
    pub currency: String,
}

/// A listed item.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemOffer {
    pub id: String,
    /// `None` for an item listed without a price.
    pub price: Option<Price>,
    pub seller: SellerStatus,
    pub listed: Option<SystemTime>,
    /// The message to paste in the game chat, in the language of the trade site searched.
    pub whisper: String,
    pub item: OfferItem,
}

/// What the tooltip of an offer shows of its item.
#[derive(Debug, Clone, PartialEq)]
pub struct OfferItem {
    /// Normal, magic, rare or unique; `None` when the trade site doesn't say.
    pub rarity: Option<Rarity>,
    pub name: Option<String>,
    pub base_type: String,
    pub item_level: Option<u32>,
    pub corrupted: bool,
    pub modifiers: Vec<OfferModifier>,
}

/// A modifier line of an offer, without the markup of the trade site.
#[derive(Debug, Clone, PartialEq)]
pub struct OfferModifier {
    pub kind: ModifierKind,
    pub text: String,
    /// The stat id, e.g. `explicit.stat_3917489142`.
    pub stat: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModifierKind {
    Enchant,
    Rune,
    Implicit,
    Explicit,
}

/// An offer of the currency exchange: `get` for `pay`.
#[derive(Debug, Clone, PartialEq)]
pub struct ExchangeOffer {
    pub seller: SellerStatus,
    pub listed: Option<SystemTime>,
    pub pay: Price,
    pub get: Price,
    /// How many of the item the seller has.
    pub stock: u64,
    pub whisper: String,
}

impl ExchangeOffer {
    /// The price of one item, in the currency paid.
    #[must_use]
    pub fn unit_price(&self) -> f64 {
        self.pay.amount / self.get.amount
    }
}

/// The answer of `/search`.
#[derive(Debug, Deserialize)]
pub(super) struct ApiSearch {
    pub id: String,
    /// The ids of the offers, the cheapest first.
    pub result: Vec<String>,
    pub total: u64,
}

/// The answer of `/fetch`.
#[derive(Debug, Deserialize)]
pub(super) struct ApiFetch {
    result: Vec<Option<ApiListed>>,
}

#[derive(Debug, Deserialize)]
struct ApiListed {
    id: String,
    listing: ApiListing,
    item: ApiItem,
}

#[derive(Debug, Deserialize)]
struct ApiListing {
    indexed: Option<String>,
    price: Option<ApiPrice>,
    account: ApiAccount,
    #[serde(default)]
    whisper: String,
}

#[derive(Debug, Deserialize)]
struct ApiPrice {
    amount: f64,
    currency: String,
}

#[derive(Debug, Deserialize)]
struct ApiAccount {
    online: Option<ApiOnline>,
}

#[derive(Debug, Deserialize)]
struct ApiOnline {
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiItem {
    #[serde(default)]
    name: String,
    #[serde(default)]
    type_line: String,
    base_type: Option<String>,
    ilvl: Option<u32>,
    #[serde(default)]
    corrupted: bool,
    /// `Unique`, `Rare`…
    rarity: Option<String>,
    /// 0 normal, 1 magic, 2 rare, 3 unique, as in the API of Path of Exile 1.
    frame_type: Option<u32>,
    #[serde(default)]
    enchant_mods: Vec<ApiModifier>,
    #[serde(default)]
    rune_mods: Vec<ApiModifier>,
    #[serde(default)]
    implicit_mods: Vec<ApiModifier>,
    #[serde(default)]
    explicit_mods: Vec<ApiModifier>,
    /// Explicit modifiers too, listed apart in the API of Path of Exile 1: on 2026-10-26, the
    /// fractured items of Path of Exile 2 had their fractured lines in `explicitMods` only.
    #[serde(default)]
    fractured_mods: Vec<ApiModifier>,
    #[serde(default)]
    desecrated_mods: Vec<ApiModifier>,
}

/// A modifier as `/fetch` gives it: with its stat on 2026-10-26, a plain text in the API of
/// Path of Exile 1.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ApiModifier {
    Detailed {
        description: String,
        hash: Option<String>,
    },
    Text(String),
}

/// The answer of `/exchange`.
#[derive(Debug, Deserialize)]
pub(super) struct ApiExchange {
    #[serde(deserialize_with = "listings")]
    result: BTreeMap<String, ApiExchangeListed>,
    pub total: u64,
}

/// The listings of `/exchange` by id; an empty list without offers.
fn listings<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<BTreeMap<String, ApiExchangeListed>, D::Error> {
    match Value::deserialize(deserializer)? {
        Value::Array(listings) if listings.is_empty() => Ok(BTreeMap::new()),
        listings => serde_json::from_value(listings).map_err(de::Error::custom),
    }
}

#[derive(Debug, Deserialize)]
struct ApiExchangeListed {
    listing: ApiExchangeListing,
}

#[derive(Debug, Deserialize)]
struct ApiExchangeListing {
    indexed: Option<String>,
    account: ApiAccount,
    offers: Vec<ApiExchangeOffer>,
    /// `@seller Hi, I'd like to buy your {0} for my {1} in League`.
    whisper: String,
}

#[derive(Debug, Deserialize)]
struct ApiExchangeOffer {
    exchange: ApiExchangeSide,
    item: ApiExchangeSide,
}

#[derive(Debug, Deserialize)]
struct ApiExchangeSide {
    currency: String,
    amount: f64,
    stock: Option<u64>,
    /// `{0} Divine Orb`.
    whisper: String,
}

impl ApiFetch {
    /// The offers, in the order of the ids asked; an offer gone since the search is left
    /// out.
    pub fn offers(self) -> Vec<ItemOffer> {
        self.result
            .into_iter()
            .flatten()
            .map(ApiListed::offer)
            .collect()
    }
}

impl ApiListed {
    fn offer(self) -> ItemOffer {
        let item = self.item;
        let modifiers = [
            (ModifierKind::Enchant, item.enchant_mods),
            (ModifierKind::Rune, item.rune_mods),
            (ModifierKind::Implicit, item.implicit_mods),
            (ModifierKind::Explicit, item.fractured_mods),
            (ModifierKind::Explicit, item.explicit_mods),
            (ModifierKind::Explicit, item.desecrated_mods),
        ]
        .into_iter()
        .flat_map(|(kind, modifiers)| {
            modifiers
                .into_iter()
                .map(move |modifier| modifier.offer_modifier(kind))
        })
        .collect();
        ItemOffer {
            id: self.id,
            price: self.listing.price.map(|price| Price {
                amount: price.amount,
                currency: price.currency,
            }),
            seller: self.listing.account.status(),
            listed: self.listing.indexed.as_deref().and_then(parse_time),
            whisper: self.listing.whisper,
            item: OfferItem {
                rarity: rarity(item.rarity.as_deref(), item.frame_type),
                name: Some(item.name).filter(|name| !name.is_empty()),
                base_type: item.base_type.unwrap_or(item.type_line),
                item_level: item.ilvl,
                corrupted: item.corrupted,
                modifiers,
            },
        }
    }
}

/// The rarity of a listed item, from its name, or else from its frame.
fn rarity(name: Option<&str>, frame_type: Option<u32>) -> Option<Rarity> {
    match (name, frame_type) {
        (Some("Normal"), _) | (None, Some(0)) => Some(Rarity::Normal),
        (Some("Magic"), _) | (None, Some(1)) => Some(Rarity::Magic),
        (Some("Rare"), _) | (None, Some(2)) => Some(Rarity::Rare),
        (Some("Unique"), _) | (None, Some(3)) => Some(Rarity::Unique),
        _ => None,
    }
}

impl ApiModifier {
    fn offer_modifier(self, kind: ModifierKind) -> OfferModifier {
        let (text, stat) = match self {
            Self::Detailed { description, hash } => (description, hash),
            Self::Text(text) => (text, None),
        };
        OfferModifier {
            kind,
            text: plain_text(&text),
            // `stat.explicit.stat_3917489142`, as the queries write it.
            stat: stat.map(|hash| match hash.strip_prefix("stat.") {
                Some(id) => id.to_owned(),
                None => hash,
            }),
        }
    }
}

impl ApiAccount {
    fn status(&self) -> SellerStatus {
        match &self.online {
            None => SellerStatus::Offline,
            Some(online) if online.status.as_deref() == Some("afk") => SellerStatus::Away,
            Some(_) => SellerStatus::Online,
        }
    }
}

impl ApiExchange {
    /// The offers, the cheapest first as the trade site sorts them; a listing can hold
    /// several.
    pub fn offers(self) -> Vec<ExchangeOffer> {
        let mut offers: Vec<ExchangeOffer> = self
            .result
            .into_values()
            .flat_map(|listed| {
                let listing = listed.listing;
                let seller = listing.account.status();
                let listed = listing.indexed.as_deref().and_then(parse_time);
                let whisper = listing.whisper;
                listing.offers.into_iter().map(move |offer| ExchangeOffer {
                    seller,
                    listed,
                    whisper: whisper
                        .replacen("{0}", &side_text(&offer.item), 1)
                        .replacen("{1}", &side_text(&offer.exchange), 1),
                    stock: offer.item.stock.unwrap_or(0),
                    pay: offer.exchange.price(),
                    get: offer.item.price(),
                })
            })
            .filter(|offer| offer.get.amount > 0.0)
            .collect();
        // The listings come by id: sorted again, the cheapest first.
        offers.sort_by(|first, second| first.unit_price().total_cmp(&second.unit_price()));
        offers
    }
}

impl ApiExchangeSide {
    fn price(&self) -> Price {
        Price {
            amount: self.amount,
            currency: self.currency.clone(),
        }
    }
}

/// `1 Divine Orb`, from `{0} Divine Orb`.
fn side_text(side: &ApiExchangeSide) -> String {
    side.whisper
        .replacen("{0}", &crate::format::number(side.amount), 1)
}

/// A text without the markup of the trade site: `[ItemRarity|Rarity of Items]` becomes
/// `Rarity of Items`, and `[Resistances]` becomes `Resistances`.
fn plain_text(text: &str) -> String {
    let mut plain = String::with_capacity(text.len());
    let mut rest = text;
    while let Some((before, after)) = rest.split_once('[') {
        plain.push_str(before);
        let Some((inside, tail)) = after.split_once(']') else {
            plain.push('[');
            rest = after;
            break;
        };
        plain.push_str(inside.rsplit_once('|').map_or(inside, |(_, shown)| shown));
        rest = tail;
    }
    plain.push_str(rest);
    plain
}

/// The time of `2025-03-25T10:35:07Z` or `2026-10-23T05:30:09+00:00`, in UTC.
fn parse_time(text: &str) -> Option<SystemTime> {
    let (date, time) = text.split_once('T')?;
    let time = time
        .strip_suffix('Z')
        .or_else(|| time.strip_suffix("+00:00"))?;
    let mut date = date.split('-').map(str::parse::<i64>);
    let (Some(Ok(year)), Some(Ok(month)), Some(Ok(day)), None) =
        (date.next(), date.next(), date.next(), date.next())
    else {
        return None;
    };
    let mut time = time.split(':').map(str::parse::<u64>);
    let (Some(Ok(hours)), Some(Ok(minutes)), Some(Ok(seconds)), None) =
        (time.next(), time.next(), time.next(), time.next())
    else {
        return None;
    };
    // Outside these years, a wrong date: refused rather than computed with an overflow.
    if !(1970..=9999).contains(&year)
        || !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hours > 23
        || minutes > 59
    {
        return None;
    }
    let days = u64::try_from(days_from_civil(year, month, day)).ok()?;
    let seconds = days * 86_400 + hours * 3600 + minutes * 60 + seconds.min(60);
    SystemTime::UNIX_EPOCH.checked_add(Duration::from_secs(seconds))
}

/// The days from 1970-01-01 to a date of the Gregorian calendar (Howard Hinnant's
/// algorithm).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month_from_march = (month + 9) % 12;
    let day_of_year = (153 * month_from_march + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two offers of `/fetch` on 2026-10-26, shortened, and an offer gone since the search.
    const FETCH: &str = r#"{"result": [
        {"id": "c03d", "listing": {
            "method": "psapi", "indexed": "2025-03-25T10:35:07Z",
            "price": {"type": "~price", "amount": 1, "currency": "divine"},
            "account": {"name": "affuent-egg#9983", "online": {"league": "Forbidden Rites"}},
            "whisper": "@seller Hi, I would like to buy your Andvarius Gold Ring listed for 1 divine in Standard"
        }, "item": {
            "name": "Andvarius", "typeLine": "Gold Ring", "baseType": "Gold Ring", "ilvl": 41,
            "rarity": "Unique",
            "implicitMods": [{"description": "14% increased [ItemRarity|Rarity of Items] found", "hash": "stat.implicit.stat_3917489142"}],
            "explicitMods": [
                {"description": "-20% to all [ElementalDamage|Elemental] [Resistances]", "hash": "stat.explicit.stat_2901986750"},
                "+10 to Dexterity"
            ],
            "fracturedMods": [{"description": "+20 to Strength", "hash": "stat.explicit.stat_4080418644"}],
            "desecratedMods": [{"description": "+5% to Chaos Resistance"}]
        }},
        null,
        {"id": "f553", "listing": {
            "indexed": "2025-06-21T08:03:29Z",
            "price": {"type": "~b/o", "amount": 2.5, "currency": "exalted"},
            "account": {"online": {"league": "Standard", "status": "afk"}},
            "whisper": "@other Hi"
        }, "item": {"typeLine": "Gold Ring", "corrupted": true, "frameType": 1}}
    ]}"#;

    /// `/exchange` on 2026-10-26, shortened, with a second listing.
    const EXCHANGE: &str = r#"{"id": "H4sI", "complexity": null, "total": 2, "result": {
        "e3ab": {"id": "e3ab", "item": null, "listing": {
            "indexed": "2026-10-23T05:30:09+00:00",
            "account": {"name": "Bruce", "online": {"league": "Forbidden Rites", "status": "afk"}},
            "offers": [{
                "exchange": {"currency": "exalted", "amount": 600, "whisper": "{0} Exalted Orb"},
                "item": {"currency": "divine", "amount": 1, "stock": 10, "id": "e3ab", "whisper": "{0} Divine Orb"}
            }],
            "whisper": "@Neyge Hi, I'd like to buy your {0} for my {1} in Forbidden Rites"
        }},
        "0a1b": {"id": "0a1b", "item": null, "listing": {
            "indexed": "2026-10-24T05:30:09+00:00",
            "account": {"name": "Other", "online": null},
            "offers": [{
                "exchange": {"currency": "exalted", "amount": 1250, "whisper": "{0} Exalted Orb"},
                "item": {"currency": "divine", "amount": 2, "stock": 4, "id": "0a1b", "whisper": "{0} Divine Orb"}
            }],
            "whisper": "@Other Hi, I'd like to buy your {0} for my {1} in Forbidden Rites"
        }}
    }}"#;

    #[test]
    fn the_listed_items_are_read_in_their_order() {
        let fetch: ApiFetch = serde_json::from_str(FETCH).expect("read");

        let offers = fetch.offers();

        assert_eq!(offers.len(), 2, "the offer gone is left out");
        let ring = &offers[0];
        assert_eq!(
            ring.price,
            Some(Price {
                amount: 1.0,
                currency: "divine".to_owned()
            })
        );
        assert_eq!(ring.seller, SellerStatus::Online);
        assert_eq!(ring.listed, parse_time("2025-03-25T10:35:07Z"));
        assert_eq!(ring.item.name.as_deref(), Some("Andvarius"));
        assert_eq!(ring.item.item_level, Some(41));
        assert_eq!(
            ring.item.modifiers[0],
            OfferModifier {
                kind: ModifierKind::Implicit,
                text: "14% increased Rarity of Items found".to_owned(),
                stat: Some("implicit.stat_3917489142".to_owned()),
            }
        );
        let explicit: Vec<(ModifierKind, &str)> = ring.item.modifiers[1..]
            .iter()
            .map(|modifier| (modifier.kind, modifier.text.as_str()))
            .collect();
        assert_eq!(
            explicit,
            [
                (ModifierKind::Explicit, "+20 to Strength"),
                (ModifierKind::Explicit, "-20% to all Elemental Resistances"),
                (ModifierKind::Explicit, "+10 to Dexterity"),
                (ModifierKind::Explicit, "+5% to Chaos Resistance"),
            ],
            "the fractured and desecrated modifiers are explicit"
        );
        assert_eq!(ring.item.modifiers[3].stat, None, "a plain text");
        assert_eq!(ring.item.rarity, Some(Rarity::Unique));
        let away = &offers[1];
        assert_eq!(away.item.rarity, Some(Rarity::Magic), "from its frame");
        assert_eq!(away.seller, SellerStatus::Away);
        assert!(away.item.corrupted);
        assert_eq!(away.item.name, None);
        assert_eq!(away.item.base_type, "Gold Ring");
    }

    #[test]
    fn the_offers_of_the_exchange_are_read_the_cheapest_first() {
        let exchange: ApiExchange = serde_json::from_str(EXCHANGE).expect("read");

        let offers = exchange.offers();

        assert_eq!(offers.len(), 2);
        // 625 exalted for one divine, listed first by its id, comes second.
        assert!((offers[0].unit_price() - 600.0).abs() < f64::EPSILON);
        assert_eq!(offers[0].stock, 10);
        assert_eq!(offers[0].seller, SellerStatus::Away);
        assert_eq!(
            offers[0].whisper,
            "@Neyge Hi, I'd like to buy your 1 Divine Orb for my 600 Exalted Orb in Forbidden Rites"
        );
        assert!((offers[1].unit_price() - 625.0).abs() < f64::EPSILON);
        assert_eq!(offers[1].seller, SellerStatus::Offline);
    }

    #[test]
    fn an_exchange_without_offers_is_an_empty_list() {
        let exchange: ApiExchange =
            serde_json::from_str(r#"{"id": "H4sI", "complexity": null, "result": [], "total": 0}"#)
                .expect("read");

        assert_eq!(exchange.offers(), Vec::new());
    }

    #[test]
    fn the_markup_of_the_trade_site_is_removed() {
        assert_eq!(
            plain_text("+10 to [Dexterity|Dexterity]"),
            "+10 to Dexterity"
        );
        assert_eq!(plain_text("[Resistances] and [A|B|C]"), "Resistances and C");
        assert_eq!(plain_text("an unclosed [bracket"), "an unclosed [bracket");
    }

    #[test]
    fn the_times_of_the_trade_site_are_read_in_utc() {
        let time = |seconds| Some(SystemTime::UNIX_EPOCH + Duration::from_secs(seconds));

        assert_eq!(parse_time("1970-01-01T00:00:00Z"), time(0));
        assert_eq!(parse_time("2000-03-01T00:00:00Z"), time(951_868_800));
        assert_eq!(parse_time("2026-10-23T05:30:09+00:00"), time(1_792_733_409));
        assert_eq!(parse_time("2026-10-23T05:30:09+02:00"), None);
        assert_eq!(parse_time("2026-13-23T05:30:09Z"), None);
        assert_eq!(parse_time("yesterday"), None);
        assert_eq!(parse_time("9999999999999999-01-01T00:00:00Z"), None);
    }
}
