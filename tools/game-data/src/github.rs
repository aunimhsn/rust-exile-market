//! What the generator reads on GitHub: the commit of a branch, and the files of a commit.

use serde::Deserialize;

use crate::Result;

/// The most a file read may hold: more than the 10 MB `ureq` reads by default, which `mods.json`
/// of RePoE passes (13 MB on 2026-11-11).
const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;

/// The API and the files of GitHub, read through one agent.
pub struct GitHub {
    agent: ureq::Agent,
}

/// A commit of a repository, with the fields the generator reads.
#[derive(Deserialize)]
pub struct Commit {
    pub sha: String,
    commit: CommitDetail,
}

/// The detail of a commit, of which the generator reads who committed it.
#[derive(Deserialize)]
struct CommitDetail {
    committer: Committer,
}

/// Who committed a commit, and when.
#[derive(Deserialize)]
struct Committer {
    date: String,
}

impl Commit {
    /// When it was committed.
    pub fn date(&self) -> &str {
        &self.commit.committer.date
    }
}

impl GitHub {
    /// A reader through the agent of `ureq` and its defaults.
    pub fn new() -> Self {
        Self {
            agent: ureq::Agent::new_with_defaults(),
        }
    }

    /// The commit `reference` (a branch or a commit) of `repository`.
    pub fn commit(&self, repository: &str, reference: &str) -> Result<Commit> {
        let url = format!("https://api.github.com/repos/{repository}/commits/{reference}");
        Ok(serde_json::from_str(&self.get(&url)?)?)
    }

    /// The file `path` of `repository` at `commit`.
    pub fn file(&self, repository: &str, commit: &str, path: &str) -> Result<String> {
        self.get(&format!(
            "https://raw.githubusercontent.com/{repository}/{commit}/{path}"
        ))
    }

    /// The body of `url`, its errors naming it.
    fn get(&self, url: &str) -> Result<String> {
        let read = || -> Result<String> {
            Ok(self
                .agent
                .get(url)
                .header(
                    "User-Agent",
                    "rust-exile-market game data generator (+https://github.com/sylent-dev/rust-exile-market)",
                )
                .call()?
                .body_mut()
                .with_config()
                .limit(MAX_FILE_BYTES)
                .read_to_string()?)
        };
        read().map_err(|error| format!("{url}: {error}").into())
    }
}
