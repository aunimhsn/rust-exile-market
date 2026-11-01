//! The application: the session, the settings, the clipboard watching, the price-check
//! hotkey and the price database, and the window built from the components of [`crate::ui`].

use std::hash::{BuildHasher, RandomState};
use std::time::{Duration, Instant, SystemTime};

use eframe::{App, CreationContext, Frame};
use egui::{
    Context, Key, OpenUrl, Ui, ViewportBuilder, ViewportCommand, ViewportId, X11WindowType, pos2,
    vec2,
};

use crate::clipboard::{ClipboardEvent, ClipboardWatch};
use crate::estimate::{self, Estimate, Reference, Worth};
use crate::filters::{Filters, Window};
use crate::hotkey::PriceCheckHotkey;
use crate::i18n::{DOWNLOAD_QUOTE_COUNT, Message, Translator};
use crate::item::{CopiedItem, Item};
use crate::league::League;
use crate::market::MarketWatch;
use crate::offers::{LONG_WAIT, Offers, OffersState};
use crate::overlay::{self, ClickWatch, KeyboardFocus, Overlay, ScreenRect};
use crate::prices::Rates;
use crate::pricing::{FixedPrice, NoFixedPrice};
use crate::recent_items::RecentItems;
use crate::session::{LeagueChange, PricesState, Session, ShownItem};
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
            ClipboardEvent::Copied { .. } => Self::Item,
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
    /// Ctrl+D copies the item in the game: the copy then arrives through the clipboard, and
    /// opens the window over the game.
    hotkey: PriceCheckHotkey,
    /// When the window over the game shows, and where.
    overlay: Overlay,
    /// The clicks in the game, which close the window over the game.
    clicks: ClickWatch,
    /// Where the window over the game was drawn, in physical pixels, while it is open.
    overlay_window: Option<ScreenRect>,
    /// The height of its content at the last frame, in points; none until it is laid out once.
    overlay_height: Option<f32>,
    /// The keyboard, given to the window over the game while one of its fields is edited.
    keyboard: KeyboardFocus,
    /// The item shown, the last one copied, and the items copied before it, with their filters
    /// and offers as the player left them.
    recent: RecentItems,
    market: MarketWatch,
    item_names: ItemNamesWatch,
    /// The searches of the trade site, asked by the player.
    search: OfferSearch,
    /// The offers of the item shown, and the waiting time before the next search.
    offers: Offers,
    /// The stats of the item are unfolded, for every item copied until the player folds them.
    stats_open: bool,
    /// The filters of the search as the player set them, for the item shown: those of another
    /// item copied are its own, by default or as the player left them.
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
        let hotkey_ctx = creation.egui_ctx.clone();
        let clicks_ctx = creation.egui_ctx.clone();
        let clipboard = ClipboardWatch::start(move || clipboard_ctx.request_repaint());
        let hotkey = PriceCheckHotkey::start(clipboard.copy_marker(), move || {
            hotkey_ctx.request_repaint();
        });
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
            clipboard,
            hotkey,
            overlay: Overlay::default(),
            clicks: ClickWatch::start(move || clicks_ctx.request_repaint()),
            overlay_window: None,
            overlay_height: None,
            keyboard: KeyboardFocus::new(),
            recent: RecentItems::default(),
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
            self.forget_offers();
        }
        self.session.choose_league(league);
        self.market.price(self.session.league());
    }

    /// Forgets the offers of the item shown and of the items remembered: they were searched in
    /// the previous league.
    fn forget_offers(&mut self) {
        self.offers.clear();
        self.recent.forget_offers();
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
    /// Draws the item shown, or the empty state; the height of its blocks, which sizes the
    /// window over the game.
    fn item(
        &mut self,
        ui: &mut Ui,
        prices: PricesState,
        kind: Option<SearchKind>,
        to_top: bool,
    ) -> f32 {
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
            return EMPTY_HEIGHT;
        };
        let Ok(item) = shown.parsed() else {
            UnreadableItem::new(&self.i18n).show(ui);
            return EMPTY_HEIGHT;
        };
        // The pill shows only when its link can be opened: the league and the search known.
        let tradable = self.session.tradable(item);
        let must_identify = self.session.must_identify(item);
        let fixed_price = self.session.fixed_price(item);
        let worth = self.session.worth();
        let estimate = shown_estimate(&self.offers, item, &fixed_price, worth);
        // Without a fixed price, the estimate tells the suspicious offers.
        let reference =
            reference(&fixed_price, worth).or(estimate.map(|estimate| estimate.reference));
        let mut offers_action = None;
        let (action, height) = ui::content(ui, to_top, |ui| {
            let action = ItemBlock::new(
                item,
                shown.lines(),
                shown.stats(),
                shown.tiers(),
                &mut self.filters,
                &self.i18n,
            )
            .tradable(tradable)
            .stats(self.stats_open, self.session.filtered_base_type(item))
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
        height
    }

    /// The window over the game, drawn at every frame and shown while it is open: the blocks of
    /// the item without the title bar, beside the panel of the game the item is in, as high as
    /// its content.
    ///
    /// Two rules keep it from a panic of egui ("the user callback was never called"), since
    /// eframe 0.37 can't create a window while the main one is minimized (it then paints outside
    /// of its event loop): it is called at every frame of `ui`, so that the window, created
    /// hidden with the first frame, is never destroyed; and the fields of its builder that
    /// eframe can only set by creating the window again (its type, override-redirect, active,
    /// taskbar, decorations) never change. While one of its fields is edited, it has the
    /// keyboard through X11 (`SetInputFocus`, [`KeyboardFocus`]), not through `with_active`.
    fn show_overlay(
        &mut self,
        ctx: &Context,
        prices: PricesState,
        kind: Option<SearchKind>,
        trade: TradeStatus,
        to_top: bool,
    ) {
        // No window over the game on the other systems yet (step 14b3, Windows).
        if !cfg!(target_os = "linux") {
            return;
        }
        let target = self.overlay.shown();
        // The pointer and the game come in the physical pixels of the screen; the points of the
        // window are scaled like those of the main window: by the screen and the zoom of the
        // player (Ctrl + plus).
        let scale = ctx
            .input(|input| input.viewport().native_pixels_per_point)
            .unwrap_or(1.0)
            * ctx.zoom_factor();
        let width = to_pixels(overlay::WIDTH, scale);
        let content = self
            .overlay_height
            .map(|height| height + ui::STATUS_BAR_HEIGHT);
        let place = target.map(|target| {
            target.place(
                width,
                content.map_or(target.game.height, |height| to_pixels(height, scale)),
            )
        });
        self.overlay_window = place;
        let mut builder = ViewportBuilder::default()
            .with_title(crate::APP_NAME)
            // Out of the hands of the window manager: above the game, which keeps the focus.
            .with_override_redirect(true)
            .with_window_type(X11WindowType::Tooltip)
            .with_decorations(false)
            .with_active(false)
            .with_taskbar(false)
            .with_resizable(false)
            // Shown once its content is laid out, at its height.
            .with_visible(place.is_some() && content.is_some());
        // Hidden where it was: it doesn't move before it hides.
        if let Some(place) = place {
            builder = builder
                .with_position(pos2(to_points(place.x, scale), to_points(place.y, scale)))
                .with_inner_size(vec2(overlay::WIDTH, to_points(place.height, scale)));
        }
        let league = self.session.league().cloned();
        // Drawn at every frame, hidden while it is closed (see above).
        let height = ctx.show_viewport_immediate(
            ViewportId::from_hash_of("window-over-the-game"),
            builder,
            |ui, _| {
                // Closed, the hotkey gave the keyboard back to the game, or the window lost it
                // (Alt+Tab, which egui doesn't take for leaving the field): no field is edited,
                // so that the status bar doesn't say the keys go to the window, and a window that
                // opens again doesn't take the keyboard. The number typed is settled, as leaving
                // its field does.
                let left = self.overlay.take_leave_field();
                let lost = self.keyboard.held()
                    && ui.input(|input| {
                        input
                            .events
                            .iter()
                            .any(|event| matches!(event, egui::Event::WindowFocused(false)))
                    });
                if (place.is_none() || left || lost) && ui.ctx().text_edit_focused() {
                    self.filters.settle_edited(Window::OverTheGame);
                    ui.ctx().memory_mut(egui::Memory::stop_text_input);
                }
                place?;
                // Its own ids: the scroll and the panels of the main window are its own.
                let height = ui
                    .push_id("window-over-the-game", |ui| {
                        StatusBar::sources(&self.i18n, prices, trade)
                            .league(league.as_ref())
                            .keyboard(self.keyboard.held())
                            .show(ui);
                        self.item(ui, prices, kind, to_top)
                    })
                    .inner;
                Some((height, ui.ctx().text_edit_focused()))
            },
        );
        let editing = height.is_some_and(|(_, editing)| editing);
        let height = height.map(|(height, _)| height);
        if !editing {
            self.filters.forget_edited(Window::OverTheGame);
        }
        // The status bar, drawn before the fields, says it at the next frame.
        let held = self.keyboard.held();
        if self.keyboard.follow(editing) != held {
            ctx.request_repaint();
        }
        if height.is_some() && self.overlay_height != height {
            self.overlay_height = height;
            ctx.request_repaint();
        }
    }

    /// Takes in an event of the clipboard watching: an item copied opens the window over the
    /// game when the hotkey copied it. The item shown, copied again, keeps its filters and
    /// offers; another item copied leaves them to the items remembered, and shows its own.
    fn take_clipboard_event(&mut self, event: ClipboardEvent) {
        let mut back = None;
        if let ClipboardEvent::Copied { item, hotkey } = &event {
            let was_open = self.overlay.shown().is_some();
            // A window that opens is laid out once before it shows; an open one keeps its
            // height until the next frame, without blinking.
            if self.overlay.copied(*hotkey) && !was_open {
                self.overlay_height = None;
            }
            // The clipboard watching alone tells the copy of the hotkey.
            if copied_again(hotkey.is_some(), self.recent.shown(), item) {
                // Its filters and offers stay.
                self.view = View::Item;
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
        self.view = self.view.after(&event);
        self.session.apply(event);
        // Read again with the names of the trade site arrived since it was left, its lines may
        // have moved, the filters set on them no longer falling on the same lines.
        let shown = self.session.item().map(ShownItem::shape);
        if lines_moved(back.as_deref(), shown.as_deref()) {
            self.filters.forget_stats();
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
    let stats = Stats::new(
        item,
        shown.lines(),
        shown.stats(),
        shown.tiers(),
        session.filtered_base_type(item),
    );
    SearchFilters {
        states: filters.lit_states_of(item).collect(),
        ..shown.stats().search_filters(&stats.filters(filters))
    }
}

impl App for ExileMarketApp {
    /// Takes in what happened since the last frame; `ui` draws the session, and takes in the
    /// clicks of the components it draws.
    fn logic(&mut self, ctx: &Context, _frame: &mut Frame) {
        let now = Instant::now();
        // In this order, so that each comes after what happened before it: the events of the
        // clipboard read so far, then the presses of the hotkey (each told before its keys, so
        // before its item), then the clicks (pressed before the items taken in below, they
        // close the window as it was, not the one these items open), and only then the items.
        let mut events = Vec::new();
        self.clipboard.receive(|event| events.push(event));
        self.hotkey
            .receive(|press| self.overlay.hotkey(press.target, press.copy, now));
        let window = self.overlay_window;
        let clicks_read = self
            .clicks
            .receive(|point| self.overlay.clicked(point, window));
        // Without its clicks, the window over the game couldn't close: it no longer opens.
        if !clicks_read && self.overlay.turn_off() {
            log::warn!(
                "The window over the game is turned off: the clicks that close it can't be read"
            );
        }
        for event in events {
            self.take_clipboard_event(event);
        }
        if let Some(left) = self.overlay.expire(now) {
            ctx.request_repaint_after(left);
        }
        self.clicks.watch(self.overlay.shown().is_some());
        let main_visible = ctx.input(|input| input.viewport().visible());
        if paint_while_hidden(self.overlay.shown().is_some(), main_visible) {
            ctx.send_viewport_cmd(ViewportCommand::RequestPaintWhileHidden);
        }
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
            self.forget_offers();
            if self.settings.set_league(&league) {
                self.settings_file.save(&self.settings);
            }
        }
        // The league shown at the first launch, or after the end of a league.
        self.market.price(self.session.league());
        self.item_names.receive(|names| {
            if self.session.apply_item_names(names) {
                self.filters.forget_stats();
            }
        });
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
        // Taken once for both windows.
        let to_top = std::mem::take(&mut self.scroll_to_top);
        self.show_overlay(&ctx, prices, search_kind, trade, to_top);
        // Another item copied leaves the field the player was in, its number already settled,
        // so that the caret doesn't go on in the same field of that item.
        if to_top && ctx.text_edit_focused() {
            ctx.memory_mut(egui::Memory::stop_text_input);
        }
        match self.view {
            View::Item => {
                self.item(ui, prices, search_kind, to_top);
            }
            View::Settings => {
                if let Some(change) = SettingsPanel::new(&self.settings, &self.i18n).show(ui) {
                    self.change_setting(ui.ctx(), change);
                }
            }
        }
        // No field of the main window edited, or its field no longer drawn (the settings
        // opened): its number settled, again when leaving it settled it already.
        if !ctx.text_edit_focused() {
            self.filters.settle_edited(Window::Main);
        }
    }

    /// eframe only keeps the size and the position of the window: the state of the widgets
    /// starts again at each launch.
    fn persist_egui_memory(&self) -> bool {
        false
    }
}

/// Whether the interface must run while the main window is hidden: while the window over the
/// game shows, which eframe only draws with the main one. Not while the main window shows: the
/// command also asks for the next frame, which would redraw both windows at every frame.
fn paint_while_hidden(overlay_shown: bool, main_visible: Option<bool>) -> bool {
    overlay_shown && main_visible == Some(false)
}

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

/// The height of the window over the game for an item that can't be read, in points.
const EMPTY_HEIGHT: f32 = 160.0;

/// `points` in the physical pixels of a screen of `scale` pixels per point.
#[expect(
    clippy::cast_possible_truncation,
    reason = "a size of the screen, far within i32"
)]
fn to_pixels(points: f32, scale: f32) -> i32 {
    (points * scale).ceil() as i32
}

/// `pixels` of the screen in points, for a screen of `scale` pixels per point.
#[expect(
    clippy::cast_precision_loss,
    reason = "a position of the screen, far within f32"
)]
fn to_points(pixels: i32, scale: f32) -> f32 {
    pixels as f32 / scale
}

/// The price of one item that tells a suspicious offer: the price of a fixed-name item, with
/// the `worth` of the currencies of its prices. Both come from the prices of the league
/// shown: the rates beside the fixed price are those of `worth`, which also prices the other
/// currencies.
fn reference<'a>(
    fixed_price: &Result<(FixedPrice, Rates), NoFixedPrice>,
    worth: Option<Worth<'a>>,
) -> Option<Reference<'a>> {
    let exalted = match fixed_price {
        Ok((FixedPrice::Currency(currency), _)) => currency.price.exalted,
        Ok((FixedPrice::Unique(unique), _)) => unique.price.exalted,
        Ok((FixedPrice::Uniques(_), _)) | Err(_) => return None,
    };
    Some(Reference {
        exalted,
        worth: worth?,
    })
}

/// The estimated price of `item`, the item shown, without a fixed price, from the `offers` of
/// its search and the `worth` of the currencies with the prices of the league.
fn shown_estimate<'a>(
    offers: &Offers,
    item: &Item,
    fixed_price: &Result<(FixedPrice, Rates), NoFixedPrice>,
    worth: Option<Worth<'a>>,
) -> Option<Estimate<'a>> {
    let Err(NoFixedPrice::NotFixedName) = fixed_price else {
        return None;
    };
    let searched_catalyst = offers.searched_catalyst(item);
    let OffersState::Items {
        offers,
        total,
        first_page,
        ..
    } = offers.state()
    else {
        return None;
    };
    // The next offers loaded don't move it.
    estimate::estimate(&offers[..*first_page], *total, worth?, searched_catalyst)
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
    fn a_lit_state_tag_reaches_the_search() {
        use crate::item::ItemState;
        use crate::trade::query::Query;

        let mut session = Session::new(Some(League::new("Standard")));
        session.apply(ClipboardEvent::Copied {
            item: CopiedItem::from_text(
                "Item Class: Rings\nRarity: Rare\nWoe Loop\nRuby Ring\n--------\n\
                 Item Level: 80\n--------\nCorrupted",
            )
            .unwrap(),
            hotkey: None,
        });
        let shown = session.item().expect("an item");
        let item = shown.parsed().expect("read");
        let corrupted = |filters: &Filters| {
            let search = session
                .trade_search(item, search_filters(&session, shown, item, filters))
                .expect("a search");
            let Query::Items(items) = search.query else {
                panic!("a search of items");
            };
            items.states.corrupted
        };
        let mut filters = Filters::default();

        assert_eq!(corrupted(&filters), None);
        filters.toggle_state(ItemState::Corrupted);
        assert_eq!(corrupted(&filters), Some(true));
    }

    #[test]
    fn paints_while_hidden_only_for_the_window_over_the_game_and_a_hidden_main_window() {
        assert!(paint_while_hidden(true, Some(false)));
        assert!(!paint_while_hidden(true, Some(true)));
        assert!(!paint_while_hidden(true, None));
        assert!(!paint_while_hidden(false, Some(false)));
    }

    #[test]
    fn the_hotkey_on_the_item_shown_keeps_it_as_it_is() {
        let ring = CopiedItem::from_text("Item Class: Rings\nRarity: Rare\nWoe Loop\nRuby Ring")
            .expect("an item");
        let amulet =
            CopiedItem::from_text("Item Class: Amulets\nRarity: Rare\nDoom Idol\nGold Amulet")
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
    fn the_gear_opens_the_settings_then_closes_them() {
        assert_eq!(View::Item.toggled(), View::Settings);
        assert_eq!(View::Settings.toggled(), View::Item);
    }

    #[test]
    fn a_copied_item_closes_the_settings() {
        let copied = ClipboardEvent::Copied {
            item: CopiedItem::from_text("Item Class: Rings\nRarity: Rare\nWoe Loop\nRuby Ring")
                .unwrap(),
            hotkey: None,
        };

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

    const RATES: Rates = Rates {
        chaos: 70.0,
        divine: 700.0,
    };
    const WORTH: Worth = Worth {
        rates: RATES,
        currencies: &[],
    };

    /// Offers of a bow for `amounts` Divine Orbs.
    fn divines(amounts: &[f64]) -> Vec<crate::trade::offers::ItemOffer> {
        use crate::trade::offers::{ItemOffer, OfferItem, Price, SellerStatus};

        amounts
            .iter()
            .map(|&amount| ItemOffer {
                id: "a".to_owned(),
                price: Some(Price {
                    amount,
                    currency: "divine".to_owned(),
                }),
                seller: SellerStatus::Online,
                listed: None,
                whisper: String::new(),
                item: OfferItem {
                    rarity: None,
                    name: None,
                    base_type: "Gemini Bow".to_owned(),
                    item_level: None,
                    corrupted: false,
                    quality: None,
                    gem_level: None,
                    gem_sockets: None,
                    requirements: crate::item::Requirements::default(),
                    waystone_properties: Vec::new(),
                    waystone_unread: false,
                    modifiers: Vec::new(),
                },
            })
            .collect()
    }

    /// The owner's amulet of mana quality.
    fn amulet() -> Item {
        Item::parse(
            &CopiedItem::from_text(include_str!(
                "item/test-items/en-owner-rare-amulet-catalyst.txt"
            ))
            .unwrap(),
        )
        .unwrap()
    }

    /// A search of items with the filters of `properties`.
    fn searched(properties: Vec<crate::trade::query::PropertyFilter>) -> Search {
        use crate::trade::query::{ItemQuery, Query, StateFilters};

        Search {
            language: crate::language::Language::English,
            league: League::new("Standard"),
            query: Query::Items(ItemQuery {
                name: None,
                base_type: Some("Gemini Bow".to_owned()),
                category: None,
                properties,
                rarity: None,
                states: StateFilters {
                    corrupted: Some(false),
                    ..StateFilters::default()
                },
                stats: Vec::new(),
            }),
        }
    }

    /// Offers left to fetch, or none.
    fn rest(more: bool) -> crate::trade::search::Rest {
        let left: &[&str] = if more { &["k"] } else { &[] };
        crate::trade::search::Rest::new(crate::language::Language::English, "Q1", left)
    }

    /// The answer to a search: `offers` of the `total` found, 10 at most; the next search
    /// allowed from `now`.
    fn answer(
        offers: Vec<crate::trade::offers::ItemOffer>,
        total: u64,
        more: bool,
        now: Instant,
    ) -> crate::trade::search::Answer {
        use crate::trade::search::{Answer, NextSearch, Outcome};

        Answer {
            outcome: Outcome::Items {
                offers,
                total,
                rest: rest(more),
            },
            next_search: NextSearch {
                items: now,
                exchange: now,
            },
        }
    }

    #[test]
    fn the_estimate_counts_the_answer_to_the_search_of_an_item_without_a_fixed_price() {
        use crate::trade::search::{Answer, NextSearch, Outcome};

        let amulet = amulet();
        let now = Instant::now();
        let rare = Err(NoFixedPrice::NotFixedName);
        let mut offers = Offers::default();
        let _ = offers.search(searched(Vec::new()), now);
        // One offer of the first ten was gone since the search.
        offers.apply(
            answer(
                divines(&[1.0, 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8]),
                30,
                true,
                now,
            ),
            SystemTime::now(),
        );
        let first = shown_estimate(&offers, &amulet, &rare, Some(WORTH)).unwrap();
        let _ = offers.more();
        // Counted, the first of them would lower the estimate to 1.1 div.
        offers.apply(
            Answer {
                outcome: Outcome::More {
                    offers: divines(&[0.5; 10]),
                    rest: rest(true),
                },
                next_search: NextSearch {
                    items: now,
                    exchange: now,
                },
            },
            SystemTime::now(),
        );

        assert_eq!(
            shown_estimate(&offers, &amulet, &rare, Some(WORTH)),
            Some(first)
        );
        assert!((first.exalted - 1.2 * 700.0).abs() < 1e-9);
        assert_eq!(first.total, 30);
        assert_eq!(
            shown_estimate(&offers, &amulet, &rare, None),
            None,
            "no rates yet"
        );
        assert_eq!(
            shown_estimate(&offers, &amulet, &Err(NoFixedPrice::NotPriced), Some(WORTH)),
            None,
            "a fixed-name item isn't estimated"
        );
        let _ = offers.search(searched(Vec::new()), now);
        assert_eq!(
            shown_estimate(&offers, &amulet, &rare, Some(WORTH)),
            None,
            "while searching"
        );
    }

    #[test]
    fn the_price_of_a_unique_tells_its_suspicious_offers_in_every_priced_currency() {
        use crate::prices::{CurrencyPrice, Price, UniqueModifiers, UniquePrice};

        let price = |exalted| Price {
            exalted,
            quantity: 100,
            history: Vec::new(),
        };
        let unique = UniquePrice {
            name: "Storm Bite".to_owned(),
            base_type: "Gemini Bow".to_owned(),
            price: price(2000.0),
            modifiers: UniqueModifiers::default(),
        };
        let currencies = [CurrencyPrice {
            name: "Vaal Orb".to_owned(),
            exchange_id: "vaal".to_owned(),
            price: price(10.0),
        }];
        let worth = Worth {
            rates: RATES,
            currencies: &currencies,
        };
        let fixed = Ok((FixedPrice::Unique(&unique), RATES));

        let unique_reference = reference(&fixed, Some(worth)).unwrap();

        // 40 Vaal Orbs are 400 ex, under a quarter of 2,000; 60 are 600 ex.
        assert!(unique_reference.suspicious(40.0, "vaal"));
        assert!(!unique_reference.suspicious(60.0, "vaal"));
        assert_eq!(reference(&fixed, None), None, "the prices not known");
        assert_eq!(
            reference(&Err(NoFixedPrice::NotFixedName), Some(worth)),
            None,
            "the estimate tells them"
        );
    }

    #[test]
    fn the_estimate_leaves_out_another_kind_of_quality_only_when_the_quality_is_searched() {
        use crate::filters::Property;
        use crate::item::{Catalyst, QualityKind};
        use crate::trade::offers::OfferQuality;
        use crate::trade::query::PropertyFilter;

        let amulet = amulet();
        // The cheapest offer, of attack quality.
        let mut first_page = divines(&[1.0, 1.1, 1.2]);
        first_page[0].item.quality = Some(OfferQuality {
            value: 40,
            kind: Some(QualityKind {
                catalyst: Some(Catalyst::Attack),
                words: "Attack Modifiers".to_owned(),
            }),
        });
        let quality = PropertyFilter {
            property: Property::Quality,
            minimum: Some(40.0),
            maximum: None,
        };

        for (properties, counted) in [(vec![quality], 2), (Vec::new(), 3)] {
            let mut offers = Offers::default();
            let now = Instant::now();
            let _ = offers.search(searched(properties), now);
            offers.apply(answer(first_page.clone(), 3, false, now), SystemTime::now());
            let estimate = shown_estimate(
                &offers,
                &amulet,
                &Err(NoFixedPrice::NotFixedName),
                Some(WORTH),
            )
            .unwrap();
            assert_eq!(estimate.counted, counted);
        }
    }

    #[test]
    fn the_state_of_the_clipboard_keeps_the_view() {
        for event in [ClipboardEvent::Unreadable, ClipboardEvent::Readable] {
            assert_eq!(View::Settings.after(&event), View::Settings);
            assert_eq!(View::Item.after(&event), View::Item);
        }
    }
}
