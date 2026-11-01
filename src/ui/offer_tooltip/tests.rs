use super::*;

const OFFERS: [&str; 3] = ["a", "b", "c"];

fn at(time: f64) -> Pointer<'static> {
    Pointer {
        time,
        ..Pointer::default()
    }
}

fn hovering(id: &'static str, time: f64) -> Pointer<'static> {
    Pointer {
        hovered: Some(id),
        ..at(time)
    }
}

/// An amulet offered with the quality of a catalyst of `catalyst`, `value` %, and its
/// `modifiers` (`+20% to Maximum Quality`).
fn amulet(value: u32, catalyst: Catalyst, words: &str, modifiers: &[&str]) -> ItemOffer {
    use crate::item::QualityKind;
    use crate::trade::offers::{OfferItem, OfferModifier, OfferQuality};

    ItemOffer {
        id: "a".to_owned(),
        price: None,
        seller: SellerStatus::Online,
        listed: None,
        whisper: String::new(),
        item: OfferItem {
            rarity: None,
            name: None,
            base_type: "Solar Amulet".to_owned(),
            item_level: None,
            corrupted: false,
            quality: Some(OfferQuality {
                value,
                kind: Some(QualityKind {
                    catalyst: Some(catalyst),
                    words: words.to_owned(),
                }),
            }),
            gem_level: None,
            gem_sockets: None,
            requirements: crate::item::Requirements::default(),
            waystone_properties: Vec::new(),
            waystone_unread: false,
            modifiers: modifiers
                .iter()
                .map(|text| OfferModifier {
                    kind: ModifierKind::Explicit,
                    text: (*text).to_owned(),
                    stat: Some("explicit.stat_2039822488".to_owned()),
                })
                .collect(),
        },
    }
}

#[test]
fn the_quality_of_an_offer_is_white_at_its_maximum_and_red_of_another_kind() {
    use crate::item::CopiedItem;

    let i18n = Translator::new(crate::language::Language::English);
    // The owner's amulet: 40 % of mana quality.
    let copied = Item::parse(
        &CopiedItem::from_text(include_str!(
            "../../item/test-items/en-owner-rare-amulet-catalyst.txt"
        ))
        .unwrap(),
    )
    .unwrap();
    let essence = ["+20% to Maximum Quality"];
    let line = |offer: &ItemOffer| quality_line(offer, &copied, &i18n).unwrap();
    let difference = |value, verdict| Some(Difference { value, verdict });

    // Below its maximum of 40 %: gray, with its difference.
    let below = line(&amulet(30, Catalyst::Mana, "Mana Modifiers", &essence));
    assert_eq!(below.text, "Mana quality: 30%");
    assert_eq!(below.tint, color::WEAK);
    assert_eq!(below.difference, difference(-10.0, Verdict::Worse));
    // At its maximum: white.
    let at_maximum = line(&amulet(40, Catalyst::Mana, "Mana Modifiers", &essence));
    assert_eq!(at_maximum.tint, color::TEXT);
    assert_eq!(at_maximum.difference, None);
    // Left beyond its maximum once its essence was removed: white too.
    let beyond = line(&amulet(50, Catalyst::Mana, "Mana Modifiers", &[]));
    assert_eq!(beyond.tint, color::TEXT);
    assert_eq!(beyond.difference, difference(10.0, Verdict::Better));
    // Another kind: red, without difference.
    let attack = line(&amulet(50, Catalyst::Attack, "Attack Modifiers", &[]));
    assert_eq!(attack.text, "Attack quality: 50%");
    assert_eq!(attack.tint, color::FALL);
    assert_eq!(attack.difference, None);
}

#[test]
fn a_kind_that_cant_be_told_apart_is_never_red() {
    use crate::item::{CopiedItem, QualityKind};
    use crate::trade::offers::OfferQuality;

    let i18n = Translator::new(crate::language::Language::English);
    let parse = |words: &str| {
        let text = include_str!("../../item/test-items/en-owner-rare-amulet-catalyst.txt")
            .replace("Mana Modifiers", words);
        Item::parse(&CopiedItem::from_text(&text).unwrap()).unwrap()
    };
    let mana = parse("Mana Modifiers");
    let line = |offer: &ItemOffer, item: &Item| quality_line(offer, item, &i18n);

    // The copied item of a kind the application doesn't know.
    let strange = parse("Strange Modifiers");
    let attack = amulet(50, Catalyst::Attack, "Attack Modifiers", &[]);
    let of_strange = line(&attack, &strange).unwrap();
    assert_eq!(of_strange.text, "Attack quality: 50%");
    assert_eq!(of_strange.tint, color::TEXT, "beyond its maximum");
    assert_eq!(
        of_strange.difference,
        Some(Difference {
            value: 10.0,
            verdict: Verdict::Better
        })
    );
    // Below its maximum: gray.
    let below = line(
        &amulet(15, Catalyst::Attack, "Attack Modifiers", &[]),
        &strange,
    )
    .unwrap();
    assert_eq!(below.tint, color::WEAK);
    // An offer of a kind the application doesn't know: its words.
    let mut unknown = amulet(30, Catalyst::Mana, "", &[]);
    unknown.item.quality = Some(OfferQuality {
        value: 30,
        kind: Some(QualityKind {
            catalyst: None,
            words: "Strange Modifiers".to_owned(),
        }),
    });
    let of_unknown = line(&unknown, &mana).unwrap();
    assert_eq!(of_unknown.text, "Quality (Strange Modifiers): 30%");
    assert_eq!(of_unknown.tint, color::TEXT);
    assert_eq!(
        of_unknown.difference,
        Some(Difference {
            value: -10.0,
            verdict: Verdict::Worse
        })
    );
    // The quality of the base.
    let mut base = amulet(10, Catalyst::Mana, "", &[]);
    base.item.quality = Some(OfferQuality {
        value: 10,
        kind: None,
    });
    let of_base = line(&base, &mana).unwrap();
    assert_eq!(of_base.text, "Quality: 10%");
    assert_eq!(of_base.tint, color::WEAK);
    assert_eq!(
        of_base.difference,
        Some(Difference {
            value: -30.0,
            verdict: Verdict::Worse
        })
    );
    // No quality.
    base.item.quality = None;
    assert_eq!(line(&base, &mana), None);
}

#[test]
fn an_item_without_the_quality_of_a_catalyst_has_no_quality_line() {
    use crate::item::CopiedItem;

    let i18n = Translator::new(crate::language::Language::English);
    let bow = Item::parse(
        &CopiedItem::from_text(include_str!(
            "../../item/test-items/en-rare-bow-fractured.txt"
        ))
        .unwrap(),
    )
    .unwrap();

    assert_eq!(
        quality_line(
            &amulet(40, Catalyst::Mana, "Mana Modifiers", &[]),
            &bow,
            &i18n
        ),
        None
    );
}

/// The owner's waystone of tier 12: revives 3, item rarity +11 %, monster rarity +25 %,
/// monster effectiveness +13 %, drop chance +50 %, no pack size.
fn owners_waystone() -> Item {
    use crate::item::CopiedItem;

    Item::parse(
        &CopiedItem::from_text(include_str!(
            "../../item/test-items/en-owner-rare-waystone.txt"
        ))
        .unwrap(),
    )
    .unwrap()
}

/// A waystone of `base_type` and of item level 81 offered with its `properties`.
fn waystone(base_type: &str, properties: &[(WaystoneProperty, i32)]) -> ItemOffer {
    use crate::trade::offers::OfferItem;

    ItemOffer {
        id: "w".to_owned(),
        price: None,
        seller: SellerStatus::Online,
        listed: None,
        whisper: String::new(),
        item: OfferItem {
            rarity: None,
            name: None,
            base_type: base_type.to_owned(),
            item_level: Some(81),
            corrupted: false,
            quality: None,
            gem_level: None,
            gem_sockets: None,
            requirements: crate::item::Requirements::default(),
            waystone_properties: properties
                .iter()
                .map(|&(property, value)| WaystoneValue { property, value })
                .collect(),
            waystone_unread: false,
            modifiers: Vec::new(),
        },
    }
}

/// The lines of `offer` as `(text, tint, difference)`.
fn waystone_texts(
    offer: &ItemOffer,
    item: &Item,
    i18n: &Translator,
) -> Vec<(String, Color32, Option<f64>)> {
    waystone_lines(offer, item, i18n)
        .into_iter()
        .map(|line| {
            (
                line.text,
                line.tint,
                line.difference.map(|difference| difference.value),
            )
        })
        .collect()
}

#[test]
fn the_properties_of_an_offer_of_waystone_show_their_difference_from_the_copied_one() {
    use WaystoneProperty::{DropChance, ItemRarity, MonsterRarity, PackSize, Revives};

    let i18n = Translator::new(crate::language::Language::English);
    // The offer of the mock-up: a pack size the owner's lacks, no monster effectiveness.
    let offer = waystone(
        "Waystone (Tier 12)",
        &[
            (Revives, 0),
            (ItemRarity, 14),
            (PackSize, 8),
            (MonsterRarity, 31),
            (DropChance, 62),
        ],
    );

    let lines = waystone_lines(&offer, &owners_waystone(), &i18n);

    let shown: Vec<(&str, Color32, Option<f64>)> = lines
        .iter()
        .map(|line| {
            (
                line.text.as_str(),
                line.tint,
                line.difference.map(|difference| difference.value),
            )
        })
        .collect();
    assert_eq!(
        shown,
        [
            ("Revives available: 0", color::WEAK, Some(-3.0)),
            ("Item rarity: +14%", color::WEAK, Some(3.0)),
            ("Pack size: +8%", color::WEAK, Some(8.0)),
            ("Monster rarity: +31%", color::WEAK, Some(6.0)),
            // Lacking: faint at 0, its loss.
            ("Monster effectiveness: 0%", color::FAINT, Some(-13.0)),
            ("Waystone drop chance: +62%", color::WEAK, Some(12.0)),
        ]
    );
    // Every property is better higher, the revives too.
    let verdicts: Vec<Verdict> = lines
        .iter()
        .filter_map(|line| line.difference.map(|difference| difference.verdict))
        .collect();
    assert_eq!(
        verdicts,
        [
            Verdict::Worse,
            Verdict::Better,
            Verdict::Better,
            Verdict::Better,
            Verdict::Worse,
            Verdict::Better,
        ]
    );
}

#[test]
fn an_offer_with_a_property_that_cant_be_read_shows_no_loss() {
    use WaystoneProperty::{DropChance, ItemRarity};

    let i18n = Translator::new(crate::language::Language::English);
    // A property renamed by a patch: the monster rarity of the offer may be that one.
    let mut offer = waystone("Waystone (Tier 12)", &[(ItemRarity, 14), (DropChance, 62)]);
    offer.item.waystone_unread = true;

    let texts: Vec<String> = waystone_texts(&offer, &owners_waystone(), &i18n)
        .into_iter()
        .map(|(text, ..)| text)
        .collect();
    assert_eq!(texts, ["Item rarity: +14%", "Waystone drop chance: +62%"]);
}

#[test]
fn the_same_property_has_no_difference_and_another_item_no_waystone_line() {
    use WaystoneProperty::{DropChance, ItemRarity};

    let i18n = Translator::new(crate::language::Language::English);
    let offer = waystone("Waystone (Tier 12)", &[(ItemRarity, 11), (DropChance, 50)]);

    let lines = waystone_lines(&offer, &owners_waystone(), &i18n);
    assert_eq!(lines[1].text, "Item rarity: +11%");
    assert_eq!(lines[1].difference, None);
    // A copied item that isn't a waystone.
    let bow = Item::parse(
        &crate::item::CopiedItem::from_text(include_str!(
            "../../item/test-items/en-rare-bow-fractured.txt"
        ))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(waystone_lines(&offer, &bow, &i18n), []);
}

#[test]
fn the_properties_of_an_offer_of_waystone_are_written_in_french() {
    use crate::item::CopiedItem;
    use WaystoneProperty::{DropChance, Revives};

    let i18n = Translator::new(crate::language::Language::French);
    // The owner's French waystone: revives 2, item rarity +25 %, pack size +9 %, monster
    // rarity +23 %, drop chance +65 %.
    let copied = Item::parse(
        &CopiedItem::from_text(include_str!("../../item/test-items/fr-rare-waystone.txt")).unwrap(),
    )
    .unwrap();
    let offer = waystone(
        "Pierre de téléportation (Palier 15)",
        &[(Revives, 0), (DropChance, 85)],
    );

    // The French colon after a no-break space.
    let texts: Vec<String> = waystone_texts(&offer, &copied, &i18n)
        .into_iter()
        .map(|(text, ..)| text)
        .collect();
    assert_eq!(
        texts,
        [
            "Résurrections disponibles\u{a0}: 0",
            "Rareté des objets\u{a0}: 0%",
            "Taille des groupes\u{a0}: 0%",
            "Rareté des monstres\u{a0}: 0%",
            "Chances de trouver des pierres de téléportation\u{a0}: +85%",
        ]
    );
    assert_eq!(
        meta(&offer, &copied, &i18n, 300.0).text,
        "Pierre de téléportation · T15 · niv.\u{a0}81"
    );
}

#[test]
fn a_negative_property_that_the_offer_lacks_is_a_gain() {
    use crate::item::CopiedItem;

    let i18n = Translator::new(crate::language::Language::English);
    let copied = Item::parse(
        &CopiedItem::from_text(
            "Item Class: Waystones\nRarity: Normal\nWaystone (Tier 3)\n--------\n\
             Item Rarity: -5% (augmented)\n--------\nItem Level: 66",
        )
        .unwrap(),
    )
    .unwrap();

    // Faint, since the offer doesn't have it, but 0 is better than -5 %.
    assert_eq!(
        waystone_texts(&waystone("Waystone (Tier 3)", &[]), &copied, &i18n),
        [("Item rarity: 0%".to_owned(), color::FAINT, Some(5.0))]
    );
}

#[test]
fn an_offer_of_waystone_writes_its_tier_short_and_in_its_color() {
    let i18n = Translator::new(crate::language::Language::English);
    let sections = |offer: &ItemOffer, item: &Item| {
        let job = meta(offer, item, &i18n, 300.0);
        job.sections
            .iter()
            .map(|section| {
                let range = &section.byte_range;
                (
                    job.text[range.start.0..range.end.0].to_owned(),
                    section.format.color,
                )
            })
            .collect::<Vec<_>>()
    };
    let copied = owners_waystone();

    let mut offer = waystone("Waystone (Tier 12)", &[]);
    offer.item.corrupted = true;
    let shown = sections(&offer, &copied);
    let text: String = shown.iter().map(|(text, _)| text.as_str()).collect();
    assert_eq!(text, "Waystone · T12 · ilvl 81 · Corrupted");
    assert!(
        shown.contains(&("T12".to_owned(), color::TIER_1)),
        "{shown:?}"
    );
    // A base without its tier, or a copied item that isn't a waystone: the base as it is.
    let plain = waystone("Waystone", &[]);
    assert_eq!(
        meta(&plain, &copied, &i18n, 300.0).text,
        "Waystone · ilvl 81"
    );
    let amulet = Item::parse(
        &crate::item::CopiedItem::from_text(include_str!(
            "../../item/test-items/en-owner-rare-amulet-catalyst.txt"
        ))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        meta(&offer, &amulet, &i18n, 300.0).text,
        "Waystone (Tier 12) · ilvl 81 · Corrupted"
    );
}

#[test]
fn hovering_a_row_opens_its_tooltip_which_lingers_after_the_pointer_leaves() {
    let mut state = TooltipState::default();

    state.update(&hovering("a", 0.0), &OFFERS);
    assert_eq!(state.open(), Some("a"));

    let redraw = state.update(&at(1.0), &OFFERS);
    assert_eq!(state.open(), Some("a"), "still open a moment");
    assert_eq!(redraw, Some(Duration::from_secs_f64(LINGER)));

    state.update(&at(1.0 + LINGER), &OFFERS);
    assert_eq!(state.open(), None);
}

#[test]
fn the_tooltip_stays_while_the_pointer_is_inside_it() {
    let mut state = TooltipState::default();
    state.update(&hovering("a", 0.0), &OFFERS);
    state.update(&at(0.1), &OFFERS);

    let inside = Pointer {
        over_tooltip: true,
        ..at(5.0)
    };
    state.update(&inside, &OFFERS);
    state.update(&inside, &OFFERS);

    assert_eq!(state.open(), Some("a"));
}

#[test]
fn hovering_another_row_moves_the_tooltip_at_once() {
    let mut state = TooltipState::default();
    state.update(&hovering("a", 0.0), &OFFERS);
    state.update(&at(0.1), &OFFERS);

    state.update(&hovering("b", 0.15), &OFFERS);

    assert_eq!(state.open(), Some("b"));
}

#[test]
fn a_click_pins_the_tooltip_until_a_click_elsewhere_or_escape() {
    let mut state = TooltipState::default();
    let click = Pointer {
        clicked: Some("a"),
        any_click: true,
        ..hovering("a", 0.0)
    };
    state.update(&click, &OFFERS);

    state.update(&at(10.0), &OFFERS);
    state.update(&hovering("b", 11.0), &OFFERS);
    assert_eq!(
        state.open(),
        Some("a"),
        "pinned: neither leaving nor hovering"
    );

    let click_inside = Pointer {
        any_click: true,
        over_tooltip: true,
        ..at(12.0)
    };
    state.update(&click_inside, &OFFERS);
    assert_eq!(state.open(), Some("a"), "Copy whisper keeps it");

    let click_elsewhere = Pointer {
        any_click: true,
        ..at(13.0)
    };
    state.update(&click_elsewhere, &OFFERS);
    assert_eq!(state.open(), None);

    state.update(&click, &OFFERS);
    let escape = Pointer {
        escape: true,
        ..at(14.0)
    };
    state.update(&escape, &OFFERS);
    assert_eq!(state.open(), None);
}

#[test]
fn the_pointer_reaches_the_tooltip_over_the_edge_of_the_next_row() {
    let mut state = TooltipState::default();
    state.update(&hovering("b", 0.0), &OFFERS);

    // Between the row and the tooltip, the pointer is on the edge of the row above.
    let gap = Pointer {
        over_tooltip: true,
        ..hovering("a", 0.05)
    };
    state.update(&gap, &OFFERS);
    assert_eq!(state.open(), Some("b"));

    let inside = Pointer {
        over_tooltip: true,
        ..at(0.1)
    };
    let redraw = state.update(&inside, &OFFERS);
    assert_eq!(state.open(), Some("b"));
    assert_eq!(redraw, None, "nothing to wait for");
}

#[test]
fn coming_back_to_the_row_keeps_the_tooltip_open() {
    let mut state = TooltipState::default();
    state.update(&hovering("a", 0.0), &OFFERS);
    state.update(&at(0.1), &OFFERS);

    state.update(&hovering("a", 0.2), &OFFERS);
    state.update(&at(0.35), &OFFERS);

    assert_eq!(
        state.open(),
        Some("a"),
        "the wait starts again from when it left"
    );
}

#[test]
fn a_click_on_another_row_pins_its_tooltip() {
    let mut state = TooltipState::default();
    let click = |id: &'static str, time| Pointer {
        clicked: Some(id),
        any_click: true,
        ..hovering(id, time)
    };
    state.update(&click("a", 0.0), &OFFERS);

    state.update(&click("b", 1.0), &OFFERS);
    state.update(&at(5.0), &OFFERS);

    assert_eq!(state.open(), Some("b"));
}

#[test]
fn a_click_inside_an_unpinned_tooltip_keeps_it_open() {
    let mut state = TooltipState::default();
    state.update(&hovering("a", 0.0), &OFFERS);

    let click_inside = Pointer {
        any_click: true,
        over_tooltip: true,
        ..at(0.1)
    };
    state.update(&click_inside, &OFFERS);

    assert_eq!(state.open(), Some("a"));
}

#[test]
fn the_tooltip_of_an_offer_gone_closes() {
    let mut state = TooltipState::default();
    state.update(&hovering("a", 0.0), &OFFERS);

    state.update(&at(0.0), &["d"]);

    assert_eq!(state.open(), None);
}

#[test]
fn the_pill_reads_copied_for_two_seconds() {
    let mut state = TooltipState::default();
    state.copied("a", 1.0);

    assert!(state.shows_copied("a", 2.9));
    assert!(!state.shows_copied("b", 2.9));
    assert!(!state.shows_copied("a", 3.0));
    let redraw = state.update(&at(2.0), &OFFERS);
    assert_eq!(redraw, Some(Duration::from_secs_f64(1.0)));
}

#[test]
fn the_gap_to_the_row_is_on_the_side_of_the_tooltip() {
    let row = Rect::from_min_max(pos2(0.0, 100.0), pos2(420.0, 128.0));
    let above = Rect::from_min_max(pos2(110.0, 20.0), pos2(410.0, 94.0));
    let below = Rect::from_min_max(pos2(110.0, 134.0), pos2(410.0, 200.0));

    assert_eq!(
        reach(above, row),
        Rect::from_min_max(pos2(0.0, 94.0), pos2(420.0, 100.0))
    );
    assert_eq!(
        reach(below, row),
        Rect::from_min_max(pos2(0.0, 128.0), pos2(420.0, 134.0))
    );
}

#[test]
fn a_difference_is_written_with_its_sign() {
    assert_eq!(signed(6.0), "+6");
    assert_eq!(signed(-7.0), "−7");
    assert_eq!(signed(3.5), "+3.5");
}

/// A Comet of level 15, 12 % of quality and 3 sockets, copied.
fn copied_comet() -> Item {
    let text = "Item Class: Skill Gems\nRarity: Gem\nComet\n--------\nLevel: 15\n\
                Quality: +12% (augmented)\n--------\nSockets: G G G";
    Item::parse(&crate::item::CopiedItem::from_text(text).unwrap()).unwrap()
}

/// A Comet offered at `level`, with its `quality` and its `sockets`.
fn comet(level: u32, quality: Option<u32>, sockets: Option<u32>) -> ItemOffer {
    use crate::trade::offers::{OfferItem, OfferQuality};

    ItemOffer {
        id: "g".to_owned(),
        price: None,
        seller: SellerStatus::Online,
        listed: None,
        whisper: String::new(),
        item: OfferItem {
            rarity: None,
            name: None,
            base_type: "Comet".to_owned(),
            item_level: None,
            corrupted: false,
            quality: quality.map(|value| OfferQuality { value, kind: None }),
            gem_level: Some(level),
            gem_sockets: sockets,
            requirements: crate::item::Requirements {
                level: Some(66),
                intelligence: Some(116),
                ..crate::item::Requirements::default()
            },
            waystone_properties: Vec::new(),
            waystone_unread: false,
            modifiers: Vec::new(),
        },
    }
}

/// The gem lines of `offer` as `(text, tint, difference)`.
fn gem_texts(
    offer: &ItemOffer,
    item: &Item,
    i18n: &Translator,
) -> Vec<(String, Color32, Option<f64>)> {
    gem_lines(offer, item, i18n)
        .into_iter()
        .map(|line| {
            (
                line.text,
                line.tint,
                line.difference.map(|difference| difference.value),
            )
        })
        .collect()
}

#[test]
fn the_quality_and_sockets_of_an_offer_of_gem_show_their_difference_white_at_their_top() {
    let i18n = Translator::new(crate::language::Language::English);
    let copied = copied_comet();
    // The offer of the mock-up: without quality, faint at 0 with its loss.
    assert_eq!(
        gem_texts(&comet(16, None, Some(4)), &copied, &i18n),
        [
            ("Quality: 0%".to_owned(), color::FAINT, Some(-12.0)),
            ("Sockets: 4".to_owned(), color::WEAK, Some(1.0)),
        ]
    );
    assert_eq!(
        gem_texts(&comet(21, Some(23), Some(5)), &copied, &i18n),
        [
            ("Quality: 23%".to_owned(), color::TEXT, Some(11.0)),
            ("Sockets: 5".to_owned(), color::TEXT, Some(2.0)),
        ]
    );
    assert_eq!(
        gem_texts(&comet(20, Some(20), Some(3)), &copied, &i18n),
        [
            ("Quality: 20%".to_owned(), color::TEXT, Some(8.0)),
            ("Sockets: 3".to_owned(), color::WEAK, None),
        ]
    );
    assert!(
        gem_lines(&comet(20, Some(20), Some(3)), &copied, &i18n)
            .iter()
            .all(|line| line.difference.is_none_or(|d| d.verdict == Verdict::Better))
    );
}

#[test]
fn a_gem_without_quality_has_no_quality_line_and_another_item_no_gem_line() {
    let i18n = Translator::new(crate::language::Language::English);
    let owners = Item::parse(
        &crate::item::CopiedItem::from_text(include_str!(
            "../../item/test-items/en-owner-skill-gem.txt"
        ))
        .unwrap(),
    )
    .unwrap();
    // Neither the owner's gem nor the offer has a quality; sockets the site doesn't give
    // aren't written.
    assert_eq!(
        gem_texts(&comet(16, None, Some(2)), &owners, &i18n),
        [("Sockets: 2".to_owned(), color::WEAK, None)]
    );
    assert_eq!(gem_texts(&comet(16, None, None), &owners, &i18n), []);
    // A copy without its sockets: no difference, never from 0.
    let without = Item::parse(
        &crate::item::CopiedItem::from_text(
            "Item Class: Skill Gems\nRarity: Gem\nComet\n--------\nLevel: 15",
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        gem_texts(&comet(16, None, Some(2)), &without, &i18n),
        [("Sockets: 2".to_owned(), color::WEAK, None)]
    );
    assert_eq!(
        gem_texts(&comet(16, Some(20), Some(4)), &owners_waystone(), &i18n),
        []
    );
}

#[test]
fn an_offer_of_gem_writes_the_class_and_its_level_in_its_color() {
    let i18n = Translator::new(crate::language::Language::English);
    let copied = copied_comet();
    let level_color = |offer: &ItemOffer| {
        let job = meta(offer, &copied, &i18n, 300.0);
        job.sections
            .iter()
            .find(|section| {
                job.text[section.byte_range.start.0..section.byte_range.end.0].starts_with("Level")
            })
            .map(|section| section.format.color)
    };
    let mut offer = comet(21, Some(23), Some(5));
    offer.item.corrupted = true;
    assert_eq!(
        meta(&offer, &copied, &i18n, 300.0).text,
        "Skill Gems · Level\u{a0}21 · Corrupted"
    );
    assert_eq!(level_color(&offer), Some(color::VAAL));
    assert_eq!(level_color(&comet(20, None, None)), Some(color::TIER_1));
    assert_eq!(level_color(&comet(16, None, None)), Some(color::TEXT));
    // French.
    let french = Translator::new(crate::language::Language::French);
    let flame_wall = Item::parse(
        &crate::item::CopiedItem::from_text(include_str!("../../item/test-items/fr-skill-gem.txt"))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        meta(&comet(16, None, None), &flame_wall, &french, 300.0).text,
        "Gemmes d'aptitude · Niveau\u{a0}16"
    );
    // An offer of gem for another copied item: its base as it is.
    assert_eq!(
        meta(&offer, &owners_waystone(), &i18n, 300.0).text,
        "Comet · Corrupted"
    );
}

#[test]
fn the_requirements_are_written_for_a_gem_a_weapon_or_an_armour_only() {
    let i18n = Translator::new(crate::language::Language::English);
    let parse =
        |text: &str| Item::parse(&crate::item::CopiedItem::from_text(text).unwrap()).unwrap();
    let written = |offer: &ItemOffer, item: &Item| {
        requirements(offer, item, &i18n, 300.0).map(|job| job.text)
    };
    let offer = comet(16, None, Some(4));
    assert_eq!(
        written(&offer, &copied_comet()).as_deref(),
        Some("Requires Level 66 · 116 Int")
    );
    let bow = parse(include_str!(
        "../../item/test-items/en-rare-bow-fractured.txt"
    ));
    assert_eq!(
        written(&offer, &bow).as_deref(),
        Some("Requires Level 66 · 116 Int")
    );
    let armour = parse(include_str!(
        "../../item/test-items/fr-rare-body-armour.txt"
    ));
    assert!(
        written(&offer, &armour).is_some(),
        "a body armour, in French"
    );
    // Neither an amulet nor a waystone.
    let amulet = parse(include_str!(
        "../../item/test-items/en-owner-rare-amulet-catalyst.txt"
    ));
    assert_eq!(written(&offer, &amulet), None);
    assert_eq!(written(&offer, &owners_waystone()), None);
    // A magic weapon, known by its whole name: from the base of the offer.
    let mace = parse(include_str!("../../item/test-items/en-magic-mace.txt"));
    let mut maul = offer.clone();
    maul.item.base_type = "Temple Maul".to_owned();
    assert_eq!(
        written(&maul, &mace).as_deref(),
        Some("Requires Level 66 · 116 Int")
    );
    // An offer without requirements.
    let mut free = offer.clone();
    free.item.requirements = crate::item::Requirements::default();
    assert_eq!(written(&free, &bow), None);
}
