//! The target and the query of a search, and its link on the trade site of the language of
//! the item: a unique by its name, an item of the currency exchange by its id, the other items
//! by their base type or their category, with their filters.

use super::*;
use crate::filters::Property;
use crate::item::CopiedItem;
use crate::item_names::tests::item_names;
use crate::trade::query::{PropertyFilter, StatFilter};
use serde_json::Value;

fn parse(text: &str) -> Item {
    Item::parse(&CopiedItem::from_text(text).expect("an item")).expect("read")
}

/// The query of a link, decoded.
fn query(url: &str) -> Value {
    let (_, encoded) = url.split_once("?q=").expect("a query");
    let mut bytes = Vec::new();
    let mut rest = encoded.as_bytes();
    while let [first, tail @ ..] = rest {
        if let (b'%', [high, low, tail @ ..]) = (first, tail) {
            let hex = std::str::from_utf8(&[*high, *low]).unwrap().to_owned();
            bytes.push(u8::from_str_radix(&hex, 16).unwrap());
            rest = tail;
        } else {
            bytes.push(*first);
            rest = tail;
        }
    }
    serde_json::from_slice(&bytes).expect("JSON")
}

#[test]
fn a_unique_is_searched_by_its_name_and_base_type_on_the_site_of_its_language() {
    let crossbow = parse(include_str!("../../item/test-items/fr-unique-crossbow.txt"));
    let target = target(&crossbow, None, None).expect("a target");

    let url = url(
        crossbow.language(),
        &League::new("HC Forbidden Rites"),
        &super::query(&crossbow, &target, SearchFilters::default()),
    );

    assert!(
        url.starts_with("https://fr.pathofexile.com/trade2/search/poe2/HC%20Forbidden%20Rites?q=")
    );
    assert_eq!(
        query(&url),
        json!({
            "query": {
                "status": { "option": "online" },
                "name": "Murmure des brumes",
                "type": "Arbalète de fortune",
                "filters": {
                    "misc_filters": { "filters": { "corrupted": { "option": "false" } } }
                }
            },
            "sort": { "price": "asc" }
        })
    );
}

#[test]
fn an_item_of_the_currency_exchange_opens_its_exchange_page() {
    let names = item_names();
    let stack = parse(include_str!("../../item/test-items/fr-currency-stack.txt"));
    let target = target(&stack, Some(&names), None).expect("a target");

    assert_eq!(target, TradeTarget::Exchange { id: "transmute" });
    let url = url(
        Language::French,
        &League::new("Standard"),
        &super::query(&stack, &target, SearchFilters::default()),
    );
    assert!(url.starts_with("https://fr.pathofexile.com/trade2/exchange/poe2/Standard?q="));
    assert_eq!(
        query(&url),
        json!({
            "exchange": {
                "status": { "option": "online" },
                "have": ["exalted"],
                "want": ["transmute"]
            }
        })
    );
}

#[test]
fn an_uncut_gem_is_an_item_of_the_exchange_and_a_skill_gem_is_searched_by_its_name() {
    let names = item_names();
    for (text, id) in [
        (
            include_str!("../../item/test-items/en-owner-currency-uncut-gem.txt"),
            "uncut-skill-gem-2",
        ),
        (
            include_str!("../../item/test-items/fr-currency-uncut-gem.txt"),
            "uncut-skill-gem-4",
        ),
    ] {
        assert_eq!(
            target(&parse(text), Some(&names), None),
            Some(TradeTarget::Exchange { id }),
            "{text}"
        );
    }
    for (text, base_type) in [
        (
            include_str!("../../item/test-items/en-owner-skill-gem.txt"),
            "Flame Wall",
        ),
        (
            include_str!("../../item/test-items/fr-skill-gem.txt"),
            "Mur de flammes",
        ),
    ] {
        assert_eq!(
            target(&parse(text), Some(&names), None),
            Some(TradeTarget::Base { base_type }),
            "{text}"
        );
    }
}

#[test]
fn the_other_items_are_searched_by_their_base_type() {
    let names = item_names();
    let mace = parse(include_str!("../../item/test-items/en-magic-mace.txt"));
    let rare = parse("Item Class: Rings\nRarity: Rare\nWoe Loop\nRuby Ring");
    let unidentified = parse("Item Class: Jewels\nRarity: Unique\nEmerald\n--------\nUnidentified");

    assert_eq!(
        target(&mace, Some(&names), None),
        Some(TradeTarget::Base {
            base_type: "Temple Maul"
        })
    );
    assert_eq!(target(&mace, None, None), None, "its base type is unknown");
    assert_eq!(
        target(&rare, None, None),
        Some(TradeTarget::Base {
            base_type: "Ruby Ring"
        })
    );
    assert_eq!(
        target(&unidentified, None, None),
        Some(TradeTarget::UnidentifiedUnique {
            base_type: "Emerald"
        })
    );
    let url = url(
        Language::English,
        &League::new("Standard"),
        &TradeTarget::Base {
            base_type: "Ruby Ring",
        }
        .query(),
    );
    assert_eq!(
        query(&url)["query"]["filters"]["type_filters"]["filters"]["rarity"]["option"],
        "nonunique"
    );
}

#[test]
fn an_english_item_of_the_exchange_is_found_in_the_prices_without_the_names() {
    use crate::prices::{CurrencyPrice, Price, Rates};
    let prices = PriceList {
        league: League::new("Standard"),
        updated: std::time::SystemTime::UNIX_EPOCH,
        rates: Rates {
            chaos: 75.0,
            divine: 750.0,
        },
        uniques: Vec::new(),
        currencies: vec![CurrencyPrice {
            name: "Exalted Orb".to_owned(),
            exchange_id: "exalted".to_owned(),
            price: Price {
                exalted: 1.0,
                quantity: 1000,
                history: Vec::new(),
            },
        }],
    };
    let orb = parse("Item Class: Stackable Currency\nRarity: Currency\nExalted Orb");

    let target = target(&orb, None, Some(&prices)).expect("a target");

    assert_eq!(target, TradeTarget::Exchange { id: "exalted" });
    // The Exalted Orb itself is paid in Divine Orbs.
    let url = url(Language::English, &League::new("Standard"), &target.query());
    assert_eq!(query(&url)["exchange"]["have"], json!(["divine"]));
}

/// The states searched for the ring of `rarity` written with `states`, the player having
/// lit the state tags `lit`.
fn searched_states(rarity: &str, states: &str, lit: &[ItemState]) -> StateFilters {
    let names = item_names();
    let ring = parse(&format!(
        "Item Class: Rings\nRarity: {rarity}\nWoe Loop\nRuby Ring\n--------\n{states}"
    ));
    let target = target(&ring, Some(&names), None).expect("a target");
    let filters = SearchFilters {
        states: lit.to_vec(),
        ..SearchFilters::default()
    };
    let Query::Items(items) = super::query(&ring, &target, filters) else {
        panic!("a search of items");
    };
    items.states
}

#[test]
fn a_corrupted_item_is_compared_with_offers_corrupted_or_not_until_its_tag_is_lit() {
    let corrupted = StateFilters {
        corrupted: Some(true),
        ..StateFilters::default()
    };
    let twice_corrupted = StateFilters {
        twice_corrupted: Some(true),
        ..StateFilters::default()
    };
    for (rarity, state, tag, lit) in [
        ("Rare", "Corrupted", ItemState::Corrupted, corrupted),
        ("Unique", "Corrupted", ItemState::Corrupted, corrupted),
        (
            "Rare",
            "Twice Corrupted",
            ItemState::TwiceCorrupted,
            twice_corrupted,
        ),
    ] {
        assert_eq!(
            searched_states(rarity, state, &[]),
            StateFilters::default(),
            "{rarity} {state}"
        );
        assert_eq!(
            searched_states(rarity, state, &[tag]),
            lit,
            "{rarity} {state}"
        );
    }
}

#[test]
fn two_lit_state_tags_search_the_offers_of_both_states() {
    assert_eq!(
        searched_states(
            "Rare",
            "Corrupted\n--------\nFractured Item",
            &[ItemState::Corrupted, ItemState::Fractured]
        ),
        StateFilters {
            corrupted: Some(true),
            fractured: Some(true),
            ..StateFilters::default()
        }
    );
}

#[test]
fn a_lit_state_tag_searches_the_offers_of_that_state_only() {
    let lit = |states: &str, tag| searched_states("Rare", states, &[tag]);

    assert_eq!(
        lit("Sanctified", ItemState::Sanctified),
        StateFilters {
            corrupted: Some(false),
            sanctified: Some(true),
            ..StateFilters::default()
        }
    );
    assert_eq!(
        lit("Mirrored", ItemState::Mirrored),
        StateFilters {
            corrupted: Some(false),
            mirrored: Some(true),
            ..StateFilters::default()
        }
    );
    // A fractured item can still be crafted.
    assert_eq!(
        lit("Fractured Item", ItemState::Fractured),
        StateFilters {
            corrupted: Some(false),
            twice_corrupted: None,
            sanctified: Some(false),
            fractured: Some(true),
            mirrored: Some(false),
        }
    );
    assert_eq!(
        searched_states("Rare", "Fractured Item", &[]).fractured,
        None,
        "fractured or not"
    );
    // A tag the item doesn't have is never searched.
    assert_eq!(
        lit("Item Level: 1", ItemState::Corrupted).corrupted,
        Some(false)
    );
}

#[test]
fn a_clean_item_is_compared_with_offers_neither_sanctified_nor_mirrored() {
    // The states searched: corrupted, sanctified, mirrored.
    let names = item_names();
    let states = |text: &str| {
        let item = parse(text);
        let target = target(&item, Some(&names), None).expect("a target");
        let Query::Items(items) = super::query(&item, &target, SearchFilters::default()) else {
            panic!("a search of items");
        };
        let states = items.states;
        (states.corrupted, states.sanctified, states.mirrored)
    };
    let ring = |rarity: &str, state: &str| {
        states(&format!(
            "Item Class: Rings\nRarity: {rarity}\nWoe Loop\nRuby Ring\n--------\n{state}"
        ))
    };

    let clean = [
        states("Item Class: Rings\nRarity: Normal\nRuby Ring\n--------\nItem Level: 1"),
        states(include_str!("../../item/test-items/en-magic-mace.txt")),
        ring("Rare", "Item Level: 1"),
    ];
    for states in clean {
        assert_eq!(states, (Some(false), Some(false), Some(false)));
    }
    // An item that can't be crafted any more is searched among the offers sanctified,
    // mirrored or not.
    assert_eq!(ring("Rare", "Corrupted"), (None, None, None));
    assert_eq!(ring("Rare", "Sanctified"), (Some(false), None, None));
    assert_eq!(ring("Rare", "Mirrored"), (Some(false), None, None));
    // A unique is searched by its name.
    assert_eq!(
        states("Item Class: Rings\nRarity: Unique\nAndvarius\nGold Ring"),
        (Some(false), None, None)
    );
}

#[test]
fn a_unique_is_searched_with_its_stats_and_the_other_items_with_all_their_filters() {
    let stats = vec![StatFilter {
        ids: vec!["explicit.stat_3299347043".to_owned()],
        minimum: Some(45.0),
        maximum: None,
    }];
    let properties = vec![PropertyFilter {
        property: Property::ItemLevel,
        minimum: Some(81.0),
        maximum: None,
    }];
    let filters = |any_base| SearchFilters {
        any_base,
        properties: properties.clone(),
        stats: stats.clone(),
        states: Vec::new(),
    };
    let searched = |item: &Item, any_base| {
        let target = target(item, None, None).expect("a target");
        match super::query(item, &target, filters(any_base)) {
            Query::Items(items) => items,
            Query::Exchange(_) => panic!("a search of items"),
        }
    };

    let unique = parse("Item Class: Rings\nRarity: Unique\nAndvarius\nGold Ring");
    let searched_unique = searched(&unique, true);
    assert_eq!(searched_unique.stats, stats);
    // A unique is searched by its name and base type, without the filters of the others.
    assert_eq!(searched_unique.base_type.as_deref(), Some("Gold Ring"));
    assert_eq!(searched_unique.properties, Vec::new());
    let rare = parse("Item Class: Rings\nRarity: Rare\nWoe Loop\nRuby Ring");
    let searched_rare = searched(&rare, false);
    assert_eq!(searched_rare.stats, stats);
    assert_eq!(searched_rare.properties, properties);
    assert_eq!(searched_rare.base_type.as_deref(), Some("Ruby Ring"));
    // The base type unchecked: any base.
    assert_eq!(searched(&rare, true).base_type, None);
}

/// The base type and the category `item` is searched with, its base type unchecked when
/// `any_base`, with a minimum item level when `narrowed`.
fn searched_base(item: &Item, any_base: bool, narrowed: bool) -> (Option<String>, Option<&str>) {
    let target = target(item, None, None).expect("a target");
    let filters = SearchFilters {
        any_base,
        properties: vec![PropertyFilter {
            property: Property::ItemLevel,
            minimum: narrowed.then_some(81.0),
            maximum: None,
        }],
        stats: Vec::new(),
        states: Vec::new(),
    };
    match super::query(item, &target, filters) {
        Query::Items(items) => (items.base_type, items.category),
        Query::Exchange(_) => panic!("a search of items"),
    }
}

#[test]
fn the_base_type_unchecked_searches_its_category() {
    let rare = parse("Item Class: Rings\nRarity: Rare\nWoe Loop\nRuby Ring");
    assert_eq!(
        searched_base(&rare, false, true),
        (Some("Ruby Ring".to_owned()), None)
    );
    // A ring among all the rings, also when nothing else narrows the search.
    for narrowed in [false, true] {
        assert_eq!(
            searched_base(&rare, true, narrowed),
            (None, Some("accessory.ring"))
        );
    }
    // In French, and a flask by its kind: the trade site tells the life flasks from the
    // mana flasks.
    let french = parse(include_str!(
        "../../item/test-items/fr-rare-body-armour.txt"
    ));
    assert_eq!(
        searched_base(&french, true, false),
        (None, Some("armour.chest"))
    );
    let flask = parse("Item Class: Mana Flasks\nRarity: Normal\nUltimate Mana Flask");
    assert_eq!(
        searched_base(&flask, true, false),
        (None, Some("flask.mana"))
    );
}

#[test]
fn a_magic_item_is_searched_by_the_category_of_the_base_its_name_holds() {
    let names = item_names();
    let mace = parse(include_str!("../../item/test-items/en-magic-mace.txt"));
    let target = target(&mace, Some(&names), None).expect("a target");
    let filters = SearchFilters {
        any_base: true,
        ..SearchFilters::default()
    };

    match super::query(&mace, &target, filters) {
        Query::Items(items) => assert_eq!(
            (items.base_type, items.category),
            (None, Some("weapon.twomace"))
        ),
        Query::Exchange(_) => panic!("a search of items"),
    }
}

#[test]
fn a_base_the_game_data_doesnt_know_is_searched_when_nothing_else_narrows_the_search() {
    let rare = parse("Item Class: Rings\nRarity: Rare\nWoe Loop\nMystery Ring");

    assert_eq!(
        searched_base(&rare, true, false),
        (Some("Mystery Ring".to_owned()), None)
    );
    assert_eq!(searched_base(&rare, true, true), (None, None));
}

#[test]
fn an_item_of_the_exchange_is_never_filtered_by_its_corrupted_state() {
    let names = item_names();
    let stack = parse(include_str!("../../item/test-items/fr-currency-stack.txt"));
    let target = target(&stack, Some(&names), None).expect("a target");

    assert_eq!(
        super::query(&stack, &target, SearchFilters::default()),
        Query::Exchange("transmute".to_owned())
    );
}
