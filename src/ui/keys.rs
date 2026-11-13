//! The keys of the application: the price-check hotkey ([`crate::hotkey::Shortcut`]) and the
//! copy of the game, drawn as key caps (the empty window, the settings) or written joined in a
//! sentence, in bold ("**Ctrl+D** is taken by another application.").

use egui::text::{LayoutJob, TextFormat};
use egui::{Align2, FontId, Rect, Response, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

use super::theme::{color, size};
use super::widgets;
use crate::i18n::{Message, Translator};

/// The keys of the copy of the game.
pub const COPY: [&str; 2] = ["Ctrl", "C"];

/// `keys` written joined, as in a sentence: `Ctrl+D`.
#[must_use]
pub fn joined<K: AsRef<str>>(keys: &[K]) -> String {
    keys.iter().map(AsRef::as_ref).collect::<Vec<_>>().join("+")
}

/// The names of `keys` in the language of the interface: Shift is "Maj" in French, as on French
/// keyboards; the others are the same in every language.
#[must_use]
pub fn named(keys: &[&str], i18n: &Translator) -> Vec<String> {
    keys.iter()
        .map(|&key| {
            if key == "Shift" {
                i18n.text(Message::KeyShift)
            } else {
                key.to_owned()
            }
        })
        .collect()
}

/// The measures of a row of key caps.
#[derive(Debug, Clone)]
pub struct Caps {
    pub font: FontId,
    pub height: f32,
    /// Left and right of the letters of a key.
    pub padding: f32,
    /// On each side of the "+" between two keys.
    pub gap: f32,
}

/// `keys` drawn as key caps on the color of the controls, joined by "+".
pub fn caps<K: AsRef<str>>(ui: &mut Ui, keys: &[K], caps: &Caps) -> Response {
    let galleys: Vec<_> = keys
        .iter()
        .map(|key| widgets::text_line(ui, key.as_ref(), caps.font.clone()))
        .collect();
    let plus = widgets::text_line(ui, "+", caps.font.clone());

    // Each key cap, and a "+" between two of them.
    let caps_width: f32 = galleys
        .iter()
        .map(|cap| cap.size().x + 2.0 * caps.padding)
        .sum();
    let separators_width: f32 = galleys
        .iter()
        .skip(1)
        .map(|_| plus.size().x + 2.0 * caps.gap)
        .sum();
    let (rect, response) = ui.allocate_exact_size(
        vec2(caps_width + separators_width, caps.height),
        Sense::hover(),
    );

    let painter = ui.painter();
    let mut left = rect.left();
    for (index, cap) in galleys.into_iter().enumerate() {
        if index > 0 {
            left += caps.gap;
            painter.galley(
                pos2(left, rect.center().y - plus.size().y / 2.0),
                plus.clone(),
                color::WEAK,
            );
            left += plus.size().x + caps.gap;
        }
        let key = Rect::from_min_size(
            pos2(left, rect.top()),
            vec2(cap.size().x + 2.0 * caps.padding, caps.height),
        );
        painter.rect_filled(key, f32::from(size::RADIUS), color::CONTROL);
        painter.rect_stroke(
            key,
            f32::from(size::RADIUS),
            Stroke::new(1.0, color::CONTROL_LINE),
            StrokeKind::Inside,
        );
        painter.galley(
            Align2::CENTER_CENTER
                .anchor_size(key.center(), cap.size())
                .min,
            cap,
            color::TEXT,
        );
        left = key.right();
    }
    response
}

/// How the keys of a warning are set apart in its sentences: each of `strong` in the format
/// `bold`, the rest in `regular`.
pub struct Emphasis<'a> {
    pub strong: &'a [&'a str],
    pub regular: TextFormat,
    pub bold: TextFormat,
}

/// `first` then, on a line of its own, `second`, set apart by `emphasis`: the two sentences of
/// the key of the hotkey taken.
#[must_use]
pub fn two_lines(first: &str, second: &str, emphasis: &Emphasis) -> LayoutJob {
    let mut job = emphasized(first, emphasis);
    job.append("\n", 0.0, emphasis.regular.clone());
    append_emphasized(&mut job, second, emphasis);
    job
}

/// `sentence` set apart by `emphasis`: the keys of a warning in bold.
#[must_use]
pub fn emphasized(sentence: &str, emphasis: &Emphasis) -> LayoutJob {
    let mut job = LayoutJob::default();
    append_emphasized(&mut job, sentence, emphasis);
    job
}

/// Appends `sentence` to `job` as [`emphasized`] writes it.
fn append_emphasized(job: &mut LayoutJob, sentence: &str, emphasis: &Emphasis) {
    let Emphasis {
        strong,
        regular,
        bold,
    } = emphasis;
    let mut rest = sentence;
    loop {
        // The first of the strong parts left, the longest when two start at the same place.
        let next = strong
            .iter()
            .filter(|part| !part.is_empty())
            .filter_map(|part| rest.find(part).map(|at| (at, part.len())))
            .min_by_key(|&(at, len)| (at, std::cmp::Reverse(len)));
        let Some((at, len)) = next else {
            break;
        };
        if at > 0 {
            job.append(&rest[..at], 0.0, regular.clone());
        }
        job.append(&rest[at..at + len], 0.0, bold.clone());
        rest = &rest[at + len..];
    }
    if !rest.is_empty() {
        job.append(rest, 0.0, regular.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::Color32;

    /// The parts of `job`, each with whether it is bold.
    fn parts(job: &LayoutJob) -> Vec<(&str, bool)> {
        job.sections
            .iter()
            .map(|section| {
                (
                    &job.text[section.byte_range.start.0..section.byte_range.end.0],
                    section.format.color == Color32::RED,
                )
            })
            .collect()
    }

    /// Each of `strong` in red, the rest in white.
    fn emphasis<'a>(strong: &'a [&'a str]) -> Emphasis<'a> {
        let font = FontId::proportional(12.0);
        Emphasis {
            strong,
            regular: TextFormat::simple(font.clone(), Color32::WHITE),
            bold: TextFormat::simple(font, Color32::RED),
        }
    }

    #[test]
    fn the_keys_are_written_joined() {
        assert_eq!(joined(&["Ctrl", "Shift", "D"]), "Ctrl+Shift+D");
        assert_eq!(joined(&COPY), "Ctrl+C");
    }

    #[test]
    fn every_key_of_a_sentence_is_bold() {
        let job = emphasized(
            "Ctrl+D is taken by another application. Choose another key, or copy with Ctrl+C.",
            &emphasis(&["Ctrl+D", "Ctrl+C"]),
        );

        assert_eq!(
            parts(&job),
            [
                ("Ctrl+D", true),
                (
                    " is taken by another application. Choose another key, or copy with ",
                    false
                ),
                ("Ctrl+C", true),
                (".", false),
            ]
        );
    }

    #[test]
    fn the_second_sentence_goes_on_a_line_of_its_own_its_keys_bold_too() {
        let job = two_lines(
            "Ctrl+D is taken by another application.",
            "Choose another key, or copy with Ctrl+C.",
            &emphasis(&["Ctrl+D", "Ctrl+C"]),
        );

        assert_eq!(
            parts(&job),
            [
                ("Ctrl+D", true),
                // egui joins the parts of the same format.
                (
                    " is taken by another application.\nChoose another key, or copy with ",
                    false
                ),
                ("Ctrl+C", true),
                (".", false),
            ]
        );
    }

    #[test]
    fn shift_is_maj_in_french() {
        use crate::language::Language;

        let keys = ["Ctrl", "Shift", "D"];
        assert_eq!(
            named(&keys, &Translator::new(Language::French)),
            ["Ctrl", "Maj", "D"]
        );
        assert_eq!(
            named(&keys, &Translator::new(Language::English)),
            ["Ctrl", "Shift", "D"]
        );
    }

    #[test]
    fn a_sentence_without_its_keys_stays_regular() {
        let job = emphasized("Choose another key.", &emphasis(&["Ctrl+D"]));

        assert_eq!(parts(&job), [("Choose another key.", false)]);
    }
}
