//! What the clients of the web services share: an agent that identifies the application and
//! gives up the requests that hang, the pace of the requests, and the JSON of the answers.

use std::fmt::{self, Write};
use std::thread;
use std::time::{Duration, Instant};

use serde::de::DeserializeOwned;

/// A request that hangs is given up, and tried again later.
const TIMEOUT: Duration = Duration::from_secs(30);
/// A server that doesn't answer is given up sooner.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

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
/// repository with each request, as the services ask.
#[must_use]
pub fn agent() -> ureq::Agent {
    let config = ureq::Agent::config_builder()
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
    let text = response.body_mut().read_to_string()?;
    Ok(serde_json::from_str(&text)?)
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
mod tests {
    use super::*;

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
