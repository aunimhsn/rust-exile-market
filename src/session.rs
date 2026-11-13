//! What the window shows, built from what happened: the state of the application, without
//! the interface.

use std::time::{Duration, SystemTime};

use crate::clipboard::ClipboardEvent;
use crate::game_data::{self, ItemBase, StatSource, TradeGroup, TradeItem, TradeStats};
use crate::item::{CopiedItem, Item, ItemState, Modifier, ParseError, Rarity, RollsSource};
use crate::item_names::ItemNames;
use crate::language::PerLanguage;
use crate::league::{League, Leagues};
use crate::logging;
use crate::market::{MarketEvent, Source};
use crate::price::estimate::Worth;
use crate::price::fixed::{self, FixedPrice, NoFixedPrice};
use crate::price::{PriceList, Rates};
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
    trade_copy: Option<TradeCopy>,
    /// Its modifier lines, which the offers are compared with; none for an unreadable item.
    lines: CopiedLines,
    /// The stat of each modifier line on the trade site, which searches it.
    stats: LineStats,
    /// The rolls of each line of a prefix or suffix over every tier of its modifier.
    tiers: TierRolls,
}

/// The modifiers of an item copied from the in-game trade as the parser read them, by where
/// their rolls come from.
#[derive(Debug)]
enum TradeCopy {
    /// A rare or magic item, its tiers found from its values in the modifier tables.
    Values(Vec<Modifier>),
    /// A unique, its ranges taken from its modifiers in the price database.
    Prices(Vec<Modifier>),
}

impl TradeCopy {
    /// The modifiers of `item` copied from the in-game trade; none for an item copied
    /// otherwise.
    fn of(item: &Item) -> Option<Self> {
        match item.rolls_source() {
            RollsSource::Values => Some(Self::Values(item.modifiers().to_vec())),
            RollsSource::Prices => Some(Self::Prices(item.modifiers().to_vec())),
            RollsSource::Headers | RollsSource::Nowhere => None,
        }
    }
}

/// What the services gave so far, which the lines of an item are read with.
#[derive(Debug, Clone, Copy)]
struct ServiceData<'a> {
    /// The stats of the trade site, once it gave them.
    trade_stats: Option<&'a TradeStats>,
    /// The names of the items of the trade site, once it gave them.
    names: Option<&'a ItemNames>,
    /// The prices of the league shown known so far.
    prices: Option<&'a PriceList>,
}

impl ShownItem {
    /// The item `copied`, read with what the services gave so far.
    fn new(copied: &CopiedItem, service_data: ServiceData) -> Self {
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
        let trade_copy = parsed.as_ref().ok().and_then(TradeCopy::of);
        let mut shown = Self {
            parsed,
            trade_copy,
            lines: CopiedLines::default(),
            stats: LineStats::default(),
            tiers: TierRolls::default(),
        };
        shown.read_lines(service_data);
        shown
    }

    /// Reads the modifier lines of the item, with the stats of the trade site, and their tiers
    /// on its base, found among the names of the trade site for a magic item; from their
    /// values for a rare or magic item copied from the in-game trade, and for a unique copied
    /// that way their ranges from its modifiers among the prices, from its modifiers as the
    /// parser read them. True when its modifiers changed shape (the lines of a hybrid joined
    /// once the names tell the base of a magic item): the keys of their filters moved.
    fn read_lines(&mut self, service_data: ServiceData) -> bool {
        let trade = service_data.trade_stats;
        let Ok(item) = &mut self.parsed else {
            return false;
        };
        let base_type = link::target(item, service_data.names, None)
            .and_then(|target| target.base_type())
            .map(str::to_owned);
        let mut reshaped = false;
        if let Some(trade_copy) = &self.trade_copy {
            let modifiers = match (trade_copy, &base_type) {
                (TradeCopy::Prices(copied), _) => unique_rolls(item, copied, service_data),
                (TradeCopy::Values(copied), Some(base_type)) => {
                    value_tiers(item, copied, base_type, trade)
                }
                // A magic item before the names of the trade site tell its base.
                (TradeCopy::Values(copied), None) => copied.clone(),
            };
            reshaped = shape(item.modifiers()) != shape(&modifiers);
            item.set_modifiers(modifiers);
        }
        self.stats = LineStats::new(item, item.modifiers(), trade);
        self.lines = CopiedLines::new(item.language(), item.modifiers(), &self.stats, trade);
        self.tiers = TierRolls::new(
            item.language(),
            item.modifiers(),
            base_type.as_deref(),
            item.item_level(),
        );
        reshaped
    }

    /// The number of lines of each of its modifiers, none for an item that can't be read: the
    /// filters of its lines are kept by their places.
    #[must_use]
    pub fn shape(&self) -> Vec<usize> {
        self.parsed
            .as_ref()
            .map(|item| shape(item.modifiers()))
            .unwrap_or_default()
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

/// The modifiers `copied` of `item`, a rare or magic item copied from the in-game trade whose
/// base is `base_type`, with the tiers found from their values in the modifier tables, the
/// stats of the trade site `trade` telling the affixes its runes and implicits allow.
fn value_tiers(
    item: &Item,
    copied: &[Modifier],
    base_type: &str,
    trade: Option<&TradeStats>,
) -> Vec<Modifier> {
    // The runes and the implicits, which tell the room, are never read again.
    let stats = LineStats::new(item, copied, trade);
    let trade_item = TradeItem {
        base: ItemBase {
            language: item.language(),
            base_type,
            item_level: item.item_level(),
        },
        limits: stats.affix_limits(item),
        is_sanctified: item.has_state(ItemState::Sanctified),
    };
    game_data::with_trade_tiers(&trade_item, copied)
}

/// The modifiers `copied` of `item`, a unique copied from the in-game trade, with the ranges of
/// its modifiers and of those of its base among the prices of `service_data`, found by its
/// English name (through the names of the trade site for a French copy); as copied while they
/// don't hold it.
fn unique_rolls(item: &Item, copied: &[Modifier], service_data: ServiceData) -> Vec<Modifier> {
    let price = fixed::price_of(item, service_data.prices, service_data.names);
    match price {
        Ok(FixedPrice::Unique(unique)) => game_data::with_unique_rolls(
            item.language(),
            copied,
            &unique.modifiers,
            service_data.trade_stats,
        ),
        Ok(FixedPrice::Currency(_) | FixedPrice::Uniques(_))
        | Err(
            NoFixedPrice::NotFixedName
            | NoFixedPrice::PricesUnknown
            | NoFixedPrice::NotPriced
            | NoFixedPrice::NamesUnknown,
        ) => copied.to_vec(),
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

/// What the window shows of the leagues, the prices, the names of the trade site and the
/// item copied, built from the events of the threads.
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
    are_leagues_reachable: bool,
    /// The league whose last download of the prices failed, if any.
    prices_failed: Option<League>,
    /// The names of the items of the trade site, which find the prices of a French item.
    item_names: Option<ItemNames>,
    /// The stats of the trade site, from the templates given with the names: the stats of
    /// a modifier line the game data doesn't know yet.
    trade_stats: Option<TradeStats>,
    /// Shown until another item is copied.
    item: Option<ShownItem>,
    /// The clipboard can be read, as its watcher last told.
    is_clipboard_readable: bool,
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
            are_leagues_reachable: true,
            prices_failed: None,
            item_names: None,
            trade_stats: None,
            item: None,
            is_clipboard_readable: true,
        }
    }

    /// Takes in what the price database sent; `closed` are the new leagues whose bubble the
    /// player closed.
    pub fn apply_market(&mut self, event: MarketEvent, closed: &[League]) -> Option<LeagueChange> {
        match event {
            MarketEvent::Leagues(leagues) => {
                self.are_leagues_reachable = true;
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
            MarketEvent::Unreachable(Source::Leagues) => self.are_leagues_reachable = false,
            MarketEvent::Unreachable(Source::Prices(league)) => self.prices_failed = Some(league),
        }
        None
    }

    fn apply_leagues(&mut self, leagues: Leagues, closed: &[League]) -> Option<LeagueChange> {
        let choice = leagues.choose(self.league.as_ref());
        if let Some(ended) = &choice.ended {
            log::info!(
                "{} has ended: switched to {}",
                logging::plain(ended.id()),
                logging::plain(choice.league.id())
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
            shown.read_lines(ServiceData {
                trade_stats: self.trade_stats.as_ref(),
                names: self.item_names.as_ref(),
                prices: self.prices.as_ref().map(|prices| &prices.list),
            });
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
        let PerLanguage { english, french } = names.take_stat_templates();
        let trade = TradeStats::new(&english, &french);
        self.item_names = Some(names);
        // An item copied before them may have a line only the trade site knows, or be a
        // magic item whose base type they tell.
        let service_data = ServiceData {
            trade_stats: Some(&trade),
            names: self.item_names.as_ref(),
            prices: shown_prices(self.prices.as_ref(), self.league.as_ref())
                .map(|prices| &prices.list),
        };
        let reshaped = self
            .item
            .as_mut()
            .is_some_and(|shown| shown.read_lines(service_data));
        self.trade_stats = Some(trade);
        reshaped
    }

    /// Takes in what the clipboard watcher sent: the item copied, read with what the services
    /// gave so far, or a change of readability.
    pub fn apply(&mut self, event: ClipboardEvent) {
        match event {
            ClipboardEvent::Copied { item: copied, .. } => {
                let service_data = ServiceData {
                    trade_stats: self.trade_stats.as_ref(),
                    names: self.item_names.as_ref(),
                    prices: self.league_prices().map(|prices| &prices.list),
                };
                let shown = ShownItem::new(&copied, service_data);
                if let Ok(item) = shown.parsed() {
                    self.log_price(item);
                    log_unknown_stats(item, service_data.trade_stats);
                }
                self.item = Some(shown);
            }
            ClipboardEvent::Unreadable => self.is_clipboard_readable = false,
            ClipboardEvent::Readable => self.is_clipboard_readable = true,
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
            log::info!("Base type of the magic item: {}", logging::plain(base_type));
        }
        match self.fixed_price(item).map(|(price, _)| price) {
            Ok(FixedPrice::Currency(currency)) => {
                log::info!(
                    "Price of {}: {:.2} ex",
                    logging::plain(&currency.name),
                    currency.price.exalted
                );
            }
            Ok(FixedPrice::Unique(unique)) => {
                log::info!(
                    "Price of {}: {:.2} ex",
                    logging::plain(&unique.name),
                    unique.price.exalted
                );
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
        let price = fixed::price_of(item, prices, self.item_names.as_ref())?;
        // A price comes from the prices, which have their rates.
        let rates = prices.ok_or(NoFixedPrice::PricesUnknown)?.rates;
        Ok((price, rates))
    }

    /// What the currencies are worth with the prices of the league shown, once known.
    #[must_use]
    pub fn worth(&self) -> Option<Worth<'_>> {
        self.league_prices().map(|prices| Worth::of(&prices.list))
    }

    /// What the trade site can do with `item` in the league shown, its target worked out once
    /// for the questions the window asks of it at each frame.
    #[must_use]
    pub fn trade<'a>(&'a self, item: &'a Item) -> ItemTrade<'a> {
        let prices = self.league_prices().map(|prices| &prices.list);
        ItemTrade {
            item,
            league: self.league.as_ref(),
            target: link::target(item, self.item_names.as_ref(), prices),
        }
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
        let offline = !self.are_leagues_reachable || prices_failed;
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
    pub fn is_clipboard_readable(&self) -> bool {
        self.is_clipboard_readable
    }
}

/// What the trade site can do with an item in the league shown ([`Session::trade`]).
#[derive(Debug)]
pub struct ItemTrade<'a> {
    item: &'a Item,
    /// None before the leagues arrive at the first session.
    league: Option<&'a League>,
    /// What its search looks for; none for a magic item whose base type isn't known yet.
    target: Option<TradeTarget<'a>>,
}

impl<'a> ItemTrade<'a> {
    /// The item asked about.
    #[must_use]
    pub fn item(&self) -> &'a Item {
        self.item
    }

    /// The base type of the item among its filters, which its search looks for unless the
    /// player unchecks it: for a rare, magic or normal item, or a jewel; none for a magic item
    /// whose base type isn't known yet.
    #[must_use]
    pub fn filtered_base_type(&self) -> Option<&'a str> {
        if !self.item.rarity().has_item_filters() {
            return None;
        }
        self.target.as_ref()?.base_type()
    }

    /// The search for the item on the trade site with its `filters`; none before the league
    /// is known, and for an unidentified unique, which is identified to be searched.
    #[must_use]
    pub fn search(&self, filters: SearchFilters) -> Option<Search> {
        let (league, target) = self.search_parts()?;
        Some(Search {
            language: self.item.language(),
            league: league.clone(),
            query: link::query(self.item, target, filters),
        })
    }

    /// The kind of the search of the item, without building it; none when it has no search
    /// ([`Self::search`]).
    #[must_use]
    pub fn search_kind(&self) -> Option<SearchKind> {
        let (_, target) = self.search_parts()?;
        Some(target.query().kind())
    }

    /// The link of the search for the item can be opened: the league and its target known.
    #[must_use]
    pub fn is_tradable(&self) -> bool {
        self.link_parts().is_some()
    }

    /// An unidentified unique, whose offers are searched once it is identified.
    #[must_use]
    pub fn must_identify(&self) -> bool {
        matches!(self.target, Some(TradeTarget::UnidentifiedUnique { .. }))
    }

    /// The link of the search for the item on the trade site with its `filters`; none before
    /// the league is known.
    #[must_use]
    pub fn url(&self, filters: SearchFilters) -> Option<String> {
        let (league, target) = self.link_parts()?;
        Some(link::url(
            self.item.language(),
            league,
            &link::query(self.item, target, filters),
        ))
    }

    /// The league and the target of the link of the item, once both are known.
    fn link_parts(&self) -> Option<(&'a League, &TradeTarget<'a>)> {
        Some((self.league?, self.target.as_ref()?))
    }

    /// The league and the target of the search of the item: those of its link, but for an
    /// unidentified unique, which is identified to be searched.
    fn search_parts(&self) -> Option<(&'a League, &TradeTarget<'a>)> {
        self.link_parts()
            .filter(|(_, target)| !matches!(target, TradeTarget::UnidentifiedUnique { .. }))
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
            let text = logging::plain(&text.collect::<Vec<_>>().join(" / "));
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
