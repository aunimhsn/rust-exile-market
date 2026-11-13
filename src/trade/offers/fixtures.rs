//! The offers the tests build: an offer of an item with its price, the rest plain, which a test
//! changes with the syntax of a struct update (`ItemOffer { seller, ..item_offer(1.0, "divine") }`).

use std::time::SystemTime;

use super::{ItemOffer, OfferItem, Price, SellerStatus};
use crate::item::Requirements;

/// An offer of a plain Gold Ring for `amount` of the currency `currency`, by a seller online,
/// listed at the start of the Unix time.
pub fn item_offer(amount: f64, currency: &str) -> ItemOffer {
    ItemOffer {
        id: "a".to_owned(),
        price: Some(Price {
            amount,
            currency: currency.to_owned(),
        }),
        seller: SellerStatus::Online,
        listed: Some(SystemTime::UNIX_EPOCH),
        whisper: String::new(),
        item: OfferItem {
            rarity: None,
            name: None,
            base_type: "Gold Ring".to_owned(),
            item_level: None,
            corrupted: false,
            quality: None,
            gem_level: None,
            gem_sockets: None,
            requirements: Requirements::default(),
            waystone_properties: Vec::new(),
            waystone_unread: false,
            modifiers: Vec::new(),
        },
    }
}
