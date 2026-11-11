//! The modifiers written again, on tiers written by hand: what the writing does whatever the
//! tables of the game. The lines still find their stat in the embedded stats of the game.

use super::*;
use crate::game_data::modifiers::fixtures::{part, range, trade_lines};

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
        currency: None,
        lord: None,
    };
    let rarity = Run {
        language: Language::English,
        parts: &rarity,
    };
    let read = |tiers: &ModifierTiers| rarity.part_tier(Affix::Prefix, tiers, 0);

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
        currency: None,
        lord: None,
    };
    let damage = Run {
        language: Language::English,
        parts: &damage,
    };
    let read_damage = |tiers: &ModifierTiers| damage.part_tier(Affix::Prefix, tiers, 0);
    assert_eq!(
        read_damage(&tiers(1.0, 6.0)),
        Some(part(Affix::Prefix, &[1, 2], &[(1.0, 4.0), (5.0, 12.0)]))
    );
    assert_eq!(read_damage(&tiers(-1.0, -2.0)), None);
}
