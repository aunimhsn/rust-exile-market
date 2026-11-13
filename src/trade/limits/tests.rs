use super::*;

const SECOND: Duration = Duration::from_secs(1);

/// A request answered at once, with the limits.
fn hit(time: Instant) -> Hit {
    Hit {
        sent: time,
        answered: time,
        counted: true,
    }
}

/// The headers of a policy with the rule `Ip`.
fn headers(policy: &str, limits: &str, state: &str) -> LimitHeaders {
    LimitHeaders {
        policy: Some(policy.to_owned()),
        rules: vec![rule("Ip", limits, state)],
        retry_after: None,
    }
}

fn rule(name: &str, limits: &str, state: &str) -> RuleHeaders {
    RuleHeaders {
        name: name.to_owned(),
        limits: limits.to_owned(),
        state: state.to_owned(),
    }
}

/// The headers of a search answer, measured on 2026-10-26, with the hits `state`.
fn search_headers(state: &str) -> LimitHeaders {
    headers(
        "trade-search-request-limit",
        "5:10:60,15:60:300,30:300:1800,600:21600:3600",
        state,
    )
}

#[test]
fn before_any_answer_no_limit_is_known() {
    let mut limits = Limits::default();
    let now = Instant::now();
    for _ in 0..10 {
        limits.sent(Policy::Search, hit(now));
    }

    assert_eq!(limits.ready_at(Policy::Search, now), now);
}

#[test]
fn a_full_window_waits_until_its_oldest_request_leaves_it() {
    let mut limits = Limits::default();
    let start = Instant::now();
    for second in 0..5 {
        let now = start + second * SECOND;
        limits.sent(Policy::Search, hit(now));
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
    let start = Instant::now();
    limits.sent(Policy::Search, hit(start));
    let now = start + 2 * SECOND;

    // A browser made 14 searches in the last minute, told at 2 s.
    limits.answered(
        Policy::Search,
        &search_headers("1:10:0,15:60:0,15:300:0,15:21600:0"),
        now,
    );

    // The window is full; ours, the oldest, leaves it first.
    assert_eq!(
        limits.ready_at(Policy::Search, now),
        start + 60 * SECOND + MARGIN
    );
}

#[test]
fn the_hits_counted_before_ours_leave_the_window_first() {
    let mut limits = Limits::default();
    let start = Instant::now();
    limits.answered(
        Policy::Fetch,
        &headers("trade-fetch-request-limit", "3:10:10", "1:10:0"),
        start,
    );
    limits.sent(Policy::Fetch, hit(start + SECOND));
    limits.sent(Policy::Fetch, hit(start + 2 * SECOND));

    // Another tool at 0 s, then ours at 1 s and 2 s: the one at 0 s leaves first.
    assert_eq!(
        limits.ready_at(Policy::Fetch, start + 3 * SECOND),
        start + 10 * SECOND + MARGIN
    );
}

#[test]
fn a_huge_count_of_hits_waits_without_failing() {
    let mut limits = Limits::default();
    let now = Instant::now();

    limits.answered(
        Policy::Search,
        &headers(
            "trade-search-request-limit",
            "5:10:60",
            "18446744073709551615:10:0",
        ),
        now,
    );

    assert_eq!(
        limits.ready_at(Policy::Search, now),
        now + 10 * SECOND + MARGIN
    );
}

#[test]
fn a_window_of_no_request_allows_none_during_its_period() {
    let mut limits = Limits::default();
    let now = Instant::now();

    limits.answered(
        Policy::Search,
        &headers("trade-search-request-limit", "0:10:60", "0:10:0"),
        now,
    );

    assert_eq!(
        limits.ready_at(Policy::Search, now),
        now + 10 * SECOND + MARGIN
    );
    // Not longer: no request would bring new limits.
    let later = now + 12 * SECOND;
    assert_eq!(limits.ready_at(Policy::Search, later), later);
}

#[test]
fn a_request_the_trade_site_no_longer_counts_doesnt_hide_the_hits_of_another_tool() {
    let mut limits = Limits::default();
    let start = Instant::now();
    let tenth = Duration::from_millis(100);
    let fetch = |state| headers("trade-fetch-request-limit", "5:10:10", state);
    // Ours, received at 0 s, answered at 0.5 s.
    limits.sent(
        Policy::Fetch,
        Hit {
            sent: start,
            answered: start + 5 * tenth,
            counted: true,
        },
    );
    limits.answered(Policy::Fetch, &fetch("1:10:0"), start + 5 * tenth);
    // A browser makes 4 requests at 9 s; ours at 10.2 s, answered at 10.4 s: 5 of 5,
    // since the trade site no longer counts the first one.
    limits.sent(
        Policy::Fetch,
        Hit {
            sent: start + 102 * tenth,
            answered: start + 104 * tenth,
            counted: true,
        },
    );
    limits.answered(Policy::Fetch, &fetch("5:10:0"), start + 104 * tenth);

    let now = start + 106 * tenth;
    assert!(limits.ready_at(Policy::Fetch, now) > now);
}

#[test]
fn a_request_that_didnt_reach_the_trade_site_doesnt_hide_the_hits_of_another_tool() {
    let mut limits = Limits::default();
    let start = Instant::now();
    let fetch = |state| headers("trade-fetch-request-limit", "5:10:10", state);
    // A request that failed without an answer, at 0 s.
    limits.sent(
        Policy::Fetch,
        Hit {
            sent: start,
            answered: start,
            counted: false,
        },
    );
    // A browser makes 4 requests at 1 s; ours at 2 s: 5 of 5.
    limits.sent(Policy::Fetch, hit(start + 2 * SECOND));
    limits.answered(Policy::Fetch, &fetch("5:10:0"), start + 2 * SECOND);

    // The failed one leaves the window at 11 s; the trade site still counts 5 then.
    let now = start + 11 * SECOND + MARGIN + Duration::from_millis(100);
    assert!(limits.ready_at(Policy::Fetch, now) > now);
}

#[test]
fn the_time_told_doesnt_come_sooner_when_asked_again() {
    let mut limits = Limits::default();
    let start = Instant::now();
    for second in 0..5 {
        let now = start + second * SECOND;
        limits.sent(Policy::Search, hit(now));
        let state = format!("{}:10:0,{0}:60:0,{0}:300:0,{0}:21600:0", second + 1);
        limits.answered(Policy::Search, &search_headers(&state), now);
    }
    let told = limits.ready_at(Policy::Search, start + 4 * SECOND);

    for tenths in 41..110 {
        let now = start + Duration::from_millis(tenths * 100);
        assert_eq!(limits.ready_at(Policy::Search, now), told, "{tenths}");
    }
    assert_eq!(limits.ready_at(Policy::Search, told), told);
}

#[test]
fn a_restriction_applies_even_beside_a_rule_that_cant_be_read() {
    let mut limits = Limits::default();
    let now = Instant::now();
    let mut headers = search_headers("6:10:60,6:60:0,6:300:0,6:21600:0");
    headers.rules.push(rule("Account", "2:30:60", "two"));

    limits.answered(Policy::Search, &headers, now);

    assert_eq!(
        limits.ready_at(Policy::Search, now),
        now + 60 * SECOND + MARGIN
    );
}

#[test]
fn the_restriction_of_a_rule_that_cant_be_read_applies() {
    let mut limits = Limits::default();
    let now = Instant::now();

    // Its limits can't be read, its state can.
    limits.answered(
        Policy::Search,
        &headers("trade-search-request-limit", "five", "6:10:60"),
        now,
    );

    assert_eq!(
        limits.ready_at(Policy::Search, now),
        now + 60 * SECOND + MARGIN
    );
}

#[test]
fn every_rule_of_an_answer_counts() {
    let mut limits = Limits::default();
    let now = Instant::now();
    let mut headers = search_headers("1:10:0,1:60:0,1:300:0,1:21600:0");
    headers.rules.push(rule("Account", "2:30:60", "2:30:0"));

    limits.answered(Policy::Search, &headers, now);

    assert_eq!(
        limits.ready_at(Policy::Search, now),
        now + 30 * SECOND + MARGIN
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

    assert_eq!(
        limits.ready_at(Policy::Search, now),
        now + 60 * SECOND + MARGIN
    );
}

#[test]
fn a_refusal_waits_for_the_delay_it_gives() {
    let mut limits = Limits::default();
    let now = Instant::now();
    let mut headers = search_headers("1:10:0,1:60:0,1:300:0,1:21600:0");
    headers.retry_after = Some("300".to_owned());

    let until = limits.refused(Policy::Search, &headers, now);

    assert_eq!(until, now + 300 * SECOND + MARGIN);
    assert_eq!(limits.ready_at(Policy::Search, now), until);
    // Without the header, a minute.
    let mut other = Limits::default();
    assert_eq!(
        other.refused(Policy::Fetch, &LimitHeaders::default(), now),
        now + DEFAULT_RETRY_AFTER + MARGIN
    );
}

#[test]
fn a_refusal_waits_for_a_restriction_longer_than_its_delay() {
    let mut limits = Limits::default();
    let now = Instant::now();
    let mut headers = search_headers("1:10:0,16:60:300,16:300:0,16:21600:0");
    headers.retry_after = Some("60".to_owned());

    assert_eq!(
        limits.refused(Policy::Search, &headers, now),
        now + 300 * SECOND + MARGIN
    );
}

#[test]
fn the_policy_named_by_the_answer_is_the_one_updated() {
    let mut limits = Limits::default();
    let now = Instant::now();

    // A full exchange window, in an answer to what was thought a search.
    let headers = headers("trade-exchange-request-limit", "5:15:60", "5:15:0");
    limits.answered(Policy::Search, &headers, now);

    assert_eq!(limits.ready_at(Policy::Search, now), now);
    assert_eq!(
        limits.ready_at(Policy::Exchange, now),
        now + 15 * SECOND + MARGIN
    );
}

#[test]
fn limits_partly_unreadable_keep_the_ones_known() {
    let now = Instant::now();
    let full = "5:10:0,5:60:0,5:300:0,5:21600:0";
    for state in [
        // A window that can't be read, among others.
        "1:10:0,five:60:0,1:300:0,1:21600:0",
        // A window of the limits missing in the state.
        "1:10:0,1:60:0",
        // A window of another period.
        "1:10:0,1:60:0,1:301:0,1:21600:0",
        // A rule whose headers are missing.
        "",
    ] {
        let mut limits = Limits::default();
        limits.answered(Policy::Search, &search_headers(full), now);

        limits.answered(Policy::Search, &search_headers(state), now);

        assert!(limits.ready_at(Policy::Search, now) > now, "{state:?}");
    }
}

#[test]
fn the_numbers_of_a_rule_are_read() {
    assert_eq!(triple("5:10:60"), Some((5, 10, 60)));
    assert_eq!(triple(" 1:21600:0"), Some((1, 21600, 0)));
    assert_eq!(triple("5:10"), None);
    assert_eq!(triple("5:10:60:1"), None);
    assert_eq!(triple("a:10:60"), None);
}
