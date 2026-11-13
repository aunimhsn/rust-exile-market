//! The blocks of an item drawn without a screen, with every copied test item: nothing is
//! checked but that the drawing ends. A panic while drawing (an index past the end, a slice
//! inside a character) closes the application in the middle of a game, and the tests of the
//! models never run the drawing.

use egui::{Event, Order, RawInput, Rect, pos2};

use super::*;
use crate::item::test_items;
use crate::language::Language;
use crate::market::MarketEvent;
use crate::price::{CurrencyPrice, Price, PriceList, UniqueModifiers, UniquePrice};
use crate::trade::offers::tests::{every_exchange_offer, every_item_offer};
use crate::trade::query::Query;
use crate::trade::search::{Answer, Failure, NextSearch, Outcome, Rest};

/// The default width of the window, and the narrowest the player can give it (`main.rs`).
const WIDTHS: [f32; 2] = [420.0, 360.0];
/// Tall enough for every block to be drawn whole.
const HEIGHT: f32 = 1600.0;
/// Between two places of the pointer going down the window: less than a row of offers.
const POINTER_STEP: f32 = 18.0;

fn league() -> League {
    League::new("Standard")
}

/// A session showing the copied item `text`.
fn session_showing(text: &str) -> Session {
    let mut session = Session::new(Some(league()));
    session.apply(ClipboardEvent::Copied {
        item: CopiedItem::from_text(text).expect("an item"),
        hotkey: None,
    });
    session
}

/// Prices of the league in which `item` has a fixed price whatever it is: as a unique, and as
/// an item of the currency exchange, traded `quantity` times.
fn prices_of(item: &Item, quantity: u32) -> PriceList {
    let price = Price {
        exalted: 1234.5,
        quantity,
        history: vec![Some(1200.0), None, Some(0.001), Some(98_765.4)],
    };
    PriceList {
        league: league(),
        updated: SystemTime::now(),
        rates: Rates {
            chaos: 70.0,
            divine: 700.0,
        },
        uniques: vec![UniquePrice {
            name: item.name().unwrap_or_default().to_owned(),
            base_type: item.base_type().to_owned(),
            price: price.clone(),
            modifiers: UniqueModifiers::default(),
        }],
        currencies: vec![CurrencyPrice {
            name: item.base_type().to_owned(),
            exchange_id: "divine".to_owned(),
            price,
        }],
    }
}

/// The answer of the trade site to a search of `kind`, with offers of every kind the tests
/// have and more to fetch.
fn answer(kind: SearchKind, language: Language) -> Answer {
    let now = Instant::now();
    let outcome = match kind {
        SearchKind::Items => Outcome::Items {
            offers: every_item_offer(language),
            total: 1234,
            rest: Rest::new(language, "Q1", &["k"]),
        },
        SearchKind::Exchange => Outcome::Exchange {
            offers: every_exchange_offer(),
            total: 2,
        },
    };
    Answer {
        outcome,
        next_search: NextSearch {
            items: now,
            exchange: now,
        },
    }
}

/// What the trade site may answer to a search, the limits letting the next one wait.
fn failure(outcome: Outcome) -> Answer {
    let later = Instant::now() + Duration::from_mins(5);
    Answer {
        outcome,
        next_search: NextSearch {
            items: later,
            exchange: later,
        },
    }
}

/// The interface as the application sets it up, without a screen.
fn context() -> Context {
    let ctx = Context::default();
    ui::install(&ctx);
    ctx
}

/// What was drawn, so that the test tells when it no longer draws what it is for.
#[derive(Default)]
struct Drawn {
    frames: usize,
    /// The time of egui, which goes on at each frame, whatever the item.
    time: f64,
    /// The frames with a layer above the blocks: the tooltip of an offer.
    with_a_tooltip: usize,
    with_a_fixed_price: usize,
    with_offers_of_items: usize,
    with_offers_of_the_exchange: usize,
    /// The frames whose price block wrote an estimated price, and those where it wrote the
    /// line that asks for a modifier to be checked instead.
    with_an_estimate: usize,
    with_a_check_hint: usize,
}

/// Whether one of `shapes`, the shapes of a frame, writes `text`.
fn writes(shapes: &[egui::epaint::ClippedShape], text: &str) -> bool {
    fn shape_writes(shape: &egui::Shape, text: &str) -> bool {
        match shape {
            egui::Shape::Text(shape) => shape.galley.text() == text,
            egui::Shape::Vec(shapes) => shapes.iter().any(|shape| shape_writes(shape, text)),
            _ => false,
        }
    }
    shapes
        .iter()
        .any(|clipped| shape_writes(&clipped.shape, text))
}

/// An item shown, with what the application keeps next to its session.
struct Scene {
    session: Session,
    offers: Offers,
    filters: Filters,
    i18n: Translator,
}

impl Scene {
    fn new(text: &str, interface: Language) -> Self {
        Self {
            session: session_showing(text),
            offers: Offers::default(),
            filters: Filters::default(),
            i18n: Translator::new(interface),
        }
    }

    /// The item shown, read: every test item is.
    fn item(&self) -> &Item {
        let shown = self.session.item().expect("an item shown");
        shown.parsed().expect("an item the parser reads")
    }

    fn kind(&self) -> Option<SearchKind> {
        self.session.search_kind(self.item())
    }

    /// The search of the offers of the item, with its filters as they are.
    fn search(&self) -> Option<Search> {
        let shown = self.session.item()?;
        let item = shown.parsed().ok()?;
        let filters = search_filters(&self.session, shown, item, &self.filters);
        self.session.trade_search(item, filters)
    }

    /// Draws one frame of `width`, the pointer at the height `pointer` in the middle of the
    /// window; the height of the blocks.
    fn draw(
        &mut self,
        ctx: &Context,
        drawn: &mut Drawn,
        width: f32,
        pointer: Option<f32>,
        stats_open: bool,
        prices: PricesState,
    ) -> f32 {
        drawn.time += 0.1;
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(width, HEIGHT))),
            time: Some(drawn.time),
            events: pointer
                .map(|y| Event::PointerMoved(pos2(width * 0.5, y)))
                .into_iter()
                .collect(),
            ..Default::default()
        };
        let kind = self.kind();
        let mut height = 0.0;
        let output = ui::fixtures::run_frame(ctx, input, |ui| {
            let shown = self.session.item().expect("an item shown");
            let view = ItemView {
                session: &self.session,
                shown,
                item: shown.parsed().expect("an item the parser reads"),
                offers: &self.offers,
                filters: &mut self.filters,
                stats_open,
                prices,
                kind,
                has_bug_report: true,
                i18n: &self.i18n,
            };
            (_, height) = ui::content(ui, false, |ui| view.show(ui));
        });
        drawn.frames += 1;
        let above = ctx.memory(|memory| {
            let layers = memory.areas().visible_layer_ids();
            layers.iter().any(|layer| layer.order > Order::Middle)
        });
        drawn.with_a_tooltip += usize::from(above);
        drawn.with_a_fixed_price += usize::from(self.session.fixed_price(self.item()).is_ok());
        // By what the price block wrote, not by its rule told again.
        let wrote = |text: &str| usize::from(writes(&output.shapes, text));
        let estimated = self.i18n.text(Message::EstimatedPriceLabel).to_uppercase();
        drawn.with_an_estimate += wrote(&estimated);
        drawn.with_a_check_hint += wrote(&self.i18n.text(Message::CheckToEstimate));
        match self.offers.state() {
            OffersState::Items(_) => drawn.with_offers_of_items += 1,
            OffersState::Exchange { .. } => drawn.with_offers_of_the_exchange += 1,
            _ => {}
        }
        height
    }

    /// Draws the item at both widths, its stats open then closed, in each state of the
    /// prices.
    fn draw_each_way(&mut self, ctx: &Context, drawn: &mut Drawn) {
        let states = [
            PricesState::Downloading { progress: 0.4 },
            PricesState::Unavailable,
            PricesState::Known {
                age: Duration::from_mins(10),
                offline: false,
            },
            PricesState::Known {
                age: Duration::from_hours(30),
                offline: true,
            },
        ];
        for width in WIDTHS {
            for (prices, stats_open) in states.into_iter().zip([true, false].into_iter().cycle()) {
                self.draw(ctx, drawn, width, None, stats_open, prices);
            }
        }
    }

    /// Draws the item with the pointer going down its blocks, over each row.
    fn draw_under_the_pointer(&mut self, ctx: &Context, drawn: &mut Drawn) {
        let known = PricesState::Known {
            age: Duration::from_mins(10),
            offline: false,
        };
        let height = self.draw(ctx, drawn, WIDTHS[0], None, true, known);
        let mut pointer = 0.0;
        while pointer < height.min(HEIGHT) {
            self.draw(ctx, drawn, WIDTHS[0], Some(pointer), true, known);
            pointer += POINTER_STEP;
        }
    }

    /// Searches the offers of the item, as a click on the search block does, and takes
    /// `answer` in.
    fn search_answered(&mut self, answer: Answer) {
        let search = self.search().expect("a search for an item with a kind");
        assert!(matches!(
            (&search.query, self.kind()),
            (Query::Items(_), Some(SearchKind::Items))
                | (Query::Exchange(_), Some(SearchKind::Exchange))
        ));
        let order = self.offers.search(search, Instant::now());
        assert!(order.is_some(), "the search is allowed");
        self.offers.apply(answer, SystemTime::now());
    }
}

/// Every state of the offers of the item of `scene`, drawn.
fn draw_the_offers(scene: &mut Scene, ctx: &Context, drawn: &mut Drawn, kind: SearchKind) {
    let interface = scene.i18n.language();
    // Searching.
    let search = scene.search().expect("a search for an item with a kind");
    let _ = scene.offers.search(search, Instant::now());
    scene.draw_each_way(ctx, drawn);
    // Its offers, then the next ones asked, which don't come.
    scene
        .offers
        .apply(answer(kind, interface), SystemTime::now());
    scene.draw_each_way(ctx, drawn);
    scene.draw_under_the_pointer(ctx, drawn);
    let _ = scene.offers.more();
    scene.draw_each_way(ctx, drawn);
    if scene.offers.busy() {
        scene.offers.apply(
            Answer {
                outcome: Outcome::Failed(Failure::Unreadable),
                ..answer(kind, interface)
            },
            SystemTime::now(),
        );
        scene.draw_each_way(ctx, drawn);
    }
    // No offer at all.
    let none = match kind {
        SearchKind::Items => Outcome::Items {
            offers: Vec::new(),
            total: 0,
            rest: Rest::new(interface, "Q2", &[]),
        },
        SearchKind::Exchange => Outcome::Exchange {
            offers: Vec::new(),
            total: 0,
        },
    };
    scene.search_answered(Answer {
        outcome: none,
        ..answer(kind, interface)
    });
    scene.draw_each_way(ctx, drawn);
    // The trade site can't be reached, turns the search down, can't be read, then refuses,
    // with a long wait.
    for failure in [Failure::Unreachable, Failure::Refused, Failure::Unreadable] {
        scene.search_answered(Answer {
            outcome: Outcome::Failed(failure),
            ..answer(kind, interface)
        });
        scene.draw_each_way(ctx, drawn);
    }
    scene.search_answered(failure(Outcome::Limited {
        until: Instant::now() + Duration::from_mins(5),
        refused: true,
    }));
    scene.draw_each_way(ctx, drawn);
}

/// The interface review of 2026-11-10: the five `show` of the item view were never run by a
/// test.
#[test]
fn every_test_item_is_drawn_in_english_without_a_panic() {
    draw_every_test_item(Language::English);
}

/// A test for each language of the interface, so that they run side by side.
#[test]
fn every_test_item_is_drawn_in_french_without_a_panic() {
    draw_every_test_item(Language::French);
}

/// Draws every test item, of both languages of the game, with the interface in `interface`.
fn draw_every_test_item(interface: Language) {
    let ctx = context();
    let mut drawn = Drawn::default();
    for (file, text) in test_items::ALL {
        let mut scene = Scene::new(text, interface);
        // Without the prices of the league, nor a search.
        scene.draw_each_way(&ctx, &mut drawn);
        scene.draw_under_the_pointer(&ctx, &mut drawn);
        // With prices, on few trades then on many.
        for quantity in [1, 5000] {
            let prices = prices_of(scene.item(), quantity);
            scene.session.apply_market(MarketEvent::Prices(prices), &[]);
            scene.draw_each_way(&ctx, &mut drawn);
        }
        if let Some(kind) = scene.kind() {
            assert!(scene.search().is_some(), "{file}: a search of {kind:?}");
            draw_the_offers(&mut scene, &ctx, &mut drawn, kind);
        }
    }

    // Not vacuous: each part of the view was drawn many times.
    let items = test_items::ALL.len();
    assert!(drawn.frames > items * 50, "{} frames", drawn.frames);
    assert!(
        drawn.with_a_fixed_price > items,
        "{}",
        drawn.with_a_fixed_price
    );
    assert!(
        drawn.with_offers_of_items > items,
        "{}",
        drawn.with_offers_of_items
    );
    assert!(
        drawn.with_offers_of_the_exchange > 0,
        "an item of the currency exchange among the test items"
    );
    assert!(drawn.with_a_tooltip > items, "{}", drawn.with_a_tooltip);
    assert!(drawn.with_an_estimate > items, "{}", drawn.with_an_estimate);
    assert!(
        drawn.with_a_check_hint > items,
        "{}",
        drawn.with_a_check_hint
    );
}
