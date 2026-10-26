//! The stat filters of the offers search, as the player sets them: whether each stat of the
//! item is searched, and its minimum. They are kept for the item shown, until an item is
//! copied, even the same one after another text; a stat the player didn't touch keeps the
//! choice made for it by default.
//!
//! The minimum searched stays within the range of the stat, whatever the player is typing
//! ([`Filter::searched_minimum`]).

use std::collections::HashMap;

use crate::format;
use crate::item::Range;

/// A line of a modifier of the item: the index of its modifier among the modifiers of the
/// item, and of the line in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StatKey {
    pub modifier: usize,
    pub line: usize,
}

/// A filter as the player set it.
#[derive(Debug, Clone, PartialEq)]
pub struct Filter {
    pub checked: bool,
    /// The text of the minimum field: a number, or empty for no minimum.
    pub minimum: String,
    /// The range the minimum stays in, when known.
    pub bounds: Option<Range>,
}

impl Filter {
    /// The minimum written, brought back within the range of the stat; `None` when the
    /// field holds no number (empty, `-` or `.` alone).
    #[must_use]
    pub fn searched_minimum(&self) -> Option<f64> {
        let value: f64 = self
            .minimum
            .parse()
            .ok()
            .filter(|value: &f64| value.is_finite())?;
        Some(self.bounds.map_or(value, |bounds| bounds.clamp(value)))
    }
}

/// The filters the player changed, for the item shown.
#[derive(Debug, Default)]
pub struct Filters {
    changed: HashMap<StatKey, Filter>,
}

impl Filters {
    /// The filter of `key` as the player set it, or else `default`.
    #[must_use]
    pub fn get(&self, key: StatKey, default: impl FnOnce() -> Filter) -> Filter {
        self.changed.get(&key).cloned().unwrap_or_else(default)
    }

    pub fn set(&mut self, key: StatKey, filter: Filter) {
        self.changed.insert(key, filter);
    }

    /// Forgets the choices of the player: an item is copied.
    pub fn clear(&mut self) {
        self.changed.clear();
    }
}

/// The text of a minimum, e.g. `40`, `7.5`, `-15`.
#[must_use]
pub fn minimum_text(value: f64) -> String {
    format::number(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_minimum_is_searched_within_the_range_of_its_stat() {
        let filter = |minimum: &str, bounds| Filter {
            checked: true,
            minimum: minimum.to_owned(),
            bounds,
        };
        let range = Some(Range {
            min: 40.0,
            max: 60.0,
        });

        assert_eq!(filter("55", range).searched_minimum(), Some(55.0));
        assert_eq!(filter("5", range).searched_minimum(), Some(40.0));
        assert_eq!(filter("600", range).searched_minimum(), Some(60.0));
        assert_eq!(filter("600", None).searched_minimum(), Some(600.0));
        // Not a number: no minimum.
        for text in ["", "-", "."] {
            assert_eq!(filter(text, range).searched_minimum(), None, "{text:?}");
        }
    }
}
