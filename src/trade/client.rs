//! The client of the trade site API: the searches of items and their offers, and the searches
//! of the currency exchange, within the limits the trade site announces.
//!
//! It never sends a request its limits don't allow, and lets 10 seconds pass at least between
//! two searches, whatever the item. It sends no session cookie: the searches work without an
//! account. The rules of its use are in `docs/trade-site-usage.md`.

use std::fmt;
use std::time::{Duration, Instant};

use serde::de::DeserializeOwned;

use super::limits::{self, Hit, LimitHeaders, Limits, Policy, RuleHeaders};
use super::offers::{ApiExchange, ApiFetch, ApiSearch, ExchangeOffer, ItemOffer};
use super::query::{self, ItemQuery, SearchKind};
use super::refusal::Refusal;
use crate::http::{self, HttpError};
use crate::language::Language;
use crate::league::League;
use crate::logging;

/// The time between two searches, at least.
pub const SEARCH_INTERVAL: Duration = Duration::from_secs(10);
/// The offers one fetch gives at most.
pub const FETCH_SIZE: usize = 10;

/// Why the trade site gave no answer.
#[derive(Debug)]
pub enum TradeError {
    /// The limits don't allow the request before this time: it wasn't sent.
    Wait(Instant),
    /// The trade site refused the request (HTTP 429, or 403): nothing is sent before this
    /// time.
    Refused(Instant),
    /// No answer, an error status, or an answer that isn't the expected one.
    Http(HttpError),
}

impl fmt::Display for TradeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let seconds = |until: &Instant| until.saturating_duration_since(Instant::now()).as_secs();
        match self {
            Self::Wait(until) => write!(formatter, "not sent, {} s to wait", seconds(until)),
            Self::Refused(until) => write!(formatter, "refused, {} s to wait", seconds(until)),
            Self::Http(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for TradeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Http(error) => Some(error),
            Self::Wait(_) | Self::Refused(_) => None,
        }
    }
}

impl From<HttpError> for TradeError {
    fn from(error: HttpError) -> Self {
        Self::Http(error)
    }
}

/// A search of items: the ids of its offers, the cheapest first, to fetch 10 at a time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemSearch {
    pub id: String,
    pub offers: Vec<String>,
    /// The offers found, more than the ids given when there are many.
    pub total: u64,
}

/// A search of the currency exchange, with its offers.
#[derive(Debug, Clone, PartialEq)]
pub struct ExchangeSearch {
    pub offers: Vec<ExchangeOffer>,
    pub total: u64,
}

/// A request to the trade site: a `GET`, or a `POST` of a JSON body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Request {
    pub url: String,
    pub body: Option<String>,
}

/// An answer of the trade site, whatever its status: its limits are read even when its body
/// can't be.
#[derive(Debug)]
pub(super) struct Answer {
    pub status: u16,
    pub limits: LimitHeaders,
    /// Of an error status too: the trade site says there why it turned a search down.
    pub body: Result<String, ureq::Error>,
}

impl Answer {
    /// Writes to the log why the trade site answered with an error status, when it says so:
    /// after a change of the game, the filter it no longer knows is named there.
    pub fn log_error_message(&self) {
        let message = self.body.as_deref().ok().and_then(error_message);
        if let Some(message) = message {
            log::warn!("Trade site: status {}, \"{message}\"", self.status);
        }
    }
}

/// The message of the body of an error, `{"error": {"code": 2, "message": "Unknown item base
/// type"}}`, as the log writes a text of a web service: one short line.
fn error_message(body: &str) -> Option<String> {
    let body: serde_json::Value = serde_json::from_str(body).ok()?;
    let message = body.get("error")?.get("message")?.as_str()?;
    Some(logging::plain(message))
}

/// How the requests are sent: over the network, or to a fake trade site in the tests.
pub(super) trait Transport {
    fn send(&mut self, request: &Request) -> Result<Answer, ureq::Error>;
}

/// The requests sent with `ureq`, which keeps no cookie (its `cookies` feature is off, see
/// `Cargo.toml`), and follows no redirection: each request is one request counted by the
/// limits, and a redirection is an error status.
pub(super) struct Network(ureq::Agent);

impl Network {
    pub fn new() -> Self {
        Self(http::agent())
    }
}

impl Transport for Network {
    fn send(&mut self, request: &Request) -> Result<Answer, ureq::Error> {
        let mut response = match &request.body {
            Some(body) => self
                .0
                .post(&request.url)
                .config()
                .http_status_as_error(false)
                .max_redirects(0)
                .build()
                .content_type("application/json")
                .send(body)?,
            None => self
                .0
                .get(&request.url)
                .config()
                .http_status_as_error(false)
                .max_redirects(0)
                .build()
                .call()?,
        };
        let headers = response.headers();
        let header = |name: &str| {
            headers
                .get(name)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned)
        };
        let rules = header("x-rate-limit-rules")
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|rule| !rule.is_empty())
            .map(|rule| {
                // Missing, its headers are empty: unreadable, they keep the limits known.
                let name = format!("x-rate-limit-{}", rule.to_ascii_lowercase());
                RuleHeaders {
                    name: rule.to_owned(),
                    limits: header(&name).unwrap_or_default(),
                    state: header(&format!("{name}-state")).unwrap_or_default(),
                }
            })
            .collect();
        let limits = LimitHeaders {
            policy: header("x-rate-limit-policy"),
            rules,
            retry_after: header("retry-after"),
        };
        let status = response.status().as_u16();
        // Of an error too, within the same limit of size: see `Answer::log_error_message`.
        let body = http::read_text(response.body_mut());
        Ok(Answer {
            status,
            limits,
            body,
        })
    }
}

/// The client of the trade site.
pub struct TradeClient<T = Network> {
    transport: T,
    limits: Limits,
    /// The wait asked by a refusal, shared with the download of the item names.
    refusal: Refusal,
    last_search: Option<Instant>,
}

impl TradeClient {
    #[must_use]
    pub fn new(refusal: Refusal) -> Self {
        Self::with_transport(Network::new(), refusal)
    }
}

impl<T: Transport> TradeClient<T> {
    pub(super) fn with_transport(transport: T, refusal: Refusal) -> Self {
        Self {
            transport,
            limits: Limits::default(),
            refusal,
            last_search: None,
        }
    }

    /// When a search of `kind` may be sent, `now` at the earliest: 10 s after the last
    /// search, once its limits allow it; for items, once they also allow the fetch of its
    /// first offers.
    #[must_use]
    pub fn next_search(&self, kind: SearchKind, now: Instant) -> Instant {
        let interval = self
            .last_search
            .map_or(now, |last| (last + SEARCH_INTERVAL).max(now));
        let policies: &[Policy] = match kind {
            SearchKind::Items => &[Policy::Search, Policy::Fetch],
            SearchKind::Exchange => &[Policy::Exchange],
        };
        policies
            .iter()
            .map(|&policy| self.ready_at(policy, now))
            .fold(interval, Instant::max)
    }

    /// When a request of `policy` may be sent: once its limits allow it, and after the wait
    /// of a refusal, whichever request of the application the trade site refused.
    fn ready_at(&self, policy: Policy, now: Instant) -> Instant {
        let ready = self.limits.ready_at(policy, now);
        self.refusal
            .until(now)
            .map_or(ready, |until| ready.max(until))
    }

    /// When the next offers of a search may be fetched.
    #[must_use]
    pub fn next_fetch(&self, now: Instant) -> Instant {
        self.ready_at(Policy::Fetch, now)
    }

    /// Searches `query` in `league`, on the trade site of `language`.
    pub fn search_items(
        &mut self,
        language: Language,
        league: &League,
        query: &ItemQuery,
    ) -> Result<ItemSearch, TradeError> {
        self.check_search(SearchKind::Items)?;
        let request = Request {
            url: format!(
                "{}/api/trade2/search/poe2/{}",
                super::host(language),
                http::encode(league.id())
            ),
            body: Some(query.to_json().to_string()),
        };
        let answer: ApiSearch = self.send(Policy::Search, &request)?;
        Ok(ItemSearch {
            id: answer.id,
            offers: answer.result,
            total: answer.total,
        })
    }

    /// The offers `ids` of the search `search`, 10 at most, in their order.
    pub fn fetch(
        &mut self,
        language: Language,
        search: &str,
        ids: &[String],
    ) -> Result<Vec<ItemOffer>, TradeError> {
        let ids = ids.get(..FETCH_SIZE).unwrap_or(ids);
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let request = Request {
            url: format!(
                "{}/api/trade2/fetch/{}?query={}&realm=poe2",
                super::host(language),
                ids.iter()
                    .map(|id| http::encode(id))
                    .collect::<Vec<_>>()
                    .join(","),
                http::encode(search)
            ),
            body: None,
        };
        let answer: ApiFetch = self.send(Policy::Fetch, &request)?;
        Ok(answer.offers(language))
    }

    /// Searches the offers of the item of the currency exchange `want` in `league`, on the
    /// trade site of `language`.
    pub fn search_exchange(
        &mut self,
        language: Language,
        league: &League,
        want: &str,
    ) -> Result<ExchangeSearch, TradeError> {
        self.check_search(SearchKind::Exchange)?;
        let request = Request {
            url: format!(
                "{}/api/trade2/exchange/poe2/{}",
                super::host(language),
                http::encode(league.id())
            ),
            body: Some(query::exchange_search(want).to_string()),
        };
        let answer: ApiExchange = self.send(Policy::Exchange, &request)?;
        let total = answer.total;
        Ok(ExchangeSearch {
            offers: answer.offers(),
            total,
        })
    }

    /// Counts a search of `kind`, unless it must wait.
    fn check_search(&mut self, kind: SearchKind) -> Result<(), TradeError> {
        let now = Instant::now();
        let ready = self.next_search(kind, now);
        if ready > now {
            return Err(TradeError::Wait(ready));
        }
        self.last_search = Some(now);
        Ok(())
    }

    /// Sends `request` of `policy`, when its limits allow it, and reads its answer.
    fn send<A: DeserializeOwned>(
        &mut self,
        policy: Policy,
        request: &Request,
    ) -> Result<A, TradeError> {
        let sent = Instant::now();
        let ready = self.ready_at(policy, sent);
        if ready > sent {
            return Err(TradeError::Wait(ready));
        }
        let result = self.transport.send(request);
        // Counted once answered, with both times: nothing else is sent meanwhile
        // (`&mut self`). Only a success that tells the limits was counted for sure by the
        // trade site: a request that didn't reach it, or that it refused, may not be.
        let now = Instant::now();
        let counted = result.as_ref().is_ok_and(|answer| {
            (200..300).contains(&answer.status) && !answer.limits.rules.is_empty()
        });
        self.limits.sent(
            policy,
            Hit {
                sent,
                answered: now,
                counted,
            },
        );
        let answer = result.map_err(|error| TradeError::Http(HttpError::Request(error)))?;
        // A 403 too: the trade site turns the address away, as it does the item names.
        if matches!(answer.status, 403 | 429) {
            log::warn!(
                "Trade site: request refused with the status {}",
                answer.status
            );
            let until = self.limits.refused(policy, &answer.limits, now);
            self.refusal
                .extend(now + limits::refusal_wait(&answer.limits));
            return Err(TradeError::Refused(until));
        }
        self.limits.answered(policy, &answer.limits, now);
        if !(200..300).contains(&answer.status) {
            answer.log_error_message();
            return Err(HttpError::Request(ureq::Error::StatusCode(answer.status)).into());
        }
        let body = answer.body.map_err(HttpError::Request)?;
        Ok(serde_json::from_str(&body).map_err(HttpError::Json)?)
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::trade::query::{RarityFilter, StateFilters};
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    /// A trade site that gives the answers it was given, in their order, and keeps the
    /// requests it got.
    #[derive(Default, Clone)]
    pub struct FakeSite {
        pub answers: Arc<Mutex<VecDeque<Answer>>>,
        pub requests: Arc<Mutex<Vec<Request>>>,
    }

    impl FakeSite {
        pub fn answer(&self, status: u16, limits: LimitHeaders, body: &str) {
            self.answers.lock().unwrap().push_back(Answer {
                status,
                limits,
                body: Ok(body.to_owned()),
            });
        }

        pub fn requests(&self) -> Vec<Request> {
            self.requests.lock().unwrap().clone()
        }
    }

    impl Transport for FakeSite {
        fn send(&mut self, request: &Request) -> Result<Answer, ureq::Error> {
            self.requests.lock().unwrap().push(request.clone());
            self.answers
                .lock()
                .unwrap()
                .pop_front()
                .ok_or(ureq::Error::ConnectionFailed)
        }
    }

    pub const SEARCH: &str = r#"{"id": "Q1", "complexity": 4, "total": 13,
        "result": ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m"]}"#;

    pub fn ring() -> ItemQuery {
        ItemQuery {
            name: Some("Andvarius".to_owned()),
            base_type: Some("Gold Ring".to_owned()),
            category: None,
            properties: Vec::new(),
            rarity: Some(RarityFilter::Unique),
            states: StateFilters::default(),
            stats: Vec::new(),
        }
    }

    /// The headers of a policy with the rule `Ip`.
    pub fn limits(policy: &str, rule: &str, state: &str) -> LimitHeaders {
        LimitHeaders {
            policy: Some(policy.to_owned()),
            rules: vec![RuleHeaders {
                name: "Ip".to_owned(),
                limits: rule.to_owned(),
                state: state.to_owned(),
            }],
            retry_after: None,
        }
    }

    #[test]
    fn a_search_is_posted_to_the_trade_site_of_the_language_of_the_item() {
        let site = FakeSite::default();
        site.answer(200, LimitHeaders::default(), SEARCH);
        let mut client = TradeClient::with_transport(site.clone(), Refusal::default());

        let search = client
            .search_items(
                Language::French,
                &League::new("HC Forbidden Rites"),
                &ring(),
            )
            .expect("an answer");

        assert_eq!(search.id, "Q1");
        assert_eq!(search.offers.len(), 13);
        assert_eq!(search.total, 13);
        let request = &site.requests()[0];
        assert_eq!(
            request.url,
            "https://fr.pathofexile.com/api/trade2/search/poe2/HC%20Forbidden%20Rites"
        );
        assert_eq!(
            request.body.as_deref(),
            Some(ring().to_json().to_string().as_str())
        );
    }

    #[test]
    fn ten_offers_at_most_are_fetched_by_their_ids() {
        let site = FakeSite::default();
        site.answer(200, LimitHeaders::default(), r#"{"result": []}"#);
        let mut client = TradeClient::with_transport(site.clone(), Refusal::default());
        let ids: Vec<String> = ('a'..='m').map(String::from).collect();

        client
            .fetch(Language::English, "Q1", &ids)
            .expect("an answer");

        assert_eq!(
            site.requests()[0],
            Request {
                url: "https://www.pathofexile.com/api/trade2/fetch/a,b,c,d,e,f,g,h,i,j?query=Q1&realm=poe2"
                    .to_owned(),
                body: None,
            }
        );
        assert!(
            client.fetch(Language::English, "Q1", &[]).is_ok(),
            "nothing to fetch, nothing sent"
        );
        assert_eq!(site.requests().len(), 1);
    }

    #[test]
    fn a_second_search_waits_ten_seconds_whatever_the_item() {
        let site = FakeSite::default();
        site.answer(200, LimitHeaders::default(), SEARCH);
        let mut client = TradeClient::with_transport(site.clone(), Refusal::default());
        let league = League::new("Standard");
        client
            .search_items(Language::English, &league, &ring())
            .expect("an answer");

        let exchange = client.search_exchange(Language::English, &league, "divine");

        assert!(matches!(exchange, Err(TradeError::Wait(_))));
        assert_eq!(site.requests().len(), 1, "not sent");
        let now = Instant::now();
        let next = client.next_search(SearchKind::Exchange, now);
        assert!(next + Duration::from_secs(1) > now + SEARCH_INTERVAL);
    }

    #[test]
    fn a_request_the_limits_dont_allow_isnt_sent() {
        let site = FakeSite::default();
        site.answer(
            200,
            limits("trade-fetch-request-limit", "1:4:10", "1:4:0"),
            r#"{"result": []}"#,
        );
        let mut client = TradeClient::with_transport(site.clone(), Refusal::default());
        let ids = ["a".to_owned()];
        client
            .fetch(Language::English, "Q1", &ids)
            .expect("an answer");

        let second = client.fetch(Language::English, "Q1", &ids);

        assert!(matches!(second, Err(TradeError::Wait(_))));
        assert_eq!(site.requests().len(), 1);
        // A search of items waits for its fetch too.
        let now = Instant::now();
        assert!(client.next_search(SearchKind::Items, now) > now);
        assert_eq!(client.next_search(SearchKind::Exchange, now), now);
    }

    #[test]
    fn a_refusal_waits_for_its_delay_without_retrying() {
        let site = FakeSite::default();
        let mut headers = limits("trade-search-request-limit", "5:10:60", "6:10:60");
        headers.retry_after = Some("60".to_owned());
        site.answer(429, headers, "");
        let mut client = TradeClient::with_transport(site.clone(), Refusal::default());

        let search = client.search_items(Language::English, &League::new("Standard"), &ring());

        let Err(TradeError::Refused(until)) = search else {
            panic!("refused: {search:?}");
        };
        assert!(until > Instant::now() + Duration::from_secs(55));
        assert_eq!(site.requests().len(), 1);
        assert!(client.next_search(SearchKind::Items, Instant::now()) >= until);
    }

    /// The security audit of 2026-11-10: the download of the item names shares this wait.
    #[test]
    fn a_refusal_makes_every_request_to_the_trade_site_wait() {
        for status in [429, 403] {
            let site = FakeSite::default();
            let mut headers = limits("trade-search-request-limit", "5:10:60", "6:10:60");
            headers.retry_after = Some("1800".to_owned());
            site.answer(status, headers, "");
            let refusal = Refusal::default();
            let mut client = TradeClient::with_transport(site.clone(), refusal.clone());
            let start = Instant::now();

            let search = client.search_items(Language::English, &League::new("Standard"), &ring());

            assert!(matches!(search, Err(TradeError::Refused(_))), "{search:?}");
            let until = refusal.until(start).expect("a shared wait");
            assert!(until >= start + Duration::from_mins(30), "{status}");
            // The other policies wait too: the same site refused the same address.
            assert!(client.next_search(SearchKind::Exchange, start) >= until);
            assert!(client.next_fetch(start) >= until);
            assert_eq!(site.requests().len(), 1, "{status}");
        }
    }

    #[test]
    fn a_refusal_without_delay_waits_a_minute() {
        let site = FakeSite::default();
        site.answer(403, LimitHeaders::default(), "");
        let refusal = Refusal::default();
        let mut client = TradeClient::with_transport(site, refusal.clone());
        let start = Instant::now();

        let search = client.search_items(Language::English, &League::new("Standard"), &ring());

        let Err(TradeError::Refused(until)) = search else {
            panic!("refused: {search:?}");
        };
        assert!(until >= start + Duration::from_mins(1));
        assert!(
            refusal
                .until(start)
                .is_some_and(|wait| wait >= start + Duration::from_mins(1))
        );
    }

    #[test]
    fn nothing_is_sent_during_the_wait_of_refused_item_names() {
        let site = FakeSite::default();
        let refusal = Refusal::default();
        let until = Instant::now() + Duration::from_hours(2);
        refusal.extend(until);
        let mut client = TradeClient::with_transport(site.clone(), refusal);
        let league = League::new("Standard");

        let items = client.search_items(Language::English, &league, &ring());
        let exchange = client.search_exchange(Language::English, &league, "divine");
        let fetch = client.fetch(Language::English, "Q1", &["a".to_owned()]);

        assert!(matches!(items, Err(TradeError::Wait(wait)) if wait == until));
        assert!(matches!(exchange, Err(TradeError::Wait(wait)) if wait == until));
        assert!(matches!(fetch, Err(TradeError::Wait(wait)) if wait == until));
        assert_eq!(site.requests(), []);
    }

    #[test]
    fn a_refusal_whose_body_cant_be_read_waits_all_the_same() {
        let site = FakeSite::default();
        let mut headers = limits("trade-search-request-limit", "5:10:60", "6:10:60");
        headers.retry_after = Some("120".to_owned());
        site.answers.lock().unwrap().push_back(Answer {
            status: 429,
            limits: headers,
            body: Err(ureq::Error::ConnectionFailed),
        });
        let mut client = TradeClient::with_transport(site, Refusal::default());

        let search = client.search_items(Language::English, &League::new("Standard"), &ring());

        let Err(TradeError::Refused(until)) = search else {
            panic!("refused: {search:?}");
        };
        assert!(until > Instant::now() + Duration::from_secs(115));
    }

    #[test]
    fn the_limits_of_an_answer_whose_body_cant_be_read_are_kept() {
        let site = FakeSite::default();
        site.answers.lock().unwrap().push_back(Answer {
            status: 200,
            limits: limits("trade-fetch-request-limit", "1:4:10", "1:4:0"),
            body: Err(ureq::Error::ConnectionFailed),
        });
        let mut client = TradeClient::with_transport(site, Refusal::default());

        let fetch = client.fetch(Language::English, "Q1", &["a".to_owned()]);

        assert!(matches!(
            fetch,
            Err(TradeError::Http(HttpError::Request(_)))
        ));
        assert!(client.next_fetch(Instant::now()) > Instant::now());
    }

    #[test]
    fn an_error_status_or_an_unexpected_answer_is_an_error() {
        let site = FakeSite::default();
        site.answer(400, LimitHeaders::default(), r#"{"error": {"code": 2}}"#);
        site.answer(200, LimitHeaders::default(), r#"{"result": "?"}"#);
        let mut client = TradeClient::with_transport(site, Refusal::default());
        let ids = ["a".to_owned()];

        assert!(matches!(
            client.fetch(Language::English, "Q1", &ids),
            Err(TradeError::Http(HttpError::Request(
                ureq::Error::StatusCode(400)
            )))
        ));
        assert!(matches!(
            client.fetch(Language::English, "Q1", &ids),
            Err(TradeError::Http(HttpError::Json(_)))
        ));
    }

    /// The network review of 2026-11-10: after a change of the game, the log only said "http
    /// status: 400".
    #[test]
    fn the_message_of_a_refused_search_is_read_as_one_short_line() {
        let body = r#"{"error": {"code": 2, "message": "Unknown item\nbase type"}}"#;

        assert_eq!(
            error_message(body).as_deref(),
            Some("Unknown item base type")
        );
        assert_eq!(error_message("<html>Bad Gateway</html>"), None);
        assert_eq!(error_message(r#"{"error": "no"}"#), None);
        let long = format!(r#"{{"error": {{"message": "{}"}}}}"#, "a".repeat(5_000));
        let length = error_message(&long).map(|message| message.chars().count());
        assert!(length.is_some_and(|length| length < 250), "{length:?}");
    }
}
