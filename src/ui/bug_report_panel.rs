//! The bug report, in place of the content of the window like the settings: a preview of what
//! the report carries, as it will be sent (`specs.md`, Bug report), then the pill that opens it
//! on GitHub.

use egui::text::{LayoutJob, TextFormat, TextWrapping};
use egui::{
    CentralPanel, CornerRadius, Frame, Label, Margin, RichText, ScrollArea, Ui, Vec2, vec2,
};

use super::theme::{color, size, text};
use super::widgets::{self, Mark};
use crate::bug_report::{Level, LogLine, Report};
use crate::i18n::{Message, Translator};

/// The space above and under the preview, as in the settings.
const PADDING_Y: f32 = 12.0;
const INTRO_GAP: f32 = 6.0;
/// The space above the label of a box.
const LABEL_GAP: f32 = 14.0;
/// The space between a label and its box, and under the box of the log.
const BOX_GAP: f32 = 6.0;
const BOX_PADDING: Margin = Margin::symmetric(10, 8);
/// The box of the log, its padding with it, scrolls inside past this height, so that the window
/// keeps its height.
const LOG_HEIGHT: f32 = 168.0;
const ITEM_GAP: f32 = 12.0;
const ACTIONS_GAP: f32 = 14.0;
const ACCOUNT_GAP: f32 = 8.0;

/// What the player did in the bug report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BugReportAction {
    /// Checked or unchecked the item shown.
    JoinItem(bool),
    /// Opened the report on GitHub.
    Open,
}

pub struct BugReportPanel<'a> {
    report: &'a Report,
    i18n: &'a Translator,
}

impl<'a> BugReportPanel<'a> {
    #[must_use]
    pub fn new(report: &'a Report, i18n: &'a Translator) -> Self {
        Self { report, i18n }
    }

    /// Draws the preview; what the player did, if anything. It scrolls in a window too low for
    /// it.
    pub fn show(self, ui: &mut Ui) -> Option<BugReportAction> {
        CentralPanel::no_frame()
            .frame(Frame::NONE.fill(color::BAND_B))
            .show(ui, |ui| {
                // Started again at each opening, at the top.
                ScrollArea::vertical()
                    .id_salt(("bug-report", self.report.opening()))
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        Frame::NONE
                            .inner_margin(vec2(size::BAND_PADDING, PADDING_Y))
                            .show(ui, |ui| self.content(ui))
                            .inner
                    })
                    .inner
            })
            .inner
    }

    fn content(&self, ui: &mut Ui) -> Option<BugReportAction> {
        ui.spacing_mut().item_spacing = Vec2::ZERO;
        ui.label(
            RichText::new(self.i18n.text(Message::BugReportTitle))
                .font(text::title())
                .color(color::TEXT),
        );
        ui.add_space(INTRO_GAP);
        ui.add(
            Label::new(
                RichText::new(self.i18n.text(Message::BugReportIntro))
                    .font(text::bubble())
                    .color(color::WEAK),
            )
            .wrap(),
        );

        self.label(ui, Message::BugReportSent);
        report_box(ui, |ui| {
            ui.add(
                Label::new(
                    RichText::new(self.report.summary())
                        .font(text::report())
                        .color(color::TEXT),
                )
                .wrap(),
            );
        });

        self.label(ui, Message::BugReportLog);
        report_box(ui, |ui| {
            // Started again at each opening, scrolled to its end: egui keeps the place of a
            // scroll area by its id.
            ScrollArea::vertical()
                .id_salt(("bug-report-log", self.report.opening()))
                .max_height(LOG_HEIGHT - BOX_PADDING.sum().y)
                .auto_shrink([false, true])
                .stick_to_bottom(true)
                .show(ui, |ui| match self.report.log_sent() {
                    Some(lines) => {
                        let mut job = log_job(lines);
                        job.wrap.max_width = ui.available_width();
                        ui.add(Label::new(job).wrap());
                    }
                    None => {
                        ui.label(
                            RichText::new(self.i18n.text(Message::BugReportNoLog))
                                .font(text::report())
                                .color(color::FAINT),
                        );
                    }
                });
        });
        ui.add_space(BOX_GAP);
        self.note(ui, Message::BugReportLogNote);

        let mut action = self.item(ui);
        ui.add_space(ACTIONS_GAP);
        if widgets::pill(ui, &self.i18n.text(Message::BugReportOpen)).clicked() {
            action = Some(BugReportAction::Open);
        }
        ui.add_space(ACCOUNT_GAP);
        self.note(ui, Message::BugReportAccount);
        action
    }

    /// The uppercase label above a box, after the space above it.
    fn label(&self, ui: &mut Ui, message: Message) {
        ui.add_space(LABEL_GAP);
        let label = widgets::spaced_line(
            ui,
            self.i18n.text(message).to_uppercase(),
            text::label(),
            text::LABEL_LETTER_SPACING,
        );
        let (rect, _) = ui.allocate_exact_size(label.size(), egui::Sense::hover());
        ui.painter().galley(rect.min, label, color::WEAK);
        ui.add_space(BOX_GAP);
    }

    fn note(&self, ui: &mut Ui, message: Message) {
        ui.add(
            Label::new(
                RichText::new(self.i18n.text(message))
                    .font(text::small())
                    .color(color::FAINT),
            )
            .wrap(),
        );
    }

    /// The checkbox that joins the item shown, its name in bold after a colon, or the item that
    /// couldn't be read; nothing before the first item. A name too long for the window is cut
    /// with "…", the whole sentence in a tooltip.
    fn item(&self, ui: &mut Ui) -> Option<BugReportAction> {
        let item = self.report.item()?;
        ui.add_space(ITEM_GAP);
        let regular = TextFormat::simple(text::bubble(), color::TEXT);
        let (label, job) = if let Some(name) = &item.name {
            let label = self.i18n.bug_report_join_item(name);
            let mut job = LayoutJob::default();
            if let Some((before, after)) = label.split_once(name.as_str()) {
                job.append(before, 0.0, regular.clone());
                job.append(
                    name,
                    0.0,
                    TextFormat::simple(text::name_in_choice(), color::TEXT),
                );
                job.append(after, 0.0, regular);
            } else {
                job.append(&label, 0.0, regular);
            }
            (label, job)
        } else {
            let label = self.i18n.text(Message::BugReportJoinUnreadable);
            let job = LayoutJob::simple_singleline(label.clone(), text::bubble(), color::TEXT);
            (label, job)
        };
        let galley = ui.painter().layout_job(one_line(job, ui.available_width()));
        let cut = galley.elided;
        let joined = self.report.joins_item();
        let response = widgets::choice_galley(ui, Mark::Check { checked: joined }, galley, &label);
        let response = if cut {
            response.on_hover_text(&label)
        } else {
            response
        };
        response
            .clicked()
            .then_some(BugReportAction::JoinItem(!joined))
    }
}

/// `job` on one line, cut with "…" past `width`, the mark of a choice before it: the name of
/// the item, last, is cut, not the start of the sentence.
fn one_line(mut job: LayoutJob, width: f32) -> LayoutJob {
    job.wrap = TextWrapping {
        max_width: (width - widgets::CHOICE_TEXT_LEFT).max(0.0),
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    job
}

/// A box on the dark of the controls, across the width, around `content`.
fn report_box(ui: &mut Ui, content: impl FnOnce(&mut Ui)) {
    Frame::NONE
        .fill(color::CONTROL)
        .corner_radius(CornerRadius::same(size::RADIUS))
        .inner_margin(BOX_PADDING)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            content(ui);
        });
}

/// The lines of the log, in the color of the warnings for an error (a panic), "another window"
/// faint and slanted.
fn log_job(lines: &[LogLine]) -> LayoutJob {
    let mut job = LayoutJob::default();
    for (index, line) in lines.iter().enumerate() {
        let tint = if line.level == Some(Level::Error) {
            color::WARNING
        } else {
            color::WEAK
        };
        let format = TextFormat::simple(text::report(), tint);
        let text = if index + 1 < lines.len() {
            format!("{}\n", line.text)
        } else {
            line.text.clone()
        };
        match &line.hidden {
            Some(hidden) => {
                job.append(&text[..hidden.start], 0.0, format.clone());
                job.append(
                    &text[hidden.clone()],
                    0.0,
                    TextFormat {
                        italics: true,
                        ..TextFormat::simple(text::report(), color::FAINT)
                    },
                );
                job.append(&text[hidden.end..], 0.0, format);
            }
            None => job.append(&text, 0.0, format),
        }
    }
    job
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bug_report::ANOTHER_WINDOW;

    /// `label` laid out on one line past `width`: its text drawn, and whether it is cut.
    fn laid_out(label: &str, width: f32) -> (String, bool) {
        let ctx = egui::Context::default();
        crate::ui::install(&ctx);
        let mut drawn = (String::new(), false);
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let job = LayoutJob::simple_singleline(label.to_owned(), text::bubble(), color::TEXT);
            let galley = ui.painter().layout_job(one_line(job, width));
            // The characters drawn, the "…" of a text cut with them.
            let text = galley
                .rows
                .iter()
                .flat_map(|row| row.glyphs.iter().map(|glyph| glyph.chr))
                .collect();
            drawn = (text, galley.elided);
        });
        // epaint panics when the texture changes of a frame are dropped unapplied.
        output.textures_delta.clear();
        drawn
    }

    #[test]
    fn a_long_name_is_cut_with_an_ellipsis_after_the_start_of_the_sentence() {
        let label = "Join the item shown: Concentrated Ultimate Life Flask of the Surgeon";
        let (drawn, cut) = laid_out(label, 300.0);

        assert!(cut);
        assert!(drawn.starts_with("Join the item shown: Conc"), "{drawn}");
        assert!(drawn.ends_with('…'), "{drawn}");
        assert!(!laid_out("Join the item shown: Headhunter", 300.0).1);
    }

    #[test]
    fn another_window_is_drawn_apart_and_an_error_in_the_color_of_the_warnings() {
        let text =
            format!("18:51:09 INFO The active window, of class {ANOTHER_WINDOW}, isn't the game");
        let start = text.find(ANOTHER_WINDOW).expect("hidden");
        let lines = [
            LogLine {
                text: text.clone(),
                level: Some(Level::Info),
                hidden: Some(start..start + ANOTHER_WINDOW.len()),
            },
            LogLine {
                text: "21:02:40 ERROR Panic in the main thread".to_owned(),
                level: Some(Level::Error),
                hidden: None,
            },
        ];
        let job = log_job(&lines);

        assert_eq!(
            job.text,
            format!("{text}\n21:02:40 ERROR Panic in the main thread")
        );
        let hidden = &job.sections[1];
        assert_eq!(
            &job.text[hidden.byte_range.start.0..hidden.byte_range.end.0],
            ANOTHER_WINDOW
        );
        assert!(hidden.format.italics);
        assert_eq!(hidden.format.color, color::FAINT);
        assert_eq!(job.sections[3].format.color, color::WARNING);
    }
}
