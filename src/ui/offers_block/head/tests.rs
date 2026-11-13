use egui::{Modifiers, Pos2};

use super::*;
use crate::item::Item;
use crate::language::Language;
use crate::offers_state::Offers;
use crate::roll_difference::CopiedLines;
use crate::trade::query::SearchKind;

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
    crate::ui::fixtures::run_frame(&ctx, egui::RawInput::default(), |ui| {
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
                    let galley = linked_galley(ui, &hint, width);
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

/// Draws the hint of a refused search, one frame per call, in a window that has the bug
/// report or not.
struct HintWindow {
    ctx: egui::Context,
    hint: Linked,
    has_bug_report: bool,
    item: Item,
    offers: Offers,
    lines: CopiedLines,
    i18n: Translator,
}

impl HintWindow {
    const WIDTH: f32 = 300.0;

    fn new(has_bug_report: bool) -> Self {
        let ctx = egui::Context::default();
        crate::ui::install(&ctx);
        let text = include_str!("../../../item/test-items/en-rare-bow-fractured.txt");
        let item = Item::parse(&crate::item::CopiedItem::from_text(text).unwrap()).unwrap();
        let i18n = Translator::new(Language::English);
        let window = Self {
            ctx,
            hint: i18n.offers_refused_hint(),
            has_bug_report,
            item,
            offers: Offers::default(),
            lines: CopiedLines::default(),
            i18n,
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
        crate::ui::fixtures::run_frame(&self.ctx, input, |ui| {
            let origin = ui.cursor().min + vec2(size::BAND_PADDING, 0.0);
            let galley = linked_galley(ui, &self.hint, Self::WIDTH);
            let rects = link_rects(&galley, &self.hint)
                .into_iter()
                .map(|rect| rect.translate(origin.to_vec2()))
                .collect();
            let clicked = self.block().failure_hint(ui, &self.hint, Self::WIDTH);
            drawn = (clicked, origin, rects);
        });
        drawn
    }

    /// The height the hint takes, in one frame.
    fn height(&self) -> f32 {
        let mut height = 0.0;
        crate::ui::fixtures::run_frame(&self.ctx, egui::RawInput::default(), |ui| {
            let top = ui.cursor().min.y;
            let block = self.block();
            block.failure_hint(ui, &self.hint, Self::WIDTH);
            height = ui.cursor().min.y - top;
        });
        height
    }

    /// The block of the offers of the window, which draws the hint.
    fn block(&self) -> OffersBlock<'_> {
        OffersBlock {
            offers: &self.offers,
            kind: SearchKind::Items,
            reference: None,
            item: &self.item,
            lines: &self.lines,
            has_bug_report: self.has_bug_report,
            i18n: &self.i18n,
        }
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

#[test]
fn the_plain_hint_wraps_on_as_many_rows_as_the_linked_one() {
    let plain = HintWindow::new(false).height();

    assert_eq!(plain, HintWindow::new(true).height());
    // On several rows, so that the cut of the sentence is compared too.
    let window = HintWindow::new(false);
    let mut one_row = 0.0;
    crate::ui::fixtures::run_frame(&window.ctx, egui::RawInput::default(), |ui| {
        one_row = widgets::text_line(ui, "Report", text::item_line()).size().y;
    });
    assert!(plain > one_row, "{plain} for a row of {one_row}");
}
