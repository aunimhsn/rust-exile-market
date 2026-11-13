//! The item shown and what the player did with it: its filters, its offers, the items copied
//! before it, and the drawing of its blocks. Without egui state nor thread, so that a test
//! builds it.

use std::time::{Instant, SystemTime};

use egui::Ui;

use super::item_view::{ItemView, search_filters};
use crate::clipboard::ClipboardEvent;
use crate::filters::{Filters, Window};
use crate::i18n::Translator;
use crate::item::CopiedItem;
use crate::item_names::ItemNames;
use crate::league::League;
use crate::market::MarketEvent;
use crate::offers_state::{LONG_WAIT, Offers};
use crate::recent_items::RecentItems;
use crate::session::{LeagueChange, PricesState, Session, ShownItem};
use crate::trade::query::SearchKind;
use crate::trade::search::{Answer, Order, Search};
use crate::ui::{self, ItemBlockAction, OffersAction, TradeStatus, UnreadableItem};

/// What a window draws the item shown with.
#[derive(Debug, Clone, Copy)]
pub(super) struct Surroundings {
    pub(super) prices: PricesState,
    /// The kind of the search of its offers, when it can be searched.
    pub(super) kind: Option<SearchKind>,
    /// An item was copied: the content scrolls back to the top, to its name and its price.
    pub(super) to_top: bool,
    pub(super) window: Window,
}

/// What the drawing of the item shown asks of the application.
#[derive(Debug, PartialEq)]
pub(super) enum DeskRequest {
    /// The order to send to the thread of the searches.
    Search(Order),
    /// The bug report, which the hint of a failed search opens.
    BugReport,
}

/// The item shown, drawn.
#[derive(Debug, PartialEq)]
pub(super) struct DrawnItem {
    /// The height of its blocks, which sizes the window over the game, in points.
    pub(super) height: f32,
    pub(super) request: Option<DeskRequest>,
}

/// The item shown and what the player did with it: the session that reads it, its filters
/// and its offers, and the items copied before it, as the player left them.
pub(super) struct Desk {
    session: Session,
    /// The item shown, the last one copied, and the items copied before it, with their filters
    /// and offers as the player left them.
    recent: RecentItems,
    /// The filters of the search as the player set them, for the item shown: those of another
    /// item copied are its own, by default or as the player left them.
    filters: Filters,
    /// The offers of the item shown, and the waiting time before the next search.
    offers: Offers,
    /// The stats of the item are unfolded, for every item copied until the player folds them.
    stats_open: bool,
    /// An item was copied: the content scrolls back to the top, to its name and its price.
    scroll_to_top: bool,
}

impl Desk {
    /// A desk without an item, showing `league`, the one of the last session, until the
    /// leagues arrive.
    pub(super) fn new(league: Option<League>) -> Self {
        Self {
            session: Session::new(league),
            recent: RecentItems::default(),
            filters: Filters::default(),
            offers: Offers::default(),
            stats_open: false,
            scroll_to_top: false,
        }
    }

    pub(super) fn session(&self) -> &Session {
        &self.session
    }

    /// The last item copied, as it was copied.
    pub(super) fn copied(&self) -> Option<&CopiedItem> {
        self.recent.shown()
    }

    /// Takes in an event of the clipboard watching. The item shown, copied again, keeps its
    /// filters and offers; another item copied leaves them to the items remembered, and shows
    /// its own.
    pub(super) fn take_clipboard_event(&mut self, event: ClipboardEvent) {
        let mut back = None;
        if let ClipboardEvent::Copied { item, hotkey } = &event {
            // The clipboard watching alone tells the copy of the hotkey.
            if copied_again(hotkey.is_some(), self.recent.shown(), item) {
                // Its filters and offers stay.
                return;
            }
            self.scroll_to_top = true;
            let shape = self
                .session
                .item()
                .map(ShownItem::shape)
                .unwrap_or_default();
            back = self
                .recent
                .show(item, shape, &mut self.filters, &mut self.offers);
        }
        self.session.apply(event);
        // Read again with the names of the trade site arrived since it was left, its lines may
        // have moved, the filters set on them no longer falling on the same lines.
        let shown = self.session.item().map(ShownItem::shape);
        if lines_moved(back.as_deref(), shown.as_deref()) {
            self.filters.forget_stats();
        }
    }

    /// Whether an item was copied since the last call: the content then scrolls back to the
    /// top, in both windows.
    pub(super) fn take_scroll_to_top(&mut self) -> bool {
        std::mem::take(&mut self.scroll_to_top)
    }

    /// Takes in an event of the price database; the league shown, when it is another one, its
    /// offers forgotten.
    pub(super) fn apply_market(
        &mut self,
        event: MarketEvent,
        closed: &[League],
    ) -> Option<LeagueChange> {
        let change = self.session.apply_market(event, closed);
        if let Some(LeagueChange::Shown(_)) = change {
            self.forget_offers();
        }
        change
    }

    /// Takes in the names of the items given by the trade site: the item shown, read again,
    /// forgets the filters of its lines when they moved.
    pub(super) fn apply_item_names(&mut self, names: ItemNames) {
        if self.session.apply_item_names(names) {
            self.filters.forget_stats();
        }
    }

    /// Shows `league`, chosen by the player, its offers forgotten when it is another one.
    pub(super) fn choose_league(&mut self, league: League) {
        if self.session.league() != Some(&league) {
            self.forget_offers();
        }
        self.session.choose_league(league);
    }

    /// Closes the bubble of the new league; the league it offered.
    pub(super) fn close_new_league(&mut self) -> Option<League> {
        self.session.close_new_league()
    }

    /// Forgets the offers of the item shown and of the items remembered: they were searched in
    /// the previous league.
    fn forget_offers(&mut self) {
        self.offers.clear();
        self.recent.forget_offers();
    }

    /// Takes in the answer of a search of the trade site, at `clock`.
    pub(super) fn take_answer(&mut self, answer: Answer, clock: SystemTime) {
        self.offers.apply(answer, clock);
    }

    /// The thread of the searches ended: a search waiting for its answer never gets it.
    pub(super) fn search_thread_lost(&mut self) {
        self.offers.thread_lost();
    }

    /// Settles the number typed in a field of `window`, as leaving it does.
    pub(super) fn settle_edited(&mut self, window: Window) {
        self.filters.settle_edited(window);
    }

    /// No field of `window` is edited any more.
    pub(super) fn forget_edited(&mut self, window: Window) {
        self.filters.forget_edited(window);
    }

    /// The kind of the search of the offers of the item shown, when it can be searched.
    pub(super) fn search_kind(&self) -> Option<SearchKind> {
        let item = self.session.item()?.parsed().ok()?;
        self.session.search_kind(item)
    }

    /// The search of the offers of the item shown, with its filters as they are, when it can
    /// be searched.
    pub(super) fn shown_search(&self) -> Option<Search> {
        let shown = self.session.item()?;
        let item = shown.parsed().ok()?;
        let filters = search_filters(&self.session, shown, item, &self.filters);
        self.session.trade_search(item, filters)
    }

    /// The state of the trade site in the status bar, at `now`: the long wait before the next
    /// search of `kind`, or else the age of the last search.
    pub(super) fn trade_status(
        &self,
        kind: SearchKind,
        now: Instant,
        clock: SystemTime,
    ) -> TradeStatus {
        match (self.offers.wait(kind, now), self.offers.last_search()) {
            (wait, _) if wait > LONG_WAIT => TradeStatus::Waiting(wait),
            (_, Some(searched)) => {
                TradeStatus::Searched(clock.duration_since(searched).unwrap_or_default())
            }
            (_, None) => TradeStatus::Idle,
        }
    }

    /// Draws the blocks of the item shown, in `around`, and takes in what the player did in
    /// them; none before the first item copied, the window then drawing its empty state.
    pub(super) fn show_item(
        &mut self,
        ui: &mut Ui,
        i18n: &Translator,
        around: Surroundings,
    ) -> Option<DrawnItem> {
        let shown = self.session.item()?;
        let Ok(item) = shown.parsed() else {
            UnreadableItem::new(i18n).show(ui);
            return Some(DrawnItem {
                height: EMPTY_HEIGHT,
                request: None,
            });
        };
        let view = ItemView {
            session: &self.session,
            shown,
            item,
            offers: &self.offers,
            filters: &mut self.filters,
            stats_open: self.stats_open,
            prices: around.prices,
            kind: around.kind,
            has_bug_report: around.window == Window::Main,
            i18n,
        };
        let ((action, offers_action), height) = ui::content(ui, around.to_top, |ui| view.show(ui));
        if action == Some(ItemBlockAction::ToggleStats) {
            self.stats_open = !self.stats_open;
        }
        let request = match offers_action {
            // Built at the click, with the filters as the player just left them.
            Some(OffersAction::Search) => self
                .shown_search()
                .and_then(|search| self.offers.search(search, Instant::now()))
                .map(DeskRequest::Search),
            Some(OffersAction::More) => self.offers.more().map(DeskRequest::Search),
            Some(OffersAction::ReportBug) => Some(DeskRequest::BugReport),
            None => None,
        };
        Some(DrawnItem { height, request })
    }
}

/// The height of the window over the game for the empty state and for an item that can't be
/// read, in points.
pub(super) const EMPTY_HEIGHT: f32 = 160.0;

/// The item shown came back with the shape `back` of its modifiers when it was left, and has the
/// shape `shown` now: read again with the names of the trade site arrived since, its lines moved.
fn lines_moved(back: Option<&[usize]>, shown: Option<&[usize]>) -> bool {
    back.is_some_and(|back| shown != Some(back))
}

/// `item` copied by the hotkey again: the item already shown, which stays as it is. Copied
/// with Ctrl+C after another text, it is read again with the latest known price, its filters
/// and offers kept.
fn copied_again(by_hotkey: bool, last: Option<&CopiedItem>, item: &CopiedItem) -> bool {
    by_hotkey && last == Some(item)
}

#[cfg(test)]
mod smoke_tests;

#[cfg(test)]
mod tests;
