use super::*;
use crate::item::CopiedItem;
use crate::language::Language;
use crate::roll_difference::CopiedLines;
use crate::tier_rolls::TierRolls;
use crate::trade::stat_filters::LineStats;

/// The texts of the block of the item copied as `text`, with the interface in `language`.
struct Texts {
    base_line: Option<String>,
    tags: Vec<String>,
    base_values: Vec<String>,
    skills: Option<String>,
    requirements: Option<String>,
}

fn texts(text: &str, language: Language) -> Texts {
    let item = Item::parse(&CopiedItem::from_text(text).expect("an item")).expect("read");
    let i18n = Translator::new(language);
    let mut filters = Filters::default();
    let lines = CopiedLines::default();
    let stats = LineStats::default();
    let tiers = TierRolls::default();
    let block = ItemBlock::new(
        &item,
        LineFacts {
            copied_lines: &lines,
            line_stats: &stats,
            tier_rolls: &tiers,
        },
        &mut filters,
        &i18n,
    );
    Texts {
        base_line: block.base_line().map(str::to_owned),
        tags: block.tags().into_iter().map(|(label, _)| label).collect(),
        base_values: block
            .base_values()
            .into_iter()
            .map(|value| format!("{} {}", i18n.text(value.label()), value.number()))
            .collect(),
        skills: block.skills(),
        requirements: block
            .requirements()
            .map(|parts| block.requirements_line(&parts).text),
    }
}

fn english(text: &str) -> Texts {
    texts(text, Language::English)
}

/// The name, the line under it and the base values of the waystone copied as `text`, of the
/// base type `base_type`, with the interface in `language`; the tooltips of the base values
/// written short after them, between parentheses.
fn waystone(text: &str, base_type: &str, language: Language) -> (String, String, Vec<String>) {
    let item = Item::parse(&CopiedItem::from_text(text).expect("an item")).expect("read");
    let i18n = Translator::new(language);
    let mut filters = Filters::default();
    let lines = CopiedLines::default();
    let stats = LineStats::default();
    let tiers = TierRolls::default();
    let block = ItemBlock::new(
        &item,
        LineFacts {
            copied_lines: &lines,
            line_stats: &stats,
            tier_rolls: &tiers,
        },
        &mut filters,
        &i18n,
    )
    .stats(false, Some(base_type));
    let values = block
        .base_values()
        .into_iter()
        .map(|value| {
            let label = i18n.text(value.label());
            let full = value
                .full_label()
                .map(|full| i18n.text(full))
                .filter(|full| *full != label)
                .map(|full| format!(" ({full})"))
                .unwrap_or_default();
            format!("{label} {}{full}", value.number())
        })
        .collect();
    (
        block.title().into_owned(),
        block.base_and_class().text,
        values,
    )
}

#[test]
fn a_waystone_shows_its_base_and_tier_and_its_properties() {
    let (name, base, values) = waystone(
        include_str!("../../item/test-items/en-owner-rare-waystone.txt"),
        "Waystone (Tier 12)",
        Language::English,
    );
    assert_eq!(name, "Arcane Choice");
    // Without its class, which only repeats it.
    assert_eq!(base, "Waystone · T12");
    assert_eq!(
        values,
        [
            "Revives 3 (Revives Available)",
            "Item Rarity +11%",
            "Monster Rarity +25%",
            "Monster Effectiveness +13%",
            "Waystone Drop Chance +50%",
        ]
    );

    let (name, base, values) = waystone(
        include_str!("../../item/test-items/fr-rare-waystone.txt"),
        "Pierre de téléportation (Palier 12)",
        Language::French,
    );
    assert_eq!(name, "Intention génitrice");
    assert_eq!(base, "Pierre de téléportation · T12");
    assert_eq!(
        values,
        [
            "Résurrections 2 (Résurrections disponibles)",
            "Rareté des objets +25%",
            "Taille des Groupes +9%",
            "Rareté des Monstres +23%",
            "Pierres trouvées +65% (Chances de trouver des Pierres de téléportation)",
        ]
    );

    // A magic waystone, named without its tier, in both languages.
    let (name, base, _) = waystone(
        "Item Class: Waystones\nRarity: Magic\nBurning Waystone (Tier 15)\n--------\n\
             Item Level: 80",
        "Waystone (Tier 15)",
        Language::English,
    );
    assert_eq!(
        (name.as_str(), base.as_str()),
        ("Burning Waystone", "Waystone · T15")
    );
    let (name, _, _) = waystone(
        "Classe d'objet: Pierres de téléportation\nRareté: Magique\n\
             Pierre de téléportation (Palier 15) incendiaire\n--------\n\
             Niveau de l'objet: 80",
        "Pierre de téléportation (Palier 15)",
        Language::French,
    );
    assert_eq!(name, "Pierre de téléportation incendiaire");
}

#[test]
fn a_waystone_shows_its_values_in_the_language_of_the_interface() {
    // A French waystone in the English interface: the words of the interface, the base
    // and its tier in the words of the item.
    let (name, base, values) = waystone(
        include_str!("../../item/test-items/fr-rare-waystone.txt"),
        "Pierre de téléportation (Palier 12)",
        Language::English,
    );
    assert_eq!(name, "Intention génitrice");
    assert_eq!(base, "Pierre de téléportation · T12");
    assert_eq!(
        values,
        [
            "Revives 2 (Revives Available)",
            "Item Rarity +25%",
            "Pack Size +9%",
            "Monster Rarity +23%",
            "Waystone Drop Chance +65%",
        ]
    );
    // A negative property with its sign, as the game writes it.
    let (_, _, values) = waystone(
        "Item Class: Waystones\nRarity: Normal\nWaystone (Tier 3)\n--------\n\
             Item Rarity: -5% (augmented)\n--------\nItem Level: 66",
        "Waystone (Tier 3)",
        Language::English,
    );
    assert_eq!(values, ["Item Rarity -5%"]);
}

#[test]
fn a_magic_waystone_of_an_unknown_base_keeps_the_name_of_the_game() {
    // The names of the trade site not downloaded yet: no base to write under the name,
    // which keeps its tier, the class alone under it, as any magic item.
    let item = Item::parse(
        &CopiedItem::from_text(
            "Item Class: Waystones\nRarity: Magic\nBurning Waystone (Tier 15)\n\
                 --------\nItem Level: 80",
        )
        .expect("an item"),
    )
    .expect("read");
    let i18n = Translator::new(Language::English);
    let mut filters = Filters::default();
    let (lines, stats, tiers) = (
        CopiedLines::default(),
        LineStats::default(),
        TierRolls::default(),
    );
    let block = ItemBlock::new(
        &item,
        LineFacts {
            copied_lines: &lines,
            line_stats: &stats,
            tier_rolls: &tiers,
        },
        &mut filters,
        &i18n,
    )
    .stats(false, None);

    assert_eq!(block.title(), "Burning Waystone (Tier 15)");
    assert_eq!(block.base_and_class().text, "Waystones");
}

#[test]
fn the_tier_of_a_waystone_has_the_color_of_its_band() {
    assert_eq!(waystone_tier_color(1), color::TEXT);
    assert_eq!(waystone_tier_color(5), color::TEXT);
    assert_eq!(waystone_tier_color(6), color::WARNING);
    assert_eq!(waystone_tier_color(10), color::WARNING);
    assert_eq!(waystone_tier_color(11), color::TIER_1);
    assert_eq!(waystone_tier_color(15), color::TIER_1);
    assert_eq!(waystone_tier_color(16), color::VAAL);
}

#[test]
fn a_rare_weapon_shows_its_tags_in_order_its_dps_and_requirements() {
    let crossbow = english(include_str!(
        "../../item/test-items/en-rare-crossbow-all-modifiers.txt"
    ));

    assert_eq!(crossbow.base_line.as_deref(), Some("Gemini Crossbow"));
    assert_eq!(
        crossbow.tags,
        ["Corrupted", "Fractured", "Q 20%", "2 sockets", "ilvl 80",]
    );
    assert_eq!(
        crossbow.base_values,
        ["Physical DPS 288", "Elemental DPS 267"]
    );
    assert_eq!(
        crossbow.requirements.as_deref(),
        Some("Requires Level 78 · 89 Str · 89 Dex")
    );
}

#[test]
fn an_armour_piece_or_a_shield_shows_its_defences() {
    let focus = english(include_str!("../../item/test-items/en-unique-focus.txt"));
    let shield = texts(
        include_str!("../../item/test-items/fr-normal-exceptional-shield.txt"),
        Language::French,
    );

    assert_eq!(focus.tags, ["ilvl 81"]);
    assert_eq!(focus.base_line.as_deref(), Some("Crystal Focus"));
    assert_eq!(focus.base_values, ["Energy Shield 44"]);
    assert_eq!(shield.base_line, None, "named by its base");
    assert_eq!(
        shield.base_values,
        ["Armure 112", "Évasion 102", "Blocage 25%"]
    );
}

#[test]
fn a_runeforged_base_ends_its_defences_with_its_runic_ward_in_sky_blue() {
    let english = english(include_str!(
        "../../item/test-items/en-owner-rare-helmet-runeforged.txt"
    ));
    let french = texts(
        include_str!("../../item/test-items/fr-rare-helmet-runeforged.txt"),
        Language::French,
    );

    assert_eq!(
        english.base_values,
        ["Armour 314", "Energy Shield 88", "Runic Ward 56"]
    );
    assert_eq!(
        french.base_values,
        [
            "Armure 314",
            "Bouclier d'énergie 88",
            "Sauvegarde runique 56"
        ]
    );
    assert_eq!(BaseValue::RunicWard(56).label_color(), color::RUNIC_WARD);
}

#[test]
fn a_stack_has_its_tag_and_no_requirements() {
    let stack = texts(
        include_str!("../../item/test-items/fr-currency-stack.txt"),
        Language::French,
    );

    assert_eq!(stack.tags, ["Pile 40/40"]);
    assert_eq!(stack.requirements, None);
    assert_eq!(stack.base_values, Vec::<String>::new());
}

#[test]
fn the_base_line_of_an_exceptional_rare_keeps_its_word() {
    let staff = english("Item Class: Staves\nRarity: Rare\nDoom Song\nExceptional Chiming Staff");

    assert_eq!(
        staff.base_line.as_deref(),
        Some("Exceptional Chiming Staff")
    );
}

#[test]
fn a_french_item_shows_its_texts_in_the_language_of_the_interface() {
    let staff = texts(
        include_str!("../../item/test-items/fr-rare-staff.txt"),
        Language::French,
    );

    assert!(
        staff.tags.iter().any(|tag| tag.starts_with("niv.")),
        "{:?}",
        staff.tags
    );
    assert!(
        staff
            .skills
            .is_some_and(|line| line.starts_with("Octroie l'aptitude"))
    );
    assert_eq!(
        staff.requirements.as_deref(),
        Some("Requiert niveau 64 · 87 Int")
    );
}

#[test]
fn the_state_tags_but_unidentified_are_filters() {
    let ring = Item::parse(
        &CopiedItem::from_text(
            "Item Class: Rings\nRarity: Rare\nRuby Ring\n--------\nItem Level: 81\n\
                 --------\nUnidentified\n--------\nTwice Corrupted",
        )
        .expect("an item"),
    )
    .expect("read");
    let i18n = Translator::new(Language::English);
    let mut filters = Filters::default();
    let (lines, line_stats, tiers) = (
        CopiedLines::default(),
        LineStats::default(),
        TierRolls::default(),
    );
    let block = ItemBlock::new(
        &ring,
        LineFacts {
            copied_lines: &lines,
            line_stats: &line_stats,
            tier_rolls: &tiers,
        },
        &mut filters,
        &i18n,
    );

    let kinds: Vec<(String, Option<ItemState>)> = block
        .tags()
        .into_iter()
        .map(|(label, kind)| match kind {
            TagKind::Plain | TagKind::Top(_) => (label, None),
            TagKind::State(state, _) => (label, Some(state)),
        })
        .collect();
    assert_eq!(
        kinds[..2],
        [
            (
                "Twice Corrupted".to_owned(),
                Some(ItemState::TwiceCorrupted)
            ),
            ("Unidentified".to_owned(), None),
        ]
    );
}

#[test]
fn a_state_tag_says_what_it_searches_gray_and_lit() {
    let tooltips = |language, state| {
        let i18n = Translator::new(language);
        let tag = state_tag(state).expect("a filter");
        (i18n.text(tag.or_not), i18n.text(tag.only))
    };
    let pair = |gray: &str, lit: &str| (gray.to_owned(), lit.to_owned());

    assert_eq!(
        tooltips(Language::English, ItemState::Corrupted),
        pair("Corrupted or not", "Corrupted offers only")
    );
    assert_eq!(
        tooltips(Language::English, ItemState::TwiceCorrupted),
        pair("Corrupted or not", "Twice corrupted offers only")
    );
    assert_eq!(
        tooltips(Language::French, ItemState::Mirrored),
        pair("Reflétées ou non", "Offres reflétées seulement")
    );
    assert_eq!(
        state_tag(ItemState::Fractured).map(|tag| tag.text),
        Some(color::FRACTURED)
    );
    assert_eq!(state_tag(ItemState::Unidentified), None);
}

#[test]
fn a_dps_under_one_or_past_the_largest_isnt_shown() {
    assert_eq!(rounded(0.4), None);
    assert_eq!(rounded(244.6), Some(245));
    assert_eq!(rounded(2e6), None);
}

/// The base line of `text` and its tags with their kind, in the interface `language`.
fn gem(text: &str, language: Language) -> (LayoutJob, Vec<(String, TagKind)>) {
    let item = Item::parse(&CopiedItem::from_text(text).expect("an item")).expect("read");
    let i18n = Translator::new(language);
    let mut filters = Filters::default();
    let (lines, stats, tiers) = (
        CopiedLines::default(),
        LineStats::default(),
        TierRolls::default(),
    );
    let block = ItemBlock::new(
        &item,
        LineFacts {
            copied_lines: &lines,
            line_stats: &stats,
            tier_rolls: &tiers,
        },
        &mut filters,
        &i18n,
    )
    .stats(false, None);
    (block.base_and_class(), block.tags())
}

#[test]
fn a_skill_gem_writes_its_level_after_its_class() {
    let (base, tags) = gem(
        include_str!("../../item/test-items/en-owner-skill-gem.txt"),
        Language::English,
    );
    assert_eq!(base.text, "Skill Gems · Level\u{a0}2");
    assert_eq!(
        base.sections.last().map(|s| s.format.color),
        Some(color::TEXT)
    );
    assert_eq!(tags, [("2 sockets".to_owned(), TagKind::Plain)]);
    let (base, _) = gem(
        include_str!("../../item/test-items/fr-skill-gem.txt"),
        Language::French,
    );
    assert_eq!(base.text, "Gemmes d'aptitude · Niveau\u{a0}2");
}

#[test]
fn the_level_quality_and_sockets_of_a_gem_at_their_top_have_their_color() {
    assert_eq!(gem_level_color(1), color::TEXT);
    assert_eq!(gem_level_color(19), color::TEXT);
    assert_eq!(gem_level_color(20), color::TIER_1);
    assert_eq!(gem_level_color(21), color::VAAL);
    let comet = |level: u32, quality: u32, sockets: usize| {
        let sockets = "G ".repeat(sockets);
        format!(
            "Item Class: Skill Gems\nRarity: Gem\nComet\n--------\nLevel: {level}\n\
                 Quality: +{quality}% (augmented)\n--------\nSockets: {sockets}"
        )
    };
    let (base, tags) = gem(&comet(21, 23, 5), Language::English);
    assert_eq!(
        base.sections.last().map(|s| s.format.color),
        Some(color::VAAL)
    );
    assert_eq!(
        tags,
        [
            ("Q 23%".to_owned(), TagKind::Top(color::VAAL)),
            ("5 sockets".to_owned(), TagKind::Top(color::TIER_1)),
        ]
    );
    let (_, tags) = gem(&comet(20, 20, 4), Language::English);
    assert_eq!(
        tags,
        [
            ("Q 20%".to_owned(), TagKind::Top(color::TIER_1)),
            ("4 sockets".to_owned(), TagKind::Plain),
        ]
    );
    let (_, tags) = gem(&comet(19, 19, 3), Language::English);
    assert!(tags.iter().all(|(_, kind)| *kind == TagKind::Plain));
    // Not on another item, where a quality of 20 % is usual.
    let (base, tags) = gem(
        "Item Class: Bows\nRarity: Normal\nGemini Bow\n--------\n\
             Quality: +20% (augmented)\n--------\nSockets: S S S S S \n--------\n\
             Item Level: 81",
        Language::English,
    );
    assert_eq!(base.text, "Bows");
    assert!(tags.iter().all(|(_, kind)| *kind == TagKind::Plain));
}
