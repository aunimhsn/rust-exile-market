//! The bubbles under the league block: a new league for a player on a permanent league once it
//! has started (its name, a pill to switch to it), and a newer version of the application (its
//! version, a pill that opens its page); each with a cross to close it. When both show, the
//! second goes under the first, aligned on its left.

use egui::text::{LayoutJob, TextFormat};
use egui::{
    Align, Area, Context, CornerRadius, CursorIcon, Frame, Id, Layout, Margin, Order, Role, Sense,
    Ui, Vec2, WidgetInfo, pos2, vec2,
};

use super::icons;
use super::theme::{self, color, size, text};
use super::title_bar::LEAGUE_BLOCK_LEFT;
use super::widgets;
use crate::i18n::{CutSentence, Message, Translator};
use crate::league::League;
use crate::update::Version;

const WIDTH: f32 = 330.0;
/// The bubble starts a little left of the league block, 4 px under the title bar like the
/// list of the leagues; a second bubble 4 px under the first.
const LEFT: f32 = LEAGUE_BLOCK_LEFT - 8.0;
const GAP: f32 = 4.0;
const PADDING_X: i8 = 14;
const PADDING_Y: i8 = 10;
const CONTENT_GAP: f32 = 10.0;
const CLOSE_ZONE: f32 = 18.0;
const CLOSE_SIZE: f32 = 7.0;

/// What the player did with a bubble.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BubbleAction {
    /// The pill: switch to the league, or download the version.
    Pill,
    Close,
}

/// A bubble: a sentence with a part in bold, its pill and its cross.
pub struct Bubble<'a> {
    id: &'static str,
    sentence: CutSentence,
    strong: String,
    pill: String,
    i18n: &'a Translator,
    top: f32,
}

/// A bubble drawn: what the player did, and its bottom, under which the next one goes.
pub struct ShownBubble {
    pub action: Option<BubbleAction>,
    pub bottom: f32,
}

impl<'a> Bubble<'a> {
    /// "Rise of the Abyssal has started.", the name in bold, and the pill Switch.
    #[must_use]
    pub fn new_league(league: &League, i18n: &'a Translator) -> Self {
        let name = league.name();
        Self::new(
            "new-league-bubble",
            i18n.new_league(),
            name,
            i18n.text(Message::NewLeagueSwitch),
            i18n,
        )
    }

    /// "Exile Market 0.2.0 is available.", the version in bold, and the pill Download.
    #[must_use]
    pub fn update(version: Version, i18n: &'a Translator) -> Self {
        let version = version.to_string();
        Self::new(
            "update-bubble",
            i18n.update_available(crate::APP_NAME),
            &version,
            i18n.text(Message::UpdateDownload),
            i18n,
        )
    }

    fn new(
        id: &'static str,
        sentence: CutSentence,
        strong: &str,
        pill: String,
        i18n: &'a Translator,
    ) -> Self {
        Self {
            id,
            sentence,
            strong: strong.to_owned(),
            pill,
            i18n,
            top: size::TITLE_BAR_HEIGHT + GAP,
        }
    }

    /// Under a bubble whose bottom is `bottom`.
    #[must_use]
    pub fn below(mut self, bottom: f32) -> Self {
        self.top = bottom + GAP;
        self
    }

    pub fn show(&self, ctx: &Context) -> ShownBubble {
        let area = Area::new(Id::NULL.with(self.id))
            .order(Order::Foreground)
            .fixed_pos(pos2(LEFT, self.top))
            .show(ctx, |ui| {
                let frame = Frame::NONE
                    .fill(color::BUBBLE)
                    .corner_radius(CornerRadius::same(size::BUBBLE_RADIUS))
                    .shadow(theme::floating_shadow())
                    .inner_margin(Margin::symmetric(PADDING_X, PADDING_Y));
                frame
                    .show(ui, |ui| {
                        ui.set_width(WIDTH - 2.0 * f32::from(PADDING_X));
                        ui.spacing_mut().item_spacing = vec2(CONTENT_GAP, 0.0);
                        ui.with_layout(Layout::left_to_right(Align::Center), |ui| self.content(ui))
                            .inner
                    })
                    .inner
            });
        ShownBubble {
            action: area.inner,
            bottom: area.response.rect.bottom(),
        }
    }

    fn content(&self, ui: &mut Ui) -> Option<BubbleAction> {
        // A long sentence wraps, so that the pill and the cross stay in the bubble.
        let text_width = ui.available_width()
            - widgets::pill_size(ui, &self.pill).x
            - CLOSE_ZONE
            - 2.0 * CONTENT_GAP;
        let mut job = self.text();
        job.wrap.max_width = text_width.max(0.0);
        ui.label(ui.painter().layout_job(job));
        let mut action = None;
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let (rect, close) = ui.allocate_exact_size(Vec2::splat(CLOSE_ZONE), Sense::click());
            let tooltip = self.i18n.text(Message::CloseTooltip);
            close.widget_info(|| WidgetInfo::labeled(Role::Button, true, &tooltip));
            let lit = close.hovered() || close.has_focus();
            icons::cross(
                ui.painter(),
                rect.center(),
                CLOSE_SIZE,
                if lit { color::TEXT } else { color::WEAK },
            );
            if close
                .on_hover_cursor(CursorIcon::PointingHand)
                .on_hover_text(tooltip)
                .clicked()
            {
                action = Some(BubbleAction::Close);
            }
            if widgets::pill(ui, &self.pill).clicked() {
                action = Some(BubbleAction::Pill);
            }
        });
        action
    }

    /// The sentence, its strong part in bold.
    fn text(&self) -> LayoutJob {
        widgets::sentence_with(
            &self.sentence,
            &self.strong,
            &TextFormat::simple(text::bubble(), color::TEXT),
            TextFormat::simple(text::bubble_strong(), color::TEXT),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::language::Language;
    use egui::RawInput;

    fn version() -> Version {
        Version::parse("0.2.0").expect("a version")
    }

    /// The bottoms of the bubble of a new league and of the update bubble under it.
    fn bottoms(language: Language) -> (f32, f32) {
        let ctx = Context::default();
        crate::ui::install(&ctx);
        let i18n = Translator::new(language);
        let league = League::new("Rise of the Abyssal");
        let mut shown = (0.0, 0.0);
        // Twice: the bottom of the first frame is known once it is laid out.
        for _ in 0..2 {
            crate::ui::fixtures::run_frame(&ctx, RawInput::default(), |ui| {
                let first = Bubble::new_league(&league, &i18n).show(ui.ctx());
                let second = Bubble::update(version(), &i18n)
                    .below(first.bottom)
                    .show(ui.ctx());
                shown = (first.bottom, second.bottom);
            });
        }
        shown
    }

    #[test]
    fn the_update_bubble_goes_under_the_bubble_of_a_new_league() {
        let (first, second) = bottoms(Language::English);
        let height = first - size::TITLE_BAR_HEIGHT - GAP;

        assert!(height > 0.0, "{first}");
        // One line each, the second 4 px under the first.
        assert!(
            (second - (first + GAP + height)).abs() < 0.5,
            "{first} {second}"
        );
    }

    #[test]
    fn the_french_update_bubble_holds_on_one_line_too() {
        // "Exile Market 0.2.0 disponible.", with "Télécharger", as in the mock-up.
        let (first, second) = bottoms(Language::French);
        let height = first - size::TITLE_BAR_HEIGHT - GAP;

        assert!(
            (second - (first + GAP + height)).abs() < 0.5,
            "{first} {second}"
        );
    }

    #[test]
    fn the_version_is_the_bold_part_of_the_update_bubble() {
        let i18n = Translator::new(Language::French);
        let job = Bubble::update(version(), &i18n).text();

        let bold: Vec<&str> = job
            .sections
            .iter()
            .filter(|section| section.format.font_id == text::bubble_strong())
            .map(|section| &job.text[section.byte_range.start.0..section.byte_range.end.0])
            .collect();
        assert_eq!(bold, ["0.2.0"]);
        assert_eq!(job.text, "Exile Market 0.2.0 disponible.");
    }
}
