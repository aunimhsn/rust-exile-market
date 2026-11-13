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

    /// Whether the query compares the stats of the item, more than its base: see
    /// [`SearchFilters::compares_stats`].
    #[must_use]
    pub fn compares_stats(&self) -> bool {
        match self {
            Self::Items(query) => compares_stats(&query.stats, &query.properties),
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
    pub base: SearchedBase,
    pub rarity: Option<RarityFilter>,
    pub states: StateFilters,
    pub properties: Vec<PropertyFilter>,
    pub stats: Vec<StatFilter>,
}

/// The base of the items searched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchedBase {
    /// One base type, e.g. `Gold Ring`.
    Type(String),
    /// The id of a category of the trade site (`weapon.bow`), searched in place of the base
    /// type: a bow among all the bows.
    Category(&'static str),
    /// Any base.
    Any,
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
    /// Something other than the base type narrows the search: a stat that rules out items of
    /// the league (not the count of its unrevealed modifiers, which any item may have), or a
    /// property with a limit.
    #[must_use]
    pub fn narrows(&self) -> bool {
        self.stats
            .iter()
            .any(|stat| stat.rules_out != RulesOut::Nothing)
            || self.properties.iter().any(PropertyFilter::limits_anything)
    }

    /// Something compares the stats of the item with the offers: a filter that rules out items
    /// of its base ([`RulesOut::OfTheBase`]), a property only with a limit. Without, the offers
    /// are any items of its base, whose price says nothing of its own.
    #[must_use]
    pub fn compares_stats(&self) -> bool {
        compares_stats(&self.stats, &self.properties)
    }
}

/// Whether `stats` and `properties` compare the stats of the item: see
/// [`SearchFilters::compares_stats`].
fn compares_stats(stats: &[StatFilter], properties: &[PropertyFilter]) -> bool {
    stats
        .iter()
        .any(|stat| stat.rules_out == RulesOut::OfTheBase)
        || properties
            .iter()
            .any(|filter| filter.rules_out == RulesOut::OfTheBase && filter.limits_anything())
}

/// What a filter rules out of the offers ([`LineStats::rules_out`]).
///
/// [`LineStats::rules_out`]: crate::trade::stat_filters::LineStats::rules_out
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RulesOut {
    /// Items of the base of the item: it compares a stat of the item, what an estimate of a
    /// rare or magic item needs.
    OfTheBase,
    /// Items of the league only: an implicit, a total its implicits reach, the DPS of its
    /// base, its item level…; it narrows a search of any base.
    OfTheLeague,
    /// No stat, and no item of the league it alone rules out: the count of the unrevealed
    /// modifiers, which any base may have.
    Nothing,
}

/// A property of the offers, with its minimum and its maximum.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyFilter {
    pub property: Property,
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    /// What it rules out of the offers. Not sent.
    pub rules_out: RulesOut,
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
    /// What it rules out of the offers. Not sent.
    pub rules_out: RulesOut,
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
        // Each group of filters, with its filters by id.
        let mut groups: Map<String, Value> = Map::new();
        let mut add = |group: &str, id: &str, filter: Value| {
            let filters = groups
                .entry(group)
                .or_insert_with(|| json!({ "filters": {} }));
            filters["filters"][id] = filter;
        };
        query.insert("status".to_owned(), online());
        if let Some(name) = &self.name {
            query.insert("name".to_owned(), json!(name));
        }
        match &self.base {
            SearchedBase::Type(base_type) => {
                query.insert("type".to_owned(), json!(base_type));
            }
            SearchedBase::Category(category) => {
                add("type_filters", "category", json!({ "option": category }));
            }
            SearchedBase::Any => {}
        }
        if !self.stats.is_empty() {
            query.insert("stats".to_owned(), self.stats_json());
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
mod tests;
