//! What the window shows, built from what happened: the state of the application, without
//! the interface.

use std::time::{Duration, SystemTime};

use crate::clipboard::ClipboardEvent;
use crate::game_data::{self, StatSource, TradeGroup, TradeStats};
use crate::item::{CopiedItem, Item, ItemState, Modifier, ParseError, Rarity};
use crate::item_names::ItemNames;
use crate::league::{League, Leagues};
use crate::market::{MarketEvent, Source};
use crate::prices::{PriceList, Rates};
use crate::pricing::{self, FixedPrice, NoFixedPrice};
use crate::roll_difference::CopiedLines;
use crate::tier_rolls::TierRolls;
use crate::trade::link::{self, TradeTarget};
use crate::trade::query::{SearchFilters, SearchKind};
use crate::trade::search::Search;
use crate::trade::stat_filters::LineStats;

/// What the parser read from the last item copied in the game.
#[derive(Debug)]
pub struct ShownItem {
    /// For a rare or magic item copied from the in-game trade, with the affixes and tiers found
    /// from the values of its lines once its base is known.
    parsed: Result<Item, ParseError>,
    /// The modifiers of a rare or magic item copied from the in-game trade as the parser read
    /// them, without affix nor tier: each reading starts from them, when the item is copied and
    /// each time the names of the trade site arrive (they tell the base of a magic item, and
    /// their stats the affixes a rune or an implicit allows).
    trade_copy: Option<Vec<Modifier>>,
    /// Its modifier lines, which the offers are compared with; none for an unreadable item.
    lines: CopiedLines,
    /// The stat of each modifier line on the trade site, which searches it.
    stats: LineStats,
    /// The rolls of each line of a prefix or suffix over every tier of its modifier.
    tiers: TierRolls,
}

impl ShownItem {
    /// The item `copied`; `trade` holds the stats of the trade site and `names` the names of
    /// its items, once it gave them.
    fn new(copied: &CopiedItem, trade: Option<&TradeStats>, names: Option<&ItemNames>) -> Self {
        // An item that can't be read shows its message in the window.
        let parsed = Item::parse(copied);
        match &parsed {
            Ok(item) => {
                log::debug!("{item:#?}");
                for warning in item.warnings() {
                    log::warn!("A part of a copied item couldn't be read: {warning}");
                }
            }
            Err(error) => log::warn!("A copied item couldn't be read: {error}"),
        }
        let trade_copy = parsed
            .as_ref()
            .ok()
            .filter(|item| item.has_tiers_from_values())
            .map(|item| item.modifiers().to_vec());
        let mut shown = Self {
            parsed,
            trade_copy,
            lines: CopiedLines::default(),
            stats: LineStats::default(),
            tiers: TierRolls::default(),
        };
        shown.read_lines(trade, names);
        shown
    }

    /// Reads the modifier lines of the item, with the stats of the trade site `trade`, and
    /// their tiers on its base, found among the `names` of the trade site for a magic item;
    /// from their values for an item copied from the in-game trade, from its modifiers as the
    /// parser read them. True when its modifiers changed shape (the lines of a hybrid joined
    /// once the names tell the base of a magic item): the keys of their filters moved.
    fn read_lines(&mut self, trade: Option<&TradeStats>, names: Option<&ItemNames>) -> bool {
        let Ok(item) = &mut self.parsed else {
            return false;
        };
        let base_type = link::target(item, names, None)
            .and_then(|target| target.base_type())
            .map(str::to_owned);
        let mut reshaped = false;
        if let Some(copied) = &self.trade_copy {
            let modifiers = match &base_type {
                Some(base_type) => {
                    // The runes and the implicits, which tell the room, are never read again.
                    let stats = LineStats::new(item.language(), copied, trade);
                    game_data::with_trade_tiers(
                        item.language(),
                        base_type,
                        item.item_level(),
                        copied,
                        stats.affix_limits(item),
                        item.has_state(ItemState::Sanctified),
                    )
                }
                None => copied.clone(),
            };
            reshaped = shape(item.modifiers()) != shape(&modifiers);
            item.set_modifiers(modifiers);
        }
        self.stats = LineStats::new(item.language(), item.modifiers(), trade);
        self.lines = CopiedLines::new(item.language(), item.modifiers(), &self.stats, trade);
        self.tiers = TierRolls::new(item.language(), item.modifiers(), base_type.as_deref());
        reshaped
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

    #[must_use]
    pub fn tiers(&self) -> &TierRolls {
        &self.tiers
    }
}

/// The number of lines of each modifier: the filter of a line is kept by its place.
fn shape(modifiers: &[Modifier]) -> Vec<usize> {
    modifiers
        .iter()
        .map(|modifier| modifier.lines.len())
        .collect()
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

    /// Takes in the names of the items given by the trade site. True when the modifiers of the
    /// item shown changed shape, read again with them: the filters the player changed on its
    /// lines no longer fall on the same lines.
    #[must_use]
    pub fn apply_item_names(&mut self, mut names: ItemNames) -> bool {
        // The names are already in the cache, with their templates.
        let (english, french) = names.take_stat_templates();
        let trade = TradeStats::new(&english, &french);
        self.item_names = Some(names);
        // An item copied before them may have a line only the trade site knows, or be a
        // magic item whose base type they tell.
        let reshaped = self
            .item
            .as_mut()
            .is_some_and(|shown| shown.read_lines(Some(&trade), self.item_names.as_ref()));
        self.trade_stats = Some(trade);
        reshaped
    }

    pub fn apply(&mut self, event: ClipboardEvent) {
        match event {
            ClipboardEvent::Copied(copied) => {
                let trade = self.trade_stats.as_ref();
                let shown = ShownItem::new(&copied, trade, self.item_names.as_ref());
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

    /// The rates of the currencies of the prices of the league shown, once known.
    #[must_use]
    pub fn rates(&self) -> Option<Rates> {
        self.league_prices().map(|prices| prices.list.rates)
    }

    /// What the search for `item` on the trade site looks for; none for a magic item whose
    /// base type isn't known yet.
    fn trade_target<'a>(&'a self, item: &'a Item) -> Option<TradeTarget<'a>> {
        let prices = self.league_prices().map(|prices| &prices.list);
        link::target(item, self.item_names.as_ref(), prices)
    }

    /// The base type of `item` among its filters, which its search looks for unless the
    /// player unchecks it: for a rare, magic or normal item, or a jewel; none for a magic item
    /// whose base type isn't known yet.
    #[must_use]
    pub fn filtered_base_type<'a>(&'a self, item: &'a Item) -> Option<&'a str> {
        if !item.rarity().has_item_filters() {
            return None;
        }
        self.trade_target(item)?.base_type()
    }

    /// The search for `item` on the trade site with its `filters`, in the league shown; none
    /// before the league is known, and for an unidentified unique, which is identified to be
    /// searched.
    #[must_use]
    pub fn trade_search(&self, item: &Item, filters: SearchFilters) -> Option<Search> {
        let league = self.league.as_ref()?;
        let target = self.searched_target(item)?;
        Some(Search {
            language: item.language(),
            league: league.clone(),
            query: link::query(item, &target, filters),
        })
    }

    /// The kind of the search of `item`, without building it; none when it has no search
    /// ([`Self::trade_search`]).
    #[must_use]
    pub fn search_kind(&self, item: &Item) -> Option<SearchKind> {
        self.league.as_ref()?;
        Some(self.searched_target(item)?.query().kind())
    }

    /// What the search of `item` looks for; none for an unidentified unique, which is
    /// identified to be searched.
    fn searched_target<'a>(&'a self, item: &'a Item) -> Option<TradeTarget<'a>> {
        self.trade_target(item)
            .filter(|target| !matches!(target, TradeTarget::UnidentifiedUnique { .. }))
    }

    /// The link of the search for `item` can be opened: the league and its target known.
    #[must_use]
    pub fn tradable(&self, item: &Item) -> bool {
        self.league.is_some() && self.trade_target(item).is_some()
    }

    /// An unidentified unique, whose offers are searched once it is identified.
    #[must_use]
    pub fn must_identify(&self, item: &Item) -> bool {
        matches!(
            self.trade_target(item),
            Some(TradeTarget::UnidentifiedUnique { .. })
        )
    }

    /// The link of the search for `item` on the trade site with its `filters`, in the league
    /// shown; none before the league is known.
    #[must_use]
    pub fn trade_url(&self, item: &Item, filters: SearchFilters) -> Option<String> {
        let league = self.league.as_ref()?;
        let target = self.trade_target(item)?;
        Some(link::url(
            item.language(),
            league,
            &link::query(item, &target, filters),
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
    use crate::item::{Affix, ModifierKind};
    use crate::language::Language;
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
    fn shows_an_item_copied_from_the_in_game_trade() {
        let mut session = session();

        copy(
            &mut session,
            include_str!("item/test-items/en-owner-trade-rare-bow-mirrored.txt"),
        );

        assert_eq!(shown_base(&session), Some("Warmonger Bow"));
        let modifiers = session
            .item()
            .and_then(|item| item.parsed().ok())
            .map(|item| item.modifiers().len());
        assert_eq!(modifiers, Some(10), "3 runes and 7 explicit lines");
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
    fn the_lines_of_the_item_shown_are_read_again_with_the_stats_of_the_trade_site() {
        use crate::filters::StatKey;
        use crate::item_names::{ItemNames, Names, StatTemplate};
        use crate::trade::stat_filters::LineStat;

        let mut session = session();
        // A modifier of a patch Exiled Exchange 2 doesn't know yet, copied before the names
        // of the trade site arrive.
        copy(
            &mut session,
            "Item Class: Rings\nRarity: Unique\nAndvarius\nGold Ring\n--------\n\
             Item Level: 80\n--------\n{ Unique Modifier }\n20% reduced Damage per Patch",
        );
        let line = StatKey {
            modifier: 0,
            line: 0,
        };
        let stat = |session: &Session| session.item().unwrap().stats().get(line).cloned();
        assert_eq!(stat(&session), Some(LineStat::Unsearchable));

        let names = |stats| Names {
            exchange: std::collections::HashMap::new(),
            base_types: Vec::new(),
            stats,
        };
        let template = StatTemplate {
            id: "explicit.stat_1000000001".to_owned(),
            text: "#% reduced Damage per Patch".to_owned(),
        };
        let reshaped = session.apply_item_names(ItemNames::new(
            SystemTime::UNIX_EPOCH,
            names(vec![template]),
            names(Vec::new()),
            Vec::new(),
        ));
        assert!(!reshaped, "a unique: its lines stay as they were");

        assert!(
            matches!(stat(&session), Some(LineStat::Searched(_))),
            "{:?}",
            stat(&session)
        );
    }

    #[test]
    fn the_base_type_among_the_filters_of_a_magic_item_comes_with_the_names() {
        const MACE: &str = include_str!("item/test-items/en-magic-mace.txt");
        let mut session = session();
        let base_type = |session: &Session| {
            let item = session.item()?.parsed().ok()?;
            session.filtered_base_type(item).map(str::to_owned)
        };

        copy(&mut session, MACE);
        assert_eq!(base_type(&session), None, "the names aren't known yet");
        assert!(!session.apply_item_names(crate::item_names::tests::item_names()));
        assert_eq!(base_type(&session).as_deref(), Some("Temple Maul"));
        copy(&mut session, RING);
        assert_eq!(base_type(&session).as_deref(), Some("Ruby Ring"));
        // A unique is searched by its name, without a filter of its base type.
        copy(
            &mut session,
            "Item Class: Rings\nRarity: Unique\nAndvarius\nGold Ring",
        );
        assert_eq!(base_type(&session), None);
    }

    #[test]
    fn a_magic_item_gets_the_rolls_of_every_tier_once_the_names_tell_its_base() {
        const MACE: &str = include_str!("item/test-items/en-magic-mace.txt");
        let mut session = session();
        // The line of its strength suffix: its added lightning damage was copied before a
        // patch changed its ranges, and no tier rolls them any more.
        let strength = |session: &Session| {
            let key = crate::filters::StatKey {
                modifier: 1,
                line: 0,
            };
            session.item()?.tiers().get(key).cloned()
        };

        copy(&mut session, MACE);
        assert_eq!(strength(&session), None, "the names aren't known yet");
        assert!(!session.apply_item_names(crate::item_names::tests::item_names()));
        assert!(strength(&session).is_some_and(|rolls| rolls.tiers.is_some()));
        // Copied once the names are known: at once.
        copy(&mut session, MACE);
        assert!(strength(&session).is_some_and(|rolls| rolls.tiers.is_some()));
    }

    #[test]
    fn an_item_copied_from_the_in_game_trade_gets_the_tiers_found_from_its_values() {
        const TIARA: &str =
            include_str!("item/test-items/en-owner-trade-rare-helmet-fractured.txt");
        use Affix::{Prefix, Suffix};
        let mut session = session();
        let explicit = |session: &Session| -> Vec<(usize, Option<Affix>, Option<u32>)> {
            let Some(Ok(item)) = session.item().map(ShownItem::parsed) else {
                return Vec::new();
            };
            item.modifiers()
                .iter()
                .filter_map(|modifier| match modifier.kind {
                    ModifierKind::Explicit(affix) => {
                        Some((modifier.lines.len(), affix, modifier.tier))
                    }
                    _ => None,
                })
                .collect()
        };

        copy(&mut session, TIARA);

        assert_eq!(
            explicit(&session),
            [
                // The fractured hybrid of `41% increased Energy Shield` and `+34 to maximum
                // Mana`, its lines joined.
                (2, Some(Prefix), Some(1)),
                (1, Some(Prefix), Some(1)),
                (1, Some(Suffix), Some(1)),
                (1, Some(Suffix), Some(1)),
                (1, Some(Suffix), Some(2)),
                (1, Some(Prefix), Some(1)),
            ]
        );
        let hybrid = session.item().and_then(|item| {
            let key = crate::filters::StatKey {
                modifier: 5,
                line: 1,
            };
            item.tiers().get(key).cloned()
        });
        assert!(
            hybrid.is_some(),
            "the rolls over every tier of its second line, after the enchantment and the runes"
        );
    }

    #[test]
    fn an_item_copied_from_the_in_game_trade_is_read_again_from_its_copy() {
        const TIARA: &str =
            include_str!("item/test-items/en-owner-trade-rare-helmet-fractured.txt");
        let mut session = session();
        let modifiers = |session: &Session| {
            let item = session.item()?.parsed().ok()?;
            Some(item.modifiers().to_vec())
        };
        copy(&mut session, TIARA);
        let read = modifiers(&session);

        // Read again from the copy as the parser read it, not from its tiers found: the same.
        assert!(!session.apply_item_names(crate::item_names::tests::item_names()));
        assert!(!session.apply_item_names(crate::item_names::tests::item_names()));

        assert!(read.as_ref().is_some_and(|read| read.len() == 11));
        assert_eq!(modifiers(&session), read);
    }

    #[test]
    fn a_magic_hybrid_joined_once_the_names_arrive_reshapes_the_modifiers() {
        const MACE: &str = "Item Class: Two Hand Maces\nRarity: Magic\n\
            Crackling Temple Maul of the Brute\n--------\nItem Level: 32\n--------\n\
            30% increased Physical Damage\n+60 to Accuracy Rating\n+8 to Strength";
        let mut session = session();
        let shape = |session: &Session| {
            let item = session.item()?.parsed().ok()?;
            Some(super::shape(item.modifiers()))
        };
        copy(&mut session, MACE);
        assert_eq!(
            shape(&session),
            Some(vec![1, 1, 1]),
            "its base isn't known yet"
        );

        // The filters the player changed on its lines no longer fall on the same lines.
        assert!(session.apply_item_names(crate::item_names::tests::item_names()));

        assert_eq!(shape(&session), Some(vec![2, 1]));
        let hybrid = session
            .item()
            .and_then(|item| item.parsed().ok())
            .and_then(|item| {
                Some((
                    item.modifiers().first()?.kind,
                    item.modifiers().first()?.tier,
                ))
            });
        assert_eq!(
            hybrid,
            Some((ModifierKind::Explicit(Some(Affix::Prefix)), Some(6)))
        );

        // Names that don't know its base: read again from its copy, its lines apart again.
        let unknown = || crate::item_names::Names {
            exchange: std::collections::HashMap::new(),
            base_types: Vec::new(),
            stats: Vec::new(),
        };
        assert!(session.apply_item_names(ItemNames::new(
            SystemTime::UNIX_EPOCH,
            unknown(),
            unknown(),
            Vec::new(),
        )));
        assert_eq!(shape(&session), Some(vec![1, 1, 1]));
    }

    #[test]
    fn a_magic_item_copied_from_the_in_game_trade_gets_its_tiers_once_the_names_tell_its_base() {
        const MACE: &str = "Item Class: Two Hand Maces\nRarity: Magic\n\
            Crackling Temple Maul of the Brute\n--------\nItem Level: 32\n--------\n\
            Adds 1 to 50 Lightning Damage\n+8 to Strength";
        let mut session = session();
        let strength = |session: &Session| {
            let item = session.item()?.parsed().ok()?;
            let modifier = item.modifiers().get(1)?;
            Some((modifier.kind, modifier.tier.is_some()))
        };

        copy(&mut session, MACE);
        assert_eq!(
            strength(&session),
            Some((ModifierKind::Explicit(None), false)),
            "the names aren't known yet"
        );
        assert!(!session.apply_item_names(crate::item_names::tests::item_names()));
        assert_eq!(
            strength(&session),
            Some((ModifierKind::Explicit(Some(Affix::Suffix)), true))
        );
        // Its added lightning damage was copied before a patch changed its ranges: no tier.
        let damage = session
            .item()
            .and_then(|item| item.parsed().ok())
            .and_then(|item| Some(item.modifiers().first()?.kind));
        assert_eq!(damage, Some(ModifierKind::Explicit(None)));
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
            unknown_league.trade_search(&parsed(&unknown_league), SearchFilters::default()),
            None,
            "the league isn't known yet"
        );
        let unknown = parsed(&unknown_league);
        assert_eq!(unknown_league.search_kind(&unknown), None);
        assert!(!unknown_league.tradable(&unknown));

        let mut session = session();
        copy(&mut session, UNIQUE);
        let unique = parsed(&session);
        let search = session
            .trade_search(&unique, SearchFilters::default())
            .expect("a search");
        assert_eq!(search.league, League::new("Forbidden Rites"));
        assert_eq!(search.language, Language::English);
        assert!(!session.must_identify(&unique));
        assert_eq!(session.search_kind(&unique), Some(SearchKind::Items));
        assert!(session.tradable(&unique));

        copy(&mut session, UNIDENTIFIED);
        let unidentified = parsed(&session);
        assert_eq!(
            session.trade_search(&unidentified, SearchFilters::default()),
            None
        );
        assert!(session.must_identify(&unidentified));
        assert_eq!(session.search_kind(&unidentified), None);
        // Its link searches its base type.
        assert!(session.tradable(&unidentified));
    }
}
