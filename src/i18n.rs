//! Interface translations: the Fluent messages of `locales/`, embedded in the executable.
//!
//! The code never names a message by a string: the messages without arguments are the
//! variants of [`Message`], and each message with arguments has its own method of
//! [`Translator`], the only place that knows the names of its arguments.

use fluent_bundle::{FluentArgs, FluentBundle, FluentResource};
use unic_langid::LanguageIdentifier;

/// A language of the interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language {
    #[default]
    English,
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "chosen in the settings (step 5) or from the copied items (step 4)"
        )
    )]
    French,
}

impl Language {
    fn identifier(self) -> LanguageIdentifier {
        let tag = match self {
            Self::English => "en",
            Self::French => "fr",
        };
        tag.parse().expect("the language tags are valid")
    }

    fn messages(self) -> &'static str {
        match self {
            Self::English => include_str!("../locales/en.ftl"),
            Self::French => include_str!("../locales/fr.ftl"),
        }
    }
}

/// A message of the interface without arguments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Message {
    LeagueModeSoftcore,
    LeagueModeHardcore,
    LeagueTooltip,
    SettingsTooltip,
    StatusPricesWaiting,
    StatusTradeIdle,
    EmptyCopy,
    EmptyPrice,
    ClipboardUnreadable,
}

impl Message {
    /// Every message, for the tests.
    #[cfg(test)]
    const ALL: [Self; 9] = [
        Self::LeagueModeSoftcore,
        Self::LeagueModeHardcore,
        Self::LeagueTooltip,
        Self::SettingsTooltip,
        Self::StatusPricesWaiting,
        Self::StatusTradeIdle,
        Self::EmptyCopy,
        Self::EmptyPrice,
        Self::ClipboardUnreadable,
    ];

    /// Its identifier in the `.ftl` files.
    fn id(self) -> &'static str {
        match self {
            Self::LeagueModeSoftcore => "league-mode-softcore",
            Self::LeagueModeHardcore => "league-mode-hardcore",
            Self::LeagueTooltip => "league-tooltip",
            Self::SettingsTooltip => "settings-tooltip",
            Self::StatusPricesWaiting => "status-prices-waiting",
            Self::StatusTradeIdle => "status-trade-idle",
            Self::EmptyCopy => "empty-copy",
            Self::EmptyPrice => "empty-price",
            Self::ClipboardUnreadable => "clipboard-unreadable",
        }
    }
}

/// The identifiers of the messages with arguments, each read by its own method.
const STATUS_BAR: &str = "status-bar";

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
        let resource = FluentResource::try_new(language.messages().to_owned())
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

    const LANGUAGES: [Language; 2] = [Language::English, Language::French];

    fn file_ids(language: Language) -> BTreeSet<String> {
        let resource =
            FluentResource::try_new(language.messages().to_owned()).expect("the messages parse");
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
            .chain([STATUS_BAR])
            .map(str::to_owned)
            .collect();
        for language in LANGUAGES {
            assert_eq!(file_ids(language), code_ids, "messages of {language:?}");
        }
    }

    #[test]
    fn every_message_formats_without_errors() {
        // The errors fail the `debug_assert` of `format`.
        for language in LANGUAGES {
            let translator = Translator::new(language);
            for message in Message::ALL {
                assert_ne!(translator.text(message), "", "{message:?} in {language:?}");
            }
            assert_ne!(translator.status_bar("A", "B"), "", "{language:?}");
        }
    }

    #[test]
    fn arguments_are_inserted_without_isolation_marks() {
        assert_eq!(
            Translator::new(Language::English).status_bar("A", "B"),
            "A · B"
        );
    }
}
