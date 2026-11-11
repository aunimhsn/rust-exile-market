use super::*;
use crate::item::{Affix, AffixTier, CopiedItem, Item, PartTier, StatLine};

fn range(min: f64, max: f64) -> Range {
    Range { min, max }
}

fn rolls_of(text: &str) -> TierRolls {
    let item = Item::parse(&CopiedItem::from_text(text).expect("an item")).expect("read");
    TierRolls::new(
        item.language(),
        item.modifiers(),
        Some(item.base_type()),
        item.item_level(),
    )
}

fn key(modifier: usize, line: usize) -> StatKey {
    StatKey { modifier, line }
}

fn one(min: f64, max: f64, lowest: u32, highest: u32) -> LineRolls {
    LineRolls {
        ranges: vec![Some(range(min, max))],
        tiers: Some(TierSpan { lowest, highest }),
    }
}

#[test]
fn a_line_rolls_from_the_lowest_roll_of_its_worst_tier_to_the_highest_of_tier_1() {
    let rolls = rolls_of(
        "Item Class: Body Armours\nRarity: Rare\nDoom Shell\nAncestral Mail\n--------\n\
         Item Level: 82\n--------\n\
         { Prefix Modifier \"Vigorous\" (Tier: 7) — Life }\n+86(85-99) to maximum Life\n\
         { Prefix Modifier \"Lacquered\" (Tier: 4) — Defences }\n\
         +98(86-102) to Armour\n+88(79-94) to Evasion Rating\n\
         { Suffix Modifier \"of the Penguin\" (Tier: 1) — Elemental, Cold, Resistance }\n\
         +41(41-45)% to Cold Resistance",
    );

    assert_eq!(rolls.get(key(0, 0)), Some(&one(10.0, 214.0, 13, 1)));
    // Each line of a hybrid over the tiers of its own numbers.
    assert_eq!(rolls.get(key(1, 0)), Some(&one(9.0, 170.0, 8, 1)));
    assert_eq!(rolls.get(key(1, 1)), Some(&one(6.0, 161.0, 8, 1)));
    assert_eq!(rolls.get(key(2, 0)), Some(&one(6.0, 45.0, 8, 1)));
}

#[test]
fn a_modifier_of_one_tier_only_on_its_base_has_no_tier_to_tell() {
    let rolls = rolls_of(
        "Item Class: Jewels\nRarity: Rare\nViper Glimmer\nRuby\n--------\n\
         Item Level: 43\n--------\n\
         { Prefix Modifier \"Brutish\" (Tier: 1) — Minion, Life }\n\
         Minions have 12(5-15)% increased maximum Life",
    );

    assert!(rolls.hides_number(0));
    // Over its only tier, without "T1–T1".
    assert_eq!(
        rolls.get(key(0, 0)),
        Some(&LineRolls {
            ranges: vec![Some(range(5.0, 15.0))],
            tiers: None,
        })
    );
}

#[test]
fn a_modifier_copied_with_another_tier_than_the_only_one_of_the_data_keeps_it() {
    // After a patch that adds a tier the data doesn't know yet.
    let rolls = rolls_of(
        "Item Class: Jewels\nRarity: Rare\nViper Glimmer\nRuby\n--------\n\
         Item Level: 43\n--------\n\
         { Prefix Modifier \"Brutish\" (Tier: 2) — Minion, Life }\n\
         Minions have 12(5-15)% increased maximum Life",
    );

    assert!(!rolls.hides_number(0));
    // Nor "T1–T1" against its T2.
    assert_eq!(rolls.get(key(0, 0)).map(|rolls| rolls.tiers), Some(None));
}

#[test]
fn a_hybrid_of_one_tier_only_has_no_tier_to_tell() {
    let flask = "Item Class: Life Flasks\nRarity: Magic\nSaturated Ultimate Life Flask\n\
                 --------\nItem Level: 82\n--------\n\
                 { Prefix Modifier \"Saturated\" (Tier: 1) — Life }\n\
                 Instant Recovery\n50(50)% reduced Amount Recovered";
    let item = Item::parse(&CopiedItem::from_text(flask).expect("an item")).expect("read");
    // The base of a magic item, from the names of the trade site.
    let rolls = TierRolls::new(
        item.language(),
        item.modifiers(),
        Some("Ultimate Life Flask"),
        item.item_level(),
    );

    assert!(rolls.hides_number(0));
    // A line without number has no rolls.
    assert_eq!(rolls.get(key(0, 0)), None);
    assert_eq!(
        rolls.get(key(0, 1)),
        Some(&LineRolls {
            ranges: vec![Some(range(50.0, 50.0))],
            tiers: None,
        })
    );
}

#[test]
fn a_fixed_value_without_tier_rolls_over_its_tier_only_of_the_genesis_tree_or_an_essence() {
    // `of the Despot` of the Genesis tree on a belt, the best of its two tiers (+1, +2),
    // which the copy writes without tier nor range: fixed in its tier.
    let belt = rolls_of(
        "Item Class: Belts\nRarity: Rare\nGrip Clasp\nHeavy Belt\n--------\n\
         Item Level: 79\n--------\n\
         { Suffix Modifier \"of the Despot\" — Minion, Gem }\n\
         +2 to Level of all Minion Skills",
    );
    assert!(belt.hides_number(0));
    // Of the tree, not a currency's.
    assert_eq!(belt.currency(0), None);
    let fixed = LineRolls {
        ranges: vec![Some(range(2.0, 2.0))],
        tiers: None,
    };
    assert_eq!(belt.get(key(0, 0)), Some(&fixed));

    // An essence on a bow, of one value.
    let bow = rolls_of(
        "Item Class: Bows\nRarity: Rare\nMiracle Siege\nObliterator Bow\n--------\n\
         Item Level: 81\n--------\n\
         { Suffix Modifier \"of the Essence\" — Attack }\n\
         +2 to Level of all Attack Skills",
    );
    assert!(bow.hides_number(0));
    assert_eq!(bow.currency(0), Some(Currency::Essence));
    assert_eq!(bow.get(key(0, 0)), Some(&fixed));
    // On a wand, where its value is the ordinary T2 of 4 as well: the essence's, fixed.
    let wand = |header: &str| {
        rolls_of(&format!(
            "Item Class: Wands\nRarity: Rare\nWoe Song\nAcrid Wand\n--------\n\
             Item Level: 81\n--------\n\
             {{ Suffix Modifier {header} — Caster, Gem }}\n\
             +3 to Level of all Spell Skills"
        ))
    };
    let essence = wand("\"of the Essence\"");
    assert!(essence.hides_number(0));
    assert_eq!(essence.currency(0), Some(Currency::Essence));
    assert_eq!(
        essence.get(key(0, 0)),
        Some(&LineRolls {
            ranges: vec![Some(range(3.0, 3.0))],
            tiers: None,
        })
    );
    assert!(!wand("\"of the Magus\" (Tier: 2)").hides_number(0));

    // The same without tier on a quiver, a modifier of the tables, of one tier there, but
    // neither of the tree nor of an essence (none adds it): no rolls.
    let quiver = |header: &str| {
        rolls_of(&format!(
            "Item Class: Quivers\nRarity: Rare\nStorm Song\nFire Quiver\n--------\n\
             Item Level: 79\n--------\n\
             {{ Suffix Modifier {header} — Attack, Gem }}\n\
             +1 to Level of all Projectile Skills"
        ))
    };
    assert!(quiver("\"of the Archer\" (Tier: 1)").hides_number(0));
    let essence = quiver("\"of the Essence\"");
    assert!(!essence.hides_number(0));
    assert_eq!(essence.get(key(0, 0)), None);

    // Nor a value of an essence that falls in a tier of the tree that rolls, `Reinforcing`
    // (19–22), which the copy would write with its range.
    let ring = rolls_of(
        "Item Class: Rings\nRarity: Rare\nGlyph Circle\nBreach Ring\n--------\n\
         Item Level: 77\n--------\n\
         { Prefix Modifier \"of the Essence\" — Life, Minion }\n\
         Minions have 21% increased maximum Life",
    );
    assert!(!ring.hides_number(0));
    assert_eq!(ring.get(key(0, 0)), None);
}

#[test]
fn a_modifier_written_without_tier_rolls_over_its_copied_range() {
    let rolls = rolls_of(
        "Item Class: Rings\nRarity: Rare\nGlyph Circle\nBreach Ring\n--------\n\
         Item Level: 77\n--------\n\
         { Prefix Modifier \"Reinforcing\" — Life, Minion }\n\
         Minions have 21(19-22)% increased maximum Life\n\
         { Prefix Modifier \"Aqua\" (Tier: 7) — Mana }\n\
         +76(65-79) to maximum Mana",
    );

    // The Genesis tree, whose tier the game doesn't write: over the copied range.
    assert!(rolls.hides_number(0));
    assert_eq!(
        rolls.get(key(0, 0)),
        Some(&LineRolls {
            ranges: vec![Some(range(19.0, 22.0))],
            tiers: None,
        })
    );
    assert!(!rolls.hides_number(1));
    assert!(
        rolls
            .get(key(1, 0))
            .is_some_and(|rolls| rolls.tiers.is_some())
    );

    // Copied from the in-game trade, without its range: no rolls.
    let rolls = rolls_of(
        "Item Class: Rings\nRarity: Rare\nGlyph Circle\nBreach Ring\n--------\n\
         Item Level: 77\n--------\n\
         Minions have 21% increased maximum Life",
    );
    assert!(!rolls.hides_number(0));
    assert_eq!(rolls.get(key(0, 0)), None);

    // In French, and before the base is known (a magic item before the names of the trade
    // site): its copied range needs no base.
    let french = "Classe d'objet: Bagues\nRareté: Rare\nCercle glyphe\nBague de Brèche\n\
                  --------\nNiveau de l'objet: 77\n--------\n\
                  { Préfixe : « Renforçant » — Vie, Créature }\n\
                  Les Créatures ont 21(19-22)% d'Augmentation de Vie maximale";
    let item = Item::parse(&CopiedItem::from_text(french).expect("an item")).expect("read");
    let rolls = TierRolls::new(item.language(), item.modifiers(), None, None);
    assert!(rolls.hides_number(0));
    assert_eq!(
        rolls.get(key(0, 0)),
        Some(&LineRolls {
            ranges: vec![Some(range(19.0, 22.0))],
            tiers: None,
        })
    );
}

#[test]
fn a_line_of_two_numbers_rolls_each_over_the_tiers_without_tiers_to_tell() {
    let rolls = rolls_of(
        "Item Class: Bows\nRarity: Rare\nStorm Bite\nGemini Bow\n--------\n\
         Item Level: 81\n--------\n\
         { Prefix Modifier \"Tempered\" (Tier: 2) — Damage, Physical, Attack }\n\
         Adds 21(21-31) to 38(36-53) Physical Damage\n\
         { Suffix Modifier \"of the Sniper\" (Tier: 2) — Gem }\n\
         +3 to Level of all Projectile Skills",
    );

    assert_eq!(
        rolls.get(key(0, 0)),
        Some(&LineRolls {
            ranges: vec![Some(range(1.0, 39.0)), Some(range(4.0, 66.0))],
            tiers: None,
        })
    );
    // Fixed in its tier, not over the tiers.
    assert_eq!(rolls.get(key(1, 0)), Some(&one(1.0, 4.0, 4, 1)));
}

#[test]
fn a_reduced_line_rolls_without_its_sign() {
    let rolls = rolls_of(
        "Item Class: Belts\nRarity: Rare\nGlyph Cord\nDouble Belt\n--------\n\
         Item Level: 82\n--------\n\
         { Suffix Modifier \"of Sipping\" (Tier: 2) — Flask }\n\
         21(20-22)% reduced Flask Charges used",
    );

    // The table rolls -8 to -25: the game writes 8 to 25.
    assert_eq!(rolls.get(key(0, 0)), Some(&one(8.0, 25.0, 6, 1)));
}

#[test]
fn a_french_line_rolls_over_the_tiers_of_its_english_modifier() {
    let rolls = rolls_of(include_str!("../item/test-items/fr-rare-body-armour.txt"));

    // After its rune and its implicit, `+70(61-73) de Bouclier d'énergie maximal`, the
    // energy shield T4 among 11.
    assert_eq!(
        rolls.get(key(2, 0)).and_then(|rolls| rolls.tiers),
        Some(TierSpan {
            lowest: 11,
            highest: 1
        })
    );
}

#[test]
fn a_line_has_no_rolls_without_its_tier_nor_base() {
    // A header the parser doesn't know gives no tier: its modifier isn't looked for in the
    // tables.
    let unknown = "Item Class: Body Armours\nRarity: Rare\nDoom Shell\nAncestral Mail\n\
                   --------\nItem Level: 82\n--------\n{ Strange Modifier }\n\
                   +86(85-99) to maximum Life";
    assert_eq!(rolls_of(unknown).get(key(0, 0)), None);

    let advanced = "Item Class: Body Armours\nRarity: Rare\nDoom Shell\nAncestral Mail\n\
                    --------\nItem Level: 82\n--------\n\
                    { Prefix Modifier \"Vigorous\" (Tier: 7) — Life }\n\
                    +86(85-99) to maximum Life";
    let item = Item::parse(&CopiedItem::from_text(advanced).expect("an item")).expect("read");
    let rolls = TierRolls::new(item.language(), item.modifiers(), None, None);
    assert_eq!(rolls.get(key(0, 0)), None);
}

#[test]
fn the_other_line_of_a_hybrid_whose_part_a_line_adds_up_rolls_over_the_tiers_of_the_hybrid() {
    let part = |tiers: Vec<u32>, min, max| PartTier {
        affix: Affix::Prefix,
        tiers,
        hides_number: false,
        ranges: vec![range(min, max)],
        currency: None,
    };
    // 258% is a pure T1 (170–179) and the hybrid T1 (75–79) with the accuracy.
    let split = Modifier {
        doubt: Some(TierDoubt::Split {
            line: 0,
            whole: part(vec![1], 75.0, 79.0),
            added: part(vec![1], 170.0, 179.0),
        }),
        ..modifier(
            PREFIX,
            None,
            &[
                "258% increased Physical Damage",
                "+195(175-200) to Accuracy Rating",
            ],
        )
    };

    let rolls = TierRolls::new(
        Language::English,
        std::slice::from_ref(&split),
        Some("Warmonger Bow"),
        None,
    );

    // A sum over no tiers; the accuracy over the 8 tiers of the hybrid.
    assert_eq!(rolls.get(key(0, 0)), None);
    assert_eq!(rolls.get(key(0, 1)), Some(&one(16.0, 200.0, 8, 1)));

    // A part whose ranges the hybrid doesn't give the line: another table, no rolls.
    let mut other = split.clone();
    other.doubt = Some(TierDoubt::Split {
        line: 0,
        whole: part(vec![1], 76.0, 79.0),
        added: part(vec![1], 170.0, 179.0),
    });
    let rolls = TierRolls::new(Language::English, &[other], Some("Warmonger Bow"), None);
    assert_eq!(rolls.get(key(0, 1)), None);

    // Two pure modifiers on one line: no rolls.
    let mut pure = split;
    pure.lines.truncate(1);
    pure.doubt = Some(TierDoubt::Split {
        line: 0,
        whole: part(vec![1], 75.0, 79.0),
        added: part(vec![1], 170.0, 179.0),
    });
    let rolls = TierRolls::new(Language::English, &[pure], Some("Warmonger Bow"), None);
    assert_eq!(rolls.get(key(0, 0)), None);
}

#[test]
fn the_other_line_of_a_hybrid_of_one_tier_only_whose_part_a_line_adds_up_tells_no_tiers() {
    use crate::item::ModifierOrigin;

    let part = |min, max| PartTier {
        affix: Affix::Prefix,
        tiers: vec![1],
        hides_number: true,
        ranges: vec![range(min, max)],
        currency: None,
    };
    // The hybrid of Kurgal (110–154, and 15% reduced Attack Speed), desecrated, and a pure
    // T1 (170–179) of a maul.
    let split = Modifier {
        origin: Some(ModifierOrigin::Desecrated),
        doubt: Some(TierDoubt::Split {
            line: 0,
            whole: part(110.0, 154.0),
            added: part(170.0, 179.0),
        }),
        ..modifier(
            PREFIX,
            None,
            &["300% increased Physical Damage", "15% reduced Attack Speed"],
        )
    };

    let rolls = TierRolls::new(
        Language::English,
        std::slice::from_ref(&split),
        Some("Temple Maul"),
        Some(82),
    );

    // Over its one tier, without "T1–T1"; the modifier not counted as one of one tier
    // only, the split line having no tier.
    let speed = rolls.get(key(0, 1)).expect("the rolls of the Attack Speed");
    assert_eq!(speed.ranges, [Some(range(15.0, 15.0))]);
    assert_eq!(speed.tiers, None);
    assert!(!rolls.hides_number(0));
}

#[test]
fn the_values_of_a_match_are_shared_among_its_lines() {
    // A stat on two lines, a number on each.
    assert_eq!(
        share_values(3, 2, &[vec![0], vec![0]]),
        Some(vec![vec![(0, 3)], vec![(0, 4)]])
    );
    // A stat on two lines, its number on the first only: the other line has no roll.
    assert_eq!(
        share_values(0, 1, &[vec![0], vec![]]),
        Some(vec![vec![(0, 0)], vec![]])
    );
    // Two numbers on a line.
    assert_eq!(
        share_values(1, 2, &[vec![0, 1]]),
        Some(vec![vec![(0, 1), (1, 2)]])
    );
    // A line without number whose string gives the value: no roll.
    assert_eq!(share_values(0, 1, &[vec![]]), None);
    // More numbers than values.
    assert_eq!(share_values(0, 1, &[vec![0, 1]]), None);
}

#[test]
fn a_range_across_zero_has_no_rolls() {
    let tier = |number, min, max| Tier {
        number,
        level: 1,
        ranges: vec![range(min, max)],
    };

    assert_eq!(
        line_rolls(&[tier(1, 5.0, 10.0), tier(2, 1.0, 4.0)], &[(0, 0)], 1),
        Some(one(1.0, 10.0, 2, 1))
    );
    assert_eq!(
        line_rolls(&[tier(1, 5.0, 10.0), tier(2, -4.0, 4.0)], &[(0, 0)], 1),
        None
    );
    // A value no tier has.
    assert_eq!(line_rolls(&[tier(1, 5.0, 10.0)], &[(0, 1)], 1), None);
    // A line of two numbers whose stat counts the first only: the 3 of `for 3 seconds`.
    assert_eq!(
        line_rolls(&[tier(1, 5.0, 10.0), tier(2, 1.0, 4.0)], &[(0, 0)], 2),
        Some(LineRolls {
            ranges: vec![Some(range(1.0, 10.0)), None],
            tiers: Some(TierSpan {
                lowest: 2,
                highest: 1
            }),
        })
    );
}

/// The parser review of 2026-11-10: the best sign that the data are behind the game was
/// skipped without a line in the log.
#[test]
fn a_tier_the_copy_writes_and_no_table_holds_is_told_apart() {
    let lacks = |lines: &str| {
        let maul = format!(
            "Item Class: Two Hand Maces\nRarity: Magic\nCrackling Temple Maul of the Brute\n\
             --------\nItem Level: 40\n--------\n{lines}"
        );
        let item = Item::parse(&CopiedItem::from_text(&maul).expect("an item")).expect("read");
        let modifier = &item.modifiers()[0];
        let matches = game_data::modifier_stats(item.language(), modifier, None);
        let base = game_data::ItemBase {
            language: item.language(),
            base_type: "Temple Maul",
            item_level: item.item_level(),
        };
        game_data::lacks_table(base, modifier, &matches)
    };

    // The tier of the data.
    assert!(!lacks(
        "{ Prefix Modifier \"Crackling\" (Tier: 7) — Damage, Elemental, Lightning, Attack }\n\
         Adds 2(1-4) to 60(53-76) Lightning Damage"
    ));
    // Ranges no tier has: the game changed them.
    assert!(lacks(
        "{ Prefix Modifier \"Crackling\" (Tier: 7) — Damage, Elemental, Lightning, Attack }\n\
         Adds 2(1-5) to 60(54-79) Lightning Damage"
    ));
    // Without a tier written, nothing says a table should hold it; nor for an implicit.
    assert!(!lacks(
        "{ Prefix Modifier \"Crackling\" — Damage, Elemental, Lightning, Attack }\n\
         Adds 2(1-5) to 60(54-79) Lightning Damage"
    ));
    assert!(!lacks(
        "{ Implicit Modifier — Damage }\nAdds 2(1-5) to 60(54-79) Lightning Damage"
    ));
}

/// An item is read again at each copy: its missing table is told once.
#[test]
fn a_missing_table_is_told_once_and_not_past_a_few_hundred() {
    let mut told = BTreeSet::new();

    assert!(first_told(
        &mut told,
        "\"Crackling\" as tier Some(7) on Temple Maul"
    ));
    assert!(!first_told(
        &mut told,
        "\"Crackling\" as tier Some(7) on Temple Maul"
    ));
    assert!(first_told(
        &mut told,
        "\"Crackling\" as tier Some(6) on Temple Maul"
    ));
    for index in told.len()..MOST_TOLD {
        assert!(first_told(&mut told, &index.to_string()));
    }
    assert!(!first_told(&mut told, "one more"));
    assert_eq!(told.len(), MOST_TOLD);
}

/// A modifier of `kind`, of `tier` when written, with one line for each of `lines`.
fn modifier(kind: ModifierKind, tier: Option<u32>, lines: &[&str]) -> Modifier {
    Modifier {
        kind,
        origin: None,
        name: None,
        tier,
        tags: Vec::new(),
        is_unrevealed: false,
        lines: lines.iter().map(|line| StatLine::parse(line)).collect(),
        doubt: None,
    }
}

const PREFIX: ModifierKind = ModifierKind::Explicit(Some(Affix::Prefix));

/// The tier `number`, whose values roll over `ranges`, one range per value.
fn tier(number: u32, ranges: &[(f64, f64)]) -> Tier {
    Tier {
        number,
        level: 1,
        ranges: ranges.iter().map(|&(min, max)| range(min, max)).collect(),
    }
}

/// A modifier of one number on each of its `lines` in the tables, of `tiers`, the tier of the
/// copied one at `found`.
fn in_tables(lines: usize, tiers: Vec<Tier>, found: usize) -> ModifierInTables {
    ModifierInTables {
        matches: (0..lines)
            .map(|line| LineMatch {
                lines: line..line + 1,
                stat: None,
            })
            .collect(),
        tiers: ModifierTiers {
            tiers,
            found,
            neighbour: None,
            counts: vec![1; lines],
            tier_unwritten: false,
            currency: None,
            lord: None,
        },
    }
}

fn rolled(ranges: &[(f64, f64)], tiers: Option<(u32, u32)>) -> LineRolls {
    LineRolls {
        ranges: ranges
            .iter()
            .map(|&(min, max)| Some(range(min, max)))
            .collect(),
        tiers: tiers.map(|(lowest, highest)| TierSpan { lowest, highest }),
    }
}

#[test]
fn how_a_modifier_is_written_tells_what_its_lines_roll_over() {
    let part = PartTier {
        affix: Affix::Prefix,
        tiers: vec![1],
        hides_number: false,
        ranges: Vec::new(),
        currency: None,
    };
    let affix_tier = |affix| AffixTier {
        affix,
        tier: 1,
        hides_number: false,
        ranges: Vec::new(),
        lord: None,
    };
    let with_doubt = |mut modifier: Modifier, doubt| {
        modifier.doubt = Some(doubt);
        modifier
    };
    let split = TierDoubt::Split {
        line: 1,
        whole: part.clone(),
        added: part,
    };
    let either = TierDoubt::EitherAffix {
        prefix: affix_tier(Affix::Prefix),
        suffix: affix_tier(Affix::Suffix),
    };
    let unrevealed = Modifier {
        is_unrevealed: true,
        ..modifier(PREFIX, None, &[])
    };
    let ranged = "Minions have 21(19-22)% increased maximum Life";
    let fixed = "+2 to Level of all Minion Skills";
    let cases = [
        (modifier(PREFIX, Some(7), &[ranged]), Some(Written::Tier)),
        (
            modifier(PREFIX, None, &[ranged]),
            Some(Written::RangesWithoutTier),
        ),
        (
            modifier(PREFIX, None, &[fixed]),
            Some(Written::FixedWithoutTier),
        ),
        // The split line first, whatever the tier.
        (
            with_doubt(modifier(PREFIX, Some(2), &[fixed, fixed]), split.clone()),
            Some(Written::Split { line: 1 }),
        ),
        (
            with_doubt(modifier(PREFIX, None, &[fixed, fixed]), split),
            Some(Written::Split { line: 1 }),
        ),
        // No tier to roll over.
        (modifier(ModifierKind::Implicit, None, &[ranged]), None),
        (modifier(ModifierKind::Rune, None, &[fixed]), None),
        (
            modifier(ModifierKind::Explicit(None), None, &[ranged]),
            None,
        ),
        (with_doubt(modifier(PREFIX, None, &[ranged]), either), None),
        (unrevealed, None),
    ];
    for (modifier, written) in cases {
        assert_eq!(Written::of(&modifier), written, "{modifier:?}");
    }
}

#[test]
fn a_modifier_written_with_its_tier_rolls_over_every_tier_of_its_base() {
    let life = modifier(PREFIX, Some(2), &["+50(45-55) to maximum Life"]);
    let tiers = vec![
        tier(1, &[(60.0, 70.0)]),
        tier(2, &[(45.0, 55.0)]),
        tier(3, &[(30.0, 40.0)]),
    ];
    assert_eq!(
        ModifierRolls::over_every_tier(&life, &in_tables(1, tiers, 1)),
        ModifierRolls {
            lines: vec![(0, rolled(&[(30.0, 70.0)], Some((3, 1))))],
            hides_number: false,
            currency: None,
        }
    );
}

#[test]
fn a_modifier_of_one_tier_only_hides_its_number_unless_copied_with_another_tier() {
    let only = || in_tables(1, vec![tier(1, &[(10.0, 15.0)])], 0);
    let over_its_tier = vec![(0, rolled(&[(10.0, 15.0)], None))];
    // "T1" would read as the best, while it is the only one; "T1–T1" would tell nothing.
    let first = modifier(PREFIX, Some(1), &["12(10-15)% increased Spirit"]);
    assert_eq!(
        ModifierRolls::over_every_tier(&first, &only()),
        ModifierRolls {
            lines: over_its_tier.clone(),
            hides_number: true,
            currency: None,
        }
    );
    // The game knows more tiers than the data: the tier the copy writes is kept.
    let second = modifier(PREFIX, Some(2), &["9(8-9)% increased Spirit"]);
    assert_eq!(
        ModifierRolls::over_every_tier(&second, &only()),
        ModifierRolls {
            lines: over_its_tier,
            hides_number: false,
            currency: None,
        }
    );
}

#[test]
fn a_fixed_value_rolls_over_its_tier_only_in_an_unwritten_tier_of_fixed_values() {
    // `of the Essence` on a bow.
    let attacks = modifier(PREFIX, None, &["+2 to Level of all Attack Skills"]);
    let of_an_essence = |tiers: Vec<Tier>, tier_unwritten: bool| {
        let mut in_tables = in_tables(1, tiers, 0);
        in_tables.tiers.tier_unwritten = tier_unwritten;
        in_tables.tiers.currency = Some(Currency::Essence);
        in_tables
    };
    let fixed = || vec![tier(1, &[(2.0, 2.0)]), tier(2, &[(1.0, 1.0)])];
    assert_eq!(
        ModifierRolls::over_fixed_tier(&attacks, &of_an_essence(fixed(), true)),
        ModifierRolls {
            lines: vec![(0, rolled(&[(2.0, 2.0)], None))],
            hides_number: true,
            currency: Some(Currency::Essence),
        }
    );
    // Not of a modifier whose tier the game writes, nor in a tier that rolls.
    assert_eq!(
        ModifierRolls::over_fixed_tier(&attacks, &of_an_essence(fixed(), false)),
        ModifierRolls::default()
    );
    assert_eq!(
        ModifierRolls::over_fixed_tier(
            &attacks,
            &of_an_essence(vec![tier(1, &[(1.0, 3.0)])], true)
        ),
        ModifierRolls::default()
    );
}

#[test]
fn the_other_lines_of_a_hybrid_split_from_a_summed_line_roll_over_its_tiers() {
    let hybrid = modifier(
        PREFIX,
        None,
        &["258% increased Physical Damage", "+195 to Accuracy Rating"],
    );
    let tiers = vec![
        tier(1, &[(75.0, 79.0), (161.0, 200.0)]),
        tier(2, &[(65.0, 74.0), (131.0, 160.0)]),
    ];
    assert_eq!(
        ModifierRolls::over_hybrid_tiers(&hybrid, &in_tables(2, tiers, 0), 0),
        ModifierRolls {
            lines: vec![(1, rolled(&[(131.0, 200.0)], Some((2, 1))))],
            hides_number: false,
            currency: None,
        }
    );
}

#[test]
fn the_ranges_a_copy_writes_need_no_table_where_the_other_ways_do() {
    let ranged = modifier(
        PREFIX,
        None,
        &["Minions have 21(19-22)% increased maximum Life"],
    );
    assert_eq!(
        Written::RangesWithoutTier.rolls(&ranged, None),
        Some(ModifierRolls {
            lines: vec![(0, rolled(&[(19.0, 22.0)], None))],
            hides_number: true,
            currency: None,
        })
    );
    // Held by the tables, its copied ranges still, whatever their tiers, and the currency of
    // the tables.
    let mut of_an_essence = in_tables(1, vec![tier(1, &[(25.0, 30.0)])], 0);
    of_an_essence.tiers.currency = Some(Currency::Essence);
    assert_eq!(
        Written::RangesWithoutTier.rolls(&ranged, Some(&of_an_essence)),
        Some(ModifierRolls {
            lines: vec![(0, rolled(&[(19.0, 22.0)], None))],
            hides_number: true,
            currency: Some(Currency::Essence),
        })
    );
    let written = modifier(PREFIX, Some(1), &["+50 to maximum Life"]);
    for way in [
        Written::Tier,
        Written::FixedWithoutTier,
        Written::Split { line: 0 },
    ] {
        assert_eq!(way.rolls(&written, None), None, "{way:?}");
    }
}
