//! What the window shows, built from what happened: the state of the application, without
//! the interface.

use std::time::{Duration, SystemTime};

use crate::clipboard::ClipboardEvent;
use crate::game_data::{self, StatSource, TradeGroup, TradeStats};
use crate::item::{CopiedItem, Item, ParseError, Rarity};
use crate::item_names::ItemNames;
use crate::language::Language;
use crate::league::{League, Leagues};
use crate::market::{MarketEvent, Source};
use crate::prices::{PriceList, Rates, UniqueModifiers};
use crate::pricing::{self, FixedPrice, NoFixedPrice};
use crate::roll_difference::CopiedLines;
use crate::trade::link::{self, TradeTarget};
use crate::trade::query::StatFilter;
use crate::trade::search::Search;
use crate::trade::stat_filters::LineStats;

/// What the parser read from the last item copied in the game.
#[derive(Debug)]
pub struct ShownItem {
    parsed: Result<Item, ParseError>,
    /// Its modifier lines, which the offers are compared with; none for an unreadable item.
    lines: CopiedLines,
    /// The stat of each modifier line on the trade site, which searches it.
    stats: LineStats,
}

impl ShownItem {
    /// The item `copied`; `trade` holds the stats of the trade site, once it gave them.
    fn new(copied: &CopiedItem, trade: Option<&TradeStats>) -> Self {
        let parsed = Item::parse(copied);
        let mut lines = CopiedLines::default();
        let mut stats = LineStats::default();
        match &parsed {
            Ok(item) => {
                log::debug!("{item:#?}");
                for warning in item.warnings() {
                    log::warn!("A part of a copied item couldn't be read: {warning}");
                }
                lines = CopiedLines::new(item.language(), item.modifiers(), trade);
                stats = LineStats::new(item.language(), item.modifiers(), trade);
            }
            Err(error) => log::warn!("A copied item couldn't be read: {error}"),
        }
        Self {
            parsed,
            lines,
            stats,
        }
    }

    pub fn parsed(&self) -> Result<&Item, ParseError> {
        self.parsed.as_ref().map_err(|&error| error)
    }

    #[must_use]
    pub fn lines(&self) -> &CopiedLines {
        &self.lines
    }

    #[must_use]
    pub fn stats(&self) -> &LineStats {
        &self.stats
    }
}

/// The modifiers of the unique `item` in the price database, from its `fixed_price`, for the
/// ranges of its rolls; none for an item copied in French, whose lines aren't in English.
#[must_use]
pub fn unique_modifiers<'a>(
    item: &Item,
    fixed_price: &Result<(FixedPrice<'a>, Rates), NoFixedPrice>,
) -> Option<&'a UniqueModifiers> {
    match fixed_price {
        Ok((FixedPrice::Unique(unique), _)) if item.language() == Language::English => {
            Some(&unique.modifiers)
        }
        _ => None,
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
    /// The stats of the trade site, from the templates given with the names: the stats of
    /// a modifier line the game data doesn't know yet.
    trade_stats: Option<TradeStats>,
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
            trade_stats: None,
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
    pub fn apply_item_names(&mut self, mut names: ItemNames) {
        // The names are already in the cache, with their templates.
        let (english, french) = names.take_stat_templates();
        self.trade_stats = Some(TradeStats::new(&english, &french));
        self.item_names = Some(names);
    }

    pub fn apply(&mut self, event: ClipboardEvent) {
        match event {
            ClipboardEvent::Copied(copied) => {
                let trade = self.trade_stats.as_ref();
                let shown = ShownItem::new(&copied, trade);
                if let Ok(item) = shown.parsed() {
                    self.log_price(item);
                    log_unknown_stats(item, trade);
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

    /// Logs what the prices say of `item` when it is copied.
    fn log_price(&self, item: &Item) {
        let names = self.item_names.as_ref();
        if item.rarity() == Rarity::Magic
            && let Some(base_type) =
                names.and_then(|names| names.magic_base_type(item.language(), item.base_type()))
        {
            log::info!("Base type of the magic item: {base_type}");
        }
        match self.fixed_price(item).map(|(price, _)| price) {
            Ok(FixedPrice::Currency(currency)) => {
                log::info!(
                    "Price of {}: {:.2} ex",
                    currency.name,
                    currency.price.exalted
                );
            }
            Ok(FixedPrice::Unique(unique)) => {
                log::info!("Price of {}: {:.2} ex", unique.name, unique.price.exalted);
            }
            Ok(FixedPrice::Uniques(candidates)) => {
                let with_price = candidates.iter().filter(|unique| unique.price.is_some());
                log::info!(
                    "An unidentified unique, one of {} uniques ({} priced)",
                    candidates.len(),
                    with_price.count()
                );
            }
            Err(NoFixedPrice::NotPriced) => log::info!("No price found for this item"),
            Err(NoFixedPrice::PricesUnknown) => log::info!("No price yet: the prices aren't known"),
            Err(NoFixedPrice::NamesUnknown) => {
                log::info!("No price yet: the item names of the trade site aren't known");
            }
            Err(NoFixedPrice::NotFixedName) => {}
        }
    }

    /// The fixed price of `item` among the prices of the league shown known so far, with the
    /// rates of the currencies of these prices.
    pub fn fixed_price(&self, item: &Item) -> Result<(FixedPrice<'_>, Rates), NoFixedPrice> {
        let prices = self.league_prices().map(|prices| &prices.list);
        let price = pricing::fixed_price(item, prices, self.item_names.as_ref())?;
        // A price comes from the prices, which have their rates.
        let rates = prices.ok_or(NoFixedPrice::PricesUnknown)?.rates;
        Ok((price, rates))
    }

    /// What the search for `item` on the trade site looks for; none for a magic item whose
    /// base type isn't known yet.
    fn trade_target<'a>(&'a self, item: &'a Item) -> Option<TradeTarget<'a>> {
        let prices = self.league_prices().map(|prices| &prices.list);
        link::target(item, self.item_names.as_ref(), prices)
    }

    /// The search for `item` on the trade site with the filters of its `stats`, in the league
    /// shown; none before the league is known, and for an unidentified unique, which is
    /// identified to be searched.
    #[must_use]
    pub fn trade_search(&self, item: &Item, stats: Vec<StatFilter>) -> Option<Search> {
        let league = self.league.as_ref()?;
        let target = self.trade_target(item)?;
        if let TradeTarget::UnidentifiedUnique { .. } = target {
            return None;
        }
        Some(Search {
            language: item.language(),
            league: league.clone(),
            query: link::query(item, &target, stats),
        })
    }

    /// An unidentified unique, whose offers are searched once it is identified.
    #[must_use]
    pub fn must_identify(&self, item: &Item) -> bool {
        matches!(
            self.trade_target(item),
            Some(TradeTarget::UnidentifiedUnique { .. })
        )
    }

    /// The link of the search for `item` on the trade site with the filters of its `stats`,
    /// in the league shown; none before the league is known.
    #[must_use]
    pub fn trade_url(&self, item: &Item, stats: Vec<StatFilter>) -> Option<String> {
        let league = self.league.as_ref()?;
        let target = self.trade_target(item)?;
        Some(link::url(
            item.language(),
            league,
            &link::query(item, &target, stats),
        ))
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

/// Logs the modifier lines of `item` whose stat isn't known, has no trade id in their group,
/// or is known by the trade site only: a modifier added by a patch of the game, or a line the
/// parser read wrong.
fn log_unknown_stats(item: &Item, trade: Option<&TradeStats>) {
    for modifier in item.modifiers() {
        let group = TradeGroup::of(modifier);
        for found in game_data::modifier_stats(item.language(), modifier, trade) {
            let lines = modifier.lines.get(found.lines).unwrap_or_default();
            let text = lines.iter().map(|line| line.text.as_str());
            let text = text.collect::<Vec<_>>().join(" / ");
            match found.stat {
                None => log::warn!("Stat not found: {text}"),
                Some(stat) if stat.stat.trade_ids(group).is_empty() => {
                    log::warn!("No {group:?} trade id for the stat: {text}");
                }
                Some(stat) if stat.source == StatSource::TradeSite => {
                    log::info!("Stat known by the trade site only: {text}");
                }
                Some(_) => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::league::LeagueMode;
    use crate::prices::{Price, Rates, UniquePrice};

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

    /// The base type of the item shown.
    fn shown_base(session: &Session) -> Option<&str> {
        let item = session.item()?.parsed().ok()?;
        Some(item.base_type())
    }

    #[test]
    fn shows_the_last_copied_item_read_by_the_parser() {
        let mut session = session();
        assert!(session.item().is_none());

        copy(&mut session, RING);
        copy(&mut session, AMULET);

        assert_eq!(shown_base(&session), Some("Jade Amulet"));
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
        assert_eq!(shown_base(&session), Some("Ruby Ring"));

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
    fn only_an_item_copied_in_english_gets_the_modifiers_of_its_unique() {
        let unique = UniquePrice {
            name: "The Eternal Spark".to_owned(),
            base_type: "Crystal Focus".to_owned(),
            price: Price {
                exalted: 1.0,
                quantity: 10,
                history: Vec::new(),
            },
            modifiers: UniqueModifiers {
                implicit: Vec::new(),
                explicit: vec!["(50-70)% increased Energy Shield".to_owned()],
            },
        };
        let rates = prices("Forbidden Rites", now()).rates;
        let fixed_price = Ok((FixedPrice::Unique(&unique), rates));
        let parse =
            |text: &str| Item::parse(&CopiedItem::from_text(text).expect("an item")).expect("read");
        let english = parse(include_str!("item/test-items/en-unique-focus.txt"));
        let french = parse(include_str!("item/test-items/fr-unique-crossbow.txt"));

        assert_eq!(
            unique_modifiers(&english, &fixed_price),
            Some(&unique.modifiers)
        );
        // Its lines aren't in English, like the modifiers of the price database.
        assert_eq!(unique_modifiers(&french, &fixed_price), None);
        assert_eq!(
            unique_modifiers(&english, &Err(NoFixedPrice::PricesUnknown)),
            None
        );
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

    #[test]
    fn the_offers_are_searched_in_the_league_shown_once_the_item_is_identified() {
        const UNIQUE: &str = "Item Class: Rings\nRarity: Unique\nAndvarius\nGold Ring";
        const UNIDENTIFIED: &str =
            "Item Class: Rings\nRarity: Unique\nGold Ring\n--------\nUnidentified";
        let parsed =
            |session: &Session| -> Item { session.item().unwrap().parsed().unwrap().clone() };
        let mut unknown_league = Session::new(None);
        copy(&mut unknown_league, UNIQUE);
        assert_eq!(
            unknown_league.trade_search(&parsed(&unknown_league), Vec::new()),
            None,
            "the league isn't known yet"
        );

        let mut session = session();
        copy(&mut session, UNIQUE);
        let unique = parsed(&session);
        let search = session.trade_search(&unique, Vec::new()).expect("a search");
        assert_eq!(search.league, League::new("Forbidden Rites"));
        assert_eq!(search.language, Language::English);
        assert!(!session.must_identify(&unique));

        copy(&mut session, UNIDENTIFIED);
        let unidentified = parsed(&session);
        assert_eq!(session.trade_search(&unidentified, Vec::new()), None);
        assert!(session.must_identify(&unidentified));
    }
}
