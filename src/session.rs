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
    /// The modifiers of an item copied from the in-game trade as the parser read them, without
    /// affix, tier nor range: each reading starts from them, when the item is copied and each
    /// time the names of the trade site arrive (they tell the base of a magic item, and their
    /// stats the affixes a rune or an implicit allows), and for a unique when the prices arrive
    /// (they write the ranges of its modifiers).
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
    /// its items, once it gave them, and `prices` the prices of the league shown known so far.
    fn new(
        copied: &CopiedItem,
        trade: Option<&TradeStats>,
        names: Option<&ItemNames>,
        prices: Option<&PriceList>,
    ) -> Self {
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
            .filter(|item| item.has_tiers_from_values() || item.has_rolls_from_prices())
            .map(|item| item.modifiers().to_vec());
        let mut shown = Self {
            parsed,
            trade_copy,
            lines: CopiedLines::default(),
            stats: LineStats::default(),
            tiers: TierRolls::default(),
        };
        shown.read_lines(trade, names, prices);
        shown
    }

    /// Reads the modifier lines of the item, with the stats of the trade site `trade`, and
    /// their tiers on its base, found among the `names` of the trade site for a magic item;
    /// from their values for a rare or magic item copied from the in-game trade, and for a
    /// unique copied that way their ranges from its modifiers among `prices`, from its
    /// modifiers as the parser read them. True when its modifiers changed shape (the lines of a
    /// hybrid joined once the names tell the base of a magic item): the keys of their filters
    /// moved.
    fn read_lines(
        &mut self,
        trade: Option<&TradeStats>,
        names: Option<&ItemNames>,
        prices: Option<&PriceList>,
    ) -> bool {
        let Ok(item) = &mut self.parsed else {
            return false;
        };
        let base_type = link::target(item, names, None)
            .and_then(|target| target.base_type())
            .map(str::to_owned);
        let mut reshaped = false;
        if let Some(copied) = &self.trade_copy {
            let modifiers = if item.has_rolls_from_prices() {
                unique_rolls(item, copied, trade, names, prices)
            } else if let Some(base_type) = &base_type {
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
            } else {
                copied.clone()
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

/// The modifiers `copied` of `item`, a unique copied from the in-game trade, with the ranges of
/// its modifiers and of those of its base among `prices`, found by its English name (through
/// the `names` of the trade site for a French copy); as copied while they don't hold it.
fn unique_rolls(
    item: &Item,
    copied: &[Modifier],
    trade: Option<&TradeStats>,
    names: Option<&ItemNames>,
    prices: Option<&PriceList>,
) -> Vec<Modifier> {
    match pricing::fixed_price(item, prices, names) {
        Ok(FixedPrice::Unique(unique)) => game_data::with_unique_rolls(
            item.language(),
            copied,
            &unique.modifiers.implicit,
            &unique.modifiers.explicit,
            trade,
        ),
        _ => copied.to_vec(),
    }
}

/// `prices` if they are those of `league`, the league shown: the borrows of the fields of the
/// session, apart, let it read them while it changes the item shown.
fn shown_prices<'a>(
    prices: Option<&'a LeaguePrices>,
    league: Option<&League>,
) -> Option<&'a LeaguePrices> {
    prices.filter(|prices| league == Some(&prices.list.league))
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
        if !self.shows(&list) {
            return;
        }
        self.prices = Some(LeaguePrices { list, downloading });
        // A unique copied from the in-game trade takes its ranges from them; the shape of its
        // modifiers never changes.
        if let Some(shown) = &mut self.item
            && shown.parsed().is_ok_and(Item::has_rolls_from_prices)
        {
            // Those of the league shown, checked above.
            let prices = self.prices.as_ref().map(|prices| &prices.list);
            shown.read_lines(self.trade_stats.as_ref(), self.item_names.as_ref(), prices);
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
        let prices =
            shown_prices(self.prices.as_ref(), self.league.as_ref()).map(|prices| &prices.list);
        let reshaped = self
            .item
            .as_mut()
            .is_some_and(|shown| shown.read_lines(Some(&trade), self.item_names.as_ref(), prices));
        self.trade_stats = Some(trade);
        reshaped
    }

    pub fn apply(&mut self, event: ClipboardEvent) {
        match event {
            ClipboardEvent::Copied(copied) => {
                let trade = self.trade_stats.as_ref();
                let prices = self.league_prices().map(|prices| &prices.list);
                let shown = ShownItem::new(&copied, trade, self.item_names.as_ref(), prices);
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
        shown_prices(self.prices.as_ref(), self.league.as_ref())
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
mod tests;
