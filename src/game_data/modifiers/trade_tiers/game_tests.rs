//! The reading of an item copied from the in-game trade on the tables of the game and the test
//! items: its hybrids, its affixes in doubt, its summed lines and its room. The numbers of a
//! patch: when the game data are generated again, these tests are the ones to record again,
//! while those of `tests`, on tables written by hand, must still pass as they are.

use super::*;
use crate::game_data::modifiers::fixtures::{
    ADVANCED_COPIES, Added, BOW_EN, BOW_FR, LinesRead, TIARA_EN, TIARA_FR, Written, added,
    affix_limits, as_in_game_trade, english_item, lines_read, modifier, parse, range, summary,
    trade_item,
};
use crate::item::{Item, ItemState};

mod random_items;

/// The tiers of `modifiers` found from their values on the base of `item`, with at most
/// `limits` prefixes and suffixes.
fn found_tiers(
    item: &Item,
    modifiers: &[Modifier],
    (prefixes, suffixes): (u32, u32),
) -> Vec<FoundTiers> {
    let limits = AffixLimits::new(prefixes, suffixes);
    trade_tiers(&trade_item(item, item.base_type(), limits), modifiers)
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
    unrevealed.is_unrevealed = true;
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

#[test]
fn the_tiers_found_from_the_values_alone_are_those_of_the_advanced_copy() {
    let mut found = 0;
    let mut sanctified = 0;
    let mut without_tier = Vec::new();
    let mut unknown_affix = Vec::new();
    for (text, base_type) in ADVANCED_COPIES {
        let item = parse(text);
        let base_type = base_type.unwrap_or(item.base_type());
        let limits = affix_limits(&item);
        let (modifiers, written) = as_in_game_trade(&item, base_type);

        let read = trade_tiers(&trade_item(&item, base_type, limits), &modifiers);

        // The values of a sanctified item can have left their tiers: none is found.
        if item.has_state(ItemState::Sanctified) {
            assert_eq!(read, [], "{base_type}");
            sanctified += written.len();
            continue;
        }
        for Written {
            modifiers: lines,
            affix,
            tier,
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
                .any(|&(read, number)| read == affix && number == tier);
            assert!(same, "{text}: {tiers:?}, written {affix:?} {tier:?}");
            if tiers.len() > 1 {
                unknown_affix.push(text);
            } else {
                found += 1;
            }
        }
    }
    // With the 4 modifiers of Amanamu, a lord of the Abyss, among the desecrated tables,
    // `Reinforcing` of the Genesis tree on the Breach Rings, the lines of the Time-Lost jewels,
    // each within the words of the passive skills it grants but the radius, and the Onslaught
    // of an essence on a bow.
    assert_eq!(found, 64);
    // The 5 modifiers of the owner's sanctified sceptre, in both languages: read from its
    // values, its `+74 to maximum Mana`, a T5 `80–89`, would be the T6.
    assert_eq!(sanctified, 10);
    assert_eq!(
        without_tier,
        [
            // An essence of an older patch: the bows take `+2` today (trade site, 2026-10-30).
            "+3 to Level of all Attack Skills",
            // Copied before a patch changed its ranges.
            "Adds 1 to 50 Lightning Damage",
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

#[test]
fn a_base_the_data_doesnt_know_has_no_tiers_found() {
    let item = parse(TIARA_EN);
    let limits = AffixLimits::new(3, 3);

    let found = trade_tiers(&english_item("Unknown Tiara", 82, limits), item.modifiers());

    assert_eq!(found, []);
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
fn lines_that_fit_pure_modifiers_and_their_hybrid_have_no_tier_while_the_item_has_room() {
    use Affix::Prefix;
    let item = parse(TIARA_EN);
    let modifiers = energy_shield_and_life();

    // The pure T6 and T8, or their hybrid T3: nothing tells which rolled.
    let found = found_tiers(&item, &modifiers, (3, 3));
    assert_eq!(found, []);

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
    let limits = AffixLimits::new(1, 3);
    let read = |item_level| {
        let found = trade_tiers(
            &english_item("Ancestral Tiara", item_level, limits),
            &modifiers,
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
            &english_item("Ancestral Tiara", item_level, limits),
            &rarity,
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
    let limits = AffixLimits::new(3, 3);
    let found = trade_tiers(
        &english_item("Bronze Greaves", item_level, limits),
        modifiers,
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
    let limits = AffixLimits::new(3, 3);
    let read = |item_level| {
        let found = trade_tiers(
            &english_item("Ancestral Tiara", item_level, limits),
            &rarity,
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
    let limits = AffixLimits::new(3, 3);

    let found = trade_tiers(&english_item("Soldier Cuirass", 82, limits), &modifiers);

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
        let limits = AffixLimits::new(prefixes, 3);
        let found = trade_tiers(
            &english_item("Warmonger Bow", item_level, limits),
            &modifiers,
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
fn no_tier_is_found_from_the_values_of_a_sanctified_item() {
    let item = parse(BOW_EN);
    let read = |sanctified| {
        trade_tiers(
            &TradeItem {
                is_sanctified: sanctified,
                ..trade_item(&item, item.base_type(), AffixLimits::new(3, 4))
            },
            item.modifiers(),
        )
    };

    // Its values can each have left their tier, for the one next to it as well.
    assert_ne!(read(false), []);
    assert_eq!(read(true), []);
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
    let limits = AffixLimits::new(3, 3);

    let found = trade_tiers(&english_item("Ancestral Tiara", 82, limits), &modifiers);

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
        "../../../item/test-items/fr-trade-rare-body-armour.txt"
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
        let limits = AffixLimits::new(3, 3);
        trade_tiers(&english_item("Soldier Cuirass", 82, limits), &modifiers)
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
