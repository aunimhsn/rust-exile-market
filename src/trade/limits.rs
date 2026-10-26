//! The limits of the trade site API, as it announces them in the headers of each answer:
//! when the next request of each kind may be sent.
//!
//! Each kind of request has its own policy, with windows of `hits` requests per `period`
//! (`X-Rate-Limit-Ip: 5:10:60,15:60:300`), and the hits the trade site counted in each window
//! (`X-Rate-Limit-Ip-State: 1:10:0,1:60:0`), for every request of the IP address: a browser or
//! another tool on the same address counts too. The application keeps the time of its own
//! requests, and adds the hits the trade site counted beyond them, as if made when it told
//! them: they leave the window later than the real ones, which keeps on the safe side. Nothing
//! is taken from constants: before the first answer, no limit is known.

use std::time::{Duration, Instant};

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
    /// `X-Rate-Limit-Rules`, then for each rule, `X-Rate-Limit-<rule>` and
    /// `X-Rate-Limit-<rule>-State`, e.g. `("Ip", "5:10:60", "1:10:0")`.
    pub rules: Vec<(String, String, String)>,
    /// `Retry-After`, in seconds.
    pub retry_after: Option<String>,
}

/// A window of a policy: `hits` requests at most per `period`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Window {
    hits: usize,
    period: Duration,
    /// The hits the trade site counted beyond the requests of the application, when it told
    /// them.
    others: Vec<Instant>,
}

/// The limits of one policy.
#[derive(Debug, Default)]
struct PolicyLimits {
    /// The requests of the application, oldest first, as long as a window counts them.
    sent: Vec<Instant>,
    windows: Vec<Window>,
    /// The end of a refusal, or of a restriction told by the trade site.
    blocked_until: Option<Instant>,
}

impl PolicyLimits {
    fn ready_at(&self, now: Instant) -> Instant {
        let mut ready = now.max(self.blocked_until.unwrap_or(now));
        for window in &self.windows {
            let start = now.checked_sub(window.period);
            let mut hits: Vec<Instant> = self
                .sent
                .iter()
                .chain(&window.others)
                .copied()
                .filter(|&hit| start.is_none_or(|start| hit > start))
                .collect();
            if hits.len() >= window.hits {
                hits.sort_unstable();
                // The window allows a request once enough hits have left it.
                if let Some(&leaving) = hits.get(hits.len() - window.hits) {
                    ready = ready.max(leaving + window.period + MARGIN);
                }
            }
        }
        ready
    }

    fn sent(&mut self, now: Instant) {
        let kept = self
            .windows
            .iter()
            .map(|window| window.period)
            .max()
            .unwrap_or(DEFAULT_HISTORY);
        if let Some(start) = now.checked_sub(kept) {
            self.sent.retain(|&hit| hit > start);
        }
        self.sent.push(now);
    }

    fn answered(&mut self, headers: &LimitHeaders, now: Instant) {
        let mut windows = Vec::new();
        for (_, rule, state) in &headers.rules {
            for (limit, counted) in rule.split(',').zip(state.split(',')) {
                let (Some((hits, period, _)), Some((counted, counted_period, restricted))) =
                    (triple(limit), triple(counted))
                else {
                    log::warn!("Unexpected limits of the trade site: {rule} / {state}");
                    continue;
                };
                if period != counted_period {
                    log::warn!("Unexpected limits of the trade site: {rule} / {state}");
                    continue;
                }
                let period = seconds(period);
                let start = now.checked_sub(period);
                let ours = self
                    .sent
                    .iter()
                    .filter(|&&hit| start.is_none_or(|start| hit > start))
                    .count();
                let others = usize::try_from(counted).unwrap_or(usize::MAX);
                windows.push(Window {
                    hits: usize::try_from(hits).unwrap_or(usize::MAX),
                    period,
                    others: vec![now; others.saturating_sub(ours)],
                });
                if restricted > 0 {
                    self.block(now + seconds(restricted));
                }
            }
        }
        // An answer without limits keeps the ones known.
        if !windows.is_empty() {
            self.windows = windows;
        }
    }

    fn block(&mut self, until: Instant) {
        self.blocked_until = Some(
            self.blocked_until
                .map_or(until, |blocked| blocked.max(until)),
        );
    }
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

    /// Counts a request of `policy` sent `now`, before its answer: a request that fails
    /// counts too.
    pub fn sent(&mut self, policy: Policy, now: Instant) {
        self.policies[policy.index()].sent(now);
    }

    /// Reads the limits of an answer received `now` to a request of `expected`, which the
    /// answer names itself when it can.
    pub fn answered(&mut self, expected: Policy, headers: &LimitHeaders, now: Instant) {
        let policy = match headers.policy.as_deref() {
            None => expected,
            Some(name) => Policy::named(name).unwrap_or_else(|| {
                log::warn!("Unknown policy of the trade site: {name}");
                expected
            }),
        };
        self.policies[policy.index()].answered(headers, now);
    }

    /// The trade site refused a request of `policy` (HTTP 429) `now`: no request of it is
    /// sent before the delay of `Retry-After`. Returns the end of the wait.
    pub fn refused(&mut self, policy: Policy, headers: &LimitHeaders, now: Instant) -> Instant {
        self.answered(policy, headers, now);
        let delay = headers
            .retry_after
            .as_deref()
            .and_then(|seconds| seconds.trim().parse().ok())
            .map_or(DEFAULT_RETRY_AFTER, seconds);
        let limits = &mut self.policies[policy.index()];
        limits.block(now + delay);
        limits.ready_at(now)
    }
}

/// A time read in a header, at most a day.
fn seconds(seconds: u64) -> Duration {
    Duration::from_secs(seconds.min(LONGEST))
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
mod tests {
    use super::*;

    /// The headers of a search answer, measured on 2026-10-26, with the hits `state`.
    fn search_headers(state: &str) -> LimitHeaders {
        LimitHeaders {
            policy: Some("trade-search-request-limit".to_owned()),
            rules: vec![(
                "Ip".to_owned(),
                "5:10:60,15:60:300,30:300:1800,600:21600:3600".to_owned(),
                state.to_owned(),
            )],
            retry_after: None,
        }
    }

    const SECOND: Duration = Duration::from_secs(1);

    #[test]
    fn before_any_answer_no_limit_is_known() {
        let mut limits = Limits::default();
        let now = Instant::now();
        for _ in 0..10 {
            limits.sent(Policy::Search, now);
        }

        assert_eq!(limits.ready_at(Policy::Search, now), now);
    }

    #[test]
    fn a_full_window_waits_until_its_oldest_request_leaves_it() {
        let mut limits = Limits::default();
        let start = Instant::now();
        for second in 0..5 {
            let now = start + second * SECOND;
            limits.sent(Policy::Search, now);
            let state = format!("{}:10:0,{0}:60:0,{0}:300:0,{0}:21600:0", second + 1);
            limits.answered(Policy::Search, &search_headers(&state), now);
        }
        let now = start + 4 * SECOND;

        // 5 per 10 s: the first request, at 0 s, leaves the window at 10 s.
        assert_eq!(
            limits.ready_at(Policy::Search, now),
            start + 10 * SECOND + MARGIN
        );
        // The other policies have their own limits.
        assert_eq!(limits.ready_at(Policy::Fetch, now), now);
    }

    #[test]
    fn the_hits_of_another_tool_on_the_same_address_count() {
        let mut limits = Limits::default();
        let now = Instant::now();
        limits.sent(Policy::Search, now);

        // A browser made 14 searches in the last minute.
        limits.answered(
            Policy::Search,
            &search_headers("1:10:0,15:60:0,15:300:0,15:21600:0"),
            now,
        );

        assert_eq!(
            limits.ready_at(Policy::Search, now),
            now + 60 * SECOND + MARGIN
        );
    }

    #[test]
    fn a_restriction_told_by_the_trade_site_waits_its_whole_time() {
        let mut limits = Limits::default();
        let now = Instant::now();

        limits.answered(
            Policy::Search,
            &search_headers("6:10:60,6:60:0,6:300:0,6:21600:0"),
            now,
        );

        assert_eq!(limits.ready_at(Policy::Search, now), now + 60 * SECOND);
    }

    #[test]
    fn a_refusal_waits_for_the_delay_it_gives() {
        let mut limits = Limits::default();
        let now = Instant::now();
        let mut headers = search_headers("1:10:0,1:60:0,1:300:0,1:21600:0");
        headers.retry_after = Some("300".to_owned());

        let until = limits.refused(Policy::Search, &headers, now);

        assert_eq!(until, now + 300 * SECOND);
        assert_eq!(limits.ready_at(Policy::Search, now), until);
        // Without the header, a minute.
        let mut other = Limits::default();
        assert_eq!(
            other.refused(Policy::Fetch, &LimitHeaders::default(), now),
            now + DEFAULT_RETRY_AFTER
        );
    }

    #[test]
    fn the_policy_named_by_the_answer_is_the_one_updated() {
        let mut limits = Limits::default();
        let now = Instant::now();

        // A full exchange window, in an answer to what was thought a search.
        let headers = LimitHeaders {
            policy: Some("trade-exchange-request-limit".to_owned()),
            rules: vec![("Ip".to_owned(), "5:15:60".to_owned(), "5:15:0".to_owned())],
            retry_after: None,
        };
        limits.answered(Policy::Search, &headers, now);

        assert_eq!(limits.ready_at(Policy::Search, now), now);
        assert_eq!(
            limits.ready_at(Policy::Exchange, now),
            now + 15 * SECOND + MARGIN
        );
    }

    #[test]
    fn unreadable_limits_keep_the_ones_known() {
        let mut limits = Limits::default();
        let now = Instant::now();
        limits.answered(
            Policy::Search,
            &search_headers("5:10:0,5:60:0,5:300:0,5:21600:0"),
            now,
        );

        limits.answered(Policy::Search, &search_headers("five"), now);

        assert!(limits.ready_at(Policy::Search, now) > now);
    }

    #[test]
    fn the_numbers_of_a_rule_are_read() {
        assert_eq!(triple("5:10:60"), Some((5, 10, 60)));
        assert_eq!(triple(" 1:21600:0"), Some((1, 21600, 0)));
        assert_eq!(triple("5:10"), None);
        assert_eq!(triple("5:10:60:1"), None);
        assert_eq!(triple("a:10:60"), None);
    }
}
