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
fn a_missing_value_takes_its_default() {
    let directory = TestDirectory::new("partial");
    directory.write("always_on_top = true\nleague = \"Forbidden Rites\"\n");

    let settings = directory.file().load();

    assert_eq!(settings.language(), LanguageSetting::System);
    assert!(settings.always_on_top());
    // Written before the hotkey setting existed: the hotkey stays on, with Ctrl+D.
    assert_eq!(settings.hotkey(), HotkeySetting::default());
}

/// The core review of 2026-11-10: one value this version didn't know lost every
/// setting, the file being replaced by the default ones a few seconds later.
#[test]
fn a_value_that_cant_be_read_takes_its_default_and_the_others_are_kept() {
    let kept = "always_on_top = true\nleague = \"Forbidden Rites\"\n";
    for odd in [
        "language = \"german\"",
        "language = 5",
        "closed_new_leagues = \"none\"",
        "closed_update = { league = \"Forbidden Rites\" }",
        "hotkey = 5",
        "[hotkey]\non = \"yes\"\nkeys = \"Alt+G\"",
    ] {
        let directory = TestDirectory::new("odd-value");
        directory.write(&format!("{kept}{odd}\n"));

        let settings = directory.file().load();

        assert!(settings.always_on_top(), "{odd}");
        assert_eq!(
            settings.league(),
            Some(League::new("Forbidden Rites")),
            "{odd}"
        );
        assert_eq!(settings.language(), LanguageSetting::System, "{odd}");
        assert!(settings.hotkey().on, "{odd}");
        assert!(directory.0.join(FILE_NAME).exists(), "{odd}");
    }
    let directory = TestDirectory::new("odd-value-of-the-hotkey");
    directory.write("[hotkey]\non = \"yes\"\nkeys = \"Alt+G\"\n");
    assert_eq!(directory.file().load().hotkey().keys.to_string(), "Alt+G");
    directory.write("always_on_top = \"yes\"\nlanguage = \"french\"\n");
    assert_eq!(directory.file().load().language(), LanguageSetting::French);
}

#[test]
fn a_file_that_isnt_toml_gives_the_default_settings_and_is_kept_aside() {
    let directory = TestDirectory::new("unreadable");
    directory.write("not toml");
    let file = directory.file();

    assert_eq!(file.load(), Settings::default());

    assert!(!directory.0.join(FILE_NAME).exists());
    let aside = fs::read_to_string(directory.0.join(INVALID_FILE_NAME)).unwrap();
    assert_eq!(aside, "not toml");
    // The default settings saved after it don't touch the file kept aside, and a second
    // unreadable file replaces the first one there.
    file.save(&changed());
    directory.write("still = not toml");
    assert_eq!(file.load(), Settings::default());
    let aside = fs::read_to_string(directory.0.join(INVALID_FILE_NAME)).unwrap();
    assert_eq!(aside, "still = not toml");
}

#[test]
fn without_a_configuration_directory_nothing_is_written() {
    let file = SettingsFile { directory: None };

    file.save(&changed());

    assert_eq!(file.load(), Settings::default());
}

/// Every setting of the version 0.1.0, away from its default.
fn every_setting() -> Settings {
    let mut settings = changed();
    settings.apply(SettingChange::HotkeyKeys(Shortcut::CHOICES[2]));
    settings.set_league(&League::new("HC Forbidden Rites"));
    settings.close_new_league(&League::new("Runes of Aldur"));
    settings.close_new_league(&League::new("HC Runes of Aldur"));
    settings.close_update(
        &release("0.2.0"),
        LeagueOfTheMoment::League("Forbidden Rites"),
    );
    settings
}

/// The file the version 0.1.0 writes, every setting changed. Once that version is out,
/// this file no longer changes: the players have it. A version that changes the format
/// then adds its own file and test, and still reads this one.
const VERSION_0_1_0: &str = include_str!("settings-v0.1.0.toml");

/// The core review of 2026-11-10: nothing fixed the file of a released version.
#[test]
fn the_file_of_the_version_0_1_0_is_read_with_every_setting() {
    let directory = TestDirectory::new("version-0-1-0");
    directory.write(VERSION_0_1_0);

    let settings = directory.file().load();

    assert_eq!(settings, every_setting());
    assert_eq!(settings.language(), LanguageSetting::French);
    assert!(settings.always_on_top());
    assert!(!settings.hotkey().on);
    assert_eq!(settings.hotkey().keys.to_string(), "Alt+G");
    assert_eq!(settings.league(), Some(League::new("HC Forbidden Rites")));
    assert_eq!(
        settings.closed_new_leagues(),
        ["Runes of Aldur", "HC Runes of Aldur"].map(League::new)
    );
    let closed = settings.closed_update().expect("a closed update");
    assert_eq!(closed.league.as_deref(), Some("Forbidden Rites"));
    assert!(directory.0.join(FILE_NAME).exists(), "not set aside");
}

/// Until a version changes the format on purpose, with its own file next to this one.
#[test]
fn this_version_writes_the_file_of_the_version_0_1_0() {
    let directory = TestDirectory::new("version-0-1-0-written");

    directory.file().save(&every_setting());

    let written = fs::read_to_string(directory.0.join(FILE_NAME)).unwrap();
    assert_eq!(written, VERSION_0_1_0);
}

/// The file of a later version, read by this one after a downgrade: what it doesn't
/// know is left out, at the root and in a table, and the rest is read. A
/// `deny_unknown_fields` added by mistake would lose every setting instead.
#[test]
fn the_settings_of_a_later_version_are_ignored_and_the_others_are_read() {
    let directory = TestDirectory::new("later-version");
    let later = VERSION_0_1_0
        .replacen("language", "future_setting = 1\nlanguage", 1)
        .replacen("[hotkey]", "[hotkey]\nsound = true", 1)
        .replacen("[closed_update]", "[closed_update]\nnotes = [1, 2]", 1);
    directory.write(&format!("{later}\n[overlay]\nopacity = 0.8\n"));

    let settings = directory.file().load();

    assert_eq!(settings, every_setting());
    assert!(directory.0.join(FILE_NAME).exists(), "not set aside");
}

/// A settings folder that can't be made, whatever the rights of the user running the
/// tests: its path goes through an ordinary file.
#[test]
fn a_failed_write_keeps_the_settings_of_the_session_without_a_panic() {
    let directory = TestDirectory::new("blocked");
    fs::create_dir_all(&directory.0).unwrap();
    fs::write(directory.0.join("blocked"), "").unwrap();
    let file = SettingsFile {
        directory: Some(directory.0.join("blocked").join("settings")),
    };

    file.save(&changed());

    assert_eq!(file.load(), Settings::default());
    assert_eq!(fs::read(directory.0.join("blocked")).unwrap(), b"");
}
