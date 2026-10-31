//! The queries of the trade site: a search of items, or of the currency exchange, among the
//! sellers online or away. The links opened in the browser and the searches of the
//! application send the same queries.

use serde_json::{Map, Value, json};

use crate::filters::Property;
use crate::item::{ItemState, WaystoneProperty};

/// What a search looks for.
#[derive(Debug, Clone, PartialEq)]
pub enum Query {
    Items(ItemQuery),
    /// The offers of an item of the currency exchange, by its id (`divine`).
    Exchange(String),
}

impl Query {
    /// The kind of search the query makes.
    #[must_use]
    pub fn kind(&self) -> SearchKind {
        match self {
            Self::Items(_) => SearchKind::Items,
            Self::Exchange(_) => SearchKind::Exchange,
        }
    }

    /// Whether the query searches `property`: with a limit, since a filter without limits
    /// isn't sent.
    #[must_use]
    pub fn searches(&self, property: Property) -> bool {
        match self {
            Self::Items(query) => query
                .properties
                .iter()
                .any(|filter| filter.property == property && filter.limits_anything()),
            Self::Exchange(_) => false,
        }
    }
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
    /// Any base type when `None`.
    pub base_type: Option<String>,
    /// The id of a category of the trade site (`weapon.bow`), searched in place of the base
    /// type: a bow among all the bows.
    pub category: Option<&'static str>,
    pub rarity: Option<RarityFilter>,
    pub states: StateFilters,
    pub properties: Vec<PropertyFilter>,
    pub stats: Vec<StatFilter>,
}

/// The states of the offers (`misc_filters` of the trade site): for each, the offers of that
/// state only, or the others only; either when `None`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StateFilters {
    pub corrupted: Option<bool>,
    pub twice_corrupted: Option<bool>,
    pub sanctified: Option<bool>,
    pub fractured: Option<bool>,
    pub mirrored: Option<bool>,
}

/// The filters of the search of an item, as the player left them.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SearchFilters {
    /// The base type unchecked: the offers may have any base of its category, or any base at
    /// all for a base the game data doesn't know.
    pub any_base: bool,
    /// The states the offers must have: the state tags the player lit.
    pub states: Vec<ItemState>,
    pub properties: Vec<PropertyFilter>,
    pub stats: Vec<StatFilter>,
}

impl SearchFilters {
    /// Something other than the base type narrows the search: a stat, or a property with a
    /// limit.
    #[must_use]
    pub fn narrows(&self) -> bool {
        !self.stats.is_empty() || self.properties.iter().any(PropertyFilter::limits_anything)
    }
}

/// A property of the offers, with its minimum and its maximum.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyFilter {
    pub property: Property,
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
}

impl PropertyFilter {
    /// Whether it has a limit: without limits, it filters nothing and isn't sent.
    #[must_use]
    pub fn limits_anything(&self) -> bool {
        self.minimum.is_some() || self.maximum.is_some()
    }

    /// The group of the filters of the trade site the property is in, and its id there.
    fn id(&self) -> (&'static str, &'static str) {
        match self.property {
            Property::ItemLevel => ("type_filters", "ilvl"),
            Property::Sockets => ("equipment_filters", "rune_sockets"),
            Property::Quality => ("type_filters", "quality"),
            Property::RequiredLevel => ("req_filters", "lvl"),
            Property::PhysicalDps => ("equipment_filters", "pdps"),
            Property::ElementalDps => ("equipment_filters", "edps"),
            // The trade site names `map_rare_monsters` "Monster Rarity" and
            // `map_magic_monsters` "Monster Effectiveness" (`/data/filters`, 2026-10-31).
            Property::GemLevel => ("misc_filters", "gem_level"),
            Property::GemSockets => ("misc_filters", "gem_sockets"),
            Property::Waystone(property) => (
                "map_filters",
                match property {
                    WaystoneProperty::Revives => "map_revives",
                    WaystoneProperty::ItemRarity => "map_iir",
                    WaystoneProperty::PackSize => "map_packsize",
                    WaystoneProperty::MonsterRarity => "map_rare_monsters",
                    WaystoneProperty::MonsterEffectiveness => "map_magic_monsters",
                    WaystoneProperty::DropChance => "map_bonus",
                },
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RarityFilter {
    Unique,
    NotUnique,
}

/// A stat of the offers, by its id on the trade site (`explicit.stat_3917489142`), with its
/// minimum and its maximum.
#[derive(Debug, Clone, PartialEq)]
pub struct StatFilter {
    /// One id; or a local and a global one written the same (`# to Accuracy Rating`), the
    /// offers having either of them.
    pub ids: Vec<String>,
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
}

impl StatFilter {
    /// The filter of `id`, as the trade site reads it.
    fn to_json(&self, id: &str) -> Value {
        let value = limits(self.minimum, self.maximum);
        if value.is_empty() {
            json!({ "id": id })
        } else {
            json!({ "id": id, "value": value })
        }
    }
}

/// A minimum and a maximum, as the trade site reads them: an empty map without limits.
fn limits(minimum: Option<f64>, maximum: Option<f64>) -> Map<String, Value> {
    let mut value = Map::new();
    if let Some(minimum) = minimum {
        value.insert("min".to_owned(), json!(minimum));
    }
    if let Some(maximum) = maximum {
        value.insert("max".to_owned(), json!(maximum));
    }
    value
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
        if let Some(base_type) = &self.base_type {
            query.insert("type".to_owned(), json!(base_type));
        }
        if !self.stats.is_empty() {
            query.insert("stats".to_owned(), self.stats_json());
        }
        // Each group of filters, with its filters by id.
        let mut groups: Map<String, Value> = Map::new();
        let mut add = |group: &str, id: &str, filter: Value| {
            let filters = groups
                .entry(group)
                .or_insert_with(|| json!({ "filters": {} }));
            filters["filters"][id] = filter;
        };
        if let Some(category) = self.category {
            add("type_filters", "category", json!({ "option": category }));
        }
        if let Some(rarity) = self.rarity {
            let option = match rarity {
                RarityFilter::Unique => "unique",
                RarityFilter::NotUnique => "nonunique",
            };
            add("type_filters", "rarity", json!({ "option": option }));
        }
        for property in self
            .properties
            .iter()
            .filter(|property| property.limits_anything())
        {
            let (group, id) = property.id();
            add(
                group,
                id,
                Value::Object(limits(property.minimum, property.maximum)),
            );
        }
        let states = self.states;
        for (id, state) in [
            ("corrupted", states.corrupted),
            ("twice_corrupted", states.twice_corrupted),
            ("sanctified", states.sanctified),
            ("fractured_item", states.fractured),
            ("mirrored", states.mirrored),
        ] {
            if let Some(state) = state {
                add("misc_filters", id, json!({ "option": state.to_string() }));
            }
        }
        if !groups.is_empty() {
            query.insert("filters".to_owned(), Value::Object(groups));
        }
        json!({ "query": query, "sort": { "price": "asc" } })
    }

    /// The stat groups of the query: the stats of one id together, which the offers all have;
    /// then a `count` group of minimum 1 for each stat of several ids, which finds the offers
    /// with either of them, like Exiled Exchange 2.
    fn stats_json(&self) -> Value {
        let mut and = Vec::new();
        let mut groups = Vec::new();
        for stat in &self.stats {
            match stat.ids.as_slice() {
                [] => {}
                [id] => and.push(stat.to_json(id)),
                ids => {
                    let filters: Vec<Value> = ids.iter().map(|id| stat.to_json(id)).collect();
                    groups.push(
                        json!({ "type": "count", "value": { "min": 1 }, "filters": filters }),
                    );
                }
            }
        }
        if !and.is_empty() {
            groups.insert(0, json!({ "type": "and", "filters": and }));
        }
        Value::Array(groups)
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
    fn a_category_is_searched_among_the_type_filters() {
        let query = ItemQuery {
            name: None,
            base_type: None,
            category: Some("weapon.bow"),
            rarity: Some(RarityFilter::NotUnique),
            states: StateFilters::default(),
            properties: Vec::new(),
            stats: Vec::new(),
        };

        assert_eq!(
            query.to_json()["query"]["filters"]["type_filters"],
            json!({ "filters": {
                "category": { "option": "weapon.bow" },
                "rarity": { "option": "nonunique" },
            } })
        );
    }

    #[test]
    fn a_search_writes_only_its_filters() {
        let query = ItemQuery {
            name: None,
            base_type: Some("Ruby Ring".to_owned()),
            category: None,
            rarity: None,
            states: StateFilters::default(),
            properties: Vec::new(),
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
    fn the_corrupted_sanctified_and_mirrored_states_share_the_misc_filters() {
        let query = ItemQuery {
            name: None,
            base_type: Some("Ruby Ring".to_owned()),
            category: None,
            rarity: Some(RarityFilter::NotUnique),
            states: StateFilters {
                corrupted: Some(false),
                sanctified: Some(false),
                mirrored: Some(false),
                ..StateFilters::default()
            },
            properties: Vec::new(),
            stats: Vec::new(),
        };

        assert_eq!(
            query.to_json(),
            json!({
                "query": {
                    "status": { "option": "online" },
                    "type": "Ruby Ring",
                    "filters": {
                        "type_filters": { "filters": { "rarity": { "option": "nonunique" } } },
                        "misc_filters": { "filters": {
                            "corrupted": { "option": "false" },
                            "sanctified": { "option": "false" },
                            "mirrored": { "option": "false" }
                        } }
                    }
                },
                "sort": { "price": "asc" }
            })
        );
    }

    #[test]
    fn the_twice_corrupted_and_fractured_states_have_their_own_ids() {
        let query = ItemQuery {
            name: None,
            base_type: Some("Ruby Ring".to_owned()),
            category: None,
            rarity: None,
            states: StateFilters {
                twice_corrupted: Some(true),
                fractured: Some(true),
                ..StateFilters::default()
            },
            properties: Vec::new(),
            stats: Vec::new(),
        };

        assert_eq!(
            query.to_json()["query"]["filters"]["misc_filters"],
            json!({ "filters": {
                "twice_corrupted": { "option": "true" },
                "fractured_item": { "option": "true" }
            } })
        );
    }

    #[test]
    fn the_stats_and_the_corrupted_state_are_filters() {
        let query = ItemQuery {
            name: Some("Andvarius".to_owned()),
            base_type: Some("Gold Ring".to_owned()),
            category: None,
            rarity: Some(RarityFilter::Unique),
            states: StateFilters {
                corrupted: Some(false),
                ..StateFilters::default()
            },
            properties: Vec::new(),
            stats: vec![
                StatFilter {
                    ids: vec!["explicit.stat_3917489142".to_owned()],
                    minimum: Some(55.0),
                    maximum: None,
                },
                StatFilter {
                    ids: vec!["explicit.stat_3261801346".to_owned()],
                    minimum: None,
                    maximum: None,
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
    fn a_local_and_a_global_stat_are_searched_in_a_count_group() {
        let query = ItemQuery {
            name: Some("Bloodthirst".to_owned()),
            base_type: Some("Gold Ring".to_owned()),
            category: None,
            rarity: None,
            states: StateFilters::default(),
            properties: Vec::new(),
            stats: vec![
                StatFilter {
                    ids: vec![
                        "explicit.stat_691932474".to_owned(),
                        "explicit.stat_803737631".to_owned(),
                    ],
                    minimum: Some(150.0),
                    maximum: Some(200.0),
                },
                StatFilter {
                    ids: vec!["explicit.stat_2250533757".to_owned()],
                    minimum: None,
                    maximum: Some(-15.0),
                },
            ],
        };

        assert_eq!(
            query.to_json()["query"]["stats"],
            json!([
                { "type": "and", "filters": [
                    { "id": "explicit.stat_2250533757", "value": { "max": -15.0 } }
                ] },
                { "type": "count", "value": { "min": 1 }, "filters": [
                    { "id": "explicit.stat_691932474", "value": { "min": 150.0, "max": 200.0 } },
                    { "id": "explicit.stat_803737631", "value": { "min": 150.0, "max": 200.0 } }
                ] }
            ])
        );
    }

    #[test]
    fn a_count_group_without_limits_searches_its_ids_alone() {
        let query = ItemQuery {
            name: None,
            base_type: Some("Gold Ring".to_owned()),
            category: None,
            rarity: None,
            states: StateFilters::default(),
            properties: Vec::new(),
            stats: vec![StatFilter {
                ids: vec![
                    "explicit.stat_691932474".to_owned(),
                    "explicit.stat_803737631".to_owned(),
                ],
                minimum: None,
                maximum: None,
            }],
        };

        assert_eq!(
            query.to_json()["query"]["stats"],
            json!([{ "type": "count", "value": { "min": 1 }, "filters": [
                { "id": "explicit.stat_691932474" },
                { "id": "explicit.stat_803737631" }
            ] }])
        );
    }

    #[test]
    fn the_properties_are_filters_and_the_base_type_may_be_left_out() {
        let query = ItemQuery {
            name: None,
            base_type: None,
            category: None,
            rarity: Some(RarityFilter::NotUnique),
            states: StateFilters {
                corrupted: Some(false),
                ..StateFilters::default()
            },
            properties: vec![
                PropertyFilter {
                    property: Property::ItemLevel,
                    minimum: Some(81.0),
                    maximum: None,
                },
                PropertyFilter {
                    property: Property::Sockets,
                    minimum: Some(3.0),
                    maximum: None,
                },
                PropertyFilter {
                    property: Property::Quality,
                    minimum: Some(24.0),
                    maximum: None,
                },
                PropertyFilter {
                    property: Property::RequiredLevel,
                    minimum: None,
                    maximum: Some(40.0),
                },
                PropertyFilter {
                    property: Property::PhysicalDps,
                    minimum: Some(220.0),
                    maximum: Some(300.0),
                },
                PropertyFilter {
                    property: Property::ElementalDps,
                    minimum: None,
                    maximum: None,
                },
            ],
            stats: Vec::new(),
        };

        assert_eq!(
            query.to_json(),
            json!({
                "query": {
                    "status": { "option": "online" },
                    "filters": {
                        "type_filters": { "filters": {
                            "rarity": { "option": "nonunique" },
                            "ilvl": { "min": 81.0 },
                            "quality": { "min": 24.0 }
                        } },
                        "equipment_filters": { "filters": {
                            "rune_sockets": { "min": 3.0 },
                            "pdps": { "min": 220.0, "max": 300.0 }
                        } },
                        "req_filters": { "filters": { "lvl": { "max": 40.0 } } },
                        "misc_filters": { "filters": { "corrupted": { "option": "false" } } }
                    }
                },
                "sort": { "price": "asc" }
            })
        );
    }

    #[test]
    fn the_properties_of_a_waystone_are_searched_among_the_endgame_filters() {
        let waystone = |property, minimum| PropertyFilter {
            property: Property::Waystone(property),
            minimum: Some(minimum),
            maximum: None,
        };
        let query = ItemQuery {
            name: None,
            base_type: Some("Waystone (Tier 12)".to_owned()),
            category: None,
            rarity: Some(RarityFilter::NotUnique),
            states: StateFilters::default(),
            properties: vec![
                waystone(WaystoneProperty::Revives, 3.0),
                waystone(WaystoneProperty::ItemRarity, 9.0),
                waystone(WaystoneProperty::PackSize, 8.0),
                waystone(WaystoneProperty::MonsterRarity, 22.0),
                waystone(WaystoneProperty::MonsterEffectiveness, 11.0),
                waystone(WaystoneProperty::DropChance, 45.0),
            ],
            stats: Vec::new(),
        };

        // The ids of `/data/filters` on 2026-10-31.
        assert_eq!(
            query.to_json()["query"]["filters"]["map_filters"],
            json!({ "filters": {
                "map_revives": { "min": 3.0 },
                "map_iir": { "min": 9.0 },
                "map_packsize": { "min": 8.0 },
                "map_rare_monsters": { "min": 22.0 },
                "map_magic_monsters": { "min": 11.0 },
                "map_bonus": { "min": 45.0 }
            } })
        );
    }

    #[test]
    fn the_level_and_the_sockets_of_a_gem_share_the_misc_filters_with_its_states() {
        let filter = |property, minimum| PropertyFilter {
            property,
            minimum: Some(minimum),
            maximum: None,
        };
        let query = ItemQuery {
            name: None,
            base_type: Some("Comet".to_owned()),
            category: None,
            rarity: Some(RarityFilter::NotUnique),
            states: StateFilters {
                corrupted: Some(false),
                ..StateFilters::default()
            },
            properties: vec![
                filter(Property::GemLevel, 15.0),
                filter(Property::Quality, 16.0),
                filter(Property::GemSockets, 3.0),
            ],
            stats: Vec::new(),
        };

        // The ids of `/data/filters` on 2026-11-01.
        let filters = &query.to_json()["query"]["filters"];
        assert_eq!(
            filters["misc_filters"],
            json!({ "filters": {
                "gem_level": { "min": 15.0 },
                "gem_sockets": { "min": 3.0 },
                "corrupted": { "option": "false" }
            } })
        );
        assert_eq!(
            filters["type_filters"],
            json!({ "filters": {
                "rarity": { "option": "nonunique" },
                "quality": { "min": 16.0 }
            } })
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
