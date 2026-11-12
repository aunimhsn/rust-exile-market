//! What the drawing computes before it draws: the tooltip of a range field, the bar of the tier
//! of a waystone.

use super::*;
use crate::tier_rolls::TierSpan;

#[test]
fn the_tooltip_of_a_range_field_tells_its_rolls_and_its_better_side() {
    let i18n = Translator::new(crate::language::Language::English);
    let range = Some(Range {
        min: 15.0,
        max: 25.0,
    });
    let tooltip = |half, rolls, tiers| {
        let limit = Limit {
            value: 18.0,
            half,
            bounds: range,
            rolls,
            tiers,
        };
        tooltip_lines(limit, &i18n)
    };
    let rolls = TooltipLine::new("Rolls 15 to 25".to_owned(), color::TEXT);
    let lower = TooltipLine::new("Lower is better".to_owned(), color::WEAK);

    assert_eq!(
        tooltip(Half::Minimum, range, None),
        std::slice::from_ref(&rolls)
    );
    assert_eq!(
        tooltip(Half::Maximum, range, None),
        [rolls.clone(), lower.clone()]
    );
    // A line of several numbers.
    assert_eq!(
        tooltip(Half::Maximum, None, None),
        std::slice::from_ref(&lower)
    );
    assert_eq!(tooltip(Half::Minimum, None, None), []);
    // The required level of a rare for leveling, searched with a maximum: lower is better.
    assert_eq!(
        tooltip_lines(Limit::maximum(40.0), &i18n),
        std::slice::from_ref(&lower)
    );
    // A prefix or suffix: the tiers of its rolls right under them, small and faint.
    let tiers = TooltipLine {
        text: "T5–T1".to_owned(),
        tint: color::FAINT,
        is_small: true,
    };
    assert_eq!(
        tooltip(
            Half::Maximum,
            range,
            Some(TierSpan {
                lowest: 5,
                highest: 1,
            })
        ),
        [rolls, tiers, lower]
    );
}

#[test]
fn the_bar_of_the_tier_of_a_waystone_goes_from_tier_1_to_tier_16() {
    assert!(waystone_tier_share(1).abs() < f32::EPSILON);
    assert!((waystone_tier_share(16) - 1.0).abs() < f32::EPSILON);
    assert!((waystone_tier_share(12) - 11.0 / 15.0).abs() < 1e-6);
    // Never out of the bar.
    assert!(waystone_tier_share(0).abs() < f32::EPSILON);
    assert!((waystone_tier_share(17) - 1.0).abs() < f32::EPSILON);
}
