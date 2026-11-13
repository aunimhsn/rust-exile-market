use super::*;
use crate::language::Language;
use crate::offers_state::SearchedBy;
use crate::trade::offers::fixtures::{exchange_offer, item_offer};
use crate::trade::search::Rest;

#[test]
fn no_offer_is_said_only_when_none_is_left_to_fetch() {
    assert!(no_offer(&[], More::None));
    assert!(
        !no_offer(&[], More::Chevron),
        "a page left out for its currency: the next ones may be shown"
    );
    assert!(!no_offer(&[], More::Loading));
    let rows = item_rows(
        &[item_offer(1.0, "divine")],
        None,
        None,
        SystemTime::UNIX_EPOCH,
    );
    assert!(!no_offer(&rows, More::None));
}

#[test]
fn the_offers_of_items_show_the_chevron_while_some_are_left_and_the_spinner_while_they_come() {
    let mut items = ItemOffers {
        offers: vec![item_offer(1.0, "divine")],
        total: 12,
        first_page: 1,
        rest: Rest::new(Language::English, "Q2", &["x"]),
        next: NextOffers::NotAwaited,
        searched_by: SearchedBy::default(),
        searched: SystemTime::UNIX_EPOCH,
    };
    assert_eq!(More::of_items(&items), More::Chevron);

    items.next = NextOffers::Loading;
    assert_eq!(More::of_items(&items), More::Loading);

    items.next = NextOffers::Failed(Failure::Unreachable);
    assert_eq!(More::of_items(&items), More::Chevron, "to ask them again");

    items.next = NextOffers::Loading;

    items.rest = Rest::new(Language::English, "Q2", &[]);
    assert_eq!(More::of_items(&items), More::Loading, "the last ones come");

    items.next = NextOffers::NotAwaited;
    assert_eq!(More::of_items(&items), More::None);
}

#[test]
fn the_offers_of_the_exchange_show_the_chevron_until_all_are_shown() {
    let offers = vec![exchange_offer(); 3];
    let rows = exchange_rows(&offers, None, SystemTime::UNIX_EPOCH);

    let first = List::of_exchange(&rows[..2], &offers, 3, SystemTime::UNIX_EPOCH);
    let all = List::of_exchange(&rows, &offers, 3, SystemTime::UNIX_EPOCH);

    assert_eq!(first.more, More::Chevron);
    assert_eq!(all.more, More::None);
}
