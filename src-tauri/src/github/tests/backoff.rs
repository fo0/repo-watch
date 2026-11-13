//! Rate limits and backoff: Retry-After, reset headers, secondary limits and clock edges.
use super::super::*;
use super::{client, dispatch_request, response};
use crate::models::DispatchState;

#[test]
fn rate_limit_backoff_honors_retry_after_and_reset_without_retrying_post() {
    let (client, sent) = client(vec![
        Ok(response(429, &[("Retry-After", "30")], "{}")),
        Ok(response(
            200,
            &[("X-RateLimit-Remaining", "0"), ("X-RateLimit-Reset", "200")],
            "[]",
        )),
        Ok(response(200, &[], "[]")),
    ]);
    assert_eq!(
        client
            .dispatch(&dispatch_request(), "octo", "demo", 100)
            .state,
        DispatchState::Rejected
    );
    let get = ApiRequest::get(
        "personal",
        ApiUrl::parse("https://api.github.com/user").unwrap(),
    )
    .unwrap();
    assert_eq!(
        client.execute(&get, 110).unwrap_err(),
        ApiError::RateLimited { retry_at: 130 }
    );
    assert_eq!(sent.lock().unwrap().len(), 1);
    client.execute(&get, 130).unwrap();
    assert_eq!(
        client.execute(&get, 199).unwrap_err(),
        ApiError::RateLimited { retry_at: 200 }
    );
    client.execute(&get, 200).unwrap();
    assert_eq!(sent.lock().unwrap().len(), 3);
}

#[test]
fn secondary_limit_and_http_date_backoff_differ_from_forbidden_permissions() {
    let http_date = "Sat, 31 Oct 2026 10:01:00 GMT";
    let start = chrono::DateTime::parse_from_rfc2822("Sat, 31 Oct 2026 10:00:00 GMT")
        .unwrap()
        .timestamp() as u64;
    let (client, sent) = client(vec![
        Ok(response(403, &[], r#"{"message":"Forbidden"}"#)),
        Ok(response(
            403,
            &[],
            r#"{"message":"You have exceeded a secondary rate limit."}"#,
        )),
        Ok(response(429, &[("Retry-After", http_date)], "{}")),
    ]);
    let request = ApiRequest::get(
        "personal",
        ApiUrl::parse("https://api.github.com/user").unwrap(),
    )
    .unwrap();
    assert_eq!(client.execute(&request, start).unwrap().status, 403);
    // Ordinary permission denial does not prevent a separate request for the account.
    assert_eq!(client.execute(&request, start).unwrap().status, 403);
    assert_eq!(
        client.execute(&request, start + 1).unwrap_err(),
        ApiError::RateLimited {
            retry_at: start + 60
        }
    );
    let other = ApiRequest::get(
        "work",
        ApiUrl::parse("https://api.github.com/user").unwrap(),
    )
    .unwrap();
    assert_eq!(client.execute(&other, start).unwrap().status, 429);
    assert_eq!(
        client.execute(&other, start + 1).unwrap_err(),
        ApiError::RateLimited {
            retry_at: start + 60
        }
    );
    assert_eq!(sent.lock().unwrap().len(), 3);
}

#[test]
fn backoff_deadlines_are_clamped_to_one_hour() {
    let get = ApiRequest::get(
        "personal",
        ApiUrl::parse("https://api.github.com/user").unwrap(),
    )
    .unwrap();
    for headers in [
        vec![("Retry-After", "999999")],
        vec![("Retry-After", "Fri, 31 Dec 2100 00:00:00 GMT")],
        vec![
            ("X-RateLimit-Remaining", "0"),
            ("X-RateLimit-Reset", "99999999999"),
        ],
    ] {
        let (client, _) = client(vec![Ok(response(429, &headers, "{}"))]);
        client.execute(&get, 1_000).unwrap();
        assert_eq!(
            client.execute(&get, 1_001).unwrap_err(),
            ApiError::RateLimited {
                retry_at: 1_000 + MAX_BACKOFF_SECONDS
            },
            "{headers:?}"
        );
    }
}

#[test]
fn a_backward_clock_jump_never_extends_a_backoff() {
    let get = ApiRequest::get(
        "personal",
        ApiUrl::parse("https://api.github.com/user").unwrap(),
    )
    .unwrap();
    let (client, _) = client(vec![
        Ok(response(429, &[("Retry-After", "120")], "{}")),
        Ok(response(200, &[], "{}")),
    ]);
    client.execute(&get, 10_000).unwrap();
    assert_eq!(
        client.execute(&get, 10_060).unwrap_err(),
        ApiError::RateLimited { retry_at: 10_120 }
    );
    // Further back than any backoff lasts: the deadline was set before the jump and is over.
    let earlier = 10_000 - MAX_BACKOFF_SECONDS;
    assert_eq!(client.execute(&get, earlier).unwrap().status, 200);
}

#[test]
fn a_reset_or_retry_date_in_the_past_waits_the_fallback_delay() {
    let get = ApiRequest::get(
        "personal",
        ApiUrl::parse("https://api.github.com/user").unwrap(),
    )
    .unwrap();
    // The local clock runs ahead of GitHub's: the reported deadline is already over.
    for headers in [
        vec![("X-RateLimit-Remaining", "0"), ("X-RateLimit-Reset", "900")],
        vec![
            ("X-RateLimit-Remaining", "0"),
            ("X-RateLimit-Reset", "1000"),
        ],
        vec![("Retry-After", "Thu, 01 Jan 1970 00:15:00 GMT")],
    ] {
        let (client, sent) = client(vec![Ok(response(429, &headers, "{}"))]);
        client.execute(&get, 1_000).unwrap();
        assert_eq!(
            client.execute(&get, 1_001).unwrap_err(),
            ApiError::RateLimited { retry_at: 1_060 },
            "{headers:?}"
        );
        assert_eq!(sent.lock().unwrap().len(), 1);
    }
}

#[test]
fn exhausted_quota_on_success_does_not_escalate_the_failure_backoff() {
    let get = ApiRequest::get(
        "personal",
        ApiUrl::parse("https://api.github.com/user").unwrap(),
    )
    .unwrap();
    let spent = || Ok(response(200, &[("X-RateLimit-Remaining", "0")], "{}"));
    let (client, _) = client(vec![
        spent(),
        spent(),
        Ok(response(429, &[], "{}")),
        Ok(response(429, &[], "{}")),
    ]);
    client.execute(&get, 100).unwrap();
    assert_eq!(
        client.execute(&get, 101).unwrap_err(),
        ApiError::RateLimited { retry_at: 160 }
    );
    client.execute(&get, 160).unwrap();
    // A second successful response keeps the base fallback instead of doubling it.
    assert_eq!(
        client.execute(&get, 161).unwrap_err(),
        ApiError::RateLimited { retry_at: 220 }
    );
    client.execute(&get, 220).unwrap();
    assert_eq!(
        client.execute(&get, 221).unwrap_err(),
        ApiError::RateLimited { retry_at: 280 }
    );
    client.execute(&get, 280).unwrap();
    assert_eq!(
        client.execute(&get, 281).unwrap_err(),
        ApiError::RateLimited { retry_at: 400 }
    );
}

#[test]
fn throttled_reads_are_rate_limits_and_plain_forbidden_stays_http() {
    let url = || ApiUrl::parse("https://api.github.com/user/repos?page=1").unwrap();
    let (listing, _) = client(vec![
        Ok(response(403, &[], r#"{"message":"Forbidden"}"#)),
        Ok(response(
            403,
            &[],
            r#"{"message":"You have exceeded a secondary rate limit."}"#,
        )),
    ]);
    assert_eq!(
        listing.get_pages("personal", url(), 100).unwrap_err(),
        ApiError::Http(403)
    );
    assert_eq!(
        listing.get_pages("personal", url(), 100).unwrap_err(),
        ApiError::RateLimited { retry_at: 160 }
    );
    let get = ApiRequest::get("work", url()).unwrap();
    let (single, sent) = client(vec![
        Ok(response(404, &[], "{}")),
        Ok(response(429, &[("Retry-After", "5")], "{}")),
    ]);
    assert_eq!(single.get(&get, 100).unwrap().status, 404);
    assert_eq!(
        single.get(&get, 100).unwrap_err(),
        ApiError::RateLimited { retry_at: 105 }
    );
    assert!(single.get(&dispatch_request(), 200).is_err());
    assert_eq!(sent.lock().unwrap().len(), 2);
}
