use std::time::Instant;

use super::*;

/// The security audit of 2026-11-10: each category is a request, a second apart.
#[test]
fn an_answer_with_too_many_categories_is_refused() {
    let categories = |uniques: usize, currencies: usize| {
        let list = |count: usize| {
            (0..count)
                .map(|index| ApiCategory {
                    api_id: index.to_string(),
                })
                .collect()
        };
        ApiCategories {
            unique_categories: list(uniques),
            currency_categories: list(currencies),
        }
    };

    assert!(categories(40, MAX_CATEGORIES - 40).checked().is_ok());
    assert!(matches!(
        categories(40, MAX_CATEGORIES - 39).checked(),
        Err(FetchError::TooManyCategories(count)) if count == MAX_CATEGORIES + 1
    ));
}

/// The shape of the answer of `/Leagues` on 2026-10-25, without the fields not read.
const LEAGUES: &str = r#"[
    {"Value": "Forbidden Rites", "ShortName": "forbiddenrites", "IsCurrent": true, "DivinePrice": 718.4},
    {"Value": "HC Forbidden Rites", "ShortName": "forbiddenriteshc", "IsCurrent": true},
    {"Value": "Dawn of the Hunt", "ShortName": "hunt", "IsCurrent": false},
    {"Value": "Standard", "ShortName": "standard", "IsCurrent": false},
    {"Value": "Runes of Aldur", "ShortName": "runes", "IsCurrent": true}
]"#;

#[test]
fn reads_the_current_leagues_in_their_order() {
    let leagues: Vec<ApiLeague> = serde_json::from_str(LEAGUES).expect("the answer parses");

    assert_eq!(
        current_leagues(leagues),
        ["Forbidden Rites", "HC Forbidden Rites", "Runes of Aldur"].map(League::new)
    );
}

/// The shape of the answer of `/ReferenceCurrencies` on 2026-10-25.
const REFERENCE_CURRENCIES: &str = r#"[
    {"ApiId": "exalted", "Text": "Exalted Orb", "RelativePrice": 1},
    {"ApiId": "chaos", "Text": "Chaos Orb", "RelativePrice": 75.4},
    {"ApiId": "divine", "Text": "Divine Orb", "RelativePrice": 754.0}
]"#;

/// The shape of a page of `/Uniques/ByCategory` on 2026-10-25, without most of the
/// fields not read.
const UNIQUES: &str = r#"{"CurrentPage": 1, "Pages": 1, "Total": 2, "Items": [
    {"UniqueItemId": 361, "Text": "Bursting Decay Unset Ring", "Name": "Bursting Decay",
     "Type": "Unset Ring", "IsChanceable": false,
     "ItemMetadata": {"name": "Bursting Decay", "base_type": "Unset Ring",
      "implicit_mods": ["Grants (1-1) additional Skill Slot"],
      "explicit_mods": ["(15-25)% increased Rarity of Items found", "+(17-23)% to Chaos Resistance"]},
     "PriceLogs": [{"Price": 149.5, "Time": "2026-10-25T00:00:00Z", "Quantity": 4}, null],
     "CurrentPrice": 150.8, "CurrentQuantity": 4},
    {"UniqueItemId": 1, "Name": "Unlisted", "Type": "Ruby Ring", "PriceLogs": [],
     "CurrentPrice": null, "CurrentQuantity": null}
]}"#;

/// The shape of a page of `/Currencies/ByCategory` on 2026-10-25.
const CURRENCIES: &str = r#"{"CurrentPage": 1, "Pages": 1, "Total": 1, "Items": [
    {"CurrencyItemId": 18, "ApiId": "mirror", "Text": "Mirror of Kalandra",
     "CategoryApiId": "currency", "ItemMetadata": null,
     "PriceLogs": [{"Price": 4207102, "Time": "2026-10-25T00:00:00Z", "Quantity": 125}],
     "CurrentPrice": 4259889, "CurrentQuantity": 15}
]}"#;

fn conversion() -> Conversion {
    let currencies: Vec<ApiReferenceCurrency> =
        serde_json::from_str(REFERENCE_CURRENCIES).expect("the answer parses");
    Conversion::new(&currencies).expect("every rate is there")
}

#[test]
fn reads_the_rates_of_the_chaos_and_divine_orbs_in_exalted_orbs() {
    assert_eq!(
        conversion().rates,
        Rates {
            chaos: 75.4,
            divine: 754.0
        }
    );
}

#[test]
fn a_missing_rate_is_an_error() {
    let currencies: Vec<ApiReferenceCurrency> =
        serde_json::from_str(r#"[{"ApiId": "exalted", "RelativePrice": 1}]"#).unwrap();

    assert!(matches!(
        Conversion::new(&currencies),
        Err(FetchError::MissingRate(CHAOS))
    ));
}

#[test]
fn the_prices_are_converted_to_exalted_orbs() {
    // Prices given in Chaos Orbs, in a league whose base currency would be the chaos.
    let currencies: Vec<ApiReferenceCurrency> = serde_json::from_str(
        r#"[{"ApiId": "exalted", "RelativePrice": 0.5},
            {"ApiId": "chaos", "RelativePrice": 1},
            {"ApiId": "divine", "RelativePrice": 10}]"#,
    )
    .unwrap();
    let conversion = Conversion::new(&currencies).unwrap();
    let page: ApiPage<ApiUnique> = serde_json::from_str(UNIQUES).unwrap();
    let price = page.items.into_iter().next().unwrap().price;

    let price = conversion.price(price).expect("a current price");

    assert!((price.exalted - 301.6).abs() < 1e-9);
    assert_eq!(conversion.rates.divine, 20.0);
}

#[test]
fn reads_the_uniques_and_skips_those_without_a_price() {
    let page: ApiPage<ApiUnique> = serde_json::from_str(UNIQUES).expect("the answer parses");
    let conversion = conversion();
    let prices: Vec<(String, String, Option<Price>)> = page
        .items
        .into_iter()
        .map(|item| (item.name, item.base_type, conversion.price(item.price)))
        .collect();

    assert_eq!(
        prices,
        [
            (
                "Bursting Decay".to_owned(),
                "Unset Ring".to_owned(),
                Some(Price {
                    exalted: 150.8,
                    quantity: 4,
                    history: vec![Some(149.5), None],
                })
            ),
            ("Unlisted".to_owned(), "Ruby Ring".to_owned(), None),
        ]
    );
}

#[test]
fn reads_the_implicit_and_explicit_modifiers_of_a_unique() {
    let page: ApiPage<ApiUnique> = serde_json::from_str(UNIQUES).expect("the answer parses");
    let modifiers: Vec<UniqueModifiers> = page
        .items
        .into_iter()
        .map(|item| unique_modifiers(item.item_metadata.as_ref()))
        .collect();

    assert_eq!(
        modifiers[0].implicit,
        ["Grants (1-1) additional Skill Slot"]
    );
    assert_eq!(
        modifiers[0].explicit,
        [
            "(15-25)% increased Rarity of Items found",
            "+(17-23)% to Chaos Resistance"
        ]
    );
    assert_eq!(modifiers[1], UniqueModifiers::default());
}

#[test]
fn modifiers_null_or_unexpected_lose_only_themselves() {
    let page: ApiPage<ApiUnique> = serde_json::from_str(
        r#"{"Pages": 1, "Items": [{"Name": "A", "Type": "Ruby Ring",
            "ItemMetadata": {"implicit_mods": null, "explicit_mods": ["+(5-10) to Life", 3]},
            "PriceLogs": [], "CurrentPrice": 1.0, "CurrentQuantity": 1}]}"#,
    )
    .expect("the page still parses");
    let modifiers = unique_modifiers(page.items[0].item_metadata.as_ref());

    assert_eq!(modifiers.implicit, Vec::<String>::new());
    assert_eq!(modifiers.explicit, ["+(5-10) to Life"]);
    for metadata in ["[]", r#"{"implicit_mods": "text"}"#, "null"] {
        let metadata: serde_json::Value = serde_json::from_str(metadata).expect("json");
        assert_eq!(
            unique_modifiers(Some(&metadata)),
            UniqueModifiers::default(),
            "{metadata}"
        );
    }
}

#[test]
fn a_unique_keeps_its_price_and_its_modifiers() {
    let page: ApiPage<ApiUnique> = serde_json::from_str(UNIQUES).expect("the answer parses");
    let conversion = conversion();
    let uniques: Vec<UniquePrice> = page
        .items
        .into_iter()
        .filter_map(|item| unique_price(&conversion, item))
        .collect();

    assert_eq!(uniques.len(), 1, "the unique without a price is skipped");
    assert_eq!(uniques[0].name, "Bursting Decay");
    assert_eq!(uniques[0].price.exalted, 150.8);
    assert_eq!(
        uniques[0].modifiers.implicit,
        ["Grants (1-1) additional Skill Slot"]
    );
    assert_eq!(uniques[0].modifiers.explicit.len(), 2);
}

#[test]
fn a_unique_with_unexpected_modifiers_keeps_its_price() {
    let page: ApiPage<ApiUnique> = serde_json::from_str(
        r#"{"Pages": 1, "Items": [{"Name": "A", "Type": "Ruby Ring",
            "ItemMetadata": {"implicit_mods": "text", "explicit_mods": {"mod": 3}},
            "PriceLogs": [], "CurrentPrice": 2.0, "CurrentQuantity": 1}]}"#,
    )
    .expect("the page still parses");
    let item = page.items.into_iter().next().unwrap();

    let unique = unique_price(&conversion(), item).expect("a current price");

    assert_eq!(unique.price.exalted, 2.0);
    assert_eq!(unique.modifiers, UniqueModifiers::default());
}

#[test]
fn reads_the_items_of_the_currency_exchange() {
    let page: ApiPage<ApiCurrency> = serde_json::from_str(CURRENCIES).expect("the answer parses");
    let mirror = page.items.into_iter().next().unwrap();

    assert_eq!(
        (mirror.text.as_str(), mirror.api_id.as_str()),
        ("Mirror of Kalandra", "mirror")
    );
    assert_eq!(
        conversion().price(mirror.price).map(|price| price.quantity),
        Some(15)
    );
}

#[test]
fn the_currency_category_is_downloaded_first() {
    let categories: ApiCategories = serde_json::from_str(
        r#"{"UniqueCategories": [], "CurrencyCategories": [
            {"ApiId": "fragments"}, {"ApiId": "runes"}, {"ApiId": "currency"}, {"ApiId": "abyss"}
        ]}"#,
    )
    .expect("the answer parses");

    let order: Vec<String> = download_order(categories.currency_categories)
        .into_iter()
        .map(|category| category.api_id)
        .collect();

    assert_eq!(order, ["currency", "fragments", "runes", "abyss"]);
}

#[test]
fn an_answer_without_the_expected_fields_is_an_error() {
    let parsed: Result<Vec<ApiLeague>, _> = serde_json::from_str(r#"[{"Name": "x"}]"#);

    assert!(parsed.is_err());
}

/// The network review of 2026-11-10: one odd item lost the 250 prices of its page, and
/// the whole download with it.
#[test]
fn an_item_that_cant_be_read_is_left_out_of_its_page() {
    // A unique whose base type is `null`, before the two of the answer.
    let page = UNIQUES.replacen(
        r#""Items": ["#,
        r#""Items": [{"Name": "Odd", "Type": null, "PriceLogs": []},"#,
        1,
    );

    let page: ApiPage<ApiUnique> = serde_json::from_str(&page).expect("the answer parses");

    let names: Vec<&str> = page.items.iter().map(|item| item.name.as_str()).collect();
    assert_eq!(names, ["Bursting Decay", "Unlisted"]);
}

/// Not a category without prices: the database changed its answer.
#[test]
fn a_page_none_of_whose_items_can_be_read_is_an_error() {
    let page = CURRENCIES.replace("ApiId", "Id");

    assert!(serde_json::from_str::<ApiPage<ApiCurrency>>(&page).is_err());
}

#[test]
fn a_price_is_read_without_its_history() {
    let page = CURRENCIES.replace("PriceLogs", "History");
    let page: ApiPage<ApiCurrency> = serde_json::from_str(&page).expect("the answer parses");

    let price = page.items.into_iter().next().map(|item| item.price);

    let price = conversion()
        .price(price.expect("an item"))
        .expect("a price");
    assert!((price.exalted - 4_259_889.0).abs() < 1e-6);
    assert_eq!(price.history, [None; 0]);
}

/// Whatever the other currencies of the answer: only the three rates are needed.
#[test]
fn the_reference_currencies_that_cant_be_read_are_left_out_however_many() {
    let odd = r#", {"ApiId": "mirror"}, {"ApiId": "vaal"}, {"ApiId": "regal"}, 5]"#;
    let values: Vec<serde_json::Value> =
        serde_json::from_str(&REFERENCE_CURRENCIES.replacen(']', odd, 1)).expect("a list");

    let currencies = reference_currencies(values);

    assert_eq!(currencies.len(), 3);
    assert!(Conversion::new(&currencies).is_ok());
    assert!(matches!(
        Conversion::new(&reference_currencies(vec![serde_json::Value::from(5)])),
        Err(FetchError::MissingRate(_))
    ));
}

#[test]
fn a_failure_names_the_request_and_tells_whether_the_database_answered() {
    let failure = |error, after_answers| FetchError::Http {
        path: "/Leagues".to_owned(),
        error: HttpError::Request(error),
        after_answers,
    };
    let hung = || ureq::Error::Timeout(ureq::Timeout::Global);

    let refused = failure(ureq::Error::StatusCode(500), false);
    assert!(refused.answered());
    assert!(refused.to_string().ends_with("(/Leagues)"), "{refused}");
    assert!(!failure(hung(), false).answered());
    // The twentieth request of a download hangs: the database was there for the others.
    assert!(failure(hung(), true).answered());
}

/// The code review of the quality fixes: left out, a league that can't be read was taken
/// for a league that ended, and the player moved to Standard. `leagues` reads a plain
/// `Vec`, which this pins; `the_leagues_are_read_whole_or_not_at_all` asks for them.
#[test]
fn the_list_of_the_leagues_has_no_lenient_reading() {
    let leagues = r#"[{"Value": "Standard", "IsCurrent": true}, {"Value": "Forbidden Rites"}]"#;

    assert!(serde_json::from_str::<Vec<ApiLeague>>(leagues).is_err());
}

/// An API that answers each request it knows with its text, any other with the status
/// 404, and keeps the requests it got, written `path?name=value&…`, with their times.
#[derive(Default)]
struct FakeApi {
    answers: Vec<(String, String)>,
    /// The request that gets no answer.
    hangs_at: Option<String>,
    requests: Vec<String>,
    times: Vec<Instant>,
}

impl FakeApi {
    fn with(mut self, request: impl Into<String>, answer: &str) -> Self {
        self.answers.push((request.into(), answer.to_owned()));
        self
    }

    fn hanging_at(mut self, request: &str) -> Self {
        self.hangs_at = Some(request.to_owned());
        self
    }
}

impl Source for FakeApi {
    fn get(&mut self, path: &str, query: &[(&str, &str)]) -> Result<String, HttpError> {
        let pairs: Vec<String> = query
            .iter()
            .map(|(name, value)| format!("{name}={value}"))
            .collect();
        let request = if pairs.is_empty() {
            path.to_owned()
        } else {
            format!("{path}?{}", pairs.join("&"))
        };
        self.times.push(Instant::now());
        self.requests.push(request.clone());
        if self.hangs_at.as_deref() == Some(request.as_str()) {
            return Err(ureq::Error::Timeout(ureq::Timeout::Global).into());
        }
        self.answers
            .iter()
            .find(|(known, _)| *known == request)
            .map(|(_, answer)| answer.clone())
            .ok_or_else(|| ureq::Error::StatusCode(404).into())
    }
}

/// What a download told, interrupted at its check number `stop_at` (from 0).
#[derive(Default)]
struct Download {
    stop_at: Option<usize>,
    checks: usize,
    /// The currencies and the uniques priced so far, and the share done.
    progress: Vec<(usize, usize, f32)>,
}

impl DownloadObserver for Download {
    fn interrupted(&mut self) -> bool {
        self.checks += 1;
        self.stop_at.is_some_and(|stop| self.checks > stop)
    }

    fn progress(&mut self, prices: &PriceList, progress: f32) {
        self.progress
            .push((prices.currencies.len(), prices.uniques.len(), progress));
    }
}

const CATEGORIES: &str = r#"{"UniqueCategories": [{"ApiId": "accessory"}],
    "CurrencyCategories": [{"ApiId": "fragments"}, {"ApiId": "currency"}]}"#;
const EMPTY_PAGE: &str = r#"{"CurrentPage": 1, "Pages": 1, "Total": 0, "Items": []}"#;

const CATEGORIES_PATH: &str = "/Leagues/HC%20Forbidden%20Rites/Items/Categories";
const RATES_PATH: &str = "/Leagues/HC%20Forbidden%20Rites/ReferenceCurrencies";

/// The request of the page `page` of a category of `kind` (`Currencies` or `Uniques`).
fn page_request(kind: &str, category: &str, page: u32) -> String {
    format!(
        "/Leagues/HC%20Forbidden%20Rites/{kind}/ByCategory?category={category}&perPage=250&page={page}"
    )
}

/// An API with the five answers of a whole download of the league of [`league`].
fn api() -> FakeApi {
    FakeApi::default()
        .with(CATEGORIES_PATH, CATEGORIES)
        .with(RATES_PATH, REFERENCE_CURRENCIES)
        .with(page_request("Currencies", "currency", 1), CURRENCIES)
        .with(page_request("Currencies", "fragments", 1), EMPTY_PAGE)
        .with(page_request("Uniques", "accessory", 1), UNIQUES)
}

fn league() -> League {
    League::new("HC Forbidden Rites")
}

/// A client of `api` that doesn't wait between two requests.
fn scout(api: FakeApi) -> Poe2Scout<FakeApi> {
    Poe2Scout::with_source(api, Duration::ZERO)
}

/// The network review of 2026-11-10: nothing ran the download itself, only its parts.
#[test]
fn a_download_asks_for_the_rates_then_the_currency_then_the_uniques() {
    let mut scout = scout(api());
    let mut download = Download::default();

    let prices = scout
        .prices(&league(), &mut download)
        .expect("every request answered")
        .expect("not interrupted");

    assert_eq!(
        scout.source.requests,
        [
            CATEGORIES_PATH.to_owned(),
            RATES_PATH.to_owned(),
            page_request("Currencies", "currency", 1),
            page_request("Currencies", "fragments", 1),
            page_request("Uniques", "accessory", 1),
        ]
    );
    assert_eq!(prices.league, league());
    assert_eq!(prices.rates, conversion().rates);
    let currencies: Vec<&str> = prices
        .currencies
        .iter()
        .map(|currency| currency.exchange_id.as_str())
        .collect();
    assert_eq!(currencies, ["mirror"]);
    let uniques: Vec<&str> = prices
        .uniques
        .iter()
        .map(|unique| unique.name.as_str())
        .collect();
    assert_eq!(uniques, ["Bursting Decay"], "the one with a price");
}

/// The prices of the currency reach the application before the uniques are asked for.
#[test]
fn a_download_tells_its_prices_after_each_category() {
    let mut download = Download::default();

    scout(api())
        .prices(&league(), &mut download)
        .expect("every request answered");

    assert_eq!(
        download.progress,
        [(0, 0, 0.4), (1, 0, 0.6), (1, 0, 0.8), (1, 1, 1.0)]
    );
}

#[test]
fn every_page_of_a_category_is_read() {
    let page = |number: u32| {
        UNIQUES
            .replace(r#""Pages": 1"#, r#""Pages": 3"#)
            .replace("Bursting Decay", &format!("Bursting Decay {number}"))
    };
    let api = (2..=3).fold(
        api().with(page_request("Uniques", "accessory", 1), &page(1)),
        |api, number| api.with(page_request("Uniques", "accessory", number), &page(number)),
    );
    // The first answer known for a request is the one given: the pages of three first.
    let api = FakeApi {
        answers: api.answers.into_iter().rev().collect(),
        ..FakeApi::default()
    };
    let mut scout = scout(api);

    let prices = scout
        .prices(&league(), &mut Download::default())
        .expect("every request answered")
        .expect("not interrupted");

    assert_eq!(scout.source.requests.len(), 7);
    assert_eq!(
        scout.source.requests[4..],
        [1, 2, 3].map(|number| page_request("Uniques", "accessory", number))
    );
    let names: Vec<&str> = prices
        .uniques
        .iter()
        .map(|unique| unique.name.as_str())
        .collect();
    assert_eq!(
        names,
        ["Bursting Decay 1", "Bursting Decay 2", "Bursting Decay 3"]
    );
}

/// A wrong count of pages must not send requests without end.
#[test]
fn a_category_stops_at_its_tenth_page() {
    let endless = CURRENCIES.replace(r#""Pages": 1"#, r#""Pages": 4000"#);
    let api = (1..=MAX_PAGES + 5).fold(FakeApi::default(), |api, page| {
        api.with(page_request("Currencies", "currency", page), &endless)
    });
    let mut scout = scout(api);

    let items = scout
        .category::<ApiCurrency>(
            "HC%20Forbidden%20Rites",
            Kind::Currencies,
            "currency",
            &mut Download::default(),
        )
        .expect("every page answered")
        .expect("not interrupted");

    assert_eq!(scout.source.requests.len(), MAX_PAGES as usize);
    assert_eq!(items.len(), MAX_PAGES as usize);
}

/// `docs/poe2scout-usage.md`: after a refusal, wait without insisting. The requests left
/// of the download aren't sent.
#[test]
fn a_download_stops_at_its_first_failed_request() {
    let fragments = page_request("Currencies", "fragments", 1);
    let mut api = api();
    api.answers.retain(|(request, _)| *request != fragments);
    let mut scout = scout(api);
    let mut download = Download::default();

    let error = scout
        .prices(&league(), &mut download)
        .expect_err("a request failed");

    assert_eq!(scout.source.requests.len(), 4, "no unique asked for");
    assert_eq!(scout.source.requests.last(), Some(&fragments));
    assert!(
        matches!(
            &error,
            FetchError::Http {
                path,
                error: HttpError::Request(ureq::Error::StatusCode(404)),
                after_answers: true,
            } if path == "/Leagues/HC%20Forbidden%20Rites/Currencies/ByCategory"
        ),
        "{error:?}"
    );
    // The prices of the currency were told before the failure.
    assert_eq!(download.progress.last(), Some(&(1, 0, 0.6)));
}

/// The code review of the quality fixes: a request that hangs after the first answers of
/// its download is a failure the database answered, and waits longer and longer.
#[test]
fn a_request_that_hangs_counts_as_answered_after_the_first_answers_of_its_download() {
    let hangs = |request: &str| {
        let mut scout = scout(api().hanging_at(request));
        let error = scout
            .prices(&league(), &mut Download::default())
            .expect_err("a request hung");
        (error.answered(), scout.source.requests.len())
    };

    assert_eq!(hangs(CATEGORIES_PATH), (false, 1));
    assert_eq!(hangs(RATES_PATH), (true, 2));
    assert_eq!(hangs(&page_request("Uniques", "accessory", 1)), (true, 5));
}

#[test]
fn the_answers_of_an_earlier_download_dont_count_for_the_next_one() {
    let mut after_leagues = scout(api().with("/Leagues", LEAGUES).hanging_at(CATEGORIES_PATH));
    after_leagues.leagues().expect("the leagues");

    let error = after_leagues
        .prices(&league(), &mut Download::default())
        .expect_err("a request hung");

    assert!(!error.answered(), "{error:?}");
    // The same for the leagues, after a download that was answered.
    let mut after_prices = scout(api().hanging_at("/Leagues"));
    after_prices
        .prices(&league(), &mut Download::default())
        .expect("every request answered");
    let error = after_prices.leagues().expect_err("a request hung");
    assert!(!error.answered(), "{error:?}");
}

#[test]
fn the_leagues_are_read_whole_or_not_at_all() {
    let leagues = |answer: &str| {
        let mut scout = scout(FakeApi::default().with("/Leagues", answer));
        let leagues = scout.leagues();
        assert_eq!(scout.source.requests, ["/Leagues"]);
        leagues
    };

    assert_eq!(
        leagues(LEAGUES).expect("the leagues"),
        ["Forbidden Rites", "HC Forbidden Rites", "Runes of Aldur"].map(League::new)
    );
    // One league that can't be read: none is, rather than the others without it.
    let odd = LEAGUES.replacen(r#""IsCurrent": true, "DivinePrice": 718.4"#, "", 1);
    let error = leagues(&odd).expect_err("a league without its fields");
    assert!(
        matches!(
            &error,
            FetchError::Http {
                error: HttpError::Json(_),
                ..
            }
        ),
        "{error:?}"
    );
    assert!(error.answered());
    assert!(matches!(
        leagues(&LEAGUES.replace("true", "false")),
        Err(FetchError::NoCurrentLeague)
    ));
}

/// The pace waits first, then the interruption is checked, right before each request.
#[test]
fn an_interrupted_download_sends_no_other_request() {
    for stop_at in 0..5 {
        let mut scout = scout(api());
        let mut download = Download {
            stop_at: Some(stop_at),
            ..Download::default()
        };

        let prices = scout.prices(&league(), &mut download);

        assert!(matches!(prices, Ok(None)), "interrupted at {stop_at}");
        assert_eq!(scout.source.requests.len(), stop_at);
    }
}

#[test]
fn rates_or_categories_that_cant_be_used_stop_the_download_before_any_price() {
    let many: Vec<String> = (0..=MAX_CATEGORIES)
        .map(|index| format!(r#"{{"ApiId": "{index}"}}"#))
        .collect();
    let many = format!(
        r#"{{"UniqueCategories": [{}], "CurrencyCategories": []}}"#,
        many.join(",")
    );
    let failure = |request: &str, answer: &str| {
        // Before the answer of `api` for the same request, so that it is the one given.
        let mut api = api();
        api.answers
            .insert(0, (request.to_owned(), answer.to_owned()));
        let mut scout = scout(api);
        let error = scout
            .prices(&league(), &mut Download::default())
            .expect_err("an answer that can't be used");
        assert_eq!(scout.source.requests.len(), 2, "{error}");
        error
    };

    assert!(matches!(
        failure(CATEGORIES_PATH, &many),
        FetchError::TooManyCategories(_)
    ));
    assert!(matches!(
        failure(RATES_PATH, r#"[{"ApiId": "exalted", "RelativePrice": 1}]"#),
        FetchError::MissingRate(CHAOS)
    ));
}

/// `docs/poe2scout-usage.md`: one request a second at most. Each request of a download
/// waits for the interval since the one before, whatever the request.
#[test]
fn the_requests_of_a_download_are_spaced() {
    const INTERVAL: Duration = Duration::from_millis(15);
    let mut scout = Poe2Scout::with_source(api().with("/Leagues", LEAGUES), INTERVAL);
    // From before the first request, which the pace counts from: the gap between two
    // instants taken after each wait could look shorter than the interval.
    let start = Instant::now();

    scout.leagues().expect("the leagues");
    scout
        .prices(&league(), &mut Download::default())
        .expect("every request answered");

    let times = &scout.source.times;
    assert_eq!(times.len(), 6);
    for (after, time) in (0_u32..).zip(times) {
        let least = INTERVAL * after;
        assert!(time.duration_since(start) >= least, "{start:?} {times:?}");
    }
}

#[test]
fn the_pace_of_the_application_is_slower_than_one_request_a_second() {
    assert!(PACE > Duration::from_secs(1));
}
