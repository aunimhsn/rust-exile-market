//! The offers of the trade site, read from its answers: the listed items of a search, and the
//! offers of the currency exchange.

use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, SystemTime};

use serde::Deserialize;
use serde::de::{self, Deserializer};
use serde_json::Value;

use crate::format::Currency;
use crate::item::{
    Catalyst, ParseWarning, QualityKind, Rarity, Requirements, WaystoneValue, read_gem_level,
    read_quality, waystone_property, whole_percent, without_tier,
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
/// The type of the level of a gem among the properties of `/fetch`, `Level` (French `Niveau`),
/// seen on 2026-11-01.
const GEM_LEVEL_PROPERTY: u64 = 5;
// The types of the requirements of `/fetch`, the same in English and in French.
/// The required level, seen on 2026-11-01.
const REQUIRED_LEVEL: u64 = 62;
/// The required strength, not seen yet: between the level and the dexterity, as in the API of
/// Path of Exile 1, also told by its name (`[Strength|Str]`).
const REQUIRED_STRENGTH: u64 = 63;
/// The required dexterity, seen on 2026-11-01.
const REQUIRED_DEXTERITY: u64 = 64;
/// The required intelligence, seen on 2026-11-01.
const REQUIRED_INTELLIGENCE: u64 = 65;

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
    /// None for an item without item level: the trade site gives a gem 0.
    pub item_level: Option<u32>,
    pub corrupted: bool,
    pub quality: Option<OfferQuality>,
    /// The level of a skill gem; none for another item.
    pub gem_level: Option<u32>,
    /// The sockets of a skill gem, for its support gems; none for another item.
    pub gem_sockets: Option<u32>,
    /// The level and the attributes the item requires.
    pub requirements: Requirements,
    /// The properties of a waystone, in the order of the trade site; none for another item.
    pub waystone_properties: Vec<WaystoneValue>,
    /// Whether a property of the waystone couldn't be read: a property it seems to lack may be
    /// that one, under another name.
    pub waystone_unread: bool,
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
    /// `Level`, `[Intelligence|Int]`…, each with its value, `[["90", 0]]`.
    #[serde(default)]
    requirements: Vec<ApiProperty>,
    /// The sockets of a gem for its support gems, `["W", "W"]`; none for another item.
    gem_sockets: Option<Vec<de::IgnoredAny>>,
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
    /// 6 for the quality, 5 for the level of a gem, 62 to 65 for the requirements; none for
    /// most of the others. Read loosely: a type of another form mustn't fail the whole answer.
    #[serde(rename = "type", default)]
    kind: Value,
}

impl ApiProperty {
    /// The property of a waystone, named as the game writes it in `language`:
    /// `[ItemRarity|Item Rarity]`, `+11%`; what can't be read of another one.
    fn waystone(&self, language: Language) -> Result<WaystoneValue, UnreadProperty> {
        let name = plain_text(&self.name);
        waystone_property(language, &name, self.first_value()).map_err(|warning| match warning {
            ParseWarning::UnknownWaystoneProperty => UnreadProperty::Name(name),
            _ => UnreadProperty::Value(self.line()),
        })
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
        self.kind() == Some(QUALITY_PROPERTY)
    }

    /// Its type, a number or a text: `6`, `"6"`.
    fn kind(&self) -> Option<u64> {
        self.kind
            .as_u64()
            .or_else(|| self.kind.as_str()?.parse().ok())
    }

    /// The level of a gem: `Level`, `21`; `None` for another property.
    fn gem_level(&self) -> Option<u32> {
        (self.kind() == Some(GEM_LEVEL_PROPERTY))
            .then(|| read_gem_level(self.first_value()))
            .flatten()
    }

    /// Its first value as a whole number: `90`.
    fn number(&self) -> Option<u32> {
        self.first_value().trim().parse().ok()
    }
}

/// The level and the attributes of the `requirements` of `/fetch`, by their types, or by the
/// name of the attribute in their markup, the same in every language (`[Dexterity|Dex]`).
fn requirements(requirements: &[ApiProperty]) -> Requirements {
    let mut read = Requirements::default();
    for requirement in requirements {
        let key = requirement
            .name
            .strip_prefix('[')
            .and_then(|name| name.split(['|', ']']).next());
        let slot = match (requirement.kind(), key) {
            (Some(REQUIRED_LEVEL), _) => &mut read.level,
            (Some(REQUIRED_STRENGTH), _) | (_, Some("Strength")) => &mut read.strength,
            (Some(REQUIRED_DEXTERITY), _) | (_, Some("Dexterity")) => &mut read.dexterity,
            (Some(REQUIRED_INTELLIGENCE), _) | (_, Some("Intelligence")) => &mut read.intelligence,
            _ => continue,
        };
        *slot = requirement.number();
    }
    read
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
        for warning in warnings(&unreadable) {
            log::warn!("{warning}");
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

/// What can't be read of an offer, to log: a name or a value the application doesn't know,
/// maybe changed by a patch.
#[derive(Debug, Default, PartialEq)]
struct Unreadable {
    /// The line of its quality.
    quality: Option<String>,
    /// The properties of an offer of waystone that can't be read. Every property of a
    /// waystone listed by the trade site is one of the properties the application knows: one
    /// it doesn't read was renamed, or its value changed form.
    waystone_properties: Vec<UnreadProperty>,
}

/// A property of an offer of waystone that can't be read.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum UnreadProperty {
    /// A name the application doesn't know: `Monster Quantity`.
    Name(String),
    /// A value that can't be read, with its name: `Item Rarity: +1.5%`.
    Value(String),
}

/// The warnings of what can't be read of the offers of a page, each once: a name the
/// application doesn't know once, whatever its values.
fn warnings(unreadable: &[Unreadable]) -> Vec<String> {
    let qualities: BTreeSet<&String> = unreadable
        .iter()
        .filter_map(|lines| lines.quality.as_ref())
        .collect();
    let properties: BTreeSet<&UnreadProperty> = unreadable
        .iter()
        .flat_map(|lines| &lines.waystone_properties)
        .collect();
    qualities
        .into_iter()
        .map(|line| format!("Trade site: a quality that can't be read, {line}"))
        .chain(properties.into_iter().map(|property| match property {
            UnreadProperty::Name(name) => {
                format!("Trade site: an unknown property of a waystone, {name}")
            }
            UnreadProperty::Value(line) => {
                format!("Trade site: a property of a waystone that can't be read, {line}")
            }
        }))
        .collect()
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
        let gem_level = item.properties.iter().find_map(ApiProperty::gem_level);
        let gem_sockets = item
            .gem_sockets
            .as_ref()
            .and_then(|sockets| u32::try_from(sockets.len()).ok());
        let mut waystone_properties = Vec::new();
        let mut unreadable_waystone = Vec::new();
        if without_tier(language, &base_type).is_some() {
            for property in &item.properties {
                match property.waystone(language) {
                    Ok(found) => waystone_properties.push(found),
                    Err(unread) => unreadable_waystone.push(unread),
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
                item_level: item.ilvl.filter(|&level| level > 0),
                corrupted: item.corrupted,
                quality,
                gem_level,
                gem_sockets,
                requirements: requirements(&item.requirements),
                waystone_properties,
                waystone_unread: !unreadable_waystone.is_empty(),
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
mod tests;
