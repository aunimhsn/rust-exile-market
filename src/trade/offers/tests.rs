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
        quality(r#"{"name": "Kwality (Mana Modifiers)", "values": [["+20%", 1]], "type": "6"}"#),
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
fn the_offers_priced_in_waystones_or_an_unknown_currency_are_left_out() {
    let listed = |id: &str, price: &str| {
        format!(
            r#"{{"id": "{id}", "listing": {{{price}"account": {{}}}}, "item": {{"typeLine": "Waystone (Tier 12)"}}}}"#
        )
    };
    let price = |amount: u32, currency: &str| {
        format!(r#""price": {{"type": "~price", "amount": {amount}, "currency": "{currency}"}}, "#)
    };
    let fetch: ApiFetch = serde_json::from_str(&format!(
        r#"{{"result": [{}, {}, {}, {}, {}]}}"#,
        listed("a", &price(1, "exalted")),
        listed("b", &price(3, "waystone-12")),
        listed("c", &price(1, "vaal")),
        listed("d", &price(4, "not-a-currency")),
        listed("e", ""),
    ))
    .expect("read");

    let offers = fetch.offers(Language::English);

    let ids: Vec<&str> = offers.iter().map(|offer| offer.id.as_str()).collect();
    assert_eq!(ids, ["a", "c", "e"], "an offer without price stays");
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
/// properties aren't those of a waystone; and a magic waystone with a property renamed, a
/// value that can't be read and a property without value.
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
            {"name": "Waystone Drop Chance", "values": [["+20%", 1]], "displayMode": 0, "type": 66},
            {"name": "[Pack|Pack Size]", "values": [], "displayMode": 0, "type": 4}
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

/// Offers of `/fetch` on 2026-11-01, shortened: a Comet of level 21 corrupted, as the English
/// and the French trade sites give it; one of level 16 without quality nor corruption; a body
/// armour and its requirements. Then a weapon written by hand, whose strength is told by its
/// name only: no strength was seen on 2026-11-01.
const GEMS_AND_ARMOURS: &str = r#"{"result": [
    {"id": "g1", "listing": {"account": {}}, "item": {
        "support": false, "gemSockets": ["W", "W"], "name": "", "typeLine": "Comet", "baseType": "Comet",
        "ilvl": 0, "corrupted": true, "frameType": 4,
        "properties": [
            {"name": "[Spell], [AoESkill|AoE], [Cold], [Repeat|Repeatable]", "values": [], "displayMode": 0},
            {"name": "Level", "values": [["21", 1]], "displayMode": 0, "type": 5},
            {"name": "", "values": [["20 Levels from Gem (Max)", 0]], "displayMode": 0},
            {"name": "", "values": [["+1 Level from Corruption", 1]], "displayMode": 0},
            {"name": "[Quality]", "values": [["+20%", 1]], "displayMode": 0, "type": 6},
            {"name": "Cost", "values": [["191 Mana", 0]], "displayMode": 0}
        ],
        "requirements": [
            {"name": "Level", "values": [["90", 0]], "displayMode": 0, "type": 62},
            {"name": "[Intelligence|Int]", "values": [["157", 0]], "displayMode": 1, "type": 65}
        ]
    }},
    {"id": "g2", "listing": {"account": {}}, "item": {
        "support": false, "gemSockets": ["W", "W"], "name": "", "typeLine": "Comète", "baseType": "Comète",
        "ilvl": 0, "corrupted": true, "frameType": 4,
        "properties": [
            {"name": "Niveau", "values": [["21", 1]], "displayMode": 0, "type": 5},
            {"name": "", "values": [["Niveau de la Gemme: 20 (maxi)", 0]], "displayMode": 0},
            {"name": "[Quality|Qualité]", "values": [["+20%", 1]], "displayMode": 0, "type": 6}
        ],
        "requirements": [
            {"name": "Niveau", "values": [["90", 0]], "displayMode": 0, "type": 62},
            {"name": "[Intelligence|Int]", "values": [["157", 0]], "displayMode": 1, "type": 65}
        ]
    }},
    {"id": "g3", "listing": {"account": {}}, "item": {
        "gemSockets": ["W", "W", "W", "W"], "name": "", "typeLine": "Comet", "baseType": "Comet", "ilvl": 0,
        "properties": [{"name": "Level", "values": [["16", 0]], "displayMode": 0, "type": 5}],
        "requirements": [
            {"name": "Level", "values": [["66", 0]], "displayMode": 0, "type": 62},
            {"name": "[Intelligence|Int]", "values": [["116", 0]], "displayMode": 1, "type": 65}
        ]
    }},
    {"id": "a1", "listing": {"account": {}}, "item": {
        "name": "Sorrow Shell", "typeLine": "Rambler Jacket", "baseType": "Rambler Jacket", "rarity": "Rare", "ilvl": 80,
        "properties": [
            {"name": "Body Armour", "values": [], "displayMode": 0, "type": 109},
            {"name": "[Evasion|Evasion Rating]", "values": [["407", 1]], "displayMode": 0, "type": 17}
        ],
        "requirements": [
            {"name": "Level", "values": [["70", 0]], "displayMode": 0, "type": 62},
            {"name": "[Dexterity|Dex]", "values": [["67", 0]], "displayMode": 1, "type": 64},
            {"name": "[Intelligence|Int]", "values": [["67", 0]], "displayMode": 1, "type": 65}
        ]
    }},
    {"id": "a2", "listing": {"account": {}}, "item": {
        "name": "Doom Song", "typeLine": "Heavy Bow", "rarity": "Rare", "ilvl": 80,
        "requirements": [
            {"name": "Level", "values": [["72", 0]], "displayMode": 0, "type": 62},
            {"name": "[Strength|Str]", "values": [["163", 0]], "displayMode": 1}
        ]
    }}
]}"#;

#[test]
fn the_level_sockets_and_requirements_of_an_offer_of_gem_are_read_in_both_languages() {
    let fetch = || serde_json::from_str::<ApiFetch>(GEMS_AND_ARMOURS).expect("read");
    let gem = |offer: &ItemOffer| {
        let item = &offer.item;
        (
            item.gem_level,
            item.quality.as_ref().map(|quality| quality.value),
            item.gem_sockets,
            item.item_level,
        )
    };
    let english = fetch().offers(Language::English);
    assert_eq!(gem(&english[0]), (Some(21), Some(20), Some(2), None));
    assert_eq!(
        english[0].item.requirements,
        Requirements {
            level: Some(90),
            intelligence: Some(157),
            ..Requirements::default()
        }
    );
    assert_eq!(gem(&english[2]), (Some(16), None, Some(4), None));
    let french = fetch().offers(Language::French);
    assert_eq!(gem(&french[1]), (Some(21), Some(20), Some(2), None));
    assert_eq!(french[1].item.requirements.level, Some(90));
    // Not a gem: no level of gem nor sockets, its item level kept.
    assert_eq!(gem(&english[3]), (None, None, None, Some(80)));
}

#[test]
fn the_requirements_of_an_offer_are_read_by_their_type_or_their_name() {
    let offers = serde_json::from_str::<ApiFetch>(GEMS_AND_ARMOURS)
        .expect("read")
        .offers(Language::English);
    assert_eq!(
        offers[3].item.requirements,
        Requirements {
            level: Some(70),
            strength: None,
            dexterity: Some(67),
            intelligence: Some(67),
        }
    );
    assert_eq!(
        offers[4].item.requirements,
        Requirements {
            level: Some(72),
            strength: Some(163),
            ..Requirements::default()
        }
    );
}

#[test]
fn a_property_of_an_offer_of_waystone_that_cant_be_read_is_kept_to_log() {
    use UnreadProperty::{Name, Value};

    let read = |language| {
        let fetch: ApiFetch = serde_json::from_str(WAYSTONES).expect("read");
        fetch
            .result
            .into_iter()
            .flatten()
            .map(|listed| listed.offer(language))
            .map(|(offer, unreadable)| (offer.item.waystone_unread, unreadable.waystone_properties))
            .collect::<Vec<_>>()
    };

    let english = read(Language::English);
    assert_eq!(english[0], (false, vec![]));
    // Not a waystone: its properties are those of another item.
    assert_eq!(english[2], (false, vec![]));
    assert_eq!(
        english[3],
        (
            true,
            vec![
                Name("Monster Quantity".to_owned()),
                Value("Item Rarity: +1.5%".to_owned()),
                Value("Pack Size: ".to_owned()),
            ]
        ),
        "a name renamed by a patch, a value that can't be read, no value"
    );
    // A base of another language isn't read as a waystone.
    assert_eq!(english[1], (false, vec![]));
    let french = read(Language::French);
    assert_eq!(french[0], (false, vec![]));
    assert_eq!(french[1], (false, vec![]));
}

#[test]
fn what_cant_be_read_of_a_page_is_logged_once_and_an_unknown_name_whatever_its_value() {
    use UnreadProperty::{Name, Value};

    let renamed = |value: &str| Unreadable {
        quality: None,
        waystone_properties: vec![
            Name("Monster Quantity".to_owned()),
            Value(format!("Item Rarity: {value}")),
        ],
    };
    let unreadable = [
        renamed("+1.5%"),
        renamed("+1.5%"),
        renamed("+2.5%"),
        Unreadable {
            quality: Some("Quality: +1.5%".to_owned()),
            waystone_properties: Vec::new(),
        },
        Unreadable::default(),
    ];

    assert_eq!(
        warnings(&unreadable),
        [
            "Trade site: a quality that can't be read, Quality: +1.5%",
            "Trade site: an unknown property of a waystone, Monster Quantity",
            "Trade site: a property of a waystone that can't be read, Item Rarity: +1.5%",
            "Trade site: a property of a waystone that can't be read, Item Rarity: +2.5%",
        ]
    );
    assert_eq!(warnings(&[]), Vec::<String>::new());
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
