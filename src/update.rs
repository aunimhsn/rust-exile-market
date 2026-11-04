//! The update notice: at launch, the latest release of the application on GitHub, once, and
//! whether to offer it to the player.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::http::{self, HttpError};
use crate::league::Leagues;
use crate::worker::{OnDrop, Worker};

/// The repository of the application, e.g. `https://github.com/aunimhsn/rust-exile-market`.
const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
const GITHUB: &str = "https://github.com/";
const GITHUB_API: &str = "https://api.github.com/repos/";

/// A version of the application: three numbers, as the tags of the releases write them
/// (`v0.2.0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    major: u64,
    minor: u64,
    patch: u64,
}

impl Version {
    /// `0.2.0` or `v0.2.0`; none for another shape (a pre-release `0.2.0-beta`, a fourth number).
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.strip_prefix('v').unwrap_or(text);
        let mut numbers = text.split('.').map(|number| {
            // `u64::from_str` takes a leading `+`.
            if number.is_empty() || !number.bytes().all(|byte| byte.is_ascii_digit()) {
                return None;
            }
            number.parse().ok()
        });
        let version = Self {
            major: numbers.next()??,
            minor: numbers.next()??,
            patch: numbers.next()??,
        };
        numbers.next().is_none().then_some(version)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// A release newer than the running version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub version: Version,
    /// Its page on GitHub, which the player downloads it from.
    pub page: String,
}

/// The league of the moment as the application knows it, which brings back the bubble of an
/// update the player closed once it changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeagueOfTheMoment<'a> {
    /// The leagues aren't known yet.
    Unknown,
    /// Between two leagues: none has started.
    Between,
    /// The name of the first current league (softcore and hardcore alike).
    League(&'a str),
}

impl<'a> LeagueOfTheMoment<'a> {
    /// The league of the moment among `leagues`, once known.
    #[must_use]
    pub fn of(leagues: Option<&'a Leagues>) -> Self {
        match leagues {
            None => Self::Unknown,
            Some(leagues) => leagues.of_the_moment().map_or(Self::Between, Self::League),
        }
    }
}

/// The bubble of an update the player closed: its version, and the league of the moment then.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClosedUpdate {
    pub version: String,
    /// The league of the moment when it was closed: its name, empty between two leagues; none
    /// when the leagues weren't known yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub league: Option<String>,
}

impl ClosedUpdate {
    /// Closed for `release` during `league`.
    #[must_use]
    pub fn new(release: &Release, league: LeagueOfTheMoment) -> Self {
        Self {
            version: release.version.to_string(),
            league: Self::stored(league),
        }
    }

    /// How `league` is saved: an empty name between two leagues, which no league has.
    fn stored(league: LeagueOfTheMoment) -> Option<String> {
        match league {
            LeagueOfTheMoment::Unknown => None,
            LeagueOfTheMoment::Between => Some(String::new()),
            LeagueOfTheMoment::League(name) => Some(name.to_owned()),
        }
    }

    /// Whether the bubble of `release` stays closed during `league`, the league of the moment:
    /// it comes back for a newer version, and once a new league has started. Closed before the
    /// leagues were known, it stays closed for any league until it takes one ([`Self::settle`]).
    #[must_use]
    pub fn hides(&self, release: &Release, league: LeagueOfTheMoment) -> bool {
        Version::parse(&self.version) == Some(release.version)
            && match (&self.league, league) {
                (Some(closed), LeagueOfTheMoment::League(league)) => closed == league,
                _ => true,
            }
    }

    /// Takes `league` as its league, once known, when it was closed before the leagues were
    /// known; whether it changed.
    pub fn settle(&mut self, league: LeagueOfTheMoment) -> bool {
        if self.league.is_some() || league == LeagueOfTheMoment::Unknown {
            return false;
        }
        self.league = Self::stored(league);
        true
    }
}

/// The answer of GitHub about the latest release, of which only the tag is read.
#[derive(Debug, Deserialize)]
struct LatestRelease {
    tag_name: String,
}

/// The latest release, once, read on its own thread for as long as this value lives.
pub struct UpdateCheck {
    worker: Worker<Release>,
}

impl UpdateCheck {
    /// Asks GitHub for the latest release; `wake` is called from the thread once it answered.
    ///
    /// # Panics
    ///
    /// If the system can't start a thread.
    #[must_use]
    pub fn start(wake: impl Fn() + Send + 'static) -> Self {
        let worker = Worker::spawn_once("update-check", OnDrop::Detach, wake, |outbox, wake| {
            if let Some(release) = check(crate::VERSION, latest_tag)
                && outbox.send(release)
            {
                wake();
            }
        });
        Self { worker }
    }

    /// Gives the newer release, once it arrived, without waiting.
    pub fn receive(&mut self, handle: impl FnMut(Release)) {
        self.worker.receive(handle);
    }
}

/// The release newer than `running` given by `latest`, which reads the tag of the latest
/// release (none before the first one); why there is none goes to the log.
fn check(
    running: &str,
    latest: impl FnOnce() -> Result<Option<String>, HttpError>,
) -> Option<Release> {
    let Some(running) = Version::parse(running) else {
        log::warn!("Update check: the running version {running} can't be compared");
        return None;
    };
    let tag = match latest() {
        Ok(Some(tag)) => tag,
        Ok(None) => {
            log::info!("Update check: no release yet");
            return None;
        }
        Err(error) => {
            log::warn!("Update check: the releases can't be read ({error})");
            return None;
        }
    };
    let Some(version) = Version::parse(&tag) else {
        log::warn!("Update check: the latest release {tag:?} isn't a version");
        return None;
    };
    if version <= running {
        log::info!("Update check: {running} is the latest version");
        return None;
    }
    let page = release_page(REPOSITORY, &tag);
    log::info!("Update check: {version} is available");
    Some(Release { version, page })
}

/// The tag of the latest release; none while there is no release (GitHub answers 404).
fn latest_tag() -> Result<Option<String>, HttpError> {
    let agent = http::agent();
    let answer = agent
        .get(latest_release_api(REPOSITORY))
        .header("Accept", "application/vnd.github+json")
        .call();
    let mut response = match answer {
        Ok(response) => response,
        Err(ureq::Error::StatusCode(404)) => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let text = response.body_mut().read_to_string()?;
    let latest: LatestRelease = serde_json::from_str(&text)?;
    Ok(Some(latest.tag_name))
}

/// The address of the API of the latest release of `repository`, a repository on GitHub (a
/// test checks that the application's is).
fn latest_release_api(repository: &str) -> String {
    let path = repository.trim_start_matches(GITHUB).trim_end_matches('/');
    format!("{GITHUB_API}{path}/releases/latest")
}

/// The page of the release `tag` of `repository`, built rather than read from the answer, so
/// that the link can only open the repository of the application.
fn release_page(repository: &str, tag: &str) -> String {
    format!(
        "{}/releases/tag/{}",
        repository.trim_end_matches('/'),
        http::encode(tag)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(text: &str) -> Version {
        Version::parse(text).expect("a version")
    }

    fn release(text: &str) -> Release {
        Release {
            version: version(text),
            page: format!("{REPOSITORY}/releases/tag/v{text}"),
        }
    }

    #[test]
    fn a_version_reads_with_or_without_its_v() {
        assert_eq!(version("v0.2.0"), version("0.2.0"));
        assert_eq!(version("v1.12.3").to_string(), "1.12.3");
    }

    #[test]
    fn a_version_of_another_shape_isnt_read() {
        for text in [
            "",
            "v",
            "0.2",
            "0.2.0.1",
            "0.2.0-beta",
            "0.+2.0",
            "v.1.2",
            "0..2",
            "x0.2.0",
            "V0.2.0",
        ] {
            assert_eq!(Version::parse(text), None, "{text}");
        }
    }

    #[test]
    fn versions_compare_by_their_numbers() {
        assert!(version("0.10.0") > version("0.9.9"));
        assert!(version("1.0.0") > version("0.99.99"));
        assert!(version("0.1.1") > version("0.1.0"));
    }

    #[test]
    fn a_newer_release_is_offered_with_its_page() {
        let found = check("0.1.0", || Ok(Some("v0.2.0".to_owned())));

        assert_eq!(found, Some(release("0.2.0")));
        assert_eq!(
            found.map(|release| release.page),
            Some("https://github.com/aunimhsn/rust-exile-market/releases/tag/v0.2.0".to_owned())
        );
    }

    #[test]
    fn the_same_or_an_older_release_isnt_offered() {
        assert_eq!(check("0.2.0", || Ok(Some("v0.2.0".to_owned()))), None);
        assert_eq!(check("0.2.0", || Ok(Some("v0.1.5".to_owned()))), None);
    }

    #[test]
    fn nothing_is_offered_without_release_network_or_version() {
        assert_eq!(check("0.1.0", || Ok(None)), None);
        let unreachable = HttpError::Request(ureq::Error::StatusCode(503));
        assert_eq!(check("0.1.0", || Err(unreachable)), None);
        assert_eq!(check("0.1.0", || Ok(Some("nightly".to_owned()))), None);
        assert_eq!(check("dev", || Ok(Some("v0.2.0".to_owned()))), None);
    }

    #[test]
    fn the_api_and_the_page_come_from_the_repository() {
        assert_eq!(
            latest_release_api("https://github.com/owner/app/"),
            "https://api.github.com/repos/owner/app/releases/latest"
        );
        assert_eq!(
            release_page("https://github.com/owner/app", "v1.0.0 rc"),
            "https://github.com/owner/app/releases/tag/v1.0.0%20rc"
        );
    }

    #[test]
    fn the_repository_of_the_application_is_on_github() {
        // The API and the page of the releases are those of GitHub.
        assert!(REPOSITORY.starts_with(GITHUB), "{REPOSITORY}");
    }

    const ABYSS: LeagueOfTheMoment = LeagueOfTheMoment::League("Rise of the Abyssal");
    const ALDUR: LeagueOfTheMoment = LeagueOfTheMoment::League("Runes of Aldur");

    #[test]
    fn the_league_of_the_moment_tells_unknown_leagues_from_none_current() {
        use crate::league::League;

        assert_eq!(LeagueOfTheMoment::of(None), LeagueOfTheMoment::Unknown);
        let permanent = Leagues::new(["Standard", "Hardcore"].map(League::new));
        assert_eq!(
            LeagueOfTheMoment::of(Some(&permanent)),
            LeagueOfTheMoment::Between
        );
        let current = Leagues::new(["Rise of the Abyssal", "Standard"].map(League::new));
        assert_eq!(LeagueOfTheMoment::of(Some(&current)), ABYSS);
    }

    #[test]
    fn a_closed_bubble_stays_closed_for_its_version_during_its_league() {
        let closed = ClosedUpdate::new(&release("0.2.0"), ABYSS);

        assert!(closed.hides(&release("0.2.0"), ABYSS));
        // The leagues not known yet at this launch, or the league over.
        assert!(closed.hides(&release("0.2.0"), LeagueOfTheMoment::Unknown));
        assert!(closed.hides(&release("0.2.0"), LeagueOfTheMoment::Between));
    }

    #[test]
    fn a_closed_bubble_comes_back_for_a_newer_version() {
        let closed = ClosedUpdate::new(&release("0.2.0"), ABYSS);

        assert!(!closed.hides(&release("0.2.1"), ABYSS));
    }

    #[test]
    fn a_closed_bubble_comes_back_when_a_new_league_starts() {
        let closed = ClosedUpdate::new(&release("0.2.0"), ABYSS);

        assert!(!closed.hides(&release("0.2.0"), ALDUR));
    }

    #[test]
    fn a_bubble_closed_between_two_leagues_comes_back_when_the_next_one_starts() {
        let mut closed = ClosedUpdate::new(&release("0.2.0"), LeagueOfTheMoment::Between);
        assert!(closed.hides(&release("0.2.0"), LeagueOfTheMoment::Between));

        assert!(!closed.settle(ALDUR));

        assert!(!closed.hides(&release("0.2.0"), ALDUR));
    }

    #[test]
    fn a_bubble_closed_before_the_leagues_takes_the_first_league_that_arrives() {
        let mut closed = ClosedUpdate::new(&release("0.2.0"), LeagueOfTheMoment::Unknown);
        assert!(closed.hides(&release("0.2.0"), ABYSS));

        assert!(!closed.settle(LeagueOfTheMoment::Unknown));
        assert!(closed.settle(ABYSS));
        assert!(!closed.settle(ALDUR));

        assert!(!closed.hides(&release("0.2.0"), ALDUR));
    }

    #[test]
    fn a_bubble_closed_before_the_leagues_known_between_two_comes_back_with_the_next() {
        let mut closed = ClosedUpdate::new(&release("0.2.0"), LeagueOfTheMoment::Unknown);

        assert!(closed.settle(LeagueOfTheMoment::Between));

        assert!(!closed.hides(&release("0.2.0"), ALDUR));
    }

    #[test]
    fn a_closed_bubble_of_an_unreadable_version_hides_nothing() {
        let closed = ClosedUpdate {
            version: "soon".to_owned(),
            league: None,
        };

        assert!(!closed.hides(&release("0.2.0"), LeagueOfTheMoment::Unknown));
    }
}
