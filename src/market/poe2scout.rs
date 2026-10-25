//! The public API of poe2scout, the price database of the Path of Exile 2 economy.
//!
//! The rules of its use (pace, identification) are in `roadmap.md` (poe2scout usage).

use std::fmt;
use std::time::Duration;

use serde::Deserialize;
use serde::de::DeserializeOwned;

use super::PriceDatabase;
use crate::league::League;

const BASE_URL: &str = "https://api.poe2scout.com/poe2";
/// A request that hangs is given up, and tried again later.
const TIMEOUT: Duration = Duration::from_secs(30);
/// A server that doesn't answer is given up sooner.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Why the price database couldn't be read.
#[derive(Debug)]
pub enum FetchError {
    /// No answer, or an error status (e.g. 429 when its limits are reached).
    Http(ureq::Error),
    /// An answer that isn't the expected JSON: the API changed.
    Json(serde_json::Error),
    /// No current league in the answer: the database is wrong for a moment, and the league of
    /// the player must not be changed for that.
    NoCurrentLeague,
}

impl fmt::Display for FetchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Http(error) => write!(formatter, "request failed: {error}"),
            Self::Json(error) => write!(formatter, "unexpected answer: {error}"),
            Self::NoCurrentLeague => formatter.write_str("no current league in the answer"),
        }
    }
}

impl std::error::Error for FetchError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Http(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::NoCurrentLeague => None,
        }
    }
}

impl From<ureq::Error> for FetchError {
    fn from(error: ureq::Error) -> Self {
        Self::Http(error)
    }
}

impl From<serde_json::Error> for FetchError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

/// A league in the answer of `/Leagues`.
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ApiLeague {
    value: String,
    is_current: bool,
}

/// The client of poe2scout, which identifies the application in each request.
pub struct Poe2Scout {
    agent: ureq::Agent,
}

impl Poe2Scout {
    #[must_use]
    pub fn new() -> Self {
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
        Self {
            agent: ureq::Agent::new_with_config(config),
        }
    }

    fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, FetchError> {
        let mut response = self.agent.get(format!("{BASE_URL}{path}")).call()?;
        let text = response.body_mut().read_to_string()?;
        Ok(serde_json::from_str(&text)?)
    }
}

impl PriceDatabase for Poe2Scout {
    fn leagues(&mut self) -> Result<Vec<League>, FetchError> {
        let leagues = current_leagues(self.get("/Leagues")?);
        if leagues.is_empty() {
            return Err(FetchError::NoCurrentLeague);
        }
        Ok(leagues)
    }
}

/// The current leagues of the answer, in its order.
fn current_leagues(leagues: Vec<ApiLeague>) -> Vec<League> {
    leagues
        .into_iter()
        .filter(|league| league.is_current)
        .map(|league| League::new(league.value))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn an_answer_without_the_expected_fields_is_an_error() {
        let parsed: Result<Vec<ApiLeague>, _> = serde_json::from_str(r#"[{"Name": "x"}]"#);

        assert!(parsed.is_err());
    }
}
