//! The application: the session, the settings, the clipboard watching, the price-check
//! hotkey and the price database, and the window built from the components of [`crate::ui`].

mod desk;
mod item_view;
mod leagues;
mod open_lists;
mod overlay_window;

use std::hash::{BuildHasher, RandomState};
use std::time::{Duration, Instant, SystemTime};

use eframe::{App, CreationContext, Frame};
use egui::{
    Context, Key, Modifiers, OpenUrl, Ui, ViewportBuilder, ViewportCommand, ViewportId,
    WindowLevel, X11WindowType, pos2, vec2,
};

use self::desk::{Desk, DeskRequest, EMPTY_HEIGHT, Surroundings};
use self::open_lists::{OpenList, OpenLists};
#[cfg(target_os = "windows")]
use self::overlay_window::overlay_frame;
use self::overlay_window::{keep_only_esc, paint_while_hidden, to_pixels, to_points};
use crate::bug_report::{Facts, JoinedItem, LogSession, Report};
use crate::clipboard::{self, ClipboardEvent, ClipboardWatch};
use crate::filters::Window;
use crate::hotkey::PriceCheckHotkey;
use crate::i18n::{DOWNLOAD_QUOTE_COUNT, Message, Translator};
use crate::item::CopiedItem;
use crate::league::League;
use crate::market::MarketWatch;
use crate::overlay::{self, Activation, ClickWatch, KeyboardFocus, Overlay, ScreenRect};
use crate::session::{LeagueChange, PricesState};
use crate::settings::{SettingChange, Settings, SettingsFile};
use crate::trade::Refusal;
use crate::trade::names_watch::ItemNamesWatch;
use crate::trade::query::SearchKind;
use crate::trade::search::OfferSearch;
use crate::ui::{
    self, BugReportAction, BugReportPanel, EmptyState, PriceDownload, ResizeEdges, SettingsPanel,
    StatusBar, TitleBar, TitleBarAction, TradeStatus, WarningBand,
};
use crate::update::{Release, UpdateCheck};

/// What the window shows under the title bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum View {
    /// The copied item, or the empty state before the first one.
    Item,
    Settings,
    /// The preview of the bug report.
    BugReport,
}

impl View {
    /// The view after a click on the icon of `panel`, the gear or the bug: it opens its panel in
    /// place of what the window shows, and closes it.
    fn toggled(self, panel: Self) -> Self {
        if self == panel { Self::Item } else { panel }
    }

    /// The view after `event`: a copied item closes the settings and the bug report, so that the
    /// player sees it.
    fn after(self, event: &ClipboardEvent) -> Self {
        match event {
            ClipboardEvent::Copied { .. } => Self::Item,
            ClipboardEvent::Unreadable | ClipboardEvent::Readable => self,
        }
    }
}

pub struct ExileMarketApp {
    i18n: Translator,
    /// The item shown and what the player did with it.
    desk: Desk,
    settings: Settings,
    settings_file: SettingsFile,
    view: View,
    /// The list open over the view, if any.
    lists: OpenLists,
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
    /// The window over the game made the active window while the game holds the pointer (a
    /// single screen, on Linux).
    activation: Activation,
    market: MarketWatch,
    item_names: ItemNamesWatch,
    /// The searches of the trade site, asked by the player.
    search: OfferSearch,
    /// The sentence shown during the download of the prices of a league, drawn at its start.
    download_quote: Option<(League, usize)>,
    /// The latest release, asked once at launch.
    update_check: UpdateCheck,
    /// A release newer than the running version, once GitHub told it.
    release: Option<Release>,
    /// Its page was opened from the bubble: the bubble is hidden until the next launch.
    release_opened: bool,
    /// The bug report, built when its preview opens; shown while the view is the bug report.
    bug_report: Option<Report>,
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
        let update_ctx = creation.egui_ctx.clone();
        let clipboard = ClipboardWatch::start(move || clipboard_ctx.request_repaint());
        let hotkey = PriceCheckHotkey::start(
            clipboard.copy_marker(),
            settings.hotkey().on,
            settings.hotkey().keys,
            move || hotkey_ctx.request_repaint(),
        );
        // The prices of the league of the last session, until the leagues arrive.
        let market = MarketWatch::start(settings.league(), move || market_ctx.request_repaint());

        // A refusal of the trade site makes its two threads wait.
        let refusal = Refusal::default();
        Self {
            // The language of the copied items never changes the interface: it only chooses
            // the item names and the trade site.
            i18n: Translator::new(settings.language().language()),
            desk: Desk::new(settings.league()),
            settings,
            settings_file,
            view: View::Item,
            lists: OpenLists::default(),
            clipboard,
            hotkey,
            overlay: Overlay::default(),
            clicks: ClickWatch::start(move || clicks_ctx.request_repaint()),
            overlay_window: None,
            overlay_height: None,
            keyboard: KeyboardFocus::new(),
            activation: Activation::new(),
            market,
            item_names: ItemNamesWatch::start(refusal.clone(), move || {
                names_ctx.request_repaint();
            }),
            search: OfferSearch::start(refusal, move || search_ctx.request_repaint()),
            download_quote: None,
            update_check: UpdateCheck::start(move || update_ctx.request_repaint()),
            release: None,
            release_opened: false,
            bug_report: None,
        }
    }

    fn act_on(&mut self, ctx: &Context, action: TitleBarAction) {
        match action {
            TitleBarAction::Move => ctx.send_viewport_cmd(ViewportCommand::StartDrag),
            TitleBarAction::Home => {
                self.view = View::Item;
                self.lists.close_all();
            }
            TitleBarAction::Leagues if self.desk.session().leagues().is_some() => {
                self.lists.toggle(OpenList::Leagues);
            }
            // Still a click outside the list of the keys.
            TitleBarAction::Leagues => self.lists.close_all(),
            TitleBarAction::BugReport => {
                if self.view == View::BugReport {
                    self.view = View::Item;
                    self.lists.close_all();
                } else {
                    self.open_bug_report();
                }
            }
            TitleBarAction::Settings => {
                self.view = self.view.toggled(View::Settings);
                self.lists.close_all();
            }
            TitleBarAction::Minimize => ctx.send_viewport_cmd(ViewportCommand::Minimized(true)),
            TitleBarAction::Close => ctx.send_viewport_cmd(ViewportCommand::Close),
        }
    }

    /// Opens the bug report of what the window shows now, in place of its content.
    fn open_bug_report(&mut self) {
        self.view = View::BugReport;
        self.lists.close_all();
        self.bug_report = Some(self.new_bug_report());
    }

    /// The bug report of what the window shows now: the summary, the log of this session or of
    /// the previous one after a crash, and the item shown, joined by default.
    fn new_bug_report(&self) -> Report {
        let copied = self.desk.copied();
        let session = self.desk.session();
        let facts = Facts {
            interface: self.i18n.language(),
            items: copied.map(CopiedItem::language),
            league: session.league(),
            always_on_top: self.settings.always_on_top(),
            hotkey: cfg!(any(target_os = "linux", target_os = "windows"))
                .then(|| self.settings.hotkey()),
        };
        let item = copied.map(|copied| JoinedItem {
            text: copied.text().to_owned(),
            name: session
                .item()
                .and_then(|shown| shown.parsed().ok())
                .map(|item| item.name().unwrap_or(item.base_type()).to_owned()),
        });
        Report::read(&facts, item)
    }

    /// What the player did in the bug report.
    fn act_on_bug_report(&mut self, ctx: &Context, action: BugReportAction) {
        let Some(report) = &mut self.bug_report else {
            return;
        };
        match action {
            BugReportAction::JoinItem(join) => report.join_item(join),
            BugReportAction::Choose(session) => report.choose(session),
            BugReportAction::Open => {
                log::info!(
                    "Bug report opened on GitHub: {} characters, the item {}, the log of {}",
                    report.address().len(),
                    if report.sends_item() {
                        "joined"
                    } else {
                        "left out"
                    },
                    match report.session() {
                        LogSession::This => "this session",
                        LogSession::Previous => "the previous session",
                    }
                );
                ctx.open_url(OpenUrl::new_tab(report.address()));
            }
            BugReportAction::CopyLog => {
                if let Some(log) = report.whole_log() {
                    report.copy_result(clipboard::write_text(&log));
                }
            }
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
            SettingChange::HotkeyOn(on) => self.hotkey.turn(on),
            SettingChange::HotkeyKeys(keys) => self.hotkey.change_keys(keys),
        }
        self.settings_file.save(&self.settings);
        // The bands above the settings were drawn with the previous values.
        ctx.request_repaint();
    }

    /// The warnings of the band under the title bar, given the state of the prices.
    fn warnings(&self, prices: PricesState) -> Vec<String> {
        let mut warnings = Vec::new();
        if !self.desk.session().clipboard_readable() {
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

    /// Draws the item shown, or the empty state before the first one; the height of its blocks,
    /// which sizes the window over the game. In the main window, the hint of a failed search
    /// opens the bug report.
    fn item(&mut self, ui: &mut Ui, around: Surroundings) -> f32 {
        let Some(drawn) = self.desk.show_item(ui, &self.i18n, around) else {
            let empty = EmptyState::new(&self.i18n, self.hotkey.state(), self.hotkey.shortcut());
            if let (PricesState::Downloading { progress }, Some((league, quote))) =
                (around.prices, &self.download_quote)
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
        match drawn.request {
            Some(DeskRequest::Search(order)) => self.search.order(order),
            Some(DeskRequest::BugReport) => self.open_bug_report(),
            None => {}
        }
        drawn.height
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
    /// On Windows, always on top, and shown without activation; a click in it activates it all
    /// the same, and it gives the focus back to the game, unless the click opens a link, the
    /// browser then coming to the front, or goes to a field, which keeps it until it is left.
    fn show_overlay(&mut self, ctx: &Context, around: Surroundings, trade: TradeStatus) {
        // No window over the game on macOS, where Path of Exile 2 has no client.
        if !cfg!(any(target_os = "linux", target_os = "windows")) {
            return;
        }
        let target = self.overlay.shown();
        let id = ViewportId::from_hash_of("window-over-the-game");
        // The pointer and the game come in the physical pixels of the screen; the points of the
        // window are scaled by its own screen, which egui-winit converts them back with (on
        // Windows, each screen has its own scale), the main window's until it exists, and by
        // the zoom of the player (Ctrl + plus).
        let native = ctx
            .input_for(id, |input| input.viewport().native_pixels_per_point)
            .or_else(|| ctx.input(|input| input.viewport().native_pixels_per_point));
        let scale = native.unwrap_or(1.0) * ctx.zoom_factor();
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
        // Windows has no override-redirect: the window stays above the game by its level.
        if cfg!(target_os = "windows") {
            builder = builder.with_window_level(WindowLevel::AlwaysOnTop);
        }
        // Hidden where it was: it doesn't move before it hides.
        if let Some(place) = place {
            builder = builder
                .with_position(pos2(to_points(place.x, scale), to_points(place.y, scale)))
                .with_inner_size(vec2(overlay::WIDTH, to_points(place.height, scale)));
        }
        let league = self.desk.session().league().cloned();
        let held = self.keyboard.held();
        // Drawn at every frame, hidden while it is closed (see above).
        let height = ctx.show_viewport_immediate(id, builder, |ui, _| {
            // Closed, the hotkey gave the keyboard back to the game, or the window lost it
            // (Alt+Tab, which egui doesn't take for leaving the field; on Windows, whatever
            // took the focus, even while no field had it; only a window that had it loses it;
            // on a single screen, as soon as it is no longer the active window, before winit
            // tells it):
            // no field is edited, so that the status bar doesn't say the keys go to the window,
            // and a window that opens again doesn't take the keyboard. The number typed is
            // settled, as leaving its field does.
            let left = self.overlay.take_leave_field();
            let lost = self.activation.lost()
                || ui.input(|input| {
                    input
                        .events
                        .iter()
                        .any(|event| matches!(event, egui::Event::WindowFocused(false)))
                });
            if (place.is_none() || left || lost) && ui.ctx().text_edit_focused() {
                self.desk.settle_edited(Window::OverTheGame);
                ui.ctx().memory_mut(egui::Memory::stop_text_input);
            }
            #[cfg(target_os = "windows")]
            if place.is_none() {
                self.keyboard.give_back(overlay::Frame::HIDDEN);
            }
            place?;
            // On a single screen, where the window is the active one, the keys are the game's
            // (Tab its map, Space its dodge, 1 to 5 its flasks and charms) unless a field is
            // edited: only Esc does something, below, so that they neither move the keyboard
            // in the window nor press its buttons nor type in its fields.
            if self.activation.escape_closes() {
                keep_only_esc(ui);
            }
            // Its own ids: the scroll and the panels of the main window are its own.
            let height = ui
                .push_id("window-over-the-game", |ui| {
                    StatusBar::sources(&self.i18n, around.prices, trade)
                        .league(league.as_ref())
                        .keyboard(self.keyboard.held())
                        .single_screen(self.activation.holds())
                        .show(ui);
                    self.item(ui, around)
                })
                .inner;
            let editing = ui.ctx().text_edit_focused();
            // On a single screen, where the window is the active one, Esc closes it, after what
            // is on top of it (the tooltip of an offer, which takes the key), and unless it
            // leaves a field: edited at the last frame, since egui takes the keyboard away at
            // Esc before this one.
            if self.activation.escape_closes()
                && ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape))
            {
                self.activation.escape();
                self.overlay.close();
                ui.ctx().request_repaint();
            }
            // A click in it took the focus of the game: given back once it is released,
            // after the content, which may open a link with this click; kept while a field is
            // edited.
            #[cfg(target_os = "windows")]
            self.keyboard.give_back(overlay_frame(ui, editing));
            Some((height, editing))
        });
        let editing = height.is_some_and(|(_, editing)| editing);
        self.activation.editing(editing);
        let height = height.map(|(height, _)| height);
        if !editing {
            self.desk.forget_edited(Window::OverTheGame);
        }
        // The status bar, drawn before the fields, says it at the next frame: on Windows, the
        // window took the keyboard during the frame already. On a single screen, the window
        // already has the keyboard while it is the active window.
        if self.keyboard.follow(editing && !self.activation.holds()) != held {
            ctx.request_repaint();
        }
        if height.is_some() && self.overlay_height != height {
            self.overlay_height = height;
            ctx.request_repaint();
        }
    }

    /// Takes in an event of the clipboard watching: an item copied opens the window over the
    /// game when the hotkey copied it, and closes the settings and the bug report.
    fn apply_clipboard_event(&mut self, event: ClipboardEvent) {
        if let ClipboardEvent::Copied { hotkey, .. } = &event {
            let was_open = self.overlay.shown().is_some();
            // A window that opens is laid out once before it shows; an open one keeps its
            // height until the next frame, without blinking.
            if self.overlay.copied(*hotkey) && !was_open {
                self.overlay_height = None;
            }
        }
        self.view = self.view.after(&event);
        self.desk.apply_clipboard_event(event);
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
        let mut pressed = false;
        self.hotkey.receive(|press| {
            self.overlay.hotkey(press.target, press.copy, now);
            pressed = true;
        });
        if pressed {
            self.activation.hotkey();
        }
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
            self.apply_clipboard_event(event);
        }
        if let Some(left) = self.overlay.expire(now) {
            ctx.request_repaint_after(left);
        }
        // Asked again soon while it waits for the window to show, the desktop to activate it, or
        // the game to hold the pointer. A press on its way, not received yet, is pending too: its
        // copy makes the game the active window before it is told.
        let waiting = self.overlay.waiting();
        let hotkey = &self.hotkey;
        if let Some(after) = self.activation.follow(
            self.overlay.opened(),
            || waiting || hotkey.on_its_way(),
            now,
        ) {
            ctx.request_repaint_after(after);
        }
        self.clicks.watch(self.overlay.shown().is_some());
        let main_focused = ctx.input(|input| input.viewport().focused.unwrap_or(false));
        self.hotkey.follow_game(main_focused);
        let main_visible = ctx.input(|input| input.viewport().visible());
        if paint_while_hidden(self.overlay.shown().is_some(), main_visible) {
            ctx.send_viewport_cmd(ViewportCommand::RequestPaintWhileHidden);
        }
        if self
            .search
            .receive(|answer| self.desk.apply_answer(answer, SystemTime::now()))
        {
            self.desk.search_thread_lost();
        }
        let mut shown = None;
        self.market.receive(|event| {
            let closed = self.settings.closed_new_leagues();
            if let Some(LeagueChange::Shown(league)) = self.desk.apply_market(event, closed) {
                shown = Some(league);
            }
        });
        if let Some(league) = shown
            && self.settings.set_league(&league)
        {
            self.settings_file.save(&self.settings);
        }
        // The league shown at the first launch, or after the end of a league.
        self.market.price(self.desk.session().league());
        self.update_check
            .receive(|release| self.release = Some(release));
        self.settle_closed_update();
        self.item_names
            .receive(|names| self.desk.apply_item_names(names));
        // Escape closes what is on top: a list, then the settings or the bug report.
        if ctx.input(|input| input.key_pressed(Key::Escape))
            && !self.lists.escape()
            && self.view != View::Item
        {
            self.view = View::Item;
        }
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut Frame) {
        let ctx = ui.ctx().clone();
        if let Some(direction) = ResizeEdges::show(&ctx) {
            ctx.send_viewport_cmd(ViewportCommand::BeginResize(direction));
        }
        // The panels on the edges come first, so that the central one takes what remains.
        let action = TitleBar::new(self.desk.session().league(), &self.i18n)
            .leagues_known(self.desk.session().leagues().is_some())
            .settings_open(self.view == View::Settings)
            .bug_report_open(self.view == View::BugReport)
            .leagues_open(self.lists.is_open(OpenList::Leagues))
            .show(ui);
        if let Some(action) = action {
            self.act_on(&ctx, action);
        }
        self.league_overlays(&ctx, action);
        let session = self.desk.session();
        let prices = session.prices_state(SystemTime::now());
        // A sentence per download: drawn again for another league.
        self.download_quote = match (prices, session.league()) {
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
        let search_kind = self.desk.search_kind();
        // The status bar tells the wait before a search of the item shown.
        let kind = search_kind.unwrap_or(SearchKind::Items);
        let trade = self
            .desk
            .trade_status(kind, Instant::now(), SystemTime::now());
        match trade {
            // The age of the last search, and the countdown of the long wait, change with the time.
            TradeStatus::Searched(age) => {
                ctx.request_repaint_after(Duration::from_secs(60 - age.as_secs() % 60));
            }
            TradeStatus::Waiting(_) => ctx.request_repaint_after(Duration::from_secs(1)),
            TradeStatus::Idle => {}
        }
        match self.view {
            View::Item | View::BugReport => StatusBar::sources(&self.i18n, prices, trade).show(ui),
            View::Settings => StatusBar::author(&self.i18n, self.release.as_ref()).show(ui),
        }
        let main = Surroundings {
            prices,
            kind: search_kind,
            // Taken once for both windows.
            to_top: self.desk.take_scroll_to_top(),
            window: Window::Main,
        };
        self.show_overlay(
            &ctx,
            Surroundings {
                window: Window::OverTheGame,
                ..main
            },
            trade,
        );
        // Another item copied leaves the field the player was in, its number already settled,
        // so that the caret doesn't go on in the same field of that item.
        if main.to_top && ctx.text_edit_focused() {
            ctx.memory_mut(egui::Memory::stop_text_input);
        }
        match self.view {
            View::Item => {
                self.item(ui, main);
            }
            View::Settings => {
                let action = SettingsPanel::new(&self.settings, &self.i18n, self.hotkey.state())
                    .held_keys(self.hotkey.shortcut())
                    .keys_open(self.lists.is_open(OpenList::Keys))
                    .show(ui);
                if let Some(change) = action.and_then(|action| self.lists.settings(action)) {
                    self.change_setting(ui.ctx(), change);
                }
            }
            View::BugReport => {
                let action = self
                    .bug_report
                    .as_ref()
                    .and_then(|report| BugReportPanel::new(report, &self.i18n).show(ui));
                if let Some(action) = action {
                    self.act_on_bug_report(&ctx, action);
                }
            }
        }
        if self.view != View::BugReport {
            self.bug_report = None;
        }
        self.lists.follow_view(self.view == View::Settings);
        if self.view != View::Settings {
            SettingsPanel::forget(&ctx);
        }
        // No field of the main window edited, or its field no longer drawn (the settings
        // opened): its number settled, again when leaving it settled it already.
        if !ctx.text_edit_focused() {
            self.desk.settle_edited(Window::Main);
        }
    }

    /// eframe only keeps the size and the position of the window: the state of the widgets
    /// starts again at each launch.
    fn persist_egui_memory(&self) -> bool {
        false
    }
}

/// A sentence of the download of the prices, drawn at random.
fn random_quote() -> usize {
    // Each `RandomState` has its own keys: enough to draw a sentence.
    let [byte, ..] = RandomState::new().hash_one(()).to_le_bytes();
    usize::from(byte) % DOWNLOAD_QUOTE_COUNT
}

#[cfg(test)]
mod tests;
