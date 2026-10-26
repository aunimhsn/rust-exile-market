//! The queries of the trade site: a search of items, or of the currency exchange, among the
//! sellers online or away. The links opened in the browser and the searches of the
//! application send the same queries.

use serde_json::{Map, Value, json};

/// What a search looks for.
#[derive(Debug, Clone, PartialEq)]
pub enum Query {
    Items(ItemQuery),
    /// The offers of an item of the currency exchange, by its id (`divine`).
    Exchange(String),
}

/// The kinds of searches, whose limits differ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchKind {
    Items,
    Exchange,
}

/// A search of items, the cheapest first.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemQuery {
    /// The name of a unique.
    pub name: Option<String>,
    pub base_type: String,
    pub rarity: Option<RarityFilter>,
    /// Corrupted offers only, or clean ones only; either when `None`.
    pub corrupted: Option<bool>,
    pub stats: Vec<StatFilter>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RarityFilter {
    Unique,
    NotUnique,
}

/// A stat of the offers, by its id on the trade site (`explicit.stat_3917489142`), with its
/// minimum.
#[derive(Debug, Clone, PartialEq)]
pub struct StatFilter {
    pub id: String,
    pub minimum: Option<f64>,
}

impl ItemQuery {
    /// The query as the trade site reads it, at `/search` and in the links.
    #[must_use]
    pub fn to_json(&self) -> Value {
        let mut query = Map::new();
        query.insert("status".to_owned(), online());
        if let Some(name) = &self.name {
            query.insert("name".to_owned(), json!(name));
        }
        query.insert("type".to_owned(), json!(self.base_type));
        if !self.stats.is_empty() {
            let filters: Vec<Value> = self
                .stats
                .iter()
                .map(|stat| match stat.minimum {
                    Some(minimum) => json!({ "id": stat.id, "value": { "min": minimum } }),
                    None => json!({ "id": stat.id }),
                })
                .collect();
            query.insert(
                "stats".to_owned(),
                json!([{ "type": "and", "filters": filters }]),
            );
        }
        let mut filters = Map::new();
        if let Some(rarity) = self.rarity {
            let option = match rarity {
                RarityFilter::Unique => "unique",
                RarityFilter::NotUnique => "nonunique",
            };
            filters.insert(
                "type_filters".to_owned(),
                json!({ "filters": { "rarity": { "option": option } } }),
            );
        }
        if let Some(corrupted) = self.corrupted {
            filters.insert(
                "misc_filters".to_owned(),
                json!({ "filters": { "corrupted": { "option": corrupted.to_string() } } }),
            );
        }
        if !filters.is_empty() {
            query.insert("filters".to_owned(), Value::Object(filters));
        }
        json!({ "query": query, "sort": { "price": "asc" } })
    }
}

/// The offers of the item of the currency exchange `want`, paid in Exalted Orbs, or in
/// Divine Orbs for the Exalted Orb itself, as the links write them.
#[must_use]
pub fn exchange(want: &str) -> Value {
    let have = if want == "exalted" {
        "divine"
    } else {
        "exalted"
    };
    json!({ "status": online(), "have": [have], "want": [want] })
}

/// The search of the currency exchange `want`, as `/exchange` reads it: the cheapest first.
#[must_use]
pub fn exchange_search(want: &str) -> Value {
    json!({ "query": exchange(want), "sort": { "have": "asc" }, "engine": "new" })
}

/// The sellers online or away.
fn online() -> Value {
    json!({ "option": "online" })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_search_writes_only_its_filters() {
        let query = ItemQuery {
            name: None,
            base_type: "Ruby Ring".to_owned(),
            rarity: None,
            corrupted: None,
            stats: Vec::new(),
        };

        assert_eq!(
            query.to_json(),
            json!({
                "query": { "status": { "option": "online" }, "type": "Ruby Ring" },
                "sort": { "price": "asc" }
            })
        );
    }

    #[test]
    fn the_stats_and_the_corrupted_state_are_filters() {
        let query = ItemQuery {
            name: Some("Andvarius".to_owned()),
            base_type: "Gold Ring".to_owned(),
            rarity: Some(RarityFilter::Unique),
            corrupted: Some(false),
            stats: vec![
                StatFilter {
                    id: "explicit.stat_3917489142".to_owned(),
                    minimum: Some(55.0),
                },
                StatFilter {
                    id: "explicit.stat_3261801346".to_owned(),
                    minimum: None,
                },
            ],
        };

        assert_eq!(
            query.to_json(),
            json!({
                "query": {
                    "status": { "option": "online" },
                    "name": "Andvarius",
                    "type": "Gold Ring",
                    "stats": [{ "type": "and", "filters": [
                        { "id": "explicit.stat_3917489142", "value": { "min": 55.0 } },
                        { "id": "explicit.stat_3261801346" }
                    ] }],
                    "filters": {
                        "type_filters": { "filters": { "rarity": { "option": "unique" } } },
                        "misc_filters": { "filters": { "corrupted": { "option": "false" } } }
                    }
                },
                "sort": { "price": "asc" }
            })
        );
    }

    #[test]
    fn the_exchange_is_searched_the_cheapest_first() {
        assert_eq!(
            exchange_search("divine"),
            json!({
                "query": {
                    "status": { "option": "online" },
                    "have": ["exalted"],
                    "want": ["divine"]
                },
                "sort": { "have": "asc" },
                "engine": "new"
            })
        );
        assert_eq!(exchange("exalted")["have"], json!(["divine"]));
    }
}
