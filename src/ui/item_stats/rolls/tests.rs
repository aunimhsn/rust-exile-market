//! The limits that find comparable items, their decimals, the signs and the shares of a roll.

use super::*;
use crate::filters;
use crate::ui::item_stats::model::Limit;

#[test]
fn a_rare_limit_leaves_10_percent_on_the_worse_side_also_for_a_negative_value() {
    let whole = |value, half| comparable(value, 0, half);

    assert!((whole(12.0, Half::Minimum) - 11.0).abs() < 1e-9);
    assert!((whole(18.0, Half::Maximum) - 20.0).abs() < 1e-9);
    // The item passes its own filter.
    assert!((whole(-15.0, Half::Minimum) - -17.0).abs() < 1e-9);
    assert!((whole(-15.0, Half::Maximum) - -14.0).abs() < 1e-9);
    assert!((comparable(-7.5, 1, Half::Minimum) - -8.3).abs() < 1e-9);
}

#[test]
fn a_rare_limit_keeps_the_decimals_of_the_copied_value_so_that_it_fits_in_its_field() {
    // 12.2 life regenerated per second, the owner's Demon Garb: 10.98 overflowed the field.
    assert!((comparable(12.2, 1, Half::Minimum) - 11.0).abs() < 1e-9);
    assert_eq!(
        filters::limit_text(comparable(12.2, 1, Half::Minimum)),
        "11"
    );
    assert!((comparable(12.5, 1, Half::Minimum) - 11.3).abs() < 1e-9);
    assert!((comparable(1.55, 2, Half::Minimum) - 1.4).abs() < 1e-9);
    assert!((comparable(0.3, 1, Half::Minimum) - 0.3).abs() < 1e-9);
    assert!((comparable(2.4, 1, Half::Maximum) - 2.6).abs() < 1e-9);
    assert!((comparable(-12.2, 1, Half::Minimum) - -13.4).abs() < 1e-9);
}

#[test]
fn a_rare_item_always_passes_its_own_filter() {
    let passes = |value: f64, decimals: u8| {
        comparable(value, decimals, Half::Minimum) <= value
            && comparable(value, decimals, Half::Maximum) >= value
    };
    // Every value written with 0, 1 or 2 decimals, positive or negative.
    for decimals in 0..=2 {
        let scale = 10f64.powi(i32::from(decimals));
        for step in -3000..=3000 {
            let value = f64::from(step) / scale;
            assert!(passes(value, decimals), "{value} with {decimals} decimals");
        }
    }
    // The rounding would cross the value: the average of three whole rolls, and a value with
    // more decimals than the numbers are written with.
    assert!((comparable(4.0 / 3.0, 0, Half::Maximum) - 4.0 / 3.0).abs() < 1e-9);
    assert!((comparable(5.0 / 3.0, 0, Half::Minimum) - 5.0 / 3.0).abs() < 1e-9);
    assert!((comparable(0.019, 2, Half::Minimum) - 0.019).abs() < 1e-9);
}

#[test]
fn the_decimals_of_a_value_are_two_at_most() {
    assert_eq!(decimals(12.0), 0);
    assert_eq!(decimals(-15.0), 0);
    assert_eq!(decimals(12.2), 1);
    assert_eq!(decimals(-7.5), 1);
    assert_eq!(decimals(1.55), 2);
    assert_eq!(decimals(0.123), 2);
    assert_eq!(decimals(7.59), 2);
    assert_eq!(decimals(-1.55), 2);
    // Floating-point noise isn't a decimal.
    assert_eq!(decimals(0.1 + 0.2), 1);
    assert_eq!(decimals(1e6 + 0.2), 1);
}

#[test]
fn a_negative_stat_takes_a_minus_sign() {
    let line = StatLine::parse("-15% to Fire Resistance");
    let negative =
        |value, bounds| Limit::new(value, Half::Minimum, bounds, value, &line).negative();

    assert!(negative(-15.0, None));
    assert!(!negative(15.0, None));
    // A range across zero.
    assert!(negative(
        2.0,
        Some(Range {
            min: -5.0,
            max: 10.0
        })
    ));
    // The game writes a negative range without its sign.
    assert_eq!(
        signed(
            Range {
                min: 10.0,
                max: 20.0
            },
            -15.0
        ),
        Range {
            min: -20.0,
            max: -10.0
        }
    );
}

#[test]
fn a_range_of_one_value_is_full() {
    assert!((share(5.0, Range { min: 5.0, max: 5.0 }) - 1.0).abs() < 1e-9);
    assert!(
        (share(
            30.0,
            Range {
                min: 20.0,
                max: 40.0
            }
        ) - 0.5)
            .abs()
            < 1e-9
    );
}
