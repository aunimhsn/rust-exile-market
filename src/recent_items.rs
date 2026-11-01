//! The last items copied, remembered with the filters the player set and their last offers, so
//! that an item copied again comes back as the player left it. They are kept while the
//! application runs, not from one launch to the next.

use std::collections::VecDeque;

use crate::filters::Filters;
use crate::item::CopiedItem;
use crate::offers::{Offers, OffersState};

/// The items remembered, the item shown among them: an eleventh item copied makes the oldest
/// one forgotten.
pub const REMEMBERED: usize = 10;

/// An item left for another one, as the player left it.
#[derive(Debug, Default)]
struct Left {
    /// Its filters, as the player set them.
    filters: Filters,
    /// Its last offers.
    offers: OffersState,
    /// The number of lines of each of its modifiers when it was left: the filters of its lines
    /// are kept by their places, which hold while the item read again keeps this shape.
    shape: Vec<usize>,
}

/// The item shown, and the items copied before it, known by their exact copied text.
#[derive(Debug, Default)]
pub struct RecentItems {
    /// The last item copied.
    shown: Option<CopiedItem>,
    /// The items left before it, as the player left them, the last one left first.
    left: VecDeque<(CopiedItem, Left)>,
}

impl RecentItems {
    /// The last item copied, the item shown.
    #[must_use]
    pub fn shown(&self) -> Option<&CopiedItem> {
        self.shown.as_ref()
    }

    /// Shows `item`, copied, in place of the item shown, whose modifiers have `shape`: the
    /// item shown is remembered with the `filters` and the `offers` the player left, which
    /// become those of `item` as they left them, or none. The item shown copied again keeps
    /// them. The shape of the modifiers of `item` when it was left, when it comes back.
    pub fn show(
        &mut self,
        item: &CopiedItem,
        shape: Vec<usize>,
        filters: &mut Filters,
        offers: &mut Offers,
    ) -> Option<Vec<usize>> {
        if self.shown.as_ref() == Some(item) {
            return None;
        }
        // A number being typed is settled, as leaving its field does.
        filters.settle_edited();
        let back = self
            .left
            .iter()
            .position(|(left, _)| left == item)
            .and_then(|index| self.left.remove(index))
            .map(|(_, back)| back);
        let back_shape = back.as_ref().map(|back| back.shape.clone());
        let back = back.unwrap_or_default();
        let left = Left {
            filters: std::mem::replace(filters, back.filters),
            offers: offers.leave(),
            shape,
        };
        offers.show(back.offers);
        if let Some(shown) = self.shown.replace(item.clone()) {
            self.left.push_front((shown, left));
            self.left.truncate(REMEMBERED - 1);
        }
        back_shape
    }

    /// Forgets the offers of every item left: they were searched in another league.
    pub fn forget_offers(&mut self) {
        for (_, left) in &mut self.left {
            left.offers = OffersState::Idle;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Instant, SystemTime};

    use super::*;
    use crate::filters::{Filter, FilterKey, Half, Property};
    use crate::language::Language;
    use crate::offers::tests::{after, answer, exchange_search, item_search};
    use crate::trade::search::{Order, Outcome, Rest};

    fn ring(name: &str) -> CopiedItem {
        CopiedItem::from_text(&format!(
            "Item Class: Rings\nRarity: Rare\n{name}\nRuby Ring"
        ))
        .expect("an item")
    }

    fn filter(checked: bool, minimum: &str) -> Filter {
        Filter {
            checked,
            minimum: minimum.to_owned(),
            maximum: String::new(),
            bounds: None,
        }
    }

    /// The item level searched: its minimum, or `None` when the player didn't set it.
    fn item_level(filters: &Filters) -> Option<String> {
        let unset = || filter(false, "unset");
        let level = filters.get(FilterKey::Property(Property::ItemLevel), unset);
        (level.minimum != "unset").then_some(level.minimum)
    }

    fn set_item_level(filters: &mut Filters, minimum: &str) {
        filters.set(
            FilterKey::Property(Property::ItemLevel),
            filter(true, minimum),
        );
    }

    /// The offers of the currency exchange shown, `total` of them found.
    fn exchange_total(offers: &Offers) -> Option<u64> {
        match offers.state() {
            OffersState::Exchange { total, .. } => Some(*total),
            _ => None,
        }
    }

    /// `offers` with the answer to a search of the currency exchange, `total` offers found.
    fn searched(offers: &mut Offers, total: u64, now: Instant) {
        let _ = offers.search(exchange_search(), now);
        offers.apply(
            answer(
                Outcome::Exchange {
                    offers: Vec::new(),
                    total,
                },
                after(now, 0),
            ),
            SystemTime::now(),
        );
    }

    /// The item shown, its filters and offers, with `recent` the items copied before it.
    struct Shown {
        recent: RecentItems,
        filters: Filters,
        offers: Offers,
    }

    impl Shown {
        fn new() -> Self {
            Self {
                recent: RecentItems::default(),
                filters: Filters::default(),
                offers: Offers::default(),
            }
        }

        fn copy(&mut self, item: &CopiedItem) -> Option<Vec<usize>> {
            self.recent
                .show(item, vec![1, 2], &mut self.filters, &mut self.offers)
        }
    }

    #[test]
    fn an_item_copied_again_after_another_comes_back_as_the_player_left_it() {
        let now = Instant::now();
        let mut shown = Shown::new();
        assert_eq!(shown.copy(&ring("Woe Loop")), None, "not remembered");
        set_item_level(&mut shown.filters, "80");
        searched(&mut shown.offers, 3, now);

        assert_eq!(shown.copy(&ring("Doom Loop")), None, "not remembered");
        assert_eq!(item_level(&shown.filters), None, "its own filters");
        assert_eq!(shown.offers.state(), &OffersState::Idle, "no offers yet");
        set_item_level(&mut shown.filters, "75");

        assert_eq!(shown.copy(&ring("Woe Loop")), Some(vec![1, 2]));
        assert_eq!(item_level(&shown.filters).as_deref(), Some("80"));
        assert_eq!(exchange_total(&shown.offers), Some(3));
        assert_eq!(shown.recent.shown(), Some(&ring("Woe Loop")));

        shown.copy(&ring("Doom Loop"));
        assert_eq!(item_level(&shown.filters).as_deref(), Some("75"));
    }

    #[test]
    fn the_item_shown_copied_again_keeps_its_filters_and_offers() {
        let now = Instant::now();
        let mut shown = Shown::new();
        shown.copy(&ring("Woe Loop"));
        set_item_level(&mut shown.filters, "80");
        searched(&mut shown.offers, 3, now);

        assert_eq!(shown.copy(&ring("Woe Loop")), None);

        assert_eq!(item_level(&shown.filters).as_deref(), Some("80"));
        assert_eq!(exchange_total(&shown.offers), Some(3));
    }

    #[test]
    fn the_item_shown_and_the_nine_items_copied_before_it_are_remembered() {
        let mut shown = Shown::new();
        // Eleven items, the first one the oldest.
        for index in 0..=REMEMBERED {
            shown.copy(&ring(&format!("Ring {index}")));
            set_item_level(&mut shown.filters, &index.to_string());
        }

        // The oldest item remembered, then the one copied before it.
        shown.copy(&ring("Ring 1"));
        assert_eq!(item_level(&shown.filters).as_deref(), Some("1"));
        shown.copy(&ring("Ring 0"));
        assert_eq!(item_level(&shown.filters), None, "forgotten");
    }

    #[test]
    fn an_item_copied_again_goes_back_to_the_first_place() {
        let mut shown = Shown::new();
        for index in 0..REMEMBERED {
            shown.copy(&ring(&format!("Ring {index}")));
            set_item_level(&mut shown.filters, &index.to_string());
        }

        // The oldest copied again, then one more item: the next oldest is forgotten.
        shown.copy(&ring("Ring 0"));
        shown.copy(&ring("Ring 10"));

        shown.copy(&ring("Ring 0"));
        assert_eq!(item_level(&shown.filters).as_deref(), Some("0"));
        shown.copy(&ring("Ring 1"));
        assert_eq!(item_level(&shown.filters), None, "forgotten");
    }

    #[test]
    fn a_number_being_typed_is_settled_when_the_item_is_left() {
        let mut shown = Shown::new();
        shown.copy(&ring("Woe Loop"));
        let key = FilterKey::Property(Property::ItemLevel);
        shown.filters.set(
            key,
            Filter {
                bounds: Some(crate::item::Range {
                    min: 1.0,
                    max: 100.0,
                }),
                ..filter(true, "120")
            },
        );
        shown.filters.edit(key, Half::Minimum);

        shown.copy(&ring("Doom Loop"));
        shown.copy(&ring("Woe Loop"));

        assert_eq!(item_level(&shown.filters).as_deref(), Some("100"));
    }

    #[test]
    fn a_list_of_offers_that_comes_back_loads_the_rest_of_its_own_search() {
        let now = Instant::now();
        let mut shown = Shown::new();
        shown.copy(&ring("Woe Loop"));
        let woe = Rest::new(Language::English, "Q1", &["k"]);
        let _ = shown.offers.search(item_search(), now);
        let outcome = Outcome::Items {
            offers: Vec::new(),
            total: 11,
            rest: woe.clone(),
        };
        shown
            .offers
            .apply(answer(outcome, after(now, 0)), SystemTime::now());
        // Its next offers asked, then left before they came.
        assert!(shown.offers.more().is_some());

        shown.copy(&ring("Doom Loop"));
        // The answer to the next offers of the item left changes nothing.
        let more = Outcome::More {
            offers: Vec::new(),
            rest: Rest::new(Language::English, "Q1", &[]),
        };
        shown
            .offers
            .apply(answer(more, after(now, 0)), SystemTime::now());
        let _ = shown.offers.search(item_search(), now);
        let doom = Outcome::Items {
            offers: Vec::new(),
            total: 12,
            rest: Rest::new(Language::English, "Q2", &["x"]),
        };
        shown
            .offers
            .apply(answer(doom, after(now, 0)), SystemTime::now());

        shown.copy(&ring("Woe Loop"));
        assert!(matches!(
            shown.offers.state(),
            OffersState::Items {
                total: 11,
                loading_more: false,
                ..
            }
        ));
        assert_eq!(shown.offers.more(), Some(Order::More(woe)));
    }

    #[test]
    fn a_search_left_before_its_answer_comes_back_without_offers() {
        let now = Instant::now();
        let mut shown = Shown::new();
        shown.copy(&ring("Woe Loop"));
        let _ = shown.offers.search(exchange_search(), now);

        shown.copy(&ring("Doom Loop"));
        let outcome = Outcome::Exchange {
            offers: Vec::new(),
            total: 3,
        };
        shown
            .offers
            .apply(answer(outcome, after(now, 10)), SystemTime::now());
        assert_eq!(shown.offers.state(), &OffersState::Idle, "not its answer");
        shown.copy(&ring("Woe Loop"));

        assert_eq!(shown.offers.state(), &OffersState::Idle);
        assert!(
            !shown
                .offers
                .wait(crate::trade::query::SearchKind::Exchange, now)
                .is_zero()
        );
    }

    #[test]
    fn a_change_of_league_forgets_the_offers_but_not_the_filters() {
        let now = Instant::now();
        let mut shown = Shown::new();
        shown.copy(&ring("Woe Loop"));
        set_item_level(&mut shown.filters, "80");
        searched(&mut shown.offers, 3, now);
        shown.copy(&ring("Doom Loop"));

        shown.recent.forget_offers();
        shown.copy(&ring("Woe Loop"));

        assert_eq!(shown.offers.state(), &OffersState::Idle);
        assert_eq!(item_level(&shown.filters).as_deref(), Some("80"));
    }
}
