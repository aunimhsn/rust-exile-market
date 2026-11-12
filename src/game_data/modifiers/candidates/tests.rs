//! The candidates of the reading of an item copied from the in-game trade, on tables written by
//! hand: the runs of lines looked up, the affixes of a candidate and the lines it splits,
//! whatever the patch of the game.

use super::*;
use crate::game_data::bases::Domain;
use crate::game_data::modifiers::fixtures::{
    AffixTiers, ENERGY_SHIELD, LIFE, MANA, RARITY, copied, equipment, marked, table,
};
use crate::game_data::modifiers::tables::TableLord;

/// What a candidate reads: the modifiers of its run, the tier number of each affix it can be,
/// and for a split line the index of its modifier among those of the item, with the affix and
/// the tier number of the modifier added.
type Read = (
    std::ops::Range<usize>,
    AffixTiers,
    Option<(usize, Affix, u32)>,
);

/// The candidates of the modifiers `copied` of an item of `item_level`, a helmet, among
/// `tables`.
fn candidates_in(
    tables: &[Table],
    item_level: Option<u32>,
    copied: &[Option<Copied<'_>>],
) -> Vec<Candidate> {
    let tables = ItemTables {
        all: tables,
        base: &equipment(&["helmet"]),
        item_level,
    };
    candidates(&tables, copied)
}

/// What each of `candidates` reads, to compare whole.
fn read(candidates: &[Candidate]) -> Vec<Read> {
    let number = |tiers: &ModifierTiers| tiers.tier().expect("its tier").number;
    candidates
        .iter()
        .map(|candidate| {
            let affixes = candidate
                .affixes
                .iter()
                .map(|(affix, tiers)| (*affix, number(tiers)))
                .collect();
            let added = candidate
                .added
                .as_ref()
                .map(|added| (added.line, added.affix, number(&added.tiers)));
            (candidate.modifiers.clone(), affixes, added)
        })
        .collect()
}

/// What the candidates of the modifiers `copied` of an item of any level read among `tables`.
fn read_in(tables: &[Table], copied: &[Option<Copied<'_>>]) -> Vec<Read> {
    read(&candidates_in(tables, None, copied))
}

/// A modifier of one line of `template` with `value`.
fn one_line(template: &'static str, value: f64) -> Copied<'static> {
    copied(&[(template, value)])
}

/// The energy shield in its one tier, the life in its second and their hybrid in its third:
/// the tier number of a candidate tells its table.
fn shield_life_and_hybrid() -> Vec<Table> {
    vec![
        table(
            "EnergyShield",
            "prefix",
            &[ENERGY_SHIELD],
            &[(1, &[(10, 19)])],
        ),
        table(
            "Life",
            "prefix",
            &[LIFE],
            &[(1, &[(20, 29)]), (30, &[(30, 39)])],
        ),
        table(
            "Hybrid",
            "prefix",
            &[ENERGY_SHIELD, LIFE],
            &[
                (1, &[(10, 19), (20, 29)]),
                (30, &[(20, 29), (30, 39)]),
                (60, &[(30, 39), (40, 49)]),
            ],
        ),
    ]
}

/// A modifier of three lines: among tables that have it, a run looks at three modifiers.
fn triple() -> Table {
    table(
        "Triple",
        "prefix",
        &[ENERGY_SHIELD, LIFE, MANA],
        &[(1, &[(10, 19), (20, 29), (30, 39)])],
    )
}

/// The rarity as a prefix of one tier, `10–19`, and as a suffix of two, `15–24` then `6–14`.
fn rarity_as_both_affixes() -> Vec<Table> {
    vec![
        table("RarityPrefix", "prefix", &[RARITY], &[(1, &[(10, 19)])]),
        table(
            "RaritySuffix",
            "suffix",
            &[RARITY],
            &[(1, &[(6, 14)]), (40, &[(15, 24)])],
        ),
    ]
}

#[test]
fn each_line_and_each_run_of_consecutive_lines_a_modifier_holds_is_a_candidate() {
    use Affix::{Prefix, Suffix};
    let mut tables = shield_life_and_hybrid();
    tables.push(table("Mana", "suffix", &[MANA], &[(1, &[(30, 39)])]));
    let copied = [
        one_line(ENERGY_SHIELD, 15.0),
        one_line(LIFE, 25.0),
        one_line(MANA, 35.0),
    ]
    .map(Some);

    // From each line, the line alone then the run it starts: the life and the mana, which no
    // modifier has both, make none.
    assert_eq!(
        read_in(&tables, &copied),
        [
            (0..1, vec![(Prefix, 1)], None),
            (0..2, vec![(Prefix, 3)], None),
            (1..2, vec![(Prefix, 2)], None),
            (2..3, vec![(Suffix, 1)], None),
        ]
    );
}

#[test]
fn a_run_has_as_many_lines_as_the_longest_hybrid_of_the_tables() {
    use Affix::Prefix;
    // A modifier of one line, and one of three.
    let tables = [
        table("Life", "prefix", &[LIFE], &[(1, &[(20, 29)])]),
        triple(),
    ];
    let copied = [
        one_line(ENERGY_SHIELD, 15.0),
        one_line(LIFE, 25.0),
        one_line(MANA, 35.0),
    ]
    .map(Some);

    assert_eq!(
        read_in(&tables, &copied),
        [
            (0..3, vec![(Prefix, 1)], None),
            (1..2, vec![(Prefix, 1)], None),
        ]
    );
}

#[test]
fn a_run_is_of_lines_of_one_origin() {
    use ModifierOrigin::{Crafted, Desecrated, Fractured};
    // A run looks at three modifiers, past the one of another origin.
    let mut tables = shield_life_and_hybrid();
    tables.push(triple());
    // The modifiers of each candidate, and whether they are fractured.
    let runs = |copied: &[Option<Copied<'_>>]| -> Vec<(std::ops::Range<usize>, bool)> {
        candidates_in(&tables, None, copied)
            .iter()
            .map(|candidate| (candidate.modifiers.clone(), candidate.fractured))
            .collect()
    };
    let shield = || Some(one_line(ENERGY_SHIELD, 15.0));
    let life = || Some(one_line(LIFE, 25.0));
    let fractured_shield = || Some(marked(Fractured, ENERGY_SHIELD, 15.0));
    let fractured_life = || Some(marked(Fractured, LIFE, 25.0));

    assert_eq!(
        runs(&[shield(), life()]),
        [(0..1, false), (0..2, false), (1..2, false)]
    );
    assert_eq!(
        runs(&[fractured_shield(), fractured_life()]),
        [(0..1, true), (0..2, true), (1..2, true)]
    );
    // A fractured line and one that isn't are never the lines of one modifier.
    assert_eq!(
        runs(&[fractured_shield(), life()]),
        [(0..1, true), (1..2, false)]
    );
    assert_eq!(
        runs(&[shield(), fractured_life()]),
        [(0..1, false), (1..2, true)]
    );
    // Nor are two fractured lines around one that isn't: they aren't consecutive.
    assert_eq!(
        runs(&[fractured_shield(), life(), fractured_life()]),
        [(0..1, true), (1..2, false), (2..3, true)]
    );
    // Nor are two marked lines of two origins, neither fractured.
    assert_eq!(
        runs(&[
            Some(marked(Crafted, ENERGY_SHIELD, 15.0)),
            Some(marked(Desecrated, LIFE, 25.0)),
        ]),
        [(0..1, false), (1..2, false)]
    );
}

#[test]
fn the_modifiers_of_a_candidate_are_fractured_when_the_game_marks_its_lines_so() {
    use ModifierOrigin::{Crafted, Fractured};
    let tables = rarity_as_both_affixes();
    // Whether the modifiers of each candidate of the one line `copied` are fractured.
    let fractured = |copied: Copied<'_>| -> Vec<bool> {
        candidates_in(&tables, None, &[Some(copied)])
            .iter()
            .map(|candidate| candidate.fractured)
            .collect()
    };

    // 22 is a suffix, or adds up a prefix and a suffix: two candidates.
    assert_eq!(fractured(marked(Fractured, RARITY, 22.0)), [true, true]);
    assert_eq!(fractured(marked(Crafted, RARITY, 22.0)), [false, false]);
    assert_eq!(fractured(one_line(RARITY, 22.0)), [false, false]);
}

#[test]
fn a_run_is_looked_up_among_the_tables_its_origin_allows() {
    use Affix::Prefix;
    use ModifierOrigin::{Crafted, Desecrated};
    // The life of a lord of the Abyss, which only a desecrated line may be.
    let tables = [Table {
        domain: Domain::Desecrated,
        lord: Some(TableLord::Kurgal),
        ..table("LordLife", "prefix", &[LIFE], &[(1, &[(20, 29)])])
    }];

    assert_eq!(
        read_in(&tables, &[Some(marked(Desecrated, LIFE, 25.0))]),
        [(0..1, vec![(Prefix, 1)], None)]
    );
    assert_eq!(read_in(&tables, &[Some(marked(Crafted, LIFE, 25.0))]), []);
    assert_eq!(read_in(&tables, &[Some(one_line(LIFE, 25.0))]), []);
}

#[test]
fn a_modifier_that_is_not_read_starts_no_run_and_ends_one() {
    use Affix::Prefix;
    // A run looks at three modifiers, past the one that isn't read.
    let mut tables = shield_life_and_hybrid();
    tables.push(triple());
    let shield = || Some(one_line(ENERGY_SHIELD, 15.0));
    let life = || Some(one_line(LIFE, 25.0));

    // Between the lines of the hybrid: they aren't consecutive.
    assert_eq!(
        read_in(&tables, &[shield(), None, life()]),
        [
            (0..1, vec![(Prefix, 1)], None),
            (2..3, vec![(Prefix, 2)], None),
        ]
    );
    // Before them: a candidate names its modifiers among all those of the item.
    assert_eq!(
        read_in(&tables, &[None, shield(), life()]),
        [
            (1..2, vec![(Prefix, 1)], None),
            (1..3, vec![(Prefix, 3)], None),
            (2..3, vec![(Prefix, 2)], None),
        ]
    );
}

#[test]
fn a_candidate_has_each_affix_whose_tables_hold_its_lines_the_prefix_first() {
    use Affix::{Prefix, Suffix};
    let tables = rarity_as_both_affixes();
    let read = |rarity| read_in(&tables, &[Some(one_line(RARITY, rarity))]);

    // In the prefix and in the second tier of the suffix.
    assert_eq!(read(12.0), [(0..1, vec![(Prefix, 1), (Suffix, 2)], None)]);
    assert_eq!(read(8.0), [(0..1, vec![(Suffix, 2)], None)]);
    // In no tier of either affix, nor in the sum of both: no candidate.
    assert_eq!(read(50.0), []);
}

#[test]
fn the_tier_of_a_candidate_is_the_worst_that_holds_its_values_among_those_the_item_level_allows() {
    use Affix::Prefix;
    let tables = [table(
        "Life",
        "prefix",
        &[LIFE],
        &[(1, &[(1, 10)]), (50, &[(5, 15)])],
    )];
    let read = |item_level, life| {
        read(&candidates_in(
            &tables,
            item_level,
            &[Some(one_line(LIFE, life))],
        ))
    };

    // In both tiers: the worst.
    assert_eq!(read(Some(60), 7.0), [(0..1, vec![(Prefix, 2)], None)]);
    assert_eq!(read(Some(60), 12.0), [(0..1, vec![(Prefix, 1)], None)]);
    // In the first tier only, which an item of level 10 can't roll.
    assert_eq!(read(Some(10), 12.0), []);
    assert_eq!(read(None, 12.0), [(0..1, vec![(Prefix, 1)], None)]);
}

#[test]
fn each_way_a_line_adds_up_two_modifiers_is_a_candidate_after_the_one_that_reads_it_whole() {
    use Affix::{Prefix, Suffix};
    let tables = rarity_as_both_affixes();

    // 22: the first tier of the suffix, or the prefix with the second tier of the suffix.
    assert_eq!(
        read_in(&tables, &[Some(one_line(RARITY, 22.0))]),
        [
            (0..1, vec![(Suffix, 1)], None),
            (0..1, vec![(Prefix, 1)], Some((0, Suffix, 2))),
        ]
    );
}

#[test]
fn a_line_of_a_run_that_adds_up_two_modifiers_is_named_among_the_modifiers_of_the_item() {
    use Affix::Prefix;
    // The hybrid, and the life alone, in two tiers.
    let tables = [
        table(
            "Hybrid",
            "prefix",
            &[ENERGY_SHIELD, LIFE],
            &[(1, &[(10, 19), (20, 29)])],
        ),
        table(
            "Life",
            "prefix",
            &[LIFE],
            &[(1, &[(30, 39)]), (40, &[(60, 69)])],
        ),
    ];
    // A line no table holds, then the hybrid, its life summed with the second tier of the life
    // alone: 60 is 20–29 and 30–39.
    let copied = [
        one_line(RARITY, 12.0),
        one_line(ENERGY_SHIELD, 15.0),
        one_line(LIFE, 60.0),
    ]
    .map(Some);

    assert_eq!(
        read_in(&tables, &copied),
        [
            (1..3, vec![(Prefix, 1)], Some((2, Prefix, 2))),
            (2..3, vec![(Prefix, 1)], None),
        ]
    );
}

#[test]
fn only_the_line_of_a_modifier_of_one_line_is_split() {
    use Affix::Prefix;
    let tables = [
        table(
            "Hybrid",
            "prefix",
            &[ENERGY_SHIELD, LIFE],
            &[(1, &[(10, 19), (20, 29)])],
        ),
        table("Life", "prefix", &[LIFE], &[(1, &[(30, 39)])]),
    ];
    let lines = [(ENERGY_SHIELD, 15.0), (LIFE, 60.0)];
    let as_two_modifiers = [
        Some(one_line(ENERGY_SHIELD, 15.0)),
        Some(one_line(LIFE, 60.0)),
    ];
    let as_one_modifier = [Some(copied(&lines))];

    // The life, 60, is 20–29 of the hybrid and 30–39 of the life alone.
    assert_eq!(
        read_in(&tables, &as_two_modifiers),
        [(0..2, vec![(Prefix, 1)], Some((1, Prefix, 1)))]
    );
    // The game writes the line that adds up two modifiers alone: not one of these lines.
    assert_eq!(read_in(&tables, &as_one_modifier), []);

    // In one run, a modifier of two lines then one of one line, the mana: 75 is 30–39 of the
    // modifier of three lines and 40–49 of the mana alone.
    let tables = [
        triple(),
        table("Mana", "prefix", &[MANA], &[(1, &[(40, 49)])]),
    ];
    let two_lines_then_one = [
        Some(copied(&[(ENERGY_SHIELD, 15.0), (LIFE, 25.0)])),
        Some(one_line(MANA, 75.0)),
    ];
    assert_eq!(
        read_in(&tables, &two_lines_then_one),
        [(0..2, vec![(Prefix, 1)], Some((1, Prefix, 1)))]
    );
}

#[test]
fn a_candidate_tells_the_affixes_each_of_its_modifiers_can_be() {
    use Affix::{Prefix, Suffix};
    let tables = rarity_as_both_affixes();
    // The affixes of the modifier of every line, then those of each modifier.
    let sets = |rarity| -> Vec<(AffixSet, Vec<AffixSet>)> {
        candidates_in(&tables, None, &[Some(one_line(RARITY, rarity))])
            .iter()
            .map(|candidate| (candidate.set(), candidate.sets().collect()))
            .collect()
    };

    // A prefix or a suffix.
    assert_eq!(sets(12.0), [(AffixSet::Both, vec![AffixSet::Both])]);
    // A suffix; or a prefix, with a suffix added.
    assert_eq!(
        sets(22.0),
        [
            (AffixSet::Only(Suffix), vec![AffixSet::Only(Suffix)]),
            (
                AffixSet::Only(Prefix),
                vec![AffixSet::Only(Prefix), AffixSet::Only(Suffix)]
            ),
        ]
    );
}

#[test]
fn a_candidate_makes_one_modifier_or_two_when_its_line_adds_up_two() {
    let tables = rarity_as_both_affixes();
    let counts: Vec<usize> = candidates_in(&tables, None, &[Some(one_line(RARITY, 22.0))])
        .iter()
        .map(Candidate::count)
        .collect();

    // 22 is a suffix, or a prefix and a suffix.
    assert_eq!(counts, [1, 2]);
}
