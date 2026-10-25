//! Interface translations: the Fluent messages of `locales/`, embedded in the executable.
//!
//! The code never names a message by a string: the messages without arguments are the
//! variants of [`Message`], and each message with arguments has its own method of
//! [`Translator`], the only place that knows the names of its arguments.

use std::time::Duration;

use fluent_bundle::{FluentArgs, FluentBundle, FluentResource};

use crate::language::Language;

/// The Fluent messages of a language, embedded in the executable.
fn messages(language: Language) -> &'static str {
    match language {
        Language::English => include_str!("../locales/en.ftl"),
        Language::French => include_str!("../locales/fr.ftl"),
    }
}

/// A message of the interface without arguments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Message {
    LeagueModeSoftcore,
    LeagueModeHardcore,
    LeagueLoading,
    NewLeagueSwitch,
    HomeTooltip,
    SettingsTooltip,
    MinimizeTooltip,
    CloseTooltip,
    StatusPricesUnavailable,
    StatusPricesDownloading,
    StatusTradeIdle,
    EmptyCopy,
    EmptyPrice,
    ClipboardUnreadable,
    PricesUnavailable,
    ItemUnreadable,
    ItemUnreadableHint,
    SettingsTitle,
    SettingsLanguage,
    LanguageSystem,
    LanguageEnglish,
    LanguageFrench,
    SettingsAlwaysOnTop,
    Disclaimer,
}

impl Message {
    /// Every message, for the tests.
    #[cfg(test)]
    const ALL: [Self; 24] = [
        Self::LeagueModeSoftcore,
        Self::LeagueModeHardcore,
        Self::LeagueLoading,
        Self::NewLeagueSwitch,
        Self::HomeTooltip,
        Self::SettingsTooltip,
        Self::MinimizeTooltip,
        Self::CloseTooltip,
        Self::StatusPricesUnavailable,
        Self::StatusPricesDownloading,
        Self::StatusTradeIdle,
        Self::EmptyCopy,
        Self::EmptyPrice,
        Self::ClipboardUnreadable,
        Self::PricesUnavailable,
        Self::ItemUnreadable,
        Self::ItemUnreadableHint,
        Self::SettingsTitle,
        Self::SettingsLanguage,
        Self::LanguageSystem,
        Self::LanguageEnglish,
        Self::LanguageFrench,
        Self::SettingsAlwaysOnTop,
        Self::Disclaimer,
    ];

    /// Its identifier in the `.ftl` files.
    fn id(self) -> &'static str {
        match self {
            Self::LeagueModeSoftcore => "league-mode-softcore",
            Self::LeagueModeHardcore => "league-mode-hardcore",
            Self::LeagueLoading => "league-loading",
            Self::NewLeagueSwitch => "new-league-switch",
            Self::HomeTooltip => "home-tooltip",
            Self::SettingsTooltip => "settings-tooltip",
            Self::MinimizeTooltip => "minimize-tooltip",
            Self::CloseTooltip => "close-tooltip",
            Self::StatusPricesUnavailable => "status-prices-unavailable",
            Self::StatusPricesDownloading => "status-prices-downloading",
            Self::StatusTradeIdle => "status-trade-idle",
            Self::EmptyCopy => "empty-copy",
            Self::EmptyPrice => "empty-price",
            Self::ClipboardUnreadable => "clipboard-unreadable",
            Self::PricesUnavailable => "prices-unavailable",
            Self::ItemUnreadable => "item-unreadable",
            Self::ItemUnreadableHint => "item-unreadable-hint",
            Self::SettingsTitle => "settings-title",
            Self::SettingsLanguage => "settings-language",
            Self::LanguageSystem => "language-system",
            Self::LanguageEnglish => "language-english",
            Self::LanguageFrench => "language-french",
            Self::SettingsAlwaysOnTop => "settings-always-on-top",
            Self::Disclaimer => "disclaimer",
        }
    }
}

/// The identifiers of the messages with arguments, each read by its own method.
const STATUS_BAR: &str = "status-bar";
const LEAGUE_TOOLTIP: &str = "league-tooltip";
const STATUS_AUTHOR: &str = "status-author";
const NEW_LEAGUE: &str = "new-league";
const STATUS_PRICES: &str = "status-prices";
const STATUS_PRICES_OFFLINE: &str = "status-prices-offline";
const PRICES_UNREACHABLE: &str = "prices-unreachable";
const AGE_NOW: &str = "age-now";
const AGE_MINUTES: &str = "age-minutes";
const AGE_HOURS: &str = "age-hours";
const AGE_DAYS: &str = "age-days";
const DOWNLOAD_TITLE: &str = "download-title";
/// The sentences under the bar of the download of the prices, one drawn at random per
/// download.
const DOWNLOAD_QUOTES: [&str; 10] = [
    "download-quote-1",
    "download-quote-2",
    "download-quote-3",
    "download-quote-4",
    "download-quote-5",
    "download-quote-6",
    "download-quote-7",
    "download-quote-8",
    "download-quote-9",
    "download-quote-10",
];
/// The number of sentences of the download of the prices.
pub const DOWNLOAD_QUOTE_COUNT: usize = DOWNLOAD_QUOTES.len();

/// Gives the texts of the interface in one language.
pub struct Translator {
    bundle: FluentBundle<FluentResource>,
}

impl Translator {
    /// # Panics
    ///
    /// If the embedded messages of `language` can't be parsed, which the tests rule out.
    #[must_use]
    pub fn new(language: Language) -> Self {
        let resource = FluentResource::try_new(messages(language).to_owned())
            .unwrap_or_else(|(_, errors)| panic!("invalid messages for {language:?}: {errors:?}"));
        let mut bundle = FluentBundle::new(vec![language.identifier()]);
        // Without the Unicode isolation marks around the arguments, which egui would draw as boxes.
        bundle.set_use_isolating(false);
        bundle
            .add_resource(resource)
            .unwrap_or_else(|errors| panic!("duplicate messages for {language:?}: {errors:?}"));
        Self { bundle }
    }

    /// The text of `message`.
    #[must_use]
    pub fn text(&self, message: Message) -> String {
        self.format(message.id(), None)
    }

    /// The line of the status bar: the state of the prices, then of the trade site.
    #[must_use]
    pub fn status_bar(&self, prices: &str, trade: &str) -> String {
        let mut args = FluentArgs::new();
        args.set("prices", prices);
        args.set("trade", trade);
        self.format(STATUS_BAR, Some(&args))
    }

    /// The state of the prices in the status bar, with their age (`12min ago`).
    #[must_use]
    pub fn status_prices(&self, age: &str) -> String {
        self.format_age(STATUS_PRICES, age)
    }

    /// The state of the prices in the status bar while the price database can't be reached.
    #[must_use]
    pub fn status_prices_offline(&self, age: &str) -> String {
        self.format_age(STATUS_PRICES_OFFLINE, age)
    }

    /// The warning of a price database that can't be reached, with the age of the prices.
    #[must_use]
    pub fn prices_unreachable(&self, age: &str) -> String {
        self.format_age(PRICES_UNREACHABLE, age)
    }

    /// How long ago something happened, rounded down: `12min ago`, `3h ago`, `2d ago`.
    #[must_use]
    pub fn age(&self, age: Duration) -> String {
        let minutes = age.as_secs() / 60;
        let (id, name, count) = match minutes {
            0 => return self.format(AGE_NOW, None),
            1..60 => (AGE_MINUTES, "minutes", minutes),
            60..1440 => (AGE_HOURS, "hours", minutes / 60),
            _ => (AGE_DAYS, "days", minutes / 1440),
        };
        let mut args = FluentArgs::new();
        args.set(name, count);
        self.format(id, Some(&args))
    }

    /// The title of the download of the prices, with the name of the league.
    #[must_use]
    pub fn download_title(&self, league: &str) -> String {
        let mut args = FluentArgs::new();
        args.set("league", league);
        self.format(DOWNLOAD_TITLE, Some(&args))
    }

    /// The sentence `index` under the bar of the download of the prices, taken modulo their
    /// number.
    #[must_use]
    pub fn download_quote(&self, index: usize) -> String {
        // The modulo keeps the index in the array.
        self.format(DOWNLOAD_QUOTES[index % DOWNLOAD_QUOTE_COUNT], None)
    }

    /// The tooltip of the league block, with the mode of the league (`Softcore`).
    #[must_use]
    pub fn league_tooltip(&self, mode: &str) -> String {
        let mut args = FluentArgs::new();
        args.set("mode", mode);
        self.format(LEAGUE_TOOLTIP, Some(&args))
    }

    /// The bubble of a new league, with its name.
    #[must_use]
    pub fn new_league(&self, league: &str) -> String {
        let mut args = FluentArgs::new();
        args.set("league", league);
        self.format(NEW_LEAGUE, Some(&args))
    }

    /// The line of the status bar while the settings are open: the application, its version,
    /// the author, then the author's site.
    #[must_use]
    pub fn author_line(&self, app: &str, version: &str, author: &str, site: &str) -> String {
        let mut args = FluentArgs::new();
        args.set("app", app);
        args.set("version", version);
        args.set("author", author);
        args.set("site", site);
        self.format(STATUS_AUTHOR, Some(&args))
    }

    fn format_age(&self, id: &str, age: &str) -> String {
        let mut args = FluentArgs::new();
        args.set("age", age);
        self.format(id, Some(&args))
    }

    /// The text of the message `id`; the id itself when the message is missing, which the
    /// tests rule out.
    fn format(&self, id: &str, args: Option<&FluentArgs>) -> String {
        let Some(pattern) = self
            .bundle
            .get_message(id)
            .and_then(|message| message.value())
        else {
            debug_assert!(false, "missing message {id}");
            return id.to_owned();
        };
        let mut errors = Vec::new();
        let text = self
            .bundle
            .format_pattern(pattern, args, &mut errors)
            .into_owned();
        debug_assert!(errors.is_empty(), "errors in message {id}: {errors:?}");
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn file_ids(language: Language) -> BTreeSet<String> {
        let resource =
            FluentResource::try_new(messages(language).to_owned()).expect("the messages parse");
        resource
            .entries()
            .filter_map(|entry| match entry {
                fluent_syntax::ast::Entry::Message(message) => Some(message.id.name.to_owned()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_code_and_every_file_have_the_same_messages() {
        let code_ids: BTreeSet<String> = Message::ALL
            .iter()
            .map(|message| message.id())
            .chain([
                STATUS_BAR,
                LEAGUE_TOOLTIP,
                STATUS_AUTHOR,
                NEW_LEAGUE,
                STATUS_PRICES,
                STATUS_PRICES_OFFLINE,
                PRICES_UNREACHABLE,
                AGE_NOW,
                AGE_MINUTES,
                AGE_HOURS,
                AGE_DAYS,
                DOWNLOAD_TITLE,
            ])
            .chain(DOWNLOAD_QUOTES)
            .map(str::to_owned)
            .collect();
        for language in Language::ALL {
            assert_eq!(file_ids(language), code_ids, "messages of {language:?}");
        }
    }

    #[test]
    fn every_message_formats_without_errors() {
        // The errors fail the `debug_assert` of `format`.
        for language in Language::ALL {
            let translator = Translator::new(language);
            for message in Message::ALL {
                assert_ne!(translator.text(message), "", "{message:?} in {language:?}");
            }
            assert_ne!(translator.status_bar("A", "B"), "", "{language:?}");
            assert_ne!(translator.league_tooltip("A"), "", "{language:?}");
            assert!(
                translator.new_league("League").contains("League"),
                "{language:?}"
            );
            for age in [0, 60, 3600, 86_400] {
                assert_ne!(translator.age(Duration::from_secs(age)), "", "{language:?}");
            }
            assert!(
                translator.download_title("League").contains("League"),
                "{language:?}"
            );
            for index in 0..DOWNLOAD_QUOTE_COUNT {
                assert_ne!(translator.download_quote(index), "", "{language:?}");
            }
            for line in [
                translator.status_prices("age"),
                translator.status_prices_offline("age"),
                translator.prices_unreachable("age"),
            ] {
                assert!(line.contains("age"), "{line} in {language:?}");
            }
            assert_ne!(
                translator.author_line("App", "1", "A", "a.b"),
                "",
                "{language:?}"
            );
        }
    }

    #[test]
    fn arguments_are_inserted_without_isolation_marks() {
        assert_eq!(
            Translator::new(Language::English).status_bar("A", "B"),
            "A · B"
        );
    }

    #[test]
    fn the_author_line_ends_with_the_site_in_every_language() {
        // The status bar draws the site as a link, cut from the end of the line.
        for language in Language::ALL {
            let line =
                Translator::new(language).author_line("App", "0.1.0", "Author", "author.site");

            assert!(line.starts_with("App 0.1.0 · "), "{line}");
            assert!(line.ends_with("Author · author.site"), "{line}");
        }
    }

    #[test]
    fn an_age_is_rounded_down_to_its_largest_unit() {
        let english = Translator::new(Language::English);
        let age = |seconds| english.age(Duration::from_secs(seconds));

        assert_eq!(age(59), "just now");
        assert_eq!(age(12 * 60 + 59), "12min ago");
        assert_eq!(age(3 * 3600 + 59 * 60), "3h ago");
        assert_eq!(age(2 * 86_400 + 3600), "2d ago");
        // Glued to the unit in French too.
        let french = Translator::new(Language::French);
        assert_eq!(french.age(Duration::from_secs(59)), "à l'instant");
        assert_eq!(french.age(Duration::from_mins(12)), "il y a 12min");
    }
}
