//! The stat rows of the item block as a whole, on the test items and on items built for a
//! case: the groups and their order, the notes, the active filters and the filters checked by
//! default.

use super::*;

use crate::filters::StatKey;
use crate::item::ItemState;
use crate::item::trade_copy::as_in_game_trade;
use crate::ui::item_stats::fixtures::{
    base_type, default_checks, explicit_rows, parse, rare_item, read_again, rows, texts,
    with_soul_tether_rolls,
};

#[test]
fn the_note_shows_also_when_the_only_tiers_found_are_those_of_a_line_in_doubt() {
    // A helmet with one modifier, which a prefix and a suffix both roll.
    let helmet = read_again(parse(
        "Item Class: Helmets\nRarity: Rare\nChimeric Visage\nAncestral Tiara\n--------\n\
         Item Level: 82\n--------\n18% increased Rarity of Items found",
    ));

    let texts = texts(&helmet);
    assert_eq!(
        texts[texts.len() - 3..],
        [
            "Explicit",
            "□ 18% increased Rarity of Items found ≥16 [T1 16–19 AffixPrefix / T1 15–18 AffixSuffix]",
            "Trade copy note",
        ]
    );
}

#[test]
fn a_sanctified_item_copied_from_the_in_game_trade_shows_no_tier_nor_the_note() {
    let text = include_str!("../../item/test-items/en-owner-trade-rare-helmet-fractured.txt");
    let tiara = read_again(parse(&format!(
        "{}\n--------\nSanctified\n",
        text.trim_end()
    )));
    assert!(tiara.has_state(crate::item::ItemState::Sanctified));

    // Its values can each have left their tier: every line as copied, without affix, tier
    // nor bar, and no note, no tier being found.
    let texts = texts(&tiara);
    let explicit = texts
        .iter()
        .position(|text| text == "Explicit")
        .expect("its explicit modifiers");
    assert_eq!(
        texts[explicit..],
        [
            "Explicit",
            "□ 41% increased Energy Shield ≥37 [Fractured]",
            "□ +34 to maximum Mana ≥31 [Fractured]",
            "□ 98% increased Energy Shield ≥88",
            "□ 18% increased Rarity of Items found ≥16",
            "□ +43% to Fire Resistance ≥39",
            "□ +40% to Cold Resistance ≥36",
            "□ +69 to maximum Energy Shield ≥62 [Desecrated]",
        ]
    );
}

#[test]
fn only_the_tiers_found_from_the_values_have_the_note() {
    // Copied from the in-game trade, not read again yet: no tier found.
    let tiara = parse(include_str!(
        "../../item/test-items/en-owner-trade-rare-helmet-fractured.txt"
    ));
    assert!(!texts(&tiara).contains(&"Trade copy note".to_owned()));
    for text in [
        include_str!("../../item/test-items/en-rare-crossbow-all-modifiers.txt"),
        include_str!("../../item/test-items/en-owner-trade-unique-belt-mutated.txt"),
        include_str!("../../item/test-items/en-normal-helmet.txt"),
    ] {
        let item = parse(text);

        assert!(
            !texts(&item).contains(&"Trade copy note".to_owned()),
            "{text}"
        );
    }
}

#[test]
fn a_unique_copied_from_the_in_game_trade_shows_the_rolls_of_poe2scout_and_a_note() {
    let belt = with_soul_tether_rolls(parse(include_str!(
        "../../item/test-items/en-owner-trade-unique-belt-mutated.txt"
    )));

    assert_eq!(
        texts(&belt),
        [
            "Enchant",
            "□ 15% increased Evasion Rating ≥15",
            "Implicit",
            "■ 16% increased Charm Effect Duration ≥16 [15–20 0.20]",
            // poe2scout writes the plural.
            "■ Has 3 Charm Slots ≥3 [1–3 1.00]",
            "Unique",
            // The Vaal modifier, which poe2scout never holds.
            "□ 14% increased Energy Shield Recovery rate ≥14 [Vaal]",
            "■ +51 to maximum Energy Shield ≥51 [40–60 0.55]",
            // `(5-5)`: it never varies.
            "You lose 5% of maximum Energy Shield per second",
            "Excess Life Recovery from Leech is applied to Energy Shield",
            "Unique rolls note",
        ]
    );
}

#[test]
fn a_rare_shows_its_enchantment_runes_implicit_prefixes_and_suffixes() {
    let crossbow = parse(include_str!(
        "../../item/test-items/en-rare-crossbow-all-modifiers.txt"
    ));

    assert_eq!(
        texts(&crossbow),
        [
            // The base type checked, the item level unchecked at the level of the item.
            "Item",
            "■ Base: Gemini Crossbow",
            "□ ItemLevel: 80 ≥80",
            // Corrupted: no socket can be added any more.
            "■ Sockets: 2 ≥2",
            // 90 % of the DPS, the elemental one unchecked.
            "Weapon",
            "■ PhysicalDps: 288 ≥259",
            "□ ElementalDps: 267 ≥240",
            "Enchant",
            // The enchantment and the runes unchecked, with their value.
            "□ 45% increased Elemental Damage with Attacks ≥45",
            "Runes",
            "□ 18% increased Physical Damage ≥18",
            "□ Gain 30 Mana per enemy killed ≥30",
            "Implicit",
            // A line without number: no minimum.
            "□ Loads an additional bolt",
            "Prefixes 3/3",
            // 90 % of the copied value, of the average of two numbers; the damage and
            // the attack speed of the weapon unchecked, since its DPS counts them. The bar
            // over every tier of the crossbow: of two numbers, the average of where each
            // falls (10 in 1–19, 273 in 7–358). Its origin after the range of its tier.
            "□ Adds 10 to 273 Lightning Damage ≥127 [0.63 T2 1–16 · 239–300 Fractured]",
            "□ Adds 24 to 51 Physical Damage ≥34 [0.47 T3 23–35 · 39–59]",
            // Not a damage of the weapon: its attacks get it from any item.
            "□ 109% increased Elemental Damage with Attacks ≥98 [0.71 T2 100–119]",
            "Suffixes 3/3",
            // Copied before a tier was added above it: tier 1 then, 7.59 in 5–8.9 now.
            "□ Leeches 7.59% of Physical Damage as Mana ≥6.83 [0.66 T1 7–7.9]",
            "□ Gain 21 Mana per enemy killed ≥19 [0.44 T3 21–27 Desecrated]",
            "□ 18% increased Attack Speed ≥16 [0.93 T1 17–19]",
        ]
    );
}

#[test]
fn a_unique_has_no_pseudo_totals() {
    let unique = parse(
        "Item Class: Rings\nRarity: Unique\nTest Loop\nGold Ring\n--------\n\
         Item Level: 80\n--------\n{ Unique Modifier }\n+52(40-60) to maximum Life\n\
         { Unique Modifier }\n+30(25-35)% to Fire Resistance",
    );

    assert_eq!(
        texts(&unique),
        [
            "Unique",
            "■ +52 to maximum Life ≥52 [40–60 0.60]",
            "■ +30% to Fire Resistance ≥30 [25–35 0.50]",
        ]
    );
}

#[test]
fn an_unrevealed_modifier_is_a_line_of_its_group_and_unchecks_the_other_stats() {
    let english = parse(include_str!(
        "../../item/test-items/en-owner-rare-focus-unrevealed.txt"
    ));
    let french = parse(include_str!(
        "../../item/test-items/fr-rare-focus-unrevealed.txt"
    ));
    assert_eq!(
        texts(&english),
        [
            "Item",
            "■ Base: Plumed Focus",
            "□ ItemLevel: 39 ≥39",
            "□ RequiredLevel: 36 ≤36",
            // Searched with its number as minimum.
            "■ UnrevealedModifiers: 1 ≥1",
            "Pseudo",
            "□ ElementalResistance: 42 ≥38",
            // Counted in the fill, last of its group, without filter.
            "Prefixes 3/3",
            "□ 52% increased Spell Damage ≥47 [0.42 T4 45–54]",
            "□ +36 to maximum Energy Shield ≥32 [0.32 T6 36–41]",
            "Unrevealed Prefix",
            "Suffixes 2/3",
            "□ +24% to Cold Resistance ≥22 [0.46 T5 21–25]",
            "□ +18% to Fire Resistance ≥16 [0.31 T6 16–20]",
            "Unrevealed note",
        ]
    );
    let french = texts(&french);
    assert_eq!(french[4], "■ UnrevealedModifiers: 1 ≥1");
    assert_eq!(french[10], "Unrevealed Prefix");
    assert_eq!(french.last().map(String::as_str), Some("Unrevealed note"));
}

#[test]
fn a_weapon_with_an_unrevealed_modifier_starts_with_its_dps_unchecked() {
    let weapon = parse(
        "Item Class: Two Hand Maces\nRarity: Rare\nStorm Fall\nTemple Maul\n--------\n\
         Physical Damage: 10-20\nLightning Damage: 1-59 (lightning)\n\
         Attacks per Second: 1.00\n--------\nItem Level: 80\n--------\n\
         { Prefix Modifier \"Veiled\" }\nDesecrated Prefix",
    );
    assert_eq!(
        texts(&weapon),
        [
            "Item",
            "■ Base: Temple Maul",
            "□ ItemLevel: 80 ≥80",
            "■ UnrevealedModifiers: 1 ≥1",
            "Weapon",
            // An unrevealed prefix can add damage.
            "□ PhysicalDps: 15 ≥13",
            "□ ElementalDps: 30 ≥27",
            "Prefixes 1/3",
            "Unrevealed Prefix",
            "Unrevealed note",
        ]
    );
}

#[test]
fn a_corrupted_item_has_no_note_and_a_lit_state_tag_counts_among_the_active_filters() {
    let headhunter = parse(
        "Item Class: Belts\nRarity: Unique\nHeadhunter\nHeavy Belt\n--------\n\
         Item Level: 80\n--------\n{ Unique Modifier }\n+55(40-60) to maximum Life\n\
         --------\nCorrupted",
    );
    let staff = parse(
        "Item Class: Staves\nRarity: Normal\nChiming Staff\n--------\n\
         Item Level: 82\n--------\nCorrupted",
    );
    let stats = |item| {
        Stats::new(
            item,
            &CopiedLines::default(),
            &LineStats::default(),
            &TierRolls::default(),
            base_type(item),
        )
    };

    // The Corrupted tag says what is searched.
    assert_eq!(
        texts(&headhunter),
        ["Unique", "■ +55 to maximum Life ≥55 [40–60 0.75]"]
    );
    let headhunter_stats = Stats {
        item: &headhunter,
        rows: rows(&headhunter),
    };
    let mut filters = Filters::default();
    assert_eq!(headhunter_stats.active_filters(&filters), 1);
    filters.toggle_state(ItemState::Corrupted);
    assert_eq!(headhunter_stats.active_filters(&filters), 2);
    // A state the item doesn't have isn't counted.
    filters.toggle_state(ItemState::Mirrored);
    assert_eq!(headhunter_stats.active_filters(&filters), 2);
    // An item without modifiers has its filters to show.
    assert!(!stats(&staff).is_empty());
    assert_eq!(
        texts(&staff),
        ["Item", "■ Base: Chiming Staff", "■ ItemLevel: 82 ≥82"]
    );
    let orb = parse("Item Class: Stackable Currency\nRarity: Currency\nExalted Orb");
    assert!(stats(&orb).is_empty());
}

#[test]
fn a_line_the_trade_site_cant_search_isnt_a_filter() {
    let unique = parse(
        "Item Class: Boots\nRarity: Unique\nStormtread\nLattice Sandals\n--------\n\
         Item Level: 72\n--------\n\
         { Unique Modifier }\n+52(40-60) to maximum Life\n\
         { Unique Modifier }\nGain 3(2-4) Thunder Charges when you Dodge Roll\n\
         { Unique Modifier }\n20(15-25)% chance to gain Onslaught for 3 seconds when you kill an\n\
         enemy affected by Abyssal Wasting\n\
         { Unique Modifier }\nLife that would be lost by taking Damage is instead Reserved\n\
         until you take no Damage to Life for 5(4-6) seconds",
    );
    let lines = CopiedLines::new(
        unique.language(),
        unique.modifiers(),
        &LineStats::default(),
        None,
    );
    let line_stats = LineStats::new(&unique, unique.modifiers(), None);
    let stats = Stats::new(&unique, &lines, &line_stats, &TierRolls::default(), None);

    assert_eq!(
        texts(&unique),
        [
            "Unique",
            "■ +52 to maximum Life ≥52 [40–60 0.60]",
            // Its roll still shows, without a filter.
            "⊘ Gain 3 Thunder Charges when you Dodge Roll [2–4 0.50]",
            // A stat on two lines is one row, searched by its first line: its field holds the
            // 20, not the 3 its string writes.
            "■ 20% chance to gain Onslaught for 3 seconds when you kill an / enemy affected by \
             Abyssal Wasting ≥20 [15–25 0.50]",
            // Or by the line with its numbers.
            "■ Life that would be lost by taking Damage is instead Reserved / until you take no \
             Damage to Life for 5 seconds ≤5 [4–6 0.50]",
        ]
    );
    let filters = Filters::default();
    assert_eq!(stats.active_filters(&filters), 3);
    let checked: Vec<FilterKey> = stats
        .checked(&filters)
        .into_iter()
        .map(|(key, _)| key)
        .collect();
    let key = |modifier, line| FilterKey::Stat(StatKey { modifier, line });
    assert_eq!(checked, [key(0, 0), key(2, 0), key(3, 1)]);
}

#[test]
fn the_active_filters_count_the_choices_of_the_player() {
    let crossbow = parse(include_str!(
        "../../item/test-items/en-rare-crossbow-all-modifiers.txt"
    ));
    let mut filters = Filters::default();

    let lines = CopiedLines::new(
        crossbow.language(),
        crossbow.modifiers(),
        &LineStats::default(),
        None,
    );
    let line_stats = LineStats::new(&crossbow, crossbow.modifiers(), None);
    let tiers = TierRolls::new(
        crossbow.language(),
        crossbow.modifiers(),
        base_type(&crossbow),
        crossbow.item_level(),
    );
    let stats = Stats::new(&crossbow, &lines, &line_stats, &tiers, base_type(&crossbow));
    let keys: Vec<FilterKey> = stats
        .rows
        .iter()
        .filter_map(|row| match row {
            Row::Stat(stat) => Some(stat.key),
            _ => None,
        })
        .collect();
    let choice = |checked| Filter {
        checked,
        minimum: String::new(),
        maximum: String::new(),
        bounds: None,
    };

    // The base type, the sockets and the physical DPS; the item level, the elemental DPS,
    // the enchantment, the two runes, the implicit and the modifiers of a rare unchecked.
    assert_eq!(stats.active_filters(&filters), 3);
    // A rune checked by the player.
    filters.set(keys[6], choice(true));
    assert_eq!(stats.active_filters(&filters), 4);
    // The implicit checked too.
    filters.set(keys[8], choice(true));
    assert_eq!(stats.active_filters(&filters), 5);
    // Another item copied: its own filters.
    filters = Filters::default();
    assert_eq!(stats.active_filters(&filters), 3);
}

/// The rows of `text`, an advanced copy, and of the same copied from the in-game trade, its
/// tiers found from its values.
fn both(text: &str) -> (Vec<String>, Vec<String>) {
    (
        texts(&parse(text)),
        texts(&read_again(parse(
            &as_in_game_trade(text).expect("a known copy"),
        ))),
    )
}

#[test]
fn the_owners_rare_items_show_the_same_rows_from_the_in_game_trade() {
    // A fractured suffix among modifiers of several tiers: the same rows from the in-game
    // trade, every tier found from its value.
    let mace = [
        "Prefixes 3/3",
        "□ Adds 12 to 26 Physical Damage ≥17 [0.21 T6 11–17 · 20–30]",
        "□ Adds 2 to 6 Cold Damage ≥4 [0.01 T10 2–3 · 4–6]",
        "□ 68% increased Physical Damage ≥61 [0.20 T6 65–84]",
        "Suffixes 3/3",
        "□ Gain 3 Mana per enemy killed ≥3 [0.02 T8 2–3 Fractured]",
        "□ +12% to Critical Damage Bonus ≥11 [0.13 T5 12–13]",
        "□ +3 to Level of all Melee Skills ≥3 [0.33 T3]",
    ];
    let (advanced, in_game_trade) = both(include_str!(
        "../../item/test-items/en-owner-rare-two-hand-mace-fractured.txt"
    ));
    assert_eq!(explicit_rows(advanced), mace);
    assert_eq!(
        explicit_rows(in_game_trade),
        [mace.as_slice(), &["Trade copy note"]].concat()
    );

    // The desecrated modifier of each lord, over its range without tier, in both copies and
    // both languages; the stat of Kurgal written on two lines, one row with its bar.
    for (text, desecrated) in [
        (
            include_str!("../../item/test-items/en-owner-rare-helmet-ulaman.txt"),
            "□ 16% increased Critical Damage Bonus ≥14 [0.43 13–20 Desecrated]",
        ),
        (
            include_str!("../../item/test-items/fr-rare-helmet-ulaman.txt"),
            "□ 16% d'Augmentation du Bonus de Dégâts Critiques ≥14 [0.43 13–20 Desecrated]",
        ),
        (
            include_str!("../../item/test-items/en-owner-rare-focus-kurgal.txt"),
            "□ 23% chance when collecting an Elemental Infusion to gain an / additional \
             Elemental Infusion of the same type ≥21 [0.80 15–25 Desecrated]",
        ),
        (
            include_str!("../../item/test-items/fr-rare-focus-kurgal.txt"),
            "□ Lorsque vous collectez une Infusion élémentaire, vous avez 23% de chances de \
             gagner une / Infusion élémentaire supplémentaire du même type ≥21 [0.80 15–25 \
             Desecrated]",
        ),
        (
            include_str!("../../item/test-items/en-owner-rare-helmet-amanamu.txt"),
            "□ 35% increased Presence Area of Effect ≥32 [1.00 25–35 Desecrated]",
        ),
        (
            include_str!("../../item/test-items/fr-rare-helmet-amanamu.txt"),
            "□ Votre Présence a 35% d'Augmentation de Zone d'effet ≥32 [1.00 25–35 Desecrated]",
        ),
    ] {
        let (advanced, in_game_trade) = both(text);
        assert!(advanced.contains(&desecrated.to_owned()), "{advanced:?}");
        assert!(
            in_game_trade.contains(&desecrated.to_owned()),
            "{in_game_trade:?}"
        );
    }
}

#[test]
fn the_owners_unset_ring_and_helmet_of_amanamu_show_their_rows_from_the_in_game_trade() {
    // The French Unset Ring, rare: its implicit without number, then its modifiers.
    let (advanced, in_game_trade) = both(include_str!(
        "../../item/test-items/fr-rare-ring-implicit.txt"
    ));
    let ring = [
        "Implicit",
        "□ Octroie 1 Emplacement d'Aptitude supplémentaire",
        "Prefixes 2/3",
        "□ +210 au Score de Précision ≥189 [0.45 T3 168–236]",
        "□ +79 de Mana maximal ≥71 [0.41 T7 65–79]",
        "Suffixes 3/3",
        "□ +13% à toutes les Résistances Élémentaires ≥12 [0.77 T2 12–14]",
        "□ 49% d'Augmentation du Taux de Régénération de Mana ≥44 [0.66 T3 40–49]",
        "□ +32% à la Résistance au froid ≥29 [0.67 T3 31–35]",
    ];
    let from_implicit = |texts: Vec<String>| -> Vec<String> {
        texts
            .into_iter()
            .skip_while(|text| text != "Implicit")
            .collect()
    };
    assert_eq!(from_implicit(advanced), ring);
    assert_eq!(
        from_implicit(in_game_trade),
        [ring.as_slice(), &["Trade copy note"]].concat()
    );

    // From the in-game trade, the rarity of the helmet of Amanamu, at the top of a prefix and
    // the bottom of a suffix, with room for both: in doubt.
    let (_, in_game_trade) = both(include_str!(
        "../../item/test-items/en-owner-rare-helmet-amanamu.txt"
    ));
    assert_eq!(
        explicit_rows(in_game_trade),
        [
            "Prefixes 2/3",
            "□ +94 to maximum Mana ≥85 [0.60 T3 90–104]",
            "□ 69% increased Armour and Energy Shield ≥62 [0.64 T3 68–79]",
            "Suffixes 2/3",
            "□ +28% to Cold Resistance ≥25 [0.56 T4 26–30]",
            "□ 35% increased Presence Area of Effect ≥32 [1.00 25–35 Desecrated]",
            "Explicit",
            "□ 15% increased Rarity of Items found ≥14 [T2 12–15 AffixPrefix / T1 15–18 \
             AffixSuffix]",
            "Trade copy note",
        ]
    );
}

#[test]
fn the_owners_normal_items_unique_charm_and_gems_show_their_rows() {
    // A normal quarterstaff: its rune, unchecked, and its implicit over its range, without it
    // from the in-game trade.
    let (advanced, in_game_trade) = both(include_str!(
        "../../item/test-items/fr-normal-quarterstaff-rune.txt"
    ));
    let quarterstaff = |implicit: &str| {
        [
            "Item",
            "■ Base: Bâton de combat d'écraseur de crânes",
            "■ ItemLevel: 80 ≥80",
            "■ Sockets: 1 ≥1",
            "■ Quality: 25 ≥25",
            "Weapon",
            "■ PhysicalDps: 148 ≥132",
            "□ ElementalDps: 19 ≥16",
            "Runes",
            "□ Ajoute 13 à 16 Dégâts de Feu ≥14.5",
            "Implicit",
            implicit,
        ]
        .map(str::to_owned)
    };
    assert_eq!(
        advanced,
        quarterstaff("■ 27% de chances d'Hébéter au Toucher ≥24 [20–50 0.23]")
    );
    assert_eq!(
        in_game_trade,
        quarterstaff("■ 27% de chances d'Hébéter au Toucher ≥24")
    );
    // The implicit of an Unset Ring, without number: searched without limit, in both copies.
    let (advanced, in_game_trade) = both(include_str!(
        "../../item/test-items/en-owner-normal-ring-implicit.txt"
    ));
    assert_eq!(advanced, in_game_trade);
    assert_eq!(
        advanced[advanced.len() - 2..],
        ["Implicit", "■ Grants 1 additional Skill Slot"]
    );

    // A unique charm: its lines of fixed value without filter, its other line over its range;
    // from the in-game trade, unchecked without range until the prices give it, as the other
    // uniques.
    let charm = include_str!("../../item/test-items/en-owner-unique-charm.txt");
    assert_eq!(
        texts(&parse(charm)),
        [
            "Implicit",
            "Used when you become Frozen",
            "Unique",
            "■ 22% Chance to gain a Charge when you kill an enemy ≥22 [20–25 0.40]",
            "Energy Shield Recharge starts on use",
        ]
    );
    assert_eq!(
        texts(&parse(&as_in_game_trade(charm).expect("a known copy"))),
        [
            "Implicit",
            "Used when you become Frozen",
            "Unique",
            "□ 22% Chance to gain a Charge when you kill an enemy ≥22",
            "Energy Shield Recharge starts on use",
        ]
    );

    // An uncut gem, an item of the currency exchange: no filter.
    for text in [
        include_str!("../../item/test-items/en-owner-currency-uncut-gem.txt"),
        include_str!("../../item/test-items/fr-currency-uncut-gem.txt"),
    ] {
        assert_eq!(texts(&parse(text)), Vec::<String>::new(), "{text}");
    }
}

#[test]
fn the_filters_of_an_item_by_default_compare_its_stats_or_ask_for_one() {
    use crate::ui::StatChecks::{Checked, NoneToCheck, Unchecked};

    let file = |name: &str| {
        let (_, text) = crate::item::test_items::ALL
            .iter()
            .find(|(file, _)| *file == name)
            .expect("a test item");
        parse(text)
    };

    for (name, expected) in [
        // Its pseudo totals.
        ("en-owner-rare-ring-breach.txt", Checked),
        // Its DPS, which its damage lines count.
        ("en-owner-rare-two-hand-mace-fractured.txt", Checked),
        // Its item rarity and its drop chance.
        ("en-owner-rare-waystone.txt", Checked),
        // A magic item keeps its modifiers checked.
        ("en-magic-mace.txt", Checked),
        // No pseudo total: every modifier unchecked.
        ("en-owner-rare-jewel-time-lost.txt", Unchecked),
        // Only the count of its unrevealed modifiers checked, which compares nothing.
        ("en-owner-rare-focus-unrevealed.txt", Unchecked),
        // Unidentified: no line to check.
        ("en-plain-magic-jewel-unidentified.txt", NoneToCheck),
        // Unidentified, its implicit only, which comes with the base, and its total.
        ("en-plain-rare-ring-unidentified.txt", NoneToCheck),
        // In French, and copied from the in-game trade, without headers.
        ("fr-rare-two-hand-mace-fractured.txt", Checked),
        ("en-owner-trade-rare-bow-mirrored.txt", Checked),
        ("fr-trade-rare-bow-mirrored.txt", Checked),
    ] {
        assert_eq!(default_checks(&file(name)), expected, "{name}");
    }
}

#[test]
fn an_item_whose_lines_the_trade_site_cant_search_has_nothing_to_check() {
    let unknown = rare_item("Ruby", "Jewels", "", "Strange words of a later patch");
    assert_eq!(default_checks(&unknown), crate::ui::StatChecks::NoneToCheck);
}
