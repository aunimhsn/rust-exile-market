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
            waystone: Vec::new(),
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

/// A waystone of `base_type` offered with its `properties`.
fn waystone(base_type: &str, properties: &[(WaystoneProperty, i32)]) -> ItemOffer {
    let mut offer = amulet(0, Catalyst::Mana, "", &[]);
    offer.item.base_type = base_type.to_owned();
    offer.item.quality = None;
    offer.item.item_level = Some(81);
    offer.item.waystone = properties
        .iter()
        .map(|&(property, value)| WaystoneValue { property, value })
        .collect();
    offer
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
