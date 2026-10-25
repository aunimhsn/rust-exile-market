//! What the window shows, built from what happened: the state of the application, without
//! the interface.

use std::time::{Duration, SystemTime};

use crate::clipboard::ClipboardEvent;
use crate::item::{CopiedItem, Item, ParseError, Rarity};
use crate::item_names::ItemNames;
use crate::league::{League, Leagues};
use crate::market::{MarketEvent, Source};
use crate::prices::PriceList;
use crate::pricing::{self, FixedPrice};

/// The last item copied in the game, and what the parser read from it.
#[derive(Debug)]
pub struct ShownItem {
    copied: CopiedItem,
    parsed: Result<Item, ParseError>,
}

impl ShownItem {
    fn new(copied: CopiedItem) -> Self {
        let parsed = Item::parse(&copied);
        match &parsed {
            Ok(item) => {
                log::debug!("{item:#?}");
                for warning in item.warnings() {
                    log::warn!("A part of a copied item couldn't be read: {warning}");
                }
            }
            Err(error) => log::warn!("A copied item couldn't be read: {error}"),
        }
        Self { copied, parsed }
    }

    #[must_use]
    pub fn copied(&self) -> &CopiedItem {
        &self.copied
    }

    pub fn parsed(&self) -> Result<&Item, ParseError> {
        self.parsed.as_ref().map_err(|&error| error)
    }
}

#[derive(Debug)]
pub struct Session {
    /// The leagues of the price database, once they arrived.
    leagues: Option<Leagues>,
    /// The league shown: the one of the last session until the leagues arrive.
    league: Option<League>,
    /// The new league offered to a player on a permanent league.
    new_league: Option<League>,
    /// The prices of the league shown, from the cache or the price database.
    prices: Option<LeaguePrices>,
    /// The last request for the leagues succeeded.
    leagues_reachable: bool,
    /// The league whose last download of the prices failed, if any.
    prices_failed: Option<League>,
    /// The names of the items of the trade site, which find the prices of a French item.
    item_names: Option<ItemNames>,
    /// Shown until another item is copied.
    item: Option<ShownItem>,
    clipboard_readable: bool,
}

/// The prices of a league, complete or still downloading.
#[derive(Debug)]
struct LeaguePrices {
    list: PriceList,
    /// The share downloaded, from 0 to 1, while a part of the prices is known; `None` once
    /// they are complete.
    downloading: Option<f32>,
}

/// The state of the prices of the league shown.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PricesState {
    /// Not complete yet: `progress` of them are downloaded (from 0 to 1), or the league isn't
    /// known yet.
    Downloading { progress: f32 },
    /// Not complete, and the price database can't be reached.
    Unavailable,
    /// Given by the price database `age` ago; `offline` while it can't be reached.
    Known { age: Duration, offline: bool },
}

/// What changed in the leagues, for the settings to keep.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeagueChange {
    /// The league shown is another one: chosen by default, or because the last one ended.
    Shown(League),
}

impl Session {
    /// A session showing `league`, the one of the last session, until the leagues arrive.
    #[must_use]
    pub fn new(league: Option<League>) -> Self {
        Self {
            leagues: None,
            league,
            new_league: None,
            prices: None,
            leagues_reachable: true,
            prices_failed: None,
            item_names: None,
            item: None,
            clipboard_readable: true,
        }
    }

    /// Takes in what the price database sent; `closed` are the new leagues whose bubble the
    /// player closed.
    pub fn apply_market(&mut self, event: MarketEvent, closed: &[League]) -> Option<LeagueChange> {
        match event {
            MarketEvent::Leagues(leagues) => {
                self.leagues_reachable = true;
                return self.apply_leagues(leagues, closed);
            }
            MarketEvent::CachedPrices(prices) => self.keep_prices(prices, None),
            MarketEvent::PartialPrices { prices, progress } => {
                self.downloaded(&prices.league);
                self.keep_prices(prices, Some(progress));
            }
            MarketEvent::Prices(prices) => {
                self.downloaded(&prices.league);
                self.keep_prices(prices, None);
            }
            MarketEvent::Unreachable(Source::Leagues) => self.leagues_reachable = false,
            MarketEvent::Unreachable(Source::Prices(league)) => self.prices_failed = Some(league),
        }
        None
    }

    fn apply_leagues(&mut self, leagues: Leagues, closed: &[League]) -> Option<LeagueChange> {
        let choice = leagues.choose(self.league.as_ref());
        if let Some(ended) = &choice.ended {
            log::info!(
                "{} has ended: switched to {}",
                ended.id(),
                choice.league.id()
            );
        }
        let change = (self.league.as_ref() != Some(&choice.league))
            .then(|| LeagueChange::Shown(choice.league.clone()));
        self.new_league = leagues.new_league(&choice.league, closed);
        self.league = Some(choice.league);
        self.leagues = Some(leagues);
        change
    }

    /// The prices of `league` were downloaded: an earlier failure of its download is over.
    fn downloaded(&mut self, league: &League) {
        if self.prices_failed.as_ref() == Some(league) {
            self.prices_failed = None;
        }
    }

    /// `prices` are those of the league shown.
    fn shows(&self, prices: &PriceList) -> bool {
        self.league.as_ref() == Some(&prices.league)
    }

    /// Keeps `prices` if they are those of the league shown, `downloading` while they are
    /// partial: the prices of a league left meanwhile are dropped.
    fn keep_prices(&mut self, list: PriceList, downloading: Option<f32>) {
        if self.shows(&list) {
            self.prices = Some(LeaguePrices { list, downloading });
        }
    }

    /// Shows `league`, chosen by the player.
    pub fn choose_league(&mut self, league: League) {
        self.new_league = None;
        self.league = Some(league);
    }

    /// Closes the bubble of the new league; the league it offered.
    pub fn close_new_league(&mut self) -> Option<League> {
        self.new_league.take()
    }

    /// Takes in the names of the items given by the trade site.
    pub fn apply_item_names(&mut self, names: ItemNames) {
        self.item_names = Some(names);
    }

    pub fn apply(&mut self, event: ClipboardEvent) {
        match event {
            ClipboardEvent::Copied(copied) => {
                let shown = ShownItem::new(copied);
                if let Ok(item) = shown.parsed() {
                    self.log_price(item);
                }
                self.item = Some(shown);
            }
            ClipboardEvent::Unreadable => self.clipboard_readable = false,
            ClipboardEvent::Readable => self.clipboard_readable = true,
        }
    }

    /// The league shown; none before the leagues arrive at the first session.
    #[must_use]
    pub fn league(&self) -> Option<&League> {
        self.league.as_ref()
    }

    #[must_use]
    pub fn leagues(&self) -> Option<&Leagues> {
        self.leagues.as_ref()
    }

    /// Logs what the prices say of `item`, until the price block shows it.
    fn log_price(&self, item: &Item) {
        let names = self.item_names.as_ref();
        if item.rarity() == Rarity::Magic
            && let Some(base_type) =
                names.and_then(|names| names.magic_base_type(item.language(), item.base_type()))
        {
            log::info!("Base type of the magic item: {base_type}");
        }
        let Some(prices) = self.league_prices() else {
            return;
        };
        match pricing::fixed_price(item, &prices.list, names) {
            Some(FixedPrice::Currency(currency)) => {
                log::info!(
                    "Price of {}: {:.2} ex",
                    currency.name,
                    currency.price.exalted
                );
            }
            Some(FixedPrice::Unique(unique)) => {
                log::info!("Price of {}: {:.2} ex", unique.name, unique.price.exalted);
            }
            Some(FixedPrice::Uniques(uniques)) => {
                log::info!(
                    "An unidentified unique, one of {} priced uniques",
                    uniques.len()
                );
            }
            None if matches!(
                item.rarity(),
                Rarity::Currency | Rarity::Gem | Rarity::Unique
            ) =>
            {
                log::info!("No price found for this item");
            }
            None => {}
        }
    }

    /// The prices of the league shown, complete or still downloading.
    fn league_prices(&self) -> Option<&LeaguePrices> {
        self.prices
            .as_ref()
            .filter(|prices| self.shows(&prices.list))
    }

    /// The state of the prices of the league shown, at `now`.
    #[must_use]
    pub fn prices_state(&self, now: SystemTime) -> PricesState {
        // A failed download of another league says nothing of this one.
        let prices_failed = self.prices_failed.is_some() && self.prices_failed == self.league;
        let offline = !self.leagues_reachable || prices_failed;
        match self.league_prices() {
            Some(LeaguePrices {
                list,
                downloading: None,
            }) => PricesState::Known {
                age: list.age(now),
                offline,
            },
            _ if offline => PricesState::Unavailable,
            prices => PricesState::Downloading {
                progress: prices.and_then(|prices| prices.downloading).unwrap_or(0.0),
            },
        }
    }

    #[must_use]
    pub fn new_league(&self) -> Option<&League> {
        self.new_league.as_ref()
    }

    #[must_use]
    pub fn item(&self) -> Option<&ShownItem> {
        self.item.as_ref()
    }

    #[must_use]
    pub fn clipboard_readable(&self) -> bool {
        self.clipboard_readable
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::league::LeagueMode;
    use crate::prices::Rates;

    fn leagues(ids: &[&str]) -> MarketEvent {
        MarketEvent::Leagues(Leagues::new(ids.iter().map(|&id| League::new(id))))
    }

    const RING: &str = "Item Class: Rings\nRarity: Rare\nWoe Loop\nRuby Ring";
    const AMULET: &str = "Item Class: Amulets\nRarity: Magic\nJade Amulet";

    fn session() -> Session {
        Session::new(Some(League::new("Forbidden Rites")))
    }

    fn copy(session: &mut Session, text: &str) {
        let copied = CopiedItem::from_text(text).expect("an item");
        session.apply(ClipboardEvent::Copied(copied));
    }

    fn shown_text(session: &Session) -> Option<&str> {
        session.item().map(|item| item.copied().text())
    }

    #[test]
    fn shows_the_last_copied_item_read_by_the_parser() {
        let mut session = session();
        assert!(session.item().is_none());

        copy(&mut session, RING);
        copy(&mut session, AMULET);

        assert_eq!(shown_text(&session), Some(AMULET));
        let parsed = session.item().map(ShownItem::parsed);
        assert!(
            parsed.is_some_and(|item| item.is_ok_and(|item| item.base_type() == "Jade Amulet"))
        );
    }

    #[test]
    fn keeps_an_item_that_can_t_be_read_with_its_error() {
        let mut session = session();

        copy(&mut session, "Item Class: Rings\nWoe Loop");

        assert_eq!(
            session.item().map(|item| item.parsed().err()),
            Some(Some(ParseError::MissingRarity))
        );
    }

    #[test]
    fn keeps_the_item_while_the_clipboard_is_unreadable() {
        let mut session = session();
        copy(&mut session, RING);

        session.apply(ClipboardEvent::Unreadable);
        assert!(!session.clipboard_readable());
        assert_eq!(shown_text(&session), Some(RING));

        session.apply(ClipboardEvent::Readable);
        assert!(session.clipboard_readable());
    }

    #[test]
    fn the_first_session_shows_the_league_by_default_and_keeps_it() {
        let mut session = Session::new(None);
        assert_eq!(session.league(), None);

        let change = session.apply_market(leagues(&["Forbidden Rites", "Runes of Aldur"]), &[]);

        let league = League::new("Forbidden Rites");
        assert_eq!(session.league(), Some(&league));
        assert_eq!(change, Some(LeagueChange::Shown(league)));
    }

    #[test]
    fn a_league_that_lasts_changes_nothing() {
        let mut session = session();

        assert_eq!(
            session.apply_market(leagues(&["Forbidden Rites"]), &[]),
            None
        );
        assert_eq!(session.new_league(), None);
    }

    #[test]
    fn an_ended_league_moves_to_standard_and_offers_the_new_one() {
        let mut session = Session::new(Some(League::new("HC Dawn of the Hunt")));

        let change = session.apply_market(leagues(&["Forbidden Rites", "HC Forbidden Rites"]), &[]);

        let hardcore = League::permanent(LeagueMode::Hardcore);
        assert_eq!(change, Some(LeagueChange::Shown(hardcore.clone())));
        assert_eq!(session.league(), Some(&hardcore));
        assert_eq!(
            session.new_league(),
            Some(&League::new("HC Forbidden Rites"))
        );
    }

    #[test]
    fn a_closed_bubble_doesnt_come_back() {
        let mut session = Session::new(Some(League::permanent(LeagueMode::Softcore)));
        session.apply_market(leagues(&["Forbidden Rites"]), &[]);

        assert_eq!(
            session.close_new_league(),
            Some(League::new("Forbidden Rites"))
        );
        session.apply_market(
            leagues(&["Forbidden Rites"]),
            &[League::new("Forbidden Rites")],
        );

        assert_eq!(session.new_league(), None);
    }

    #[test]
    fn choosing_a_league_closes_the_bubble() {
        let mut session = Session::new(Some(League::permanent(LeagueMode::Softcore)));
        session.apply_market(leagues(&["Forbidden Rites"]), &[]);

        session.choose_league(League::new("Forbidden Rites"));

        assert_eq!(session.new_league(), None);
        assert_eq!(session.league(), Some(&League::new("Forbidden Rites")));
    }

    fn prices(league: &str, updated: SystemTime) -> PriceList {
        PriceList {
            league: League::new(league),
            updated,
            rates: Rates {
                chaos: 75.0,
                divine: 750.0,
            },
            uniques: Vec::new(),
            currencies: Vec::new(),
        }
    }

    /// A time of 2026-10-25.
    fn now() -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(1_793_000_000)
    }

    fn ago(minutes: u64) -> SystemTime {
        now() - Duration::from_mins(minutes)
    }

    #[test]
    fn the_prices_are_downloading_until_they_arrive() {
        let mut session = session();
        assert_eq!(
            session.prices_state(now()),
            PricesState::Downloading { progress: 0.0 }
        );

        session.apply_market(
            MarketEvent::Unreachable(Source::Prices(League::new("Forbidden Rites"))),
            &[],
        );
        assert_eq!(session.prices_state(now()), PricesState::Unavailable);

        session.apply_market(MarketEvent::Prices(prices("Forbidden Rites", ago(2))), &[]);
        assert_eq!(
            session.prices_state(now()),
            PricesState::Known {
                age: Duration::from_mins(2),
                offline: false
            }
        );
    }

    #[test]
    fn cached_prices_are_offline_while_the_database_cant_be_reached() {
        let mut session = session();

        session.apply_market(MarketEvent::Unreachable(Source::Leagues), &[]);
        session.apply_market(
            MarketEvent::CachedPrices(prices("Forbidden Rites", ago(180))),
            &[],
        );

        assert_eq!(
            session.prices_state(now()),
            PricesState::Known {
                age: Duration::from_hours(3),
                offline: true
            }
        );
    }

    #[test]
    fn the_prices_of_another_league_are_not_shown() {
        let mut session = session();
        session.apply_market(MarketEvent::Prices(prices("Forbidden Rites", now())), &[]);

        session.choose_league(League::new("HC Forbidden Rites"));
        assert!(session.league_prices().is_none());
        // Downloaded before the player left that league.
        session.apply_market(MarketEvent::Prices(prices("Forbidden Rites", now())), &[]);
        assert!(session.league_prices().is_none());

        session.choose_league(League::new("Forbidden Rites"));
        assert!(session.league_prices().is_some());
    }

    #[test]
    fn partial_prices_are_shown_while_downloading() {
        let mut session = session();

        session.apply_market(
            MarketEvent::PartialPrices {
                prices: prices("Forbidden Rites", now()),
                progress: 0.25,
            },
            &[],
        );

        assert!(session.league_prices().is_some());
        assert_eq!(
            session.prices_state(now()),
            PricesState::Downloading { progress: 0.25 }
        );
    }

    #[test]
    fn the_leagues_read_again_dont_hide_a_failing_download() {
        let mut session = session();
        session.apply_market(MarketEvent::Prices(prices("Forbidden Rites", now())), &[]);

        session.apply_market(
            MarketEvent::Unreachable(Source::Prices(League::new("Forbidden Rites"))),
            &[],
        );
        session.apply_market(leagues(&["Forbidden Rites"]), &[]);

        assert!(matches!(
            session.prices_state(now()),
            PricesState::Known { offline: true, .. }
        ));
    }

    #[test]
    fn a_failed_download_of_the_previous_league_says_nothing_of_the_new_one() {
        let mut session = session();

        session.choose_league(League::new("HC Forbidden Rites"));
        // Sent before the thread saw the new league.
        session.apply_market(
            MarketEvent::Unreachable(Source::Prices(League::new("Forbidden Rites"))),
            &[],
        );

        assert_eq!(
            session.prices_state(now()),
            PricesState::Downloading { progress: 0.0 }
        );
    }

    #[test]
    fn a_failed_download_of_an_ended_league_says_nothing_of_standard() {
        let mut session = Session::new(Some(League::new("Dawn of the Hunt")));
        session.apply_market(leagues(&["Forbidden Rites"]), &[]);

        session.apply_market(
            MarketEvent::Unreachable(Source::Prices(League::new("Dawn of the Hunt"))),
            &[],
        );

        assert_eq!(
            session.prices_state(now()),
            PricesState::Downloading { progress: 0.0 }
        );
    }

    #[test]
    fn an_unreachable_database_changes_nothing() {
        let mut session = session();

        assert_eq!(
            session.apply_market(
                MarketEvent::Unreachable(Source::Prices(League::new("Forbidden Rites"))),
                &[]
            ),
            None
        );
        assert_eq!(session.leagues(), None);
    }
}
