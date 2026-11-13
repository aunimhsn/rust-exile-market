use super::*;

#[test]
fn a_category_is_searched_among_the_type_filters() {
    let query = ItemQuery {
        name: None,
        base: SearchedBase::Category("weapon.bow"),
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
        base: SearchedBase::Type("Ruby Ring".to_owned()),
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
        base: SearchedBase::Type("Ruby Ring".to_owned()),
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
        base: SearchedBase::Type("Ruby Ring".to_owned()),
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
        base: SearchedBase::Type("Gold Ring".to_owned()),
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
                rules_out: RulesOut::OfTheBase,
            },
            StatFilter {
                ids: vec!["explicit.stat_3261801346".to_owned()],
                minimum: None,
                maximum: None,
                rules_out: RulesOut::OfTheBase,
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
        base: SearchedBase::Type("Gold Ring".to_owned()),
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
                rules_out: RulesOut::OfTheBase,
            },
            StatFilter {
                ids: vec!["explicit.stat_2250533757".to_owned()],
                minimum: None,
                maximum: Some(-15.0),
                rules_out: RulesOut::OfTheBase,
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
        base: SearchedBase::Type("Gold Ring".to_owned()),
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
            rules_out: RulesOut::OfTheBase,
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
        base: SearchedBase::Any,
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
                rules_out: RulesOut::OfTheLeague,
            },
            PropertyFilter {
                property: Property::Sockets,
                minimum: Some(3.0),
                maximum: None,
                rules_out: RulesOut::OfTheLeague,
            },
            PropertyFilter {
                property: Property::Quality,
                minimum: Some(24.0),
                maximum: None,
                rules_out: RulesOut::OfTheLeague,
            },
            PropertyFilter {
                property: Property::RequiredLevel,
                minimum: None,
                maximum: Some(40.0),
                rules_out: RulesOut::OfTheLeague,
            },
            PropertyFilter {
                property: Property::PhysicalDps,
                minimum: Some(220.0),
                maximum: Some(300.0),
                rules_out: RulesOut::OfTheBase,
            },
            PropertyFilter {
                property: Property::ElementalDps,
                minimum: None,
                maximum: None,
                rules_out: RulesOut::OfTheBase,
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
        rules_out: RulesOut::OfTheBase,
    };
    let query = ItemQuery {
        name: None,
        base: SearchedBase::Type("Waystone (Tier 12)".to_owned()),
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
        rules_out: RulesOut::OfTheBase,
    };
    let query = ItemQuery {
        name: None,
        base: SearchedBase::Type("Comet".to_owned()),
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

#[test]
fn a_search_compares_the_item_by_a_filter_that_rules_out_items_of_its_base() {
    use RulesOut::{Nothing, OfTheBase, OfTheLeague};

    let property = |rules_out, minimum| PropertyFilter {
        property: Property::PhysicalDps,
        minimum,
        maximum: None,
        rules_out,
    };
    let stat = |rules_out| StatFilter {
        ids: vec!["explicit.stat_3917489142".to_owned()],
        minimum: None,
        maximum: None,
        rules_out,
    };
    let filters = |properties, stats| SearchFilters {
        any_base: false,
        states: Vec::new(),
        properties,
        stats,
    };
    let league = || property(OfTheLeague, Some(80.0));

    // Nothing, or filters that rule out items of the league only: any items of the base.
    assert!(!filters(Vec::new(), Vec::new()).compares_stats());
    let base = filters(vec![league()], vec![stat(OfTheLeague), stat(Nothing)]);
    assert!(!base.compares_stats());
    // A property without limit isn't sent.
    assert!(!filters(vec![property(OfTheBase, None)], Vec::new()).compares_stats());
    assert!(filters(vec![property(OfTheBase, Some(80.0))], Vec::new()).compares_stats());
    assert!(filters(Vec::new(), vec![stat(Nothing), stat(OfTheBase)]).compares_stats());

    // Narrowed: anything but the count of the unrevealed modifiers.
    assert!(!filters(Vec::new(), vec![stat(Nothing)]).narrows());
    assert!(filters(Vec::new(), vec![stat(OfTheLeague)]).narrows());
    assert!(filters(vec![league()], Vec::new()).narrows());

    // The query sent says the same; a search of the currency exchange compares nothing.
    let query = |stats| {
        Query::Items(ItemQuery {
            name: None,
            base: SearchedBase::Type("Ruby".to_owned()),
            rarity: None,
            states: StateFilters::default(),
            properties: Vec::new(),
            stats,
        })
    };
    assert!(!query(vec![stat(OfTheLeague)]).compares_stats());
    assert!(query(vec![stat(OfTheBase)]).compares_stats());
    assert!(!Query::Exchange("divine".to_owned()).compares_stats());
}
