//! The settings, in place of the content of the window: the language, always on top, the
//! price-check hotkey, and the disclaimer at the bottom.

use egui::text::TextFormat;
use egui::{Align, CentralPanel, Frame, Label, Layout, RichText, Ui, Vec2, vec2};

use super::hotkey_list::{self, HotkeyList, HotkeyListAction};
use super::keys;
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

/// What the player did in the settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsAction {
    Change(SettingChange),
    /// Opened or closed the list of the keys of the hotkey, with its button.
    ToggleKeys,
    /// Clicked outside the open list of the keys.
    CloseKeys,
}

pub struct SettingsPanel<'a> {
    settings: &'a Settings,
    i18n: &'a Translator,
    hotkey: HotkeyState,
    keys_open: bool,
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
            keys_open: false,
        }
    }

    /// Opens the list of the keys of the hotkey under its button.
    #[must_use]
    pub fn keys_open(mut self, open: bool) -> Self {
        self.keys_open = open;
        self
    }

    /// The settings no longer drawn: the list of the keys forgets its animation, so that it
    /// doesn't fade out when they open again.
    pub fn forget(ctx: &egui::Context) {
        hotkey_list::forget(ctx);
    }

    /// Draws the settings; what the player did, if anything.
    pub fn show(self, ui: &mut Ui) -> Option<SettingsAction> {
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
                // A change wins over the click outside the list of the keys, which it closes too.
                language
                    .or(always_on_top)
                    .map(SettingsAction::Change)
                    .or(hotkey)
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

    /// The price-check hotkey: its checkbox, the button of its keys on the right of the row and
    /// their list under it, then a line under the row, what it does, or in the color of the
    /// warnings that its key is taken.
    fn hotkey(&self, ui: &mut Ui) -> Option<SettingsAction> {
        self.group_label(ui, Message::SettingsHotkey);
        ui.add_space(CHOICE_GAP);
        let on = self.settings.hotkey().on;
        let shortcut = self.settings.hotkey().keys;
        let hotkey = keys::joined(&keys::named(&shortcut.keys(), self.i18n));
        let label = self.i18n.text(Message::SettingsHotkeyPriceCheck);
        let (checked, button) = ui
            .horizontal(|ui| {
                let checked = widgets::choice(ui, Mark::Check { checked: on }, &label).clicked();
                let button = ui
                    .with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let tooltip = self.i18n.text(Message::SettingsHotkeyChange);
                        hotkey_list::button(ui, &hotkey, on, self.keys_open, &tooltip)
                    })
                    .inner;
                (checked, button)
            })
            .inner;
        let list = HotkeyList::new(shortcut, self.i18n).show(ui.ctx(), &button, self.keys_open);
        ui.add_space(CHOICE_GAP);
        let note = if self.hotkey == HotkeyState::Taken {
            let copy = keys::joined(&keys::COPY);
            keys::two_lines(
                &self.i18n.hotkey_taken(&hotkey),
                &self.i18n.hotkey_taken_settings(&copy),
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
        hotkey_action(checked.then_some(!on), button.clicked(), list)
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

/// What the player did with the hotkey in a frame: the checkbox `turned` on or off, the button of
/// the keys `toggled`, and what they did with the `list`. The click on the button that closes the
/// list is outside it too: the button wins, so that the list doesn't close then open again. The
/// checkbox wins over the click outside the list, which its change closes too.
fn hotkey_action(
    turned: Option<bool>,
    toggled: bool,
    list: Option<HotkeyListAction>,
) -> Option<SettingsAction> {
    if let Some(on) = turned {
        return Some(SettingsAction::Change(SettingChange::HotkeyOn(on)));
    }
    if toggled {
        return Some(SettingsAction::ToggleKeys);
    }
    list.map(|action| match action {
        HotkeyListAction::Choose(keys) => SettingsAction::Change(SettingChange::HotkeyKeys(keys)),
        HotkeyListAction::Close => SettingsAction::CloseKeys,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hotkey::Shortcut;

    #[test]
    fn keys_chosen_in_the_list_change_the_setting() {
        let keys = Shortcut::CHOICES[1];
        assert_eq!(
            hotkey_action(None, false, Some(HotkeyListAction::Choose(keys))),
            Some(SettingsAction::Change(SettingChange::HotkeyKeys(keys)))
        );
    }

    #[test]
    fn the_button_that_closes_the_list_isnt_a_click_outside_it() {
        assert_eq!(
            hotkey_action(None, true, Some(HotkeyListAction::Close)),
            Some(SettingsAction::ToggleKeys)
        );
        assert_eq!(
            hotkey_action(None, false, Some(HotkeyListAction::Close)),
            Some(SettingsAction::CloseKeys)
        );
    }

    #[test]
    fn the_checkbox_turns_the_hotkey_on_or_off() {
        assert_eq!(
            hotkey_action(Some(false), false, None),
            Some(SettingsAction::Change(SettingChange::HotkeyOn(false)))
        );
        assert_eq!(hotkey_action(None, false, None), None);
    }

    #[test]
    fn the_checkbox_clicked_while_the_list_is_open_changes_the_setting() {
        // A change of a setting closes the list too (`OpenLists::settings`).
        assert_eq!(
            hotkey_action(Some(false), false, Some(HotkeyListAction::Close)),
            Some(SettingsAction::Change(SettingChange::HotkeyOn(false)))
        );
    }
}
