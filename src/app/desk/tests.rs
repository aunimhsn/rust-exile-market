use std::time::Duration;

use super::*;
use crate::league::League;
use crate::trade::search::Search;

#[test]
fn the_hotkey_on_the_item_shown_keeps_it_as_it_is() {
    let ring = CopiedItem::from_text("Item Class: Rings\nRarity: Rare\nWoe Loop\nRuby Ring")
        .expect("an item");
    let amulet = CopiedItem::from_text("Item Class: Amulets\nRarity: Rare\nDoom Idol\nGold Amulet")
        .expect("an item");

    assert!(copied_again(true, Some(&ring), &ring));
    assert!(!copied_again(true, Some(&ring), &amulet));
    assert!(!copied_again(true, None, &ring));
    // Copied with Ctrl+C after another text: read again.
    assert!(!copied_again(false, Some(&ring), &ring));
}

#[test]
fn an_item_that_comes_back_forgets_its_stat_filters_only_when_its_lines_moved() {
    // The hybrid of a magic mace joined by the names arrived since it was left.
    assert!(lines_moved(Some(&[1, 1, 1]), Some(&[2, 1])));
    assert!(!lines_moved(Some(&[2, 1]), Some(&[2, 1])));
    // An item not remembered, or the item shown copied again.
    assert!(!lines_moved(None, Some(&[2, 1])));
}

#[test]
fn the_status_bar_tells_a_long_wait_rather_than_the_age_of_the_last_search() {
    use crate::trade::query::Query;
    use crate::trade::search::{Answer, NextSearch, Outcome};

    let now = Instant::now();
    let clock = SystemTime::UNIX_EPOCH + Duration::from_secs(600);
    let mut desk = Desk::new(None);
    assert_eq!(
        desk.trade_status(SearchKind::Items, now, clock),
        TradeStatus::Idle
    );
    let search = Search {
        language: crate::language::Language::English,
        league: League::new("Standard"),
        query: Query::Exchange("divine".to_owned()),
    };
    let _ = desk.offers.search(search, now);
    let answer = |seconds| Answer {
        outcome: Outcome::Exchange {
            offers: Vec::new(),
            total: 0,
        },
        next_search: NextSearch {
            items: now + Duration::from_secs(seconds),
            exchange: now + Duration::from_secs(10),
        },
    };
    desk.take_answer(
        answer(240),
        SystemTime::UNIX_EPOCH + Duration::from_secs(480),
    );

    assert_eq!(
        desk.trade_status(SearchKind::Items, now, clock),
        TradeStatus::Waiting(Duration::from_secs(240))
    );
    assert_eq!(
        desk.trade_status(SearchKind::Exchange, now, clock),
        TradeStatus::Searched(Duration::from_secs(120)),
        "the usual 10 s aren't told"
    );
}
