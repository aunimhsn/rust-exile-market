//! The settings of the player, and the file that keeps them between the launches.
//!
//! They are saved in `settings.toml`, in the configuration directory of the user, as soon as
//! they change. The size and the position of the window are kept by eframe, in `window.ron`
//! next to it.

use std::fs;
use std::io::{self, Write};
use std::mem;
use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use egui::WindowLevel;
use serde::{Deserialize, Serialize};

use crate::language::Language;
use crate::logging;

const FILE_NAME: &str = "settings.toml";
/// The new settings are written to this file and synced to the disk first, then renamed, so
/// that a crash while writing never leaves half a file.
const NEW_FILE_NAME: &str = "settings.toml.new";
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

/// A change the player makes in the settings panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingChange {
    Language(LanguageSetting),
    AlwaysOnTop(bool),
}

/// The settings of the player. A missing value takes its default, so that a file written by
/// an older version still reads.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    language: LanguageSetting,
    always_on_top: bool,
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
    ProjectDirs::from("", "", crate::FOLDER_NAME)
        .map(|directories| directories.config_dir().to_path_buf())
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
    fs::create_dir_all(directory)?;
    let new_path = directory.join(NEW_FILE_NAME);
    {
        let mut file = fs::File::create(&new_path)?;
        file.write_all(text.as_bytes())?;
        // On the disk before the rename makes it the settings file; closed at the end of
        // the block, before the rename.
        file.sync_all()?;
    }
    fs::rename(new_path, directory.join(FILE_NAME))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A folder of its own for one test, removed when it ends.
    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir()
                .join(format!("exile-market-test-{}-{name}", std::process::id()));
            let _ = fs::remove_dir_all(&path);
            Self(path)
        }

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

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn changed() -> Settings {
        let mut settings = Settings::default();
        settings.apply(SettingChange::Language(LanguageSetting::French));
        settings.apply(SettingChange::AlwaysOnTop(true));
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
    fn a_change_says_whether_it_changed_something() {
        let mut settings = Settings::default();

        assert!(settings.apply(SettingChange::AlwaysOnTop(true)));
        assert!(!settings.apply(SettingChange::AlwaysOnTop(true)));
        assert!(settings.apply(SettingChange::Language(LanguageSetting::English)));
        assert!(!settings.apply(SettingChange::Language(LanguageSetting::English)));
        assert_eq!(settings.window_level(), WindowLevel::AlwaysOnTop);
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
    }

    #[test]
    fn saved_settings_load_back() {
        let directory = TestDirectory::new("round-trip");
        let file = directory.file();

        file.save(&changed());

        assert_eq!(file.load(), changed());
        assert!(!directory.0.join(NEW_FILE_NAME).exists());
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
