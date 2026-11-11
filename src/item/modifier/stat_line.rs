//! A line of a modifier: its text, and its values with the ranges the advanced copy writes,
//! e.g. `Adds 10(1-16) to 273(239-300) Lightning Damage`.

/// The range of a tier, from its lowest to its highest value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Range {
    pub min: f64,
    pub max: f64,
}

impl Range {
    /// `value` brought back within the range; never panics, unlike `f64::clamp`, on a range
    /// written from its highest value or with a NaN end.
    #[must_use]
    pub fn clamp(self, value: f64) -> f64 {
        value.max(self.min).min(self.max)
    }

    /// The range widened to hold `value`, e.g. the value of a legacy unique, rolled in a
    /// range that changed since.
    #[must_use]
    pub fn holding(self, value: f64) -> Self {
        Self {
            min: self.min.min(value),
            max: self.max.max(value),
        }
    }

    /// The range from the lowest to the highest value of both.
    #[must_use]
    pub fn spanning(self, other: Self) -> Self {
        self.holding(other.min).holding(other.max)
    }

    /// The range of the absolute values: the game shows a negative roll without its sign
    /// ("20% reduced"); none for a range across zero, which needs its sign.
    #[must_use]
    pub fn magnitude(self) -> Option<Self> {
        if self.min * self.max < 0.0 {
            return None;
        }
        let (first, second) = (self.min.abs(), self.max.abs());
        Some(Self {
            min: first.min(second),
            max: first.max(second),
        })
    }

    /// Whether the range holds one value only, `2–2`.
    #[must_use]
    pub fn is_one_value(self) -> bool {
        (self.max - self.min).abs() < f64::EPSILON
    }

    /// The ranges of `rolls`, values each with its range without sign, as written under a line:
    /// none when each value is fixed, its range of one value being the value (its sign left
    /// out), as the advanced copy most often writes it, without range ("T2", not "T2 2";
    /// Item block in `specs.md`); a range of one value other than the value stays, `29(30)%`
    /// of a sanctified item ("T2 30"). The values compared as they are, not within the rounding
    /// of the game: a fixed tier with decimals read from a rounded value keeps its range, none
    /// met on 2026-10-30.
    #[must_use]
    pub fn unless_fixed(rolls: impl IntoIterator<Item = (f64, Self)>) -> Vec<Self> {
        let rolls: Vec<(f64, Self)> = rolls.into_iter().collect();
        let fixed = rolls.iter().all(|(value, range)| {
            range.is_one_value() && (value.abs() - range.min).abs() < f64::EPSILON
        });
        if fixed {
            return Vec::new();
        }
        rolls.into_iter().map(|(_, range)| range).collect()
    }
}

/// A value of a line, with the range of its tier when the item was copied with its advanced
/// description, e.g. `10(1-16)`, or when its tier was found from the values of an item copied
/// from the in-game trade.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Roll {
    pub value: f64,
    pub range: Option<Range>,
}

/// A line of a modifier: its text without the ranges and the markers, and its values.
#[derive(Debug, Clone, PartialEq)]
pub struct StatLine {
    /// E.g. `Adds 10 to 273 Lightning Damage`.
    pub text: String,
    pub rolls: Vec<Roll>,
}

impl StatLine {
    /// Reads the values of a line, and removes their ranges from its text:
    /// `Adds 10(1-16) to 273(239-300) Lightning Damage` gives `Adds 10 to 273 Lightning
    /// Damage` and the values 10 and 273 with their ranges.
    pub fn parse(line: &str) -> Self {
        let mut text = String::with_capacity(line.len());
        let mut rolls = Vec::new();
        let mut chars = line.chars().peekable();
        let mut previous: Option<char> = None;

        while let Some(c) = chars.next() {
            if !c.is_ascii_digit() {
                text.push(c);
                previous = Some(c);
                continue;
            }
            let mut number = String::from(c);
            while let Some(&next) = chars.peek() {
                // A point is in the number only between two digits, as the strings of the
                // stats are read (`numbers` in `game_data/lines.rs`): `2.5`, not the point
                // that ends a sentence (`de 2.5.`).
                let decimal_point = next == '.'
                    && chars
                        .clone()
                        .nth(1)
                        .is_some_and(|after| after.is_ascii_digit());
                if next.is_ascii_digit() || decimal_point {
                    number.push(next);
                    chars.next();
                } else {
                    break;
                }
            }
            text.push_str(&number);
            // A minus sign right before the number, not inside a word: `-10% to…`.
            let negative = previous == Some('-') && !text_before_sign_is_word(&text, &number);
            previous = number.chars().last();
            let Ok(value) = number.parse::<f64>() else {
                continue;
            };
            let value = if negative { -value } else { value };
            let range = if chars.peek() == Some(&'(') {
                read_range(&mut chars)
            } else {
                None
            };
            rolls.push(Roll { value, range });
        }
        Self { text, rolls }
    }
}

/// Whether the character before the minus sign of `number`, at the end of `text`, is a
/// letter or a digit (`X-10` is not a negative number).
fn text_before_sign_is_word(text: &str, number: &str) -> bool {
    text.strip_suffix(number)
        .and_then(|before| before.strip_suffix('-'))
        .and_then(|before| before.chars().last())
        .is_some_and(char::is_alphanumeric)
}

/// Reads `(min-max)` after a value, if it is a range, or `(value)`, a range of one value
/// (`29(30)% reduced Attribute Requirements` of a sanctified item): closed, with digits, points
/// and minus signs only. The characters are only consumed when it is one.
fn read_range(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> Option<Range> {
    let mut lookahead = chars.clone();
    lookahead.next(); // (
    let mut inside = String::new();
    loop {
        match lookahead.next()? {
            ')' => break,
            c if c.is_ascii_digit() || c == '.' || c == '-' => inside.push(c),
            _ => return None,
        }
    }
    let (low, high) = split_range(&inside).unwrap_or((&inside, &inside));
    let (low, high) = (low.parse::<f64>().ok()?, high.parse::<f64>().ok()?);
    *chars = lookahead;
    Some(Range {
        min: low.min(high),
        max: low.max(high),
    })
}

/// Splits `1-16` or `-10--5` at the minus sign between the two numbers.
fn split_range(inside: &str) -> Option<(&str, &str)> {
    let start = usize::from(inside.starts_with('-'));
    let (index, _) = inside.char_indices().skip(start).find(|&(_, c)| c == '-')?;
    let low = inside.get(..index)?;
    let high = inside.get(index + 1..)?;
    Some((low, high))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roll(value: f64, range: Option<(f64, f64)>) -> Roll {
        Roll {
            value,
            range: range.map(|(min, max)| Range { min, max }),
        }
    }

    #[test]
    fn the_ranges_of_fixed_values_only_are_left_out() {
        let range = |min, max| Range { min, max };

        assert_eq!(Range::unless_fixed([]), []);
        // Each value fixed, also a negative one, its range without sign.
        assert_eq!(
            Range::unless_fixed([(2.0, range(2.0, 2.0)), (-10.0, range(10.0, 10.0))]),
            []
        );
        // One varies: every range stays, as one varying alone.
        assert_eq!(
            Range::unless_fixed([(5.0, range(5.0, 5.0)), (10.0, range(8.0, 12.0))]),
            [range(5.0, 5.0), range(8.0, 12.0)]
        );
        // A sanctified value out of its range of one value.
        assert_eq!(
            Range::unless_fixed([(29.0, range(30.0, 30.0))]),
            [range(30.0, 30.0)]
        );
    }

    #[test]
    fn a_range_spanning_another_goes_from_the_lowest_to_the_highest_of_both() {
        let range = |min, max| Range { min, max };
        // Apart, in either order.
        assert_eq!(range(1.0, 4.0).spanning(range(7.0, 9.0)), range(1.0, 9.0));
        assert_eq!(range(7.0, 9.0).spanning(range(1.0, 4.0)), range(1.0, 9.0));
        // One within the other.
        assert_eq!(range(1.0, 9.0).spanning(range(3.0, 4.0)), range(1.0, 9.0));
        assert_eq!(range(3.0, 4.0).spanning(range(1.0, 9.0)), range(1.0, 9.0));
    }

    #[test]
    fn reads_the_values_and_their_ranges() {
        let line = StatLine::parse("Adds 10(1-16) to 273(239-300) Lightning Damage");

        assert_eq!(line.text, "Adds 10 to 273 Lightning Damage");
        assert_eq!(
            line.rolls,
            [
                roll(10.0, Some((1.0, 16.0))),
                roll(273.0, Some((239.0, 300.0)))
            ]
        );
    }

    #[test]
    fn reads_decimal_negative_and_reversed_values() {
        let line = StatLine::parse("Leeches 7.59(7-7.9)% of Physical Damage as Mana");
        assert_eq!(line.rolls, [roll(7.59, Some((7.0, 7.9)))]);

        let line = StatLine::parse("-10% to Cold Resistance");
        assert_eq!(line.rolls, [roll(-10.0, None)]);

        let line = StatLine::parse("Adds 1(-10--5) to X-10");
        assert_eq!(
            line.rolls,
            [roll(1.0, Some((-10.0, -5.0))), roll(10.0, None)]
        );

        let line =
            StatLine::parse("Réduit la durée des saignements vous affectant de 41(45-41) %.");
        assert_eq!(
            line.text,
            "Réduit la durée des saignements vous affectant de 41 %."
        );
        assert_eq!(line.rolls, [roll(41.0, Some((41.0, 45.0)))]);

        // The point that ends a sentence isn't in its number.
        let line = StatLine::parse("augmentent le nombre maximum de totems invoqués de 2.5.");
        assert_eq!(
            line.text,
            "augmentent le nombre maximum de totems invoqués de 2.5."
        );
        assert_eq!(line.rolls, [roll(2.5, None)]);
        let line = StatLine::parse("de 41.");
        assert_eq!(
            (line.text.as_str(), line.rolls),
            ("de 41.", vec![roll(41.0, None)])
        );
        let line = StatLine::parse("de 2.5(2-3).");
        assert_eq!(
            (line.text.as_str(), line.rolls),
            ("de 2.5.", vec![roll(2.5, Some((2.0, 3.0)))])
        );
    }

    #[test]
    fn reads_a_range_of_one_value() {
        let line = StatLine::parse("29(30)% reduced Attribute Requirements");
        assert_eq!(line.text, "29% reduced Attribute Requirements");
        assert_eq!(line.rolls, [roll(29.0, Some((30.0, 30.0)))]);

        let line = StatLine::parse("0.5(0.6)% of Physical Attack Damage Leeched as Life");
        assert_eq!(line.text, "0.5% of Physical Attack Damage Leeched as Life");
        assert_eq!(line.rolls, [roll(0.5, Some((0.6, 0.6)))]);
    }

    #[test]
    fn keeps_the_parentheses_that_are_not_ranges() {
        let line = StatLine::parse("Adds 5 (cold) and 3(a-b)");
        assert_eq!(line.text, "Adds 5 (cold) and 3(a-b)");
        assert_eq!(line.rolls, [roll(5.0, None), roll(3.0, None)]);

        // Only a closed range of digits, points and minus signs: what `f64` reads besides
        // (`nan`, `1e3`, `+5`) stays in the text.
        for text in [
            "4() a",
            "6(x) a",
            "7(nan) a",
            "8(inf) a",
            "9(1e3) a",
            "1(+5) a",
            "2(5 - 6) a",
            "3(10",
        ] {
            let line = StatLine::parse(text);
            assert_eq!(line.text, text);
            assert_eq!(
                line.rolls.first().and_then(|roll| roll.range),
                None,
                "{text}"
            );
        }
    }
}
