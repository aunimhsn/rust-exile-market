//! Interface translations: the Fluent messages of `locales/`, embedded in the executable.
//!
//! The code never names a message by a string: the messages without arguments are the
//! variants of [`Message`], and each message with arguments has its own method of
//! [`Translator`], the only place that knows the names of its arguments.

use std::time::Duration;

use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};

use crate::filters::Property;
use crate::language::Language;
use crate::prices::Direction;
use crate::trade::pseudo::Pseudo;

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
    ItemWithoutDetails,
    ItemWithoutDetailsHint,
    SettingsTitle,
    SettingsLanguage,
    LanguageSystem,
    LanguageEnglish,
    LanguageFrench,
    SettingsAlwaysOnTop,
    Disclaimer,
    CheckOnTrade,
    TagCorrupted,
    TagTwiceCorrupted,
    TagSanctified,
    TagFractured,
    TagMirrored,
    TagUnidentified,
    PriceLabel,
    EstimatedPriceLabel,
    PossibleUniques,
    IdentifyToPrice,
    NoOffers,
    SearchToEstimate,
    NoMarketPriceRare,
    NoMarketPriceMagic,
    NoMarketPriceNormal,
    FrenchNamesDownloading,
    PricesDownloadingNoLeague,
    HideStats,
    GroupImplicit,
    GroupPrefixes,
    GroupSuffixes,
    GroupExplicit,
    DpsPhysical,
    DpsElemental,
    DpsChaos,
    DefenceArmour,
    DefenceEvasion,
    DefenceEnergyShield,
    DefenceBlock,
    GroupEnchant,
    GroupRunes,
    GroupUnique,
    GroupItem,
    GroupWeapon,
    GroupPseudo,
    ShowFilters,
    HideFilters,
    FilterRange,
    FilterLowerBetter,
    FilterUnsearchable,
    FilterUnsearchableHint,
    CorruptedOffersOnly,
    OffersLabel,
    OffersSearchHint,
    OffersSearchTooltip,
    OffersSearching,
    OffersColumnPrice,
    OffersColumnPerUnit,
    OffersColumnStock,
    OffersColumnListed,
    OffersAgainTooltip,
    OffersBusyTooltip,
    OffersNone,
    OffersNoneHint,
    OffersUnreachable,
    OffersIdentify,
    OffersMoreTooltip,
    OffersSuspicious,
    SellerOnline,
    SellerAway,
    SellerOffline,
    CopyWhisper,
    WhisperCopied,
}

impl Message {
    /// Every message, for the tests.
    #[cfg(test)]
    const ALL: [Self; 90] = [
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
        Self::ItemWithoutDetails,
        Self::ItemWithoutDetailsHint,
        Self::SettingsTitle,
        Self::SettingsLanguage,
        Self::LanguageSystem,
        Self::LanguageEnglish,
        Self::LanguageFrench,
        Self::SettingsAlwaysOnTop,
        Self::Disclaimer,
        Self::CheckOnTrade,
        Self::TagCorrupted,
        Self::TagTwiceCorrupted,
        Self::TagSanctified,
        Self::TagFractured,
        Self::TagMirrored,
        Self::TagUnidentified,
        Self::PriceLabel,
        Self::EstimatedPriceLabel,
        Self::PossibleUniques,
        Self::IdentifyToPrice,
        Self::NoOffers,
        Self::SearchToEstimate,
        Self::NoMarketPriceRare,
        Self::NoMarketPriceMagic,
        Self::NoMarketPriceNormal,
        Self::FrenchNamesDownloading,
        Self::PricesDownloadingNoLeague,
        Self::HideStats,
        Self::GroupImplicit,
        Self::GroupPrefixes,
        Self::GroupSuffixes,
        Self::GroupExplicit,
        Self::DpsPhysical,
        Self::DpsElemental,
        Self::DpsChaos,
        Self::DefenceArmour,
        Self::DefenceEvasion,
        Self::DefenceEnergyShield,
        Self::DefenceBlock,
        Self::GroupEnchant,
        Self::GroupRunes,
        Self::GroupUnique,
        Self::GroupItem,
        Self::GroupWeapon,
        Self::GroupPseudo,
        Self::ShowFilters,
        Self::HideFilters,
        Self::FilterRange,
        Self::FilterLowerBetter,
        Self::FilterUnsearchable,
        Self::FilterUnsearchableHint,
        Self::CorruptedOffersOnly,
        Self::OffersLabel,
        Self::OffersSearchHint,
        Self::OffersSearchTooltip,
        Self::OffersSearching,
        Self::OffersColumnPrice,
        Self::OffersColumnPerUnit,
        Self::OffersColumnStock,
        Self::OffersColumnListed,
        Self::OffersAgainTooltip,
        Self::OffersBusyTooltip,
        Self::OffersNone,
        Self::OffersNoneHint,
        Self::OffersUnreachable,
        Self::OffersIdentify,
        Self::OffersMoreTooltip,
        Self::OffersSuspicious,
        Self::SellerOnline,
        Self::SellerAway,
        Self::SellerOffline,
        Self::CopyWhisper,
        Self::WhisperCopied,
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
            Self::ItemWithoutDetails => "item-without-details",
            Self::ItemWithoutDetailsHint => "item-without-details-hint",
            Self::SettingsTitle => "settings-title",
            Self::SettingsLanguage => "settings-language",
            Self::LanguageSystem => "language-system",
            Self::LanguageEnglish => "language-english",
            Self::LanguageFrench => "language-french",
            Self::SettingsAlwaysOnTop => "settings-always-on-top",
            Self::Disclaimer => "disclaimer",
            Self::CheckOnTrade => "check-on-trade",
            Self::TagCorrupted => "tag-corrupted",
            Self::TagTwiceCorrupted => "tag-twice-corrupted",
            Self::TagSanctified => "tag-sanctified",
            Self::TagFractured => "tag-fractured",
            Self::TagMirrored => "tag-mirrored",
            Self::TagUnidentified => "tag-unidentified",
            Self::PriceLabel => "price-label",
            Self::EstimatedPriceLabel => "estimated-price-label",
            Self::PossibleUniques => "possible-uniques",
            Self::IdentifyToPrice => "identify-to-price",
            Self::NoOffers => "no-offers",
            Self::SearchToEstimate => "search-to-estimate",
            Self::NoMarketPriceRare => "no-market-price-rare",
            Self::NoMarketPriceMagic => "no-market-price-magic",
            Self::NoMarketPriceNormal => "no-market-price-normal",
            Self::FrenchNamesDownloading => "french-names-downloading",
            Self::PricesDownloadingNoLeague => "prices-downloading-no-league",
            Self::HideStats => "hide-stats",
            Self::GroupImplicit => "group-implicit",
            Self::GroupPrefixes => "group-prefixes",
            Self::GroupSuffixes => "group-suffixes",
            Self::GroupExplicit => "group-explicit",
            Self::DpsPhysical => "dps-physical",
            Self::DpsElemental => "dps-elemental",
            Self::DpsChaos => "dps-chaos",
            Self::DefenceArmour => "defence-armour",
            Self::DefenceEvasion => "defence-evasion",
            Self::DefenceEnergyShield => "defence-energy-shield",
            Self::DefenceBlock => "defence-block",
            Self::GroupEnchant => "group-enchant",
            Self::GroupRunes => "group-runes",
            Self::GroupUnique => "group-unique",
            Self::GroupItem => "group-item",
            Self::GroupWeapon => "group-weapon",
            Self::GroupPseudo => "group-pseudo",
            Self::ShowFilters => "show-filters",
            Self::HideFilters => "hide-filters",
            Self::FilterRange => "filter-range",
            Self::FilterLowerBetter => "filter-lower-better",
            Self::FilterUnsearchable => "filter-unsearchable",
            Self::FilterUnsearchableHint => "filter-unsearchable-hint",
            Self::CorruptedOffersOnly => "corrupted-offers-only",
            Self::OffersLabel => "offers-label",
            Self::OffersSearchHint => "offers-search-hint",
            Self::OffersSearchTooltip => "offers-search-tooltip",
            Self::OffersSearching => "offers-searching",
            Self::OffersColumnPrice => "offers-column-price",
            Self::OffersColumnPerUnit => "offers-column-per-unit",
            Self::OffersColumnStock => "offers-column-stock",
            Self::OffersColumnListed => "offers-column-listed",
            Self::OffersAgainTooltip => "offers-again-tooltip",
            Self::OffersBusyTooltip => "offers-busy-tooltip",
            Self::OffersNone => "offers-none",
            Self::OffersNoneHint => "offers-none-hint",
            Self::OffersUnreachable => "offers-unreachable",
            Self::OffersIdentify => "offers-identify",
            Self::OffersMoreTooltip => "offers-more-tooltip",
            Self::OffersSuspicious => "offers-suspicious",
            Self::SellerOnline => "seller-online",
            Self::SellerAway => "seller-away",
            Self::SellerOffline => "seller-offline",
            Self::CopyWhisper => "copy-whisper",
            Self::WhisperCopied => "whisper-copied",
        }
    }
}

/// The tags of the item block with a number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumberTag {
    Quality,
    Sockets,
    ItemLevel,
}

/// The parts of the requirements line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Requirement {
    Level,
    Strength,
    Dexterity,
    Intelligence,
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
const WAIT_SECONDS: &str = "wait-seconds";
const WAIT_MINUTES: &str = "wait-minutes";
const DOWNLOAD_TITLE: &str = "download-title";
const TAG_QUALITY: &str = "tag-quality";
const TAG_SOCKETS: &str = "tag-sockets";
const TAG_ITEM_LEVEL: &str = "tag-item-level";
const TAG_STACK: &str = "tag-stack";
const GRANTED_SKILL: &str = "granted-skill";
const REQUIRES: &str = "requires";
const REQUIREMENT_LEVEL: &str = "requirement-level";
const REQUIREMENT_STRENGTH: &str = "requirement-strength";
const REQUIREMENT_DEXTERITY: &str = "requirement-dexterity";
const REQUIREMENT_INTELLIGENCE: &str = "requirement-intelligence";
const OFFERS: &str = "offers";
const STACK_OF: &str = "stack-of";
const UNIT_PRICE: &str = "unit-price";
const TREND_RISE: &str = "trend-rise";
const TREND_FALL: &str = "trend-fall";
const TREND_STABLE: &str = "trend-stable";
const UNCERTAIN: &str = "uncertain";
const SIMILAR: &str = "similar";
const ESTIMATE_UNCERTAIN: &str = "estimate-uncertain";
const ESTIMATE_MEDIAN: &str = "estimate-median";
const ESTIMATE_MEDIAN_CHEAPEST: &str = "estimate-median-cheapest";
const PRICES_DOWNLOADING: &str = "prices-downloading";
const NO_PRICE_FOUND: &str = "no-price-found";
const SHOW_STATS: &str = "show-stats";
const AFFIX_FILL: &str = "affix-fill";
const AFFIX_FILL_OPEN: &str = "affix-fill-open";
const ACTIVE_FILTERS: &str = "active-filters";
const FILTER_ROLLS: &str = "filter-rolls";
const FILTER_BASE_TYPE: &str = "filter-base-type";
const FILTER_ITEM_CLASS: &str = "filter-item-class";
const FILTER_ITEM_LEVEL: &str = "filter-item-level";
const FILTER_SOCKETS: &str = "filter-sockets";
const FILTER_QUALITY: &str = "filter-quality";
const FILTER_REQUIRED_LEVEL: &str = "filter-required-level";
const FILTER_PHYSICAL_DPS: &str = "filter-physical-dps";
const FILTER_ELEMENTAL_DPS: &str = "filter-elemental-dps";
const PSEUDO_ELEMENTAL_RESISTANCE: &str = "pseudo-elemental-resistance";
const PSEUDO_CHAOS_RESISTANCE: &str = "pseudo-chaos-resistance";
const PSEUDO_LIFE: &str = "pseudo-life";
const PSEUDO_MANA: &str = "pseudo-mana";
const PSEUDO_STRENGTH: &str = "pseudo-strength";
const PSEUDO_DEXTERITY: &str = "pseudo-dexterity";
const PSEUDO_INTELLIGENCE: &str = "pseudo-intelligence";
const PSEUDO_ALL_ATTRIBUTES: &str = "pseudo-all-attributes";
const STATUS_TRADE: &str = "status-trade";
const STATUS_TRADE_WAIT: &str = "status-trade-wait";
const OFFERS_SEARCHED: &str = "offers-searched";
const OFFERS_COUNT: &str = "offers-count";
const OFFERS_NEXT_SEARCH_TOOLTIP: &str = "offers-next-search-tooltip";
const OFFERS_LIMIT: &str = "offers-limit";
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

    /// A wait, rounded up so that it never ends too soon: `8s` under a minute, `4min` from it.
    #[must_use]
    pub fn wait(&self, wait: Duration) -> String {
        let seconds = wait.as_secs() + u64::from(wait.subsec_nanos() > 0);
        let (id, name, count) = if seconds < 60 {
            (WAIT_SECONDS, "seconds", seconds)
        } else {
            (WAIT_MINUTES, "minutes", seconds.div_ceil(60))
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

    /// The pill that unfolds the stats of the item: "Show 6 stats".
    #[must_use]
    pub fn show_stats(&self, count: u32) -> String {
        self.format_number(SHOW_STATS, "count", count)
    }

    /// The number of active filters, beside the pill of the stats: "3 filters".
    #[must_use]
    pub fn active_filters(&self, count: u32) -> String {
        self.format_number(ACTIVE_FILTERS, "count", count)
    }

    /// The fill of the prefixes or suffixes of an item: "2 of 3 · 1 open".
    #[must_use]
    pub fn affix_fill(&self, filled: u32, max: u32) -> String {
        let mut args = FluentArgs::new();
        args.set("filled", filled);
        args.set("max", max);
        let open = max.saturating_sub(filled);
        if open == 0 {
            return self.format(AFFIX_FILL, Some(&args));
        }
        args.set("open", open);
        self.format(AFFIX_FILL_OPEN, Some(&args))
    }

    /// The rolls a range field accepts, in its tooltip: "Rolls 40 to 60"; `low` and `high`
    /// written by [`crate::format::number`].
    #[must_use]
    pub fn filter_rolls(&self, low: &str, high: &str) -> String {
        let mut args = FluentArgs::new();
        args.set("low", low);
        args.set("high", high);
        self.format(FILTER_ROLLS, Some(&args))
    }

    /// The filter of the base type of an item: "Base: Gemini Bow".
    #[must_use]
    pub fn filter_base_type(&self, base_type: &str) -> String {
        let mut args = FluentArgs::new();
        args.set("base", base_type);
        self.format(FILTER_BASE_TYPE, Some(&args))
    }

    /// Under the filter of the base type unchecked, the item class searched in its place:
    /// "Class searched: Bows".
    #[must_use]
    pub fn filter_item_class(&self, class: &str) -> String {
        let mut args = FluentArgs::new();
        args.set("class", class);
        self.format(FILTER_ITEM_CLASS, Some(&args))
    }

    /// The filter of a property of an item, with its `value` written by
    /// [`crate::format::number`]: "Item level: 81", "Quality: 24%", "Physical DPS: 245".
    #[must_use]
    pub fn filter_property(&self, property: Property, value: &str) -> String {
        let id = match property {
            Property::ItemLevel => FILTER_ITEM_LEVEL,
            Property::Sockets => FILTER_SOCKETS,
            Property::Quality => FILTER_QUALITY,
            Property::RequiredLevel => FILTER_REQUIRED_LEVEL,
            Property::PhysicalDps => FILTER_PHYSICAL_DPS,
            Property::ElementalDps => FILTER_ELEMENTAL_DPS,
        };
        let mut args = FluentArgs::new();
        args.set("value", value);
        self.format(id, Some(&args))
    }

    /// The filter of a pseudo total, with its `value` written by [`crate::format::number`]:
    /// "+83% total Elemental Resistance".
    #[must_use]
    pub fn filter_pseudo(&self, pseudo: Pseudo, value: &str) -> String {
        let id = match pseudo {
            Pseudo::ElementalResistance => PSEUDO_ELEMENTAL_RESISTANCE,
            Pseudo::ChaosResistance => PSEUDO_CHAOS_RESISTANCE,
            Pseudo::Life => PSEUDO_LIFE,
            Pseudo::Mana => PSEUDO_MANA,
            Pseudo::Strength => PSEUDO_STRENGTH,
            Pseudo::Dexterity => PSEUDO_DEXTERITY,
            Pseudo::Intelligence => PSEUDO_INTELLIGENCE,
            Pseudo::AllAttributes => PSEUDO_ALL_ATTRIBUTES,
        };
        let mut args = FluentArgs::new();
        args.set("value", value);
        self.format(id, Some(&args))
    }

    /// The number of offers behind the price of a unique: "23 offers".
    #[must_use]
    pub fn offers(&self, count: u32) -> String {
        self.format_number(OFFERS, "count", count)
    }

    /// The size of a copied stack, on the right of the price title: "Stack of 7".
    #[must_use]
    pub fn stack_of(&self, size: u32) -> String {
        self.format_number(STACK_OF, "size", size)
    }

    /// The unit price of a stack, from its amounts in the three currencies joined by the
    /// caller.
    #[must_use]
    pub fn unit_price(&self, prices: &str) -> String {
        let mut args = FluentArgs::new();
        args.set("prices", prices);
        self.format(UNIT_PRICE, Some(&args))
    }

    /// The trend of a price: its change in percent, rounded, over `days` days.
    #[must_use]
    pub fn trend(&self, direction: Direction, percent: u32, days: u32) -> String {
        let id = match direction {
            Direction::Rise => TREND_RISE,
            Direction::Fall => TREND_FALL,
            Direction::Stable => TREND_STABLE,
        };
        let mut args = FluentArgs::new();
        args.set("percent", percent);
        args.set("days", days);
        self.format(id, Some(&args))
    }

    /// The warning of a price with few offers.
    #[must_use]
    pub fn uncertain(&self, count: u32) -> String {
        self.format_number(UNCERTAIN, "count", count)
    }

    /// The similar offers found, on the right of the title of an estimated price: "18 similar".
    #[must_use]
    pub fn similar(&self, total: u64) -> String {
        self.format_number(SIMILAR, "count", total)
    }

    /// The warning of an estimate from few offers.
    #[must_use]
    pub fn estimate_uncertain(&self, counted: usize) -> String {
        self.format_number(ESTIMATE_UNCERTAIN, "count", counted)
    }

    /// How an estimate from all the `counted` offers is made.
    #[must_use]
    pub fn estimate_median(&self, counted: usize) -> String {
        self.format_number(ESTIMATE_MEDIAN, "count", counted)
    }

    /// How an estimate from the `counted` cheapest offers is made.
    #[must_use]
    pub fn estimate_median_cheapest(&self, counted: usize) -> String {
        self.format_number(ESTIMATE_MEDIAN_CHEAPEST, "count", counted)
    }

    /// The price block while the prices of `league` are downloading.
    #[must_use]
    pub fn prices_downloading(&self, league: &str) -> String {
        self.format_league(PRICES_DOWNLOADING, league)
    }

    /// The price block of a fixed-name item missing from the prices of `league`.
    #[must_use]
    pub fn no_price_found(&self, league: &str) -> String {
        self.format_league(NO_PRICE_FOUND, league)
    }

    /// A tag of the item block with a number: the quality (`Q 20%`), the sockets, the item
    /// level.
    #[must_use]
    pub fn tag(&self, tag: NumberTag, value: u32) -> String {
        let id = match tag {
            NumberTag::Quality => TAG_QUALITY,
            NumberTag::Sockets => TAG_SOCKETS,
            NumberTag::ItemLevel => TAG_ITEM_LEVEL,
        };
        self.format_number(id, "value", value)
    }

    /// The stack tag of the item block: its size and its largest size.
    #[must_use]
    pub fn stack_tag(&self, size: u32, max: u32) -> String {
        let mut args = FluentArgs::new();
        args.set("size", size);
        args.set("max", max);
        self.format(TAG_STACK, Some(&args))
    }

    /// The summary of an item that grants skills, e.g. a staff, from the skills joined by
    /// the caller.
    #[must_use]
    pub fn granted_skill(&self, skill: &str) -> String {
        let mut args = FluentArgs::new();
        args.set("skill", skill);
        self.format(GRANTED_SKILL, Some(&args))
    }

    /// The texts of the requirements line before and after its parts, which the caller
    /// writes each in its color: `("Requires ", "")`.
    #[must_use]
    pub fn requires_around(&self) -> (String, String) {
        // A character no translation writes, in place of the parts.
        const PARTS: &str = "\u{1}";
        let mut args = FluentArgs::new();
        args.set("parts", PARTS);
        let line = self.format(REQUIRES, Some(&args));
        match line.split_once(PARTS) {
            Some((before, after)) => (before.to_owned(), after.to_owned()),
            None => (line, String::new()),
        }
    }

    /// A part of the requirements line: the level, or an attribute.
    #[must_use]
    pub fn requirement(&self, requirement: Requirement, value: u32) -> String {
        let id = match requirement {
            Requirement::Level => REQUIREMENT_LEVEL,
            Requirement::Strength => REQUIREMENT_STRENGTH,
            Requirement::Dexterity => REQUIREMENT_DEXTERITY,
            Requirement::Intelligence => REQUIREMENT_INTELLIGENCE,
        };
        self.format_number(id, "value", value)
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

    /// The state of the trade site in the status bar: the age of its last search.
    #[must_use]
    pub fn status_trade(&self, age: &str) -> String {
        self.format_age(STATUS_TRADE, age)
    }

    /// The state of the trade site in the status bar during a long wait before the next
    /// search, written by [`wait`].
    #[must_use]
    pub fn status_trade_wait(&self, wait: Duration) -> String {
        self.format_wait(STATUS_TRADE_WAIT, wait)
    }

    /// The age of the offers, under their title: "searched 2min ago".
    #[must_use]
    pub fn offers_searched(&self, age: &str) -> String {
        self.format_age(OFFERS_SEARCHED, age)
    }

    /// The offers shown, of those found: "10 of 13 online".
    #[must_use]
    pub fn offers_count(&self, shown: u64, total: u64) -> String {
        let mut args = FluentArgs::new();
        args.set("shown", shown);
        args.set("total", total);
        self.format(OFFERS_COUNT, Some(&args))
    }

    /// The tooltip of the countdown before the next search.
    #[must_use]
    pub fn offers_next_search_tooltip(&self, wait: Duration) -> String {
        self.format_wait(OFFERS_NEXT_SEARCH_TOOLTIP, wait)
    }

    /// The line under the title of the offers during a long wait.
    #[must_use]
    pub fn offers_limit(&self, wait: Duration) -> String {
        self.format_wait(OFFERS_LIMIT, wait)
    }

    fn format_wait(&self, id: &str, wait: Duration) -> String {
        let mut args = FluentArgs::new();
        args.set("wait", self.wait(wait));
        self.format(id, Some(&args))
    }

    fn format_league(&self, id: &str, league: &str) -> String {
        let mut args = FluentArgs::new();
        args.set("league", league);
        self.format(id, Some(&args))
    }

    fn format_number(
        &self,
        id: &str,
        name: &'static str,
        value: impl Into<FluentValue<'static>>,
    ) -> String {
        let mut args = FluentArgs::new();
        args.set(name, value);
        self.format(id, Some(&args))
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
                WAIT_SECONDS,
                WAIT_MINUTES,
                AGE_HOURS,
                AGE_DAYS,
                DOWNLOAD_TITLE,
                TAG_QUALITY,
                TAG_SOCKETS,
                TAG_ITEM_LEVEL,
                TAG_STACK,
                GRANTED_SKILL,
                REQUIRES,
                REQUIREMENT_LEVEL,
                REQUIREMENT_STRENGTH,
                REQUIREMENT_DEXTERITY,
                REQUIREMENT_INTELLIGENCE,
                SHOW_STATS,
                AFFIX_FILL,
                AFFIX_FILL_OPEN,
                ACTIVE_FILTERS,
                FILTER_ROLLS,
                FILTER_BASE_TYPE,
                FILTER_ITEM_CLASS,
                FILTER_ITEM_LEVEL,
                FILTER_SOCKETS,
                FILTER_QUALITY,
                FILTER_REQUIRED_LEVEL,
                FILTER_PHYSICAL_DPS,
                FILTER_ELEMENTAL_DPS,
                PSEUDO_ELEMENTAL_RESISTANCE,
                PSEUDO_CHAOS_RESISTANCE,
                PSEUDO_LIFE,
                PSEUDO_MANA,
                PSEUDO_STRENGTH,
                PSEUDO_DEXTERITY,
                PSEUDO_INTELLIGENCE,
                PSEUDO_ALL_ATTRIBUTES,
                OFFERS,
                STACK_OF,
                UNIT_PRICE,
                TREND_RISE,
                TREND_FALL,
                TREND_STABLE,
                UNCERTAIN,
                SIMILAR,
                ESTIMATE_UNCERTAIN,
                ESTIMATE_MEDIAN,
                ESTIMATE_MEDIAN_CHEAPEST,
                PRICES_DOWNLOADING,
                NO_PRICE_FOUND,
                STATUS_TRADE,
                STATUS_TRADE_WAIT,
                OFFERS_SEARCHED,
                OFFERS_COUNT,
                OFFERS_NEXT_SEARCH_TOOLTIP,
                OFFERS_LIMIT,
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
            for tag in [NumberTag::Quality, NumberTag::Sockets, NumberTag::ItemLevel] {
                for value in [0, 1, 2, 20] {
                    let text = translator.tag(tag, value);
                    assert!(text.contains(&value.to_string()), "{text} in {language:?}");
                }
            }
            assert!(translator.stack_tag(7, 20).contains("7/20"), "{language:?}");
            assert!(translator.granted_skill("Skill").contains("Skill"));
            for count in [0, 1, 4, 23] {
                assert!(translator.offers(count).contains(&count.to_string()));
                assert_ne!(translator.uncertain(count), "");
            }
            assert!(translator.stack_of(7).contains('7'));
            for count in [1, 3, 5, 18] {
                assert!(translator.similar(count).contains(&count.to_string()));
            }
            for count in [1, 4] {
                assert_ne!(translator.estimate_uncertain(count), "");
                assert_ne!(translator.estimate_median(count), "");
            }
            assert!(translator.estimate_median_cheapest(5).contains('5'));
            assert!(translator.show_stats(1).contains('1'));
            assert!(translator.show_stats(6).contains('6'));
            assert!(translator.affix_fill(3, 3).contains('3'));
            assert!(translator.affix_fill(2, 3).contains('1'));
            assert!(translator.active_filters(1).contains('1'));
            assert!(translator.active_filters(3).contains('3'));
            assert!(translator.filter_rolls("-7.5", "60").contains("-7.5"));
            assert!(translator.filter_rolls("-7.5", "60").contains("60"));
            assert!(translator.unit_price("prices").contains("prices"));
            for direction in [Direction::Rise, Direction::Fall, Direction::Stable] {
                assert!(translator.trend(direction, 4, 7).contains('7'));
            }
            assert!(translator.prices_downloading("League").contains("League"));
            assert!(translator.no_price_found("League").contains("League"));
            assert_ne!(translator.requires_around().0, "", "{language:?}");
            for requirement in [
                Requirement::Level,
                Requirement::Strength,
                Requirement::Dexterity,
                Requirement::Intelligence,
            ] {
                assert!(translator.requirement(requirement, 65).contains("65"));
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
    fn the_open_affixes_agree_in_number_in_french() {
        let french = Translator::new(Language::French);

        assert_eq!(french.affix_fill(2, 3), "2 sur 3 · 1 libre");
        assert_eq!(french.affix_fill(1, 3), "1 sur 3 · 2 libres");
        assert_eq!(french.affix_fill(3, 3), "3 sur 3");
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
    fn the_fill_of_the_affixes_tells_the_open_ones() {
        let english = Translator::new(Language::English);

        assert_eq!(english.affix_fill(3, 3), "3 of 3");
        assert_eq!(english.affix_fill(2, 3), "2 of 3 · 1 open");
    }

    #[test]
    fn no_offer_has_its_own_warning_in_every_language() {
        // In French, the plural category "one" covers 0 too.
        let french = Translator::new(Language::French);
        assert!(french.uncertain(0).starts_with("Aucune offre"));
        assert!(french.uncertain(1).starts_with("Une seule offre"));
        assert!(
            Translator::new(Language::English)
                .uncertain(0)
                .starts_with("No offer")
        );
    }

    #[test]
    fn an_estimate_from_one_offer_is_its_price() {
        for language in Language::ALL {
            let translator = Translator::new(language);
            assert!(!translator.estimate_median(1).contains('1'), "{language:?}");
            assert!(translator.estimate_median(3).contains('3'));
        }
        assert_eq!(
            Translator::new(Language::English).estimate_uncertain(3),
            "Only 3 similar offers: the estimate is uncertain"
        );
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

    #[test]
    fn a_wait_is_rounded_up_so_that_it_never_ends_too_soon() {
        let english = Translator::new(Language::English);
        assert_eq!(english.wait(Duration::from_millis(8_200)), "9s");
        assert_eq!(english.wait(Duration::from_secs(8)), "8s");
        assert_eq!(english.wait(Duration::from_secs(59)), "59s");
        assert_eq!(english.wait(Duration::from_secs(60)), "1min");
        assert_eq!(english.wait(Duration::from_secs(181)), "4min");
        let french = Translator::new(Language::French);
        assert_eq!(
            french.offers_limit(Duration::from_secs(240)),
            "Limite du site d'échange\u{a0}: recherche dans 4min."
        );
    }
}
