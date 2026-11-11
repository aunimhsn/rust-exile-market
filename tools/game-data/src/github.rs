//! What the generator reads on GitHub: the commit of a branch, and the files of a commit.

use serde::Deserialize;

use crate::Result;

#[derive(Deserialize)]
pub(crate) struct Commit {
    pub(crate) sha: String,
    pub(crate) commit: CommitDetail,
}

#[derive(Deserialize)]
pub(crate) struct CommitDetail {
    pub(crate) committer: Committer,
}

#[derive(Deserialize)]
pub(crate) struct Committer {
    pub(crate) date: String,
}

/// The commit `reference` (a branch or a commit) of the GitHub `repository`.
pub(crate) fn latest_commit(
    agent: &ureq::Agent,
    repository: &str,
    reference: &str,
) -> Result<Commit> {
    Ok(serde_json::from_str(&get(
        agent,
        &format!("https://api.github.com/repos/{repository}/commits/{reference}"),
    )?)?)
}

pub(crate) fn raw_url(repository: &str, commit: &str, path: &str) -> String {
    format!("https://raw.githubusercontent.com/{repository}/{commit}/{path}")
}

pub(crate) fn get(agent: &ureq::Agent, url: &str) -> Result<String> {
    read(agent, url).map_err(|error| format!("{url}: {error}").into())
}

fn read(agent: &ureq::Agent, url: &str) -> Result<String> {
    Ok(agent
        .get(url)
        .header(
            "User-Agent",
            "rust-exile-market game data generator (+https://github.com/aunimhsn/rust-exile-market)",
        )
        .call()?
        .body_mut()
        .with_config()
        .limit(64 * 1024 * 1024)
        .read_to_string()?)
}
