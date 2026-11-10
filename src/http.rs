//! What the clients of the web services share: an agent that identifies the application and
//! gives up the requests that hang, the pace of the requests, and the JSON of the answers.

use std::fmt::{self, Write};
use std::io::Read;
use std::thread;
use std::time::{Duration, Instant};

use serde::de::DeserializeOwned;

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
/// gives it as a success): [`get_json`] makes it an error.
#[must_use]
pub fn agent() -> ureq::Agent {
    let config = ureq::Agent::config_builder()
        .https_only(true)
        .max_redirects(0)
        .timeout_global(Some(TIMEOUT))
        .timeout_connect(Some(CONNECT_TIMEOUT))
        .user_agent(format!(
            "{}/{} (+{})",
            crate::FOLDER_NAME,
            crate::VERSION,
            env!("CARGO_PKG_REPOSITORY")
        ))
        .build();
    ureq::Agent::new_with_config(config)
}

/// The JSON answer of `url` with the parameters `query`.
pub fn get_json<T: DeserializeOwned>(
    agent: &ureq::Agent,
    url: &str,
    query: &[(&str, &str)],
) -> Result<T, HttpError> {
    let mut response = agent.get(url).query_pairs(query.iter().copied()).call()?;
    let status = response.status().as_u16();
    // Not followed (see `agent`), and not the JSON asked for.
    if (300..400).contains(&status) {
        return Err(ureq::Error::StatusCode(status).into());
    }
    let text = read_text(response.body_mut())?;
    Ok(serde_json::from_str(&text)?)
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

#[cfg(test)]
pub mod tests {
    use std::io::{BufRead, BufReader, Write as _};
    use std::net::TcpListener;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use flate2::Compression;
    use flate2::write::GzEncoder;

    use super::*;

    /// The address of a local server that answers each request with the status line and the
    /// headers `head` and with `body`, and the number of requests it got.
    pub fn server(head: impl Into<String>, body: Vec<u8>) -> (String, Arc<AtomicUsize>) {
        let head = head.into();
        let listener = TcpListener::bind("127.0.0.1:0").expect("a local port");
        let address = listener.local_addr().expect("its address");
        let requests = Arc::new(AtomicUsize::new(0));
        let counted = Arc::clone(&requests);
        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { return };
                counted.fetch_add(1, Ordering::SeqCst);
                let mut request = BufReader::new(stream.try_clone().expect("the same stream"));
                let mut line = String::new();
                while request.read_line(&mut line).is_ok_and(|read| read > 2) {
                    line.clear();
                }
                let head = format!(
                    "HTTP/1.1 {head}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                // The reader may hang up before the end: it refuses a long answer.
                let _ = stream.write_all(head.as_bytes());
                let _ = stream.write_all(&body);
            }
        });
        (format!("http://{address}/"), requests)
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
        let config = ureq::Agent::config_builder()
            .max_redirects(0)
            .timeout_global(Some(Duration::from_secs(10)))
            .build();
        ureq::Agent::new_with_config(config)
    }

    /// The code review of the fixes of the audit: ureq gives a redirection it doesn't follow
    /// as a success.
    #[test]
    fn a_redirection_is_an_error_status_and_isnt_followed() {
        let (elsewhere, followed) = server("200 OK", b"[1]".to_vec());
        let (url, requests) = server(format!("302 Found\r\nLocation: {elsewhere}"), Vec::new());

        let answer = get_json::<Vec<u8>>(&local_agent(), &url, &[]);

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

        let answer = get_json::<Vec<String>>(&local_agent(), &url, &[]);

        assert_eq!(answer.expect("a text"), ["\u{fffd}"]);
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
        let answer = get_json::<Vec<u8>>(&agent, "http://127.0.0.1:9/", &[]);
        assert!(
            matches!(
                answer,
                Err(HttpError::Request(ureq::Error::RequireHttpsOnly(_)))
            ),
            "{answer:?}"
        );
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
