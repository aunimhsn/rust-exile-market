//! The settings of the player, and the file that keeps them between the launches.
//!
//! They are saved in `settings.toml`, in the configuration directory of the user, as soon as
//! they change. The size and the position of the window are kept by eframe, in `window.ron`
//! next to it.

use std::fs;
use std::io;
use std::mem;
use std::path::{Path, PathBuf};

use egui::WindowLevel;
use serde::{Deserialize, Serialize};

use crate::files;
use crate::hotkey::Shortcut;
use crate::language::Language;
use crate::league::League;
use crate::logging;
use crate::update::{ClosedUpdate, LeagueOfTheMoment, Release};

const FILE_NAME: &str = "settings.toml";
const WINDOW_FILE_NAME: &str = "window.ron";

/// The language chosen for the interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LanguageSetting {
    /// The language of the system, when the interface has it.
    #[default]
    System,
    English,
    French,
}

impl LanguageSetting {
    pub const ALL: [Self; 3] = [Self::System, Self::English, Self::French];

    /// The language of the interface.
    #[must_use]
    pub fn language(self) -> Language {
        match self {
            Self::System => Language::of_system(),
            Self::English => Language::English,
            Self::French => Language::French,
        }
    }
}

/// The price-check hotkey (`specs.md`, Settings): on by default, so that a file written before
/// it existed keeps it on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct HotkeySetting {
    /// Whether the hotkey copies in the game; off, its key is left to the game and to the other
    /// applications.
    pub on: bool,
    /// Its keys, written joined (`keys = "Alt+D"`): Ctrl+D by default, and for keys that can't be
    /// read or can't be the hotkey, written to the log, so that the other settings still read.
    #[serde(
        serialize_with = "serialize_shortcut",
        deserialize_with = "deserialize_shortcut"
    )]
    pub keys: Shortcut,
}

impl Default for HotkeySetting {
    fn default() -> Self {
        Self {
            on: true,
            keys: Shortcut::default(),
        }
    }
}

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "the signature serde asks for"
)]
fn serialize_shortcut<S: serde::Serializer>(
    shortcut: &Shortcut,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.collect_str(shortcut)
}

/// The keys of the file, or Ctrl+D when they can't be the hotkey, or aren't text (`keys = 5`).
fn deserialize_shortcut<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Shortcut, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Keys {
        Text(String),
        Other(serde::de::IgnoredAny),
    }

    let shortcut = Shortcut::default();
    Ok(match Keys::deserialize(deserializer)? {
        Keys::Text(text) => text.parse().unwrap_or_else(|error| {
            log::warn!("The hotkey \"{text}\" can't be used ({error}): {shortcut} instead");
            shortcut
        }),
        Keys::Other(_) => {
            log::warn!("The hotkey of the settings isn't text: {shortcut} instead");
            shortcut
        }
    })
}

/// A change the player makes in the settings panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingChange {
    Language(LanguageSetting),
    AlwaysOnTop(bool),
    /// The price-check hotkey turned on or off.
    HotkeyOn(bool),
    /// The keys of the price-check hotkey chosen.
    HotkeyKeys(Shortcut),
}

/// The settings of the player. A missing value takes its default, so that a file written by
/// an older version still reads.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    language: LanguageSetting,
    always_on_top: bool,
    hotkey: HotkeySetting,
    /// The chosen league; none until the player or the default chooses one.
    league: Option<League>,
    /// The new leagues whose bubble the player closed, or answered by choosing a league.
    closed_new_leagues: Vec<League>,
    /// The last update whose bubble the player closed, and the league of the moment then.
    closed_update: Option<ClosedUpdate>,
}

impl Settings {
    #[must_use]
    pub fn language(&self) -> LanguageSetting {
        self.language
    }

    #[must_use]
    pub fn always_on_top(&self) -> bool {
        self.always_on_top
    }

    #[must_use]
    pub fn hotkey(&self) -> HotkeySetting {
        self.hotkey
    }

    /// The league chosen at the last session, if any.
    #[must_use]
    pub fn league(&self) -> Option<League> {
        self.league.clone()
    }

    /// Keeps `league` as the chosen one; `false` when it already was.
    pub fn set_league(&mut self, league: &League) -> bool {
        if self.league.as_ref() == Some(league) {
            return false;
        }
        self.league = Some(league.clone());
        true
    }

    #[must_use]
    pub fn closed_new_leagues(&self) -> &[League] {
        &self.closed_new_leagues
    }

    /// Remembers that the player closed the bubble of `league`; `false` when it already was.
    pub fn close_new_league(&mut self, league: &League) -> bool {
        if self.closed_new_leagues.contains(league) {
            return false;
        }
        self.closed_new_leagues.push(league.clone());
        true
    }

    /// The last update whose bubble the player closed.
    #[must_use]
    pub fn closed_update(&self) -> Option<&ClosedUpdate> {
        self.closed_update.as_ref()
    }

    /// Closes the bubble of `release` during `league`, the league of the moment; whether it
    /// changed.
    pub fn close_update(&mut self, release: &Release, league: LeagueOfTheMoment) -> bool {
        let closed = Some(ClosedUpdate::new(release, league));
        if self.closed_update == closed {
            return false;
        }
        self.closed_update = closed;
        true
    }

    /// Gives the update closed before the leagues were known `league`, the league of the
    /// moment, once known; whether it changed.
    pub fn settle_closed_update(&mut self, league: LeagueOfTheMoment) -> bool {
        self.closed_update
            .as_mut()
            .is_some_and(|closed| closed.settle(league))
    }

    /// The level of the window: above the other windows when it is always on top.
    #[must_use]
    pub fn window_level(&self) -> WindowLevel {
        if self.always_on_top {
            WindowLevel::AlwaysOnTop
        } else {
            WindowLevel::Normal
        }
    }

    /// Applies `change`; `false` when the setting already had this value.
    pub fn apply(&mut self, change: SettingChange) -> bool {
        match change {
            SettingChange::Language(language) => {
                mem::replace(&mut self.language, language) != language
            }
            SettingChange::AlwaysOnTop(always_on_top) => {
                mem::replace(&mut self.always_on_top, always_on_top) != always_on_top
            }
            SettingChange::HotkeyOn(on) => mem::replace(&mut self.hotkey.on, on) != on,
            SettingChange::HotkeyKeys(keys) => mem::replace(&mut self.hotkey.keys, keys) != keys,
        }
    }
}

/// The file of the settings.
pub struct SettingsFile {
    /// `None` without a configuration directory: the settings then last only one session.
    directory: Option<PathBuf>,
}

impl SettingsFile {
    /// The file in the configuration directory of the user.
    #[must_use]
    pub fn in_config_directory() -> Self {
        Self {
            directory: config_directory(),
        }
    }

    /// The file where eframe keeps the size and the position of the window, next to the
    /// settings.
    #[must_use]
    pub fn window_file(&self) -> Option<PathBuf> {
        self.directory
            .as_ref()
            .map(|directory| directory.join(WINDOW_FILE_NAME))
    }

    /// The settings of the file, or the default ones when it is missing or can't be read:
    /// the next change then writes it again.
    #[must_use]
    pub fn load(&self) -> Settings {
        let Some(directory) = &self.directory else {
            log::warn!("No configuration directory: the settings won't be saved");
            return Settings::default();
        };
        let path = directory.join(FILE_NAME);
        log::info!("Settings file: {}", logging::without_home(&path).display());
        match read(&path) {
            Ok(Some(settings)) => {
                log::info!("Settings: {settings:?}");
                settings
            }
            Ok(None) => {
                log::info!("No settings file yet: the default settings are used");
                Settings::default()
            }
            Err(error) => {
                log::warn!(
                    "{} can't be read, the default settings are used: {error}",
                    logging::without_home(&path).display()
                );
                Settings::default()
            }
        }
    }

    /// Writes `settings` to the file. A failure is logged: the settings still apply to this
    /// session.
    pub fn save(&self, settings: &Settings) {
        let Some(directory) = &self.directory else {
            return;
        };
        match write(directory, settings) {
            Ok(()) => log::info!("Settings saved: {settings:?}"),
            Err(error) => log::warn!(
                "The settings couldn't be saved in {}: {error}",
                logging::without_home(directory).display()
            ),
        }
    }
}

fn config_directory() -> Option<PathBuf> {
    files::project_directories().map(|directories| directories.config_dir().to_path_buf())
}

/// The settings of the file at `path`; `None` when there is no file.
fn read(path: &Path) -> io::Result<Option<Settings>> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    toml::from_str(&text)
        .map(Some)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

fn write(directory: &Path, settings: &Settings) -> io::Result<()> {
    let text = toml::to_string(settings).map_err(io::Error::other)?;
    files::write_atomically(directory, FILE_NAME, text.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files::TestDirectory;

    impl TestDirectory {
        fn file(&self) -> SettingsFile {
            SettingsFile {
                directory: Some(self.0.clone()),
            }
        }

        fn write(&self, text: &str) {
            fs::create_dir_all(&self.0).unwrap();
            fs::write(self.0.join(FILE_NAME), text).unwrap();
        }
    }

    fn changed() -> Settings {
        let mut settings = Settings::default();
        settings.apply(SettingChange::Language(LanguageSetting::French));
        settings.apply(SettingChange::AlwaysOnTop(true));
        settings.apply(SettingChange::HotkeyOn(false));
        settings
    }

    #[test]
    fn the_default_settings_follow_the_system_below_the_other_windows() {
        let settings = Settings::default();

        assert_eq!(settings.language(), LanguageSetting::System);
        assert!(!settings.always_on_top());
        assert_eq!(settings.window_level(), WindowLevel::Normal);
    }

    #[test]
    fn the_hotkey_is_on_by_default_and_can_be_turned_off() {
        let mut settings = Settings::default();
        assert!(settings.hotkey().on);

        assert!(settings.apply(SettingChange::HotkeyOn(false)));
        assert!(!settings.apply(SettingChange::HotkeyOn(false)));
        assert!(!settings.hotkey().on);
    }

    #[test]
    fn the_keys_chosen_are_kept_and_saved() {
        let directory = TestDirectory::new("hotkey-keys-chosen");
        let mut settings = Settings::default();
        let keys = Shortcut::CHOICES[2];

        assert!(settings.apply(SettingChange::HotkeyKeys(keys)));
        assert!(!settings.apply(SettingChange::HotkeyKeys(keys)));
        directory.file().save(&settings);

        assert_eq!(directory.file().load().hotkey().keys.to_string(), "Alt+G");
    }

    #[test]
    fn a_change_says_whether_it_changed_something() {
        let mut settings = Settings::default();

        assert!(settings.apply(SettingChange::AlwaysOnTop(true)));
        assert!(!settings.apply(SettingChange::AlwaysOnTop(true)));
        assert!(settings.apply(SettingChange::Language(LanguageSetting::English)));
        assert!(!settings.apply(SettingChange::Language(LanguageSetting::English)));
        assert_eq!(settings.window_level(), WindowLevel::AlwaysOnTop);
    }

    #[test]
    fn keeps_the_chosen_league_and_the_closed_bubbles() {
        let mut settings = Settings::default();
        let league = League::new("HC Forbidden Rites");

        assert_eq!(settings.league(), None);
        assert!(settings.set_league(&league));
        assert!(!settings.set_league(&league));
        assert!(settings.close_new_league(&league));
        assert!(!settings.close_new_league(&league));

        assert_eq!(settings.league(), Some(league.clone()));
        assert_eq!(settings.closed_new_leagues(), [league]);
    }

    #[test]
    fn the_leagues_are_saved_with_the_other_settings() {
        let directory = TestDirectory::new("leagues");
        let file = directory.file();
        let mut settings = changed();
        settings.set_league(&League::new("Runes of Aldur"));
        settings.close_new_league(&League::new("HC Runes of Aldur"));

        file.save(&settings);

        assert_eq!(file.load(), settings);
    }

    fn release(version: &str) -> Release {
        Release {
            version: crate::update::Version::parse(version).expect("a version"),
            page: String::new(),
        }
    }

    #[test]
    fn the_closed_update_is_saved_with_its_league() {
        let directory = TestDirectory::new("closed-update");
        let file = directory.file();
        let mut settings = Settings::default();

        let league = LeagueOfTheMoment::League("Rise of the Abyssal");
        assert!(settings.close_update(&release("0.2.0"), league));
        assert!(!settings.close_update(&release("0.2.0"), league));
        file.save(&settings);

        let table: toml::Table = toml::from_str(&toml::to_string(&settings).unwrap()).unwrap();
        assert_eq!(table["closed_update"]["version"].as_str(), Some("0.2.0"));
        assert_eq!(
            table["closed_update"]["league"].as_str(),
            Some("Rise of the Abyssal")
        );
        assert_eq!(file.load(), settings);
    }

    #[test]
    fn an_update_closed_before_the_leagues_takes_the_league_of_the_moment() {
        let abyss = LeagueOfTheMoment::League("Rise of the Abyssal");
        let mut settings = Settings::default();
        assert!(!settings.settle_closed_update(abyss));

        settings.close_update(&release("0.2.0"), LeagueOfTheMoment::Unknown);

        assert!(!settings.settle_closed_update(LeagueOfTheMoment::Unknown));
        assert!(settings.settle_closed_update(abyss));
        assert!(!settings.settle_closed_update(LeagueOfTheMoment::League("Runes of Aldur")));
        assert_eq!(
            settings
                .closed_update()
                .and_then(|closed| closed.league.as_deref()),
            Some("Rise of the Abyssal")
        );
    }

    #[test]
    fn an_update_closed_between_two_leagues_is_saved_with_an_empty_league() {
        let directory = TestDirectory::new("closed-update-between");
        let file = directory.file();
        let mut settings = Settings::default();

        settings.close_update(&release("0.2.0"), LeagueOfTheMoment::Between);
        file.save(&settings);

        let table: toml::Table = toml::from_str(&toml::to_string(&settings).unwrap()).unwrap();
        assert_eq!(table["closed_update"]["league"].as_str(), Some(""));
        assert_eq!(file.load(), settings);
    }

    #[test]
    fn a_fixed_language_setting_gives_its_language() {
        assert_eq!(LanguageSetting::English.language(), Language::English);
        assert_eq!(LanguageSetting::French.language(), Language::French);
    }

    #[test]
    fn the_file_is_readable_toml() {
        let text = toml::to_string(&changed()).unwrap();
        let table: toml::Table = toml::from_str(&text).unwrap();

        assert_eq!(table["language"].as_str(), Some("french"));
        assert_eq!(table["always_on_top"].as_bool(), Some(true));
        assert_eq!(table["hotkey"]["on"].as_bool(), Some(false));
        assert_eq!(table["hotkey"]["keys"].as_str(), Some("Ctrl+D"));
    }

    #[test]
    fn the_keys_of_the_hotkey_are_read_from_the_file() {
        let directory = TestDirectory::new("hotkey-keys");
        directory.write("[hotkey]\nkeys = \"alt + d\"\n");

        let settings = directory.file().load();

        assert_eq!(settings.hotkey().keys.to_string(), "Alt+D");
        assert!(settings.hotkey().on);
    }

    #[test]
    fn keys_that_cant_be_the_hotkey_give_ctrl_d_and_keep_the_other_settings() {
        for keys in [
            "\"Ctrl+C\"",
            "\"D\"",
            "\"Super+D\"",
            "\"\"",
            "5",
            "[\"Alt\", \"D\"]",
        ] {
            let directory = TestDirectory::new("hotkey-refused");
            directory.write(&format!(
                "always_on_top = true\n[hotkey]\non = false\nkeys = {keys}\n"
            ));

            let settings = directory.file().load();

            assert_eq!(settings.hotkey().keys, Shortcut::default(), "{keys}");
            assert!(!settings.hotkey().on, "{keys}");
            assert!(settings.always_on_top(), "{keys}");
        }
    }

    #[test]
    fn saved_settings_load_back() {
        let directory = TestDirectory::new("round-trip");
        let file = directory.file();

        file.save(&changed());

        assert_eq!(file.load(), changed());
    }

    #[test]
    fn a_new_save_replaces_the_previous_one() {
        let directory = TestDirectory::new("replace");
        let file = directory.file();

        file.save(&changed());
        file.save(&Settings::default());

        assert_eq!(file.load(), Settings::default());
    }

    #[test]
    fn a_missing_file_gives_the_default_settings() {
        let directory = TestDirectory::new("missing");

        assert_eq!(directory.file().load(), Settings::default());
    }

    #[test]
    fn a_missing_value_takes_its_default_and_an_unknown_one_is_ignored() {
        let directory = TestDirectory::new("partial");
        directory.write("always_on_top = true\nleague = \"Forbidden Rites\"\n");

        let settings = directory.file().load();

        assert_eq!(settings.language(), LanguageSetting::System);
        assert!(settings.always_on_top());
        // Written before the hotkey setting existed: the hotkey stays on, with Ctrl+D.
        assert_eq!(settings.hotkey(), HotkeySetting::default());
    }

    #[test]
    fn an_unreadable_file_gives_the_default_settings() {
        for text in [
            "language = \"german\"",
            "always_on_top = \"yes\"",
            "not toml",
        ] {
            let directory = TestDirectory::new("unreadable");
            directory.write(text);

            assert_eq!(directory.file().load(), Settings::default(), "{text}");
        }
    }

    #[test]
    fn without_a_configuration_directory_nothing_is_written() {
        let file = SettingsFile { directory: None };

        file.save(&changed());

        assert_eq!(file.load(), Settings::default());
    }
}
