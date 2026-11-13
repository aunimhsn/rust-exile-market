//! The way requests reach the trade site: over the network with `ureq`, or to a fake trade
//! site in the tests, and the answers, whatever their status, with the headers of their
//! limits. Every request goes through [`Guarded`], which sends nothing during the wait of a
//! refusal.

use std::time::Instant;

use serde::de::DeserializeOwned;

use super::limits::{self, LimitHeaders, RuleHeaders};
use super::refusal::Refusal;
use crate::http::{self, HttpError};
use crate::logging;

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
    /// The status is a success (2xx).
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// The JSON of a success. An error status is an error, whose message is logged.
    pub fn into_json<A: DeserializeOwned>(self) -> Result<A, HttpError> {
        if !self.is_success() {
            self.log_error_message();
            return Err(HttpError::Request(ureq::Error::StatusCode(self.status)));
        }
        let body = self.body.map_err(HttpError::Request)?;
        serde_json::from_str(&body).map_err(HttpError::Json)
    }

    /// Writes to the log why the trade site answered with an error status, when it says so:
    /// after a change of the game, the filter it no longer knows is named there.
    fn log_error_message(&self) {
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

/// The only way to the trade site: nothing is sent during the wait of a refusal, and a
/// refusal (403, 429) starts one, for every request of the application, since all of them
/// reach the same site from the same address. Its fields are private, so that no request
/// can go around it.
pub(super) struct Guarded<T> {
    transport: T,
    /// The wait asked by a refusal, shared by the searches of offers and the download of the
    /// item names.
    refusal: Refusal,
}

/// What became of an attempt to send a request through [`Guarded`].
#[derive(Debug)]
pub(super) enum Attempt {
    /// Not sent: the trade site refused a request a moment ago, and nothing is sent before
    /// this time.
    Waiting(Instant),
    /// Sent, but no answer came.
    Unanswered(ureq::Error),
    /// The trade site refused it (403, 429): nothing is sent before `until`, nor after it
    /// while another refusal asks a longer wait.
    Refused { answer: Answer, until: Instant },
    /// An answer that isn't a refusal, of any other status.
    Answered(Answer),
}

impl Guarded<Network> {
    /// The trade site over the network, whose wait of a refusal is `refusal`: the only way
    /// out of this module to build a [`Network`].
    pub fn network(refusal: Refusal) -> Self {
        Self {
            transport: Network::new(),
            refusal,
        }
    }
}

impl<T: Transport> Guarded<T> {
    /// A fake trade site `transport` behind the guard, for the tests of its callers.
    #[cfg(test)]
    pub fn new(transport: T, refusal: Refusal) -> Self {
        Self { transport, refusal }
    }

    /// The end of the wait of a refusal, while it is after `now`.
    pub fn refused_until(&self, now: Instant) -> Option<Instant> {
        self.refusal.until(now)
    }

    /// Sends `request`, unless the wait of a refusal holds; a refusal starts one, of the delay
    /// of its `Retry-After`.
    pub fn send(&mut self, request: &Request) -> Attempt {
        if let Some(until) = self.refusal.until(Instant::now()) {
            return Attempt::Waiting(until);
        }
        let answer = match self.transport.send(request) {
            Ok(answer) => answer,
            Err(error) => return Attempt::Unanswered(error),
        };
        // A 403 too: the trade site turns the address away.
        if matches!(answer.status, 403 | 429) {
            let until = Instant::now() + limits::refusal_wait(&answer.limits);
            self.refusal.extend(until);
            return Attempt::Refused { answer, until };
        }
        Attempt::Answered(answer)
    }
}

/// The requests sent with `ureq`, which keeps no cookie (its `cookies` feature is off, see
/// `Cargo.toml`), and follows no redirection: each request is one request counted by the
/// limits, and a redirection is an error status.
pub(super) struct Network(ureq::Agent);

impl Network {
    fn new() -> Self {
        Self(http::agent())
    }
}

impl Transport for Network {
    fn send(&mut self, request: &Request) -> Result<Answer, ureq::Error> {
        let mut response = match &request.body {
            Some(body) => any_status(self.0.post(&request.url))
                .content_type("application/json")
                .send(body)?,
            None => any_status(self.0.get(&request.url)).call()?,
        };
        let limits = limit_headers(response.headers());
        let status = response.status().as_u16();
        // Of an error too, a short one: see `Answer::log_error_message`.
        let body = if (200..300).contains(&status) {
            http::read_text(response.body_mut())
        } else {
            http::read_error_text(response.body_mut())
        };
        Ok(Answer {
            status,
            limits,
            body,
        })
    }
}

/// An error status is an answer too, whose limits and message are read. The agent follows
/// no redirection (`http::agent`).
fn any_status<B>(request: ureq::RequestBuilder<B>) -> ureq::RequestBuilder<B> {
    request.config().http_status_as_error(false).build()
}

/// The headers of an answer the limits read: `X-Rate-Limit-Rules` names the rules, each with
/// its limits in `X-Rate-Limit-<rule>` and its counts in `X-Rate-Limit-<rule>-State`.
fn limit_headers(headers: &ureq::http::HeaderMap) -> LimitHeaders {
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
    LimitHeaders {
        policy: header("x-rate-limit-policy"),
        rules,
        retry_after: header("retry-after"),
    }
}

#[cfg(test)]
pub(super) mod tests {
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use super::*;
    use crate::trade::client::tests::SEARCH;
    use crate::trade::limits::{Limits, Policy};

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

    fn header_map(headers: &[(&'static str, &'static str)]) -> ureq::http::HeaderMap {
        headers
            .iter()
            .map(|&(name, value)| {
                (
                    ureq::http::HeaderName::from_static(name),
                    ureq::http::HeaderValue::from_static(value),
                )
            })
            .collect()
    }

    /// The headers of a search, as measured on 2026-10-23 (`docs/trade-site-usage.md`).
    const SEARCH_HEADERS: [(&str, &str); 4] = [
        ("x-rate-limit-policy", "trade-search-request-limit"),
        ("x-rate-limit-rules", "Ip"),
        (
            "x-rate-limit-ip",
            "5:10:60,15:60:300,30:300:1800,600:21600:3600",
        ),
        ("x-rate-limit-ip-state", "1:10:0,1:60:0,1:300:0,1:21600:0"),
    ];

    /// The network review of 2026-11-10: every test gave the client headers already read, so
    /// that a header renamed left the application without limits, and without a test failing.
    #[test]
    fn the_limits_are_read_from_the_headers_of_an_answer() {
        let read = limit_headers(&header_map(&SEARCH_HEADERS));

        assert_eq!(
            read,
            limits(
                "trade-search-request-limit",
                "5:10:60,15:60:300,30:300:1800,600:21600:3600",
                "1:10:0,1:60:0,1:300:0,1:21600:0"
            )
        );
        // The limits themselves read them: four windows, a search allowed at once.
        let mut known = Limits::default();
        let now = Instant::now();
        known.answered(Policy::Search, &read, now);
        assert_eq!(known.ready_at(Policy::Search, now), now);
    }

    /// The rule `Account` is the one of a signed-in session, which the application never has:
    /// here for a second rule, and one that lacks its counts.
    #[test]
    fn each_rule_is_read_with_its_own_headers() {
        let read = limit_headers(&header_map(&[
            ("x-rate-limit-rules", "Ip, Account,"),
            ("x-rate-limit-ip", "5:10:60"),
            ("x-rate-limit-ip-state", "5:10:60"),
            ("x-rate-limit-account", "3:5:60"),
            ("retry-after", "60"),
        ]));

        assert_eq!(
            read,
            LimitHeaders {
                policy: None,
                rules: vec![
                    RuleHeaders {
                        name: "Ip".to_owned(),
                        limits: "5:10:60".to_owned(),
                        state: "5:10:60".to_owned(),
                    },
                    RuleHeaders {
                        name: "Account".to_owned(),
                        limits: "3:5:60".to_owned(),
                        state: String::new(),
                    },
                ],
                retry_after: Some("60".to_owned()),
            }
        );
    }

    #[test]
    fn an_answer_without_the_headers_tells_no_rule() {
        let read = limit_headers(&header_map(&[("content-type", "application/json")]));

        assert_eq!(read, LimitHeaders::default());
    }

    /// The head of an answer of a local server with the headers of a search, written as the
    /// trade site writes them, and a session cookie as it sets one in each answer.
    fn search_head(status: &str) -> String {
        format!(
            "{status}\r\nSet-Cookie: POESESSID=abc; Path=/\r\n\
             X-Rate-Limit-Policy: trade-search-request-limit\r\nX-Rate-Limit-Rules: Ip\r\n\
             X-Rate-Limit-Ip: 5:10:60,15:60:300,30:300:1800,600:21600:3600\r\n\
             X-Rate-Limit-Ip-State: 1:10:0,1:60:0,1:300:0,1:21600:0"
        )
    }

    /// What stays on the network side of [`Transport`], through a real answer.
    #[test]
    fn a_request_over_the_network_reads_the_limits_and_sends_no_cookie_back() {
        use crate::http::tests::{header, local_agent, recording_server};

        let (url, _, heads) = recording_server(search_head("200 OK"), SEARCH.into());
        let mut network = Network(local_agent());
        let post = Request {
            url: url.clone(),
            body: Some(r#"{"query": {}}"#.to_owned()),
        };

        let answer = network.send(&post).expect("an answer");
        network
            .send(&Request { url, body: None })
            .expect("an answer");

        assert_eq!(answer.status, 200);
        assert_eq!(answer.limits, limit_headers(&header_map(&SEARCH_HEADERS)));
        assert_eq!(answer.body.expect("a body"), SEARCH);
        let heads = heads.lock().unwrap();
        assert!(heads[0][0].starts_with("POST "), "{:?}", heads[0]);
        assert_eq!(header(&heads[0], "content-type"), Some("application/json"));
        assert!(heads[1][0].starts_with("GET "), "{:?}", heads[1]);
        for request in heads.iter() {
            assert_eq!(header(request, "cookie"), None, "{request:?}");
            assert_eq!(
                header(request, "user-agent"),
                Some(http::user_agent().as_str())
            );
        }
    }

    #[test]
    fn a_refusal_over_the_network_is_an_answer_with_its_delay_and_its_message() {
        use crate::http::tests::{local_agent, recording_server};

        let body = r#"{"error": {"code": 3, "message": "Rate limit exceeded"}}"#;
        let head = format!(
            "{}\r\nRetry-After: 60",
            search_head("429 Too Many Requests")
        );
        let (url, _, _) = recording_server(head, body.into());

        let answer = Network(local_agent())
            .send(&Request { url, body: None })
            .expect("an answer, not an error");

        assert_eq!(answer.status, 429);
        assert_eq!(answer.limits.retry_after.as_deref(), Some("60"));
        assert_eq!(answer.limits.rules.len(), 1);
        assert_eq!(answer.body.expect("a body"), body);
    }

    #[test]
    fn a_redirection_over_the_network_is_an_answer_and_isnt_followed() {
        use crate::http::tests::{local_agent, server};
        use std::sync::atomic::Ordering;

        let (elsewhere, followed) = server("200 OK", SEARCH.into());
        let (url, requests) = server(format!("302 Found\r\nLocation: {elsewhere}"), Vec::new());
        let mut network = Network(local_agent());

        let post = network.send(&Request {
            url: url.clone(),
            body: Some("{}".to_owned()),
        });
        let get = network.send(&Request { url, body: None });

        assert_eq!(post.expect("an answer").status, 302);
        assert_eq!(get.expect("an answer").status, 302);
        assert_eq!(requests.load(Ordering::SeqCst), 2);
        assert_eq!(followed.load(Ordering::SeqCst), 0);
    }

    /// A request to the fake site.
    fn get() -> Request {
        Request {
            url: "https://www.pathofexile.com/api/trade2/data/items".to_owned(),
            body: None,
        }
    }

    /// The headers of a refusal that asks to wait `seconds`.
    pub fn retry_after(seconds: &str) -> LimitHeaders {
        LimitHeaders {
            retry_after: Some(seconds.to_owned()),
            ..LimitHeaders::default()
        }
    }

    #[test]
    fn a_refusal_stops_the_requests_that_follow() {
        for status in [429, 403] {
            let site = FakeSite::default();
            site.answer(status, retry_after("600"), "");
            let refusal = Refusal::default();
            let mut guarded = Guarded::new(site.clone(), refusal.clone());
            let start = Instant::now();

            let refused = guarded.send(&get());
            let next = guarded.send(&get());

            let Attempt::Refused { answer, until } = refused else {
                panic!("a refusal: {refused:?}");
            };
            assert_eq!(answer.status, status);
            assert!(until >= start + Duration::from_mins(10), "{status}");
            assert!(until < start + Duration::from_mins(11), "{status}");
            assert_eq!(refusal.until(start), Some(until), "{status}");
            assert!(
                matches!(next, Attempt::Waiting(wait) if wait == until),
                "{next:?}"
            );
            assert_eq!(site.requests().len(), 1, "{status}");
        }
    }

    #[test]
    fn nothing_is_sent_during_the_wait_of_a_refusal_of_another_request() {
        let site = FakeSite::default();
        let refusal = Refusal::default();
        let until = Instant::now() + Duration::from_mins(5);
        refusal.extend(until);
        let mut guarded = Guarded::new(site.clone(), refusal);

        let sent = guarded.send(&get());

        assert!(
            matches!(sent, Attempt::Waiting(wait) if wait == until),
            "{sent:?}"
        );
        assert_eq!(guarded.refused_until(Instant::now()), Some(until));
        assert_eq!(site.requests(), []);
    }

    #[test]
    fn an_error_status_or_no_answer_isnt_a_refusal() {
        let site = FakeSite::default();
        site.answer(400, LimitHeaders::default(), "{}");
        let refusal = Refusal::default();
        let mut guarded = Guarded::new(site.clone(), refusal.clone());

        let error_status = guarded.send(&get());
        // The fake site has no answer left: a request without answer.
        let unanswered = guarded.send(&get());

        assert!(
            matches!(&error_status, Attempt::Answered(answer) if answer.status == 400),
            "{error_status:?}"
        );
        assert!(
            matches!(unanswered, Attempt::Unanswered(_)),
            "{unanswered:?}"
        );
        assert_eq!(refusal.until(Instant::now()), None);
        assert_eq!(site.requests().len(), 2);
    }
}
