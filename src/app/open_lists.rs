//! The lists open over the view, the leagues and the keys of the hotkey: one at a time, so that
//! opening one closes the other. What opens and closes them, without egui, so that the rules are
//! tested.

use crate::settings::SettingChange;
use crate::ui::SettingsAction;

/// A list open over the view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenList {
    /// The leagues, under the title bar.
    Leagues,
    /// The keys of the hotkey, in the settings.
    Keys,
}

/// The list open over the view, if any.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct OpenLists(Option<OpenList>);

impl OpenLists {
    /// Whether `list` is open.
    pub fn is_open(self, list: OpenList) -> bool {
        self.0 == Some(list)
    }

    /// Opens `list`, closing the other, or closes it when it is the one open.
    pub fn toggle(&mut self, list: OpenList) {
        self.0 = (!self.is_open(list)).then_some(list);
    }

    /// Closes `list`, when it is the one open: a click outside a list told late doesn't close the
    /// other, just opened by that click.
    pub fn close(&mut self, list: OpenList) {
        if self.is_open(list) {
            self.0 = None;
        }
    }

    /// Closes the list open, if any.
    pub fn close_all(&mut self) {
        self.0 = None;
    }

    /// Escape closes what is on top: the list open, if any; whether it closed one, so that the
    /// same press doesn't close the settings under it too.
    pub fn escape(&mut self) -> bool {
        self.0.take().is_some()
    }

    /// Takes in what the player did in the settings: the button of the keys opens or closes their
    /// list; a click outside it closes it, a change of any setting too, since it is a click
    /// outside the list (the mock-up), and the change is given back to apply.
    pub fn settings(&mut self, action: SettingsAction) -> Option<SettingChange> {
        match action {
            SettingsAction::Change(change) => {
                self.close(OpenList::Keys);
                Some(change)
            }
            SettingsAction::ToggleKeys => {
                self.toggle(OpenList::Keys);
                None
            }
            SettingsAction::CloseKeys => {
                self.close(OpenList::Keys);
                None
            }
        }
    }

    /// Follows the view, `settings` shown or not: the list of the keys goes with the settings, so
    /// that it is closed when they open again.
    pub fn follow_view(&mut self, settings: bool) {
        if !settings {
            self.close(OpenList::Keys);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hotkey::Shortcut;
    use crate::settings::LanguageSetting;

    /// The lists with `list` open.
    fn open(list: OpenList) -> OpenLists {
        let mut lists = OpenLists::default();
        lists.toggle(list);
        lists
    }

    #[test]
    fn one_list_at_a_time() {
        let mut lists = open(OpenList::Keys);

        lists.toggle(OpenList::Leagues);
        assert!(lists.is_open(OpenList::Leagues));
        assert!(!lists.is_open(OpenList::Keys));
    }

    #[test]
    fn its_button_closes_the_list_it_opened() {
        let mut lists = open(OpenList::Keys);

        lists.toggle(OpenList::Keys);
        assert_eq!(lists, OpenLists::default());
    }

    #[test]
    fn a_late_close_of_a_list_leaves_the_other_open() {
        let mut lists = open(OpenList::Leagues);

        lists.close(OpenList::Keys);
        assert!(lists.is_open(OpenList::Leagues));
    }

    #[test]
    fn escape_closes_the_list_before_the_settings() {
        let mut lists = open(OpenList::Keys);

        assert!(lists.escape());
        assert!(!lists.escape());
    }

    #[test]
    fn keys_chosen_close_their_list() {
        let mut lists = open(OpenList::Keys);
        let change = SettingChange::HotkeyKeys(Shortcut::CHOICES[3]);

        assert_eq!(lists.settings(SettingsAction::Change(change)), Some(change));
        assert_eq!(lists, OpenLists::default());
    }

    #[test]
    fn a_click_on_another_setting_closes_the_list_and_still_changes_it() {
        for change in [
            SettingChange::Language(LanguageSetting::French),
            SettingChange::AlwaysOnTop(true),
            SettingChange::HotkeyOn(false),
        ] {
            let mut lists = open(OpenList::Keys);

            assert_eq!(lists.settings(SettingsAction::Change(change)), Some(change));
            assert_eq!(lists, OpenLists::default());
        }
    }

    #[test]
    fn a_change_of_a_setting_leaves_the_leagues_open() {
        let mut lists = open(OpenList::Leagues);
        let change = SettingChange::AlwaysOnTop(true);

        lists.settings(SettingsAction::Change(change));
        assert!(lists.is_open(OpenList::Leagues));
    }

    #[test]
    fn the_button_of_the_keys_opens_and_closes_their_list() {
        let mut lists = OpenLists::default();

        assert_eq!(lists.settings(SettingsAction::ToggleKeys), None);
        assert!(lists.is_open(OpenList::Keys));
        lists.settings(SettingsAction::ToggleKeys);
        assert_eq!(lists, OpenLists::default());
        lists.toggle(OpenList::Keys);
        lists.settings(SettingsAction::CloseKeys);
        assert_eq!(lists, OpenLists::default());
    }

    #[test]
    fn the_list_of_the_keys_goes_with_the_settings() {
        let mut lists = open(OpenList::Keys);
        lists.follow_view(true);
        assert!(lists.is_open(OpenList::Keys));
        lists.follow_view(false);
        assert_eq!(lists, OpenLists::default());

        // The leagues stay open over the item.
        let mut lists = open(OpenList::Leagues);
        lists.follow_view(false);
        assert!(lists.is_open(OpenList::Leagues));
    }
}
