//! The filters of the properties of the item, of a waystone and of a gem, of the DPS and of the
//! pseudo totals, and the values of the game their fields stay within.

use super::*;
use crate::filters::Filters;
use crate::i18n::{Message, Translator};
use crate::language::Language;
use crate::roll_difference::CopiedLines;
use crate::tier_rolls::TierRolls;
use crate::ui::item_stats::Stats;
use crate::ui::item_stats::fixtures::{
    base_type, default_checks, parse, rare_item, row_texts, rows, texts,
};
use crate::ui::item_stats::model::{Group, Row};

#[test]
fn a_weapon_mostly_elemental_is_searched_by_its_elemental_dps_too() {
    // With `lines`, its modifiers.
    let weapon = |physical: &str, lightning: &str, lines: &str| {
        parse(&format!(
            "Item Class: Two Hand Maces\nRarity: Rare\nStorm Fall\nTemple Maul\n--------\n\
             Physical Damage: {physical} (augmented)\nLightning Damage: {lightning} (lightning)\n\
             Attacks per Second: 1.00\n--------\nItem Level: 80\n--------\n{lines}"
        ))
    };
    let dps = |item: &Item| texts(item)[4..6].to_vec();
    let strong = "100% increased Physical Damage";

    assert_eq!(
        dps(&weapon(
            "20-40",
            "1-59",
            &format!("{strong}\nAdds 1 to 59 Lightning Damage")
        )),
        ["■ PhysicalDps: 30 ≥27", "■ ElementalDps: 30 ≥27"]
    );
    // 90 % of a small DPS, of 2.5 and not of the 3 shown, rounded down: the weapon
    // passes its own filter.
    assert_eq!(
        dps(&weapon(
            "20-40",
            "1-4",
            &format!("{strong}\nAdds 1 to 4 Lightning Damage")
        )),
        ["■ PhysicalDps: 30 ≥27", "□ ElementalDps: 3 ≥2"]
    );
    // A small line of physical damage: its base reaches 15 at 20 % quality, beyond the
    // minimum, so that every weapon of it may pass; the elemental one, smaller, is checked in
    // its place.
    assert_eq!(
        dps(&weapon(
            "10-20",
            "1-4",
            "20% increased Physical Damage\nAdds 1 to 4 Lightning Damage"
        )),
        ["□ PhysicalDps: 15 ≥13", "■ ElementalDps: 3 ≥2"]
    );
    // Without a line of damage nor of attack speed: the DPS of its base, unchecked.
    assert_eq!(
        dps(&weapon("10-20", "1-59", "+20 to Strength")),
        ["□ PhysicalDps: 15 ≥13", "□ ElementalDps: 30 ≥27"]
    );
}

#[test]
fn a_corrupted_item_counts_each_unrevealed_modifier() {
    let ring = parse(
        "Item Class: Rings\nRarity: Rare\nWoe Loop\nRuby Ring\n--------\n\
         Item Level: 80\n--------\n\
         { Prefix Modifier \"Veiled\" }\nDesecrated Prefix\n\
         { Suffix Modifier \"of the Veil\" }\nDesecrated Suffix\n--------\nCorrupted",
    );
    assert_eq!(
        texts(&ring),
        [
            "Item",
            "■ Base: Ruby Ring",
            "□ ItemLevel: 80 ≥80",
            "■ UnrevealedModifiers: 2 ≥2",
            "Prefixes 1/3",
            "Unrevealed Prefix",
            "Suffixes 1/3",
            "Unrevealed Suffix",
            "Unrevealed note",
        ]
    );
}

/// The rows of the Item group of `item`, without its title.
fn item_group(item: &Item) -> Vec<String> {
    item_group_of(item, base_type(item))
}

/// The rows of the Item group of `item` searched by `base_type`, without its title: the
/// base type of a magic item is found among the names of the trade site.
fn item_group_of<'a>(item: &'a Item, base_type: Option<&'a str>) -> Vec<String> {
    let lines = CopiedLines::default();
    let stats = LineStats::new(item, item.modifiers(), None);
    let rows = crate::ui::item_stats::rows(item, &lines, &stats, &TierRolls::default(), base_type);
    row_texts(
        rows.into_iter()
            .skip_while(|row| *row != Row::Group(Group::Item))
            .skip(1)
            .take_while(|row| matches!(row, Row::Stat(_))),
    )
}

#[test]
fn a_normal_item_is_searched_by_its_base_sockets_item_level_and_quality() {
    let shield = parse(include_str!(
        "../../../item/test-items/fr-normal-exceptional-shield.txt"
    ));
    let helmet = parse(include_str!(
        "../../../item/test-items/en-normal-helmet.txt"
    ));
    let targe = parse(
        "Item Class: Shields\nRarity: Normal\nExceptional Mammoth Targe\n--------\n\
         Quality: +24% (augmented)\nArmour: 139 (augmented)\n--------\nItem Level: 82",
    );

    // An exceptional base by its sockets: the trade site doesn't know its name.
    assert_eq!(
        item_group(&shield),
        [
            "■ Base: Targe de mammouth",
            "■ ItemLevel: 82 ≥82",
            "■ Sockets: 2 ≥2",
        ]
    );
    assert_eq!(
        item_group(&helmet),
        [
            "■ Base: Divine Crown",
            "■ ItemLevel: 81 ≥81",
            "□ Quality: 9 ≥9",
        ]
    );
    // An exceptional base by its quality.
    assert_eq!(
        item_group(&targe),
        [
            "■ Base: Mammoth Targe",
            "■ ItemLevel: 82 ≥82",
            "■ Quality: 24 ≥24",
        ]
    );
}

#[test]
fn the_base_type_tells_the_item_class_searched_when_it_is_unchecked() {
    // The class under the base type `base_type` of `item`, when it is unchecked.
    let class_of = |item: &Item, base_type: &str| {
        let lines = CopiedLines::default();
        let stats = LineStats::new(item, item.modifiers(), None);
        let tiers = TierRolls::default();
        crate::ui::item_stats::rows(item, &lines, &stats, &tiers, Some(base_type))
            .into_iter()
            .find_map(|row| match row {
                Row::Stat(Stat {
                    text: StatText::BaseType { class, .. },
                    ..
                }) => Some(class.map(str::to_owned)),
                _ => None,
            })
    };

    // The item class of the copied text, in its language.
    let armour = parse(include_str!(
        "../../../item/test-items/fr-rare-body-armour.txt"
    ));
    assert_eq!(
        class_of(&armour, armour.base_type()),
        Some(Some("Armures".to_owned()))
    );
    // A magic item, by the base type its name holds.
    let mace = parse(include_str!("../../../item/test-items/en-magic-mace.txt"));
    assert_eq!(
        class_of(&mace, "Temple Maul"),
        Some(Some("Two Hand Maces".to_owned()))
    );
    // A base the game data doesn't know is searched among any base instead.
    let ring = parse("Item Class: Rings\nRarity: Rare\nWoe Loop\nMystery Ring");
    assert_eq!(class_of(&ring, ring.base_type()), Some(None));
}

/// The filter of `property` among the filters of `item`.
fn property_filter_of(item: &Item, property: Property) -> StatFilter {
    rows(item)
        .into_iter()
        .find_map(|row| match row {
            Row::Stat(Stat {
                key: FilterKey::Property(key),
                filter: Some(filter),
                ..
            }) if key == property => Some(filter),
            _ => None,
        })
        .expect("the filter of the property")
}

/// The value the player types in `half` of the field of `property`, as it is when they
/// leave it.
fn settled(item: &Item, property: Property, half: Half, typed: &str) -> String {
    let mut choice = property_filter_of(item, property).default_choice();
    *choice.text_mut(half) = typed.to_owned();
    choice.settle(half);
    choice.text(half).to_owned()
}

#[test]
fn the_fields_of_the_properties_stay_within_the_values_of_the_game() {
    let crossbow = parse(include_str!(
        "../../../item/test-items/en-rare-crossbow-all-modifiers.txt"
    ));
    let leveling = parse(
        "Item Class: Body Armours\nRarity: Rare\nHate Shell\nWaxed Jacket\n--------\n\
         Quality: +23% (augmented)\nEvasion Rating: 120\n--------\n\
         Requires: Level 40, 30 Dex\n--------\nItem Level: 45",
    );

    assert_eq!(
        settled(&crossbow, Property::ItemLevel, Half::Minimum, "5000"),
        "100"
    );
    assert_eq!(
        settled(&crossbow, Property::ItemLevel, Half::Minimum, "0"),
        "1"
    );
    assert_eq!(
        settled(&crossbow, Property::Sockets, Half::Minimum, "9"),
        "6"
    );
    // Up to the 30 % of an exceptional base or of a Vaal Infuser.
    assert_eq!(
        settled(&leveling, Property::Quality, Half::Minimum, "80"),
        "30"
    );
    assert_eq!(
        settled(&leveling, Property::RequiredLevel, Half::Maximum, "95"),
        "90"
    );
}

#[test]
fn a_property_out_of_the_values_of_the_game_widens_its_field() {
    let item = parse(
        "Item Class: Rings\nRarity: Rare\nStorm Loop\nGold Ring\n--------\n\
         Item Level: 101",
    );

    // Its own value stays; a higher one is brought back to it.
    assert_eq!(
        settled(&item, Property::ItemLevel, Half::Minimum, "101"),
        "101"
    );
    assert_eq!(
        settled(&item, Property::ItemLevel, Half::Minimum, "5000"),
        "101"
    );
}

#[test]
fn the_sockets_are_checked_above_those_of_the_base_or_on_a_corrupted_item() {
    let armour = |sockets: &str, corrupted: &str| {
        parse(&format!(
            "Item Class: Body Armours\nRarity: Rare\nHate Shell\nWaxed Jacket\n--------\n\
             Sockets: {sockets}\n--------\nItem Level: 80\n--------\n\
             {{ Prefix Modifier }}\n+80 to maximum Life{corrupted}"
        ))
    };
    let sockets = |item: &Item| {
        item_group(item)
            .into_iter()
            .find(|text| text.contains("Sockets"))
    };

    // A body armour usually has two.
    assert_eq!(
        sockets(&armour("S S", "")).as_deref(),
        Some("□ Sockets: 2 ≥2")
    );
    assert_eq!(
        sockets(&armour("S S S", "")).as_deref(),
        Some("■ Sockets: 3 ≥3")
    );
    assert_eq!(
        sockets(&armour("S", "\n--------\nCorrupted")).as_deref(),
        Some("■ Sockets: 1 ≥1")
    );
    let none = parse(
        "Item Class: Body Armours\nRarity: Rare\nHate Shell\nWaxed Jacket\n--------\n\
         Item Level: 80\n--------\n{ Prefix Modifier }\n+80 to maximum Life",
    );
    assert_eq!(sockets(&none), None);
}

#[test]
fn an_exceptional_quality_is_checked_except_on_a_rare() {
    let bow = parse(include_str!(
        "../../../item/test-items/en-rare-bow-fractured.txt"
    ));
    let magic = parse(
        "Item Class: Gloves\nRarity: Magic\nVaal Mitts of the Fox\n--------\n\
         Quality: +23% (augmented)\n--------\nItem Level: 70\n--------\n\
         { Suffix Modifier }\n+20 to Dexterity",
    );
    let twenty = parse(
        "Item Class: Gloves\nRarity: Magic\nVaal Mitts of the Fox\n--------\n\
         Quality: +20% (augmented)\n--------\nItem Level: 70\n--------\n\
         { Suffix Modifier }\n+20 to Dexterity",
    );

    // Its bow usually has two sockets; its required level is above 75.
    assert_eq!(
        item_group(&bow),
        [
            "■ Base: Obliterator Bow",
            "□ ItemLevel: 81 ≥81",
            "□ Sockets: 2 ≥2",
            "□ Quality: 25 ≥25",
        ]
    );
    assert!(item_group(&magic).contains(&"■ Quality: 23 ≥23".to_owned()));
    assert!(
        !item_group(&twenty)
            .iter()
            .any(|text| text.contains("Quality"))
    );
}

#[test]
fn a_rare_for_leveling_has_its_required_level_as_a_maximum() {
    let ring = |required: u32, item_level: u32| {
        parse(&format!(
            "Item Class: Rings\nRarity: Rare\nHate Loop\nRuby Ring\n--------\n\
             Requires: Level {required}\n--------\nItem Level: {item_level}\n--------\n\
             {{ Prefix Modifier }}\n+30 to maximum Life"
        ))
    };
    let required = |item: &Item| {
        item_group(item)
            .into_iter()
            .find(|text| text.contains("RequiredLevel"))
    };

    assert_eq!(
        required(&ring(40, 45)).as_deref(),
        Some("□ RequiredLevel: 40 ≤40")
    );
    assert_eq!(
        required(&ring(75, 75)).as_deref(),
        Some("□ RequiredLevel: 75 ≤75")
    );
    assert_eq!(required(&ring(40, 80)), None);
    assert_eq!(required(&ring(78, 80)), None);
    // A magic item, even for leveling.
    let magic = parse(
        "Item Class: Rings\nRarity: Magic\nVital Ruby Ring\n--------\n\
         Requires: Level 40\n--------\nItem Level: 45\n--------\n\
         { Prefix Modifier }\n+30 to maximum Life",
    );
    assert_eq!(required(&magic), None);
}

#[test]
fn the_sockets_of_a_base_the_data_doesnt_know_are_checked_only_on_a_corrupted_item() {
    let armour = |corrupted: &str| {
        parse(&format!(
            "Item Class: Body Armours\nRarity: Rare\nHate Shell\nPatch Jacket\n--------\n\
             Sockets: S S S\n--------\nItem Level: 80\n--------\n\
             {{ Prefix Modifier }}\n+80 to maximum Life{corrupted}"
        ))
    };
    let sockets = |item: &Item| {
        item_group(item)
            .into_iter()
            .find(|text| text.contains("Sockets"))
    };

    assert_eq!(sockets(&armour("")).as_deref(), Some("□ Sockets: 3 ≥3"));
    assert_eq!(
        sockets(&armour("\n--------\nCorrupted")).as_deref(),
        Some("■ Sockets: 3 ≥3")
    );
}

#[test]
fn the_quality_of_a_catalyst_is_a_filter_with_its_kind_unchecked() {
    let amulet = parse(include_str!(
        "../../../item/test-items/en-owner-rare-amulet-catalyst.txt"
    ));
    let ring = parse(include_str!(
        "../../../item/test-items/fr-rare-ring-catalyst.txt"
    ));

    assert_eq!(
        item_group(&amulet),
        [
            "■ Base: Gloam Amulet",
            "□ ItemLevel: 79 ≥79",
            "□ Mana quality: 40 ≥40",
        ]
    );
    // 20 %, a quality a base of a ring has only from a catalyst, is a filter too.
    assert_eq!(
        item_group(&ring),
        [
            "■ Base: Bague prismatique",
            "□ ItemLevel: 82 ≥82",
            "□ Cold quality: 20 ≥20",
        ]
    );
    // Its text says that the trade site searches any kind.
    let catalyst = rows(&amulet)
        .into_iter()
        .find_map(|row| match row {
            Row::Stat(stat) if matches!(stat.text, StatText::CatalystQuality(..)) => {
                Some(stat.text)
            }
            _ => None,
        })
        .expect("the filter of the quality");
    assert_eq!(catalyst.tip(), Some(Message::FilterAnyQualityKind));
}

#[test]
fn the_field_of_the_quality_stays_within_the_highest_quality_of_the_base() {
    // A normal item, whose quality is a filter.
    let normal = |class: &str, base: &str| {
        parse(&format!(
            "Item Class: {class}\nRarity: Normal\n{base}\n--------\n\
             Quality: +10% (augmented)\n--------\nItem Level: 80"
        ))
    };
    let highest = |item: &Item| settled(item, Property::Quality, Half::Minimum, "99");
    assert_eq!(highest(&normal("Bows", "Gemini Bow")), "30");
    assert_eq!(highest(&normal("One Hand Maces", "Flanged Mace")), "40");
    assert_eq!(highest(&normal("Life Flasks", "Ultimate Life Flask")), "23");
    assert_eq!(highest(&normal("Charms", "Ruby Charm")), "23");
    let jewel = parse(
        "Item Class: Jewels\nRarity: Rare\nWoe Spark\nSapphire\n--------\n\
         Quality (Caster Modifiers): +20% (augmented)\n--------\nItem Level: 80",
    );
    assert_eq!(highest(&jewel), "20");
    // A quality above the highest of its base widens its field.
    let bow = parse(
        "Item Class: Bows\nRarity: Normal\nGemini Bow\n--------\n\
         Quality: +45% (augmented)\n--------\nItem Level: 80",
    );
    assert_eq!(highest(&bow), "45");
    assert_eq!(
        highest(&parse(include_str!(
            "../../../item/test-items/en-owner-rare-amulet-catalyst.txt"
        ))),
        "50"
    );
    assert_eq!(
        highest(&parse(include_str!(
            "../../../item/test-items/en-owner-rare-ring-catalyst.txt"
        ))),
        "75"
    );
    // A base the game data doesn't know: the highest quality of any item.
    assert_eq!(highest(&normal("Rings", "Mystery Ring")), "75");
}

#[test]
fn the_quality_of_a_charm_counts_from_10_and_of_a_flask_from_20() {
    let quality = |class: &str, name: &str, base: &str, quality: u32| {
        let item = parse(&format!(
            "Item Class: {class}\nRarity: Magic\n{name}\n--------\n\
             Quality: +{quality}% (augmented)\n--------\nItem Level: 60\n--------\n\
             {{ Suffix Modifier }}\n+10% to Fire Resistance"
        ));
        item_group_of(&item, Some(base))
            .into_iter()
            .find(|text| text.contains("Quality"))
    };
    let charm = |value| quality("Charms", "Ruby Charm of the Fox", "Ruby Charm", value);
    let flask = |value| {
        quality(
            "Life Flasks",
            "Ultimate Life Flask of the Fox",
            "Ultimate Life Flask",
            value,
        )
    };

    assert_eq!(charm(5).as_deref(), Some("□ Quality: 5 ≥5"));
    assert_eq!(charm(10).as_deref(), Some("■ Quality: 10 ≥10"));
    assert_eq!(flask(15), None);
    assert_eq!(flask(20).as_deref(), Some("□ Quality: 20 ≥20"));
    assert_eq!(flask(23).as_deref(), Some("■ Quality: 23 ≥23"));
}

#[test]
fn a_waystone_is_searched_by_its_properties_and_its_tier() {
    // The owner's waystones of tier 12: the base without its tier, a bar of the tier under it;
    // the properties in a group of their own, the item rarity and the drop chance checked, a
    // percentage from 90 % of its value, rounded down, the revives from their number.
    let english = parse(include_str!(
        "../../../item/test-items/en-owner-rare-waystone.txt"
    ));
    let french = parse(include_str!(
        "../../../item/test-items/fr-rare-waystone.txt"
    ));
    let properties = |texts: Vec<String>| -> Vec<String> {
        texts
            .into_iter()
            .take_while(|text| !text.starts_with("Prefixes"))
            .collect()
    };

    assert_eq!(
        properties(texts(&english)),
        [
            "Item",
            "■ Base: Waystone [Waystone T12]",
            "□ ItemLevel: 80 ≥80",
            "Waystone",
            "□ Waystone(Revives): 3 ≥3",
            "■ Waystone(ItemRarity): 11 ≥9",
            "□ Waystone(MonsterRarity): 25 ≥22",
            "□ Waystone(MonsterEffectiveness): 13 ≥11",
            "■ Waystone(DropChance): 50 ≥45",
        ]
    );
    assert_eq!(
        properties(texts(&french)),
        [
            "Item",
            "■ Base: Pierre de téléportation [Waystone T12]",
            "□ ItemLevel: 78 ≥78",
            "Waystone",
            "□ Waystone(Revives): 2 ≥2",
            "■ Waystone(ItemRarity): 25 ≥22",
            "□ Waystone(PackSize): 9 ≥8",
            "□ Waystone(MonsterRarity): 23 ≥20",
            "■ Waystone(DropChance): 65 ≥58",
        ]
    );

    // A corrupted waystone raised to tier 16, without revives left: a property of zero
    // searches nothing.
    let corrupted = parse(
        "Item Class: Waystones\nRarity: Normal\nWaystone (Tier 16)\n--------\n\
         Revives Available: 0 (augmented)\nItem Rarity: +38% (augmented)\n--------\n\
         Item Level: 82\n--------\nCorrupted",
    );
    assert_eq!(
        properties(texts(&corrupted)),
        [
            "Item",
            "■ Base: Waystone [Waystone T16]",
            "■ ItemLevel: 82 ≥82",
            "Waystone",
            "■ Waystone(ItemRarity): 38 ≥34",
        ]
    );
}

#[test]
fn a_waystone_searches_its_base_and_its_checked_properties() {
    // From the copied waystone to the query sent: the whole base type, with its tier; the
    // item rarity and the drop chance among the endgame filters; no modifier.
    for (text, base_type, item_rarity, drop_chance) in [
        (
            include_str!("../../../item/test-items/en-owner-rare-waystone.txt"),
            "Waystone (Tier 12)",
            9.0,
            45.0,
        ),
        (
            include_str!("../../../item/test-items/fr-rare-waystone.txt"),
            "Pierre de téléportation (Palier 12)",
            22.0,
            58.0,
        ),
    ] {
        let item = parse(text);
        let lines = CopiedLines::new(
            item.language(),
            item.modifiers(),
            &LineStats::default(),
            None,
        );
        let line_stats = LineStats::new(&item, item.modifiers(), None);
        let tiers = TierRolls::new(
            item.language(),
            item.modifiers(),
            Some(base_type),
            item.item_level(),
        );
        let stats = Stats::new(&item, &lines, &line_stats, &tiers, Some(base_type));
        let target = link::target(&item, None, None).expect("a target");
        let query = |filters: &Filters| match link::query(
            &item,
            &target,
            line_stats.search_filters(&stats.filters(filters)),
        ) {
            crate::trade::query::Query::Items(items) => items.to_json(),
            crate::trade::query::Query::Exchange(_) => panic!("an item search"),
        };
        let mut filters = Filters::default();

        let json = query(&filters);
        assert_eq!(json["query"]["type"], base_type);
        assert_eq!(
            json["query"]["filters"]["map_filters"],
            serde_json::json!({ "filters": {
                "map_iir": { "min": item_rarity },
                "map_bonus": { "min": drop_chance }
            } })
        );
        assert!(json["query"].get("stats").is_none(), "{json}");

        // The base unchecked: any waystone, of any tier.
        let (_, mut base) = stats
            .filters(&filters)
            .into_iter()
            .find(|(key, _)| *key == FilterKey::BaseType)
            .expect("the filter of the base");
        base.checked = false;
        filters.set(FilterKey::BaseType, base);
        let json = query(&filters);
        assert!(json["query"].get("type").is_none(), "{json}");
        assert_eq!(
            json["query"]["filters"]["type_filters"]["filters"]["category"],
            serde_json::json!({ "option": "map.waystone" })
        );
    }
}

#[test]
fn the_filter_of_a_waystone_property_writes_its_value_as_the_game() {
    let english = Translator::new(Language::English);
    let french = Translator::new(Language::French);
    let text = |property, value, i18n| {
        StatText::Property(Property::Waystone(property), value)
            .text(i18n)
            .into_owned()
    };

    assert_eq!(
        text(WaystoneProperty::ItemRarity, 11.0, &english),
        "Item rarity: +11%"
    );
    assert_eq!(
        text(WaystoneProperty::Revives, 3.0, &english),
        "Revives available: 3"
    );
    assert_eq!(
        text(WaystoneProperty::DropChance, 65.0, &french),
        "Chances de trouver des pierres de téléportation\u{a0}: +65%"
    );
    // A zero, as the tooltip of an offer writes a property it lacks, and a negative value,
    // without a sign of their own.
    assert_eq!(
        text(WaystoneProperty::PackSize, 0.0, &english),
        "Pack size: 0%"
    );
    assert_eq!(
        text(WaystoneProperty::ItemRarity, -5.0, &english),
        "Item rarity: -5%"
    );
    assert_eq!(
        text(WaystoneProperty::Revives, 0.0, &english),
        "Revives available: 0"
    );
}

#[test]
fn a_skill_gem_is_searched_by_its_level_quality_and_sockets() {
    // The owner's gem of level 2: its level checked, its 2 sockets, which every gem has, not.
    for text in [
        include_str!("../../../item/test-items/en-owner-skill-gem.txt"),
        include_str!("../../../item/test-items/fr-skill-gem.txt"),
    ] {
        assert_eq!(
            texts(&parse(text)),
            ["Gem", "■ GemLevel: 2 ≥2", "□ GemSockets: 2 ≥2"],
            "{text}"
        );
    }
    // The quality checked from 16 %, the sockets from 3, the level always.
    let gem = |level: u32, quality: u32, sockets: usize| {
        let sockets = "G ".repeat(sockets);
        parse(&format!(
            "Item Class: Skill Gems\nRarity: Gem\nComet\n--------\nSpell, AoE, Cold\n\
             Level: {level}\nQuality: +{quality}% (augmented)\n--------\nSockets: {sockets}"
        ))
    };
    assert_eq!(
        texts(&gem(15, 12, 3)),
        [
            "Gem",
            "■ GemLevel: 15 ≥15",
            "□ Quality: 12 ≥12",
            "■ GemSockets: 3 ≥3"
        ]
    );
    assert_eq!(
        texts(&gem(20, 16, 2)),
        [
            "Gem",
            "■ GemLevel: 20 ≥20",
            "■ Quality: 16 ≥16",
            "□ GemSockets: 2 ≥2"
        ]
    );
    // The fields within the levels, qualities and sockets of a gem, widened to hold a value
    // beyond them.
    let bounds = |item: &Item| -> Vec<(f64, f64)> {
        rows(item)
            .into_iter()
            .filter_map(|row| match row {
                Row::Stat(stat) => {
                    let bounds = stat.filter?.limit?.bounds?;
                    Some((bounds.min, bounds.max))
                }
                _ => None,
            })
            .collect()
    };
    assert_eq!(
        bounds(&gem(21, 23, 5)),
        [(1.0, 21.0), (0.0, 23.0), (0.0, 5.0)]
    );
    assert_eq!(
        bounds(&gem(22, 25, 6)),
        [(1.0, 22.0), (0.0, 25.0), (0.0, 6.0)]
    );
    // A quality of zero searches nothing: no filter.
    assert_eq!(
        texts(&gem(15, 0, 2)),
        ["Gem", "■ GemLevel: 15 ≥15", "□ GemSockets: 2 ≥2"]
    );
}

#[test]
fn the_search_of_a_skill_gem_sends_its_name_and_its_checked_properties() {
    for (text, name) in [
        (
            include_str!("../../../item/test-items/en-owner-skill-gem.txt"),
            "Flame Wall",
        ),
        (
            include_str!("../../../item/test-items/fr-skill-gem.txt"),
            "Mur de flammes",
        ),
    ] {
        let item = parse(text);
        let lines = CopiedLines::default();
        let line_stats = LineStats::new(&item, item.modifiers(), None);
        let tiers = TierRolls::default();
        let target = link::target(&item, None, None).expect("a target");
        assert_eq!(target, link::TradeTarget::Base { base_type: name });
        let stats = Stats::new(&item, &lines, &line_stats, &tiers, target.base_type());
        let crate::trade::query::Query::Items(query) = link::query(
            &item,
            &target,
            line_stats.search_filters(&stats.filters(&Filters::default())),
        ) else {
            panic!("an item search");
        };
        let json = query.to_json();

        // Its level, checked; its 2 sockets, unchecked, aren't sent; no base type filter.
        assert_eq!(json["query"]["type"], name, "{json}");
        assert_eq!(
            json["query"]["filters"]["misc_filters"],
            serde_json::json!({ "filters": {
                "gem_level": { "min": 2.0 },
                "corrupted": { "option": "false" }
            } }),
            "{json}"
        );
        assert_eq!(
            json["query"]["filters"]["type_filters"],
            serde_json::json!({ "filters": { "rarity": { "option": "nonunique" } } }),
            "{json}"
        );
    }
}

#[test]
fn a_support_gem_without_level_has_no_filter() {
    let support = "Item Class: Support Gems\nRarity: Gem\nFire Penetration I\n--------\n\
                   Support, Fire\n--------\nRequires: Level 5";
    assert_eq!(texts(&parse(support)), Vec::<String>::new());
}

/// A rare item of `base`, of the item `class`, as the advanced copy writes it: its `implicit`
/// line under its header, then each of its modifier `lines` under the header of a prefix (what
/// a filter compares doesn't depend on the affix).
fn advanced_rare(base: &str, class: &str, implicit: &str, lines: &str) -> Item {
    let modifiers = lines
        .lines()
        .map(|line| format!("{{ Prefix Modifier }}\n{line}"))
        .collect::<Vec<_>>()
        .join("\n");
    parse(&format!(
        "Item Class: {class}\nRarity: Rare\nGale Coil\n{base}\n--------\n\
         Item Level: 80\n--------\n{{ Implicit Modifier }}\n{implicit}\n--------\n{modifiers}"
    ))
}

/// A line no pseudo total nor DPS counts, to check on a rare.
const RARITY: &str = "16% increased Rarity of Items found";

#[test]
fn a_total_that_the_lines_of_the_base_reach_compares_nothing() {
    use crate::ui::StatChecks::{Checked, Unchecked};

    // A Prismatic Ring, its implicit +8(7-10)% to all Elemental Resistances, with `lines`.
    let ring = |lines: &str| {
        let implicit = "+8(7-10)% to all Elemental Resistances";
        advanced_rare("Prismatic Ring", "Rings", implicit, lines)
    };
    // Its resistance from its implicit only, which every Prismatic Ring has.
    let alone = ring(&format!("+125 to Evasion Rating\n{RARITY}"));
    assert_eq!(default_checks(&alone), Unchecked);
    // A small resistance on top, 32 in all: its minimum of 29 within the 30 of the best roll
    // of the implicit.
    let small = ring(&format!("+8% to Cold Resistance\n{RARITY}"));
    assert_eq!(default_checks(&small), Unchecked);
    // 34 in all: its minimum of 31 beyond the best roll, within the 36 it gives with the
    // usual quality of a catalyst, which any Prismatic Ring may have.
    let catalysed = ring(&format!("+10% to Cold Resistance\n{RARITY}"));
    assert_eq!(default_checks(&catalysed), Unchecked);
    assert_eq!(default_checks(&ring("+40% to Cold Resistance")), Checked);
    // Copied from the in-game trade, its implicit without range: its best roll unknown, so
    // that no resistance on top compares it.
    let traded = rare_item(
        "Prismatic Ring",
        "Rings",
        "",
        &format!(
            "+8% to all Elemental Resistances (implicit)\n--------\n+40% to Cold Resistance\n{RARITY}"
        ),
    );
    assert_eq!(default_checks(&traded), Unchecked);
    // In the advanced copy, a number without range is a fixed value, its best roll: 10 of
    // Strength, 12 with a catalyst, under the minimum of 20 of its total of 22.
    let fixed = advanced_rare(
        "Prismatic Ring",
        "Rings",
        "+10 to Strength",
        "+12 to Strength",
    );
    assert_eq!(default_checks(&fixed), Checked);
    // The quality of an armour raises its defences, not its implicit: 80 of Life at its best
    // roll, under the minimum of 87 of its total of 97 (96 with a catalyst).
    let armour = advanced_rare(
        "Ancestral Mail",
        "Body Armours",
        "+65(60-80) to maximum Life",
        "+32 to maximum Life",
    );
    assert_eq!(default_checks(&armour), Checked);
    // A rune goes on any item of the base: a total it alone gives compares nothing.
    let helmet = rare_item(
        "Hunter Hood",
        "Helmets",
        "",
        &format!("+12% to Fire Resistance (rune)\n--------\n{RARITY}"),
    );
    assert_eq!(default_checks(&helmet), Unchecked);
}

#[test]
fn a_dps_that_the_base_of_a_weapon_reaches_compares_nothing() {
    use crate::ui::StatChecks::{Checked, Unchecked};

    let maul = |top: &str, lines: &str| {
        rare_item(
            "Temple Maul",
            "Two Hand Maces",
            &format!("{top}\n--------\n"),
            lines,
        )
    };
    // A small line of physical damage: its base reaches the minimum at 20 % quality.
    let small = maul(
        "Physical Damage: 10-20 (augmented)\nAttacks per Second: 1.00",
        &format!("20% increased Physical Damage\n{RARITY}"),
    );
    assert_eq!(default_checks(&small), Unchecked);
    // An attack speed of its base, implicit: every Temple Maul has it. Its DPS reaches
    // 20 × 1.2 × 1.10 = 26.4, beyond the minimum of 25 (24 with its implicit left out).
    let fast = maul(
        "Physical Damage: 20-32 (augmented)\nAttacks per Second: 1.10 (augmented)",
        &format!(
            "10% increased Attack Speed (implicit)\n--------\n\
             30% increased Physical Damage\n{RARITY}"
        ),
    );
    assert_eq!(default_checks(&fast), Unchecked);

    let projectiles = "+3 to Level of all Projectile Skills";
    let bow = |top: &str, lines: &str| {
        let top = format!("Physical Damage: 40-90\n{top}Attacks per Second: 1.20\n--------\n");
        rare_item("Gemini Bow", "Bows", &top, lines)
    };
    // Without a line of damage nor of attack speed: the DPS of its base.
    assert_eq!(default_checks(&bow("", projectiles)), Unchecked);
    // A rune goes on any bow: the elemental DPS it alone gives compares nothing.
    let runed = bow(
        "Fire Damage: 5-10 (fire)\n",
        &format!("Adds 5 to 10 Fire Damage (rune)\n--------\n{projectiles}"),
    );
    assert_eq!(default_checks(&runed), Unchecked);
    // A strong rune and a small modifier: 31 of elemental damage a hit, 37.2 a second, its
    // minimum of 33 within the 34.8 the bow reaches with its rune alone.
    let strong_rune = bow(
        "Fire Damage: 26-32 (fire)\nCold Damage: 1-3 (cold)\n",
        &format!(
            "Adds 26 to 32 Fire Damage (rune)\n--------\nAdds 1 to 3 Cold Damage\n{projectiles}"
        ),
    );
    assert_eq!(default_checks(&strong_rune), Unchecked);
    // As much from a modifier as from the rune: 58 a hit, its minimum of 62 beyond 34.8.
    let strong_modifier = bow(
        "Fire Damage: 26-32 (fire)\nCold Damage: 26-32 (cold)\n",
        "Adds 26 to 32 Fire Damage (rune)\n--------\nAdds 26 to 32 Cold Damage",
    );
    assert_eq!(default_checks(&strong_modifier), Checked);
}
