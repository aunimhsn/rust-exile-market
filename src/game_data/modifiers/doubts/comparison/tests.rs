//! The comparison with the best reading, on readings written by hand: the modifiers read the
//! same, the parts of the item, the lines in doubt and the room left to a prefix or suffix.

use super::*;
use crate::game_data::modifiers::doubts::fixtures::placement;
use crate::game_data::modifiers::fixtures::range;
use crate::game_data::modifiers::split_lines::AddedTiers;
use crate::game_data::modifiers::{ModifierTiers, Tier};

fn shown(affix: Affix, tier: u32, modifiers: &[usize]) -> Shown {
    Shown {
        affix,
        tier,
        modifiers: modifiers.to_vec(),
        ranges: vec![range(1.0, 2.0)],
    }
}

#[test]
fn a_line_is_read_the_same_with_one_modifier_for_each_of_the_best_one() {
    use Affix::{Prefix, Suffix};
    let prefix = shown(Prefix, 1, &[0]);
    let suffix = shown(Suffix, 1, &[0]);
    let hybrid = shown(Prefix, 4, &[0, 1]);

    // A modifier that can be a prefix or a suffix: either is the same.
    let either = [vec![prefix.clone(), suffix.clone()]];
    assert!(same(&either, &[&prefix]));
    assert!(same(&either, &[&suffix]));
    assert!(!same(&either, &[&shown(Prefix, 2, &[0])]));
    // A split line: its two modifiers in either order, each once.
    let split = [vec![hybrid.clone()], vec![prefix.clone()]];
    assert!(same(&split, &[&prefix, &hybrid]));
    assert!(same(&split, &[&hybrid, &prefix]));
    assert!(!same(&split, &[&hybrid, &hybrid]));
    assert!(!same(&split, &[&hybrid]));
    // A line the best reading doesn't read.
    assert!(same(&[], &[]));
    assert!(!same(&[], &[&prefix]));
}

#[test]
fn a_part_of_a_split_in_two_neighbouring_tiers_is_read_the_same_in_either() {
    use Affix::Prefix;
    let tier = |number| Tier {
        number,
        level: 1,
        ranges: vec![range(1.0, 2.0)],
    };
    let tiers = |found, neighbour| ModifierTiers {
        tiers: vec![tier(1), tier(2)],
        found,
        neighbour,
        counts: vec![1],
        tier_unwritten: false,
        currency: None,
        lord: None,
    };
    // A line split into two pure prefixes, the first a T2 or a T1, the other a T1: no base
    // rolls two pure prefixes of one stat today, but the comparison mustn't depend on the
    // order the search reads them in.
    let split = Candidate {
        modifiers: 0..1,
        fractured: false,
        affixes: vec![(Prefix, tiers(1, Some(0)))],
        added: Some(AddedTiers {
            line: 0,
            affix: Prefix,
            tiers: tiers(0, None),
        }),
    };

    let best = best_shown(&[&split], 1);

    let (first, second) = (shown(Prefix, 1, &[0]), shown(Prefix, 2, &[0]));
    let line = best.first().expect("the line");
    assert!(same(line, &[&second, &first]));
    assert!(same(line, &[&first, &second]));
    assert!(same(line, &[&first, &first]));
    assert!(!same(line, &[&second, &second]));
    assert!(!same(line, &[&first]));
    assert!(!same(line, &[&first, &first, &first]));
}

#[test]
fn the_parts_of_the_item_are_the_lines_a_modifier_joins() {
    let mut parts = Parts::new(5);
    parts.join([0, 1]);
    parts.join([3, 1]);
    parts.join([2]);
    parts.join([4, 4]);
    parts.join([]);

    let roots: Vec<usize> = (0..5).map(|line| parts.root(line)).collect();
    assert_eq!(roots[0], roots[1]);
    assert_eq!(roots[1], roots[3]);
    assert_ne!(roots[0], roots[2]);
    assert_ne!(roots[0], roots[4]);
    assert_ne!(roots[2], roots[4]);
    // A line out of the item is its own part.
    assert_eq!(parts.root(9), 9);
}

#[test]
fn lines_are_in_doubt_where_another_reading_is_as_likely_as_the_best_one() {
    // The best reading splits a line of the part, or leaves without tier a line the other
    // reading reads: in doubt, whatever the other reading makes.
    assert!(PartRead::BestSplits.leaves_in_doubt());
    assert!(PartRead::ReadsMore.leaves_in_doubt());
    // A reading adding up modifiers on a line: with no more of them than the best one.
    let sums = |made, best_made| PartRead::Sums { made, best_made };
    assert!(sums(2, 3).leaves_in_doubt());
    assert!(sums(3, 3).leaves_in_doubt());
    assert!(!sums(4, 3).leaves_in_doubt());
    // A hybrid in place of two pure modifiers, or the reverse, no line adding up: nothing
    // tells which rolled.
    assert!(PartRead::JoinsOtherwise.leaves_in_doubt());
    // A reading that only takes another tier of the modifiers of the best one.
    assert!(!PartRead::SameLines.leaves_in_doubt());
}

#[test]
fn a_reading_leaves_the_same_room_to_a_prefix_or_suffix_with_as_many_of_each_elsewhere() {
    use Affix::{Prefix, Suffix};
    let on = |line: usize, affix| placement("Kind", affix, &[line], &[(line, range(1.0, 100.0))]);
    // Three lines: a rarity, a life and an accuracy.
    let best = BestShown {
        lines: Vec::new(),
        // The rarity a prefix or a suffix, the life a prefix, the accuracy a suffix.
        candidates: vec![
            BestCandidate {
                modifiers: 0..1,
                sets: vec![AffixSet::Both],
            },
            BestCandidate {
                modifiers: 1..2,
                sets: vec![AffixSet::Only(Prefix)],
            },
            BestCandidate {
                modifiers: 2..3,
                sets: vec![AffixSet::Only(Suffix)],
            },
        ],
    };
    let same =
        |placements: &[Placement<'_>]| best.same_room(&placements.iter().collect::<Vec<_>>());

    // The rarity either affix, the others as the best reading.
    assert!(same(&[on(0, Prefix), on(1, Prefix), on(2, Suffix)]));
    assert!(same(&[on(0, Suffix), on(1, Prefix), on(2, Suffix)]));
    // The same totals, a prefix of the others moved to a suffix.
    assert!(!same(&[on(0, Prefix), on(1, Suffix), on(2, Suffix)]));
    // A prefix less on the others.
    assert!(!same(&[on(0, Prefix), on(2, Suffix)]));
    // A hybrid of the rarity and the life takes the prefix of the life.
    let hybrid = placement(
        "Hybrid",
        Prefix,
        &[0, 1],
        &[(0, range(1.0, 10.0)), (1, range(1.0, 10.0))],
    );
    assert!(same(&[hybrid, on(2, Suffix)]));
    // A prefix and a suffix added up on the rarity line.
    assert!(same(&[
        on(0, Prefix),
        on(0, Suffix),
        on(1, Prefix),
        on(2, Suffix)
    ]));
}
