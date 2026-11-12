//! The modifiers of the test items written again from their values, on the tables of the game:
//! the affix, the tier and the ranges the advanced copy writes. The numbers of a patch, to
//! record again when the game data are generated again.

use super::*;
use crate::game_data::modifiers::fixtures::{
    ADVANCED_COPIES, BOW_EN, TIARA_EN, affix_tier, as_in_game_trade, desecrated, english_item,
    first_doubt, modifier, parse, part, range, tiers, trade_item, trade_lines,
};
use crate::game_data::modifiers::trade_tiers::AffixLimits;
use crate::item::{Lord, ModifierOrigin, StatLine};

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

/// The range of each value of the lines of `modifier`, as written under it
/// ([`Range::unless_fixed`]): none when every value is fixed, as the advanced copy most often
/// writes them.
fn shown_ranges(modifier: &Modifier) -> Vec<Option<Range>> {
    let rolls: Vec<(f64, Option<Range>)> = modifier
        .lines
        .iter()
        .flat_map(|line| &line.rolls)
        .map(|roll| (roll.value, roll.range.and_then(Range::magnitude)))
        .collect();
    let fixed = rolls.iter().all(|&(value, range)| {
        range.is_none_or(|range| Range::unless_fixed([(value, range)]).is_empty())
    });
    if fixed {
        return vec![None; rolls.len()];
    }
    rolls.into_iter().map(|(_, range)| range).collect()
}

#[test]
fn the_lines_read_again_from_their_values_are_the_modifiers_of_the_advanced_copy() {
    let mut read_again = 0;
    for (text, base_type, limits) in ADVANCED_COPIES {
        let item = parse(text);
        let base_type = base_type.unwrap_or(item.base_type());
        let (modifiers, written) = as_in_game_trade(&item, base_type);

        let read = with_trade_tiers(&trade_item(&item, base_type, limits), &modifiers);

        let advanced = item.modifiers().iter().filter(|modifier| {
            matches!(modifier.kind, ModifierKind::Explicit(Some(_))) && !modifier.is_unrevealed
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
            // Without tier when the copy writes none (`Reinforcing` of the Genesis tree).
            assert_eq!(again.tier, modifier.tier.and(written.tier), "{text}");
            // As both copies show them: no range under a fixed value, which the advanced
            // copy writes without range (`1% of Damage taken Recouped as Life` of a
            // Time-Lost jewel), the tables with its range of one value.
            assert_eq!(shown_ranges(again), shown_ranges(modifier), "{text}");
            read_again += 1;
        }
        // The other lines stay as they were.
        let lines =
            |modifiers: &[Modifier]| -> Vec<String> { modifiers.iter().flat_map(texts).collect() };
        assert_eq!(lines(&read), lines(&modifiers));
    }
    // With `Reinforcing` of the Breach Rings and the Onslaught of an essence on a bow, read
    // again without tier, as the copy writes them, and the lines of the Time-Lost jewels;
    // without the sanctified sceptre, whose lines stay as they were.
    assert_eq!(read_again, 64);
}

#[test]
fn the_lines_of_a_tiara_read_again_join_its_hybrid_and_keep_an_unknown_affix() {
    let item = parse(TIARA_EN);
    let read = |modifiers: &[Modifier]| {
        let limits = AffixLimits::new(3, 3);
        with_trade_tiers(&trade_item(&item, item.base_type(), limits), modifiers)
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
    let limits = AffixLimits::new(3, 3);
    let read = |modifiers: &[Modifier]| {
        with_trade_tiers(&trade_item(&item, item.base_type(), limits), modifiers)
    };
    let find = |modifiers: &[Modifier], words: &str| {
        modifiers
            .iter()
            .find(|modifier| modifier.lines[0].text.contains(words))
            .cloned()
            .expect(words)
    };
    let tier = |affix, min, max| affix_tier(affix, 1, &[(min, max)]);
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

/// `modifiers` of an item of `base_type` and `item_level`, in English, read again with the
/// tiers found from their values, with at most `(prefixes, suffixes)`.
fn read_again(
    base_type: &str,
    item_level: u32,
    modifiers: &[Modifier],
    (prefixes, suffixes): (u32, u32),
) -> Vec<Modifier> {
    let limits = AffixLimits::new(prefixes, suffixes);
    with_trade_tiers(&english_item(base_type, item_level, limits), modifiers)
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
    let limits = AffixLimits::new(3, 4);

    let read = with_trade_tiers(
        &trade_item(&item, item.base_type(), limits),
        item.modifiers(),
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

    // A fixed value: no range, as under any modifier; the prefix of Amanamu named, its
    // number hidden.
    let of_amanamu = AffixTier {
        hides_number: true,
        lord: Some(Lord::Amanamu),
        ..affix_tier(Affix::Prefix, 1, &[])
    };
    assert_eq!(
        first_doubt(&read),
        Some(TierDoubt::EitherAffix {
            prefix: of_amanamu,
            suffix: affix_tier(Affix::Suffix, 2, &[]),
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
        "../../../item/test-items/en-owner-rare-ring-breach.txt"
    ));
    let (modifiers, _) = as_in_game_trade(&item, "Breach Ring");

    let read = read_again("Breach Ring", 77, &modifiers, (3, 3));

    let prefix = ModifierKind::Explicit(Some(Affix::Prefix));
    let suffix = ModifierKind::Explicit(Some(Affix::Suffix));
    let ranged = |min, max| vec![Some(range(min, max))];
    // A prefix, which fills the room of the prefixes, without tier, over the range of its tier:
    // what the advanced copy writes of it, which its bar and its field are drawn from.
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
}

#[test]
fn a_minion_modifier_of_the_genesis_tree_merges_with_no_ordinary_one() {
    let levels = ["+2 to Level of all Minion Skills"];
    // Written without tier, as the game writes the tree's; with its T2 on an amulet.
    let suffix = modifier(Some(Affix::Suffix), &levels);
    let of_tier_2 = Modifier {
        tier: Some(2),
        ..suffix.clone()
    };
    let numbers = |base_type: &str| {
        let suffix = if base_type.ends_with("Amulet") {
            &of_tier_2
        } else {
            &suffix
        };
        tiers(base_type, suffix).map(|tiers| {
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
    // Written without tier on an amulet, as the game writes the tree's only: the tree's T1 of 2.
    let unwritten = tiers("Gold Amulet", &suffix).map(|tiers| {
        let number = tiers.tier().map(|tier| tier.number);
        (tiers.tiers.len(), number, tiers.hides_number())
    });
    assert_eq!(unwritten, Some((2, Some(1), true)));
    assert_eq!(read("Gold Amulet"), [(kind, Some(2), two.clone())]);
    // On a belt, the tree's only, whose tier the game doesn't write.
    assert_eq!(numbers("Heavy Belt"), Some((2, Some(1), true)));
    assert!(tiers("Heavy Belt", &suffix).is_some_and(|tiers| !tiers.single()));
    assert_eq!(read("Heavy Belt"), [(kind, None, two)]);
    // The tree rules it out of the rings.
    assert_eq!(numbers("Gold Ring"), None);
}

#[test]
fn a_line_of_an_alloy_copied_from_the_in_game_trade_gets_its_affix_and_range() {
    // Marked crafted, as the game writes it.
    let read = |base_type: &str, level: u32, line: &str| {
        let mut lines = trade_lines(&[line]);
        lines[0].origin = Some(ModifierOrigin::Crafted);
        explicit_tiers(&read_again(base_type, level, &lines, (3, 3)))
    };

    // Without tier, as the advanced copy writes it.
    assert_eq!(
        read(
            "Adorned Wraps",
            80,
            "11% increased Attack Speed while missing Runic Ward"
        ),
        [(
            ModifierKind::Explicit(Some(Affix::Suffix)),
            None,
            vec![Some(range(10.0, 15.0))]
        )]
    );
    // Unmarked, no alloy's: the line stays as it was.
    let unmarked = trade_lines(&["11% increased Attack Speed while missing Runic Ward"]);
    assert_eq!(
        explicit_tiers(&read_again("Adorned Wraps", 80, &unmarked, (3, 3))),
        [(ModifierKind::Explicit(None), None, vec![None])]
    );
    // Whatever the level of the item, below the 25 of the modifier in RePoE: a helmet of level
    // 7 on the trade site (2026-10-31).
    assert_eq!(
        read("Face Mask", 7, "26% increased Mana Cost Efficiency"),
        [(
            ModifierKind::Explicit(Some(Affix::Prefix)),
            None,
            vec![Some(range(18.0, 29.0))]
        )]
    );
}

#[test]
fn a_line_of_an_essence_copied_from_the_in_game_trade_gets_its_affix_and_range() {
    let read = |base_type: &str, line: &str| {
        explicit_tiers(&read_again(base_type, 80, &trade_lines(&[line]), (3, 3)))
    };
    let prefix = ModifierKind::Explicit(Some(Affix::Prefix));

    // A bow takes the range of the one-handed weapons, a crossbow that of the two-handed ones;
    // without tier, as the advanced copy writes it.
    assert_eq!(
        read("Gemini Bow", "Gain 18% of Damage as Extra Fire Damage"),
        [(prefix, None, vec![Some(range(15.0, 20.0))])]
    );
    assert_eq!(
        read("Gemini Crossbow", "Gain 30% of Damage as Extra Fire Damage"),
        [(prefix, None, vec![Some(range(25.0, 33.0))])]
    );
    // Whatever the level of the item, below the 72 of the Perfect essences in RePoE: a Cultist
    // Bow of level 33 on the trade site (2026-10-30).
    let low = read_again(
        "Cultist Bow",
        33,
        &trade_lines(&["+2 to Level of all Attack Skills"]),
        (3, 3),
    );
    let suffix = ModifierKind::Explicit(Some(Affix::Suffix));
    assert_eq!(
        explicit_tiers(&low),
        [(suffix, None, vec![Some(range(2.0, 2.0))])]
    );
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
fn a_desecrated_line_is_never_one_of_a_currency() {
    let mut infusions = trade_lines(&["+1 to maximum number of Elemental Infusions"]);
    infusions[0].origin = Some(ModifierOrigin::Desecrated);
    let mut defences = trade_lines(&["22% increased Global Armour, Evasion and Energy Shield"]);
    defences[0].origin = Some(ModifierOrigin::Desecrated);

    // On a staff, the prefix of Kurgal (1–2), not the suffix of an alloy (1) as well.
    assert_eq!(
        explicit_tiers(&read_again("Ashen Staff", 82, &infusions, (3, 3))),
        [(
            ModifierKind::Explicit(Some(Affix::Prefix)),
            Some(1),
            vec![Some(range(1.0, 2.0))]
        )]
    );
    // On an amulet, the 15–25 of Amanamu, not the 20–30 of an essence, before it in the file.
    assert_eq!(
        explicit_tiers(&read_again("Gold Amulet", 82, &defences, (3, 3))),
        [(
            ModifierKind::Explicit(Some(Affix::Prefix)),
            Some(1),
            vec![Some(range(15.0, 25.0))]
        )]
    );
}

#[test]
fn only_lines_marked_crafted_may_be_a_hybrid_of_an_alloy() {
    // The Celestial Alloy on a wand, `+1 to Level of all Spell Skills` and `+142–188 to
    // maximum Mana`, both lines marked crafted from the in-game trade: its hybrid, or two
    // crafted ordinary modifiers, the mana maybe adding up a pure one and the part of the
    // alloy; every reading as good, both lines stay without tier.
    let mut lines = trade_lines(&["+1 to Level of all Spell Skills", "+160 to maximum Mana"]);
    let read = |lines: &[Modifier]| {
        explicit_tiers(&read_again("Acrid Wand", 82, lines, (3, 3)))
            .into_iter()
            .map(|(kind, tier, _)| (kind, tier.is_some()))
            .collect::<Vec<_>>()
    };
    let unmarked = read(&lines);
    for line in &mut lines {
        line.origin = Some(ModifierOrigin::Crafted);
    }

    assert_eq!(
        read(&lines),
        [
            (ModifierKind::Explicit(None), false),
            (ModifierKind::Explicit(None), false),
        ]
    );
    // Unmarked, two ordinary modifiers, the T4 of the spell levels and the T1 of the mana:
    // the mana as the sum of a pure one and of the part of the alloy would put them in doubt.
    assert_eq!(
        unmarked,
        [
            (ModifierKind::Explicit(Some(Affix::Suffix)), true),
            (ModifierKind::Explicit(Some(Affix::Prefix)), true),
        ]
    );
}
