//! The application: the session, the settings, the clipboard watching and the price
//! database, and the window built from the components of [`crate::ui`].

use std::hash::{BuildHasher, RandomState};
use std::time::{Duration, Instant, SystemTime};

use eframe::{App, CreationContext, Frame};
use egui::{Context, Key, OpenUrl, Ui, ViewportCommand};

use crate::clipboard::{ClipboardEvent, ClipboardWatch};
use crate::estimate::{self, Estimate, Reference};
use crate::filters::Filters;
use crate::i18n::{DOWNLOAD_QUOTE_COUNT, Message, Translator};
use crate::item::Item;
use crate::league::League;
use crate::market::MarketWatch;
use crate::offers::{LONG_WAIT, Offers, OffersState};
use crate::prices::Rates;
use crate::pricing::{FixedPrice, NoFixedPrice};
use crate::session::{self, LeagueChange, PricesState, Session, ShownItem};
use crate::settings::{SettingChange, Settings, SettingsFile};
use crate::trade::ItemNamesWatch;
use crate::trade::query::{SearchFilters, SearchKind};
use crate::trade::search::{OfferSearch, Search};
use crate::ui::{
    self, BubbleAction, EmptyState, ItemBlock, ItemBlockAction, LeagueList, LeagueListAction,
    NewLeagueBubble, OffersAction, OffersBlock, PriceBlock, PriceDownload, ResizeEdges,
    SettingsPanel, Stats, StatusBar, TitleBar, TitleBarAction, TradeStatus, UnreadableItem,
    WarningBand,
};

/// What the window shows under the title bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum View {
    /// The copied item, or the empty state before the first one.
    Item,
    Settings,
}

impl View {
    /// The view after a click on the gear: it opens the settings, and closes them.
    fn toggled(self) -> Self {
        match self {
            Self::Item => Self::Settings,
            Self::Settings => Self::Item,
        }
    }

    /// The view after `event`: a copied item closes the settings, so that the player sees it.
    fn after(self, event: &ClipboardEvent) -> Self {
        match event {
            ClipboardEvent::Copied(_) => Self::Item,
            ClipboardEvent::Unreadable | ClipboardEvent::Readable => self,
        }
    }
}

pub struct ExileMarketApp {
    i18n: Translator,
    session: Session,
    settings: Settings,
    settings_file: SettingsFile,
    view: View,
    /// The list of the leagues is open over the view.
    leagues_open: bool,
    clipboard: ClipboardWatch,
    market: MarketWatch,
    item_names: ItemNamesWatch,
    /// The searches of the trade site, asked by the player.
    search: OfferSearch,
    /// The offers of the item shown, and the waiting time before the next search.
    offers: Offers,
    /// The stats of the item are unfolded, for every item copied until the player folds them.
    stats_open: bool,
    /// The filters of the search as the player set them, for the item shown: cleared at each
    /// copy, even of the same item after another text.
    filters: Filters,
    /// An item was copied: the content scrolls back to the top, to its name and its price.
    scroll_to_top: bool,
    /// The sentence shown during the download of the prices of a league, drawn at its start.
    download_quote: Option<(League, usize)>,
}

impl ExileMarketApp {
    /// The application with the settings read from `settings_file`, which the window
    /// already applies.
    #[must_use]
    pub fn new(
        creation: &CreationContext<'_>,
        settings: Settings,
        settings_file: SettingsFile,
    ) -> Self {
        ui::install(&creation.egui_ctx);
        let clipboard_ctx = creation.egui_ctx.clone();
        let market_ctx = creation.egui_ctx.clone();
        let names_ctx = creation.egui_ctx.clone();
        let search_ctx = creation.egui_ctx.clone();
        // The prices of the league of the last session, until the leagues arrive.
        let market = MarketWatch::start(settings.league(), move || market_ctx.request_repaint());

        Self {
            // The language of the copied items never changes the interface: it only chooses
            // the item names and the trade site.
            i18n: Translator::new(settings.language().language()),
            // The league of the last session, until the leagues arrive.
            session: Session::new(settings.league()),
            settings,
            settings_file,
            view: View::Item,
            leagues_open: false,
            clipboard: ClipboardWatch::start(move || clipboard_ctx.request_repaint()),
            market,
            item_names: ItemNamesWatch::start(move || names_ctx.request_repaint()),
            search: OfferSearch::start(move || search_ctx.request_repaint()),
            offers: Offers::default(),
            stats_open: false,
            filters: Filters::default(),
            scroll_to_top: false,
            download_quote: None,
        }
    }

    fn act_on(&mut self, ctx: &Context, action: TitleBarAction) {
        match action {
            TitleBarAction::Move => ctx.send_viewport_cmd(ViewportCommand::StartDrag),
            TitleBarAction::Home => {
                self.view = View::Item;
                self.leagues_open = false;
            }
            TitleBarAction::Leagues => {
                self.leagues_open = !self.leagues_open && self.session.leagues().is_some();
            }
            TitleBarAction::Settings => {
                self.view = self.view.toggled();
                self.leagues_open = false;
            }
            TitleBarAction::Minimize => ctx.send_viewport_cmd(ViewportCommand::Minimized(true)),
            TitleBarAction::Close => ctx.send_viewport_cmd(ViewportCommand::Close),
        }
    }

    /// Shows `league`, chosen by the player, and keeps it in the settings. Choosing answers
    /// the bubble of a new league too: it doesn't come back for that league.
    fn choose_league(&mut self, league: League) {
        self.leagues_open = false;
        let mut changed = self.settings.set_league(&league);
        if let Some(offered) = self.session.close_new_league() {
            changed |= self.settings.close_new_league(&offered);
        }
        if changed {
            self.settings_file.save(&self.settings);
        }
        // The offers shown were searched in the previous league.
        if self.session.league() != Some(&league) {
            self.offers.clear();
        }
        self.session.choose_league(league);
        self.market.price(self.session.league());
    }

    /// Closes the bubble of the new league for good.
    fn close_new_league(&mut self) {
        if let Some(league) = self.session.close_new_league()
            && self.settings.close_new_league(&league)
        {
            self.settings_file.save(&self.settings);
        }
    }

    /// Draws the list of the leagues and the bubble of a new league over the window.
    fn league_overlays(&mut self, ctx: &Context, title_bar_action: Option<TitleBarAction>) {
        if let Some(leagues) = self.session.leagues() {
            let list = LeagueList::new(leagues.rows(), self.session.league(), &self.i18n);
            match list.show(ctx, self.leagues_open) {
                Some(LeagueListAction::Choose(league)) => self.choose_league(league),
                // The click on the league block that opened the list isn't outside it.
                Some(LeagueListAction::Close)
                    if title_bar_action != Some(TitleBarAction::Leagues) =>
                {
                    self.leagues_open = false;
                }
                Some(LeagueListAction::Close) | None => {}
            }
        }
        // Once the list is gone: the bubble would show through it while it fades out.
        if LeagueList::visibility(ctx, self.leagues_open) > 0.0 || self.view != View::Item {
            return;
        }
        let Some(league) = self.session.new_league().cloned() else {
            return;
        };
        match NewLeagueBubble::new(&league, &self.i18n).show(ctx) {
            Some(BubbleAction::Switch) => self.choose_league(league),
            Some(BubbleAction::Close) => self.close_new_league(),
            None => {}
        }
    }

    /// Applies a change of the settings at once, and saves them.
    fn change_setting(&mut self, ctx: &Context, change: SettingChange) {
        if !self.settings.apply(change) {
            return;
        }
        match change {
            SettingChange::Language(language) => {
                self.i18n = Translator::new(language.language());
            }
            SettingChange::AlwaysOnTop(_) => {
                ctx.send_viewport_cmd(ViewportCommand::WindowLevel(self.settings.window_level()));
            }
        }
        self.settings_file.save(&self.settings);
        // The bands above the settings were drawn with the previous values.
        ctx.request_repaint();
    }

    /// The warnings of the band under the title bar, given the state of the prices.
    fn warnings(&self, prices: PricesState) -> Vec<String> {
        let mut warnings = Vec::new();
        if !self.session.clipboard_readable() {
            warnings.push(self.i18n.text(Message::ClipboardUnreadable));
        }
        match prices {
            PricesState::Unavailable => warnings.push(self.i18n.text(Message::PricesUnavailable)),
            PricesState::Known { age, offline: true } => {
                warnings.push(self.i18n.prices_unreachable(&self.i18n.age(age)));
            }
            PricesState::Downloading { .. } | PricesState::Known { offline: false, .. } => {}
        }
        warnings
    }

    /// The item shown, with the `kind` of the search of its offers when it can be searched.
    fn item(&mut self, ui: &mut Ui, prices: PricesState, kind: Option<SearchKind>) {
        let Some(shown) = self.session.item() else {
            let empty = EmptyState::new(&self.i18n);
            if let (PricesState::Downloading { progress }, Some((league, quote))) =
                (prices, &self.download_quote)
            {
                empty
                    .downloading(PriceDownload {
                        league: league.name(),
                        progress,
                        quote: *quote,
                    })
                    .show(ui);
            } else {
                empty.show(ui);
            }
            return;
        };
        let Ok(item) = shown.parsed() else {
            UnreadableItem::new(&self.i18n).show(ui);
            return;
        };
        // The pill shows only when its link can be opened: the league and the search known.
        let tradable = self.session.tradable(item);
        let must_identify = self.session.must_identify(item);
        let fixed_price = self.session.fixed_price(item);
        let unique_modifiers = session::unique_modifiers(item, &fixed_price);
        let estimate = estimate(&self.offers, &fixed_price, self.session.rates());
        // Without a fixed price, the estimate tells the suspicious offers.
        let reference = reference(&fixed_price).or(estimate.map(|estimate| estimate.reference));
        let to_top = std::mem::take(&mut self.scroll_to_top);
        let mut offers_action = None;
        let action = ui::content(ui, to_top, |ui| {
            let action = ItemBlock::new(
                item,
                shown.lines(),
                shown.stats(),
                &mut self.filters,
                &self.i18n,
            )
            .tradable(tradable)
            .stats(
                self.stats_open,
                unique_modifiers,
                self.session.filtered_base_type(item),
            )
            .show(ui);
            if action == Some(ItemBlockAction::CheckOnTrade) {
                // With the filters as the player just left them.
                let filters = search_filters(&self.session, shown, item, &self.filters);
                if let Some(url) = self.session.trade_url(item, filters) {
                    ui.ctx().open_url(OpenUrl::new_tab(url));
                }
            }
            let league = self.session.league().map(League::name);
            PriceBlock::new(item, fixed_price, prices, league, &self.i18n)
                .estimate(estimate)
                .show(ui);
            if let Some(kind) = kind {
                offers_action = OffersBlock::new(
                    &self.offers,
                    kind,
                    reference,
                    item,
                    shown.lines(),
                    &self.i18n,
                )
                .show(ui);
            } else if must_identify {
                OffersBlock::identify(ui, &self.i18n);
            }
            action
        });
        if action == Some(ItemBlockAction::ToggleStats) {
            self.stats_open = !self.stats_open;
        }
        let order = match offers_action {
            // Built at the click, with the filters as the player just left them.
            Some(OffersAction::Search) => self
                .shown_search()
                .and_then(|search| self.offers.search(search, Instant::now())),
            Some(OffersAction::More) => self.offers.more(),
            None => None,
        };
        if let Some(order) = order {
            self.search.order(order);
        }
    }

    /// The search of the offers of the item shown, when it can be searched.
    fn shown_search(&self) -> Option<Search> {
        let shown = self.session.item()?;
        let item = shown.parsed().ok()?;
        let filters = search_filters(&self.session, shown, item, &self.filters);
        self.session.trade_search(item, filters)
    }
}

/// The filters of the search of `item`, the item `shown`: as the player set them, or else
/// by default.
fn search_filters(
    session: &Session,
    shown: &ShownItem,
    item: &Item,
    filters: &Filters,
) -> SearchFilters {
    let fixed_price = session.fixed_price(item);
    let unique_modifiers = session::unique_modifiers(item, &fixed_price);
    let stats = Stats::new(
        item,
        shown.lines(),
        shown.stats(),
        unique_modifiers,
        session.filtered_base_type(item),
    );
    shown.stats().search_filters(&stats.filters(filters))
}

impl App for ExileMarketApp {
    /// Takes in what happened since the last frame; `ui` draws the session, and takes in the
    /// clicks of the components it draws.
    fn logic(&mut self, ctx: &Context, _frame: &mut Frame) {
        self.clipboard.receive(|event| {
            if matches!(event, ClipboardEvent::Copied(_)) {
                self.scroll_to_top = true;
                self.filters.clear();
                self.offers.clear();
            }
            self.view = self.view.after(&event);
            self.session.apply(event);
        });
        if self
            .search
            .receive(|answer| self.offers.apply(answer, SystemTime::now()))
        {
            self.offers.thread_lost();
        }
        let mut shown = None;
        self.market.receive(|event| {
            let closed = self.settings.closed_new_leagues();
            if let Some(LeagueChange::Shown(league)) = self.session.apply_market(event, closed) {
                shown = Some(league);
            }
        });
        if let Some(league) = shown {
            // The offers shown were searched in the previous league.
            self.offers.clear();
            if self.settings.set_league(&league) {
                self.settings_file.save(&self.settings);
            }
        }
        // The league shown at the first launch, or after the end of a league.
        self.market.price(self.session.league());
        self.item_names
            .receive(|names| self.session.apply_item_names(names));
        // Escape closes what is on top: the list of the leagues, then the settings.
        if ctx.input(|input| input.key_pressed(Key::Escape)) {
            if self.leagues_open {
                self.leagues_open = false;
            } else if self.view == View::Settings {
                self.view = View::Item;
            }
        }
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut Frame) {
        let ctx = ui.ctx().clone();
        if let Some(direction) = ResizeEdges::show(&ctx) {
            ctx.send_viewport_cmd(ViewportCommand::BeginResize(direction));
        }
        // The panels on the edges come first, so that the central one takes what remains.
        let action = TitleBar::new(self.session.league(), &self.i18n)
            .leagues_known(self.session.leagues().is_some())
            .settings_open(self.view == View::Settings)
            .leagues_open(self.leagues_open)
            .show(ui);
        if let Some(action) = action {
            self.act_on(&ctx, action);
        }
        self.league_overlays(&ctx, action);
        let prices = self.session.prices_state(SystemTime::now());
        // A sentence per download: drawn again for another league.
        self.download_quote = match (prices, self.session.league()) {
            (PricesState::Downloading { .. }, Some(league)) => match self.download_quote.take() {
                Some((drawn, quote)) if drawn == *league => Some((drawn, quote)),
                _ => Some((league.clone(), random_quote())),
            },
            _ => None,
        };
        WarningBand::new(self.warnings(prices)).show(ui);
        if let PricesState::Known { age, .. } = prices {
            // The age of the prices changes with the time, without an event.
            ctx.request_repaint_after(Duration::from_secs(60 - age.as_secs() % 60));
        }
        let search_kind = self
            .session
            .item()
            .and_then(|shown| shown.parsed().ok())
            .and_then(|item| self.session.search_kind(item));
        // The status bar tells the wait before a search of the item shown.
        let kind = search_kind.unwrap_or(SearchKind::Items);
        let trade = trade_status(&self.offers, kind, Instant::now(), SystemTime::now());
        match trade {
            // The age of the last search, and the countdown of the long wait, change with the time.
            TradeStatus::Searched(age) => {
                ctx.request_repaint_after(Duration::from_secs(60 - age.as_secs() % 60));
            }
            TradeStatus::Waiting(_) => ctx.request_repaint_after(Duration::from_secs(1)),
            TradeStatus::Idle => {}
        }
        match self.view {
            View::Item => StatusBar::sources(&self.i18n, prices, trade).show(ui),
            View::Settings => StatusBar::author(&self.i18n).show(ui),
        }
        match self.view {
            View::Item => self.item(ui, prices, search_kind),
            View::Settings => {
                if let Some(change) = SettingsPanel::new(&self.settings, &self.i18n).show(ui) {
                    self.change_setting(ui.ctx(), change);
                }
            }
        }
    }

    /// eframe only keeps the size and the position of the window: the state of the widgets
    /// starts again at each launch.
    fn persist_egui_memory(&self) -> bool {
        false
    }
}

/// The price of one item that tells a suspicious offer: the price of a fixed-name item.
fn reference(fixed_price: &Result<(FixedPrice, Rates), NoFixedPrice>) -> Option<Reference> {
    let (exalted, rates) = match fixed_price {
        Ok((FixedPrice::Currency(currency), rates)) => (currency.price.exalted, *rates),
        Ok((FixedPrice::Unique(unique), rates)) => (unique.price.exalted, *rates),
        Ok((FixedPrice::Uniques(_), _)) | Err(_) => return None,
    };
    Some(Reference { exalted, rates })
}

/// The estimated price of an item without a fixed price, from the `offers` of its search
/// and the `rates` of the prices of the league.
fn estimate(
    offers: &Offers,
    fixed_price: &Result<(FixedPrice, Rates), NoFixedPrice>,
    rates: Option<Rates>,
) -> Option<Estimate> {
    let Err(NoFixedPrice::NotFixedName) = fixed_price else {
        return None;
    };
    let OffersState::Items { offers, total, .. } = offers.state() else {
        return None;
    };
    estimate::estimate(offers, *total, rates?)
}

/// The state of the trade site in the status bar, at `now`: the long wait before the next
/// search of `kind`, or else the age of the last search.
fn trade_status(offers: &Offers, kind: SearchKind, now: Instant, clock: SystemTime) -> TradeStatus {
    match (offers.wait(kind, now), offers.last_search()) {
        (wait, _) if wait > LONG_WAIT => TradeStatus::Waiting(wait),
        (_, Some(searched)) => {
            TradeStatus::Searched(clock.duration_since(searched).unwrap_or_default())
        }
        (_, None) => TradeStatus::Idle,
    }
}

/// A sentence of the download of the prices, drawn at random.
fn random_quote() -> usize {
    // Each `RandomState` has its own keys: enough to draw a sentence.
    let [byte, ..] = RandomState::new().hash_one(()).to_le_bytes();
    usize::from(byte) % DOWNLOAD_QUOTE_COUNT
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::CopiedItem;

    #[test]
    fn the_gear_opens_the_settings_then_closes_them() {
        assert_eq!(View::Item.toggled(), View::Settings);
        assert_eq!(View::Settings.toggled(), View::Item);
    }

    #[test]
    fn a_copied_item_closes_the_settings() {
        let copied = ClipboardEvent::Copied(
            CopiedItem::from_text("Item Class: Rings\nRarity: Rare\nWoe Loop\nRuby Ring").unwrap(),
        );

        assert_eq!(View::Settings.after(&copied), View::Item);
        assert_eq!(View::Item.after(&copied), View::Item);
    }

    #[test]
    fn the_status_bar_tells_a_long_wait_rather_than_the_age_of_the_last_search() {
        use crate::trade::query::Query;
        use crate::trade::search::{Answer, NextSearch, Outcome};

        let now = Instant::now();
        let clock = SystemTime::UNIX_EPOCH + Duration::from_secs(600);
        let mut offers = Offers::default();
        assert_eq!(
            trade_status(&offers, SearchKind::Items, now, clock),
            TradeStatus::Idle
        );
        let search = Search {
            language: crate::language::Language::English,
            league: League::new("Standard"),
            query: Query::Exchange("divine".to_owned()),
        };
        let _ = offers.search(search, now);
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
        offers.apply(
            answer(240),
            SystemTime::UNIX_EPOCH + Duration::from_secs(480),
        );

        assert_eq!(
            trade_status(&offers, SearchKind::Items, now, clock),
            TradeStatus::Waiting(Duration::from_secs(240))
        );
        assert_eq!(
            trade_status(&offers, SearchKind::Exchange, now, clock),
            TradeStatus::Searched(Duration::from_secs(120)),
            "the usual 10 s aren't told"
        );
    }

    #[test]
    fn the_state_of_the_clipboard_keeps_the_view() {
        for event in [ClipboardEvent::Unreadable, ClipboardEvent::Readable] {
            assert_eq!(View::Settings.after(&event), View::Settings);
            assert_eq!(View::Item.after(&event), View::Item);
        }
    }
}
