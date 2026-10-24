//! The languages of the game client and of the interface.

use unic_langid::LanguageIdentifier;

/// A language of the game client, also a language of the interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Language {
    #[default]
    English,
    French,
}

impl Language {
    pub const ALL: [Self; 2] = [Self::English, Self::French];

    /// Its ISO 639-1 code, e.g. `fr`.
    fn code(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::French => "fr",
        }
    }

    /// Its BCP 47 identifier, e.g. `fr`.
    #[must_use]
    pub fn identifier(self) -> LanguageIdentifier {
        self.code().parse().expect("the language codes are valid")
    }

    /// The language of a locale of the system, e.g. `fr-FR` or `fr_CA.UTF-8`; `None` for a
    /// language the interface doesn't have.
    #[must_use]
    pub fn from_locale(locale: &str) -> Option<Self> {
        let code = locale
            .split(['-', '_', '.', '@'])
            .next()
            .unwrap_or_default();
        Self::ALL
            .into_iter()
            .find(|language| language.code().eq_ignore_ascii_case(code))
    }

    /// The first of the preferred locales, in their order, whose language the interface has;
    /// English when there is none.
    #[must_use]
    pub fn from_locales<S: AsRef<str>>(locales: impl IntoIterator<Item = S>) -> Self {
        locales
            .into_iter()
            .find_map(|locale| Self::from_locale(locale.as_ref()))
            .unwrap_or_default()
    }

    /// The language of the interface from the preferred locales of the system.
    #[must_use]
    pub fn of_system() -> Self {
        let locales: Vec<String> = sys_locale::get_locales().collect();
        let language = Self::from_locales(&locales);
        log::info!("System locales: {locales:?}, interface language: {language:?}");
        language
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_language_of_a_locale() {
        for (locale, language) in [
            ("fr-FR", Some(Language::French)),
            ("fr_CA.UTF-8", Some(Language::French)),
            ("FR", Some(Language::French)),
            ("fr", Some(Language::French)),
            ("en-US", Some(Language::English)),
            ("en_GB.UTF-8@euro", Some(Language::English)),
            ("en_GB@euro", Some(Language::English)),
            ("de-DE", None),
            ("frr-DE", None),
            ("C", None),
            ("C.UTF-8", None),
            ("POSIX", None),
            ("", None),
        ] {
            assert_eq!(Language::from_locale(locale), language, "{locale}");
        }
    }

    #[test]
    fn takes_the_first_preferred_locale_that_the_interface_has() {
        assert_eq!(
            Language::from_locales(["de-DE", "fr-FR", "en-US"]),
            Language::French
        );
        assert_eq!(
            Language::from_locales(["en-US", "fr-FR"]),
            Language::English
        );
        assert_eq!(
            Language::from_locales(["de-DE", "es-ES"]),
            Language::English
        );
        assert_eq!(
            Language::from_locales(Vec::<String>::new()),
            Language::English
        );
    }

    #[test]
    fn every_language_has_an_identifier() {
        for language in Language::ALL {
            assert_eq!(language.identifier().language.as_str(), language.code());
        }
    }
}
