//! The title row of the offers block: its label, the line under it and the count of the offers,
//! the action block in its corner (search, search again, the countdown, the spinner), and
//! what to do after a failed search, its words that open the bug report a link.

use std::sync::Arc;
use std::time::Duration;

use egui::text::{LayoutJob, TextFormat};
use egui::{
    Color32, CursorIcon, Galley, Rect, Role, Sense, Stroke, Ui, Vec2, WidgetInfo, pos2, vec2,
};

use super::{
    HEAD_GAP, HOVER_TIME, List, OffersAction, OffersBlock, PADDING_Y, TITLE_GAP, band, label,
    overshoot,
};
use crate::i18n::{Linked, Message, Translator};
use crate::trade::search::Failure;
use crate::ui::theme::{color, size, text};
use crate::ui::{icons, widgets};

/// The title row, and the square action block in its corner: search, wait, search again.
const HEAD_HEIGHT: f32 = 48.0;
const BLOCK_SIZE: f32 = HEAD_HEIGHT;

/// The space between the count and the action block, and before the count.
const COUNT_GAP: f32 = 12.0;

/// The icons of the search and refresh blocks.
const SEARCH_ICON: f32 = 22.0;
const REFRESH_ICON: f32 = 18.0;

/// The growth of the icons under the pointer, and the quarter turn of the refresh icon.
const SEARCH_GROWTH: f32 = 0.15;
const REFRESH_GROWTH: f32 = 0.12;

/// The action block of the band.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Block {
    /// The first search of the item.
    Search,
    /// Search again.
    Again,
    /// The time left before the next search; `None` once it is over while an answer is
    /// awaited.
    Waiting(Option<Duration>),
    /// The search is sent: the spinner, until its answer.
    Searching,
}

/// The block of a band whose action is `ready` once the `wait` is over and the offers aren't
/// `busy`.
pub(super) fn block(ready: Block, wait: Duration, busy: bool) -> Block {
    if !wait.is_zero() {
        Block::Waiting(Some(wait))
    } else if busy {
        Block::Waiting(None)
    } else {
        ready
    }
}

/// The title row of the offers.
pub(super) struct Head {
    /// Under the label: the age of the offers, or what happens before they show.
    line: String,
    line_color: Color32,
    count: Option<String>,
    block: Block,
}

impl Head {
    /// The title row before the offers show, without a count: `line` under the label, in
    /// its color, and the action `block` in the corner.
    pub(super) fn of((line, line_color): (String, Color32), block: Block) -> Self {
        Self {
            line,
            line_color,
            count: None,
            block,
        }
    }
}

/// The line of the title row after a search that failed for `failure`, and what to do under
/// the row: nothing when trying again is enough.
pub(super) fn failure_lines(
    i18n: &Translator,
    failure: Failure,
) -> ((String, Color32), Option<Linked>) {
    let line = i18n.text(failure_message(failure));
    match failure {
        Failure::Unreachable => ((line, color::WEAK), None),
        Failure::Refused => ((line, color::WARNING), Some(i18n.offers_refused_hint())),
        Failure::Unreadable => ((line, color::WARNING), Some(i18n.offers_unreadable_hint())),
    }
}

/// What a search, or the fetch of its next offers, that failed for `failure` says.
pub(super) fn failure_message(failure: Failure) -> Message {
    match failure {
        Failure::Unreachable => Message::OffersUnreachable,
        Failure::Refused => Message::OffersRefused,
        Failure::Unreadable => Message::OffersUnreadable,
    }
}

/// What to do after a failed search, wrapped at `width`, in the window that has the bug
/// report: its words that open the report are in the color of the text, underlined under the
/// pointer and with the tooltip `tooltip`. Whether they were clicked.
fn linked_hint(ui: &mut Ui, hint: &Linked, width: f32, tooltip: &str) -> bool {
    let galley = linked_galley(ui, hint, width);
    let (rect, _) =
        ui.allocate_exact_size(vec2(ui.available_width(), galley.size().y), Sense::hover());
    let origin = pos2(rect.left() + size::BAND_PADDING, rect.top());
    let words = hint.text.get(hint.link.clone()).unwrap_or_default();
    let mut clicked = false;
    let mut lit = false;
    let id = ui.make_persistent_id("report-link");
    let rects: Vec<Rect> = link_rects(&galley, hint)
        .into_iter()
        .map(|part| part.translate(origin.to_vec2()))
        .collect();
    for (index, part) in rects.iter().enumerate() {
        // One stop of the keyboard for the whole link, on its first part.
        let sense = if index == 0 {
            Sense::click()
        } else {
            Sense::CLICK
        };
        let response = ui.interact(*part, id.with(index), sense);
        if index == 0 {
            response.widget_info(|| WidgetInfo::labeled(Role::Link, true, words));
        }
        let response = response.on_hover_text(tooltip);
        lit |= response.hovered() || response.has_focus();
        clicked |= response.clicked();
        if response.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
    }
    let painter = ui.painter();
    painter.galley(origin, galley, color::WEAK);
    if lit {
        for part in &rects {
            painter.hline(part.x_range(), part.bottom(), Stroke::new(1.0, color::TEXT));
        }
    }
    clicked
}

/// The hint laid out at `width`: in the weak color, the words of its link in the color of the
/// text.
fn linked_galley(ui: &Ui, hint: &Linked, width: f32) -> Arc<Galley> {
    let font = text::item_line();
    let mut job = LayoutJob::default();
    let text = hint.text.as_str();
    let (start, end) = (hint.link.start, hint.link.end);
    for (part, tint) in [
        (text.get(..start), color::WEAK),
        (text.get(start..end), color::TEXT),
        (text.get(end..), color::WEAK),
    ] {
        if let Some(part) = part.filter(|part| !part.is_empty()) {
            job.append(part, 0.0, TextFormat::simple(font.clone(), tint));
        }
    }
    job.wrap.max_width = width;
    ui.painter().layout_job(job)
}

/// Where the words of the link of `hint` are in its `galley`, one rectangle per row they
/// are on (the words wrap like the rest of the sentence). The glyphs are counted as the
/// characters of the text, as `Galley::layout_from_cursor` of epaint counts them: a row keeps
/// the space it breaks at, and a newline has no glyph. To check again when egui changes.
fn link_rects(galley: &Galley, hint: &Linked) -> Vec<Rect> {
    let first = hint
        .text
        .get(..hint.link.start)
        .unwrap_or_default()
        .chars()
        .count();
    let last = first
        + hint
            .text
            .get(hint.link.clone())
            .unwrap_or_default()
            .chars()
            .count();
    let mut rects = Vec::new();
    let mut index = 0;
    for row in &galley.rows {
        let mut part: Option<Rect> = None;
        for glyph in &row.row.glyphs {
            if (first..last).contains(&index) && !glyph.chr.is_whitespace() {
                let glyph_rect = glyph.logical_rect().translate(row.pos.to_vec2());
                part = Some(part.map_or(glyph_rect, |part| part.union(glyph_rect)));
            }
            index += 1;
        }
        if row.ends_with_newline {
            index += 1;
        }
        rects.extend(part);
    }
    rects
}

impl OffersBlock<'_> {
    /// Until the offers show, the band is only the title row, its `head`; after a search that
    /// failed, what to do under the row (`hint`).
    pub(super) fn title_row(
        &self,
        ui: &mut Ui,
        head: &Head,
        hint: Option<&Linked>,
    ) -> Option<OffersAction> {
        let mut action = None;
        band(ui, |ui, width| {
            if self.head(ui, head) {
                action = Some(OffersAction::Search);
            }
            if let Some(hint) = hint {
                ui.add_space(HEAD_GAP);
                if self.failure_hint(ui, hint, width) {
                    action = Some(OffersAction::ReportBug);
                }
                ui.add_space(PADDING_Y);
            }
        });
        action
    }

    /// What to do after a failed search, `hint`, wrapped at `width`: its words that open the
    /// bug report are a link in the window that has it, plain elsewhere. Whether they were
    /// clicked.
    fn failure_hint(&self, ui: &mut Ui, hint: &Linked, width: f32) -> bool {
        if self.has_bug_report {
            linked_hint(ui, hint, width, &self.i18n.text(Message::BugReportTooltip))
        } else {
            let galley = widgets::wrapped_line(ui, &hint.text, text::item_line(), width);
            widgets::text_row(ui, galley, size::BAND_PADDING, color::WEAK);
            false
        }
    }

    /// The title row of the results: when the search was sent, `age` ago, the count of the
    /// offers shown out of all found, and the block to search again once `wait` is over.
    pub(super) fn results_head(&self, list: &List, wait: Duration, age: Duration) -> Head {
        let i18n = self.i18n;
        let count = (list.total > 0).then(|| {
            i18n.offers_count(
                u64::try_from(list.rows.len()).unwrap_or(u64::MAX),
                list.total,
            )
        });
        Head {
            line: i18n.offers_searched(&i18n.age(age)),
            line_color: color::FAINT,
            count,
            block: block(Block::Again, wait, self.offers.busy()),
        }
    }

    /// The label and the line under it, the count on the right, then the action block in
    /// the top right corner; whether the block was clicked.
    pub(super) fn head(&self, ui: &mut Ui, head: &Head) -> bool {
        let (rect, _) =
            ui.allocate_exact_size(vec2(ui.available_width(), HEAD_HEIGHT), Sense::hover());
        let left = rect.left() + size::BAND_PADDING;
        let block_rect = Rect::from_min_max(
            pos2(rect.right() - BLOCK_SIZE, rect.top()),
            rect.right_bottom(),
        );
        let mut line_right = block_rect.left() - COUNT_GAP;
        if let Some(count) = &head.count {
            let galley = widgets::text_line(ui, count.as_str(), text::tag());
            let size = galley.size();
            line_right -= size.x + COUNT_GAP;
            ui.painter().galley(
                pos2(
                    block_rect.left() - COUNT_GAP - size.x,
                    rect.center().y - size.y / 2.0,
                ),
                galley,
                color::WEAK,
            );
        }
        let label = label(ui, self.i18n);
        // Cut with "…" in a window too narrow for it.
        let line = widgets::truncated_line(
            ui,
            head.line.as_str(),
            text::small(),
            (line_right - left).max(0.0),
        );
        let painter = ui.painter();
        let top = rect.center().y - (label.size().y + TITLE_GAP + line.size().y) / 2.0;
        painter.galley(pos2(left, top), label.clone(), color::WEAK);
        painter.galley(
            pos2(left, top + label.size().y + TITLE_GAP),
            line,
            head.line_color,
        );
        self.action_block(ui, block_rect, head.block)
    }

    /// Draws `block` in `rect`; whether it was clicked, and could be.
    fn action_block(&self, ui: &mut Ui, rect: Rect, block: Block) -> bool {
        let i18n = self.i18n;
        let clickable = matches!(block, Block::Search | Block::Again);
        let sense = if clickable {
            Sense::click()
        } else {
            Sense::hover()
        };
        let response = ui.interact(rect, ui.scope_id().with("offers-block"), sense);
        let tooltip = match block {
            Block::Search => i18n.text(Message::OffersSearchTooltip),
            Block::Again => i18n.text(Message::OffersAgainTooltip),
            Block::Waiting(Some(wait)) => i18n.offers_next_search_tooltip(wait),
            Block::Waiting(None) | Block::Searching => i18n.text(Message::OffersBusyTooltip),
        };
        let hover = ui.ctx().animate_bool_with_time_and_easing(
            response.id.with("hover"),
            clickable && response.hovered(),
            HOVER_TIME,
            overshoot,
        );
        let painter = ui.painter();
        match block {
            Block::Search => {
                painter.rect_filled(rect, 0.0, color::ACCENT);
                let size = SEARCH_ICON * (1.0 + SEARCH_GROWTH * hover);
                let square = Rect::from_center_size(rect.center(), Vec2::splat(size));
                icons::search(painter, square, color::ON_ACCENT);
            }
            Block::Again => {
                painter.rect_filled(rect, 0.0, color::ACCENT);
                let size = REFRESH_ICON * (1.0 + REFRESH_GROWTH * hover);
                let square = Rect::from_center_size(rect.center(), Vec2::splat(size));
                icons::refresh(painter, square, hover, color::ON_ACCENT);
            }
            Block::Searching => {
                painter.rect_filled(rect, 0.0, color::CONTROL);
                icons::spinner(painter, rect.center(), ui.input(|input| input.time));
                ui.ctx().request_repaint();
            }
            Block::Waiting(wait) => {
                painter.rect_filled(rect, 0.0, color::CONTROL);
                if let Some(wait) = wait {
                    let galley = widgets::text_line(ui, i18n.wait(wait), text::countdown());
                    let size = galley.size();
                    ui.painter()
                        .galley(rect.center() - size / 2.0, galley, color::WEAK);
                }
            }
        }
        response.widget_info(|| WidgetInfo::labeled(Role::Button, clickable, &tooltip));
        let response = response.on_hover_text(tooltip);
        clickable && response.on_hover_cursor(CursorIcon::PointingHand).clicked()
    }
}

#[cfg(test)]
mod tests;
