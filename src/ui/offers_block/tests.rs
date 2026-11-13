use super::*;
use crate::trade::offers::fixtures::item_offer;

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
