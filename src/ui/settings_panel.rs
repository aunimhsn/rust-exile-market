//! The settings, in place of the content of the window: the language, always on top, the
//! price-check hotkey, and the disclaimer at the bottom.

use egui::text::TextFormat;
use egui::{Align, CentralPanel, Frame, Label, Layout, RichText, Ui, Vec2, vec2};

use super::keys::{self, Caps};
use super::theme::{color, size, text};
use super::widgets::{self, Mark};
use crate::hotkey::HotkeyState;
use crate::i18n::{Message, Translator};
use crate::settings::{LanguageSetting, SettingChange, Settings};

/// The space above and under the settings.
const PADDING_Y: f32 = 12.0;
/// The space above each group of choices.
const GROUP_GAP: f32 = 16.0;
/// The space between the choices of a group, and under its label.
const CHOICE_GAP: f32 = 6.0;
const DISCLAIMER_GAP: f32 = 22.0;
/// The key caps of the hotkey, smaller than in the empty window (the mock-up).
const KEY_HEIGHT: f32 = 20.0;
const KEY_PADDING: f32 = 7.0;
const KEY_GAP: f32 = 4.0;

pub struct SettingsPanel<'a> {
    settings: &'a Settings,
    i18n: &'a Translator,
    hotkey: HotkeyState,
}

impl<'a> SettingsPanel<'a> {
    /// The settings, with what the price-check `hotkey` does now: `Unavailable` on a system
    /// without it, whose group isn't drawn.
    #[must_use]
    pub fn new(settings: &'a Settings, i18n: &'a Translator, hotkey: HotkeyState) -> Self {
        Self {
            settings,
            i18n,
            hotkey,
        }
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
                // macOS has no hotkey: Path of Exile 2 has no client there.
                let hotkey = if cfg!(any(target_os = "linux", target_os = "windows")) {
                    self.hotkey(ui)
                } else {
                    None
                };
                self.disclaimer(ui);
                language.or(always_on_top).or(hotkey)
            })
            .inner
    }

    /// The label above a group of choices, after the space above the group.
    fn group_label(&self, ui: &mut Ui, message: Message) {
        ui.add_space(GROUP_GAP);
        let label = widgets::spaced_line(
            ui,
            self.i18n.text(message).to_uppercase(),
            text::label(),
            text::LABEL_LETTER_SPACING,
        );
        let (rect, _) = ui.allocate_exact_size(label.size(), egui::Sense::hover());
        ui.painter().galley(rect.min, label, color::WEAK);
    }

    fn language(&self, ui: &mut Ui) -> Option<SettingChange> {
        self.group_label(ui, Message::SettingsLanguage);

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

    /// The price-check hotkey: its checkbox, its keys on the right of the row, then a line under
    /// it, what it does, or in the color of the warnings that its key is taken.
    fn hotkey(&self, ui: &mut Ui) -> Option<SettingChange> {
        self.group_label(ui, Message::SettingsHotkey);
        ui.add_space(CHOICE_GAP);
        let on = self.settings.hotkey().on;
        let hotkey_keys = self.settings.hotkey().keys.keys();
        let label = self.i18n.text(Message::SettingsHotkeyPriceCheck);
        let clicked = ui
            .horizontal(|ui| {
                let clicked = widgets::choice(ui, Mark::Check { checked: on }, &label).clicked();
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let caps = Caps {
                        font: text::setting_keys(),
                        height: KEY_HEIGHT,
                        padding: KEY_PADDING,
                        gap: KEY_GAP,
                    };
                    let letters = if on { color::TEXT } else { color::FAINT };
                    keys::caps(ui, &hotkey_keys, &caps, letters, on);
                });
                clicked
            })
            .inner;
        ui.add_space(CHOICE_GAP);
        let note = if self.hotkey == HotkeyState::Taken {
            let hotkey = keys::joined(&hotkey_keys);
            let copy = keys::joined(&keys::COPY);
            keys::emphasized(
                &self.i18n.hotkey_taken_settings(&hotkey, &copy),
                &[&hotkey, &copy],
                &TextFormat::simple(text::small(), color::WARNING),
                &TextFormat::simple(text::small_strong(), color::WARNING),
            )
        } else {
            egui::text::LayoutJob::simple_format(
                self.i18n.text(Message::SettingsHotkeyHelp),
                TextFormat::simple(text::small(), color::FAINT),
            )
        };
        ui.add(Label::new(note).wrap());
        clicked.then_some(SettingChange::HotkeyOn(!on))
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
