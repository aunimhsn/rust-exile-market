use super::*;
use crate::modifiers::fixtures::{BASES, modifier, repoe};
use crate::modifiers::reduce;
use crate::report::Report;

/// The tables from `modifiers` of `mods.json` on the bases of [`BASES`], those of `essences`
/// among them, each with a line `{0} to Level of all Spell Skills`.
fn essence_tables(modifiers: &str, essences: &[CurrencyModifier]) -> Result<serde_json::Value> {
    let translations = r#"[{"ids": ["spell_skill_gem_level_+"], "English": [{"string": "{0:+d} to Level of all Spell Skills", "index_handlers": [[]]}], "hidden": null}]"#;
    let template = "# to Level of all Spell Skills";
    let (tables, _) = reduce(
        &repoe(modifiers, BASES, translations),
        &[(template.to_owned(), template.to_owned())].into(),
        &[(Currency::Essence, essences)],
        &mut Report::default(),
    )?;
    Ok(serde_json::to_value(tables).expect("written"))
}

/// A modifier of `kind` of `+value to Level of all Spell Skills`, rolling where `spawn`
/// says.
fn spell_levels(kind: &str, level: u32, spawn: &str, value: u32) -> String {
    modifier(
        kind,
        level,
        spawn,
        &format!(r#"[{{"id": "spell_skill_gem_level_+", "min": {value}, "max": {value}}}]"#),
    )
}

#[test]
fn a_modifier_of_an_essence_rolls_on_the_bases_of_its_list_in_a_table_of_its_own() {
    let nowhere = r#"[{"tag": "default", "weight": 0}]"#;
    let modifiers = format!(
        r#"{{"Ordinary1": {}, "Ordinary2": {}, "Essence": {}, "Unlisted": {}}}"#,
        spell_levels(
            "SpellLevel",
            1,
            r#"[{"tag": "bow", "weight": 1}, {"tag": "default", "weight": 0}]"#,
            2
        ),
        spell_levels(
            "SpellLevel",
            40,
            r#"[{"tag": "bow", "weight": 1}, {"tag": "default", "weight": 0}]"#,
            3
        ),
        // Of the same type and value as the ordinary T1, with `default` at 0 alone.
        spell_levels("SpellLevel", 72, nowhere, 3),
        // Another one of the files, which the list doesn't name: on no base.
        spell_levels("Other", 72, nowhere, 5),
    );

    let tables = essence_tables(&modifiers, &[("Essence", &[("bow", 1)])]).expect("tables");

    let summary: Vec<_> = tables
        .as_array()
        .expect("tables")
        .iter()
        .map(|table| {
            let levels: Vec<_> = table["tiers"]
                .as_array()
                .expect("tiers")
                .iter()
                .map(|tier| tier["level"].clone())
                .collect();
            serde_json::json!([table["type"], table["spawn"], table.get("source"), levels])
        })
        .collect();
    // The ordinary table first, the essence's apart, after it.
    assert_eq!(
        summary,
        [
            serde_json::json!(["SpellLevel", [["bow", 1], ["default", 0]], null, [1, 40]]),
            // Of level 1: an essence adds it whatever the level of the item.
            serde_json::json!(["SpellLevel", [["bow", 1], ["default", 0]], "essence", [1]]),
        ]
    );
}

#[test]
fn a_modifier_of_an_essence_missing_rolling_or_mistyped_stops_the_generator() {
    let nowhere = r#"[{"tag": "default", "weight": 0}]"#;
    let on_bows = r#"[{"tag": "bow", "weight": 1}, {"tag": "default", "weight": 0}]"#;
    let error = |spawn: &str, essences: &[CurrencyModifier]| {
        essence_tables(
            &format!(r#"{{"Essence": {}}}"#, spell_levels("Kind", 72, spawn, 3)),
            essences,
        )
        .err()
        .map(|error| error.to_string())
    };

    assert_eq!(
        error(nowhere, &[("Renamed", &[("bow", 1)])]).as_deref(),
        Some("Renamed: a modifier of an essence that RePoE doesn't have")
    );
    // Rolling on a base of its own after a patch: the list is to be checked again.
    assert_eq!(
        error(on_bows, &[("Essence", &[("bow", 1)])]).as_deref(),
        Some("Essence: a modifier of an essence: rolls without an essence")
    );
    // A tag mistyped would leave its bases out without a word.
    assert_eq!(
        error(nowhere, &[("Essence", &[("bwo", 1)])]).as_deref(),
        Some("Essence: a modifier of an essence: a tag no base of the equipment has, bwo")
    );
    assert_eq!(
        error(
            nowhere,
            &[("Essence", &[("bow", 1)]), ("Essence", &[("ring", 1)])]
        )
        .as_deref(),
        Some("Essence: a modifier of an essence: written twice in the list")
    );
}

#[test]
fn a_modifier_of_an_alloy_without_line_stops_the_generator() {
    let modifiers = format!(
        r#"{{"Alloy": {}}}"#,
        modifier(
            "Kind",
            1,
            r#"[{"tag": "default", "weight": 0}]"#,
            r#"[{"id": "spell_skill_gem_level_+", "min": 0, "max": 0}]"#
        )
    );
    let translations = r#"[{"ids": ["spell_skill_gem_level_+"], "English": [{"string": "{0:+d} to Level of all Spell Skills", "index_handlers": [[]]}], "hidden": null}]"#;
    let template = "# to Level of all Spell Skills";
    let error = reduce(
        &repoe(&modifiers, BASES, translations),
        &[(template.to_owned(), template.to_owned())].into(),
        &[(Currency::Alloy, &[("Alloy", &[("bow", 1)])])],
        &mut Report::default(),
    )
    .err()
    .map(|error| error.to_string());

    assert_eq!(
        error.as_deref(),
        Some("Alloy: a modifier of an alloy without line, its values all 0")
    );
}

#[test]
fn a_modifier_of_an_essence_with_a_line_without_template_stops_the_generator() {
    let nowhere = r#"[{"tag": "default", "weight": 0}]"#;
    let modifiers = format!(
        r#"{{"Essence": {}}}"#,
        modifier(
            "Kind",
            72,
            nowhere,
            r#"[{"id": "spell_skill_gem_level_+", "min": 3, "max": 3}, {"id": "other", "min": 1, "max": 1}]"#
        )
    );
    let translations = r#"[
        {"ids": ["spell_skill_gem_level_+"], "English": [{"string": "{0:+d} to Level of all Spell Skills", "index_handlers": [[]]}], "hidden": null},
        {"ids": ["other"], "English": [{"string": "{0} Unknown things", "index_handlers": [[]]}], "hidden": null}
    ]"#;
    let template = "# to Level of all Spell Skills";
    let error = reduce(
        &repoe(&modifiers, BASES, translations),
        &[(template.to_owned(), template.to_owned())].into(),
        &[(Currency::Essence, &[("Essence", &[("bow", 1)])])],
        &mut Report::default(),
    )
    .err()
    .map(|error| error.to_string());

    // Named by its id: two modifiers of an essence may share a type.
    assert_eq!(
        error.as_deref(),
        Some("Essence: a modifier of an essence with a line without template, # Unknown things")
    );
}

/// The one-handed martial weapons, with the bows, as the essences have them.
const ONE_HAND: [&str; 7] = [
    "Bow",
    "Dagger",
    "Flail",
    "One Hand Axe",
    "One Hand Mace",
    "One Hand Sword",
    "Spear",
];

/// The two-handed martial weapons, without the bows.
const TWO_HAND: [&str; 6] = [
    "Crossbow",
    "Talisman",
    "Two Hand Axe",
    "Two Hand Mace",
    "Two Hand Sword",
    "Warstaff",
];

/// The armours, the shields and the foci: the bases of the `armour` tag.
const ARMOUR: [&str; 7] = [
    "Body Armour",
    "Boots",
    "Buckler",
    "Focus",
    "Gloves",
    "Helmet",
    "Shield",
];

/// Checks that each modifier of `list` rolls on every base of the categories `expected`
/// gives it, in the same order, and on no other base of the equipment.
fn assert_rolls_on_classes(list: &[CurrencyModifier], expected: &[(&str, Vec<&str>)]) {
    // The categories of the equipment and the tags of each base, as the generator wrote
    // them; a name of several bases has none.
    let mut categories: BTreeMap<String, Vec<Vec<String>>> = BTreeMap::new();
    for line in include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/game-data/bases-en.ndjson"
    ))
    .lines()
    {
        let base: serde_json::Value = serde_json::from_str(line).expect("a base");
        // Unwritten when it has none.
        let tags: Vec<String> = base
            .get("tags")
            .map(|tags| serde_json::from_value(tags.clone()).expect("its tags"))
            .unwrap_or_default();
        if base.get("domain").is_none() && !tags.is_empty() {
            let category = base["category"].as_str().expect("its category").to_owned();
            categories.entry(category).or_default().push(tags);
        }
    }
    let rolls = |spawn: &[(&str, u32)], tags: &[String]| {
        spawn
            .iter()
            .chain(&[("default", 0)])
            .find(|(tag, _)| tags.iter().any(|base| base == tag))
            .is_some_and(|&(_, weight)| weight > 0)
    };
    assert_eq!(list.len(), expected.len());
    for (&(id, spawn), (expected_id, classes)) in list.iter().zip(expected) {
        assert_eq!(id, *expected_id);
        for (category, bases) in &categories {
            // Every base of a class, or none.
            let on: Vec<bool> = bases.iter().map(|tags| rolls(spawn, tags)).collect();
            let all = classes.contains(&category.as_str());
            assert!(on.iter().all(|&on| on == all), "{id} on {category}: {on:?}");
        }
    }
}

/// The categories of the bases of the equipment, as the generator wrote them.
fn equipment_categories() -> Vec<String> {
    let mut categories: Vec<String> = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/game-data/bases-en.ndjson"
    ))
    .lines()
    .map(|line| serde_json::from_str::<serde_json::Value>(line).expect("a base"))
    .filter(|base| base.get("domain").is_none())
    .map(|base| base["category"].as_str().expect("its category").to_owned())
    .collect();
    categories.sort();
    categories.dedup();
    categories
}

#[test]
fn the_modifiers_of_the_essences_roll_on_the_item_classes_of_craft_of_exile() {
    let one_hand = ONE_HAND.to_vec();
    let two_hand = TWO_HAND.to_vec();
    let martial: Vec<&str> = ONE_HAND.iter().chain(&TWO_HAND).copied().collect();
    let categories = equipment_categories();
    let every: Vec<&str> = categories.iter().map(String::as_str).collect();
    let classes = |classes: &[&'static str]| classes.to_vec();
    // The item classes of each modifier in Craft of Exile (2026-10-30), the bows with the
    // one-handed weapons as the trade site has them.
    let expected: [(&str, Vec<&str>); 33] = [
        ("EssenceAbyssPrefix", every.clone()),
        ("EssenceAbyssSuffix", every.clone()),
        ("EssenceAttackSkillLevel1H1", one_hand.clone()),
        ("EssenceAttackSkillLevel2H1", two_hand.clone()),
        ("EssenceAuraEffect1", classes(&["Sceptre"])),
        ("EssenceBreach", classes(&["Amulet", "Ring"])),
        ("EssenceColdRecoupLife1", classes(&["Helmet"])),
        ("EssenceCorruptForTwoEnchantments1", classes(&["Belt"])),
        ("EssenceDamageasExtraCold1", one_hand.clone()),
        ("EssenceDamageasExtraCold2H", two_hand.clone()),
        ("EssenceDamageasExtraFire1", one_hand.clone()),
        ("EssenceDamageasExtraFire2H", two_hand.clone()),
        ("EssenceDamageasExtraLightning1", one_hand.clone()),
        ("EssenceDamageasExtraLightning2H", two_hand.clone()),
        ("EssenceDamageasExtraPhysical1", one_hand.clone()),
        ("EssenceDamageasExtraPhysical2H", two_hand.clone()),
        ("EssenceFireRecoupLife1", classes(&["Belt"])),
        ("EssenceGlobalDefences1", classes(&["Amulet"])),
        ("EssenceGoldDropped1", classes(&["Gloves"])),
        ("EssenceIncreasedLifePercent1", classes(&["Body Armour"])),
        ("EssenceIncreasedManaPercent1", classes(&["Ring"])),
        ("EssenceLightningRecoupLife1", classes(&["Gloves"])),
        (
            "EssenceLocalRuneAndSoulCoreEffect1",
            classes(&["Boots", "Gloves"]),
        ),
        ("EssenceManaCostReduction", classes(&["Focus", "Wand"])),
        ("EssenceManaCostReduction2H", classes(&["Staff"])),
        ("EssenceOnslaughtonKill1", martial.clone()),
        ("EssencePercentDexterity1", classes(&["Amulet"])),
        ("EssencePercentIntelligence1", classes(&["Amulet"])),
        ("EssencePercentStrength1", classes(&["Amulet"])),
        (
            "EssencePhysicalDamageTakenAsChaos1",
            classes(&["Body Armour"]),
        ),
        (
            "EssenceReducedCriticalDamageAgainstYou1",
            classes(&["Body Armour"]),
        ),
        ("EssenceSpellSkillLevel1H1", classes(&["Wand"])),
        ("EssenceSpellSkillLevel2H1", classes(&["Staff"])),
    ];

    assert_rolls_on_classes(ESSENCES, &expected);
}

#[test]
fn the_modifiers_of_the_alloys_roll_on_the_item_classes_of_poe2db() {
    let martial: Vec<&str> = ONE_HAND.iter().chain(&TWO_HAND).copied().collect();
    let spell = ["Staff", "Wand"];
    let weapons: Vec<&str> = martial
        .iter()
        .chain(&spell)
        .chain(&["Sceptre"])
        .copied()
        .collect();
    let armour = ARMOUR.to_vec();
    let classes = |classes: &[&'static str]| classes.to_vec();
    // The item classes of each modifier on the page of its alloy in poe2db (2026-10-31),
    // the cast speed and cold damage of the staves on the wands and the foci too, as the
    // trade site has them.
    let expected: [(&str, Vec<&str>); 48] = [
        ("AlloyAccuracyAttackSpeedHybrid1", martial.clone()),
        ("AlloyAilmentMagnitude1", martial.clone()),
        ("AlloyArchonDuration1", classes(&["Helmet"])),
        ("AlloyAttackAreaOfEffect1", classes(&["Gloves"])),
        (
            "AlloyAttackSpeedIfMissingWardRecently1",
            classes(&["Gloves"]),
        ),
        ("AlloyAttackSpeedRing1", classes(&["Ring"])),
        (
            "AlloyAttributeIncreasedLocalPhysicalDamageHybrid1",
            martial.clone(),
        ),
        ("AlloyBallistaLimit1", classes(&["Crossbow"])),
        ("AlloyBellLimit1", classes(&["Warstaff"])),
        (
            "AlloyCastSpeedDamageAsExtraColdHybrid1",
            classes(&["Focus", "Staff", "Wand"]),
        ),
        (
            "AlloyCastSpeedDamageAsExtraColdHybridOneHand1",
            classes(&["Focus", "Wand"]),
        ),
        ("AlloyCastSpeedGloves1", classes(&["Gloves"])),
        ("AlloyChanceToChain1", classes(&["Quiver"])),
        (
            "AlloyDamageAsExtraFireTwoHandWhileMissingRunicWard1",
            classes(&["Staff"]),
        ),
        (
            "AlloyDamageAsExtraFireWhileMissingRunicWard1",
            classes(&["Wand"]),
        ),
        ("AlloyDamagingAilmentDuration1", classes(&["Gloves"])),
        (
            "AlloyEffectOfResistanceMods1",
            classes(&["Amulet", "Belt", "Ring"]),
        ),
        ("AlloyEffectOfSocketedAugments1", weapons.clone()),
        ("AlloyElementalPenetration1", classes(&["Gloves"])),
        ("AlloyElementalSkillLimit1", classes(&["Wand"])),
        ("AlloyExposureEffect1", classes(&["Focus", "Staff", "Wand"])),
        ("AlloyFlaskChargesPerSecond1", classes(&["Belt"])),
        ("AlloyLightningDamageIgnites1", classes(&["Talisman"])),
        ("AlloyLocalWardIncreasePercent1", armour.clone()),
        ("AlloyManaCostEfficiency1", classes(&["Helmet"])),
        ("AlloyMarkEffect", classes(&["Bow"])),
        (
            "AlloyMaximumElementalInfusions1",
            classes(&["Sceptre", "Staff", "Wand"]),
        ),
        ("AlloyMaximumRunicWard1", classes(&["Ring"])),
        ("AlloyMaximumRunicWardPercent1", classes(&["Amulet"])),
        ("AlloyMaximumRunicWardWeapon1", weapons.clone()),
        ("AlloyMeleeStrikeRange1", classes(&["Spear"])),
        (
            "AlloyMinionDamagingAilmentMagnitude1",
            classes(&["Sceptre"]),
        ),
        ("AlloyNaturesArchon1", classes(&["Staff"])),
        ("AlloyPresenceAreaOfEffect1", classes(&["Body Armour"])),
        ("AlloyPuppetMasterChance1", classes(&["Sceptre"])),
        ("AlloyPuppeteerStacks1", classes(&["Sceptre"])),
        ("AlloyRecoverRunicWardOnCharmUse1", classes(&["Belt"])),
        ("AlloyReducedSlowPotency1", classes(&["Body Armour"])),
        ("AlloyRemnantPickupRange1", classes(&["Gloves"])),
        (
            "AlloyRetainGlory1",
            classes(&["One Hand Mace", "Two Hand Mace"]),
        ),
        ("AlloyRunicWardOnBlock1", classes(&["Buckler", "Shield"])),
        ("AlloyRunicWardRechargeRate1", classes(&["Belt"])),
        ("AlloySkillEffectDuration1", classes(&["Boots"])),
        ("AlloySpellAreaOfEffect1", classes(&["Helmet"])),
        ("AlloySpellLevelManaHybrid1", classes(&spell)),
        ("AlloySpiritOnBoots1", classes(&["Boots"])),
        ("AlloyTemporaryMinionSkillLimit1", classes(&["Boots"])),
        (
            "AlloyTotemPlacementSpeed1",
            classes(&["Buckler", "Focus", "Shield"]),
        ),
    ];

    assert_rolls_on_classes(ALLOYS, &expected);
}
