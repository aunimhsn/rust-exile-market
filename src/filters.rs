//! The filters of the offers search, as the player sets them: whether each stat of the item,
//! or its base type, item level, sockets, quality, required level, DPS, the properties of a
//! waystone or of a gem, or pseudo totals, is searched, its minimum and its maximum, and the
//! state tags the player lit. They are kept for the item shown, until an item is copied, even
//! the same one after another text; a stat the player didn't touch keeps the choice made for it
//! by default.
//!
//! The minimum and the maximum searched stay within the range of the stat, whatever the
//! player is typing ([`Filter::searched`]).

use std::collections::HashMap;

use crate::format;
use crate::item::{Item, ItemState, Range, WaystoneProperty};
use crate::trade::pseudo::Pseudo;

/// A line of a modifier of the item: the index of its modifier among the modifiers of the
/// item, and of the line in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StatKey {
    pub modifier: usize,
    pub line: usize,
}

/// What a filter searches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FilterKey {
    /// A line of a modifier.
    Stat(StatKey),
    /// The base type of the item: unchecked, the offers may have any base.
    BaseType,
    Property(Property),
    /// A total the trade site adds up from several lines.
    Pseudo(Pseudo),
}

/// A property of the item the trade site searches with a minimum and a maximum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Property {
    ItemLevel,
    /// The augmentable sockets, empty or not.
    Sockets,
    /// The quality, in percent.
    Quality,
    /// The level a character needs, searched with a maximum.
    RequiredLevel,
    PhysicalDps,
    /// Fire, cold and lightning together.
    ElementalDps,
    Waystone(WaystoneProperty),
    /// The level of a gem.
    GemLevel,
    /// The sockets of a gem, for its support gems.
    GemSockets,
}

/// A half of the range field of a filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Half {
    Minimum,
    Maximum,
}

impl Half {
    fn other(self) -> Self {
        match self {
            Self::Minimum => Self::Maximum,
            Self::Maximum => Self::Minimum,
        }
    }
}

/// A filter as the player set it.
#[derive(Debug, Clone, PartialEq)]
pub struct Filter {
    pub checked: bool,
    /// The text of the minimum: a number, or empty for no minimum.
    pub minimum: String,
    /// The text of the maximum: a number, or empty for no maximum.
    pub maximum: String,
    /// The range the minimum and the maximum stay in, when known.
    pub bounds: Option<Range>,
}

impl Filter {
    /// The number written in `half`, brought back within the range of the stat; `None` when
    /// the half holds no number (empty, `-` or `.` alone).
    #[must_use]
    pub fn searched(&self, half: Half) -> Option<f64> {
        let value: f64 = self
            .text(half)
            .parse()
            .ok()
            .filter(|value: &f64| value.is_finite())?;
        Some(self.bounds.map_or(value, |bounds| bounds.clamp(value)))
    }

    /// The minimum and the maximum searched, within the range of the stat; a minimum above
    /// the maximum brings the maximum to it, even while the player is still in a half
    /// ([`Filter::settle`] runs only when they leave it), so that a search never asks for
    /// nothing.
    #[must_use]
    pub fn limits(&self) -> (Option<f64>, Option<f64>) {
        let minimum = self.searched(Half::Minimum);
        let maximum = self.searched(Half::Maximum);
        match (minimum, maximum) {
            (Some(low), Some(high)) if low > high => (minimum, minimum),
            _ => (minimum, maximum),
        }
    }

    #[must_use]
    pub fn text(&self, half: Half) -> &str {
        match half {
            Half::Minimum => &self.minimum,
            Half::Maximum => &self.maximum,
        }
    }

    pub fn text_mut(&mut self, half: Half) -> &mut String {
        match half {
            Half::Minimum => &mut self.minimum,
            Half::Maximum => &mut self.maximum,
        }
    }

    /// The player leaves `half`: its number is brought back within the range of the stat, the
    /// half emptied when it holds no number; a minimum above the maximum brings the other
    /// half to the same number.
    pub fn settle(&mut self, half: Half) {
        *self.text_mut(half) = self.searched(half).map(limit_text).unwrap_or_default();
        let (Some(value), Some(other)) = (self.searched(half), self.searched(half.other())) else {
            return;
        };
        let crossed = match half {
            Half::Minimum => value > other,
            Half::Maximum => value < other,
        };
        if crossed {
            *self.text_mut(half.other()) = self.text(half).to_owned();
        }
    }
}

/// The filters the player changed, for the item shown.
#[derive(Debug, Default)]
pub struct Filters {
    changed: HashMap<FilterKey, Filter>,
    /// The state tags lit: the offers of these states only.
    lit: Vec<ItemState>,
    /// The half of a range field the player was in last: settled again when its field
    /// disappears before they leave it, which settling twice leaves the same.
    edited: Option<(FilterKey, Half)>,
}

impl Filters {
    /// The filter of `key` as the player set it, or else `default`.
    #[must_use]
    pub fn get(&self, key: FilterKey, default: impl FnOnce() -> Filter) -> Filter {
        self.changed.get(&key).cloned().unwrap_or_else(default)
    }

    pub fn set(&mut self, key: FilterKey, filter: Filter) {
        self.changed.insert(key, filter);
    }

    /// The states of `item` whose tag the player lit: those searched, and counted among the
    /// active filters.
    pub fn lit_states_of<'a>(&'a self, item: &'a Item) -> impl Iterator<Item = ItemState> + 'a {
        self.lit
            .iter()
            .copied()
            .filter(|&state| item.has_state(state))
    }

    #[must_use]
    pub fn is_lit(&self, state: ItemState) -> bool {
        self.lit.contains(&state)
    }

    /// Lights the tag of `state`, or turns it gray again.
    pub fn toggle_state(&mut self, state: ItemState) {
        if self.is_lit(state) {
            self.lit.retain(|&lit| lit != state);
        } else {
            self.lit.push(state);
        }
    }

    /// Forgets the choices of the player on the lines of the modifiers, whose keys moved: the
    /// item shown was read again.
    pub fn forget_stats(&mut self) {
        self.changed
            .retain(|key, _| !matches!(key, FilterKey::Stat(_)));
    }

    /// The player is in `half` of the range field of `key`.
    pub fn edit(&mut self, key: FilterKey, half: Half) {
        self.edited = Some((key, half));
    }

    /// Settles the half the player was in, as leaving it does, when its field disappears before
    /// they leave it (the window over the game closed): a widget no longer drawn never sees its
    /// focus lost.
    pub fn settle_edited(&mut self) {
        if let Some((key, half)) = self.edited.take()
            && let Some(filter) = self.changed.get_mut(&key)
        {
            filter.settle(half);
        }
    }

    /// Forgets the choices of the player: an item is copied.
    pub fn clear(&mut self) {
        self.changed.clear();
        self.lit.clear();
        self.edited = None;
    }
}

/// The text of a minimum or a maximum, e.g. `40`, `7.5`, `-15`.
#[must_use]
pub fn limit_text(value: f64) -> String {
    format::number(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    const RANGE: Option<Range> = Some(Range {
        min: 40.0,
        max: 60.0,
    });

    fn filter(minimum: &str, maximum: &str, bounds: Option<Range>) -> Filter {
        Filter {
            checked: true,
            minimum: minimum.to_owned(),
            maximum: maximum.to_owned(),
            bounds,
        }
    }

    /// The texts of a filter of `RANGE` holding `minimum` and `maximum`, once the player
    /// leaves `half`.
    fn settled(minimum: &str, maximum: &str, half: Half) -> (String, String) {
        let mut filter = filter(minimum, maximum, RANGE);
        filter.settle(half);
        (filter.minimum, filter.maximum)
    }

    fn texts(minimum: &str, maximum: &str) -> (String, String) {
        (minimum.to_owned(), maximum.to_owned())
    }

    #[test]
    fn the_choices_on_the_lines_are_forgotten_when_the_item_is_read_again() {
        let mut filters = Filters::default();
        let line = FilterKey::Stat(StatKey {
            modifier: 1,
            line: 0,
        });
        let unchecked = || Filter {
            checked: false,
            ..filter("", "", None)
        };
        filters.set(line, unchecked());
        filters.set(FilterKey::BaseType, unchecked());
        filters.toggle_state(ItemState::Corrupted);

        filters.forget_stats();

        assert!(filters.get(line, || filter("", "", None)).checked);
        assert!(
            !filters
                .get(FilterKey::BaseType, || filter("", "", None))
                .checked
        );
        assert!(filters.is_lit(ItemState::Corrupted));
    }

    #[test]
    fn a_click_lights_a_state_tag_and_another_turns_it_gray() {
        let mut filters = Filters::default();

        filters.toggle_state(ItemState::Corrupted);
        filters.toggle_state(ItemState::Fractured);
        assert!(filters.is_lit(ItemState::Corrupted));
        assert!(filters.is_lit(ItemState::Fractured));
        filters.toggle_state(ItemState::Corrupted);
        assert!(!filters.is_lit(ItemState::Corrupted));
        assert!(filters.is_lit(ItemState::Fractured));
        // A copied item starts gray.
        filters.clear();
        assert!(!filters.is_lit(ItemState::Fractured));
    }

    #[test]
    fn only_the_states_of_the_item_are_lit_for_it() {
        let ring = Item::parse(
            &crate::item::CopiedItem::from_text(
                "Item Class: Rings\nRarity: Rare\nWoe Loop\nRuby Ring\n--------\nCorrupted",
            )
            .expect("an item"),
        )
        .expect("read");
        let mut filters = Filters::default();

        filters.toggle_state(ItemState::Mirrored);
        filters.toggle_state(ItemState::Corrupted);
        assert_eq!(
            filters.lit_states_of(&ring).collect::<Vec<_>>(),
            [ItemState::Corrupted]
        );
    }

    #[test]
    fn a_limit_is_searched_within_the_range_of_its_stat() {
        for half in [Half::Minimum, Half::Maximum] {
            let searched = |text: &str, bounds| {
                let mut filter = filter("", "", bounds);
                *filter.text_mut(half) = text.to_owned();
                filter.searched(half)
            };

            assert_eq!(searched("55", RANGE), Some(55.0));
            assert_eq!(searched("5", RANGE), Some(40.0));
            assert_eq!(searched("600", RANGE), Some(60.0));
            assert_eq!(searched("600", None), Some(600.0));
            // Not a number: no limit.
            for text in ["", "-", "."] {
                assert_eq!(searched(text, RANGE), None, "{text:?}");
            }
        }
    }

    #[test]
    fn a_half_left_is_brought_back_within_the_range_of_its_stat() {
        assert_eq!(settled("5", "", Half::Minimum), texts("40", ""));
        assert_eq!(settled("", "600", Half::Maximum), texts("", "60"));
        assert_eq!(settled("45.", "", Half::Minimum), texts("45", ""));
        assert_eq!(settled("-", "50", Half::Minimum), texts("", "50"));
        // Only the half left.
        assert_eq!(settled("5", "600", Half::Maximum), texts("5", "60"));
    }

    #[test]
    fn a_minimum_above_the_maximum_brings_the_other_half_to_it() {
        assert_eq!(settled("55", "50", Half::Minimum), texts("55", "55"));
        assert_eq!(settled("55", "50", Half::Maximum), texts("50", "50"));
        // Brought within the range first.
        assert_eq!(settled("70", "50", Half::Minimum), texts("60", "60"));
        assert_eq!(settled("45", "50", Half::Minimum), texts("45", "50"));
        assert_eq!(settled("50", "50", Half::Maximum), texts("50", "50"));
        // An empty half has no limit.
        assert_eq!(settled("55", "", Half::Minimum), texts("55", ""));
    }

    #[test]
    fn a_minimum_above_the_maximum_is_never_searched() {
        let limits = |minimum: &str, maximum: &str| filter(minimum, maximum, RANGE).limits();

        assert_eq!(limits("45", "55"), (Some(45.0), Some(55.0)));
        assert_eq!(limits("50", "50"), (Some(50.0), Some(50.0)));
        // The player is still typing in a half: the maximum is brought to the minimum.
        assert_eq!(limits("55", "45"), (Some(55.0), Some(55.0)));
        // Brought within the range first: 70 is searched at 60, 5 at 40.
        assert_eq!(limits("70", "5"), (Some(60.0), Some(60.0)));
        assert_eq!(limits("", "45"), (None, Some(45.0)));
        assert_eq!(limits("-", ""), (None, None));
    }

    #[test]
    fn a_filter_without_range_or_with_a_negative_one_is_settled_alike() {
        let settled = |minimum: &str, maximum: &str, bounds, half| {
            let mut filter = filter(minimum, maximum, bounds);
            filter.settle(half);
            (filter.minimum, filter.maximum)
        };
        let negative = Some(Range {
            min: -20.0,
            max: -10.0,
        });

        // Without range: any number, written as a number.
        assert_eq!(
            settled("600.50", "", None, Half::Minimum),
            texts("600.5", "")
        );
        assert_eq!(settled("70", "50", None, Half::Minimum), texts("70", "70"));
        assert_eq!(settled("-", "", None, Half::Minimum), texts("", ""));
        // A negative range: -5 is above it, -25 under it.
        assert_eq!(settled("-5", "", negative, Half::Minimum), texts("-10", ""));
        assert_eq!(
            settled("", "-25", negative, Half::Maximum),
            texts("", "-20")
        );
        assert_eq!(
            settled("-12", "-15", negative, Half::Maximum),
            texts("-15", "-15")
        );
    }

    #[test]
    fn the_half_left_unsettled_is_settled_when_its_field_disappears() {
        let mut filters = Filters::default();
        filters.set(
            FilterKey::Property(Property::ItemLevel),
            filter("99", "", RANGE),
        );
        filters.edit(FilterKey::Property(Property::ItemLevel), Half::Minimum);
        filters.settle_edited();
        let settled = filters.get(FilterKey::Property(Property::ItemLevel), || {
            filter("", "", None)
        });
        assert_eq!(settled.minimum, "60");
        // Once only: the half is no longer remembered.
        filters.set(
            FilterKey::Property(Property::ItemLevel),
            filter("99", "", RANGE),
        );
        filters.settle_edited();
        let unsettled = filters.get(FilterKey::Property(Property::ItemLevel), || {
            filter("", "", None)
        });
        assert_eq!(unsettled.minimum, "99");
    }

    #[test]
    fn a_new_item_forgets_the_half_edited() {
        let mut filters = Filters::default();
        filters.edit(FilterKey::Property(Property::ItemLevel), Half::Minimum);
        filters.clear();
        filters.set(
            FilterKey::Property(Property::ItemLevel),
            filter("99", "", RANGE),
        );
        filters.settle_edited();
        let unsettled = filters.get(FilterKey::Property(Property::ItemLevel), || {
            filter("", "", None)
        });
        assert_eq!(unsettled.minimum, "99");
    }
}
