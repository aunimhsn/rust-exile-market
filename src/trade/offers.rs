//! The offers of the trade site, read from its answers: the listed items of a search, and the
//! offers of the currency exchange.

use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, SystemTime};

use serde::Deserialize;
use serde::de::{self, Deserializer};
use serde_json::Value;

use crate::format::Currency;
use crate::item::{
    Catalyst, QualityKind, Rarity, WaystoneValue, read_quality, waystone_property, whole_percent,
    without_tier,
};
use crate::language::Language;

/// The highest quality of a catalyst on a ring, an amulet or a jewel, without a modifier
/// `+#% to Maximum Quality`.
const CATALYST_QUALITY: u32 = 20;
/// The stat of `+#% to Maximum Quality` (the Essence of the Breach, the implicit of a Breach
/// Ring), whatever its kind (`explicit.`, `implicit.`…).
const MAXIMUM_QUALITY_STAT: &str = "stat_2039822488";
/// The type of the quality among the properties of `/fetch`, whatever its kind or its language
/// (seen on 2026-10-31, as in the API of Path of Exile 1).
const QUALITY_PROPERTY: u64 = 6;

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

/// The currency of the prices of the trade site `id`; `None` for another currency.
#[must_use]
pub fn currency(id: &str) -> Option<Currency> {
    match id {
        "exalted" => Some(Currency::Exalted),
        "divine" => Some(Currency::Divine),
        "chaos" => Some(Currency::Chaos),
        _ => None,
    }
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
    pub quality: Option<OfferQuality>,
    /// The properties of a waystone, in the order of the trade site; none for another item.
    pub waystone_properties: Vec<WaystoneValue>,
    pub modifiers: Vec<OfferModifier>,
}

/// The quality of a listed item.
#[derive(Debug, Clone, PartialEq)]
pub struct OfferQuality {
    pub value: u32,
    /// The kind of the quality of a catalyst, as the trade site writes it (`Quality (Attack
    /// Modifiers)`); none for the quality of the base.
    pub kind: Option<QualityKind>,
}

impl ItemOffer {
    /// The kind of the quality of a catalyst of the item, when it is another kind than
    /// `catalyst`: its filter searches any kind. A kind the application doesn't know can't be
    /// told apart.
    #[must_use]
    pub fn other_quality(&self, catalyst: Catalyst) -> Option<Catalyst> {
        self.item
            .quality
            .as_ref()
            .and_then(|quality| quality.kind.as_ref()?.catalyst)
            .filter(|&kind| kind != catalyst)
    }

    /// Whether the item has the quality of a catalyst of another kind than `catalyst`.
    #[must_use]
    pub fn has_other_quality(&self, catalyst: Catalyst) -> bool {
        self.other_quality(catalyst).is_some()
    }
}

impl OfferItem {
    /// The highest quality of a catalyst on the item: 20 %, more with its modifiers `+#% to
    /// Maximum Quality`. A quality left beyond it when such a modifier was removed stays (seen
    /// on the trade site on 2026-10-31: amulets of 50 % without it).
    #[must_use]
    pub fn max_catalyst_quality(&self) -> u32 {
        self.modifiers
            .iter()
            .filter(|modifier| {
                modifier
                    .stat
                    .as_deref()
                    .is_some_and(|stat| stat.rsplit('.').next() == Some(MAXIMUM_QUALITY_STAT))
            })
            // `+20% to Maximum Quality`, `+20% à la Qualité maximale`.
            .filter_map(|modifier| whole_percent(modifier.text.split_whitespace().next()?))
            .fold(CATALYST_QUALITY, u32::saturating_add)
    }
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
    /// `[Quality]`, `Quality (Attack Modifiers)`…, each with its values, `[["+20%", 1]]`.
    #[serde(default)]
    properties: Vec<ApiProperty>,
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

/// A property as `/fetch` gives it: its values are pairs of a text and a display mode, only
/// the text is read.
#[derive(Debug, Deserialize)]
struct ApiProperty {
    #[serde(default)]
    name: String,
    #[serde(default)]
    values: Vec<Value>,
    /// 6 for the quality; none for most of the others. Read loosely: it only tells a quality
    /// that can't be read, which mustn't fail the whole answer.
    #[serde(rename = "type", default)]
    kind: Value,
}

impl ApiProperty {
    /// The property of a waystone, named as the game writes it in `language`:
    /// `[ItemRarity|Item Rarity]`, `+11%`; none for another property.
    fn waystone(&self, language: Language) -> Option<WaystoneValue> {
        waystone_property(language, &plain_text(&self.name), self.first_value())
    }

    /// Its name without the markup of the trade site and its first value: `Item Rarity: +11%`.
    fn line(&self) -> String {
        format!("{}: {}", plain_text(&self.name), self.first_value())
    }

    /// The text of its first value: `+11%`; empty without value.
    fn first_value(&self) -> &str {
        self.values
            .first()
            .and_then(|value| value.get(0))
            .and_then(Value::as_str)
            .unwrap_or_default()
    }

    /// The quality, read as the game writes it in `language`: `Quality (Attack Modifiers):
    /// +20%`; the line of a quality that can't be read, its value or its name, as an error;
    /// `None` for another property.
    fn quality(&self, language: Language) -> Option<Result<OfferQuality, String>> {
        let line = self.line();
        match read_quality(&line, language) {
            Some((kind, Some(value))) => Some(Ok(OfferQuality { value, kind })),
            Some((_, None)) => Some(Err(line)),
            None => self.is_quality().then_some(Err(line)),
        }
    }

    /// Whether its type is that of the quality, `6` or `"6"`.
    fn is_quality(&self) -> bool {
        let kind = self
            .kind
            .as_u64()
            .or_else(|| self.kind.as_str()?.parse().ok());
        kind == Some(QUALITY_PROPERTY)
    }
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
    /// The offers, in the order of the ids asked, from the trade site of `language`; an offer
    /// gone since the search is left out.
    pub fn offers(self, language: Language) -> Vec<ItemOffer> {
        let (offers, unreadable): (Vec<ItemOffer>, Vec<Unreadable>) = self
            .result
            .into_iter()
            .flatten()
            .map(|listed| listed.offer(language))
            .unzip();
        // Once per page, not per offer.
        let qualities: BTreeSet<String> = unreadable
            .iter()
            .filter_map(|lines| lines.quality.clone())
            .collect();
        for line in qualities {
            log::warn!("Trade site: a quality that can't be read, {line}");
        }
        let waystones: BTreeSet<&String> = unreadable
            .iter()
            .flat_map(|lines| &lines.waystone_properties)
            .collect();
        for line in waystones {
            log::warn!("Trade site: a property of a waystone that can't be read, {line}");
        }
        let unknown: BTreeSet<&str> = offers
            .iter()
            .filter_map(|offer| offer.item.quality.as_ref()?.kind.as_ref())
            .filter(|kind| kind.catalyst.is_none())
            .map(|kind| kind.words.as_str())
            .collect();
        for words in unknown {
            log::warn!("Trade site: an unknown kind of quality, {words}");
        }
        offers
    }
}

/// The lines of an offer that can't be read, to log: a name or a value the application doesn't
/// know, maybe changed by a patch.
#[derive(Debug, Default, PartialEq)]
struct Unreadable {
    quality: Option<String>,
    /// Those of a waystone, whose properties are all those of a waystone: a property shown as
    /// lacking while the offer has it under another name would read as a loss.
    waystone_properties: Vec<String>,
}

impl ApiListed {
    /// The offer, and its lines that can't be read.
    fn offer(self, language: Language) -> (ItemOffer, Unreadable) {
        let item = self.item;
        let (quality, unreadable_quality) = match item
            .properties
            .iter()
            .find_map(|property| property.quality(language))
        {
            Some(Ok(quality)) => (Some(quality), None),
            Some(Err(line)) => (None, Some(line)),
            None => (None, None),
        };
        let base_type = item.base_type.unwrap_or(item.type_line);
        let mut waystone_properties = Vec::new();
        let mut unreadable_waystone = Vec::new();
        if without_tier(language, &base_type).is_some() {
            for property in &item.properties {
                match property.waystone(language) {
                    Some(found) => waystone_properties.push(found),
                    None => unreadable_waystone.push(property.line()),
                }
            }
        }
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
        let offer = ItemOffer {
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
                base_type,
                item_level: item.ilvl,
                corrupted: item.corrupted,
                quality,
                waystone_properties,
                modifiers,
            },
        };
        let unreadable = Unreadable {
            quality: unreadable_quality,
            waystone_properties: unreadable_waystone,
        };
        (offer, unreadable)
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

        let offers = fetch.offers(Language::English);

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

    /// Two amulets of `/fetch` on 2026-10-31, shortened: a quality of a catalyst raised by
    /// the Essence of the Breach, and one left at 50 % without it; a bow with the quality of
    /// its base; and an amulet of a kind the application doesn't know.
    const QUALITIES: &str = r#"{"result": [
        {"id": "a1", "listing": {"account": {}}, "item": {
            "typeLine": "Absent Amulet", "rarity": "Rare",
            "properties": [{"name": "Quality (Caster Modifiers)", "values": [["+50%", 1]], "displayMode": 0, "type": 6}],
            "explicitMods": [{"description": "+20% to Maximum [Quality]", "hash": "stat.explicit.stat_2039822488"}]
        }},
        {"id": "a2", "listing": {"account": {}}, "item": {
            "typeLine": "Solar Amulet", "rarity": "Rare",
            "properties": [{"name": "Quality (Mana Modifiers)", "values": [["+50%", 1]], "displayMode": 0, "type": 6}],
            "explicitMods": [{"description": "+5% to [Quality] of all Skills", "hash": "stat.explicit.stat_3655769732"}]
        }},
        {"id": "b1", "listing": {"account": {}}, "item": {
            "typeLine": "Gemini Bow", "rarity": "Rare",
            "properties": [{"name": "[Bow]", "values": [], "displayMode": 0}, {"name": "[Quality]", "values": [["+20%", 1]], "displayMode": 0, "type": 6}]
        }},
        {"id": "a3", "listing": {"account": {}}, "item": {
            "typeLine": "Gold Amulet", "rarity": "Rare",
            "properties": [{"name": "Quality (Strange Modifiers)", "values": [["+20%", 1]], "displayMode": 0, "type": 6}]
        }}
    ]}"#;

    /// An amulet of `/fetch` on the French trade site on 2026-10-31, shortened.
    const FRENCH_QUALITY: &str = r#"{"result": [
        {"id": "f1", "listing": {"account": {}}, "item": {
            "typeLine": "Amulette de l'absence", "rarity": "Rare",
            "properties": [
                {"name": "Amulette", "values": [], "displayMode": 0},
                {"name": "Qualité (Modificateurs de Lanceur de sorts)", "values": [["+50%", 1]], "displayMode": 0, "type": 6}
            ],
            "explicitMods": [{"description": "+20% à la [Quality|Qualité] maximale", "hash": "stat.explicit.stat_2039822488"}]
        }}
    ]}"#;

    #[test]
    fn the_quality_of_an_offer_is_read_with_its_kind_and_its_maximum() {
        let fetch: ApiFetch = serde_json::from_str(QUALITIES).expect("read");

        let offers = fetch.offers(Language::English);

        let caster = &offers[0];
        let quality = caster.item.quality.as_ref().expect("a quality");
        assert_eq!(quality.value, 50);
        assert_eq!(
            quality.kind,
            Some(QualityKind {
                catalyst: Some(Catalyst::Caster),
                words: "Caster Modifiers".to_owned(),
            })
        );
        assert_eq!(caster.item.max_catalyst_quality(), 40, "with its essence");
        assert!(caster.has_other_quality(Catalyst::Mana));
        assert!(!caster.has_other_quality(Catalyst::Caster));
        // Left at 50 % once its modifier was removed: beyond its maximum. The quality of the
        // skills isn't the maximum quality.
        let mana = &offers[1];
        assert_eq!(
            mana.item.quality.as_ref().map(|quality| quality.value),
            Some(50)
        );
        assert_eq!(mana.item.max_catalyst_quality(), 20);
        let bow = &offers[2];
        assert_eq!(
            bow.item.quality,
            Some(OfferQuality {
                value: 20,
                kind: None
            })
        );
        assert!(
            !bow.has_other_quality(Catalyst::Mana),
            "the quality of the base"
        );
        // A kind the application doesn't know can't be told apart from any kind.
        let strange = &offers[3];
        let kind = strange
            .item
            .quality
            .as_ref()
            .and_then(|quality| quality.kind.as_ref());
        assert_eq!(
            kind.map(|kind| (kind.catalyst, kind.words.as_str())),
            Some((None, "Strange Modifiers"))
        );
        for catalyst in Catalyst::ALL {
            assert_eq!(strange.other_quality(catalyst), None, "{catalyst:?}");
        }
    }

    #[test]
    fn a_quality_that_cant_be_read_fails_neither_the_answer_nor_its_offer() {
        let property = |json: &str| -> ApiProperty { serde_json::from_str(json).expect("read") };
        let quality = |json: &str| property(json).quality(Language::English);

        // A type written as a text, and a name the application doesn't know.
        assert_eq!(
            quality(
                r#"{"name": "Kwality (Mana Modifiers)", "values": [["+20%", 1]], "type": "6"}"#
            ),
            Some(Err("Kwality (Mana Modifiers): +20%".to_owned()))
        );
        // A name read, without type, and a value that isn't a whole percentage.
        assert_eq!(
            quality(r#"{"name": "Quality (Mana Modifiers)", "values": [["+12.5%", 1]]}"#),
            Some(Err("Quality (Mana Modifiers): +12.5%".to_owned()))
        );
        // Another property, of another type or without type.
        assert_eq!(
            quality(r#"{"name": "[Amulet]", "values": [], "displayMode": 0}"#),
            None
        );
        assert_eq!(
            quality(r#"{"name": "Spirit", "values": [["100", 1]], "type": 26}"#),
            None
        );
        // The offer is read, without quality.
        let fetch: ApiFetch = serde_json::from_str(
            r#"{"result": [{"id": "k1", "listing": {"account": {}}, "item": {
                "typeLine": "Gold Amulet",
                "properties": [{"name": "Kwality (Mana Modifiers)", "values": [["+20%", 1]], "type": "6"}]
            }}]}"#,
        )
        .expect("read");
        let offers = fetch.offers(Language::English);
        assert_eq!(offers.len(), 1);
        assert_eq!(offers[0].item.quality, None);
    }

    #[test]
    fn every_maximum_quality_of_an_offer_adds_up() {
        // A Breach Ring with the Essence of the Breach: 20 % and both modifiers.
        let fetch: ApiFetch = serde_json::from_str(
            r#"{"result": [{"id": "r1", "listing": {"account": {}}, "item": {
                "typeLine": "Breach Ring", "rarity": "Rare",
                "properties": [{"name": "Quality (Attack Modifiers)", "values": [["+60%", 1]], "type": 6}],
                "implicitMods": [{"description": "+20% to Maximum [Quality]", "hash": "stat.implicit.stat_2039822488"}],
                "explicitMods": [
                    {"description": "+20% to Maximum [Quality]", "hash": "stat.explicit.stat_2039822488"},
                    {"description": "+12.5% to Maximum [Quality]", "hash": "stat.explicit.stat_2039822488"},
                    {"description": "+5% to [Quality] of all Skills", "hash": "stat.explicit.stat_3655769732"}
                ]
            }}]}"#,
        )
        .expect("read");

        let offers = fetch.offers(Language::English);

        assert_eq!(offers[0].item.max_catalyst_quality(), 60, "not a decimal");
    }

    #[test]
    fn the_quality_of_an_offer_of_the_french_trade_site_is_read_with_its_kind() {
        let fetch: ApiFetch = serde_json::from_str(FRENCH_QUALITY).expect("read");

        let offers = fetch.offers(Language::French);

        let caster = &offers[0];
        let quality = caster.item.quality.as_ref().expect("a quality");
        assert_eq!(quality.value, 50);
        assert_eq!(
            quality.kind.as_ref().and_then(|kind| kind.catalyst),
            Some(Catalyst::Caster)
        );
        assert_eq!(caster.item.max_catalyst_quality(), 40);
        assert_eq!(caster.other_quality(Catalyst::Mana), Some(Catalyst::Caster));
    }

    /// Two waystones of `/fetch` on 2026-10-31, shortened: an English one with the names seen
    /// that day, without pack size, and a French one of the French trade site; a ring, whose
    /// properties aren't those of a waystone; and a magic waystone with a property renamed and
    /// a value that can't be read.
    const WAYSTONES: &str = r#"{"result": [
        {"id": "w1", "listing": {"account": {}}, "item": {
            "typeLine": "Waystone (Tier 12)", "rarity": "Rare",
            "properties": [
                {"name": "[LimitedRespawn|Revives Available]", "values": [["0", 1]], "displayMode": 0, "type": 98},
                {"name": "[ItemRarity|Item Rarity]", "values": [["+14%", 1]], "displayMode": 0, "type": 3},
                {"name": "[MonsterRarity|Monster Rarity]", "values": [["+31%", 1]], "displayMode": 0, "type": 103},
                {"name": "[MonsterEffectiveness|Monster Effectiveness]", "values": [["+26%", 1]], "displayMode": 0, "type": 102},
                {"name": "Waystone Drop Chance", "values": [["+62%", 1]], "displayMode": 0, "type": 66}
            ]
        }},
        {"id": "w2", "listing": {"account": {}}, "item": {
            "typeLine": "Pierre de téléportation (Palier 15)", "rarity": "Rare",
            "properties": [
                {"name": "[LimitedRespawn|Résurrections disponibles]", "values": [["0", 1]], "displayMode": 0, "type": 98},
                {"name": "[ItemRarity|Rareté des objets]", "values": [["+28%", 1]], "displayMode": 0, "type": 3},
                {"name": "[Pack|Taille des Groupes]", "values": [["+7%", 1]], "displayMode": 0, "type": 4},
                {"name": "[MonsterRarity|Rareté des Monstres]", "values": [["+19%", 1]], "displayMode": 0, "type": 103},
                {"name": "[MonsterEffectiveness|Efficacité des Monstres]", "values": [["+26%", 1]], "displayMode": 0, "type": 102},
                {"name": "Chances de trouver des Pierres de téléportation", "values": [["+85%", 1]], "displayMode": 0, "type": 66}
            ]
        }},
        {"id": "r1", "listing": {"account": {}}, "item": {
            "typeLine": "Gold Ring", "rarity": "Rare",
            "properties": [{"name": "[Quality]", "values": [["+20%", 1]], "displayMode": 0, "type": 6}]
        }},
        {"id": "w3", "listing": {"account": {}}, "item": {
            "typeLine": "Burning Waystone (Tier 15)", "rarity": "Magic",
            "properties": [
                {"name": "[MonsterQuantity|Monster Quantity]", "values": [["+5%", 1]], "displayMode": 0},
                {"name": "[ItemRarity|Item Rarity]", "values": [["+1.5%", 1]], "displayMode": 0, "type": 3},
                {"name": "Waystone Drop Chance", "values": [["+20%", 1]], "displayMode": 0, "type": 66}
            ]
        }}
    ]}"#;

    #[test]
    fn the_properties_of_an_offer_of_waystone_are_read_in_both_languages() {
        use crate::item::WaystoneProperty::{
            DropChance, ItemRarity, MonsterEffectiveness, MonsterRarity, PackSize, Revives,
        };
        let fetch = || serde_json::from_str::<ApiFetch>(WAYSTONES).expect("read");
        let properties = |offer: &ItemOffer| {
            offer
                .item
                .waystone_properties
                .iter()
                .map(|found| (found.property, found.value))
                .collect::<Vec<_>>()
        };

        let english = fetch().offers(Language::English);
        assert_eq!(
            properties(&english[0]),
            [
                (Revives, 0),
                (ItemRarity, 14),
                (MonsterRarity, 31),
                (MonsterEffectiveness, 26),
                (DropChance, 62),
            ]
        );
        assert_eq!(properties(&english[2]), [], "a ring");
        let french = fetch().offers(Language::French);
        assert_eq!(
            properties(&french[1]),
            [
                (Revives, 0),
                (ItemRarity, 28),
                (PackSize, 7),
                (MonsterRarity, 19),
                (MonsterEffectiveness, 26),
                (DropChance, 85),
            ]
        );
        // The names of another language aren't read.
        assert_eq!(properties(&french[0]), [], "the English names");
        // Without base type, from the name of a magic waystone; the property renamed and the
        // value that can't be read left out.
        assert_eq!(properties(&english[3]), [(DropChance, 20)]);
    }

    #[test]
    fn a_property_of_an_offer_of_waystone_that_cant_be_read_is_kept_to_log() {
        let unreadable = |language| {
            let fetch: ApiFetch = serde_json::from_str(WAYSTONES).expect("read");
            fetch
                .result
                .into_iter()
                .flatten()
                .map(|listed| listed.offer(language).1.waystone_properties)
                .collect::<Vec<_>>()
        };

        let english = unreadable(Language::English);
        assert_eq!(english[0], Vec::<String>::new());
        // Not a waystone: its properties are those of another item.
        assert_eq!(english[2], Vec::<String>::new());
        assert_eq!(
            english[3],
            ["Monster Quantity: +5%", "Item Rarity: +1.5%"],
            "a name renamed by a patch, a value that can't be read"
        );
        // A base of another language isn't read as a waystone.
        assert_eq!(english[1], Vec::<String>::new());
        let french = unreadable(Language::French);
        assert_eq!(french[0], Vec::<String>::new());
        assert_eq!(french[1], Vec::<String>::new());
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
