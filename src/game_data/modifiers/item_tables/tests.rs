//! The modifier tables as an item sees them, on tables written by hand: the tiers of a
//! modifier on a base and the modifiers that have the lines of a copied one, whatever the
//! patch of the game.

use super::*;
use crate::game_data::bases::Domain;
use crate::game_data::modifiers::copied::{CopiedLine, CopiedTier};
use crate::game_data::modifiers::fixtures::{ENERGY_SHIELD, LIFE, MANA, equipment, range, table};
use crate::game_data::modifiers::tables::{TableLord, TableSource};
use crate::item::{Currency, ModifierOrigin};

/// The life as a prefix on the helmets, with `tiers`: the level and the range of each.
fn life(tiers: &[(u32, &[(u32, u32)])]) -> Table {
    table("Life", "prefix", &[LIFE], tiers)
}

/// `table` on the bases of `tag`, in place of the helmets.
fn on(tag: &str, table: Table) -> Table {
    Table {
        spawn: vec![(tag.to_owned(), 1)],
        ..table
    }
}

/// A copied modifier of `lines`, the template and the value of each, without header: no
/// range, no tier, no origin.
fn copied(lines: &[(&'static str, f64)]) -> Copied<'static> {
    Copied {
        lines: lines
            .iter()
            .map(|&(template, value)| CopiedLine {
                templates: vec![template],
                values: vec![value],
                written: None,
            })
            .collect(),
        ranges: None,
        tier: CopiedTier::Unknown,
        origin: None,
    }
}

/// The tier `number` of `level` over `ranges`.
fn tier(number: u32, level: u32, ranges: &[(f64, f64)]) -> Tier {
    Tier {
        number,
        level,
        ranges: ranges.iter().map(|&(min, max)| range(min, max)).collect(),
    }
}

#[test]
fn the_tiers_of_a_modifier_on_a_base_are_those_of_its_tables_that_roll_on_it_the_best_first() {
    let tables = [
        life(&[(1, &[(1, 10)]), (30, &[(11, 20)])]),
        // The same modifier on the rings, then on the armours, which a helmet is too.
        on("ring", life(&[(1, &[(1, 5)]), (60, &[(21, 30)])])),
        on("armour", life(&[(45, &[(15, 25)])])),
        // Another modifier with its line.
        table("Vigour", "prefix", &[LIFE], &[(80, &[(31, 40)])]),
    ];
    let tiers = |table: usize, tags: &[&str], item_level| {
        let tables = ItemTables {
            all: &tables,
            base: &equipment(tags),
            item_level,
        };
        tables.tiers_of(&tables.all[table], &[0], &copied(&[(LIFE, 12.0)]))
    };
    let on_a_helmet = Some(vec![
        tier(1, 45, &[(15.0, 25.0)]),
        tier(2, 30, &[(11.0, 20.0)]),
        tier(3, 1, &[(1.0, 10.0)]),
    ]);

    // From any table of the modifier, one that doesn't roll on the base too.
    for table in 0..3 {
        assert_eq!(tiers(table, &["helmet", "armour"], None), on_a_helmet);
    }
    // Those above the level of the item too.
    assert_eq!(tiers(0, &["helmet", "armour"], Some(1)), on_a_helmet);
    assert_eq!(
        tiers(0, &["ring"], None),
        Some(vec![
            tier(1, 60, &[(21.0, 30.0)]),
            tier(2, 1, &[(1.0, 5.0)])
        ])
    );
    // A base none of its tables rolls on: no tier.
    assert_eq!(tiers(0, &["amulet"], None), Some(Vec::new()));
}

#[test]
fn tiers_of_one_level_are_numbered_by_their_ranges_the_higher_the_better() {
    let helmet = equipment(&["helmet", "armour"]);
    let ranges = |tables: &[Table], copied: &Copied<'_>, order: &[usize]| {
        let tables = ItemTables {
            all: tables,
            base: &helmet,
            item_level: None,
        };
        let tiers = tables.tiers_of(&tables.all[0], order, copied)?;
        Some(
            tiers
                .into_iter()
                .map(|tier| (tier.number, tier.ranges))
                .collect::<Vec<_>>(),
        )
    };
    let life_copied = copied(&[(LIFE, 30.0)]);
    let numbered = Some(vec![
        (1, vec![range(39.0, 47.0)]),
        (2, vec![range(26.0, 31.0)]),
    ]);

    // Two tables of the modifier with a tier of the same level, in either order of the file.
    let lower = || life(&[(1, &[(26, 31)])]);
    let higher = || on("armour", life(&[(1, &[(39, 47)])]));
    assert_eq!(ranges(&[lower(), higher()], &life_copied, &[0]), numbered);
    assert_eq!(ranges(&[higher(), lower()], &life_copied, &[0]), numbered);

    // Two tiers that differ by their second line only.
    let hybrid = table(
        "Hybrid",
        "prefix",
        &[ENERGY_SHIELD, LIFE],
        &[(1, &[(10, 19), (5, 9)]), (1, &[(10, 19), (1, 4)])],
    );
    let hybrid_copied = copied(&[(ENERGY_SHIELD, 15.0), (LIFE, 3.0)]);
    assert_eq!(
        ranges(&[hybrid], &hybrid_copied, &[0, 1]),
        Some(vec![
            (1, vec![range(10.0, 19.0), range(5.0, 9.0)]),
            (2, vec![range(10.0, 19.0), range(1.0, 4.0)]),
        ])
    );
}

#[test]
fn the_ranges_of_a_tier_follow_the_copied_lines_each_from_its_lower_bound() {
    // The range of the life written from its higher bound, as the one of a stat that rolls
    // negative values.
    let hybrid = table(
        "Hybrid",
        "prefix",
        &[ENERGY_SHIELD, LIFE],
        &[(1, &[(10, 19), (29, 20)])],
    );
    let helmet = equipment(&["helmet"]);
    let tables = ItemTables {
        all: std::slice::from_ref(&hybrid),
        base: &helmet,
        item_level: None,
    };
    // The life copied first: the second line of the table.
    let copied = copied(&[(LIFE, 25.0), (ENERGY_SHIELD, 15.0)]);

    assert_eq!(
        tables.tiers_of(&hybrid, &[1, 0], &copied),
        Some(vec![tier(1, 1, &[(20.0, 29.0), (10.0, 19.0)])])
    );
}

#[test]
fn a_tier_without_a_range_for_each_value_of_the_copied_lines_gives_no_tiers() {
    let life = life(&[(1, &[(1, 10)]), (30, &[(11, 20)])]);
    let helmet = equipment(&["helmet"]);
    let tables = ItemTables {
        all: std::slice::from_ref(&life),
        base: &helmet,
        item_level: None,
    };
    let tiers = |order: &[usize], copied: &Copied<'_>| tables.tiers_of(&life, order, copied);

    assert!(tiers(&[0], &copied(&[(LIFE, 5.0)])).is_some());
    // A copied line of two values, the tiers with one range for it.
    let two_values = Copied {
        lines: vec![CopiedLine {
            templates: vec![LIFE],
            values: vec![3.0, 9.0],
            written: None,
        }],
        ..copied(&[])
    };
    assert_eq!(tiers(&[0], &two_values), None);
    // A second copied line, the tiers without ranges for it.
    let two_lines = copied(&[(LIFE, 5.0), (ENERGY_SHIELD, 15.0)]);
    assert_eq!(tiers(&[0, 1], &two_lines), None);
}

#[test]
fn an_item_can_roll_the_tiers_of_its_level_at_most_and_any_tier_without_its_level() {
    let helmet = equipment(&["helmet"]);
    let allows = |item_level, tier_level| {
        let tables = ItemTables {
            all: &[],
            base: &helmet,
            item_level,
        };
        tables.allows(&tier(1, tier_level, &[]))
    };

    assert!(allows(Some(50), 49));
    assert!(allows(Some(50), 50));
    assert!(!allows(Some(50), 51));
    // A copy that doesn't write the level of its item.
    assert!(allows(None, 100));
}

/// What a lookup gave of each modifier: the index of its first table, the number and the
/// level of each of its tiers, and the copied values.
type HeldModifiers = Vec<(usize, Vec<(u32, u32)>, Vec<f64>)>;

/// What `held` gave, to compare whole.
fn modifiers<'a>(held: impl Iterator<Item = Held<'a>>) -> HeldModifiers {
    held.map(|held| {
        let tiers = held
            .tiers
            .iter()
            .map(|tier| (tier.number, tier.level))
            .collect();
        (held.index, tiers, held.values)
    })
    .collect()
}

/// The tables of the lookups: the life of two tiers on the helmets, then a table each check
/// of a lookup rules out, and those another origin or another affix give.
fn lookup_tables() -> Vec<Table> {
    let life_tiers: &[(u32, &[(u32, u32)])] = &[(1, &[(1, 30)]), (40, &[(31, 60)])];
    let life_as = |kind: &str, affix: &str| table(kind, affix, &[LIFE], life_tiers);
    let two_ranges = TableTier {
        level: 1,
        ranges: vec![vec![[1.0, 30.0], [31.0, 60.0]]],
    };
    vec![
        life(life_tiers),
        // Other lines.
        table("Mana", "prefix", &[MANA], life_tiers),
        // The life again, on the rings: not the first table of its modifier.
        on("ring", life(&[(60, &[(61, 90)])])),
        // A tier with two ranges for the one value of the line: its tiers can't be read.
        Table {
            tiers: vec![two_ranges],
            ..life_as("TwoValues", "prefix")
        },
        life_as("LifeSuffix", "suffix"),
        // An alloy's, which only a marked line may be.
        Table {
            source: Some(TableSource::Alloy),
            ..life_as("AlloyLife", "prefix")
        },
        // A lord's, which only a desecrated line may be.
        Table {
            domain: Domain::Desecrated,
            lord: Some(TableLord::Kurgal),
            ..life_as("LordLife", "prefix")
        },
        // A modifier that rolls on the rings only.
        on("ring", life_as("RingLife", "prefix")),
    ]
}

#[test]
fn the_modifiers_that_have_the_lines_of_a_copied_one_are_given_once_in_the_order_of_the_file() {
    let tables = lookup_tables();
    let helmet = equipment(&["helmet"]);
    // An item of level 1: the tiers above its level are given too.
    let tables = ItemTables {
        all: &tables,
        base: &helmet,
        item_level: Some(1),
    };
    let holding = |origin| {
        let copied = Copied {
            origin,
            ..copied(&[(LIFE, 25.0)])
        };
        modifiers(tables.holding(&copied))
    };
    let two_tiers = vec![(1, 40), (2, 1)];
    let life = (0, two_tiers.clone(), vec![25.0]);
    let suffix = (4, two_tiers.clone(), vec![25.0]);
    let alloy = (5, two_tiers.clone(), vec![25.0]);
    let lord = (6, two_tiers, vec![25.0]);
    // A modifier that doesn't roll on the base is given too, without tier.
    let on_the_rings = (7, Vec::new(), vec![25.0]);

    assert_eq!(
        holding(None),
        [life.clone(), suffix.clone(), on_the_rings.clone()]
    );
    assert_eq!(
        holding(Some(ModifierOrigin::Crafted)),
        [life.clone(), suffix.clone(), alloy, on_the_rings.clone()]
    );
    assert_eq!(
        holding(Some(ModifierOrigin::Desecrated)),
        [life, suffix, lord, on_the_rings]
    );
}

#[test]
fn the_modifiers_of_an_affix_are_those_of_its_tables_alone() {
    let tables = lookup_tables();
    let helmet = equipment(&["helmet"]);
    let tables = ItemTables {
        all: &tables,
        base: &helmet,
        item_level: None,
    };
    let copied = copied(&[(LIFE, 25.0)]);
    let indexes = |affix| -> Vec<usize> {
        tables
            .holding_as(&copied, affix)
            .map(|held| held.index)
            .collect()
    };

    assert_eq!(indexes(TableAffix::Prefix), [0, 7]);
    assert_eq!(indexes(TableAffix::Suffix), [4]);
}

#[test]
fn the_lines_of_a_hybrid_copied_in_another_order_give_its_values_and_ranges_in_that_order() {
    let hybrid = table(
        "Hybrid",
        "prefix",
        &[ENERGY_SHIELD, LIFE],
        &[(1, &[(10, 19), (20, 29)])],
    );
    let helmet = equipment(&["helmet"]);
    let tables = ItemTables {
        all: std::slice::from_ref(&hybrid),
        base: &helmet,
        item_level: None,
    };
    let copied = copied(&[(LIFE, 25.0), (ENERGY_SHIELD, 15.0)]);

    let held: Vec<(Vec<Tier>, Vec<f64>)> = tables
        .holding(&copied)
        .map(|held| (held.tiers, held.values))
        .collect();

    assert_eq!(
        held,
        [(
            vec![tier(1, 1, &[(20.0, 29.0), (10.0, 19.0)])],
            vec![25.0, 15.0]
        )]
    );
}

#[test]
fn a_line_without_number_copied_in_another_order_is_divided_as_its_own_line_shows_it() {
    const RAGE: &str = "Inherent Rage loss starts 1 second later";
    // Shown in seconds, stored in milliseconds.
    let mut timed = table(
        "Timed",
        "prefix",
        &[RAGE, LIFE],
        &[(1, &[(1, 3), (20, 29)])],
    );
    timed.lines[0].divisor = 1000.0;
    let helmet = equipment(&["helmet"]);
    let tables = ItemTables {
        all: std::slice::from_ref(&timed),
        base: &helmet,
        item_level: None,
    };
    // The life copied first, then the line whose string gives 2000 milliseconds.
    let mut copied = copied(&[(LIFE, 25.0), (RAGE, 0.0)]);
    copied.lines[1].values = Vec::new();
    copied.lines[1].written = Some(2000.0);

    let values: Vec<Vec<f64>> = tables.holding(&copied).map(|held| held.values).collect();

    assert_eq!(values, [[25.0, 2.0]]);
}

#[test]
fn the_tiers_of_a_modifier_held_tell_what_its_table_tells_of_the_copied_one() {
    let life_as = |kind: &str| table(kind, "prefix", &[LIFE], &[(1, &[(1, 30)])]);
    let tables = [
        Table {
            source: Some(TableSource::Essence),
            ..life_as("EssenceLife")
        },
        Table {
            domain: Domain::Desecrated,
            lord: Some(TableLord::Amanamu),
            ..life_as("LordLife")
        },
    ];
    let helmet = equipment(&["helmet"]);
    let tables = ItemTables {
        all: &tables,
        base: &helmet,
        item_level: None,
    };
    let desecrated = Copied {
        origin: Some(ModifierOrigin::Desecrated),
        ..copied(&[(LIFE, 25.0)])
    };
    // Found in the tier of the index given, whatever its tiers.
    let tiers_of = |copied: &Copied<'_>, found| -> Vec<ModifierTiers> {
        tables
            .holding(copied)
            .map(|held| held.into_tiers(found, copied))
            .collect()
    };
    let one_tier = vec![tier(1, 1, &[(1.0, 30.0)])];

    assert_eq!(
        tiers_of(&copied(&[(LIFE, 25.0)]), 0),
        [ModifierTiers {
            tiers: one_tier.clone(),
            found: 0,
            neighbour: None,
            counts: vec![1],
            tier_unwritten: true,
            currency: Some(Currency::Essence),
            lord: None,
        }]
    );
    assert_eq!(
        tiers_of(&desecrated, 3),
        [ModifierTiers {
            tiers: one_tier,
            found: 3,
            neighbour: None,
            counts: vec![1],
            tier_unwritten: false,
            currency: None,
            lord: Some(Lord::Amanamu),
        }]
    );
}
