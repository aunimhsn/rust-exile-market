//! The official trade site: the links to its searches ([`link`]), its offers, searched on
//! their own thread when the player asks ([`search`]), and the names of the items in each
//! language, read on their own thread ([`names_watch`]), which find the price of an item
//! copied in French, with the templates of its stats.

mod client;
mod limits;
pub mod link;
pub mod names_watch;
pub mod offers;
pub mod pseudo;
pub mod query;
mod refusal;
pub mod search;
mod site;
pub mod stat_filters;

use crate::language::{Language, PerLanguage};
pub use refusal::Refusal;

/// The trade site of each language.
const HOSTS: PerLanguage<&str> = PerLanguage {
    english: "https://www.pathofexile.com",
    french: "https://fr.pathofexile.com",
};

/// The trade site of `language`.
fn host(language: Language) -> &'static str {
    HOSTS.of(language)
}
