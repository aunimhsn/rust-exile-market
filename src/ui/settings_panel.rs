//! The settings, in place of the content of the window: the language, always on top, and
//! the disclaimer at the bottom.

use egui::{CentralPanel, Frame, Label, RichText, Ui, Vec2, vec2};

use super::theme::{color, size, text};
use super::widgets::{self, Mark};
use crate::i18n::{Message, Translator};
use crate::settings::{LanguageSetting, SettingChange, Settings};

/// The space above and under the settings.
const PADDING_Y: f32 = 12.0;
/// The space above each group of choices.
const GROUP_GAP: f32 = 16.0;
/// The space between the choices of a group, and under its label.
const CHOICE_GAP: f32 = 6.0;
const DISCLAIMER_GAP: f32 = 22.0;

pub struct SettingsPanel<'a> {
    settings: &'a Settings,
    i18n: &'a Translator,
}

impl<'a> SettingsPanel<'a> {
    #[must_use]
    pub fn new(settings: &'a Settings, i18n: &'a Translator) -> Self {
        Self { settings, i18n }
    }

    /// Draws the settings; the change the player clicked, if any.
    pub fn show(self, ui: &mut Ui) -> Option<SettingChange> {
        CentralPanel::no_frame()
            .frame(
                Frame::NONE
                    .fill(color::BAND_B)
                    .inner_margin(vec2(size::BAND_PADDING, PADDING_Y)),
            )
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                ui.label(
                    RichText::new(self.i18n.text(Message::SettingsTitle))
                        .font(text::title())
                        .color(color::TEXT),
                );
                let language = self.language(ui);
                let always_on_top = self.always_on_top(ui);
                self.disclaimer(ui);
                language.or(always_on_top)
            })
            .inner
    }

    fn language(&self, ui: &mut Ui) -> Option<SettingChange> {
        ui.add_space(GROUP_GAP);
        let label = widgets::spaced_line(
            ui,
            self.i18n.text(Message::SettingsLanguage).to_uppercase(),
            text::label(),
            text::LABEL_LETTER_SPACING,
        );
        let (rect, _) = ui.allocate_exact_size(label.size(), egui::Sense::hover());
        ui.painter().galley(rect.min, label, color::WEAK);

        let mut change = None;
        for setting in LanguageSetting::ALL {
            ui.add_space(CHOICE_GAP);
            let name = self.i18n.text(match setting {
                LanguageSetting::System => Message::LanguageSystem,
                LanguageSetting::English => Message::LanguageEnglish,
                LanguageSetting::French => Message::LanguageFrench,
            });
            let mark = Mark::Radio {
                selected: self.settings.language() == setting,
            };
            if widgets::choice(ui, mark, &name).clicked() {
                change = Some(SettingChange::Language(setting));
            }
        }
        change
    }

    fn always_on_top(&self, ui: &mut Ui) -> Option<SettingChange> {
        ui.add_space(GROUP_GAP);
        let checked = self.settings.always_on_top();
        let label = self.i18n.text(Message::SettingsAlwaysOnTop);
        widgets::choice(ui, Mark::Check { checked }, &label)
            .clicked()
            .then_some(SettingChange::AlwaysOnTop(!checked))
    }

    fn disclaimer(&self, ui: &mut Ui) {
        ui.add_space(DISCLAIMER_GAP);
        ui.add(
            Label::new(
                RichText::new(self.i18n.text(Message::Disclaimer))
                    .font(text::small())
                    .color(color::FAINT),
            )
            .wrap(),
        );
    }
}
