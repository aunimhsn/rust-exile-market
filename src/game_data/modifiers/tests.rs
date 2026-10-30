//! The modifier tables: the tier and range the advanced copy writes, found again on the test
//! items; and the tiers found from the values of an item copied from the in-game trade, its
//! hybrids, its affixes in doubt, its summed lines, its room and the bounds of the search.

use super::*;
use crate::item::{CopiedItem, Item, ItemState, Rarity, StatLine};

#[test]
fn every_table_of_the_embedded_file_is_read() {
    let text = include_str!("../../../assets/game-data/modifiers.ndjson");

    assert_eq!(TABLES.len(), text.lines().count());
}

fn range(min: f64, max: f64) -> Range {
    Range { min, max }
}

/// A base of the equipment with `tags`.
fn equipment(tags: &[&str]) -> BaseSpawn {
    BaseSpawn {
        domain: Domain::Item,
        tags: tags.iter().map(|&tag| tag.to_owned()).collect(),
    }
}

#[test]
fn a_table_rolls_on_the_bases_of_its_domain_only() {
    let charges = TABLES
        .iter()
        .find(|table| table.kind == "FlaskIncreasedMaxCharges")
        .expect("the charges of a flask");
    let base = |name| bases::spawn(Language::English, name).expect("a base");

    // Its only tag, `default`, is a tag of every base.
    assert_eq!(charges.spawn, [("default".to_owned(), 1)]);
    assert!(charges.rolls_on(base("Ultimate Life Flask")));
    assert!(!charges.rolls_on(base("Gold Ring")));
    assert!(!charges.rolls_on(base("Ruby")));

    // A desecrated table rolls on the equipment and the jewels: the presence of Amanamu on
    // the helmets, and a hybrid of the Abyss on the jewels.
    let desecrated = |kind: &str| {
        TABLES
            .iter()
            .find(|table| table.kind == kind && table.domain == Domain::Desecrated)
            .expect(kind)
    };
    let presence = desecrated("PresenceRadius");
    assert!(presence.rolls_on(base("Divine Crown")));
    assert!(!presence.rolls_on(base("Gold Ring")));
    let jewel = desecrated("HybridAbyssModJewelSpellDamageArmour");
    assert!(jewel.rolls_on(base("Ruby")));
    assert!(!jewel.rolls_on(base("Ultimate Life Flask")));
}

#[test]
fn the_tables_are_ordinary_then_of_the_genesis_tree_then_desecrated() {
    // Of two tables that hold a copied modifier as well, the first is taken: the ordinary one,
    // then the minion one of the Genesis tree.
    let order: Vec<_> = TABLES
        .iter()
        .map(|table| (table.domain == Domain::Desecrated, table.genesis))
        .collect();

    assert!(order.is_sorted());
    for kind in [(false, true), (true, false), (true, true)] {
        assert!(order.contains(&kind), "{kind:?}");
    }
}

/// A tier number and the ranges of its values, as the game writes them.
type TierRanges = (u32, Vec<Range>);

/// The tier found from the tables for each prefix and suffix of `text`, and the tier the
/// advanced copy writes, with its ranges: (found, written). A magic item gives its
/// `base_type`, found among the names of the trade site in the application.
fn tiers_of(text: &str, base_type: Option<&str>) -> Vec<(Option<TierRanges>, TierRanges)> {
    let item = Item::parse(&CopiedItem::from_text(text).expect("an item")).expect("read");
    let base_type = base_type.unwrap_or(item.base_type());
    item.modifiers()
        .iter()
        .filter(|modifier| {
            matches!(modifier.kind, ModifierKind::Explicit(Some(_))) && modifier.tier.is_some()
        })
        .map(|modifier| {
            let matches = modifier_stats(item.language(), modifier, None);
            let found = modifier_tiers(item.language(), base_type, None, modifier, &matches)
                .and_then(|tiers| {
                    let tier = tiers.tier()?;
                    Some((
                        tier.number,
                        tier.ranges
                            .iter()
                            .filter_map(|range| range.magnitude())
                            .collect(),
                    ))
                });
            let written = (
                modifier.tier.expect("a tier"),
                modifier
                    .lines
                    .iter()
                    .flat_map(|line| &line.rolls)
                    .filter_map(|roll| roll.range)
                    .collect(),
            );
            (found, written)
        })
        .collect()
}

#[test]
fn the_tables_find_the_tier_written_by_the_advanced_copy() {
    let mut found = 0;
    let mut missing = Vec::new();
    for (text, base_type) in [
        (
            include_str!("../../item/test-items/en-rare-crossbow-all-modifiers.txt"),
            None,
        ),
        (
            include_str!("../../item/test-items/en-rare-bow-fractured.txt"),
            None,
        ),
        (
            include_str!("../../item/test-items/en-magic-mace.txt"),
            Some("Temple Maul"),
        ),
        (
            include_str!("../../item/test-items/fr-rare-staff.txt"),
            None,
        ),
        (
            include_str!("../../item/test-items/fr-rare-body-armour.txt"),
            None,
        ),
        (
            include_str!("../../item/test-items/fr-rare-body-armour-two-hybrids.txt"),
            None,
        ),
        (
            include_str!("../../item/test-items/en-owner-rare-sceptre-sanctified.txt"),
            None,
        ),
        (
            include_str!("../../item/test-items/fr-rare-sceptre-sanctified.txt"),
            None,
        ),
    ] {
        for (tier, (number, ranges)) in tiers_of(text, base_type) {
            // The crossbow was copied before a patch added a tier above its leech, `of
            // the Desperate` (8–8.9), the S1 of the offers checked on the trade site on
            // 2026-10-27.
            let number = if ranges == [range(7.0, 7.9)] {
                2
            } else {
                number
            };
            if let Some(tier) = tier {
                assert_eq!(tier, (number, ranges));
                found += 1;
            } else {
                missing.push(ranges);
            }
        }
    }
    // With the two modifiers of Amanamu, a lord of the Abyss, among the desecrated tables.
    assert_eq!(found, 39);
    // A line whose stat the tables have under another tier is a mistake; a line the tables
    // don't have isn't: the lightning damage of the mace, copied before a patch changed
    // its ranges.
    assert_eq!(missing, [vec![range(1.0, 4.0), range(46.0, 66.0)]]);
}

#[test]
fn a_sanctified_value_out_of_its_tier_finds_it_by_the_copied_ranges() {
    for text in [
        include_str!("../../item/test-items/en-owner-rare-sceptre-sanctified.txt"),
        include_str!("../../item/test-items/fr-rare-sceptre-sanctified.txt"),
    ] {
        let found: Vec<_> = tiers_of(text, None)
            .into_iter()
            .filter_map(|(found, _)| found)
            .collect();
        // `+74(80-89) to maximum Mana` and `29(30)% reduced Attribute Requirements`.
        assert!(found.contains(&(5, vec![range(80.0, 89.0)])), "{found:?}");
        assert!(found.contains(&(2, vec![range(30.0, 30.0)])), "{found:?}");
    }

    // A line of two values, one of them out of its tier: `4(1-4) to 75(60-71)`.
    let text = include_str!("../../item/test-items/en-owner-rare-sceptre-sanctified.txt")
        .replace("60(60-71)", "75(60-71)");
    let (found, written) = tiers_of(&text, None).swap_remove(0);
    assert_eq!(written, (1, vec![range(1.0, 4.0), range(60.0, 71.0)]));
    assert_eq!(found, Some(written));

    // Without the ranges of the advanced copy, a value out of every tier finds none.
    let mana = modifier(Some(Affix::Prefix), &["+74 to maximum Mana"]);
    assert_eq!(
        tiers("Rattling Sceptre", &mana).and_then(|tiers| Some(tiers.tier()?.number)),
        Some(6),
        "74 is in the range of tier 6"
    );
    let mana = modifier(Some(Affix::Prefix), &["+400 to maximum Mana"]);
    assert_eq!(tiers("Rattling Sceptre", &mana), None);
}

/// A modifier of `affix` with `lines`, without the ranges of the advanced copy: found by
/// the values the stats read in the text.
fn modifier(affix: Option<Affix>, lines: &[&str]) -> Modifier {
    Modifier {
        kind: ModifierKind::Explicit(affix),
        origin: None,
        name: None,
        tier: None,
        tags: Vec::new(),
        unrevealed: false,
        lines: lines
            .iter()
            .map(|text| StatLine {
                text: (*text).to_owned(),
                rolls: Vec::new(),
            })
            .collect(),
        doubt: None,
    }
}

/// The tiers of `modifier` on `base_type`, in English.
fn tiers(base_type: &str, modifier: &Modifier) -> Option<ModifierTiers> {
    let matches = modifier_stats(Language::English, modifier, None);
    modifier_tiers(Language::English, base_type, None, modifier, &matches)
}

#[test]
fn a_tier_is_numbered_among_the_tiers_of_the_base() {
    let lightning = modifier(Some(Affix::Prefix), &["Adds 10 to 273 Lightning Damage"]);

    let tiers = tiers("Gemini Crossbow", &lightning).expect("its tiers");

    // The tiers of a two-handed weapon, not those of a one-handed one.
    assert_eq!(tiers.tiers.len(), 10);
    assert_eq!(
        tiers.tiers.first().map(|tier| tier.ranges.clone()),
        Some(vec![range(1.0, 19.0), range(310.0, 358.0)])
    );
    assert_eq!(tiers.tiers.last().map(|tier| tier.level), Some(1));
    assert_eq!(tiers.tier().map(|tier| tier.number), Some(2));
}

#[test]
fn a_line_written_without_number_finds_its_tier_by_the_value_of_its_string() {
    let number_of = |line: &str| {
        let bolts = modifier(Some(Affix::Suffix), &[line]);
        tiers("Gemini Crossbow", &bolts).and_then(|tiers| Some(tiers.tier()?.number))
    };

    assert_eq!(number_of("Loads an additional bolt"), Some(2));
    assert_eq!(number_of("Loads 2 additional bolts"), Some(1));
    assert_eq!(number_of("Loads 3 additional bolts"), None);
}

#[test]
fn the_lines_of_a_hybrid_in_another_order_are_the_same_modifier() {
    let champion = modifier(
        Some(Affix::Prefix),
        &["+100 to Accuracy Rating", "50% increased Physical Damage"],
    );

    let tiers = tiers("Gemini Crossbow", &champion).expect("its tiers");

    let tier = tiers.tier().expect("its tier");
    assert_eq!(tier.number, 4);
    // In the order of the copied lines.
    assert_eq!(tier.ranges, [range(98.0, 123.0), range(45.0, 54.0)]);
}

#[test]
fn a_modifier_outside_the_prefixes_and_suffixes_has_no_tiers() {
    let mut implicit = modifier(None, &["Loads an additional bolt"]);
    implicit.kind = ModifierKind::Implicit;
    // A header the parser doesn't know tells no affix.
    let unknown = modifier(None, &["Adds 10 to 273 Lightning Damage"]);
    let mut desecrated = modifier(Some(Affix::Prefix), &["Adds 10 to 273 Lightning Damage"]);
    desecrated.origin = Some(ModifierOrigin::Desecrated);

    assert_eq!(tiers("Gemini Crossbow", &implicit), None);
    assert_eq!(tiers("Gemini Crossbow", &unknown), None);
    // A base the data doesn't know.
    assert_eq!(tiers("Unknown Crossbow", &desecrated), None);
    // An ordinary modifier desecrated is in the tables.
    assert!(tiers("Gemini Crossbow", &desecrated).is_some());
}

#[test]
fn the_value_of_a_line_without_number_is_divided_as_its_table_shows_it() {
    let tables = read(concat!(
        r#"{"type": "Rage", "affix": "suffix", "lines": [{"template": "Inherent Rage loss starts 1 second later", "divisor": 1000}], "#,
        r#""spawn": [["weapon", 1]], "tiers": [{"level": 1, "ranges": [[[1, 1]]]}, {"level": 50, "ranges": [[[2, 3]]]}]}"#
    ));
    let copied = |values: Vec<f64>, written| Copied {
        lines: vec![CopiedLine {
            template: "Inherent Rage loss starts 1 second later",
            values,
            written,
        }],
        ranges: None,
        tier: None,
        desecrated: false,
    };
    let number = |copied| {
        find(
            &tables,
            TableAffix::Suffix,
            &equipment(&["weapon"]),
            &copied,
            Tie::Best,
        )
        .and_then(|tiers| Some(tiers.tier()?.number))
    };

    // `Inherent Rage loss starts 1 second later` gives the 1000 milliseconds of the stat.
    assert_eq!(number(copied(Vec::new(), Some(1000.0))), Some(2));
    assert_eq!(number(copied(vec![2.5], None)), Some(1));
}

#[test]
fn a_line_without_value_is_found_with_the_other_lines_of_its_modifier() {
    let instant = modifier(
        Some(Affix::Prefix),
        &["Instant Recovery", "50% reduced Amount Recovered"],
    );

    let tiers = tiers("Ultimate Life Flask", &instant).expect("its tiers");

    assert_eq!(tiers.tier().map(|tier| tier.number), Some(1));
    assert_eq!(
        tiers.tier().map(|tier| tier.ranges.clone()),
        Some(vec![range(-50.0, -50.0)])
    );
}

#[test]
fn the_modifiers_of_the_time_lost_jewels_arent_in_the_tables_yet() {
    // Written within "Small Passive Skills in Radius also grant…" by the game, step 13k.
    let ailments = modifier(
        Some(Affix::Suffix),
        &["5% increased chance to inflict Ailments"],
    );

    assert_eq!(tiers("Time-Lost Emerald", &ailments), None);
}

/// A modifier of `affix` with `lines`, the game marking it desecrated.
fn desecrated(affix: Affix, lines: &[&str]) -> Modifier {
    Modifier {
        origin: Some(ModifierOrigin::Desecrated),
        ..modifier(Some(affix), lines)
    }
}

#[test]
fn a_desecrated_table_is_a_modifier_of_one_tier_of_its_own() {
    // The cost efficiency of Kurgal, written twice with the same range on the maces.
    let efficiency = desecrated(Affix::Suffix, &["12% increased Cost Efficiency of Attacks"]);
    // The flask charges of Kurgal (10–15) and of Ulaman (10–18) on the belts.
    let charges = |value: u32| {
        desecrated(
            Affix::Prefix,
            &[&format!(
                "{value}% chance for Flasks you use to not consume Charges"
            )],
        )
    };
    let only = |base_type: &str, modifier: &Modifier| {
        tiers(base_type, modifier).map(|tiers| {
            let ranges = tiers.tier().map(|tier| tier.ranges.clone());
            (tiers.single(), ranges)
        })
    };

    assert_eq!(
        only("Temple Maul", &efficiency),
        Some((true, Some(vec![range(8.0, 15.0)])))
    );
    // Not a T1 and a T2 of one modifier: each its own tier, the first of the file on a tie.
    assert_eq!(
        only("Heavy Belt", &charges(12)),
        Some((true, Some(vec![range(10.0, 15.0)])))
    );
    assert_eq!(
        only("Heavy Belt", &charges(17)),
        Some((true, Some(vec![range(10.0, 18.0)])))
    );
    // The presence of Amanamu on a helmet, which the game always marks desecrated.
    let presence = ["30% increased Presence Area of Effect"];
    assert_eq!(
        only("Divine Crown", &desecrated(Affix::Suffix, &presence)),
        Some((true, Some(vec![range(25.0, 35.0)])))
    );
    assert_eq!(
        only("Divine Crown", &modifier(Some(Affix::Suffix), &presence)),
        None
    );
}

#[test]
fn of_two_tiers_as_good_the_one_the_advanced_copy_wrote_is_taken() {
    // A desecrated table of one tier first, then an ordinary one whose T3 has the same range.
    let tables = read(concat!(
        r##"{"type": "Lord", "affix": "prefix", "domain": "desecrated", "lines": [{"template": "# to maximum Life"}], "##,
        r#""spawn": [["helmet", 1]], "tiers": [{"level": 65, "ranges": [[[10, 20]]]}]}"#,
        "\n",
        r##"{"type": "Life", "affix": "prefix", "lines": [{"template": "# to maximum Life"}], "##,
        r#""spawn": [["helmet", 1]], "tiers": [{"level": 1, "ranges": [[[10, 20]]]}, {"level": 30, "ranges": [[[21, 30]]]}, {"level": 60, "ranges": [[[31, 40]]]}]}"#
    ));
    let copied = |tier| Copied {
        lines: vec![CopiedLine {
            template: "# to maximum Life",
            values: vec![15.0],
            written: None,
        }],
        ranges: Some(vec![range(10.0, 20.0)]),
        tier,
        // A desecrated line: an ordinary modifier, or a lord's.
        desecrated: true,
    };
    let found = |tier| {
        find(
            &tables,
            TableAffix::Prefix,
            &equipment(&["helmet"]),
            &copied(tier),
            Tie::Best,
        )
        .and_then(|tiers| Some((tiers.single(), tiers.tier()?.number)))
    };

    // A T3 isn't hidden by the modifier of one tier before it.
    assert_eq!(found(Some(3)), Some((false, 3)));
    assert_eq!(found(Some(1)), Some((true, 1)));
    // Without the number, the first of the file.
    assert_eq!(found(None), Some((true, 1)));
}

#[test]
fn two_tables_with_the_same_lines_in_another_order_are_two_modifiers() {
    let table = |kind: &str, lines: [&str; 2], level: u32| {
        format!(
            r#"{{"type": "{kind}", "affix": "prefix", "lines": [{{"template": "{}"}}, {{"template": "{}"}}], "spawn": [["weapon", 1]], "tiers": [{{"level": {level}, "ranges": [[[1, 5]], [[10, 20]]]}}]}}"#,
            lines[0], lines[1]
        )
    };
    let tables = read(
        &[
            table("Hybrid", ["A #", "B #"], 1),
            table("Hybrid", ["B #", "A #"], 50),
        ]
        .join("\n"),
    );
    let copied = Copied {
        lines: vec![
            CopiedLine {
                template: "B #",
                values: vec![15.0],
                written: None,
            },
            CopiedLine {
                template: "A #",
                values: vec![3.0],
                written: None,
            },
        ],
        ranges: None,
        tier: None,
        desecrated: false,
    };

    let tiers = find(
        &tables,
        TableAffix::Prefix,
        &equipment(&["weapon"]),
        &copied,
        Tie::Best,
    )
    .expect("its tiers");

    // The first table holds the copied values, its lines put in the copied order; the
    // second, its own lines in the copied order, holds neither 15 in 1–5 nor 3 in 10–20.
    assert_eq!(tiers.tiers.len(), 1);
    assert_eq!(tiers.tiers.first().map(|tier| tier.level), Some(1));
    assert_eq!(
        tiers.tier().map(|tier| tier.ranges.clone()),
        Some(vec![range(10.0, 20.0), range(1.0, 5.0)])
    );
}

#[test]
fn a_line_with_a_divisor_has_one_value() {
    // The divisor is for the value of a line written without number, a single value.
    for table in TABLES.iter() {
        for (index, line) in table.lines.iter().enumerate() {
            if (line.divisor - 1.0).abs() > f64::EPSILON {
                for tier in &table.tiers {
                    assert_eq!(
                        tier.ranges.get(index).map(Vec::len),
                        Some(1),
                        "{}",
                        line.template
                    );
                }
            }
        }
    }
}

/// The tiara and the bow copied by the owner from the in-game trade, English and French.
const TIARA_EN: &str =
    include_str!("../../item/test-items/en-owner-trade-rare-helmet-fractured.txt");
const TIARA_FR: &str = include_str!("../../item/test-items/fr-trade-rare-helmet-fractured.txt");
const BOW_EN: &str = include_str!("../../item/test-items/en-owner-trade-rare-bow-mirrored.txt");
const BOW_FR: &str = include_str!("../../item/test-items/fr-trade-rare-bow-mirrored.txt");

/// The tier number of each affix a modifier can be.
type AffixTiers = Vec<(Affix, u32)>;

/// The texts of the lines of each modifier read from its values, and the tier number of
/// each affix it can be.
type LinesRead = Vec<(Vec<String>, AffixTiers)>;

/// The text of each line split, and the affix and tier number of its pure modifier.
type Added = Vec<(String, Affix, u32)>;

fn parse(text: &str) -> Item {
    Item::parse(&CopiedItem::from_text(text).expect("an item")).expect("read")
}

/// The tiers of `modifiers` found from their values on the base of `item`, with at most
/// `limits` prefixes and suffixes.
fn found_tiers(
    item: &Item,
    modifiers: &[Modifier],
    (prefixes, suffixes): (u32, u32),
) -> Vec<FoundTiers> {
    let limits = AffixLimits { prefixes, suffixes };
    trade_tiers(
        item.language(),
        item.base_type(),
        item.item_level(),
        modifiers,
        limits,
        false,
    )
}

/// For each modifier read from its values, the texts of its lines and the tier number of
/// each affix it can be.
fn lines_read(modifiers: &[Modifier], found: &[FoundTiers]) -> LinesRead {
    found
        .iter()
        .map(|found| {
            let lines = modifiers[found.modifiers.clone()]
                .iter()
                .flat_map(|modifier| &modifier.lines)
                .map(|line| line.text.clone())
                .collect();
            let affixes = found
                .affixes
                .iter()
                .map(|(affix, tiers)| (*affix, tiers.tier().expect("its tier").number))
                .collect();
            (lines, affixes)
        })
        .collect()
}

/// For each line read as two modifiers, its text and the affix and tier number of the
/// other one, a pure modifier of the line.
fn added(modifiers: &[Modifier], found: &[FoundTiers]) -> Added {
    found
        .iter()
        .filter_map(|found| {
            let added = found.added.as_ref()?;
            let text = modifiers.get(added.line)?.lines.first()?.text.clone();
            Some((text, added.affix, added.tiers.tier()?.number))
        })
        .collect()
}

/// For each part of a line split in two neighbouring tiers, the tier number of its tier and
/// of the neighbouring one.
fn neighbours(found: &[FoundTiers]) -> Vec<(u32, u32)> {
    found
        .iter()
        .flat_map(|found| {
            let added = found.added.as_ref().map(|added| &added.tiers);
            found.affixes.iter().map(|(_, tiers)| tiers).chain(added)
        })
        .filter_map(|tiers| {
            let neighbour = tiers.tiers.get(tiers.neighbour?)?;
            Some((tiers.tier()?.number, neighbour.number))
        })
        .collect()
}

/// For each modifier read from its values, how many lines it has and the tier number of
/// each affix it can be: the same in every language.
fn summary(modifiers: &[Modifier], found: &[FoundTiers]) -> Vec<(usize, AffixTiers)> {
    lines_read(modifiers, found)
        .into_iter()
        .map(|(lines, affixes)| (lines.len(), affixes))
        .collect()
}

#[test]
fn the_tiers_of_a_tiara_copied_from_the_in_game_trade_are_found_from_its_values() {
    use Affix::{Prefix, Suffix};
    for text in [TIARA_EN, TIARA_FR] {
        let item = parse(text);

        let found = found_tiers(&item, item.modifiers(), (3, 3));

        assert_eq!(
            summary(item.modifiers(), &found),
            [
                // The fractured `41% increased Energy Shield` and `+34 to maximum Mana`,
                // one hybrid: read apart, the tiara would have four prefixes.
                (2, vec![(Prefix, 1)]),
                (1, vec![(Prefix, 1)]),
                // `18% increased Rarity of Items found` rolls as a prefix and as a suffix:
                // the prefixes are full.
                (1, vec![(Suffix, 1)]),
                (1, vec![(Suffix, 1)]),
                // `+40% to Cold Resistance`.
                (1, vec![(Suffix, 2)]),
                // The desecrated `+69 to maximum Energy Shield`.
                (1, vec![(Prefix, 1)]),
            ]
        );
        let hybrid = found.first().and_then(|found| found.affixes.first());
        assert_eq!(
            hybrid.and_then(|(_, tiers)| Some(tiers.tier()?.ranges.clone())),
            Some(vec![range(39.0, 42.0), range(33.0, 39.0)]),
            "in the order of the copied lines"
        );
    }
}

#[test]
fn an_affix_stays_unknown_while_the_item_has_room_for_both() {
    let item = parse(TIARA_EN);
    // Without the desecrated `+69 to maximum Energy Shield`, a prefix is open.
    let modifiers = &item.modifiers()[..item.modifiers().len() - 1];

    let found = found_tiers(&item, modifiers, (3, 3));

    let rarity = found
        .iter()
        .find(|found| {
            modifiers[found.modifiers.clone()][0].lines[0]
                .text
                .contains("Rarity")
        })
        .expect("the rarity");
    let tiers: Vec<_> = rarity
        .affixes
        .iter()
        .map(|(affix, tiers)| {
            let tier = tiers.tier().expect("its tier");
            (*affix, tier.number, tier.ranges.clone())
        })
        .collect();
    // As the mock-up shows it: "T1 16–19 Prefix", "T1 15–18 Suffix".
    assert_eq!(
        tiers,
        [
            (Affix::Prefix, 1, vec![range(16.0, 19.0)]),
            (Affix::Suffix, 1, vec![range(15.0, 18.0)]),
        ]
    );
}

#[test]
fn a_modifier_whose_header_tells_its_affix_takes_its_room() {
    let item = parse(TIARA_EN);
    let mut modifiers = item.modifiers().to_vec();
    // A desecrated prefix not revealed yet in place of `+69 to maximum Energy Shield`.
    let mut unrevealed = modifier(Some(Affix::Prefix), &[]);
    unrevealed.unrevealed = true;
    *modifiers.last_mut().expect("a modifier") = unrevealed;

    let found = found_tiers(&item, &modifiers, (3, 3));

    let rarity = lines_read(&modifiers, &found)
        .into_iter()
        .find(|(lines, _)| lines == &["18% increased Rarity of Items found"]);
    assert_eq!(
        rarity.map(|(_, affixes)| affixes),
        Some(vec![(Affix::Suffix, 1)])
    );
}

#[test]
fn the_lines_of_two_origins_arent_read_as_one_modifier() {
    let item = parse(TIARA_EN);
    let mut modifiers = item.modifiers().to_vec();
    let mana = modifiers
        .iter_mut()
        .find(|modifier| modifier.lines[0].text == "+34 to maximum Mana")
        .expect("the mana");
    mana.origin = None;

    let found = found_tiers(&item, &modifiers, (3, 3));

    let read = lines_read(&modifiers, &found);
    assert_eq!(read[0].0, ["41% increased Energy Shield"]);
    assert_eq!(read[1].0, ["+34 to maximum Mana"]);
}

#[test]
fn without_a_reading_that_fits_the_room_every_affix_is_kept() {
    use Affix::{Prefix, Suffix};
    let item = parse(TIARA_EN);

    // One prefix and one suffix, as on a magic item: no reading fits.
    let found = found_tiers(&item, item.modifiers(), (1, 1));

    assert_eq!(
        summary(item.modifiers(), &found),
        [
            (2, vec![(Prefix, 1)]),
            (1, vec![(Prefix, 1)]),
            (1, vec![(Prefix, 1), (Suffix, 1)]),
            (1, vec![(Suffix, 1)]),
            (1, vec![(Suffix, 2)]),
            (1, vec![(Prefix, 1)]),
        ]
    );
}

#[test]
fn the_tiers_of_a_bow_copied_from_the_in_game_trade_are_found_from_its_values() {
    use Affix::{Prefix, Suffix};
    let item = parse(BOW_EN);

    // Its rune allows a fourth suffix.
    let found = found_tiers(&item, item.modifiers(), (3, 4));

    let read = lines_read(item.modifiers(), &found);
    let line = |text: &str| vec![text.to_owned()];
    let hybrid = vec![
        "258% increased Physical Damage".to_owned(),
        "+195 to Accuracy Rating".to_owned(),
    ];
    assert_eq!(
        read,
        [
            (line("+5% to Critical Hit Chance"), vec![(Suffix, 1)]),
            // `258% increased Physical Damage` adds up two modifiers, which no tier holds
            // alone: the hybrid T1 with the accuracy, 75–79 · 175–200…
            (hybrid, vec![(Prefix, 1)]),
            (line("+25% to Critical Damage Bonus"), vec![(Suffix, 1)]),
            (
                line("+4 to Level of all Projectile Skills"),
                vec![(Suffix, 1)]
            ),
            (
                line("+181% Surpassing chance to fire an additional Arrow"),
                vec![(Suffix, 1)]
            ),
            (line("Adds 36 to 56 Physical Damage"), vec![(Prefix, 1)]),
        ]
    );
    // … and the pure T1, 170–179: 179 and 79, the only way that fits 3 prefixes.
    assert_eq!(
        added(item.modifiers(), &found),
        [("258% increased Physical Damage".to_owned(), Prefix, 1)]
    );
    let ranges = |tiers: &ModifierTiers| tiers.tier().map(|tier| tier.ranges.clone());
    let split = found
        .iter()
        .find_map(|found| Some((&found.affixes.first()?.1, &found.added.as_ref()?.tiers)));
    assert_eq!(
        split.map(|(whole, added)| (ranges(whole), ranges(added))),
        Some((
            Some(vec![range(75.0, 79.0), range(175.0, 200.0)]),
            Some(vec![range(170.0, 179.0)])
        ))
    );
    // With room for two prefixes, the line stays without tier, the accuracy a pure prefix
    // of tier 5.
    let found = found_tiers(&item, item.modifiers(), (2, 4));
    assert_eq!(added(item.modifiers(), &found), []);
    let read = lines_read(item.modifiers(), &found);
    assert!(read.contains(&(line("+195 to Accuracy Rating"), vec![(Prefix, 5)])));
    let summed = |line: &String| line == "258% increased Physical Damage";
    assert!(!read.iter().any(|(lines, _)| lines.iter().any(summed)));

    let item = parse(BOW_FR);
    let found = found_tiers(&item, item.modifiers(), (3, 4));
    assert_eq!(
        summary(item.modifiers(), &found),
        [
            (1, vec![(Suffix, 1)]),
            (1, vec![(Prefix, 1)]),
            (1, vec![(Prefix, 1)]),
            (1, vec![(Suffix, 1)]),
            (1, vec![(Suffix, 1)]),
            (1, vec![(Suffix, 1)]),
            // The desecrated `79% d'Augmentation des Dégâts Physiques` and `+193 au Score
            // de Précision`, one hybrid.
            (2, vec![(Prefix, 1)]),
        ]
    );
}

/// The rare and magic items of the advanced copy, with the base type of a magic one.
const ADVANCED_COPIES: [(&str, Option<&str>); 13] = [
    (
        include_str!("../../item/test-items/en-rare-crossbow-all-modifiers.txt"),
        None,
    ),
    (
        include_str!("../../item/test-items/en-rare-bow-fractured.txt"),
        None,
    ),
    (
        include_str!("../../item/test-items/en-magic-mace.txt"),
        Some("Temple Maul"),
    ),
    (
        include_str!("../../item/test-items/fr-rare-staff.txt"),
        None,
    ),
    (
        include_str!("../../item/test-items/fr-rare-body-armour.txt"),
        None,
    ),
    (
        include_str!("../../item/test-items/en-owner-rare-sceptre-sanctified.txt"),
        None,
    ),
    (
        include_str!("../../item/test-items/fr-rare-sceptre-sanctified.txt"),
        None,
    ),
    (
        include_str!("../../item/test-items/en-owner-rare-helmet-runeforged.txt"),
        None,
    ),
    (
        include_str!("../../item/test-items/fr-rare-helmet-runeforged.txt"),
        None,
    ),
    (
        include_str!("../../item/test-items/en-owner-rare-focus-unrevealed.txt"),
        None,
    ),
    (
        include_str!("../../item/test-items/fr-rare-focus-unrevealed.txt"),
        None,
    ),
    (
        include_str!("../../item/test-items/en-owner-rare-ring-breach.txt"),
        None,
    ),
    (
        include_str!("../../item/test-items/fr-rare-ring-breach.txt"),
        None,
    ),
];

/// A modifier of the advanced copy written as the in-game trade writes it.
struct Written {
    /// The indexes of the modifiers its lines make, one per stat.
    modifiers: std::ops::Range<usize>,
    affix: Affix,
    /// Its tier, found by the ranges of the copy.
    tier: Option<u32>,
    /// Whether its values are within these ranges: not a sanctified value out of its tier.
    within: bool,
}

/// The explicit modifiers of an advanced copy as the in-game trade writes them, one per
/// stat without header nor range, and what the copy wrote of each.
fn as_in_game_trade(item: &Item, base_type: &str) -> (Vec<Modifier>, Vec<Written>) {
    let mut modifiers = Vec::new();
    let mut written = Vec::new();
    for modifier in item.modifiers() {
        let ModifierKind::Explicit(Some(affix)) = modifier.kind else {
            modifiers.push(modifier.clone());
            continue;
        };
        if modifier.unrevealed {
            modifiers.push(modifier.clone());
            continue;
        }
        let matches = modifier_stats(item.language(), modifier, None);
        let tier = modifier_tiers(item.language(), base_type, None, modifier, &matches)
            .and_then(|tiers| Some(tiers.tier()?.number));
        let rolls = modifier.lines.iter().flat_map(|line| &line.rolls);
        let within = rolls
            .filter_map(|roll| Some((roll.value, roll.range?)))
            .all(|(value, range)| range.min <= value && value <= range.max);
        let start = modifiers.len();
        for line_match in &matches {
            let mut lines = modifier.lines[line_match.lines.clone()].to_vec();
            for roll in lines.iter_mut().flat_map(|line| &mut line.rolls) {
                roll.range = None;
            }
            modifiers.push(Modifier {
                kind: ModifierKind::Explicit(None),
                origin: modifier.origin,
                name: None,
                tier: None,
                tags: Vec::new(),
                unrevealed: false,
                lines,
                doubt: None,
            });
        }
        written.push(Written {
            modifiers: start..modifiers.len(),
            affix,
            tier,
            within,
        });
    }
    (modifiers, written)
}

#[test]
fn the_tiers_found_from_the_values_alone_are_those_of_the_advanced_copy() {
    let mut found = 0;
    let mut without_tier = Vec::new();
    let mut unknown_affix = Vec::new();
    for (text, base_type) in ADVANCED_COPIES {
        let item = parse(text);
        let base_type = base_type.unwrap_or(item.base_type());
        let limit = if item.rarity() == Rarity::Magic { 1 } else { 3 };
        let limits = AffixLimits {
            prefixes: limit,
            suffixes: limit,
        };
        let (modifiers, written) = as_in_game_trade(&item, base_type);

        let read = trade_tiers(
            item.language(),
            base_type,
            item.item_level(),
            &modifiers,
            limits,
            item.has_state(ItemState::Sanctified),
        );

        for Written {
            modifiers: lines,
            affix,
            tier,
            within,
        } in written
        {
            let text = modifiers[lines.start].lines[0].text.clone();
            let Some(read) = read
                .iter()
                .find(|read| read.modifiers.contains(&lines.start))
            else {
                without_tier.push(text);
                continue;
            };
            // Never a hybrid made of the lines of two modifiers, nor one cut in two, nor a
            // line split.
            assert_eq!(read.modifiers, lines, "{text}");
            assert_eq!(read.added, None, "{text}");
            let tiers: Vec<_> = read
                .affixes
                .iter()
                .map(|(affix, tiers)| (*affix, tiers.tier().map(|tier| tier.number)))
                .collect();
            let same = tiers
                .iter()
                .any(|&(read, number)| read == affix && (!within || number == tier));
            assert!(same, "{text}: {tiers:?}, written {affix:?} {tier:?}");
            if tiers.len() > 1 {
                unknown_affix.push(text);
            } else {
                found += 1;
            }
        }
    }
    // With the 4 modifiers of Amanamu, a lord of the Abyss, among the desecrated tables, and
    // `Reinforcing` of the Genesis tree on the Breach Rings.
    assert_eq!(found, 59);
    assert_eq!(
        without_tier,
        [
            // Two modifiers of an essence, without tier in the advanced copy too.
            "20% chance to gain Onslaught on Killing Hits with this Weapon",
            "+3 to Level of all Attack Skills",
            // Copied before a patch changed its ranges.
            "Adds 1 to 50 Lightning Damage",
            // A sanctified value out of its tier, 29(30)%.
            "29% reduced Attribute Requirements",
            "29% de Réduction du prérequis d'Attributs",
        ]
    );
    // The rarity of the runeforged crown rolls as both: with two prefixes and two other
    // suffixes, the suffix of Amanamu among them, it has room for either.
    assert_eq!(
        unknown_affix,
        [
            "15% increased Rarity of Items found",
            "15% d'Augmentation de la Rareté des Objets cédés",
        ]
    );
}

/// The texts of the lines of `modifier`.
fn texts(modifier: &Modifier) -> Vec<String> {
    modifier
        .lines
        .iter()
        .map(|line| line.text.clone())
        .collect()
}

/// The range of each value of the lines of `modifier`, as the game writes it.
fn written_ranges(modifier: &Modifier) -> Vec<Option<Range>> {
    modifier
        .lines
        .iter()
        .flat_map(|line| &line.rolls)
        .map(|roll| roll.range.and_then(Range::magnitude))
        .collect()
}

#[test]
fn the_lines_read_again_from_their_values_are_the_modifiers_of_the_advanced_copy() {
    let mut read_again = 0;
    for (text, base_type) in ADVANCED_COPIES {
        let item = parse(text);
        let base_type = base_type.unwrap_or(item.base_type());
        let limit = if item.rarity() == Rarity::Magic { 1 } else { 3 };
        let limits = AffixLimits {
            prefixes: limit,
            suffixes: limit,
        };
        let (modifiers, written) = as_in_game_trade(&item, base_type);

        let read = with_trade_tiers(
            item.language(),
            base_type,
            item.item_level(),
            &modifiers,
            limits,
            item.has_state(ItemState::Sanctified),
        );

        let advanced = item.modifiers().iter().filter(|modifier| {
            matches!(modifier.kind, ModifierKind::Explicit(Some(_))) && !modifier.unrevealed
        });
        for (modifier, written) in advanced.zip(written) {
            let again = read.iter().find(|again| texts(again) == texts(modifier));
            // Read with its affix, not in doubt.
            let read = |again: &&Modifier| {
                matches!(again.kind, ModifierKind::Explicit(Some(_))) && again.doubt.is_none()
            };
            let Some(again) = again.filter(read) else {
                continue;
            };
            let text = &modifier.lines[0].text;
            assert_eq!(again.kind, modifier.kind, "{text}");
            assert_eq!(again.origin, modifier.origin, "{text}");
            if written.within {
                // Without tier when the copy writes none (`Reinforcing` of the Genesis tree).
                assert_eq!(again.tier, modifier.tier.and(written.tier), "{text}");
                assert_eq!(written_ranges(again), written_ranges(modifier), "{text}");
            }
            read_again += 1;
        }
        // The other lines stay as they were.
        let lines =
            |modifiers: &[Modifier]| -> Vec<String> { modifiers.iter().flat_map(texts).collect() };
        assert_eq!(lines(&read), lines(&modifiers));
    }
    // With `Reinforcing` of the Breach Rings, read again without tier, as the copy writes it.
    assert_eq!(read_again, 59);
}

#[test]
fn the_lines_of_a_tiara_read_again_join_its_hybrid_and_keep_an_unknown_affix() {
    let item = parse(TIARA_EN);
    let read = |modifiers: &[Modifier]| {
        let limits = AffixLimits {
            prefixes: 3,
            suffixes: 3,
        };
        with_trade_tiers(
            item.language(),
            item.base_type(),
            item.item_level(),
            modifiers,
            limits,
            false,
        )
    };
    let explicit = |modifiers: Vec<Modifier>| -> Vec<(Vec<String>, ModifierKind, Option<u32>)> {
        modifiers
            .into_iter()
            .filter(|modifier| matches!(modifier.kind, ModifierKind::Explicit(_)))
            .map(|modifier| (texts(&modifier), modifier.kind, modifier.tier))
            .collect()
    };
    let line = |text: &str| vec![text.to_owned()];
    let prefix = ModifierKind::Explicit(Some(Affix::Prefix));
    let suffix = ModifierKind::Explicit(Some(Affix::Suffix));

    let full = read(item.modifiers());
    // Without the desecrated `+69 to maximum Energy Shield`, a prefix is open.
    let open = read(&item.modifiers()[..item.modifiers().len() - 1]);

    let hybrid = (
        vec![
            "41% increased Energy Shield".to_owned(),
            "+34 to maximum Mana".to_owned(),
        ],
        prefix,
        Some(1),
    );
    let joined = full.iter().find(|modifier| modifier.lines.len() == 2);
    assert_eq!(
        joined.map(written_ranges),
        Some(vec![Some(range(39.0, 42.0)), Some(range(33.0, 39.0))]),
    );
    assert_eq!(
        joined.map(|modifier| modifier.origin),
        Some(Some(ModifierOrigin::Fractured))
    );
    assert_eq!(
        explicit(full),
        [
            hybrid.clone(),
            (line("98% increased Energy Shield"), prefix, Some(1)),
            (line("18% increased Rarity of Items found"), suffix, Some(1)),
            (line("+43% to Fire Resistance"), suffix, Some(1)),
            (line("+40% to Cold Resistance"), suffix, Some(2)),
            (line("+69 to maximum Energy Shield"), prefix, Some(1)),
        ]
    );
    assert_eq!(
        explicit(open),
        [
            hybrid,
            (line("98% increased Energy Shield"), prefix, Some(1)),
            // A prefix or a suffix: left without affix nor tier.
            (
                line("18% increased Rarity of Items found"),
                ModifierKind::Explicit(None),
                None
            ),
            (line("+43% to Fire Resistance"), suffix, Some(1)),
            (line("+40% to Cold Resistance"), suffix, Some(2)),
        ]
    );
}

#[test]
fn the_lines_of_a_tiara_read_again_keep_the_tiers_of_the_affix_in_doubt() {
    let item = parse(TIARA_EN);
    let limits = AffixLimits {
        prefixes: 3,
        suffixes: 3,
    };
    let read = |modifiers: &[Modifier]| {
        with_trade_tiers(
            item.language(),
            item.base_type(),
            item.item_level(),
            modifiers,
            limits,
            false,
        )
    };
    let find = |modifiers: &[Modifier], words: &str| {
        modifiers
            .iter()
            .find(|modifier| modifier.lines[0].text.contains(words))
            .cloned()
            .expect(words)
    };
    let tier = |affix, min, max| AffixTier {
        affix,
        tier: 1,
        hides_number: false,
        ranges: vec![range(min, max)],
    };
    let desecrated = item.modifiers().len() - 1;
    // A third resistance in place of the desecrated `+69 to maximum Energy Shield`: the
    // suffixes full, a prefix open.
    let mut resistances = item.modifiers().to_vec();
    resistances[desecrated].lines = vec![StatLine::parse("+30% to Lightning Resistance")];
    resistances[desecrated].origin = None;

    let full = read(item.modifiers());
    // Without the desecrated `+69 to maximum Energy Shield`, a prefix is open.
    let open = find(&read(&item.modifiers()[..desecrated]), "Rarity");
    let rarity_prefix = find(&read(&resistances), "Rarity");

    // A suffix, since the prefixes are full; a prefix, since the suffixes are full: nothing
    // left in doubt.
    let rarity = find(&full, "Rarity");
    assert_eq!(rarity.kind, ModifierKind::Explicit(Some(Affix::Suffix)));
    assert_eq!(rarity.doubt, None);
    assert_eq!(
        rarity_prefix.kind,
        ModifierKind::Explicit(Some(Affix::Prefix))
    );
    assert_eq!(rarity_prefix.doubt, None);
    // A prefix or a suffix: the tier of each, the line without range.
    assert_eq!(
        open.doubt,
        Some(TierDoubt::EitherAffix {
            prefix: tier(Affix::Prefix, 16.0, 19.0),
            suffix: tier(Affix::Suffix, 15.0, 18.0),
        })
    );
    assert_eq!(written_ranges(&open), [None]);
}

/// Modifiers of one line each, as the in-game trade writes them, with their values.
fn trade_lines(lines: &[&str]) -> Vec<Modifier> {
    lines
        .iter()
        .map(|line| Modifier {
            lines: vec![StatLine::parse(line)],
            ..modifier(None, &[])
        })
        .collect()
}

/// `modifiers` of an item of `base_type` and `item_level`, in English, read again with the
/// tiers found from their values, with at most `(prefixes, suffixes)`.
fn read_again(
    base_type: &str,
    item_level: u32,
    modifiers: &[Modifier],
    (prefixes, suffixes): (u32, u32),
) -> Vec<Modifier> {
    let limits = AffixLimits { prefixes, suffixes };
    with_trade_tiers(
        Language::English,
        base_type,
        Some(item_level),
        modifiers,
        limits,
        false,
    )
}

/// A part of a split line of `affix`, in `tiers`, over `ranges`.
fn part(affix: Affix, tiers: &[u32], ranges: &[(f64, f64)]) -> PartTier {
    PartTier {
        affix,
        tiers: tiers.to_vec(),
        hides_number: false,
        ranges: ranges.iter().map(|&(min, max)| range(min, max)).collect(),
    }
}

/// The modifier of `read` with a split line.
fn split_of(read: &[Modifier]) -> &Modifier {
    read.iter()
        .find(|modifier| matches!(modifier.doubt, Some(TierDoubt::Split { .. })))
        .expect("a split line")
}

#[test]
fn a_split_line_is_read_with_the_other_lines_of_its_hybrid_and_the_tiers_of_each_part() {
    let item = parse(BOW_EN);
    let limits = AffixLimits {
        prefixes: 3,
        suffixes: 4,
    };

    let read = with_trade_tiers(
        item.language(),
        item.base_type(),
        item.item_level(),
        item.modifiers(),
        limits,
        false,
    );

    // The hybrid T1 with the accuracy, and the pure T1: a prefix, two modifiers, without
    // tier, the ranges of each part on the summed line only, the accuracy with the range of
    // the hybrid.
    let split = split_of(&read);
    assert_eq!(
        texts(split),
        ["258% increased Physical Damage", "+195 to Accuracy Rating"]
    );
    assert_eq!(split.kind, ModifierKind::Explicit(Some(Affix::Prefix)));
    assert_eq!(split.tier, None);
    assert_eq!(written_ranges(split), [None, Some(range(175.0, 200.0))]);
    assert_eq!(
        split.doubt,
        Some(TierDoubt::Split {
            line: 0,
            whole: part(Affix::Prefix, &[1], &[(75.0, 79.0)]),
            added: part(Affix::Prefix, &[1], &[(170.0, 179.0)]),
        })
    );
    // The other lines stay where they were.
    let lines =
        |modifiers: &[Modifier]| -> Vec<String> { modifiers.iter().flat_map(texts).collect() };
    assert_eq!(lines(&read), lines(item.modifiers()));
}

#[test]
fn a_split_line_after_the_other_line_of_its_hybrid_keeps_its_place() {
    let modifiers = trade_lines(&["+195 to Accuracy Rating", "258% increased Physical Damage"]);

    let read = read_again("Warmonger Bow", 82, &modifiers, (2, 3));

    // The ranges of each part on the summed line, the second.
    let split = split_of(&read);
    assert_eq!(
        texts(split),
        ["+195 to Accuracy Rating", "258% increased Physical Damage"]
    );
    assert_eq!(
        split.doubt,
        Some(TierDoubt::Split {
            line: 1,
            whole: part(Affix::Prefix, &[1], &[(75.0, 79.0)]),
            added: part(Affix::Prefix, &[1], &[(170.0, 179.0)]),
        })
    );
}

#[test]
fn a_part_whose_two_tiers_give_its_line_other_ranges_isnt_read() {
    let rarity = trade_lines(&["18% increased Rarity of Items found"]);
    let tier = |number, ranges: Vec<Range>| Tier {
        number,
        level: 1,
        ranges,
    };
    let tiers = |second: Vec<Range>| ModifierTiers {
        tiers: vec![tier(1, vec![range(16.0, 19.0)]), tier(2, second)],
        found: 1,
        neighbour: Some(0),
        counts: vec![1],
        tier_unwritten: false,
    };
    let read =
        |tiers: &ModifierTiers| part_tier(Language::English, &rarity, Affix::Prefix, tiers, 0);

    assert_eq!(
        read(&tiers(vec![range(12.0, 15.0)])),
        Some(part(Affix::Prefix, &[1, 2], &[(12.0, 19.0)]))
    );
    // A tier without the range of the value: the ranges of the line can't be told apart.
    assert_eq!(read(&tiers(Vec::new())), None);

    // Each tier without the range of another value (across zero, which needs its sign): as
    // many ranges, but not of the same values.
    let damage = trade_lines(&["Adds 3 to 9 Physical Damage"]);
    let tiers = |first: f64, second: f64| ModifierTiers {
        tiers: vec![
            tier(1, vec![range(first, 2.0), range(5.0, 10.0)]),
            tier(2, vec![range(1.0, 4.0), range(second, 12.0)]),
        ],
        found: 1,
        neighbour: Some(0),
        counts: vec![2],
        tier_unwritten: false,
    };
    let read_damage =
        |tiers: &ModifierTiers| part_tier(Language::English, &damage, Affix::Prefix, tiers, 0);
    assert_eq!(
        read_damage(&tiers(1.0, 6.0)),
        Some(part(Affix::Prefix, &[1, 2], &[(1.0, 4.0), (5.0, 12.0)]))
    );
    assert_eq!(read_damage(&tiers(-1.0, -2.0)), None);
}

#[test]
fn a_part_of_a_split_line_in_two_neighbouring_tiers_is_read_over_the_range_of_both() {
    let modifiers = trade_lines(&["248% increased Physical Damage", "+195 to Accuracy Rating"]);

    let read = read_again("Warmonger Bow", 82, &modifiers, (2, 3));

    // The pure T1 (170–179) or T2 (155–169), the best first, with the hybrid T1.
    assert_eq!(
        split_of(&read).doubt,
        Some(TierDoubt::Split {
            line: 0,
            whole: part(Affix::Prefix, &[1], &[(75.0, 79.0)]),
            added: part(Affix::Prefix, &[1, 2], &[(155.0, 179.0)]),
        })
    );

    // The summed part in two tiers: at level 35, 18% of rarity is the suffix T3 (6–10) with
    // the prefix T2 (12–15) or T3 (8–11).
    let rarity = trade_lines(&["18% increased Rarity of Items found"]);
    let read = read_again("Ancestral Tiara", 35, &rarity, (3, 3));
    assert_eq!(
        split_of(&read).doubt,
        Some(TierDoubt::Split {
            line: 0,
            whole: part(Affix::Prefix, &[2, 3], &[(8.0, 15.0)]),
            added: part(Affix::Suffix, &[3], &[(6.0, 10.0)]),
        })
    );
}

#[test]
fn a_hybrid_prefix_added_up_with_a_pure_suffix_is_explicit_with_its_other_line() {
    let modifiers = trade_lines(&["35% increased Armour and Evasion", "+310 to Stun Threshold"]);

    // The other prefixes and suffixes full: with room, a reading of three modifiers fits too.
    let read = read_again("Avian Targe", 80, &modifiers, (1, 1));

    // Above the highest pure roll (304), 310 is the hybrid T2 (64–94) with the 35% and the
    // pure suffix T2 (207–253): explicit, the 35% with the range of the hybrid.
    let split = split_of(&read);
    assert_eq!(split.kind, ModifierKind::Explicit(None));
    assert_eq!(written_ranges(split), [Some(range(33.0, 38.0)), None]);
    assert_eq!(
        split.doubt,
        Some(TierDoubt::Split {
            line: 1,
            whole: part(Affix::Prefix, &[2], &[(64.0, 94.0)]),
            added: part(Affix::Suffix, &[2], &[(207.0, 253.0)]),
        })
    );
}

#[test]
fn a_line_that_adds_up_a_hybrid_of_three_lines_gives_both_other_lines_their_ranges() {
    // The Armour and the Evasion in no tier of their hybrid of two lines, the other
    // prefixes full.
    let modifiers = trade_lines(&[
        "+50 to Armour",
        "+41 to Evasion Rating",
        "115% increased Armour and Evasion",
    ]);

    let read = read_again("Ancestral Mail", 80, &modifiers, (2, 3));

    // Above the highest pure roll (110), 115% is the pure T4 (68–79) and the hybrid T1
    // (39–42) with the Armour (42–52) and the Evasion (40–50).
    let split = split_of(&read);
    assert_eq!(split.kind, ModifierKind::Explicit(Some(Affix::Prefix)));
    assert_eq!(
        written_ranges(split),
        [Some(range(42.0, 52.0)), Some(range(40.0, 50.0)), None]
    );
    assert_eq!(
        split.doubt,
        Some(TierDoubt::Split {
            line: 2,
            whole: part(Affix::Prefix, &[1], &[(39.0, 42.0)]),
            added: part(Affix::Prefix, &[4], &[(68.0, 79.0)]),
        })
    );
}

#[test]
fn a_line_that_adds_up_a_prefix_and_a_suffix_is_explicit_without_affix() {
    let mut rarity = trade_lines(&["37% increased Rarity of Items found"]);
    rarity[0].origin = Some(ModifierOrigin::Desecrated);

    let read = read_again("Ancestral Tiara", 82, &rarity, (3, 3));

    // The prefix T1 (16–19) first, the suffix T1 (15–18); its origin kept.
    let split = split_of(&read);
    assert_eq!(split.kind, ModifierKind::Explicit(None));
    assert_eq!(split.origin, Some(ModifierOrigin::Desecrated));
    assert_eq!(
        split.doubt,
        Some(TierDoubt::Split {
            line: 0,
            whole: part(Affix::Prefix, &[1], &[(16.0, 19.0)]),
            added: part(Affix::Suffix, &[1], &[(15.0, 18.0)]),
        })
    );
}

#[test]
fn a_desecrated_line_of_a_lord_and_of_an_ordinary_suffix_keeps_the_tier_of_each() {
    // `+1` desecrated is the prefix of Amanamu, the only one, or the T2 of the suffix of a
    // cannon, which a desecration reveals too.
    let mut grenades = trade_lines(&["Grenade Skills have +1 Cooldown Use"]);
    let ordinary = read_again("Trarthan Cannon", 82, &grenades, (3, 3));
    grenades[0].origin = Some(ModifierOrigin::Desecrated);

    let read = read_again("Trarthan Cannon", 82, &grenades, (3, 3));

    let tier = |affix, tier, hides_number| AffixTier {
        affix,
        tier,
        hides_number,
        ranges: vec![range(1.0, 1.0)],
    };
    assert_eq!(
        read.first().and_then(|modifier| modifier.doubt.clone()),
        Some(TierDoubt::EitherAffix {
            prefix: tier(Affix::Prefix, 1, true),
            suffix: tier(Affix::Suffix, 2, false),
        })
    );
    // Not marked, the line is no lord's: the T2 of the suffix.
    let kind_and_tier = ordinary
        .first()
        .map(|modifier| (modifier.kind, modifier.tier, modifier.doubt.is_some()));
    assert_eq!(
        kind_and_tier,
        Some((ModifierKind::Explicit(Some(Affix::Suffix)), Some(2), false))
    );
}

#[test]
fn the_lines_of_ordinary_modifiers_arent_read_as_a_lords() {
    // A hybrid of the Abyss (`5–10` and `4–8`) or of Kurgal (`110–154` and `15% reduced
    // Attack Speed`) would add up with them, leaving them in doubt.
    let read = |base_type: &str, lines: &[&str], room| {
        read_again(base_type, 82, &trade_lines(lines), room)
            .iter()
            .map(|modifier| (modifier.kind, modifier.tier))
            .collect::<Vec<_>>()
    };
    let prefix = ModifierKind::Explicit(Some(Affix::Prefix));
    let suffix = ModifierKind::Explicit(Some(Affix::Suffix));

    assert_eq!(
        read(
            "Ruby",
            &["6% increased Attack Damage", "17% increased Armour"],
            (2, 2)
        ),
        [(prefix, Some(1)), (prefix, Some(1))]
    );
    let maul = read(
        "Temple Maul",
        &[
            "130% increased Physical Damage",
            "10% increased Attack Speed",
        ],
        (3, 3),
    );
    assert!(maul.iter().all(|(_, tier)| tier.is_some()), "{maul:?}");
    assert_eq!(maul.first().map(|(kind, _)| *kind), Some(prefix));
    assert_eq!(maul.get(1).map(|(kind, _)| *kind), Some(suffix));
}

/// The kind, the tier and the ranges of each explicit modifier of `modifiers`.
fn explicit_tiers(modifiers: &[Modifier]) -> Vec<(ModifierKind, Option<u32>, Vec<Option<Range>>)> {
    modifiers
        .iter()
        .filter(|modifier| matches!(modifier.kind, ModifierKind::Explicit(_)))
        .map(|modifier| (modifier.kind, modifier.tier, written_ranges(modifier)))
        .collect()
}

#[test]
fn a_minion_modifier_of_the_genesis_tree_is_read_as_the_advanced_copy_writes_it() {
    // The owner's Breach Ring as the in-game trade writes it: `Reinforcing`, which the advanced
    // copy writes without tier (`Minions have 21(19-22)% increased maximum Life`), the third of
    // the six tiers of the tree.
    let item = parse(include_str!(
        "../../item/test-items/en-owner-rare-ring-breach.txt"
    ));
    let (modifiers, _) = as_in_game_trade(&item, "Breach Ring");

    let read = read_again("Breach Ring", 77, &modifiers, (3, 3));

    let prefix = ModifierKind::Explicit(Some(Affix::Prefix));
    let suffix = ModifierKind::Explicit(Some(Affix::Suffix));
    let ranged = |min, max| vec![Some(range(min, max))];
    // A prefix, which fills the room of the prefixes, without tier, over the range of its tier.
    assert_eq!(
        explicit_tiers(&read),
        [
            (prefix, None, ranged(19.0, 22.0)),
            (prefix, Some(7), ranged(65.0, 79.0)),
            (prefix, Some(4), ranged(108.0, 141.0)),
            (suffix, Some(5), ranged(17.0, 20.0)),
            (suffix, Some(2), ranged(29.0, 40.0)),
            (suffix, Some(3), ranged(16.0, 19.0)),
        ]
    );
    // Its bar and its field over that range, without tier to tell, as in the advanced copy;
    // the implicit first.
    let rolls =
        crate::tier_rolls::TierRolls::new(Language::English, &read, Some("Breach Ring"), Some(77));
    assert!(rolls.single(1));
    assert!(!rolls.single(2));
    let key = crate::filters::StatKey {
        modifier: 1,
        line: 0,
    };
    let over_its_tier = crate::tier_rolls::LineRolls {
        ranges: vec![Some(range(19.0, 22.0))],
        tiers: None,
    };
    assert_eq!(rolls.get(key), Some(&over_its_tier));
}

#[test]
fn a_minion_modifier_of_the_genesis_tree_merges_with_no_ordinary_one() {
    let levels = ["+2 to Level of all Minion Skills"];
    let suffix = modifier(Some(Affix::Suffix), &levels);
    let numbers = |base_type: &str| {
        tiers(base_type, &suffix).map(|tiers| {
            let number = tiers.tier().map(|tier| tier.number);
            (tiers.tiers.len(), number, tiers.hides_number())
        })
    };
    let read =
        |base_type: &str| explicit_tiers(&read_again(base_type, 80, &trade_lines(&levels), (3, 3)));
    let kind = ModifierKind::Explicit(Some(Affix::Suffix));
    let two = vec![Some(range(2.0, 2.0))];

    // `of the Despot` of an amulet, its T2 of 3 (the owner's amulet): with the two tiers of the
    // tree, it would be a T3 of 5.
    assert_eq!(numbers("Gold Amulet"), Some((3, Some(2), false)));
    assert_eq!(read("Gold Amulet"), [(kind, Some(2), two.clone())]);
    // On a belt, the tree's only, whose tier the game doesn't write.
    assert_eq!(numbers("Heavy Belt"), Some((2, Some(1), true)));
    assert!(tiers("Heavy Belt", &suffix).is_some_and(|tiers| !tiers.single()));
    assert_eq!(read("Heavy Belt"), [(kind, None, two)]);
    // The tree rules it out of the rings.
    assert_eq!(numbers("Gold Ring"), None);
}

#[test]
fn a_desecrated_modifier_of_the_genesis_tree_is_one_of_its_own() {
    let line = |text: &str| desecrated(Affix::Suffix, &[text]);
    let only = |base_type: &str, modifier: &Modifier| {
        tiers(base_type, modifier).map(|tiers| {
            let ranges = tiers.tier().map(|tier| tier.ranges.clone());
            (tiers.single(), ranges)
        })
    };
    let exposure = |value: u32| line(&format!("{value}% increased Exposure Effect"));
    let single = |min, max| Some((true, Some(vec![range(min, max)])));

    // `of Drenching` of the tree (25–35) on the rings only, `of Kurgal` (10–15) on the rings and
    // the amulets: two modifiers.
    assert_eq!(only("Gold Ring", &exposure(30)), single(25.0, 35.0));
    assert_eq!(only("Gold Ring", &exposure(12)), single(10.0, 15.0));
    assert_eq!(only("Gold Amulet", &exposure(30)), None);
    assert_eq!(only("Gold Amulet", &exposure(12)), single(10.0, 15.0));
    // From the in-game trade: the tree's without tier, as the game writes it, Kurgal's its T1.
    let read = |value: u32| {
        let mut lines = trade_lines(&[&format!("{value}% increased Exposure Effect")]);
        lines[0].origin = Some(ModifierOrigin::Desecrated);
        explicit_tiers(&read_again("Gold Ring", 80, &lines, (3, 3)))
    };
    let kind = ModifierKind::Explicit(Some(Affix::Suffix));
    assert_eq!(read(30), [(kind, None, vec![Some(range(25.0, 35.0))])]);
    assert_eq!(read(12), [(kind, Some(1), vec![Some(range(10.0, 15.0))])]);
    // `of Binding` on the belts, as the trade site shows it on every kind of belt.
    let duration = line("40% increased Minion Duration");
    assert_eq!(only("Heavy Belt", &duration), single(35.0, 49.0));
    assert_eq!(only("Gold Ring", &duration), None);
}

#[test]
fn a_value_in_several_tiers_takes_the_worst_without_the_ranges_of_the_copy() {
    let tables = read(concat!(
        r##"{"type": "Life", "affix": "prefix", "lines": [{"template": "# to maximum Life"}], "##,
        r#""spawn": [["helmet", 1]], "tiers": [{"level": 1, "ranges": [[[1, 10]]]}, {"level": 50, "ranges": [[[5, 15]]]}]}"#
    ));
    let copied = Copied {
        lines: vec![CopiedLine {
            template: "# to maximum Life",
            values: vec![7.0],
            written: None,
        }],
        ranges: None,
        tier: None,
        desecrated: false,
    };
    let number = |tie| {
        find(
            &tables,
            TableAffix::Prefix,
            &equipment(&["helmet"]),
            &copied,
            tie,
        )
        .and_then(|tiers| Some(tiers.tier()?.number))
    };

    assert_eq!(number(Tie::Best), Some(1));
    assert_eq!(number(Tie::Worst { item_level: None }), Some(2));
}

#[test]
fn the_tiers_of_the_hybrid_of_a_split_are_those_of_its_part() {
    // T2 at level 1, T1 at level 50, the Armour of both touching at 20.
    let tables = read(concat!(
        r##"{"type": "ArmourAndStun", "affix": "prefix", "lines": [{"template": "#% increased Armour"}, {"template": "# to Stun Threshold"}], "##,
        r#""spawn": [["shield", 1]], "tiers": [{"level": 1, "ranges": [[[10, 20]], [[20, 29]]]}, {"level": 50, "ranges": [[[20, 29]], [[30, 39]]]}]}"#
    ));
    let copied = |armour: f64| Copied {
        lines: vec![
            CopiedLine {
                template: "#% increased Armour",
                values: vec![armour],
                written: None,
            },
            CopiedLine {
                template: "# to Stun Threshold",
                values: vec![100.0],
                written: None,
            },
        ],
        ranges: None,
        tier: None,
        desecrated: false,
    };
    let base = equipment(&["shield"]);
    // The tier and its neighbour found, by their numbers.
    let tiers = |armour, whole: PartTier| {
        hybrid_tiers(&tables, &base, copied(armour), (1, &whole), None).map(|tiers| {
            let number = |index: usize| tiers.tiers.get(index).map(|tier| tier.number);
            (number(tiers.found), tiers.neighbour.and_then(number))
        })
    };

    assert_eq!(
        tiers(25.0, part(Affix::Prefix, &[1], &[(30.0, 39.0)])),
        Some((Some(1), None))
    );
    // In two neighbouring tiers, the Armour in both: the worse found, over the range of both.
    assert_eq!(
        tiers(20.0, part(Affix::Prefix, &[1, 2], &[(20.0, 39.0)])),
        Some((Some(2), Some(1)))
    );
    // Ranges the tiers of that table don't give the summed line: another table.
    assert_eq!(
        tiers(25.0, part(Affix::Prefix, &[1], &[(31.0, 39.0)])),
        None
    );
    assert_eq!(
        tiers(25.0, part(Affix::Prefix, &[3], &[(30.0, 39.0)])),
        None
    );
}

#[test]
fn the_hybrid_of_a_split_is_the_first_table_whose_tiers_of_its_part_the_item_level_allows() {
    // Two hybrids of the same lines, both holding the Armour, the first with its own range of
    // Armour: it rolls its T1 from level 80, or gives the Stun Threshold another range.
    let table = |kind: &str, level: u32, armour: u32, stun: u32| {
        format!(
            r##"{{"type": "{kind}", "affix": "prefix", "lines": [{{"template": "#% increased Armour"}}, {{"template": "# to Stun Threshold"}}], "spawn": [["shield", 1]], "tiers": [{{"level": {level}, "ranges": [[[20, {armour}]], [[{stun}, 39]]]}}]}}"##
        )
    };
    let copied = || Copied {
        lines: vec![
            CopiedLine {
                template: "#% increased Armour",
                values: vec![25.0],
                written: None,
            },
            CopiedLine {
                template: "# to Stun Threshold",
                values: vec![100.0],
                written: None,
            },
        ],
        ranges: None,
        tier: None,
        desecrated: false,
    };
    let whole = part(Affix::Prefix, &[1], &[(30.0, 39.0)]);
    // The range of the Armour of the tier found, which tells the table.
    let armour = |tables: &[String], item_level| {
        let tables = read(&tables.join("\n"));
        hybrid_tiers(
            &tables,
            &equipment(&["shield"]),
            copied(),
            (1, &whole),
            item_level,
        )
        .and_then(|tiers| tiers.tier()?.ranges.first().copied())
    };
    let high = [table("First", 80, 35, 30), table("Second", 1, 30, 30)];
    let other = [table("First", 1, 35, 31), table("Second", 1, 30, 30)];

    // The tier of the first above the item level, as `splits` reads it: the second.
    assert_eq!(armour(&high, Some(50)), Some(range(20.0, 30.0)));
    // Without the item level, any tier.
    assert_eq!(armour(&high, None), Some(range(20.0, 35.0)));
    // The first gives the summed line other ranges: the second.
    assert_eq!(armour(&other, Some(82)), Some(range(20.0, 30.0)));
    // The first holds the Armour give or take the rounding only, the second within its
    // range: the second, as `splits` keeps the ways that fit best.
    let rounded = [
        table("First", 1, 35, 30).replace("[[20, 35]]", "[[20, 24.96]]"),
        table("Second", 1, 30, 30),
    ];
    assert_eq!(armour(&rounded, Some(82)), Some(range(20.0, 30.0)));
}

#[test]
fn two_hybrids_of_the_same_lines_that_split_a_line_are_two_ways() {
    // Two hybrids of the same lines and Armour, other Stun Thresholds, and a pure Stun
    // Threshold: `100` adds up the pure `50–70` with `30–39` or with `40–49`.
    let hybrid = |kind: &str, stun: u32| {
        format!(
            r##"{{"type": "{kind}", "affix": "prefix", "lines": [{{"template": "#% increased Armour"}}, {{"template": "# to Stun Threshold"}}], "spawn": [["shield", 1]], "tiers": [{{"level": 1, "ranges": [[[20, 30]], [[{stun}, {}]]]}}]}}"##,
            stun + 9
        )
    };
    let pure = r##"{"type": "Stun", "affix": "suffix", "lines": [{"template": "# to Stun Threshold"}], "spawn": [["shield", 1]], "tiers": [{"level": 1, "ranges": [[[50, 70]]]}]}"##;
    let tables = read(&[hybrid("First", 30), hybrid("Second", 40), pure.to_owned()].join("\n"));
    let copied = Copied {
        lines: vec![
            CopiedLine {
                template: "#% increased Armour",
                values: vec![25.0],
                written: None,
            },
            CopiedLine {
                template: "# to Stun Threshold",
                values: vec![100.0],
                written: None,
            },
        ],
        ranges: None,
        tier: None,
        desecrated: false,
    };

    let ways = splits(&tables, &equipment(&["shield"]), &copied, (1, 1), Some(82));

    // Two ways as good, of two tables: the line is left without tier by the search, as two
    // candidates split it (`the_lines_split_in_two_ways_as_good_keep_their_room`).
    let pairs: Vec<_> = ways.iter().map(|way| way.tables).collect();
    assert_eq!(pairs, [(0, 2), (1, 2)]);
}

#[test]
fn a_base_the_data_doesnt_know_has_no_tiers_found() {
    let item = parse(TIARA_EN);
    let limits = AffixLimits {
        prefixes: 3,
        suffixes: 3,
    };

    let found = trade_tiers(
        Language::English,
        "Unknown Tiara",
        Some(82),
        item.modifiers(),
        limits,
        false,
    );

    assert_eq!(found, []);
}

#[test]
fn many_lines_that_pair_as_hybrids_are_read_within_a_bound() {
    let item = parse(TIARA_EN);
    // Each line makes a hybrid with the one before and the one after: more readings than
    // the bound.
    let modifiers: Vec<_> = (0..20)
        .map(|index| {
            let line = if index % 2 == 0 {
                "41% increased Energy Shield"
            } else {
                "+34 to maximum Mana"
            };
            modifier(None, &[line])
        })
        .collect();

    let found = found_tiers(&item, &modifiers, (10, 0));

    // The readings that fit, ten hybrids, are past the bound: the lines are read alone,
    // the pure energy shield T6 and mana T8, every affix kept.
    let alone = [(1, vec![(Affix::Prefix, 6)]), (1, vec![(Affix::Prefix, 8)])];
    let expected: Vec<_> = alone.iter().cycle().take(20).cloned().collect();
    assert_eq!(summary(&modifiers, &found), expected);
}

/// The lines of two pure prefixes of a helmet that also fit the hybrid of both, T3
/// (`27–32 · 26–32`).
fn energy_shield_and_life() -> Vec<Modifier> {
    vec![
        modifier(None, &["30% increased Energy Shield"]),
        modifier(None, &["+30 to maximum Life"]),
    ]
}

#[test]
fn lines_that_fit_a_hybrid_are_read_alone_while_the_item_has_room() {
    use Affix::Prefix;
    let item = parse(TIARA_EN);
    let modifiers = energy_shield_and_life();

    let found = found_tiers(&item, &modifiers, (3, 3));
    assert_eq!(
        summary(&modifiers, &found),
        [(1, vec![(Prefix, 6)]), (1, vec![(Prefix, 8)])]
    );

    // With room for one prefix only, the hybrid.
    let found = found_tiers(&item, &modifiers, (1, 3));
    assert_eq!(summary(&modifiers, &found), [(2, vec![(Prefix, 3)])]);
}

#[test]
fn a_line_only_a_hybrid_holds_is_read_in_it() {
    let item = parse(BOW_EN);
    // The pure `#% increased Physical Damage` starts at 40%, the hybrid with the accuracy
    // at 15%: read alone, the physical damage would have no tier.
    let modifiers = [
        modifier(None, &["30% increased Physical Damage"]),
        modifier(None, &["+60 to Accuracy Rating"]),
    ];

    let found = found_tiers(&item, &modifiers, (3, 3));

    assert_eq!(summary(&modifiers, &found), [(2, vec![(Affix::Prefix, 6)])]);
}

#[test]
fn two_fractured_lines_are_one_hybrid_even_when_the_item_has_room() {
    let item = parse(TIARA_EN);

    // Room for four prefixes: the two fractured lines read alone would fit it, but an item
    // has one fractured modifier only.
    let found = found_tiers(&item, item.modifiers(), (4, 3));

    let first = found.first().map(|found| found.modifiers.len());
    assert_eq!(first, Some(2));
}

#[test]
fn a_tier_above_the_item_level_isnt_found() {
    let modifiers = energy_shield_and_life();
    let limits = AffixLimits {
        prefixes: 1,
        suffixes: 3,
    };
    let read = |item_level| {
        let found = trade_tiers(
            Language::English,
            "Ancestral Tiara",
            Some(item_level),
            &modifiers,
            limits,
            false,
        );
        summary(&modifiers, &found)
    };

    // The hybrid T3 needs the level 46: on an item of level 40, the lines are two
    // prefixes, more than its room, every affix kept.
    assert_eq!(read(46), [(2, vec![(Affix::Prefix, 3)])]);
    assert_eq!(
        read(40),
        [(1, vec![(Affix::Prefix, 6)]), (1, vec![(Affix::Prefix, 8)])]
    );

    // The prefix of rarity `16–19` needs the level 47, the suffix `15–18` the level 40.
    let rarity = [modifier(None, &["18% increased Rarity of Items found"])];
    let affixes = |item_level| {
        let found = trade_tiers(
            Language::English,
            "Ancestral Tiara",
            Some(item_level),
            &rarity,
            limits,
            false,
        );
        summary(&rarity, &found)
    };
    assert_eq!(
        affixes(47),
        [(1, vec![(Affix::Prefix, 1), (Affix::Suffix, 1)])]
    );
    assert_eq!(affixes(45), [(1, vec![(Affix::Suffix, 1)])]);
}

#[test]
fn a_line_no_tier_holds_is_split_only_when_one_way_fits() {
    use Affix::{Prefix, Suffix};
    let item = parse(TIARA_EN);
    let read = |line: &str| {
        let modifiers = [modifier(None, &[line])];
        let found = found_tiers(&item, &modifiers, (3, 3));
        (summary(&modifiers, &found), added(&modifiers, &found))
    };

    // The rarity rolls as a suffix (6–10, 11–14, 15–18) and as a prefix (8–11, 12–15,
    // 16–19): 37% is the suffix T1 at 18 and the prefix T1 at 19 only, the prefix first,
    // though the suffix comes first in the tables.
    let rarity = |value| format!("{value}% increased Rarity of Items found");
    assert_eq!(
        read(&rarity(37)),
        (vec![(1, vec![(Prefix, 1)])], vec![(rarity(37), Suffix, 1)])
    );
    // 30% is T1 and T2, or T2 and T1, both parts in doubt: without tier; so is 33%, also
    // both T1.
    assert_eq!(read(&rarity(30)), (vec![], vec![]));
    assert_eq!(read(&rarity(33)), (vec![], vec![]));
    // 18% fits one modifier: not split, though two would hold it too.
    assert_eq!(
        read(&rarity(18)),
        (vec![(1, vec![(Prefix, 1), (Suffix, 1)])], vec![])
    );
}

#[test]
fn the_lines_split_in_two_ways_as_good_keep_their_room() {
    let item = parse(TIARA_EN);
    let modifiers = [
        modifier(None, &["33% increased Rarity of Items found"]),
        modifier(None, &["30% increased Energy Shield"]),
        modifier(None, &["+30 to maximum Life"]),
        modifier(None, &["+34 to maximum Mana"]),
    ];

    let found = found_tiers(&item, &modifiers, (3, 3));

    // The rarity is a prefix and a suffix in three ways as good, without tier: with its
    // prefix, the other prefixes are full, the energy shield and the life one hybrid T3
    // (27–32 · 26–32), not two pure prefixes of T6 and T8.
    let hybrid = vec![
        "30% increased Energy Shield".to_owned(),
        "+30 to maximum Life".to_owned(),
    ];
    assert_eq!(
        lines_read(&modifiers, &found),
        [
            (hybrid, vec![(Affix::Prefix, 3)]),
            (
                vec!["+34 to maximum Mana".to_owned()],
                vec![(Affix::Prefix, 8)]
            ),
        ]
    );
}

#[test]
fn a_fractured_line_isnt_split() {
    let item = parse(TIARA_EN);
    // Split, the line would be two fractured modifiers.
    let modifiers = [Modifier {
        origin: Some(ModifierOrigin::Fractured),
        ..modifier(None, &["37% increased Rarity of Items found"])
    }];

    let found = found_tiers(&item, &modifiers, (3, 3));

    assert_eq!(found, []);
}

/// The lines of greaves with three resistances, whose `armour` may add up the pure
/// modifier and the hybrid with the `+30 to Stun Threshold` after it.
fn greaves(armour: u32) -> Vec<Modifier> {
    let armour = format!("{armour}% increased Armour");
    [
        armour.as_str(),
        "+30 to Stun Threshold",
        "+30% to Fire Resistance",
        "+30% to Cold Resistance",
        "+30% to Lightning Resistance",
    ]
    .iter()
    .map(|line| modifier(None, &[line]))
    .collect()
}

/// The lines of `modifiers` read from their values on greaves of `item_level` with 3
/// prefixes and 3 suffixes, and the lines split.
fn greaves_read(modifiers: &[Modifier], item_level: u32) -> (LinesRead, Added) {
    let limits = AffixLimits {
        prefixes: 3,
        suffixes: 3,
    };
    let found = trade_tiers(
        Language::English,
        "Bronze Greaves",
        Some(item_level),
        modifiers,
        limits,
        false,
    );
    (lines_read(modifiers, &found), added(modifiers, &found))
}

#[test]
fn a_line_a_modifier_holds_alone_is_split_when_the_room_needs_it() {
    use Affix::{Prefix, Suffix};
    let line = |text: &str| vec![text.to_owned()];
    let resistances = [
        (line("+30% to Fire Resistance"), vec![(Suffix, 4)]),
        (line("+30% to Cold Resistance"), vec![(Suffix, 4)]),
        (line("+30% to Lightning Resistance"), vec![(Suffix, 4)]),
    ];

    // Read alone, the armour a pure prefix, the stun threshold would be a fourth suffix:
    // the hybrid T4 (21–26 · 25–40) and the pure T6.
    let modifiers = greaves(60);
    let hybrid = vec![
        "60% increased Armour".to_owned(),
        "+30 to Stun Threshold".to_owned(),
    ];
    let mut expected = vec![(hybrid, vec![(Prefix, 4)])];
    expected.extend(resistances.clone());
    assert_eq!(
        greaves_read(&modifiers, 82),
        (
            expected,
            vec![("60% increased Armour".to_owned(), Prefix, 6)]
        )
    );

    // 50% is the same hybrid with the pure T7 or T6: one split, the pure modifier in both.
    let modifiers = greaves(50);
    let hybrid = vec![
        "50% increased Armour".to_owned(),
        "+30 to Stun Threshold".to_owned(),
    ];
    let mut expected = vec![(hybrid, vec![(Prefix, 4)])];
    expected.extend(resistances);
    assert_eq!(
        greaves_read(&modifiers, 82),
        (
            expected,
            vec![("50% increased Armour".to_owned(), Prefix, 7)]
        )
    );
}

#[test]
fn a_part_of_a_split_line_in_two_neighbouring_tiers_keeps_both() {
    let item = parse(BOW_EN);
    // The pure part, 169–173, is a T2 at 169 or a T1, with the same hybrid T1 at 75–79.
    let modifiers = [
        modifier(None, &["248% increased Physical Damage"]),
        modifier(None, &["+195 to Accuracy Rating"]),
    ];

    // With two prefixes, the other readings are the same as the best one but for the tier
    // of the pure part.
    let found = found_tiers(&item, &modifiers, (2, 3));
    assert_eq!(summary(&modifiers, &found), [(2, vec![(Affix::Prefix, 1)])]);
    assert_eq!(
        added(&modifiers, &found),
        [(
            "248% increased Physical Damage".to_owned(),
            Affix::Prefix,
            2
        )]
    );
    assert_eq!(neighbours(&found), [(2, 1)]);

    // With a prefix open, the accuracy could add up a pure modifier too: without tier, the
    // accuracy too, which a pure prefix of tier 5 would hold alone.
    assert_eq!(found_tiers(&item, &modifiers, (3, 3)), []);
}

#[test]
fn the_summed_part_of_a_split_line_in_two_neighbouring_tiers_keeps_both() {
    let rarity = [modifier(None, &["18% increased Rarity of Items found"])];
    let limits = AffixLimits {
        prefixes: 3,
        suffixes: 3,
    };
    let read = |item_level| {
        let found = trade_tiers(
            Language::English,
            "Ancestral Tiara",
            Some(item_level),
            &rarity,
            limits,
            false,
        );
        let numbers = |tiers: &ModifierTiers| -> Vec<u32> {
            tiers.possible().map(|tier| tier.number).collect()
        };
        found
            .iter()
            .map(|found| {
                let affixes: Vec<_> = found
                    .affixes
                    .iter()
                    .map(|(affix, tiers)| (*affix, numbers(tiers)))
                    .collect();
                let added = found
                    .added
                    .as_ref()
                    .map(|added| (added.affix, numbers(&added.tiers)));
                (affixes, added)
            })
            .collect::<Vec<_>>()
    };

    // At level 35, the rarity rolls as a prefix 8–11 (T3) or 12–15 (T2), and as a suffix
    // 6–10 (T3) or 11–14 (T2): 18% fits neither alone, and is the suffix T3 with the prefix
    // T2 or T3, the prefix the summed part.
    assert_eq!(
        read(35),
        [(
            vec![(Affix::Prefix, vec![2, 3])],
            Some((Affix::Suffix, vec![3]))
        )]
    );
    // At level 40, the suffix T1 (15–18) holds it alone: not split.
    assert_eq!(read(40), [(vec![(Affix::Suffix, vec![1])], None)]);
}

#[test]
fn a_line_held_alone_is_split_when_the_other_line_of_the_hybrid_has_no_tier_alone() {
    let modifiers = [
        modifier(None, &["9% increased Armour"]),
        modifier(None, &["+100 to maximum Life"]),
    ];
    let limits = AffixLimits {
        prefixes: 3,
        suffixes: 3,
    };

    let found = trade_tiers(
        Language::English,
        "Soldier Cuirass",
        Some(82),
        &modifiers,
        limits,
        false,
    );

    // 9% is under every pure tier: the hybrid T6 (6–13 · 7–10), and the life the pure T7
    // (85–99), not the pure T6 (100–119) the life alone would be.
    assert_eq!(summary(&modifiers, &found), [(2, vec![(Affix::Prefix, 6)])]);
    assert_eq!(
        added(&modifiers, &found),
        [("+100 to maximum Life".to_owned(), Affix::Prefix, 7)]
    );
}

#[test]
fn the_lines_in_doubt_keep_their_room() {
    use Affix::{Prefix, Suffix};
    let item = parse(TIARA_EN);
    let lines = |mana: bool| {
        let mut lines = vec!["10% increased Energy Shield", "+127 to maximum Life"];
        lines.extend(mana.then_some("+34 to maximum Mana"));
        lines.push("18% increased Rarity of Items found");
        lines
            .iter()
            .map(|line| modifier(None, &[*line]))
            .collect::<Vec<_>>()
    };

    // The energy shield, under every pure tier, is the hybrid T6 (6–13 · 7–10) with the
    // life, whose line adds up a pure T3 or T2: two prefixes. With the mana, the prefixes are
    // full, the rarity a suffix; but the energy shield may be the hybrid with the mana, whose
    // line adds up a pure mana, the life a pure modifier, three prefixes too: the three lines
    // without tier.
    let modifiers = lines(true);
    let found = found_tiers(&item, &modifiers, (3, 3));
    assert_eq!(summary(&modifiers, &found), [(1, vec![(Suffix, 1)])]);

    // Without the mana, a prefix is open: the rarity could be a prefix and a suffix added up
    // too, one more modifier than either alone, which leaves it as it is.
    let modifiers = lines(false);
    let found = found_tiers(&item, &modifiers, (3, 3));
    assert_eq!(
        summary(&modifiers, &found),
        [(2, vec![(Prefix, 6)]), (1, vec![(Prefix, 1), (Suffix, 1)])]
    );
    assert_eq!(
        added(&modifiers, &found),
        [("+127 to maximum Life".to_owned(), Prefix, 3)]
    );
    assert_eq!(neighbours(&found), [(3, 2)]);
}

#[test]
fn a_line_after_the_other_line_of_its_hybrid_is_split() {
    let item = parse(BOW_EN);
    let modifiers = [
        modifier(None, &["+195 to Accuracy Rating"]),
        modifier(None, &["258% increased Physical Damage"]),
    ];

    // With a prefix open, the accuracy could add up a pure modifier too: without tier.
    assert_eq!(found_tiers(&item, &modifiers, (3, 3)), []);
    let found = found_tiers(&item, &modifiers, (2, 3));

    assert_eq!(summary(&modifiers, &found), [(2, vec![(Affix::Prefix, 1)])]);
    assert_eq!(
        added(&modifiers, &found),
        [(
            "258% increased Physical Damage".to_owned(),
            Affix::Prefix,
            1
        )]
    );
}

#[test]
fn the_tiers_of_a_split_are_those_the_item_level_allows() {
    let modifiers = [
        modifier(None, &["258% increased Physical Damage"]),
        modifier(None, &["+195 to Accuracy Rating"]),
    ];
    let read = |item_level, prefixes| {
        let limits = AffixLimits {
            prefixes,
            suffixes: 3,
        };
        let found = trade_tiers(
            Language::English,
            "Warmonger Bow",
            Some(item_level),
            &modifiers,
            limits,
            false,
        );
        (summary(&modifiers, &found), added(&modifiers, &found))
    };

    // The pure T1 170–179 needs the level 82: below it, 258 is out of reach, the accuracy
    // a pure prefix.
    let split = (
        vec![(2, vec![(Affix::Prefix, 1)])],
        vec![(
            "258% increased Physical Damage".to_owned(),
            Affix::Prefix,
            1,
        )],
    );
    assert_eq!(read(82, 2), split);
    assert_eq!(read(81, 2), (vec![(1, vec![(Affix::Prefix, 5)])], vec![]));
    // With a prefix open, the accuracy could add up a pure modifier too: without tier.
    assert_eq!(read(82, 3), (vec![], vec![]));
}

#[test]
fn no_line_of_a_sanctified_item_is_split() {
    let item = parse(BOW_EN);
    let limits = AffixLimits {
        prefixes: 3,
        suffixes: 4,
    };

    let found = trade_tiers(
        item.language(),
        item.base_type(),
        item.item_level(),
        item.modifiers(),
        limits,
        true,
    );

    // Its values can leave their tiers: the `258%` without tier, the accuracy a pure prefix.
    assert_eq!(added(item.modifiers(), &found), []);
    let accuracy = (
        vec!["+195 to Accuracy Rating".to_owned()],
        vec![(Affix::Prefix, 5)],
    );
    assert!(lines_read(item.modifiers(), &found).contains(&accuracy));
}

#[test]
fn both_lines_of_a_hybrid_that_may_add_up_another_modifier_have_no_tier() {
    let item = parse(BOW_EN);
    let lines = |adds: bool| {
        let mut lines = Vec::new();
        lines.extend(adds.then_some("Adds 36 to 56 Physical Damage"));
        lines.extend(["217% increased Physical Damage", "+200 to Accuracy Rating"]);
        lines
            .iter()
            .map(|line| modifier(None, &[*line]))
            .collect::<Vec<_>>()
    };

    // The hybrid T1 (75–79 · 175–200) and a pure T3 (135–154), two prefixes; or a pure T1
    // (170–179), the hybrid T5 (45–54 · 98–123) and a pure accuracy T8 (61–84), three.
    let modifiers = lines(false);
    assert_eq!(found_tiers(&item, &modifiers, (3, 3)), []);

    // With a third prefix, the first reading is the only one.
    let modifiers = lines(true);
    let found = found_tiers(&item, &modifiers, (3, 3));
    assert_eq!(
        summary(&modifiers, &found),
        [(1, vec![(Affix::Prefix, 1)]), (2, vec![(Affix::Prefix, 1)])]
    );
    assert_eq!(
        added(&modifiers, &found),
        [(
            "217% increased Physical Damage".to_owned(),
            Affix::Prefix,
            3
        )]
    );
}

#[test]
fn an_affix_the_room_ruled_out_is_kept_when_a_reading_in_doubt_frees_it() {
    use Affix::{Prefix, Suffix};
    let modifiers: Vec<_> = [
        "16% increased Rarity of Items found",
        "41% increased Energy Shield",
        "+15 to maximum Life",
        "+20 to maximum Mana",
        "+16% to Chaos Resistance",
    ]
    .iter()
    .map(|line| modifier(None, &[*line]))
    .collect();
    let limits = AffixLimits {
        prefixes: 3,
        suffixes: 3,
    };

    let found = trade_tiers(
        Language::English,
        "Ancestral Tiara",
        Some(82),
        &modifiers,
        limits,
        false,
    );

    // The energy shield, the life and the mana are three pure prefixes, which leave the rarity
    // a suffix; but they are in doubt, another reading of them, with a line adding up two
    // modifiers, taking a prefix less: the rarity can be a prefix too.
    assert_eq!(
        summary(&modifiers, &found),
        [(1, vec![(Prefix, 1), (Suffix, 1)]), (1, vec![(Suffix, 3)])]
    );
}

#[test]
fn the_lines_of_two_hybrids_sharing_a_line_have_no_tier() {
    let item = parse(BOW_EN);
    let modifiers = [
        modifier(None, &["10% increased Light Radius"]),
        modifier(None, &["+235 to Accuracy Rating"]),
        modifier(None, &["78% increased Physical Damage"]),
    ];

    let found = found_tiers(&item, &modifiers, (3, 3));

    // The light radius with the accuracy T2 (10 · 21–40), the physical damage with the
    // accuracy T1 (75–79 · 175–200); not the light radius hybrid with a pure accuracy T5
    // (168–236) on its line, and a pure 78% T6 (65–84), as read before.
    assert_eq!(found, []);
}

#[test]
fn the_lines_of_the_owners_body_armour_adding_up_two_hybrids_have_no_tier() {
    use Affix::Suffix;
    let item = parse(include_str!(
        "../../item/test-items/fr-trade-rare-body-armour.txt"
    ));

    let found = found_tiers(&item, item.modifiers(), (3, 3));

    // `47% d'Augmentation de l'Armure et de l'Évasion` adds up the 23 of a hybrid with `+23 à
    // l'Armure` and `+19 au Score d'Évasion` (T4) and the 24 of a hybrid with the life (T4),
    // whose line `+93 de Vie maximale` adds up a pure T8 (the advanced copy of the same
    // item): three prefixes, as many as the hybrid of the armour and the evasion T7, a pure
    // 47% T6 and a pure life T7, read before. The suffixes are sure.
    let line = |text: &str| vec![text.to_owned()];
    assert_eq!(
        lines_read(item.modifiers(), &found),
        [
            (line("+26 de Force"), vec![(Suffix, 3)]),
            (line("+31% à la Résistance au froid"), vec![(Suffix, 3)]),
            (line("12.2 de Vie Régénérée par seconde"), vec![(Suffix, 6)]),
        ]
    );
}

#[test]
fn a_line_split_with_a_neighbour_its_hybrid_may_not_have_is_in_doubt() {
    let read = |lines: &[&str]| {
        let modifiers: Vec<_> = lines.iter().map(|line| modifier(None, &[*line])).collect();
        let limits = AffixLimits {
            prefixes: 3,
            suffixes: 3,
        };
        trade_tiers(
            Language::English,
            "Soldier Cuirass",
            Some(82),
            &modifiers,
            limits,
            false,
        )
    };

    // 119% is above every pure tier: the pure T2 (92–100) with the hybrid with the life T4
    // (21–26 · 20–25), the `+74` a pure T9 (57–77); or the hybrid with the `+74` T2 (33–38 ·
    // 58–75) with a pure T3, the life a pure modifier. The copy sorts its lines by stat: the
    // lines of a hybrid needn't follow each other, so that the neighbour of the summed line
    // isn't more likely.
    let armour = [
        "119% increased Armour",
        "+74 to Armour",
        "+23 to maximum Life",
    ];
    assert_eq!(read(&armour), []);
    let armour = [
        "+74 to Armour",
        "119% increased Armour",
        "+23 to maximum Life",
    ];
    assert_eq!(read(&armour), []);
}

#[test]
fn a_split_holds_its_value_give_or_take_the_rounding() {
    let tier = |ranges: &[(f64, f64)]| Tier {
        number: 1,
        level: 1,
        ranges: ranges.iter().map(|&(min, max)| range(min, max)).collect(),
    };
    // A hybrid of two values, the second one summed with a pure 1–2.
    let whole = tier(&[(10.0, 20.0), (4.41, 5.0)]);
    let added = tier(&[(1.0, 2.0)]);
    let fit = |summed| split_fit(&[15.0, summed], &whole, &added, &(1..2));

    assert_eq!(fit(7.0), Some(Fit::Within));
    assert_eq!(fit(5.41), Some(Fit::Within));
    assert_eq!(fit(7.03), Some(Fit::Rounded));
    assert_eq!(fit(5.3), None);
    assert_eq!(fit(7.1), None);
    // The value of the other line within its own range only.
    assert_eq!(split_fit(&[9.0, 6.0], &whole, &added, &(1..2)), None);
}

/// A candidate that splits the line `line` into a modifier of `affix` over the modifiers
/// `modifiers` and a pure one of `added`, without tiers.
fn split(modifiers: std::ops::Range<usize>, affix: Affix, added: Affix, line: usize) -> Candidate {
    let whole = candidate(modifiers, &[affix], false);
    let tiers = whole.affixes.first().map(|(_, tiers)| tiers.clone());
    Candidate {
        added: tiers.map(|tiers| AddedTiers {
            line,
            affix: added,
            tiers,
        }),
        ..whole
    }
}

#[test]
fn a_line_split_in_two_ways_as_good_is_ambiguous() {
    use Affix::{Prefix, Suffix};
    let room = |prefixes, suffixes| Room { prefixes, suffixes };
    let search = |candidates: &[Candidate], room| {
        let mut search = Search::new(candidates, 2, room, 0);
        search.walk(0, &mut Vec::new());
        let best = search.best.map(|(_, reading)| reading);
        (best, search.ambiguous)
    };
    // The line 1 split into two prefixes, or a prefix and a suffix; the line 0 alone.
    let candidates = [
        candidate(0..1, &[Prefix], false),
        split(1..2, Prefix, Prefix, 1),
        split(1..2, Prefix, Suffix, 1),
    ];

    // Room for both ways.
    assert_eq!(search(&candidates, room(3, 3)), (Some(vec![0, 1]), vec![1]));
    // Room for the prefix and the suffix only: one way fits.
    assert_eq!(search(&candidates, room(2, 1)), (Some(vec![0, 2]), vec![]));
    // A line split in another way by worse readings only isn't ambiguous: its split
    // alone reads a line fewer than the one with the line 0.
    let candidates = [
        split(0..2, Prefix, Prefix, 1),
        split(1..2, Prefix, Suffix, 1),
    ];
    assert_eq!(search(&candidates, room(3, 3)), (Some(vec![0]), vec![]));
    // A split over the lines 1 and 2, a hybrid, against one of the line 2 only, alone: every
    // line of both.
    let candidates = [
        candidate(0..1, &[Prefix], false),
        split(1..3, Prefix, Prefix, 2),
        split(2..3, Prefix, Prefix, 2),
    ];
    assert_eq!(differently_split(&candidates, &[0, 1], &[0, 2]), [1, 2, 2]);
    assert_eq!(differently_split(&candidates, &[0, 1], &[0, 1]), [0; 0]);
    assert_eq!(differently_split(&candidates, &[0], &[0]), [0; 0]);
    // Room for one split only, of either line: both ambiguous, the line 0 without tier
    // looked at first.
    let candidates = [
        split(0..1, Prefix, Prefix, 0),
        split(1..2, Prefix, Prefix, 1),
    ];
    assert_eq!(search(&candidates, room(2, 0)), (Some(vec![1]), vec![1, 0]));
}

/// A candidate of the modifiers `modifiers` that can be `affixes`, without tiers.
fn candidate(modifiers: std::ops::Range<usize>, affixes: &[Affix], fractured: bool) -> Candidate {
    let tiers = ModifierTiers {
        tiers: Vec::new(),
        found: 0,
        neighbour: None,
        counts: Vec::new(),
        tier_unwritten: false,
    };
    Candidate {
        modifiers,
        fractured,
        affixes: affixes
            .iter()
            .map(|&affix| (affix, tiers.clone()))
            .collect(),
        added: None,
    }
}

/// A way to split a line into the modifiers of the tables `tables`, in the tiers of indexes
/// `found`, without ranges.
fn way(tables: (usize, usize), (found, added_found): (usize, usize)) -> Split {
    let tiers = |found| ModifierTiers {
        tiers: Vec::new(),
        found,
        neighbour: None,
        counts: Vec::new(),
        tier_unwritten: false,
    };
    Split {
        tables,
        affix: Affix::Prefix,
        tiers: tiers(found),
        added: AddedTiers {
            line: 0,
            affix: Affix::Prefix,
            tiers: tiers(added_found),
        },
        fit: Fit::Within,
    }
}

/// The tables of each way, and the index of the tier of each of its modifiers with its
/// neighbour.
type WayTiers = (
    (usize, usize),
    (usize, Option<usize>),
    (usize, Option<usize>),
);

fn way_tiers(ways: &[Split]) -> Vec<WayTiers> {
    ways.iter()
        .map(|way| {
            let tiers = |tiers: &ModifierTiers| (tiers.found, tiers.neighbour);
            (way.tables, tiers(&way.tiers), tiers(&way.added.tiers))
        })
        .collect()
}

#[test]
fn two_ways_that_differ_by_a_neighbouring_tier_of_one_modifier_are_one() {
    let joined = |ways: &[Split]| way_tiers(&neighbours_joined(ways));

    // The added modifier in the tier of index 1 or 0, or the summed one: the worse found,
    // the better its neighbour (the summed one on an item:
    // `the_summed_part_of_a_split_line_in_two_neighbouring_tiers_keeps_both`).
    assert_eq!(
        joined(&[way((0, 1), (0, 0)), way((0, 1), (0, 1))]),
        [((0, 1), (0, None), (1, Some(0)))]
    );
    assert_eq!(
        joined(&[way((0, 1), (2, 1)), way((0, 1), (1, 1))]),
        [((0, 1), (2, Some(1)), (1, None))]
    );
    // Both modifiers in other tiers, tiers that aren't neighbours, three ways (also a
    // modifier in three tiers), or two pairs of modifiers: apart.
    let apart = [
        vec![way((0, 1), (0, 1)), way((0, 1), (1, 0))],
        vec![way((0, 1), (0, 0)), way((0, 1), (0, 2))],
        vec![
            way((0, 1), (0, 0)),
            way((0, 1), (0, 1)),
            way((0, 1), (1, 0)),
        ],
        vec![
            way((0, 1), (0, 0)),
            way((0, 1), (0, 1)),
            way((0, 1), (0, 2)),
        ],
        vec![way((0, 1), (0, 0)), way((0, 2), (0, 1))],
    ];
    for ways in apart {
        assert_eq!(joined(&ways), way_tiers(&ways));
    }
}

#[test]
fn the_room_tells_the_affix_of_the_modifiers_that_can_be_both() {
    use Affix::{Prefix, Suffix};
    let candidates = [
        candidate(0..1, &[Prefix], false),
        candidate(1..2, &[Prefix, Suffix], false),
        candidate(2..3, &[Prefix, Suffix], false),
        candidate(3..4, &[Suffix], false),
    ];
    let all = [0, 1, 2, 3];
    let room = |prefixes, suffixes| Room { prefixes, suffixes };

    // One more prefix and one more suffix: one of the two is a prefix, either.
    assert_eq!(room(2, 2).bounds(&all, &candidates), Some((1, 1)));
    assert_eq!(room(2, 2).both(&all, &candidates), (true, true));
    // No room for another suffix: both are prefixes.
    assert_eq!(room(3, 1).bounds(&all, &candidates), Some((2, 2)));
    assert_eq!(room(3, 1).both(&all, &candidates), (true, false));
    // No room for another prefix: both are suffixes.
    assert_eq!(room(1, 3).both(&all, &candidates), (false, true));
    // Too few: none fits, every affix kept.
    assert_eq!(room(1, 2).bounds(&all, &candidates), None);
    assert_eq!(room(1, 2).both(&all, &candidates), (true, true));
    assert_eq!(room(0, 5).bounds(&all, &candidates), None);
    assert_eq!(room(0, 0).bounds(&[], &candidates), Some((0, 0)));
}

#[test]
fn the_first_reading_of_the_most_modifiers_that_fits_is_taken() {
    use Affix::Prefix;
    let candidates = [
        candidate(0..1, &[Prefix], false),
        candidate(0..2, &[Prefix], false),
        candidate(1..2, &[Prefix], false),
        candidate(1..3, &[Prefix], false),
        candidate(2..3, &[Prefix], false),
    ];
    let room = Room {
        prefixes: 2,
        suffixes: 0,
    };
    let mut search = Search::new(&candidates, 3, room, 0);

    search.walk(0, &mut Vec::new());

    // The three lines alone make three prefixes; then `A, BC` and `AB, C`, the first one
    // looked at taken.
    assert_eq!(search.readings, 3);
    assert_eq!(search.best.map(|(_, reading)| reading), Some(vec![0, 3]));
}

#[test]
fn the_readings_looked_at_are_bounded() {
    use Affix::Prefix;
    let count = 30;
    let candidates: Vec<_> = (0..count)
        .flat_map(|start| {
            let pair = (start + 2 <= count).then(|| candidate(start..start + 2, &[Prefix], false));
            [Some(candidate(start..start + 1, &[Prefix], false)), pair]
        })
        .flatten()
        .collect();
    let room = Room {
        prefixes: 0,
        suffixes: 0,
    };
    let mut search = Search::new(&candidates, count, room, 0);

    search.walk(0, &mut Vec::new());

    assert_eq!(search.readings, MAX_READINGS);
    // The lines alone, looked at first.
    assert_eq!(search.best.map(|(key, _)| key.modifiers), Some(count));
}

#[test]
fn a_modifier_without_line_isnt_read() {
    let item = parse(TIARA_EN);
    // Its lines would be those of the line before: the fire resistance alone.
    let modifiers = [
        modifier(None, &["+43% to Fire Resistance"]),
        modifier(None, &[]),
    ];

    let found = found_tiers(&item, &modifiers, (3, 3));

    assert_eq!(found.len(), 1);
    assert_eq!(
        found.first().map(|found| found.modifiers.clone()),
        Some(0..1)
    );
}
