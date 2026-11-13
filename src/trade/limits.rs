//! The limits of the trade site API, as it announces them in the headers of each answer:
//! when the next request of each kind may be sent.
//!
//! Each kind of request has its own policy, with windows of `hits` requests per `period`
//! (`X-Rate-Limit-Ip: 5:10:60,15:60:300`), and the hits the trade site counted in each window
//! (`X-Rate-Limit-Ip-State: 1:10:0,1:60:0`), for every request of the IP address: a browser or
//! another tool on the same address counts too. The application keeps the times of its own
//! requests, and adds the hits the trade site counted beyond them, as if made when it told
//! them: they leave the window later than the real ones, which keeps on the safe side. Nothing
//! is taken from constants: before the first answer, no limit is known. When in doubt (limits
//! that can't be read), the limits known are kept.

use std::time::{Duration, Instant};

use crate::logging;

/// Added to the end of a window: the clock of the trade site isn't ours.
const MARGIN: Duration = Duration::from_secs(1);
/// How long the requests are kept before the windows are known: the longest window measured
/// (6 h).
const DEFAULT_HISTORY: Duration = Duration::from_hours(6);
/// The wait after a refusal that doesn't say how long to wait.
const DEFAULT_RETRY_AFTER: Duration = Duration::from_mins(1);
/// The longest time read in a header, so that a wrong number can't overflow a time: the
/// longest window measured is 6 h.
const LONGEST: u64 = 24 * 60 * 60;

/// The kinds of requests, each with its own limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Policy {
    /// A search of items, which gives the ids of the offers.
    Search,
    /// The offers of a search, 10 at most per request.
    Fetch,
    /// A search of the currency exchange, which gives its offers.
    Exchange,
}

impl Policy {
    const ALL: [Self; 3] = [Self::Search, Self::Fetch, Self::Exchange];

    /// The policy named in `X-Rate-Limit-Policy`.
    fn named(name: &str) -> Option<Self> {
        match name {
            "trade-search-request-limit" => Some(Self::Search),
            "trade-fetch-request-limit" => Some(Self::Fetch),
            "trade-exchange-request-limit" => Some(Self::Exchange),
            _ => None,
        }
    }

    fn index(self) -> usize {
        match self {
            Self::Search => 0,
            Self::Fetch => 1,
            Self::Exchange => 2,
        }
    }
}

/// The headers of an answer the limits read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LimitHeaders {
    /// `X-Rate-Limit-Policy`.
    pub policy: Option<String>,
    /// One for each rule of `X-Rate-Limit-Rules`.
    pub rules: Vec<RuleHeaders>,
    /// `Retry-After`, in seconds.
    pub retry_after: Option<String>,
}

/// The headers of a rule; empty when missing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuleHeaders {
    /// `Ip`.
    pub name: String,
    /// `X-Rate-Limit-<name>`, e.g. `5:10:60,15:60:300`.
    pub limits: String,
    /// `X-Rate-Limit-<name>-State`, e.g. `1:10:0,1:60:0`.
    pub state: String,
}

/// A request of the application: when it left, and when its answer, or its failure, came.
/// The trade site counted it in between, if it did: it is in a window since it left at the
/// earliest, and until its answer leaves it at the latest; each count takes the time and
/// the case on the safe side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hit {
    pub sent: Instant,
    pub answered: Instant,
    /// The trade site counted it for sure: it answered a success with its limits. Only
    /// these are taken from its counts to find the hits of the other tools; all of them stay
    /// in the windows.
    pub counted: bool,
}

/// A window of a policy: `hits` requests at most per `period`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Window {
    hits: usize,
    period: Duration,
    /// The hits the trade site counted beyond the requests of the application…
    others: usize,
    /// …when it told them.
    told: Instant,
}

impl Window {
    /// When the window allows one more request, among the requests `hits` of the
    /// application (oldest first). Without a new request or answer, the time told never
    /// comes sooner when asked again: the margin applies to whether a hit is in the window
    /// too.
    fn ready_at(&self, hits: &[Hit], now: Instant) -> Instant {
        if self.hits == 0 {
            // A window of no request allows none during its period.
            return (self.told + self.period + MARGIN).max(now);
        }
        let start = now.checked_sub(self.period + MARGIN);
        let inside = |hit: Instant| start.is_none_or(|start| hit > start);
        // The requests of the application in the window, oldest first.
        let first = hits.partition_point(|hit| !inside(hit.answered));
        let ours = hits.get(first..).unwrap_or_default();
        let others = if inside(self.told) { self.others } else { 0 };
        let count = ours.len().saturating_add(others);
        if count < self.hits {
            return now;
        }
        // The window allows a request once `count - hits + 1` hits have left it: the one
        // of index `count - hits` among all of them, oldest first, found without listing
        // them.
        let index = count - self.hits;
        let before_told = ours.partition_point(|hit| hit.answered <= self.told);
        let leaving = if index < before_told {
            ours.get(index).map(|hit| hit.answered)
        } else if index - before_told < others {
            Some(self.told)
        } else {
            ours.get(index - others).map(|hit| hit.answered)
        };
        // Always found: the index is below the count.
        leaving.map_or(now, |leaving| leaving + self.period + MARGIN)
    }
}

/// The limits of one policy.
#[derive(Debug, Default)]
struct PolicyLimits {
    /// The requests of the application, in the order of their answers, as long as a window
    /// counts them.
    sent: Vec<Hit>,
    windows: Vec<Window>,
    /// The end of a refusal, or of a restriction told by the trade site.
    blocked_until: Option<Instant>,
}

impl PolicyLimits {
    fn ready_at(&self, now: Instant) -> Instant {
        self.windows
            .iter()
            .map(|window| window.ready_at(&self.sent, now))
            .fold(now.max(self.blocked_until.unwrap_or(now)), Instant::max)
    }

    fn sent(&mut self, hit: Hit) {
        let kept = self
            .windows
            .iter()
            .map(|window| window.period)
            .max()
            .unwrap_or(DEFAULT_HISTORY);
        if let Some(start) = hit.answered.checked_sub(kept + MARGIN) {
            self.sent.retain(|kept| kept.answered > start);
        }
        self.sent.push(hit);
    }

    fn answered(&mut self, headers: &LimitHeaders, now: Instant) {
        let mut windows = Vec::new();
        let mut unreadable = false;
        let mut restricted = Duration::ZERO;
        for rule in &headers.rules {
            let Some(read) = read_rule(rule) else {
                log::warn!(
                    "Unexpected limits of the trade site, the known ones are kept: {} {} / {}",
                    logging::plain(&rule.name),
                    logging::plain(&rule.limits),
                    logging::plain(&rule.state)
                );
                unreadable = true;
                // Its restrictions apply all the same, those that can be read.
                let rule_restricted = rule
                    .state
                    .split(',')
                    .filter_map(triple)
                    .map(|(_, _, restricted)| seconds(restricted))
                    .max()
                    .unwrap_or_default();
                restricted = restricted.max(rule_restricted);
                continue;
            };
            for window in read {
                // Only the requests the trade site counts for sure: counted, and left within
                // the window, less the margin. Fewer of ours leave more hits to the other
                // tools.
                let start = now.checked_sub(window.period.saturating_sub(MARGIN));
                let ours = self
                    .sent
                    .iter()
                    .filter(|hit| hit.counted && start.is_none_or(|start| hit.sent > start))
                    .count();
                windows.push(Window {
                    hits: window.hits,
                    period: window.period,
                    others: window.counted.saturating_sub(ours),
                    told: now,
                });
                restricted = restricted.max(window.restricted);
            }
        }
        // An answer without limits, or with limits that can't all be read, keeps the ones
        // known; the longest restriction read applies all the same.
        if !unreadable && !windows.is_empty() {
            self.windows = windows;
        }
        if !restricted.is_zero() {
            self.block(now + restricted + MARGIN);
        }
    }

    fn block(&mut self, until: Instant) {
        self.blocked_until = Some(
            self.blocked_until
                .map_or(until, |blocked| blocked.max(until)),
        );
    }
}

/// A window as the headers tell it.
struct ReadWindow {
    hits: usize,
    period: Duration,
    /// The hits the trade site counted in it.
    counted: usize,
    /// How long the trade site refuses the requests, when they went over it.
    restricted: Duration,
}

/// The windows of a rule; `None` when a window can't be read, or when the limits and the
/// state don't have the same windows.
fn read_rule(rule: &RuleHeaders) -> Option<Vec<ReadWindow>> {
    let limits: Vec<&str> = rule.limits.split(',').collect();
    let states: Vec<&str> = rule.state.split(',').collect();
    if limits.len() != states.len() {
        return None;
    }
    limits
        .iter()
        .zip(&states)
        .map(|(limit, state)| {
            let (hits, period, _) = triple(limit)?;
            let (counted, counted_period, restricted) = triple(state)?;
            (period == counted_period).then(|| ReadWindow {
                hits: count(hits),
                period: seconds(period),
                counted: count(counted),
                restricted: seconds(restricted),
            })
        })
        .collect()
}

/// The limits of every policy.
#[derive(Debug, Default)]
pub struct Limits {
    policies: [PolicyLimits; Policy::ALL.len()],
}

impl Limits {
    /// When a request of `policy` may be sent, `now` at the earliest.
    #[must_use]
    pub fn ready_at(&self, policy: Policy, now: Instant) -> Instant {
        self.policies[policy.index()].ready_at(now)
    }

    /// Counts a request of `policy`, once its answer, or its failure, came: a request that
    /// failed counts too.
    pub fn sent(&mut self, policy: Policy, hit: Hit) {
        self.policies[policy.index()].sent(hit);
    }

    /// Reads the limits of an answer received `now` to a request of `expected`, which the
    /// answer names itself when it can.
    pub fn answered(&mut self, expected: Policy, headers: &LimitHeaders, now: Instant) {
        let policy = match headers.policy.as_deref() {
            None => expected,
            Some(name) => Policy::named(name).unwrap_or_else(|| {
                log::warn!("Unknown policy of the trade site: {}", logging::plain(name));
                expected
            }),
        };
        self.policies[policy.index()].answered(headers, now);
    }

    /// The trade site refused a request of `policy` (HTTP 429 or 403) `now`: no request of it is
    /// sent before the delay of `Retry-After`, nor before the restriction it tells. Returns
    /// the end of the wait.
    pub fn refused(&mut self, policy: Policy, headers: &LimitHeaders, now: Instant) -> Instant {
        self.answered(policy, headers, now);
        let limits = &mut self.policies[policy.index()];
        limits.block(now + refusal_wait(headers));
        limits.ready_at(now)
    }
}

/// How long a refusal asks to wait: the delay of `Retry-After`, a day at most, or a minute
/// when it gives none, and the margin.
pub fn refusal_wait(headers: &LimitHeaders) -> Duration {
    let delay = headers
        .retry_after
        .as_deref()
        .and_then(|seconds| seconds.trim().parse().ok())
        .map_or(DEFAULT_RETRY_AFTER, seconds);
    delay + MARGIN
}

/// A time read in a header, at most a day.
fn seconds(seconds: u64) -> Duration {
    Duration::from_secs(seconds.min(LONGEST))
}

/// A number of hits read in a header.
fn count(hits: u64) -> usize {
    usize::try_from(hits).unwrap_or(usize::MAX)
}

/// The three numbers of `5:10:60`.
fn triple(text: &str) -> Option<(u64, u64, u64)> {
    let mut numbers = text.trim().split(':').map(str::parse);
    match (
        numbers.next(),
        numbers.next(),
        numbers.next(),
        numbers.next(),
    ) {
        (Some(Ok(first)), Some(Ok(second)), Some(Ok(third)), None) => Some((first, second, third)),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
