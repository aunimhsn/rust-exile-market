//! What the clients of the web services share: an agent that identifies the application and
//! gives up the requests that hang, the pace of the requests, and the JSON of the answers.

use std::fmt::{self, Write};
use std::io::Read;
use std::thread;
use std::time::{Duration, Instant};

use serde::de::{DeserializeOwned, Error as _};
use serde::{Deserialize, Deserializer};
use serde_json::Value;

use crate::logging;

/// A request that hangs is given up, and tried again later.
const TIMEOUT: Duration = Duration::from_secs(30);
/// A server that doesn't answer is given up sooner.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// The longest answer read, once decompressed: the largest ones (the stats of the trade site,
/// a page of prices) are a few megabytes.
const MAX_BODY: u64 = 16 * 1024 * 1024;

/// Why a web service couldn't be read.
#[derive(Debug)]
pub enum HttpError {
    /// No answer, or an error status (e.g. 429 when its limits are reached).
    Request(ureq::Error),
    /// An answer that isn't the expected JSON: the service changed.
    Json(serde_json::Error),
}

impl HttpError {
    /// The service answered, with an error or something else than the expected JSON: the same
    /// request would likely fail the same way, so the next attempt waits longer and longer
    /// ([`Backoff`]). Without an answer (no network, a request that hangs), the next attempt
    /// can come soon; a request that hangs may have reached the service all the same, which
    /// this error alone can't tell: a client that sends several requests in a row counts a
    /// failure after its first answers as answered too (`after_answers` in its error).
    #[must_use]
    pub fn answered(&self) -> bool {
        match self {
            Self::Request(error) => matches!(
                error,
                ureq::Error::StatusCode(_) | ureq::Error::BodyExceedsLimit(_)
            ),
            Self::Json(_) => true,
        }
    }
}

impl fmt::Display for HttpError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Request(error) => write!(formatter, "request failed: {error}"),
            Self::Json(error) => write!(formatter, "unexpected answer: {error}"),
        }
    }
}

impl std::error::Error for HttpError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Request(error) => Some(error),
            Self::Json(error) => Some(error),
        }
    }
}

impl From<ureq::Error> for HttpError {
    fn from(error: ureq::Error) -> Self {
        Self::Request(error)
    }
}

impl From<serde_json::Error> for HttpError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

/// An agent that sends the name of the application, its version and the address of its
/// repository with each request, as the services ask. It only speaks HTTPS and follows no
/// redirection: every address is written in the code, so that a request never leaves for
/// another host, nor in clear text. A redirection is then an answer of its status (ureq
/// gives it as a success): [`get_text`] makes it an error.
#[must_use]
pub fn agent() -> ureq::Agent {
    agent_over(Scheme::HttpsOnly)
}

/// Whether an agent refuses the addresses in clear text: always, but for the local server of
/// the tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scheme {
    HttpsOnly,
    #[cfg(test)]
    AlsoHttp,
}

fn agent_over(scheme: Scheme) -> ureq::Agent {
    let config = ureq::Agent::config_builder()
        .https_only(scheme == Scheme::HttpsOnly)
        .max_redirects(0)
        .timeout_global(Some(TIMEOUT))
        .timeout_connect(Some(CONNECT_TIMEOUT))
        .user_agent(user_agent())
        .build();
    ureq::Agent::new_with_config(config)
}

/// What the application says it is in each request: its name, its version and the address
/// of its repository.
#[must_use]
pub fn user_agent() -> String {
    format!(
        "{}/{} (+{})",
        crate::FOLDER_NAME,
        crate::VERSION,
        env!("CARGO_PKG_REPOSITORY")
    )
}

/// The text of the answer of `url` with the parameters `query`.
pub fn get_text(
    agent: &ureq::Agent,
    url: &str,
    query: &[(&str, &str)],
) -> Result<String, HttpError> {
    let mut response = agent.get(url).query_pairs(query.iter().copied()).call()?;
    let status = response.status().as_u16();
    // Not followed (see `agent`), and not the answer asked for.
    if (300..400).contains(&status) {
        return Err(ureq::Error::StatusCode(status).into());
    }
    Ok(read_text(response.body_mut())?)
}

/// The longest body read of an error status: a line of JSON that says why.
const MAX_ERROR_BODY: u64 = 64 * 1024;

/// The text of the answer of an error status, refused past a few kilobytes: only a short
/// message is read in it.
pub fn read_error_text(body: &mut ureq::Body) -> Result<String, ureq::Error> {
    read_text_up_to(body, MAX_ERROR_BODY)
}

/// The text of an answer, refused past `MAX_BODY`. The limit of `Body::read_to_string` counts
/// the bytes received, before they are decompressed: a few megabytes of gzip could fill the
/// memory. As there, a byte that isn't UTF-8 is read as the replacement character.
pub fn read_text(body: &mut ureq::Body) -> Result<String, ureq::Error> {
    read_text_up_to(body, MAX_BODY)
}

fn read_text_up_to(body: &mut ureq::Body, limit: u64) -> Result<String, ureq::Error> {
    let mut bytes = Vec::new();
    body.as_reader()
        .take(limit.saturating_add(1))
        .read_to_end(&mut bytes)?;
    if u64::try_from(bytes.len()).is_ok_and(|length| length <= limit) {
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    } else {
        Err(ureq::Error::BodyExceedsLimit(limit))
    }
}

/// Each element of a list read on its own: one the application can't read is left out, so
/// that a single odd item doesn't lose a whole answer. An error when more than half of them
/// can't be read, since the service changed its answer then: a list with a few elements left
/// would replace the complete one of the cache, or say "nothing" instead.
pub fn read_each<T: DeserializeOwned>(values: Vec<Value>) -> Result<Vec<T>, serde_json::Error> {
    let keyed = values.into_iter().map(|value| ((), value)).collect();
    Ok(read_each_keyed(keyed)?
        .into_iter()
        .map(|((), element)| element)
        .collect())
}

/// [`read_each`] for the values of a map, each kept with its key.
pub fn read_each_keyed<K, T: DeserializeOwned>(
    values: Vec<(K, Value)>,
) -> Result<Vec<(K, T)>, serde_json::Error> {
    let total = values.len();
    let mut read = Vec::with_capacity(total);
    let mut first = None;
    for (key, value) in values {
        match serde_json::from_value(value) {
            Ok(element) => read.push((key, element)),
            Err(error) => {
                first.get_or_insert(error);
            }
        }
    }
    match first {
        Some(error) if read.len() * 2 < total => Err(error),
        Some(error) => {
            log::warn!(
                "{} of the {total} elements of an answer can't be read, the first one: {}",
                total - read.len(),
                logging::plain(&error.to_string())
            );
            Ok(read)
        }
        None => Ok(read),
    }
}

/// [`read_each`] for a list inside an answer: `#[serde(deserialize_with = "http::each")]`.
pub fn each<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned,
{
    read_each(Vec::<Value>::deserialize(deserializer)?).map_err(D::Error::custom)
}

/// `text` escaped for a segment of a path or a value of a query, e.g. a league name with
/// spaces: everything but the letters, the digits and `-._~` is written as `%XX`.
#[must_use]
pub fn encode(text: &str) -> String {
    let mut encoded = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
            encoded.push(char::from(byte));
        } else {
            // Writing to a `String` never fails.
            let _ = write!(encoded, "%{byte:02X}");
        }
    }
    encoded
}

/// Spaces the requests to a service.
#[derive(Debug)]
pub struct Pace {
    interval: Duration,
    last: Option<Instant>,
}

impl Pace {
    /// At most one request every `interval`.
    #[must_use]
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            last: None,
        }
    }

    /// Waits until `interval` has passed since the last request, and counts the next one
    /// from now.
    pub fn wait(&mut self) {
        if let Some(last) = self.last {
            thread::sleep(self.interval.saturating_sub(last.elapsed()));
        }
        // Counted even when it fails: a refused request counts in the limits too.
        self.last = Some(Instant::now());
    }
}

/// The wait after a failure the service answered ([`HttpError::answered`]): `first`, then
/// doubled at each failure in a row, `longest` at most. A changed answer fails the same way
/// at each attempt: asking again as often would only cost the service requests.
#[derive(Debug, Clone, Copy)]
pub struct Backoff {
    first: Duration,
    longest: Duration,
    failures: u32,
}

impl Backoff {
    #[must_use]
    pub fn new(first: Duration, longest: Duration) -> Self {
        Self {
            first,
            longest,
            failures: 0,
        }
    }

    /// The wait before the next attempt, after one more failure.
    pub fn failed(&mut self) -> Duration {
        let factor = 2_u32.saturating_pow(self.failures);
        self.failures = self.failures.saturating_add(1);
        self.first.saturating_mul(factor).min(self.longest)
    }

    /// The service answered as expected: the next failure waits `first` again.
    pub fn succeeded(&mut self) {
        self.failures = 0;
    }
}

#[cfg(test)]
pub mod tests {
    use std::io::{BufRead, BufReader, Write as _};
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    use flate2::Compression;
    use flate2::write::GzEncoder;

    use super::*;

    /// The address of a local server that answers each request with the status line and the
    /// headers `head` and with `body`, and the number of requests it got.
    pub fn server(head: impl Into<String>, body: Vec<u8>) -> (String, Arc<AtomicUsize>) {
        let (address, requests, _) = recording_server(head, body);
        (address, requests)
    }

    /// The lines of the requests a local server got, a request each: the request line, then
    /// its headers, their names in lowercase.
    pub type Heads = Arc<Mutex<Vec<Vec<String>>>>;

    /// [`server`], which also keeps the lines of each request.
    pub fn recording_server(
        head: impl Into<String>,
        body: Vec<u8>,
    ) -> (String, Arc<AtomicUsize>, Heads) {
        let head = head.into();
        let listener = TcpListener::bind("127.0.0.1:0").expect("a local port");
        let address = listener.local_addr().expect("its address");
        let requests = Arc::new(AtomicUsize::new(0));
        let counted = Arc::clone(&requests);
        let heads = Heads::default();
        let kept = Arc::clone(&heads);
        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { return };
                let mut request = BufReader::new(stream.try_clone().expect("the same stream"));
                let mut lines = Vec::new();
                let mut line = String::new();
                while request.read_line(&mut line).is_ok_and(|read| read > 2) {
                    // The name of a header in lowercase, its value as it was sent.
                    let sent = line.trim_end();
                    lines.push(match sent.split_once(':') {
                        Some((name, value)) => format!("{}:{value}", name.to_ascii_lowercase()),
                        None => sent.to_owned(),
                    });
                    line.clear();
                }
                // The body of a `POST`, read whole before the answer.
                let length = header(&lines, "content-length").and_then(|text| text.parse().ok());
                let mut body_sent = vec![0; length.unwrap_or(0)];
                if request.read_exact(&mut body_sent).is_err() {
                    continue;
                }
                // Before the count, which the tests wait for.
                kept.lock().expect("the lines").push(lines);
                counted.fetch_add(1, Ordering::SeqCst);
                let head = format!(
                    "HTTP/1.1 {head}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                // The reader may hang up before the end: it refuses a long answer.
                let _ = stream.write_all(head.as_bytes());
                let _ = stream.write_all(&body);
            }
        });
        (format!("http://{address}/"), requests, heads)
    }

    /// The address of a local server that answers with `text`, compressed.
    fn gzip_server(text: &[u8]) -> String {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
        encoder.write_all(text).expect("a compressed text");
        let body = encoder.finish().expect("a compressed text");
        server("200 OK\r\nContent-Encoding: gzip", body).0
    }

    /// The agent of the application, but for a local server, which doesn't speak HTTPS.
    pub fn local_agent() -> ureq::Agent {
        agent_over(Scheme::AlsoHttp)
    }

    /// The header `name` (in lowercase) of a request kept by [`recording_server`].
    pub fn header<'a>(request: &'a [String], name: &str) -> Option<&'a str> {
        request.iter().find_map(|line| {
            let (key, value) = line.split_once(':')?;
            (key == name).then_some(value.trim())
        })
    }

    /// The code review of the fixes of the audit: ureq gives a redirection it doesn't follow
    /// as a success.
    #[test]
    fn a_redirection_is_an_error_status_and_isnt_followed() {
        let (elsewhere, followed) = server("200 OK", b"[1]".to_vec());
        let (url, requests) = server(format!("302 Found\r\nLocation: {elsewhere}"), Vec::new());

        let answer = get_text(&local_agent(), &url, &[]);

        assert!(
            matches!(
                answer,
                Err(HttpError::Request(ureq::Error::StatusCode(302)))
            ),
            "{answer:?}"
        );
        assert_eq!(requests.load(Ordering::SeqCst), 1);
        assert_eq!(followed.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn an_answer_that_isnt_utf8_is_read_with_the_replacement_character() {
        let (url, _) = server("200 OK", b"[\"\xff\"]".to_vec());

        let answer = get_text(&local_agent(), &url, &[]);

        assert_eq!(answer.expect("a text"), "[\"\u{fffd}\"]");
    }

    /// The security audit of 2026-11-10: the limit of ureq counts the compressed bytes.
    #[test]
    fn an_answer_is_refused_past_the_limit_once_decompressed() {
        const LIMIT: u64 = 64 * 1024;
        // 8 MB of zeros: about 8 KB of gzip, far under the limit.
        let url = gzip_server(&vec![b'0'; 8 * 1024 * 1024]);
        let mut response = ureq::Agent::new_with_defaults()
            .get(&url)
            .call()
            .expect("an answer");

        let text = read_text_up_to(response.body_mut(), LIMIT);

        assert!(
            matches!(text, Err(ureq::Error::BodyExceedsLimit(LIMIT))),
            "{text:?}"
        );
    }

    #[test]
    fn an_answer_up_to_the_limit_is_read_decompressed() {
        let url = gzip_server(b"[1, 2]");
        let mut response = ureq::Agent::new_with_defaults()
            .get(&url)
            .call()
            .expect("an answer");

        let text = read_text_up_to(response.body_mut(), 6).expect("a short answer");

        assert_eq!(text, "[1, 2]");
    }

    #[test]
    fn the_agent_only_speaks_https_and_follows_no_redirection() {
        let agent = agent();

        assert_eq!(agent.config().max_redirects(), 0);
        let answer = get_text(&agent, "http://127.0.0.1:9/", &[]);
        assert!(
            matches!(
                answer,
                Err(HttpError::Request(ureq::Error::RequireHttpsOnly(_)))
            ),
            "{answer:?}"
        );
    }

    /// The network review of 2026-11-10: both usage documents ask for it, and nothing checked
    /// what is really sent.
    #[test]
    fn each_request_names_the_application_its_version_and_its_repository() {
        let (url, _, heads) = recording_server("200 OK", b"[1]".to_vec());

        get_text(&local_agent(), &url, &[]).expect("an answer");

        let heads = heads.lock().unwrap();
        let sent = header(&heads[0], "user-agent").expect("a User-Agent");
        assert_eq!(sent, user_agent());
        assert_eq!(
            user_agent(),
            format!(
                "exile-market/{} (+https://github.com/aunimhsn/rust-exile-market)",
                env!("CARGO_PKG_VERSION")
            )
        );
    }

    /// The trade site sets a `POESESSID` cookie in each answer, which must never be sent
    /// back (`docs/trade-site-usage.md`). Only the `cookies` feature of ureq being off
    /// keeps it so, and the features of a crate are those every crate of the build asks
    /// for: a dependency turning it on would send the cookie without a line changed here.
    #[test]
    fn a_cookie_set_by_an_answer_is_never_sent_back() {
        let (url, requests, heads) = recording_server(
            "200 OK\r\nSet-Cookie: POESESSID=abc; Path=/",
            b"[1]".to_vec(),
        );
        let agent = local_agent();

        for _ in 0..2 {
            get_text(&agent, &url, &[]).expect("an answer");
        }

        assert_eq!(requests.load(Ordering::SeqCst), 2);
        for request in heads.lock().unwrap().iter() {
            assert_eq!(header(request, "cookie"), None, "{request:?}");
        }
    }

    /// Only "not sooner" is checked: "at once" would depend on how busy the machine is. The
    /// time is counted from before the first request, which the pace counts from: an instant
    /// taken after each wait could come late, and the gap to the next one look too short.
    #[test]
    fn the_requests_are_spaced_by_the_interval_at_least() {
        const INTERVAL: Duration = Duration::from_millis(20);
        let mut pace = Pace::new(INTERVAL);
        let start = Instant::now();

        pace.wait();
        pace.wait();
        assert!(start.elapsed() >= INTERVAL);
        pace.wait();
        assert!(start.elapsed() >= INTERVAL * 2);
    }

    #[test]
    fn an_error_status_or_an_unexpected_answer_is_an_answer_of_the_service() {
        let json = serde_json::from_str::<u8>("{").expect_err("not a number");

        assert!(HttpError::Request(ureq::Error::StatusCode(500)).answered());
        assert!(HttpError::Request(ureq::Error::BodyExceedsLimit(1)).answered());
        assert!(HttpError::Json(json).answered());
        assert!(!HttpError::Request(ureq::Error::HostNotFound).answered());
        assert!(!HttpError::Request(ureq::Error::ConnectionFailed).answered());
        assert!(!HttpError::Request(ureq::Error::Timeout(ureq::Timeout::Global)).answered());
    }

    #[test]
    fn the_wait_after_an_answered_failure_doubles_up_to_the_longest() {
        let minute = Duration::from_mins(1);
        let mut backoff = Backoff::new(minute, Duration::from_hours(1));

        let waits: Vec<u64> = (0..8).map(|_| backoff.failed().as_secs() / 60).collect();

        assert_eq!(waits, [1, 2, 4, 8, 16, 32, 60, 60]);
    }

    #[test]
    fn the_wait_never_overflows_after_many_failures() {
        let mut backoff = Backoff::new(Duration::from_mins(1), Duration::from_hours(1));

        let last = (0..100).map(|_| backoff.failed()).last();

        assert_eq!(last, Some(Duration::from_hours(1)));
    }

    #[test]
    fn a_success_starts_the_wait_over() {
        let mut backoff = Backoff::new(Duration::from_mins(1), Duration::from_hours(1));
        backoff.failed();
        backoff.failed();

        backoff.succeeded();

        assert_eq!(backoff.failed(), Duration::from_mins(1));
    }

    #[test]
    fn an_element_that_cant_be_read_is_left_out_of_its_list() {
        let values = vec![Value::from(1), Value::from("two"), Value::from(3)];

        let read = read_each::<u8>(values).expect("two numbers");

        assert_eq!(read, [1, 3]);
    }

    /// The code review of the quality fixes: a page with one price left in 250 replaced the
    /// complete prices of the cache.
    #[test]
    fn a_list_more_than_half_of_which_cant_be_read_is_an_error() {
        let list = |numbers: usize, texts: usize| -> Vec<Value> {
            let numbers = (0..numbers).map(|_| Value::from(1));
            numbers
                .chain((0..texts).map(|_| Value::from("odd")))
                .collect()
        };

        assert_eq!(read_each::<u8>(list(2, 2)).expect("half").len(), 2);
        assert_eq!(read_each::<u8>(list(125, 125)).expect("half").len(), 125);
        assert!(read_each::<u8>(list(1, 2)).is_err());
        assert!(read_each::<u8>(list(1, 249)).is_err());
    }

    #[test]
    fn the_values_of_a_map_keep_their_keys() {
        let values = vec![
            ("a", Value::from(1)),
            ("b", Value::Null),
            ("c", Value::from(3)),
        ];

        let read = read_each_keyed::<_, u8>(values).expect("two numbers");

        assert_eq!(read, [("a", 1), ("c", 3)]);
    }

    /// A list of another shape isn't an empty list: "no offer" would be shown for it.
    #[test]
    fn a_list_none_of_whose_elements_can_be_read_is_an_error() {
        let values = vec![Value::from("one"), Value::from("two")];

        assert!(read_each::<u8>(values).is_err());
        assert_eq!(
            read_each::<u8>(Vec::new()).expect("an empty list"),
            [0_u8; 0]
        );
    }

    #[test]
    fn a_list_inside_an_answer_leaves_out_what_cant_be_read() {
        #[derive(Deserialize)]
        struct Answer {
            #[serde(deserialize_with = "each")]
            result: Vec<u8>,
        }

        let answer: Answer = serde_json::from_str(r#"{"result": [1, null, 3]}"#).expect("a list");

        assert_eq!(answer.result, [1, 3]);
        assert!(serde_json::from_str::<Answer>(r#"{"result": [null]}"#).is_err());
        assert!(serde_json::from_str::<Answer>(r#"{"result": 1}"#).is_err());
    }

    #[test]
    fn a_text_is_escaped_for_a_url() {
        assert_eq!(encode("HC Forbidden Rites"), "HC%20Forbidden%20Rites");
        assert_eq!(
            encode("Rise of the Abyssal (PL1)"),
            "Rise%20of%20the%20Abyssal%20%28PL1%29"
        );
        assert_eq!(encode(r#"{"a":"é"}"#), "%7B%22a%22%3A%22%C3%A9%22%7D");
    }
}
