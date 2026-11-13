use super::*;
use crate::estimate::Worth;
use crate::prices::Rates;
use crate::trade::offers::OfferItem;

const RATES: Rates = Rates {
    chaos: 75.0,
    divine: 750.0,
};
const WORTH: Worth = Worth {
    rates: RATES,
    currencies: &[],
};
/// 12.4 Divine Orbs.
const REFERENCE: Reference = Reference {
    exalted: 9300.0,
    worth: WORTH,
};

fn offer(amount: f64, currency: &str, seller: SellerStatus) -> ItemOffer {
    ItemOffer {
        id: "a".to_owned(),
        price: Some(Price {
            amount,
            currency: currency.to_owned(),
        }),
        seller,
        listed: Some(SystemTime::UNIX_EPOCH),
        whisper: String::new(),
        item: OfferItem {
            rarity: None,
            name: Some("Headhunter".to_owned()),
            base_type: "Heavy Belt".to_owned(),
            item_level: None,
            corrupted: true,
            quality: None,
            gem_level: None,
            gem_sockets: None,
            requirements: crate::item::Requirements::default(),
            waystone_properties: Vec::new(),
            waystone_unread: false,
            modifiers: Vec::new(),
        },
    }
}

#[test]
fn an_offer_under_a_quarter_of_the_market_price_is_suspicious() {
    let offers = [
        offer(6.0, "exalted", SellerStatus::Online),
        offer(1.0, "chaos", SellerStatus::Online),
        offer(3.0, "divine", SellerStatus::Online),
        offer(3.2, "divine", SellerStatus::Away),
    ];
    let now = SystemTime::UNIX_EPOCH + Duration::from_hours(2);

    let rows = item_rows(&offers, Some(REFERENCE), None, now);

    let suspicious: Vec<bool> = rows.iter().map(|row| row.suspicious).collect();
    assert_eq!(suspicious, [true, true, true, false]);
    assert_eq!(
        rows[3],
        Row {
            price: Some(RowPrice {
                amount: "3.2".to_owned(),
                currency: currencies::shown("divine"),
            }),
            suspicious: false,
            other_quality: None,
            stock: None,
            away: true,
            listed: Some(Duration::from_hours(2)),
        }
    );
    assert!(
        item_rows(&offers, None, None, now)
            .iter()
            .all(|row| !row.suspicious),
        "without a market price, nothing tells"
    );
}

#[test]
fn an_offer_of_another_kind_of_quality_is_marked_while_the_quality_is_searched() {
    use crate::item::QualityKind;
    use crate::trade::offers::OfferQuality;

    let of_kind = |amount, catalyst| {
        let mut offer = offer(amount, "divine", SellerStatus::Online);
        offer.item.quality = Some(OfferQuality {
            value: 40,
            kind: Some(QualityKind {
                catalyst: Some(catalyst),
                words: String::new(),
            }),
        });
        offer
    };
    // An attack amulet far below the market of the mana amulets: another market, not
    // bait.
    let offers = [
        of_kind(1.0, Catalyst::Attack),
        of_kind(12.0, Catalyst::Mana),
    ];
    let now = SystemTime::UNIX_EPOCH;

    let searched = item_rows(&offers, Some(REFERENCE), Some(Catalyst::Mana), now);
    let unsearched = item_rows(&offers, Some(REFERENCE), None, now);

    assert_eq!(searched[0].other_quality, Some(Catalyst::Attack));
    assert!(!searched[0].suspicious);
    assert_eq!(searched[1].other_quality, None);
    assert_eq!(unsearched[0].other_quality, None, "every offer counts");
    assert!(unsearched[0].suspicious);
}

#[test]
fn another_currency_has_its_icon_and_its_name_and_isnt_suspicious_without_price() {
    let rows = item_rows(
        &[offer(1.0, "annul", SellerStatus::Online)],
        Some(REFERENCE),
        None,
        SystemTime::UNIX_EPOCH,
    );

    let price = rows[0].price.as_ref().unwrap();
    assert_eq!(
        price.currency.map(|currency| currency.id.as_str()),
        Some("annul")
    );
    assert_eq!(price.label(Language::English), "1 Orb of Annulment");
    assert_eq!(price.label(Language::French), "1 Orbe d'annulation");
    assert!(!rows[0].suspicious);
}

#[test]
fn an_offer_in_a_currency_of_the_price_database_may_be_suspicious() {
    // 12.4 div of reference: 10 Orbs of Annulment at 150 ex are 1,500 ex, under a quarter.
    let currencies = [crate::prices::CurrencyPrice {
        name: "Orb of Annulment".to_owned(),
        exchange_id: "annul".to_owned(),
        price: crate::prices::Price {
            exalted: 150.0,
            quantity: 100,
            history: Vec::new(),
        },
    }];
    let reference = Reference {
        exalted: 9300.0,
        worth: Worth {
            rates: RATES,
            currencies: &currencies,
        },
    };
    let offers = [
        offer(10.0, "annul", SellerStatus::Online),
        offer(60.0, "annul", SellerStatus::Online),
    ];

    let rows = item_rows(&offers, Some(reference), None, SystemTime::UNIX_EPOCH);

    let suspicious: Vec<bool> = rows.iter().map(|row| row.suspicious).collect();
    assert_eq!(suspicious, [true, false]);
}

#[test]
fn every_currency_is_named_the_three_orbs_too() {
    let price = row_price(12.4, "divine");

    assert_eq!(price.label(Language::English), "12.4 Divine Orb");
    assert_eq!(price.label(Language::French), "12.4 Orbe divin");
}

#[test]
fn no_offer_is_said_only_when_none_is_left_to_fetch() {
    assert!(no_offer(&[], More::None));
    assert!(
        !no_offer(&[], More::Chevron),
        "a page left out for its currency: the next ones may be shown"
    );
    assert!(!no_offer(&[], More::Loading));
    let rows = item_rows(
        &[offer(1.0, "divine", SellerStatus::Online)],
        None,
        None,
        SystemTime::UNIX_EPOCH,
    );
    assert!(!no_offer(&rows, More::None));
}

#[test]
fn a_currency_the_application_doesnt_know_has_no_icon_nor_name() {
    let price = row_price(4.0, "not-a-currency");

    assert_eq!(price.currency, None);
    assert_eq!(price.label(Language::English), "4");
}

#[test]
fn an_offer_of_the_exchange_shows_the_price_of_one_item_and_its_stock() {
    let offer = ExchangeOffer {
        seller: SellerStatus::Online,
        listed: None,
        pay: Price {
            amount: 350.0,
            currency: "exalted".to_owned(),
        },
        get: Price {
            amount: 2.0,
            currency: "annul".to_owned(),
        },
        stock: 40,
        whisper: String::new(),
    };
    let reference = Reference {
        exalted: 182.0,
        worth: WORTH,
    };

    let rows = exchange_rows(&[offer], Some(reference), SystemTime::UNIX_EPOCH);

    assert_eq!(
        rows,
        [Row {
            price: Some(RowPrice {
                amount: "175".to_owned(),
                currency: currencies::shown("exalted"),
            }),
            suspicious: false,
            other_quality: None,
            stock: Some(40),
            away: false,
            listed: None,
        }]
    );
}

#[test]
fn the_block_searches_only_once_the_wait_is_over_and_no_answer_is_awaited() {
    assert_eq!(block(Block::Again, Duration::ZERO, false), Block::Again);
    let eight = Duration::from_secs(8);
    assert_eq!(
        block(Block::Again, eight, false),
        Block::Waiting(Some(eight))
    );
    assert_eq!(
        block(Block::Again, eight, true),
        Block::Waiting(Some(eight)),
        "the countdown stays while more offers come"
    );
    assert_eq!(
        block(Block::Search, Duration::ZERO, true),
        Block::Waiting(None)
    );
}

/// The characters of `galley` inside each of `rects`, its spaces left out.
fn found_in(galley: &Galley, rects: &[Rect]) -> Vec<String> {
    rects
        .iter()
        .map(|rect| {
            galley
                .rows
                .iter()
                .flat_map(|row| {
                    row.row.glyphs.iter().filter_map(move |glyph| {
                        let at = glyph.logical_rect().translate(row.pos.to_vec2());
                        (rect.contains(at.center()) && !glyph.chr.is_whitespace())
                            .then_some(glyph.chr)
                    })
                })
                .collect()
        })
        .collect()
}

#[test]
fn the_link_of_a_hint_is_found_on_each_row_it_wraps_on() {
    let ctx = egui::Context::default();
    crate::ui::install(&ctx);
    let mut checked = 0;
    crate::ui::test_frame(&ctx, egui::RawInput::default(), |ui| {
        for language in Language::ALL {
            let i18n = Translator::new(language);
            // The link inside the sentence, after accented letters in French, and first
            // in a sentence of its own.
            for hint in [i18n.offers_refused_hint(), i18n.offers_unreadable_hint()] {
                let words: String = hint.text[hint.link.clone()]
                    .chars()
                    .filter(|chr| !chr.is_whitespace())
                    .collect();
                // Wide, the link on one row; narrow, its words on several rows (cut at
                // the hyphen of "signalez-le").
                for (width, rows) in [(1000.0, 1..=1), (40.0, 2..=3)] {
                    let galley = linked_galley(ui, &hint, width, true);
                    let found = found_in(&galley, &link_rects(&galley, &hint));
                    assert_eq!(found.concat(), words, "{hint:?} at {width}");
                    assert!(rows.contains(&found.len()), "{found:?} at {width}");
                    checked += 1;
                }
            }
        }
    });
    assert_eq!(checked, 8);
}

/// Draws the hint of a refused search, one frame per call.
struct HintWindow {
    ctx: egui::Context,
    hint: Linked,
    link: bool,
}

impl HintWindow {
    const WIDTH: f32 = 300.0;

    fn new(link: bool) -> Self {
        let ctx = egui::Context::default();
        crate::ui::install(&ctx);
        let window = Self {
            ctx,
            hint: Translator::new(Language::English).offers_refused_hint(),
            link,
        };
        window.frame(vec![]);
        window
    }

    /// One frame with `events`: whether the link was clicked, where the sentence starts
    /// and where its link is.
    fn frame(&self, events: Vec<egui::Event>) -> (bool, Pos2, Vec<Rect>) {
        let input = egui::RawInput {
            events,
            ..Default::default()
        };
        let mut drawn = (false, Pos2::ZERO, Vec::new());
        crate::ui::test_frame(&self.ctx, input, |ui| {
            let origin = ui.cursor().min + vec2(size::BAND_PADDING, 0.0);
            let galley = linked_galley(ui, &self.hint, Self::WIDTH, true);
            let rects = link_rects(&galley, &self.hint)
                .into_iter()
                .map(|rect| rect.translate(origin.to_vec2()))
                .collect();
            let clicked = linked_hint(ui, &self.hint, Self::WIDTH, self.link, "Report a bug");
            drawn = (clicked, origin, rects);
        });
        drawn
    }

    /// Whether a click at `at` clicked the link.
    fn click(&self, at: Pos2) -> bool {
        let button = |pressed| egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        self.frame(vec![egui::Event::PointerMoved(at)]);
        self.frame(vec![button(true)]);
        self.frame(vec![button(false)]).0
    }
}

#[test]
fn a_click_on_the_words_of_the_link_opens_the_bug_report() {
    let window = HintWindow::new(true);
    let (_, origin, rects) = window.frame(vec![]);

    assert!(window.click(rects[0].center()));
    // The start of the sentence isn't the link.
    assert!(!window.click(origin + vec2(4.0, 6.0)));
}

#[test]
fn in_the_window_over_the_game_the_words_are_no_link() {
    let window = HintWindow::new(false);
    let (_, _, rects) = window.frame(vec![]);

    assert!(!window.click(rects[0].center()));
}
