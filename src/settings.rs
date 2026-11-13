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
/// Where a settings file that isn't TOML is moved before the default settings replace it.
const INVALID_FILE_NAME: &str = "settings.invalid.toml";

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
    #[serde(deserialize_with = "or_on")]
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

/// The value of the key `key` of the file, or its default when this version can't read it
/// (`language = "german"`, written by a later version or by hand), so that the other settings
/// still read: a file read as a whole lost them all for one value.
fn or_else<'de, D, T>(
    deserializer: D,
    key: &str,
    default: impl FnOnce() -> T,
) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::de::DeserializeOwned,
{
    let value = toml::Value::deserialize(deserializer)?;
    Ok(value.try_into().unwrap_or_else(|error: toml::de::Error| {
        log::warn!(
            "The setting `{key}` can't be read ({}): its default instead",
            logging::plain(error.message())
        );
        default()
    }))
}

/// A function for `deserialize_with` that reads the key `$key` with [`or_else`] and its
/// default: serde gives such a function no argument, so each key has its own, to be named in
/// the log.
macro_rules! or_default {
    ($name:ident, $key:literal) => {
        fn $name<'de, D, T>(deserializer: D) -> Result<T, D::Error>
        where
            D: serde::Deserializer<'de>,
            T: serde::de::DeserializeOwned + Default,
        {
            or_else(deserializer, $key, T::default)
        }
    };
}

or_default!(language_or_default, "language");
or_default!(always_on_top_or_default, "always_on_top");
or_default!(hotkey_or_default, "hotkey");
or_default!(league_or_default, "league");
or_default!(closed_new_leagues_or_default, "closed_new_leagues");
or_default!(closed_update_or_default, "closed_update");

/// Whether the hotkey is on: on when the file doesn't say it readably.
fn or_on<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<bool, D::Error> {
    or_else(deserializer, "hotkey.on", || HotkeySetting::default().on)
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
/// an older version still reads; so does a value this version can't read ([`or_else`]),
/// and a value it doesn't know is ignored, so that a file written by a later version reads
/// too.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    #[serde(deserialize_with = "language_or_default")]
    language: LanguageSetting,
    #[serde(deserialize_with = "always_on_top_or_default")]
    always_on_top: bool,
    #[serde(deserialize_with = "hotkey_or_default")]
    hotkey: HotkeySetting,
    /// The chosen league; none until the player or the default chooses one.
    #[serde(deserialize_with = "league_or_default")]
    league: Option<League>,
    /// The new leagues whose bubble the player closed, or answered by choosing a league.
    #[serde(deserialize_with = "closed_new_leagues_or_default")]
    closed_new_leagues: Vec<League>,
    /// The last update whose bubble the player closed, and the league of the moment then.
    #[serde(deserialize_with = "closed_update_or_default")]
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
    /// the next change then writes it again. A file that isn't TOML is first moved aside
    /// (`settings.invalid.toml`): the default league is saved a few seconds after the launch,
    /// which replaced it before the player could see what was wrong in it.
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
                if error.kind() == io::ErrorKind::InvalidData {
                    set_aside(directory, &path);
                }
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

/// Moves the file at `path`, which isn't TOML, next to itself, in place of the one moved
/// there before; a failure is logged, and the file is then replaced at the next change.
fn set_aside(directory: &Path, path: &Path) {
    let aside = directory.join(INVALID_FILE_NAME);
    match fs::rename(path, &aside) {
        Ok(()) => log::warn!("It is kept as {}", logging::without_home(&aside).display()),
        Err(error) => log::warn!("It couldn't be kept aside: {error}"),
    }
}

fn write(directory: &Path, settings: &Settings) -> io::Result<()> {
    let text = toml::to_string(settings).map_err(io::Error::other)?;
    files::write_atomically(directory, FILE_NAME, text.as_bytes())
}

#[cfg(test)]
mod tests;
