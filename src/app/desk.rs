//! The item shown and what the player did with it: its filters, its offers, the items copied
//! before it, and the drawing of its blocks.

use std::time::{Instant, SystemTime};

use crate::item::CopiedItem;
use crate::offers_state::{LONG_WAIT, Offers};
use crate::trade::query::SearchKind;
use crate::ui::TradeStatus;

/// The height of the window over the game for an item that can't be read, in points.
pub(super) const EMPTY_HEIGHT: f32 = 160.0;

/// The item shown came back with the shape `back` of its modifiers when it was left, and has the
/// shape `shown` now: read again with the names of the trade site arrived since, its lines moved.
pub(super) fn lines_moved(back: Option<&[usize]>, shown: Option<&[usize]>) -> bool {
    back.is_some_and(|back| shown != Some(back))
}

/// `item` copied by the hotkey again: the item already shown, which stays as it is. Copied
/// with Ctrl+C after another text, it is read again with the latest known price, its filters
/// and offers kept.
pub(super) fn copied_again(by_hotkey: bool, last: Option<&CopiedItem>, item: &CopiedItem) -> bool {
    by_hotkey && last == Some(item)
}

/// The state of the trade site in the status bar, at `now`: the long wait before the next
/// search of `kind`, or else the age of the last search.
pub(super) fn trade_status(
    offers: &Offers,
    kind: SearchKind,
    now: Instant,
    clock: SystemTime,
) -> TradeStatus {
    match (offers.wait(kind, now), offers.last_search()) {
        (wait, _) if wait > LONG_WAIT => TradeStatus::Waiting(wait),
        (_, Some(searched)) => {
            TradeStatus::Searched(clock.duration_since(searched).unwrap_or_default())
        }
        (_, None) => TradeStatus::Idle,
    }
}

#[cfg(test)]
mod smoke_tests;

#[cfg(test)]
mod tests;
