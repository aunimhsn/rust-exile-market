use super::*;
use crate::language::PerLanguage;
use crate::trade::limits::LimitHeaders;
use crate::trade::transport::Answer;
use crate::trade::transport::tests::{FakeSite, retry_after};

/// The shape of `/data/items` on www.pathofexile.com on 2026-10-25, a few items of it.
const ENGLISH_ITEMS: &str = r#"{"result": [
    {"id": "accessory", "label": "Accessories", "entries": [
        {"type": "Ruby Ring"},
        {"type": "Gold Ring", "text": "Andvarius Gold Ring", "name": "Andvarius", "flags": {"unique": true}}
    ]},
    {"id": "weapon", "label": "Weapons", "entries": [
        {"type": "Temple Maul"},
        {"type": "Rippled Greatsword"},
        {"type": "Makeshift Crossbow", "text": "Mist Whisper Makeshift Crossbow", "name": "Mist Whisper", "flags": {"unique": true}},
        {"type": "Shrine Sceptre", "text": "Guiding Palm Shrine Sceptre (Legacy)", "name": "Guiding Palm", "disc": "legacy", "flags": {"unique": true}}
    ]},
    {"id": "jewel", "label": "Jewels", "entries": [
        {"type": "Emerald", "text": "Grand Spectrum Emerald", "name": "Grand Spectrum", "flags": {"unique": true}},
        {"type": "Emerald", "text": "Prism Guardian Emerald", "name": "Prism Guardian", "flags": {"unique": true}}
    ]},
    {"id": "currency", "label": "Currency", "entries": [{"type": "Preserved Rib"}]}
]}"#;

/// The same items on fr.pathofexile.com: a base type is missing, and so is a unique.
const FRENCH_ITEMS: &str = r#"{"result": [
    {"id": "accessory", "label": "Accessoires", "entries": [
        {"type": "Bague de rubis"},
        {"type": "Bague en or", "text": "Andvarius Bague en or", "name": "Andvarius", "flags": {"unique": true}}
    ]},
    {"id": "weapon", "label": "Armes", "entries": [
        {"type": "Massue du temple"},
        {"type": "Arbalète de fortune", "text": "Murmure des brumes Arbalète de fortune", "name": "Murmure des brumes", "flags": {"unique": true}},
        {"type": "Sceptre sanctuaire", "text": "Paume directrice Sceptre sanctuaire (Legacy)", "name": "Paume directrice", "disc": "legacy", "flags": {"unique": true}}
    ]},
    {"id": "jewel", "label": "Joyaux", "entries": [
        {"type": "Émeraude", "text": "Grand spectre Émeraude", "name": "Grand spectre", "flags": {"unique": true}}
    ]},
    {"id": "currency", "label": "Objets monétaires", "entries": [{"type": "Côte préservée"}]}
]}"#;

/// The shape of `/data/static` on 2026-10-25.
const ENGLISH_STATIC: &str = r#"{"result": [
    {"id": "Currency", "label": "Currency", "entries": [
        {"id": "divine", "text": "Divine Orb", "image": "/gen/image/divine.png"},
        {"id": "sep", "image": "/gen/image/separator.png"}
    ]}
]}"#;

const FRENCH_STATIC: &str = r#"{"result": [
    {"id": "Currency", "label": "Objets monétaires", "entries": [
        {"id": "divine", "text": "Orbe divin", "image": "/gen/image/divine.png"}
    ]}
]}"#;

/// The shape of `/data/stats` on 2026-10-26, a few stats of it.
const ENGLISH_STATS: &str = r##"{"result": [
    {"id": "pseudo", "label": "Pseudo", "entries": [
        {"id": "pseudo.pseudo_total_life", "text": "+# total maximum Life", "type": "pseudo"}
    ]},
    {"id": "explicit", "label": "Explicit", "entries": [
        {"id": "explicit.stat_3299347043", "text": "# to maximum Life", "type": "explicit"},
        {"id": "explicit.stat_2954116742|46224", "text": "Allocates Arcane Alchemy", "type": "explicit"}
    ]},
    {"id": "rune", "label": "Augment", "entries": [
        {"id": "rune.stat_1992191903", "text": "# to Level of all Mark Skills", "type": "augment"}
    ]},
    {"id": "sanctum", "label": "Sanctum", "entries": [
        {"id": "sanctum.stat_2878762585", "text": "Resolve Mitigation chance is #%", "type": "sanctum"}
    ]}
]}"##;

const FRENCH_STATS: &str = r##"{"result": [
    {"id": "explicit", "label": "Explicite", "entries": [
        {"id": "explicit.stat_3299347043", "text": "# de Vie maximale", "type": "explicit"}
    ]}
]}"##;

fn data(language: Language, items: &str, exchange: &str, stats: &str) -> LanguageData {
    let items: ApiResult<ApiItemCategory> = serde_json::from_str(items).expect("items");
    let exchange: ApiResult<ApiStaticGroup> = serde_json::from_str(exchange).expect("static");
    let stats: ApiResult<ApiStatGroup> = serde_json::from_str(stats).expect("stats");
    LanguageData {
        language,
        items: items.result,
        exchange: exchange.result,
        stats: stats.result,
    }
}

fn english() -> LanguageData {
    data(
        Language::English,
        ENGLISH_ITEMS,
        ENGLISH_STATIC,
        ENGLISH_STATS,
    )
}

fn french() -> LanguageData {
    data(Language::French, FRENCH_ITEMS, FRENCH_STATIC, FRENCH_STATS)
}

fn names_of_both() -> ItemNames {
    item_names(SystemTime::UNIX_EPOCH, &english(), &french()).expect("every kind of names")
}

/// The uniques of a category, as `misalignment` takes them.
fn list(items: &[ApiItem]) -> Vec<&ApiItem> {
    items.iter().collect()
}

/// A unique of the item data.
fn item(base_type: &str, name: &str, disc: Option<&str>) -> ApiItem {
    ApiItem {
        base_type: base_type.to_owned(),
        name: Some(name.to_owned()),
        disc: disc.map(str::to_owned),
    }
}

#[test]
fn the_items_of_the_currency_exchange_are_found_by_their_name_in_each_language() {
    let names = names_of_both();

    assert_eq!(
        names.exchange_id(Language::English, "Divine Orb"),
        Some("divine")
    );
    assert_eq!(
        names.exchange_id(Language::French, "Orbe divin"),
        Some("divine")
    );
    assert_eq!(names.exchange_id(Language::English, ""), None);
}

#[test]
fn the_uniques_are_paired_in_their_order_without_the_older_versions() {
    let names = names_of_both();

    assert_eq!(
        names.english_unique(Language::French, "Murmure des brumes"),
        Some("Mist Whisper")
    );
    assert_eq!(
        names.english_unique(Language::French, "Andvarius"),
        Some("Andvarius")
    );
    assert_eq!(
        names.english_unique(Language::French, "Paume directrice"),
        None
    );
}

#[test]
fn a_category_whose_uniques_differ_in_number_is_left_out() {
    let names = names_of_both();

    assert_eq!(
        names.english_unique(Language::French, "Grand spectre"),
        None
    );
    assert_eq!(names.counts().uniques, 2);
}

#[test]
fn lists_of_the_same_length_that_dont_line_up_are_found() {
    let english = [
        item("Emerald", "Grand Spectrum", None),
        item("Ruby", "Lioneye's Fall", None),
        item("Emerald", "Prism Guardian", None),
    ];
    // Shifted by one: the emerald of English is paired with the ruby of French.
    let shifted = [
        item("Rubis", "Chute d'Œil-de-lion", None),
        item("Émeraude", "Grand spectre", None),
        item("Émeraude", "Gardien prismatique", None),
    ];
    let legacy = [
        item("Émeraude", "Grand spectre", Some("legacy")),
        item("Rubis", "Chute d'Œil-de-lion", None),
        item("Émeraude", "Gardien prismatique", None),
    ];
    let lined_up = [
        item("Émeraude", "Grand spectre", None),
        item("Rubis", "Chute d'Œil-de-lion", None),
        item("Émeraude", "Gardien prismatique", None),
    ];

    assert!(misalignment(&list(&english), &list(&shifted)).is_some());
    assert!(misalignment(&list(&english), &list(&legacy)).is_some());
    assert_eq!(misalignment(&list(&english), &list(&lined_up)), None);
}

#[test]
fn a_base_type_paired_with_two_others_is_found_either_way() {
    let english = [
        item("Emerald", "Grand Spectrum", None),
        item("Ruby", "Lioneye's Fall", None),
    ];
    // Each English base has one translation, but the French one has two originals.
    let french = [
        item("Émeraude", "Grand spectre", None),
        item("Émeraude", "Chute d'Œil-de-lion", None),
    ];

    assert!(misalignment(&list(&english), &list(&french)).is_some());
}

#[test]
fn the_uniques_of_a_category_missing_in_french_are_left_out() {
    let english: ApiResult<ApiItemCategory> = serde_json::from_str(ENGLISH_ITEMS).unwrap();
    let french: ApiResult<ApiItemCategory> = serde_json::from_str(FRENCH_ITEMS).unwrap();
    let without_weapons: Vec<ApiItemCategory> = french
        .result
        .into_iter()
        .filter(|category| category.id != "weapon")
        .collect();

    let pairs = unique_pairs(&english.result, &without_weapons);

    let names: Vec<&str> = pairs
        .iter()
        .map(|pair| pair.english.name.as_str())
        .collect();
    assert_eq!(names, ["Andvarius"]);
}

#[test]
fn a_refusal_waits_for_its_delay_and_makes_the_searches_wait() {
    for status in [429, 403] {
        let site = FakeSite::default();
        site.answer(status, retry_after("7200"), "");
        let refusal = Refusal::default();
        let mut names = TradeSite::with_transport(site.clone(), refusal.clone());
        let start = Instant::now();

        let error = names.item_names().expect_err("a refusal");

        let until = error.refused_until().expect("a wait");
        assert!(until >= start + Duration::from_hours(2), "{status}");
        assert!(until < start + Duration::from_hours(2) + Duration::from_mins(1));
        assert_eq!(refusal.until(start), Some(until), "{status}");
        // The five other requests didn't leave.
        assert_eq!(site.requests().len(), 1, "{status}");
    }
}

#[test]
fn nothing_is_sent_during_the_wait_of_a_refused_search() {
    let site = FakeSite::default();
    let refusal = Refusal::default();
    let until = Instant::now() + Duration::from_mins(30);
    refusal.extend(until);
    let mut names = TradeSite::with_transport(site.clone(), refusal);

    let error = names.item_names().expect_err("a wait");

    assert!(matches!(error, NamesError::Wait(wait) if wait == until));
    assert_eq!(error.refused_until(), None);
    assert_eq!(site.requests(), []);
}

/// A trade site that refuses a search of the other thread while it answers the names.
struct RefusedMeanwhile {
    site: FakeSite,
    refusal: Refusal,
    until: Instant,
}

impl Transport for RefusedMeanwhile {
    fn send(&mut self, request: &Request) -> Result<Answer, ureq::Error> {
        let answer = self.site.send(request);
        self.refusal.extend(self.until);
        answer
    }
}

#[test]
fn a_search_refused_between_two_requests_stops_the_next_ones() {
    let site = FakeSite::default();
    for _ in 0..6 {
        site.answer(200, LimitHeaders::default(), r#"{"result": []}"#);
    }
    let refusal = Refusal::default();
    let until = Instant::now() + Duration::from_mins(30);
    let transport = RefusedMeanwhile {
        site: site.clone(),
        refusal: refusal.clone(),
        until,
    };
    let mut names = TradeSite::with_transport(transport, refusal);
    names.pace = Pace::new(Duration::ZERO);

    let error = names.item_names().expect_err("a wait");

    assert!(matches!(error, NamesError::Wait(wait) if wait == until));
    assert_eq!(site.requests().len(), 1);
}

#[test]
fn an_error_status_isnt_a_refusal_and_a_redirection_is_one_request() {
    for status in [302, 503] {
        let site = FakeSite::default();
        site.answer(status, LimitHeaders::default(), "");
        let refusal = Refusal::default();
        let mut names = TradeSite::with_transport(site.clone(), refusal.clone());

        let error = names.item_names().expect_err("an error status");

        assert!(matches!(
            error,
            NamesError::Http { error: HttpError::Request(ureq::Error::StatusCode(code)), .. } if code == status
        ));
        assert_eq!(refusal.until(Instant::now()), None);
        assert_eq!(site.requests().len(), 1, "{status}");
    }
}

#[test]
fn the_names_are_asked_on_the_trade_site_of_each_language() {
    let site = FakeSite::default();
    for _ in 0..6 {
        site.answer(200, LimitHeaders::default(), r#"{"result": []}"#);
    }
    let mut names = TradeSite::with_transport(site.clone(), Refusal::default());
    names.pace = Pace::new(Duration::ZERO);

    // Empty answers: read, then refused as names.
    let error = names.item_names().expect_err("no name");

    assert!(matches!(error, NamesError::Missing(..)));
    let urls: Vec<String> = site
        .requests()
        .into_iter()
        .map(|request| request.url)
        .collect();
    assert_eq!(
        urls,
        [
            "https://www.pathofexile.com/api/trade2/data/items",
            "https://www.pathofexile.com/api/trade2/data/static",
            "https://www.pathofexile.com/api/trade2/data/stats",
            "https://fr.pathofexile.com/api/trade2/data/items",
            "https://fr.pathofexile.com/api/trade2/data/static",
            "https://fr.pathofexile.com/api/trade2/data/stats",
        ]
    );
}

#[test]
fn an_answer_without_a_kind_of_names_is_an_error() {
    let empty = r#"{"result": []}"#;

    let names = item_names(
        SystemTime::UNIX_EPOCH,
        &english(),
        &data(Language::French, FRENCH_ITEMS, empty, FRENCH_STATS),
    );

    assert!(matches!(
        names,
        Err(NamesError::Missing(_, Language::French))
    ));

    let english_empty = item_names(
        SystemTime::UNIX_EPOCH,
        &data(Language::English, empty, ENGLISH_STATIC, ENGLISH_STATS),
        &french(),
    );
    assert!(matches!(
        english_empty,
        Err(NamesError::Missing(_, Language::English))
    ));

    // Only the pseudo stats.
    let only_others = r#"{"result": [{"id": "pseudo", "label": "Pseudo", "entries": [
        {"id": "pseudo.pseudo_total_life", "text": "+# total maximum Life", "type": "pseudo"}
    ]}]}"#;
    let without_stats = item_names(
        SystemTime::UNIX_EPOCH,
        &english(),
        &data(Language::French, FRENCH_ITEMS, FRENCH_STATIC, only_others),
    );
    assert!(matches!(
        without_stats,
        Err(NamesError::Missing("stat templates", Language::French))
    ));
}

/// An answer gone wrong with templates by the hundred thousand would freeze the window while
/// they are grouped (the security audit of 2026-11-14): refused, as an answer without a kind.
#[test]
fn an_answer_with_too_many_stat_templates_is_an_error() {
    let with_templates = |count: usize| {
        let mut french = french();
        french.stats.push(ApiStatGroup {
            id: "explicit".to_owned(),
            entries: (0..count)
                .map(|number| ApiStat {
                    id: format!("explicit.stat_{number}"),
                    text: "# de Vie".to_owned(),
                })
                .collect(),
        });
        item_names(SystemTime::UNIX_EPOCH, &english(), &french)
    };

    // The French answer has one template already.
    assert!(with_templates(MAX_STAT_TEMPLATES - 1).is_ok());
    assert!(matches!(
        with_templates(MAX_STAT_TEMPLATES),
        Err(NamesError::TooManyStatTemplates(count, Language::French))
            if count == MAX_STAT_TEMPLATES + 1
    ));
}

#[test]
fn the_stat_templates_are_those_of_the_modifier_lines() {
    let names = names_of_both();

    let ids: Vec<&str> = names
        .stat_templates(Language::English)
        .iter()
        .map(|template| template.id.as_str())
        .collect();
    assert_eq!(
        ids,
        [
            "explicit.stat_3299347043",
            "explicit.stat_2954116742|46224",
            "rune.stat_1992191903"
        ]
    );
    assert_eq!(
        names.stat_templates(Language::French),
        [StatTemplate {
            id: "explicit.stat_3299347043".to_owned(),
            text: "# de Vie maximale".to_owned(),
        }]
    );
}

#[test]
fn the_stat_templates_taken_out_are_no_longer_in_the_names() {
    let mut names = names_of_both();

    let PerLanguage { english, french } = names.take_stat_templates();

    assert_eq!((english.len(), french.len()), (3, 1));
    assert_eq!(names.stat_templates(Language::English), []);
    assert_eq!(names.stat_templates(Language::French), []);
}

#[test]
fn the_base_types_are_those_of_the_equipment_in_each_language() {
    let names = names_of_both();

    assert_eq!(
        names.magic_base_type(Language::English, "Crackling Temple Maul of the Brute"),
        Some("Temple Maul")
    );
    assert_eq!(
        names.magic_base_type(Language::French, "Massue du temple crépitante"),
        Some("Massue du temple")
    );
    assert_eq!(
        names.magic_base_type(Language::English, "Preserved Rib"),
        None,
        "not equipment"
    );
}

#[test]
fn an_answer_without_the_expected_fields_is_an_error() {
    let parsed: Result<ApiResult<ApiItemCategory>, _> =
        serde_json::from_str(r#"{"result": [{"id": "weapon"}]}"#);

    assert!(parsed.is_err());
}

/// The second code review of the quality fixes: the flag was only tested on an error built
/// by hand.
#[test]
fn a_request_without_answer_after_the_first_ones_counts_as_answered_in_each_download() {
    let site = FakeSite::default();
    let mut names = TradeSite::with_transport(site.clone(), Refusal::default());

    // Nothing answers the first request: the trade site can't be reached.
    let first = names.item_names().expect_err("no answer");
    assert!(!first.answered(), "{first}");

    // One answer, then nothing: it can be reached, and the next attempt waits longer.
    site.answer(200, LimitHeaders::default(), r#"{"result": []}"#);
    let second = names.item_names().expect_err("no second answer");
    assert!(second.answered(), "{second}");
    assert!(matches!(
        second,
        NamesError::Http {
            after_answers: true,
            ..
        }
    ));

    // A new download starts over.
    let third = names.item_names().expect_err("no answer");
    assert!(!third.answered(), "{third}");
}
