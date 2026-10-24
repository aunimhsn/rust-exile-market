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

    /// Its BCP 47 identifier, e.g. `fr`.
    #[must_use]
    pub fn identifier(self) -> LanguageIdentifier {
        let tag = match self {
            Self::English => "en",
            Self::French => "fr",
        };
        tag.parse().expect("the language tags are valid")
    }
}
