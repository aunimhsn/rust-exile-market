//! The tiers of the tables of the game numbered on their bases: the numbers of a patch, to
//! record again when the game data are generated again.

use crate::game_data::modifiers::fixtures::{modifier, range, tiers};
use crate::item::{Affix, Modifier, ModifierOrigin};

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
fn the_radius_of_a_time_lost_jewel_upgraded_to_large_is_the_better_of_two_tiers() {
    // `Upgrades Radius to Medium` or `to Large`, of one type, both of level 1: the owner's
    // Sapphire upgraded to Large writes it `(Palier : 1)`.
    let tier = |line: &str| {
        let upgrade = modifier(Some(Affix::Prefix), &[line]);
        tiers("Time-Lost Ruby", &upgrade).map(|tiers| {
            let number = tiers.tier().map(|tier| tier.number);
            (tiers.tiers.len(), number)
        })
    };

    assert_eq!(tier("Upgrades Radius to Large"), Some((2, Some(1))));
    assert_eq!(tier("Upgrades Radius to Medium"), Some((2, Some(2))));
}

#[test]
fn the_tiers_of_two_tables_of_one_level_are_numbered_by_their_ranges() {
    // The cast speed and cold damage of the Transcendent Alloy on a wand, crafted: the values
    // of the staves, the better, T1; the one-handed ones T2.
    let suffix = Modifier {
        origin: Some(ModifierOrigin::Crafted),
        ..modifier(
            Some(Affix::Suffix),
            &[
                "28% increased Cast Speed",
                "Gain 9% of Elemental Damage as Extra Cold Damage",
            ],
        )
    };
    let numbers: Option<Vec<_>> = tiers("Acrid Wand", &suffix).map(|tiers| {
        tiers
            .tiers
            .iter()
            .map(|tier| (tier.number, tier.ranges.first().copied()))
            .collect()
    });

    assert_eq!(
        numbers,
        Some(vec![
            (1, Some(range(39.0, 47.0))),
            (2, Some(range(26.0, 31.0)))
        ])
    );
}
