//! The amounts of currency as the window writes them, the same in every language: a point
//! before the decimals, no thousands separator, `K` and `M` for the large amounts.

/// The currencies the prices show in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Currency {
    Divine,
    Chaos,
    Exalted,
}

/// `value` of `currency`, e.g. `12.4`, `182`, `1.3K`, `125K`, `1.2M`. At most three
/// significant digits and two decimals, without trailing zeros; the Exalted Orbs, worth
/// little, without decimals from 10. Under 0.01, `<0.01`.
#[must_use]
pub fn amount(value: f64, currency: Currency) -> String {
    if !value.is_finite() || value <= 0.0 {
        return "0".to_owned();
    }
    if value < 0.01 {
        return "<0.01".to_owned();
    }
    let decimals: u8 = match value {
        v if v >= 100.0 => 0,
        v if v >= 10.0 && currency == Currency::Exalted => 0,
        v if v >= 10.0 => 1,
        _ => 2,
    };
    let small = rounded(value, decimals);
    if small < 1000.0 {
        return trimmed(small, decimals);
    }
    // One decimal under 100K, none from it.
    let thousands = rounded(value / 1e3, u8::from(value < 1e5));
    if thousands < 1000.0 {
        return format!("{}K", trimmed(thousands, 1));
    }
    // One decimal under 100M, none from it.
    let millions = rounded(value / 1e6, u8::from(value < 1e8));
    format!("{}M", trimmed(millions, 1))
}

/// A number of a modifier or of its range, e.g. `40`, `1.5`: two decimals at most, without
/// trailing zeros.
#[must_use]
pub fn number(value: f64) -> String {
    trimmed(rounded(value, 2), 2)
}

/// `value` rounded to `decimals` decimals.
fn rounded(value: f64, decimals: u8) -> f64 {
    let scale = 10f64.powi(i32::from(decimals));
    (value * scale).round() / scale
}

/// `value` with at most `decimals` decimals, without trailing zeros.
fn trimmed(value: f64, decimals: u8) -> String {
    let decimals = usize::from(decimals);
    let text = format!("{value:.decimals$}");
    if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.').to_owned()
    } else {
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_amounts_of_the_mock_up_are_written_as_drawn() {
        assert_eq!(amount(12.4, Currency::Divine), "12.4");
        assert_eq!(amount(123.2, Currency::Chaos), "123");
        assert_eq!(amount(8990.0, Currency::Exalted), "9K");
        assert_eq!(amount(0.25, Currency::Divine), "0.25");
        assert_eq!(amount(2.49, Currency::Chaos), "2.49");
        assert_eq!(amount(182.3, Currency::Exalted), "182");
        assert_eq!(amount(1320.0, Currency::Exalted), "1.3K");
        assert_eq!(amount(27_640.0, Currency::Exalted), "27.6K");
        assert_eq!(amount(1.8, Currency::Divine), "1.8");
    }

    #[test]
    fn the_large_amounts_are_shortened() {
        assert_eq!(amount(125_400.0, Currency::Exalted), "125K");
        assert_eq!(amount(1_240_000.0, Currency::Exalted), "1.2M");
        // Rounded up to the next unit.
        assert_eq!(amount(999.6, Currency::Exalted), "1K");
        assert_eq!(amount(999_960.0, Currency::Exalted), "1M");
        assert_eq!(amount(99_960.0, Currency::Exalted), "100K");
        assert_eq!(amount(123_456_789.0, Currency::Exalted), "123M");
        assert_eq!(amount(12_345_678.0, Currency::Exalted), "12.3M");
    }

    #[test]
    fn a_number_of_a_modifier_has_no_trailing_zeros() {
        assert_eq!(number(40.0), "40");
        assert_eq!(number(1.5), "1.5");
        assert_eq!(number(0.125), "0.13");
    }

    #[test]
    fn an_amount_that_isnt_a_number_is_zero() {
        assert_eq!(amount(f64::INFINITY, Currency::Exalted), "0");
        assert_eq!(amount(f64::NAN, Currency::Exalted), "0");
    }

    #[test]
    fn the_small_amounts_keep_their_decimals() {
        assert_eq!(amount(0.1, Currency::Exalted), "0.1");
        assert_eq!(amount(4.0, Currency::Divine), "4");
        assert_eq!(amount(17.44, Currency::Chaos), "17.4");
        assert_eq!(amount(17.44, Currency::Exalted), "17");
        assert_eq!(amount(0.004, Currency::Divine), "<0.01");
        assert_eq!(amount(0.0, Currency::Divine), "0");
    }
}
