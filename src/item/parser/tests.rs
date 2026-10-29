//! The parser on real copied items (see `../test-items/README.md`).

use super::*;
use crate::game_data;
use crate::item::modifier::{Range, Roll, StatLine};
use crate::item::{Affix, Defences, ModifierOrigin};
use crate::language::Language;

macro_rules! test_item {
    ($file:literal) => {
        include_str!(concat!("../test-items/", $file))
    };
}

const FR_RARE_STAFF: &str = test_item!("fr-rare-staff.txt");
const FR_RARE_BODY_ARMOUR: &str = test_item!("fr-rare-body-armour.txt");
const FR_UNIQUE_CROSSBOW: &str = test_item!("fr-unique-crossbow.txt");
const FR_CURRENCY_STACK: &str = test_item!("fr-currency-stack.txt");
const FR_NORMAL_EXCEPTIONAL_SHIELD: &str = test_item!("fr-normal-exceptional-shield.txt");
const FR_UNIQUE_MACE_FIXED_VALUES: &str = test_item!("fr-unique-mace-fixed-values.txt");
const FR_MAGIC_MANA_FLASK: &str = test_item!("fr-magic-mana-flask.txt");
const FR_RARE_SCEPTRE_SANCTIFIED: &str = test_item!("fr-rare-sceptre-sanctified.txt");
const EN_UNIQUE_MACE_FIXED_VALUES: &str = test_item!("en-owner-unique-mace-fixed-values.txt");
const FR_RARE_FOCUS_UNREVEALED: &str = test_item!("fr-rare-focus-unrevealed.txt");
const FR_UNIQUE_RING_MIRRORED: &str = test_item!("fr-unique-ring-mirrored.txt");
const FR_UNIQUE_BELT_VAAL: &str = test_item!("fr-unique-belt-vaal.txt");
const FR_RARE_HELMET_RUNEFORGED: &str = test_item!("fr-rare-helmet-runeforged.txt");
const EN_RARE_HELMET_RUNEFORGED: &str = test_item!("en-owner-rare-helmet-runeforged.txt");
const EN_RARE_SCEPTRE_SANCTIFIED: &str = test_item!("en-owner-rare-sceptre-sanctified.txt");
const EN_RARE_FOCUS_UNREVEALED: &str = test_item!("en-owner-rare-focus-unrevealed.txt");
const EN_RARE_CROSSBOW: &str = test_item!("en-rare-crossbow-all-modifiers.txt");
const EN_RARE_BOW_FRACTURED: &str = test_item!("en-rare-bow-fractured.txt");
const EN_RARE_WAND_UNIDENTIFIED: &str = test_item!("en-rare-wand-unidentified.txt");
const EN_MAGIC_MACE: &str = test_item!("en-magic-mace.txt");
const EN_UNIQUE_FOCUS: &str = test_item!("en-unique-focus.txt");
const EN_NORMAL_HELMET: &str = test_item!("en-normal-helmet.txt");
const EN_CURRENCY_UNCUT_GEM: &str = test_item!("en-currency-uncut-gem.txt");
const EN_PLAIN_RARE_RING: &str = test_item!("en-plain-rare-ring.txt");
const EN_PLAIN_RARE_RING_UNIDENTIFIED: &str = test_item!("en-plain-rare-ring-unidentified.txt");
const EN_PLAIN_UNIQUE_JEWEL: &str = test_item!("en-plain-unique-jewel.txt");
const EN_PLAIN_MAGIC_JEWEL_UNIDENTIFIED: &str = test_item!("en-plain-magic-jewel-unidentified.txt");
const EN_PLAIN_CURRENCY_LARGE_STACK: &str = test_item!("en-plain-currency-large-stack.txt");
const FR_TRADE_RARE_HELMET: &str = test_item!("fr-trade-rare-helmet-fractured.txt");
const FR_TRADE_RARE_BOW: &str = test_item!("fr-trade-rare-bow-mirrored.txt");
const FR_TRADE_UNIQUE_BELT: &str = test_item!("fr-trade-unique-belt-mutated.txt");
const EN_TRADE_RARE_HELMET: &str = test_item!("en-owner-trade-rare-helmet-fractured.txt");
const EN_TRADE_RARE_BOW: &str = test_item!("en-owner-trade-rare-bow-mirrored.txt");
const EN_TRADE_UNIQUE_BELT: &str = test_item!("en-owner-trade-unique-belt-mutated.txt");

const ALL: [&str; 34] = [
    FR_RARE_STAFF,
    FR_RARE_BODY_ARMOUR,
    FR_UNIQUE_CROSSBOW,
    FR_CURRENCY_STACK,
    FR_NORMAL_EXCEPTIONAL_SHIELD,
    FR_UNIQUE_MACE_FIXED_VALUES,
    FR_MAGIC_MANA_FLASK,
    FR_RARE_SCEPTRE_SANCTIFIED,
    FR_RARE_FOCUS_UNREVEALED,
    FR_UNIQUE_RING_MIRRORED,
    FR_UNIQUE_BELT_VAAL,
    FR_RARE_HELMET_RUNEFORGED,
    EN_RARE_HELMET_RUNEFORGED,
    EN_UNIQUE_MACE_FIXED_VALUES,
    EN_RARE_SCEPTRE_SANCTIFIED,
    EN_RARE_FOCUS_UNREVEALED,
    EN_RARE_CROSSBOW,
    EN_RARE_BOW_FRACTURED,
    EN_RARE_WAND_UNIDENTIFIED,
    EN_MAGIC_MACE,
    EN_UNIQUE_FOCUS,
    EN_NORMAL_HELMET,
    EN_CURRENCY_UNCUT_GEM,
    EN_PLAIN_RARE_RING,
    EN_PLAIN_RARE_RING_UNIDENTIFIED,
    EN_PLAIN_UNIQUE_JEWEL,
    EN_PLAIN_MAGIC_JEWEL_UNIDENTIFIED,
    EN_PLAIN_CURRENCY_LARGE_STACK,
    FR_TRADE_RARE_HELMET,
    FR_TRADE_RARE_BOW,
    FR_TRADE_UNIQUE_BELT,
    EN_TRADE_RARE_HELMET,
    EN_TRADE_RARE_BOW,
    EN_TRADE_UNIQUE_BELT,
];

const PREFIX: ModifierKind = ModifierKind::Explicit(Some(Affix::Prefix));
const SUFFIX: ModifierKind = ModifierKind::Explicit(Some(Affix::Suffix));

fn try_parse(text: &str) -> Result<Item, ParseError> {
    parse(&CopiedItem::from_text(text).expect("an item"))
}

fn item(text: &str) -> Item {
    let item = try_parse(text).expect("the item is read");
    assert_eq!(item.warnings(), [], "{text}");
    item
}

fn of_kind(item: &Item, kind: ModifierKind) -> Vec<&Modifier> {
    item.modifiers()
        .iter()
        .filter(|modifier| modifier.kind == kind)
        .collect()
}

fn explicit(item: &Item) -> Vec<&Modifier> {
    item.modifiers()
        .iter()
        .filter(|modifier| matches!(modifier.kind, ModifierKind::Explicit(_)))
        .collect()
}

fn first_line(modifier: &Modifier) -> &StatLine {
    modifier.lines.first().expect("a line")
}

fn roll(value: f64, min: f64, max: f64) -> Roll {
    Roll {
        value,
        range: Some(Range { min, max }),
    }
}

#[test]
fn reads_a_french_rare_staff() {
    let item = item(FR_RARE_STAFF);

    assert_eq!(item.language(), Language::French);
    assert_eq!(item.class(), "Bâtons");
    assert_eq!(item.rarity(), Rarity::Rare);
    // The game writes a space before the name of the rare items.
    assert_eq!(item.name(), Some("Mât sulfureux"));
    assert_eq!(item.base_type(), "Bâton rugissant");
    assert_eq!(
        item.requirements(),
        Requirements {
            level: Some(64),
            intelligence: Some(87),
            ..Requirements::default()
        }
    );
    assert_eq!(item.sockets(), Some(2));
    assert_eq!(item.item_level(), Some(80));
    assert_eq!(item.granted_skills(), ["Niveau 18 Déchaînement"]);
    assert_eq!(item.states(), [ItemState::Corrupted]);

    let runes = of_kind(&item, ModifierKind::Rune);
    assert_eq!(runes.len(), 2);
    assert_eq!(
        first_line(runes[0]).text,
        "30% d'Augmentation des Dégâts des Sorts"
    );

    let explicit = explicit(&item);
    assert_eq!(
        explicit
            .iter()
            .map(|modifier| (modifier.kind, modifier.tier))
            .collect::<Vec<_>>(),
        [
            (PREFIX, Some(1)),
            (PREFIX, Some(1)),
            (PREFIX, Some(3)),
            (SUFFIX, Some(2)),
            (SUFFIX, Some(3)),
            (SUFFIX, Some(3))
        ]
    );
    let storm = explicit[0];
    assert_eq!(storm.name.as_deref(), Some("Lié à la tempête"));
    assert_eq!(storm.tags, ["Dégâts", "Élémentaire", "Foudre"]);
    assert_eq!(first_line(storm).rolls, [roll(55.0, 55.0, 60.0)]);
    assert_eq!(explicit[1].origin, Some(ModifierOrigin::Desecrated));
    assert_eq!(explicit[2].origin, Some(ModifierOrigin::Crafted));
}

#[test]
fn reads_a_french_rare_body_armour() {
    let item = item(FR_RARE_BODY_ARMOUR);

    assert_eq!(item.name(), Some("Manteau du destin"));
    assert_eq!(item.base_type(), "Vêtement gracieux");
    assert_eq!(item.quality(), Some(20));
    assert!(item.properties().contains(&Property {
        name: "Bouclier d'énergie".to_owned(),
        value: "567".to_owned(),
        augmented: true,
    }));
    assert_eq!(
        item.requirements(),
        Requirements {
            level: Some(70),
            intelligence: Some(121),
            ..Requirements::default()
        }
    );

    let implicit = of_kind(&item, ModifierKind::Implicit);
    assert_eq!(implicit.len(), 1);
    assert_eq!(implicit[0].tags, ["Mana"]);

    let explicit = explicit(&item);
    assert_eq!(explicit.len(), 6);
    let regeneration = first_line(explicit[3]);
    assert_eq!(regeneration.text, "24.7 de Vie Régénérée par seconde");
    assert_eq!(regeneration.rolls, [roll(24.7, 23.1, 29.0)]);
    // A range written from the best value: the lowest comes first.
    assert_eq!(first_line(explicit[4]).rolls, [roll(41.0, 41.0, 45.0)]);
}

#[test]
fn reads_a_sanctified_item_and_a_range_of_one_value() {
    for (text, requirements) in [
        (
            EN_RARE_SCEPTRE_SANCTIFIED,
            "29% reduced Attribute Requirements",
        ),
        (
            FR_RARE_SCEPTRE_SANCTIFIED,
            "29% de Réduction du prérequis d'Attributs",
        ),
    ] {
        let item = item(text);
        assert!(item.has_state(ItemState::Sanctified));

        let explicit = explicit(&item);
        // A value out of the range of its tier, as the game writes it.
        assert_eq!(first_line(explicit[1]).rolls, [roll(74.0, 80.0, 89.0)]);
        // `29(30)%`: the range of one value is taken out of the text.
        let line = first_line(explicit[3]);
        assert_eq!(line.text, requirements);
        assert_eq!(line.rolls, [roll(29.0, 30.0, 30.0)]);
    }
}

#[test]
fn reads_the_mirrored_state() {
    let item = item(FR_UNIQUE_RING_MIRRORED);
    assert_eq!(item.name(), Some("Toucher de Kalandra"));
    assert_eq!(item.states(), [ItemState::Mirrored]);
}

#[test]
fn reads_an_unrevealed_modifier_from_its_header_and_its_only_line() {
    for (text, affix) in [
        (EN_RARE_FOCUS_UNREVEALED, Affix::Prefix),
        (FR_RARE_FOCUS_UNREVEALED, Affix::Prefix),
        (EN_RARE_SCEPTRE_SANCTIFIED, Affix::Suffix),
        (FR_RARE_SCEPTRE_SANCTIFIED, Affix::Suffix),
    ] {
        let item = item(text);
        assert_eq!(item.unrevealed_modifiers(), 1);
        let unrevealed = explicit(&item)
            .into_iter()
            .find(|modifier| modifier.unrevealed)
            .expect("an unrevealed modifier");
        assert_eq!(unrevealed.kind, ModifierKind::Explicit(Some(affix)));
        // `Desecrated Prefix` isn't a line of a stat.
        assert_eq!(unrevealed.lines, []);
        assert_eq!(unrevealed.tier, None);
    }
    // The other modifiers are revealed.
    assert_eq!(item(FR_RARE_STAFF).unrevealed_modifiers(), 0);
}

#[test]
fn an_unrevealed_line_under_a_header_without_affix_stays_a_line() {
    // An unknown header gives no affix: `Desecrated Prefix` can't mark it unrevealed.
    let item = try_parse(
        "Item Class: Rings\nRarity: Rare\nWoe Loop\nRuby Ring\n--------\nItem Level: 80\n\
         --------\n{ Unknown Modifier }\nDesecrated Prefix",
    )
    .expect("the item is read");
    assert_eq!(item.warnings(), [ParseWarning::UnknownModifierHeader]);
    assert_eq!(item.unrevealed_modifiers(), 0);
    let modifier = &item.modifiers()[0];
    assert_eq!(modifier.kind, ModifierKind::Explicit(None));
    assert_eq!(first_line(modifier).text, "Desecrated Prefix");
}

#[test]
fn an_unrevealed_modifier_keeps_no_line_after_its_marker() {
    let item = item(
        "Item Class: Rings\nRarity: Rare\nWoe Loop\nRuby Ring\n--------\nItem Level: 80\n\
         --------\n{ Prefix Modifier \"Veiled\" }\nDesecrated Prefix\n+10 to maximum Life",
    );
    let unrevealed = &explicit(&item)[0];
    assert!(unrevealed.unrevealed);
    assert_eq!(unrevealed.lines, []);
}

#[test]
fn reads_a_french_unique_without_its_flavour_text() {
    let item = item(FR_UNIQUE_CROSSBOW);

    assert_eq!(item.rarity(), Rarity::Unique);
    assert_eq!(item.name(), Some("Murmure des brumes"));
    assert_eq!(item.base_type(), "Arbalète de fortune");
    assert_eq!(item.item_level(), Some(78));
    assert_eq!(item.sockets(), None);
    assert!(item.properties().iter().any(|property| {
        property.name == "Dégâts de froid" && property.value == "8-14 (cold)"
    }));

    let explicit = explicit(&item);
    assert_eq!(explicit.len(), 5);
    assert!(
        explicit
            .iter()
            .all(|modifier| modifier.kind == ModifierKind::Explicit(None))
    );
    assert_eq!(item.modifiers().len(), 5);
}

#[test]
fn reads_the_lines_of_fixed_value_without_their_marker() {
    let english = item(EN_UNIQUE_MACE_FIXED_VALUES);
    let texts: Vec<&str> = english
        .modifiers()
        .iter()
        .map(|modifier| first_line(modifier).text.as_str())
        .collect();
    assert_eq!(
        texts,
        [
            "Strikes deal Splash Damage",
            "Adds 34 to 43 Physical Damage",
            "12% chance to cause Bleeding on Hit",
            "Leeches 10% of Physical Damage as Life",
            "Cannot use Warcries",
            "All Attacks count as Empowered Attacks",
        ]
    );

    let item = item(FR_UNIQUE_MACE_FIXED_VALUES);

    let implicit = of_kind(&item, ModifierKind::Implicit);
    assert_eq!(
        first_line(implicit[0]).text,
        "Les Frappes infligent des Dégâts collatéraux"
    );
    let texts: Vec<&str> = explicit(&item)
        .iter()
        .map(|modifier| first_line(modifier).text.as_str())
        .collect();
    assert_eq!(
        texts,
        [
            "Ajoute 34 à 43 Dégâts Physiques",
            "12% de chances d'infliger le Saignement au Toucher",
            "10% des Dégâts Physiques Drainent de la Vie",
            "Vous ne pouvez pas utiliser de Cris de guerre",
            "Toutes les Attaques sont considérées comme des Attaques Améliorées",
        ]
    );
}

#[test]
fn reads_a_french_currency_stack_without_its_description() {
    let item = item(FR_CURRENCY_STACK);

    assert_eq!(item.rarity(), Rarity::Currency);
    assert_eq!(item.name(), None);
    assert_eq!(item.base_type(), "Orbe de transmutation");
    assert_eq!(item.stack(), Some(Stack { size: 40, max: 40 }));
    assert_eq!(item.modifiers(), []);
}

#[test]
fn reads_a_french_exceptional_base() {
    let item = item(FR_NORMAL_EXCEPTIONAL_SHIELD);

    assert_eq!(item.rarity(), Rarity::Normal);
    // "Exceptionnelle" for a feminine base.
    assert_eq!(item.base_type(), "Targe de mammouth");
    assert!(item.is_exceptional());
    assert_eq!(item.sockets(), Some(2));
    assert_eq!(
        item.requirements(),
        Requirements {
            level: Some(65),
            strength: Some(50),
            dexterity: Some(50),
            intelligence: None,
        }
    );
    assert_eq!(item.granted_skills(), ["Levée de bouclier"]);
}

#[test]
fn reads_an_english_item_with_every_kind_of_modifier() {
    let item = item(EN_RARE_CROSSBOW);

    assert_eq!(item.language(), Language::English);
    assert_eq!(
        (item.name(), item.base_type()),
        (Some("Storm Core"), "Gemini Crossbow")
    );
    assert_eq!(item.quality(), Some(20));
    assert_eq!(
        item.requirements(),
        Requirements {
            level: Some(78),
            strength: Some(89),
            dexterity: Some(89),
            intelligence: None,
        }
    );
    assert_eq!(item.sockets(), Some(2));
    assert_eq!(of_kind(&item, ModifierKind::Enchant).len(), 1);
    assert_eq!(of_kind(&item, ModifierKind::Rune).len(), 2);
    let implicit = of_kind(&item, ModifierKind::Implicit);
    assert_eq!(first_line(implicit[0]).text, "Loads an additional bolt");
    assert_eq!(item.granted_skills(), ["Level 18 Cackling Companions"]);

    let explicit = explicit(&item);
    assert_eq!(explicit.len(), 6);
    assert_eq!(explicit[0].origin, Some(ModifierOrigin::Fractured));
    assert_eq!(explicit[4].origin, Some(ModifierOrigin::Desecrated));
    assert_eq!(
        first_line(explicit[0]).text,
        "Adds 10 to 273 Lightning Damage"
    );
    assert!(item.has_state(ItemState::Corrupted));
    assert!(item.has_state(ItemState::Fractured));
}

#[test]
fn reads_the_modifiers_of_several_lines_and_the_unmet_requirements() {
    let item = item(EN_RARE_BOW_FRACTURED);

    assert_eq!(item.requirements().dexterity, Some(163));
    let explicit = explicit(&item);
    assert_eq!(explicit.len(), 6);
    // Without a word in the header, the marker of the line tells the origin.
    assert_eq!(explicit[0].origin, Some(ModifierOrigin::Fractured));
    assert_eq!(explicit[2].lines.len(), 2);
    assert_eq!(explicit[3].tier, None);
}

#[test]
fn reads_an_unidentified_rare_by_its_base_type() {
    let item = item(EN_RARE_WAND_UNIDENTIFIED);

    assert_eq!((item.name(), item.base_type()), (None, "Volatile Wand"));
    assert!(item.has_state(ItemState::Unidentified));
    assert_eq!(item.requirements().intelligence, Some(113));
}

#[test]
fn keeps_the_whole_name_of_a_magic_item() {
    let item = item(EN_MAGIC_MACE);

    assert_eq!(item.rarity(), Rarity::Magic);
    assert_eq!(item.base_type(), "Crackling Temple Maul of the Brute");
    assert_eq!(
        explicit(&item)
            .iter()
            .map(|modifier| modifier.kind)
            .collect::<Vec<_>>(),
        [PREFIX, SUFFIX]
    );
}

#[test]
fn reads_an_english_unique_without_its_flavour_text() {
    let item = item(EN_UNIQUE_FOCUS);

    assert_eq!(
        (item.name(), item.base_type()),
        (Some("The Eternal Spark"), "Crystal Focus")
    );
    assert_eq!(item.modifiers().len(), 5);
}

#[test]
fn removes_exceptional_and_superior_from_the_base_type_in_any_order() {
    for text in [
        "Item Class: Rings\nRarity: Normal\nExceptional Ruby Ring",
        "Item Class: Rings\nRarity: Normal\nSuperior Exceptional Ruby Ring",
        "Item Class: Rings\nRarity: Normal\nExceptional Superior Ruby Ring",
    ] {
        let item = item(text);
        assert_eq!(item.base_type(), "Ruby Ring", "{text}");
        assert!(item.is_exceptional(), "{text}");
    }

    let helmet = item(EN_NORMAL_HELMET);
    assert_eq!(helmet.base_type(), "Divine Crown");
    assert!(!helmet.is_exceptional());
    assert_eq!(helmet.quality(), Some(9));
    assert_eq!(helmet.requirements().strength, Some(67));
}

#[test]
fn reads_a_currency_without_stack_nor_description() {
    let item = item(EN_CURRENCY_UNCUT_GEM);

    assert_eq!(item.rarity(), Rarity::Currency);
    assert_eq!(item.base_type(), "Uncut Skill Gem (Level 19)");
    assert_eq!((item.stack(), item.modifiers().len()), (None, 0));
}

#[test]
fn reads_the_implicit_of_an_unidentified_item_twice_corrupted() {
    let item = item(
        "Item Class: Rings\nRarity: Rare\nRuby Ring\n--------\nRequires: Level 12\n--------\n\
         Item Level: 70\n--------\n{ Implicit Modifier — Elemental, Fire, Resistance }\n\
         +25(20-30)% to Fire Resistance\n--------\nUnidentified\n--------\nTwice Corrupted",
    );

    assert_eq!(item.modifiers().len(), 1);
    assert_eq!(item.modifiers()[0].kind, ModifierKind::Implicit);
    assert!(item.has_state(ItemState::Unidentified));
    assert!(item.has_state(ItemState::TwiceCorrupted));
}

#[test]
fn reads_no_description_as_a_modifier_on_a_plain_unidentified_item() {
    let item = item(EN_PLAIN_MAGIC_JEWEL_UNIDENTIFIED);

    assert!(item.has_state(ItemState::Unidentified));
    assert_eq!(item.modifiers(), []);
}

/// The kinds of the modifiers of an item, in order, each with its origin.
fn kinds(item: &Item) -> Vec<(ModifierKind, Option<ModifierOrigin>)> {
    item.modifiers()
        .iter()
        .map(|modifier| (modifier.kind, modifier.origin))
        .collect()
}

/// The texts of the explicit lines of an item.
fn explicit_texts(item: &Item) -> Vec<&str> {
    explicit(item)
        .into_iter()
        .flat_map(|modifier| modifier.lines.iter().map(|line| line.text.as_str()))
        .collect()
}

#[test]
fn reads_the_items_copied_from_the_in_game_trade() {
    use ModifierKind::{Enchant, Implicit, Rune};
    use ModifierOrigin::{Desecrated, Fractured, Vaal};
    let explicit = ModifierKind::Explicit(None);
    let line = |origin| (explicit, origin);
    let tiara = [
        [(Enchant, None)].as_slice(),
        &[(Rune, None); 4],
        &[line(Some(Fractured)); 2],
        &[line(None); 4],
        &[line(Some(Desecrated))],
    ]
    .concat();
    let english_bow = [
        [(Rune, None); 3].as_slice(),
        &[line(Some(Fractured))],
        &[line(None); 5],
        &[line(Some(Desecrated))],
    ]
    .concat();
    let french_bow = [
        [(Rune, None); 5].as_slice(),
        &[line(Some(Fractured))],
        &[line(None); 5],
        &[line(Some(Desecrated)); 2],
    ]
    .concat();
    let belt = |explicit_lines: &[(ModifierKind, Option<ModifierOrigin>)]| {
        [
            [(Enchant, None), (Implicit, None), (Implicit, None)].as_slice(),
            explicit_lines,
        ]
        .concat()
    };
    for (text, expected) in [
        (FR_TRADE_RARE_HELMET, tiara.clone()),
        (EN_TRADE_RARE_HELMET, tiara),
        (FR_TRADE_RARE_BOW, french_bow),
        (EN_TRADE_RARE_BOW, english_bow),
        (
            FR_TRADE_UNIQUE_BELT,
            belt(&[line(None), line(None), line(None), line(Some(Vaal))]),
        ),
        (
            EN_TRADE_UNIQUE_BELT,
            belt(&[line(Some(Vaal)), line(None), line(None), line(None)]),
        ),
    ] {
        let item = item(text);
        assert_eq!(kinds(&item), expected, "{text}");
        // One line each, without tier nor range.
        for modifier in item.modifiers() {
            assert_eq!((modifier.lines.len(), modifier.tier), (1, None), "{text}");
            let rolls = &first_line(modifier).rolls;
            assert!(rolls.iter().all(|roll| roll.range.is_none()), "{text}");
        }
    }
}

#[test]
fn reads_a_rare_copied_from_the_in_game_trade() {
    let tiara = item(EN_TRADE_RARE_HELMET);

    assert_eq!(
        (tiara.name(), tiara.base_type()),
        (Some("Chimeric Visage"), "Ancestral Tiara")
    );
    assert_eq!(
        (tiara.quality(), tiara.sockets(), tiara.item_level()),
        (Some(20), Some(2), Some(82))
    );
    assert_eq!(tiara.states(), [ItemState::Fractured]);
    // Without their markers; the line of Hinekora's Lock, after the states, isn't read.
    assert_eq!(
        explicit_texts(&tiara),
        [
            "41% increased Energy Shield",
            "+34 to maximum Mana",
            "98% increased Energy Shield",
            "18% increased Rarity of Items found",
            "+43% to Fire Resistance",
            "+40% to Cold Resistance",
            "+69 to maximum Energy Shield",
        ]
    );
    assert_eq!(
        first_line(explicit(&tiara)[0]).rolls,
        [Roll {
            value: 41.0,
            range: None
        }]
    );
    let french = item(FR_TRADE_RARE_HELMET);
    assert_eq!(french.states(), [ItemState::Fractured]);
    assert!(
        french
            .modifiers()
            .iter()
            .all(|modifier| !first_line(modifier).text.contains("Hinekora"))
    );

    let bow = item(EN_TRADE_RARE_BOW);
    assert_eq!(bow.states(), [ItemState::Mirrored, ItemState::Fractured]);
    assert_eq!(
        first_line(explicit(&bow)[1]).text,
        "258% increased Physical Damage"
    );
    assert!(bow.weapon().is_some());
}

#[test]
fn reads_a_unique_copied_from_the_in_game_trade() {
    let belt = item(EN_TRADE_UNIQUE_BELT);

    assert_eq!(belt.states(), [ItemState::Corrupted]);
    assert_eq!(
        first_line(of_kind(&belt, ModifierKind::Implicit)[1]).text,
        "Has 3 Charm Slots"
    );
    // Its flavour text, after its lines, isn't read.
    assert_eq!(
        explicit_texts(&belt),
        [
            "14% increased Energy Shield Recovery rate",
            "+51 to maximum Energy Shield",
            "You lose 5% of maximum Energy Shield per second",
            "Excess Life Recovery from Leech is applied to Energy Shield",
        ]
    );
    let french = item(FR_TRADE_UNIQUE_BELT);
    assert_eq!(
        first_line(explicit(&french)[3]).text,
        "14% d'Augmentation de la Récupération de Bouclier d'énergie"
    );
}

#[test]
fn reads_the_items_written_without_the_headers() {
    use ModifierKind::Implicit;
    let explicit = ModifierKind::Explicit(None);
    // The magic flask of the owner, in French, without its headers.
    let french_flask = FR_MAGIC_MANA_FLASK
        .lines()
        .filter(|line| !line.starts_with('{'))
        .collect::<Vec<_>>()
        .join("\n");
    let normal_ring = "Item Class: Rings\nRarity: Normal\nRuby Ring\n--------\nItem Level: 70\n\
                       --------\n+25% to Fire Resistance (implicit)";
    for (text, expected) in [
        (
            EN_PLAIN_RARE_RING,
            vec![
                (Implicit, None),
                (explicit, None),
                (explicit, Some(ModifierOrigin::Crafted)),
            ],
        ),
        (EN_PLAIN_RARE_RING_UNIDENTIFIED, vec![(Implicit, None)]),
        (EN_PLAIN_UNIQUE_JEWEL, vec![(explicit, None); 2]),
        (normal_ring, vec![(Implicit, None)]),
        (&french_flask, vec![(explicit, None); 2]),
    ] {
        assert_eq!(kinds(&item(text)), expected, "{text}");
    }
    // Their descriptions aren't read.
    assert_eq!(
        explicit_texts(&item(&french_flask)),
        [
            "53% d'Augmentation du Montant Récupéré",
            "55% d'Augmentation de Charges"
        ]
    );
}

#[test]
fn reads_no_description_as_a_modifier_of_an_item_without_headers() {
    // Runes and an enchantment only, which have no header in either copy.
    let runes_and_enchantment = "Item Class: Body Armours\nRarity: Normal\nVile Robe\n--------\n\
                                 Item Level: 81\n--------\n+12% to Fire Resistance (rune)\n\
                                 --------\n45% increased Armour (enchant)";
    assert_eq!(
        kinds(&item(runes_and_enchantment)),
        [(ModifierKind::Rune, None), (ModifierKind::Enchant, None)]
    );
    // A magic flask whose only modifier was removed: its description is no modifier.
    let (before, modifiers) = FR_MAGIC_MANA_FLASK.split_once("--------\n{").unwrap();
    let description = modifiers.split_once("--------\n").unwrap().1;
    let flask_without_modifier = format!("{before}--------\n{description}");
    assert_eq!(item(&flask_without_modifier).modifiers(), []);
    // Only the first plain section after the item level: a flavour text with a number after
    // the lines isn't read.
    let jewel = item(&EN_PLAIN_UNIQUE_JEWEL.replace("A rainbow", "A rainbow of 7"));
    assert_eq!(explicit(&jewel).len(), 2);
    // Every copied item is read without warning.
    for text in ALL {
        item(text);
    }
}

#[test]
fn removes_the_french_exceptional_word_in_each_gender() {
    // "Exceptionnel" for a masculine base, as shown by the French trade site.
    for (text, base_type) in [
        (
            "Classe d'objet: Boucliers\nRareté: Normal\nExceptionnel Écu sophistiqué",
            "Écu sophistiqué",
        ),
        (
            "Classe d'objet: Boucliers\nRareté: Normal\nExceptionnelle Targe de mammouth",
            "Targe de mammouth",
        ),
    ] {
        let item = item(text);
        assert_eq!(item.base_type(), base_type);
        assert!(item.is_exceptional());
    }
}

#[test]
fn reads_a_stack_with_thousands_separators() {
    assert_eq!(
        item(EN_PLAIN_CURRENCY_LARGE_STACK).stack(),
        Some(Stack {
            size: 12_345,
            max: 100_000
        })
    );
}

#[test]
fn keeps_the_lines_of_an_unknown_header_apart() {
    let item = try_parse(
        "Item Class: Rings\nRarity: Rare\nWoe Loop\nRuby Ring\n--------\nItem Level: 80\n\
         --------\n{ Prefix Modifier \"Icy\" (Tier: 8) — Cold }\nAdds 9 to 13 Cold Damage\n\
         { Strange Modifier }\n+10 to Strength",
    )
    .expect("the item is read");

    assert_eq!(item.warnings(), [ParseWarning::UnknownModifierHeader]);
    let modifiers = item.modifiers();
    assert_eq!(modifiers.len(), 2);
    assert_eq!((modifiers[0].tier, modifiers[0].lines.len()), (Some(8), 1));
    assert_eq!(
        (modifiers[1].kind, modifiers[1].tier),
        (ModifierKind::Explicit(None), None)
    );
    assert_eq!(first_line(&modifiers[1]).text, "+10 to Strength");
}

#[test]
fn keeps_the_lines_before_the_first_header_with_a_warning() {
    let item = try_parse(
        "Item Class: Rings\nRarity: Rare\nWoe Loop\nRuby Ring\n--------\nItem Level: 80\n\
         --------\n+10 to Strength\n{ Suffix Modifier \"of the Brute\" (Tier: 8) }\n+8 to Strength",
    )
    .expect("the item is read");

    assert_eq!(item.warnings(), [ParseWarning::ModifierWithoutHeader]);
    assert_eq!(
        item.modifiers()
            .iter()
            .map(|modifier| modifier.kind)
            .collect::<Vec<_>>(),
        [ModifierKind::Explicit(None), SUFFIX]
    );
}

#[test]
fn reads_an_item_without_the_parts_that_can_t_be_read() {
    let item = try_parse(
        "Item Class: Rings\nRarity: Rare\nWoe Loop\nRuby Ring\n--------\n\
         Requires: Level 40, 12 Luck\n--------\nItem Level: high",
    )
    .expect("the item is read");

    assert_eq!(item.requirements().level, Some(40));
    assert_eq!(item.item_level(), None);
    assert_eq!(
        item.warnings(),
        [
            ParseWarning::InvalidValue(Field::Requirements),
            ParseWarning::InvalidValue(Field::ItemLevel)
        ]
    );
}

#[test]
fn tells_why_an_item_can_t_be_read() {
    assert_eq!(
        try_parse("Item Class: Rings\nWoe Loop").err(),
        Some(ParseError::MissingRarity)
    );
    assert_eq!(
        try_parse("Item Class: Rings\nRarity: Legendary\nWoe Loop").err(),
        Some(ParseError::UnknownRarity)
    );
    assert_eq!(
        try_parse("Item Class: Rings\nRarity: Rare").err(),
        Some(ParseError::MissingBaseType)
    );
}

#[test]
fn reads_the_small_parts() {
    assert_eq!(digits("+20%"), Some(20));
    assert_eq!(digits("1 234"), Some(1234));
    assert_eq!(digits("none"), None);
    assert_eq!(digits("99999999999"), None);

    let labels = Labels::of(Language::English);
    assert_eq!(count_sockets("Sockets: S S S", labels), 3);
    assert_eq!(count_sockets("Sockets: ", labels), 0);
    assert_eq!(
        read_requirements("Requires: 113 Intelligence", labels),
        (
            Requirements {
                intelligence: Some(113),
                ..Requirements::default()
            },
            true
        )
    );
}

#[test]
fn never_panics_on_a_cut_or_changed_item() {
    for text in ALL {
        let chars: Vec<char> = text.chars().collect();
        for end in 0..=chars.len() {
            let cut: String = chars.iter().take(end).collect();
            if let Some(copied) = CopiedItem::from_text(&cut) {
                let _ = parse(&copied);
            }
        }
        for line in text.lines().filter(|line| !line.is_empty()) {
            let changed = text.replace(line, "{ ( « \" — : 1(2- -) }");
            if let Some(copied) = CopiedItem::from_text(&changed) {
                let _ = parse(&copied);
            }
        }
    }
}

/// The DPS of an item, rounded to a tenth for the comparisons.
fn dps(text: &str) -> Option<(f64, f64, f64)> {
    let round = |value: f64| (value * 10.0).round() / 10.0;
    item(text).weapon().map(|weapon| {
        let dps = weapon.dps();
        (round(dps.physical), round(dps.elemental), round(dps.chaos))
    })
}

#[test]
fn reads_the_damage_and_the_speed_of_a_weapon() {
    // 381-705 × 1.15, with the quality already in the damage.
    assert_eq!(dps(EN_RARE_BOW_FRACTURED), Some((624.4, 0.0, 0.0)));
    // The attacks per second augmented by a modifier.
    assert_eq!(dps(EN_RARE_CROSSBOW), Some((288.2, 267.4, 0.0)));
    assert_eq!(dps(EN_MAGIC_MACE), Some((64.2, 30.6, 0.0)));
    assert_eq!(dps(FR_UNIQUE_CROSSBOW), Some((15.2, 17.6, 0.0)));
    assert_eq!(dps(EN_NORMAL_HELMET), None, "not a weapon");
}

#[test]
fn the_title_is_the_name_the_game_shows() {
    assert_eq!(item(EN_RARE_BOW_FRACTURED).title(), "Miracle Siege");
    assert_eq!(
        item(EN_MAGIC_MACE).title(),
        "Crackling Temple Maul of the Brute"
    );
    // The base type keeps its word "Exceptional" in the title only.
    let shield = item(FR_NORMAL_EXCEPTIONAL_SHIELD);
    assert_eq!(shield.title(), "Exceptionnelle Targe de mammouth");
    assert_eq!(shield.base_type(), "Targe de mammouth");
}

#[test]
fn reads_the_defences_of_an_armour_piece_or_a_shield() {
    assert_eq!(
        item(EN_UNIQUE_FOCUS).defences(),
        Defences {
            energy_shield: Some(44),
            ..Defences::default()
        }
    );
    assert_eq!(
        item(FR_NORMAL_EXCEPTIONAL_SHIELD).defences(),
        Defences {
            armour: Some(112),
            evasion: Some(102),
            energy_shield: None,
            block: Some(25),
            runic_ward: None,
        }
    );
    assert_eq!(
        item(FR_RARE_BODY_ARMOUR).defences().energy_shield,
        Some(567)
    );
    assert_eq!(item(EN_RARE_CROSSBOW).defences(), Defences::default());
}

#[test]
fn reads_the_runic_ward_of_a_runeforged_base() {
    for text in [EN_RARE_HELMET_RUNEFORGED, FR_RARE_HELMET_RUNEFORGED] {
        assert_eq!(
            item(text).defences(),
            Defences {
                armour: Some(314),
                energy_shield: Some(88),
                runic_ward: Some(56),
                ..Defences::default()
            }
        );
    }
}

#[test]
fn reads_the_vaal_modifier_of_a_corrupted_unique_from_its_header() {
    let belt = item(FR_UNIQUE_BELT_VAAL);
    let origins: Vec<_> = explicit(&belt)
        .into_iter()
        .map(|modifier| (modifier.kind, modifier.origin))
        .collect();
    let unique = ModifierKind::Explicit(None);
    assert_eq!(
        origins,
        [
            (unique, None),
            (unique, None),
            (unique, Some(ModifierOrigin::Vaal)),
            (unique, None)
        ]
    );
    let vaal = explicit(&belt)[2];
    assert_eq!(
        first_line(vaal).text,
        "+112 au Score d'Évasion tant que votre Vie est Basse"
    );
    assert_eq!(vaal.tags, ["Évasion"]);

    let english = item(
        "Item Class: Belts\nRarity: Unique\nCoward's Legacy\nPlate Belt\n--------\n\
         Item Level: 79\n--------\n{ Vaal Unique Modifier — Evasion }\n\
         +112(100-150) to Evasion Rating while on Low Life\n--------\nCorrupted",
    );
    assert_eq!(
        explicit(&english)
            .into_iter()
            .map(|modifier| (modifier.kind, modifier.origin))
            .collect::<Vec<_>>(),
        [(unique, Some(ModifierOrigin::Vaal))]
    );
}

#[test]
fn reads_the_line_of_a_unique_written_without_header() {
    let ring = item(FR_UNIQUE_RING_MIRRORED);
    let modifiers = explicit(&ring);
    assert_eq!(modifiers.len(), 1);
    let line = modifiers[0];
    assert_eq!(
        (line.kind, line.origin, line.tier),
        (ModifierKind::Explicit(None), None, None)
    );
    assert_eq!(
        line.lines,
        [StatLine {
            text: "Reflète la Bague opposée".to_owned(),
            rolls: Vec::new(),
        }]
    );
    // Its flavour text, after it, isn't read.
    assert_eq!(ring.modifiers().len(), 1);

    // Only its first plain section, also with a third one after the flavour text.
    let noted = item(
        "Classe d'objet: Bagues\nRareté: Unique\nToucher de Kalandra\nBague\n--------\n\
         Niveau de l'objet: 80\n--------\nReflète la Bague opposée\n--------\n\
         Le pouvoir n'est qu'affaire de perception.\n--------\nUne description.",
    );
    let lines: Vec<&str> = explicit(&noted)
        .into_iter()
        .flat_map(|modifier| modifier.lines.iter().map(|line| line.text.as_str()))
        .collect();
    assert_eq!(lines, ["Reflète la Bague opposée"]);
}

#[test]
fn a_unique_copied_without_details_is_read_with_its_lines_with_or_without_number() {
    // A copy of the trade of the game: its lines with numbers, also without a flavour text
    // after them.
    for text in [
        "Item Class: Rings\nRarity: Unique\nTest Loop\nGold Ring\n--------\n\
         Item Level: 80\n--------\n+52 to maximum Life\n--------\nFlavour text.",
        "Item Class: Rings\nRarity: Unique\nTest Loop\nGold Ring\n--------\n\
         Item Level: 80\n--------\n+52 to maximum Life",
    ] {
        assert_eq!(
            explicit_texts(&item(text)),
            ["+52 to maximum Life"],
            "{text}"
        );
    }
    // Its lines without number are the same on every copy: read like those of Kalandra's
    // Touch.
    let ring = item(
        "Item Class: Rings\nRarity: Unique\nTest Loop\nGold Ring\n--------\n\
         Item Level: 80\n--------\nReflects opposite Ring\n--------\nFlavour text.",
    );
    assert_eq!(
        first_line(explicit(&ring)[0]).text,
        "Reflects opposite Ring"
    );
}

#[test]
fn reads_no_flavour_text_nor_description_as_a_line_without_header() {
    let explicit_lines = |text: &str| -> Vec<String> {
        let item = try_parse(text).expect("the item is read");
        explicit(&item)
            .into_iter()
            .flat_map(|modifier| modifier.lines.iter().map(|line| line.text.clone()))
            .collect()
    };
    // A unique whose modifiers have headers: its flavour text and its description.
    assert_eq!(
        explicit_lines(
            "Item Class: Rings\nRarity: Unique\nTest Loop\nGold Ring\n--------\n\
             Item Level: 80\n--------\n{ Unique Modifier }\n+52 to maximum Life\n--------\n\
             Flavour text.\n--------\nA description."
        ),
        ["+52 to maximum Life"]
    );
    // A flavour text alone, unidentified, on another rarity, or after an implicit with its
    // header: a copy with a `{ … }` header writes one before every modifier.
    for text in [
        "Item Class: Rings\nRarity: Unique\nTest Loop\nGold Ring\n--------\nItem Level: 80\n\
         --------\n{ Implicit Modifier }\n+10 to Strength\n--------\nFlavour text.\n--------\n\
         A description.",
        "Item Class: Rings\nRarity: Unique\nTest Loop\nGold Ring\n--------\nItem Level: 80\n\
         --------\nFlavour text.",
        "Item Class: Rings\nRarity: Unique\nGold Ring\n--------\nItem Level: 80\n--------\n\
         Unidentified\n--------\nFlavour text.\n--------\nA description.",
        "Item Class: Rings\nRarity: Normal\nGold Ring\n--------\nItem Level: 80\n--------\n\
         A description.\n--------\nAnother description.",
    ] {
        assert_eq!(explicit_lines(text), Vec::<String>::new(), "{text}");
    }
    // Before the item level: the properties and the requirements.
    assert_eq!(
        explicit_lines(
            "Item Class: Rings\nRarity: Unique\nTest Loop\nGold Ring\n--------\nA property\n\
             --------\nAnother text\n--------\nItem Level: 80"
        ),
        Vec::<String>::new()
    );
}

/// The id of the stat the game data finds on all the lines of `modifier` together.
fn stat_id(modifier: &Modifier) -> Option<&'static str> {
    match game_data::modifier_stats(Language::English, modifier, None).as_slice() {
        [found] => found.stat.as_ref()?.stat.id.as_deref(),
        _ => None,
    }
}

#[test]
fn joins_the_lines_of_one_stat_written_without_header() {
    let rare = item(
        "Item Class: Gloves\nRarity: Rare\nTest Grip\nVaal Gloves\n--------\nItem Level: 80\n\
         --------\n+40 to maximum Life\nInflict Anaemia on Hit (desecrated)\n\
         Anaemia allows 2 Corrupted Blood debuffs to be inflicted on enemies (desecrated)\n\
         +30% to Fire Resistance (desecrated)",
    );
    let lines: Vec<usize> = explicit(&rare).iter().map(|m| m.lines.len()).collect();
    assert_eq!(lines, [1, 2, 1]);
    let anaemia = explicit(&rare)[1];
    assert_eq!(anaemia.origin, Some(ModifierOrigin::Desecrated));
    assert_eq!(stat_id(anaemia), Some("apply_anaemia_magnitude_on_hit"));

    // The second line alone would take the stat of every rarity.
    let unique = item(
        "Item Class: Gloves\nRarity: Unique\nTest Grasp\nVaal Gloves\n--------\nItem Level: 80\n\
         --------\n50% increased Rarity of Items found\n\
         Your other Modifiers to Rarity of Items found do not apply\n+20 to Strength\n\
         --------\nFlavour text.",
    );
    let lines: Vec<usize> = explicit(&unique).iter().map(|m| m.lines.len()).collect();
    assert_eq!(lines, [2, 1]);
    assert_eq!(
        stat_id(explicit(&unique)[0]),
        Some("unique_gloves_item_rarity_only_+%")
    );
}

#[test]
fn keeps_apart_the_lines_of_other_modifiers() {
    // Another origin: two modifiers.
    let rare = item(
        "Item Class: Gloves\nRarity: Rare\nTest Grip\nVaal Gloves\n--------\nItem Level: 80\n\
         --------\nInflict Anaemia on Hit (desecrated)\n\
         Anaemia allows 2 Corrupted Blood debuffs to be inflicted on enemies",
    );
    assert_eq!(explicit(&rare).len(), 2);
    // Two headers: the advanced copy tells its modifiers apart.
    let headed = item(
        "Item Class: Gloves\nRarity: Unique\nTest Grasp\nVaal Gloves\n--------\nItem Level: 80\n\
         --------\n{ Unique Modifier }\n50(40-50)% increased Rarity of Items found\n\
         { Unique Modifier }\nYour other Modifiers to Rarity of Items found do not apply",
    );
    assert_eq!(explicit(&headed).len(), 2);
    // The copied items keep their modifiers: none of them is written on several lines.
    for text in [
        FR_TRADE_RARE_BOW,
        EN_TRADE_RARE_HELMET,
        EN_TRADE_UNIQUE_BELT,
    ] {
        let item = item(text);
        assert!(
            item.modifiers().iter().all(|m| m.lines.len() == 1),
            "{text}"
        );
    }
}

#[test]
fn reads_a_section_of_fractured_lines_only_as_the_explicit_modifiers() {
    let rare = item(
        "Item Class: Rings\nRarity: Rare\nTest Loop\nGold Ring\n--------\nItem Level: 80\n\
         --------\n+40 to maximum Life (fractured)\n+30% to Fire Resistance (fractured)",
    );
    let explicit_kind = ModifierKind::Explicit(None);
    assert_eq!(
        kinds(&rare),
        [(explicit_kind, Some(ModifierOrigin::Fractured)); 2]
    );
}

#[test]
fn a_french_unique_without_header_is_read_before_a_note_with_a_number() {
    let ring = item(
        "Classe d'objet: Bagues\nRareté: Unique\nToucher de Kalandra\nBague\n--------\n\
         Niveau de l'objet: 80\n--------\nReflète la Bague opposée\n--------\n\
         Le pouvoir n'est qu'affaire de perception.\n--------\n~prix 5 divine",
    );
    assert_eq!(explicit_texts(&ring), ["Reflète la Bague opposée"]);
}

#[test]
fn loses_the_explicit_lines_without_number_of_a_magic_item_without_headers() {
    // A known limit (step 13k5): a line without number can't be told from a description.
    let waystone = item(
        "Item Class: Waystones\nRarity: Magic\nBurning Waystone (Tier 15)\n--------\n\
         Item Level: 80\n--------\nArea has patches of Ignited Ground",
    );
    assert_eq!(waystone.modifiers(), []);
}

#[test]
fn an_advanced_copy_never_has_its_explicit_modifiers_found_without_header() {
    for text in ALL.into_iter().filter(|text| text.contains("\n{ ")) {
        let copied = CopiedItem::from_text(text).expect("an item");
        let labels = Labels::of(copied.language());
        let text = copied.text().replace(['\u{a0}', '\u{202f}'], " ");
        let recognized: Vec<Recognized<'_>> = split_sections(&text)
            .skip(1)
            .map(|lines| recognize(lines, labels))
            .collect();
        let rarity = item(&text).rarity();
        assert_eq!(explicit_section(&recognized, rarity), None, "{text}");
    }
}
