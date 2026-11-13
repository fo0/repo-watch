use super::*;
use crate::models::{Conclusion, DispatchState};
use std::collections::VecDeque;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

struct FakeTransport {
    responses: VecDeque<Result<ApiResponse, TransportError>>,
    requests: Arc<Mutex<Vec<ApiRequest>>>,
}

impl Transport for FakeTransport {
    fn send(&mut self, request: &ApiRequest) -> Result<ApiResponse, TransportError> {
        self.requests.lock().unwrap().push(request.clone());
        self.responses
            .pop_front()
            .expect("unexpected automatic retry")
    }
}

#[test]
#[cfg(feature = "network")]
fn moving_discovery_windows_follow_pagination_without_retaining_pages() {
    let (client, requests) = client(vec![
        Ok(response(200, &[("Link", "<https://api.github.com/repos/demo/project/actions/runs?created=2026-10-31&page=2>; rel=\"next\""), ("ETag", "\"first\"")], "{\"workflow_runs\":[]}")),
        Ok(response(200, &[("ETag", "\"second\"")], "{\"workflow_runs\":[]}")),
    ]);
    let pages = client
        .get_pages_uncached(
            "fixture",
            ApiUrl::parse(
                "https://api.github.com/repos/demo/project/actions/runs?created=2026-10-31",
            )
            .unwrap(),
            0,
        )
        .unwrap();
    assert_eq!(pages.bodies.len(), 2);
    assert!(!pages.truncated);
    assert_eq!(requests.lock().unwrap().len(), 2);
    assert!(client.state.lock().unwrap().pages.is_empty());
}

fn response(status: u16, headers: &[(&str, &str)], body: &str) -> ApiResponse {
    ApiResponse {
        status,
        headers: headers
            .iter()
            .map(|(name, value)| (name.to_string(), value.to_string()))
            .collect(),
        body: body.as_bytes().to_vec(),
    }
}

fn client(
    responses: Vec<Result<ApiResponse, TransportError>>,
) -> (ApiClient<FakeTransport>, Arc<Mutex<Vec<ApiRequest>>>) {
    let requests = Arc::new(Mutex::new(vec![]));
    (
        ApiClient::new(FakeTransport {
            responses: responses.into(),
            requests: requests.clone(),
        }),
        requests,
    )
}

fn dispatch_request() -> ApiRequest {
    ApiRequest::workflow_dispatch("personal", "octo", "demo", 42, "main", &Inputs::new()).unwrap()
}

#[test]
fn rejects_off_origin_urls_and_credentials_without_transport() {
    for value in [
        "http://api.github.com/user",
        "https://api.github.com.evil.example/user",
        "https://api.github.com@evil.example/user",
        "https://api.github.com:443/user",
        "https://api.github.com//evil.example/user",
        "https://api.github.com/user?access_token=hidden",
        "https://api.github.com/user?%74oken=hidden",
        "https://api.github.com/user#fragment",
        "https://api.github.com/repos/../user",
    ] {
        assert!(ApiUrl::parse(value).is_err(), "{value}");
    }
}

#[test]
fn paginates_with_independent_etags_and_reuses_cached_pages_on_304() {
    let first = "https://api.github.com/user/repos?page=1";
    let next = "<https://api.github.com/user/repos?page=2>; rel=\"next\"";
    let (client, sent) = client(vec![
        Ok(response(
            200,
            &[("ETag", "\"page1\""), ("Link", next)],
            "[1]",
        )),
        Ok(response(200, &[("ETag", "\"page2\"")], "[2]")),
        Ok(response(304, &[], "")),
        Ok(response(304, &[], "")),
        Ok(response(200, &[], "[9]")),
    ]);
    let pages = client
        .get_pages("personal", ApiUrl::parse(first).unwrap(), 100)
        .unwrap();
    assert_eq!(pages, vec![b"[1]".to_vec(), b"[2]".to_vec()]);
    assert_eq!(
        client
            .get_pages("personal", ApiUrl::parse(first).unwrap(), 101)
            .unwrap(),
        pages
    );
    client
        .get_pages("work", ApiUrl::parse(first).unwrap(), 102)
        .unwrap();
    let sent = sent.lock().unwrap();
    assert_eq!(sent[2].headers()["If-None-Match"], "\"page1\"");
    assert_eq!(sent[3].headers()["If-None-Match"], "\"page2\"");
    assert!(!sent[4].headers().contains_key("If-None-Match"));
    assert_eq!(sent[0].headers()["X-GitHub-Api-Version"], API_VERSION);
}

#[test]
fn app_installations_and_installation_repositories_follow_all_pages_independently() {
    let (client, sent) = client(vec![
        Ok(response(
            200,
            &[(
                "Link",
                "<https://api.github.com/user/installations?page=2>; rel=\"next\"",
            )],
            r#"{"installations":[{"id":1}]}"#,
        )),
        Ok(response(200, &[], r#"{"installations":[{"id":2}]}"#)),
        Ok(response(
            200,
            &[(
                "Link",
                "<https://api.github.com/user/installations/1/repositories?page=2>; rel=\"next\"",
            )],
            r#"{"repositories":[{"id":10}]}"#,
        )),
        Ok(response(200, &[], r#"{"repositories":[{"id":11}]}"#)),
    ]);
    assert_eq!(
        client
            .get_pages(
                "github-device-42",
                ApiUrl::parse("https://api.github.com/user/installations?page=1").unwrap(),
                0
            )
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        client
            .get_pages(
                "github-device-42",
                ApiUrl::parse("https://api.github.com/user/installations/1/repositories?page=1")
                    .unwrap(),
                0
            )
            .unwrap()
            .len(),
        2
    );
    assert_eq!(sent.lock().unwrap().len(), 4);
}

#[test]
fn pagination_rejects_foreign_hosts_changed_resources_and_cycles() {
    for link in [
        "<https://evil.example/page2>; rel=\"next\"",
        "<https://api.github.com/user/keys>; rel=\"next\"",
        "<https://api.github.com/user/repos?page=1>; rel=\"next\"",
    ] {
        let (client, sent) = client(vec![Ok(response(200, &[("Link", link)], "[]"))]);
        assert!(client
            .get_pages(
                "personal",
                ApiUrl::parse("https://api.github.com/user/repos?page=1").unwrap(),
                0
            )
            .is_err());
        assert_eq!(sent.lock().unwrap().len(), 1);
    }
}

#[test]
fn accepts_2026_dispatch_response_and_tracks_only_that_run() {
    let confirmation_response = response(
        200,
        &[],
        r#"{"workflow_run_id":77,"run_url":"https://api.github.com/repos/octo/demo/actions/runs/77","html_url":"https://github.com/octo/demo/actions/runs/77"}"#,
    );
    let confirmation = parse_dispatch_confirmation(&confirmation_response, "octo", "demo").unwrap();
    let tracker = RunTracker {
        account_id: "personal".into(),
        repo_id: "repo-id".into(),
        workflow_id: 42,
        confirmation,
    };
    assert_eq!(
        tracker.request().unwrap().url().as_str(),
        "https://api.github.com/repos/octo/demo/actions/runs/77"
    );
    let run_response = response(
        200,
        &[],
        r#"{"id":77,"workflow_id":42,"name":"Build","head_branch":"main","status":"completed","conclusion":"success","created_at":"2026-10-31T09:00:00Z","updated_at":"2026-10-31T09:01:00Z","html_url":"https://github.com/octo/demo/actions/runs/77"}"#,
    );
    let run = tracker.parse(&run_response).unwrap();
    assert_eq!(run.id, 77);
    assert_eq!(run.repo_id, "repo-id");
    assert_eq!(run.conclusion, Some(Conclusion::Success));
    let mut wrong = run_response;
    wrong.body = String::from_utf8(wrong.body)
        .unwrap()
        .replace("\"id\":77", "\"id\":78")
        .into_bytes();
    assert!(tracker.parse(&wrong).is_err());
    let (client, sent) = client(vec![Ok(confirmation_response)]);
    assert_eq!(
        client
            .dispatch(&dispatch_request(), "octo", "demo", 0)
            .state,
        DispatchState::Accepted
    );
    assert_eq!(sent.lock().unwrap().len(), 1);
}

#[test]
fn legacy_204_timeout_server_error_and_malformed_200_are_uncertain_never_retried() {
    for result in [
        Ok(response(204, &[], "")),
        Err(TransportError::OutcomeUnknown),
        Ok(response(500, &[], "{}")),
        Ok(response(200, &[], "{}")),
    ] {
        let (client, sent) = client(vec![result]);
        assert_eq!(
            client
                .dispatch(&dispatch_request(), "octo", "demo", 0)
                .state,
            DispatchState::Uncertain
        );
        assert_eq!(sent.lock().unwrap().len(), 1);
    }
    let (client, _) = client(vec![Err(TransportError::BeforeSend)]);
    assert_eq!(
        client
            .dispatch(&dispatch_request(), "octo", "demo", 0)
            .state,
        DispatchState::Rejected
    );
}

#[test]
fn central_queue_never_sends_concurrent_requests() {
    struct ConcurrencyTransport {
        active: Arc<AtomicUsize>,
        maximum: Arc<AtomicUsize>,
    }
    impl Transport for ConcurrencyTransport {
        fn send(&mut self, _: &ApiRequest) -> Result<ApiResponse, TransportError> {
            let count = self.active.fetch_add(1, Ordering::SeqCst) + 1;
            self.maximum.fetch_max(count, Ordering::SeqCst);
            std::thread::yield_now();
            self.active.fetch_sub(1, Ordering::SeqCst);
            Ok(response(200, &[], "[]"))
        }
    }
    let maximum = Arc::new(AtomicUsize::new(0));
    let client = Arc::new(ApiClient::new(ConcurrencyTransport {
        active: Arc::new(AtomicUsize::new(0)),
        maximum: maximum.clone(),
    }));
    let threads: Vec<_> = (0..8)
        .map(|index| {
            let client = client.clone();
            std::thread::spawn(move || {
                let request = ApiRequest::get(
                    &format!("account-{index}"),
                    ApiUrl::parse("https://api.github.com/user").unwrap(),
                )
                .unwrap();
                client.execute(&request, 0).unwrap();
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
    assert_eq!(maximum.load(Ordering::SeqCst), 1);
}

fn chained_pages(count: usize) -> Vec<Result<ApiResponse, TransportError>> {
    (1..=count)
        .map(|page| {
            let link = format!(
                "<https://api.github.com/user/repos?page={}>; rel=\"next\"",
                page + 1
            );
            Ok(response(
                200,
                &[("Link", link.as_str())],
                &format!("[{page}]"),
            ))
        })
        .collect()
}

#[test]
fn long_listings_fail_when_complete_and_report_their_cut_when_budgeted() {
    let first = || ApiUrl::parse("https://api.github.com/user/repos?page=1").unwrap();
    // A complete listing never passes its first pages off as all of it.
    let (full, sent) = client(chained_pages(MAX_PAGES));
    assert_eq!(
        full.get_pages("personal", first(), 0).unwrap_err(),
        ApiError::Truncated
    );
    assert_eq!(sent.lock().unwrap().len(), MAX_PAGES);
    let mut exact = chained_pages(MAX_PAGES - 1);
    exact.push(Ok(response(200, &[], &format!("[{MAX_PAGES}]"))));
    let (fits, _) = client(exact);
    assert_eq!(
        fits.get_pages("personal", first(), 0).unwrap().len(),
        MAX_PAGES
    );

    let (budgeted, sent) = client(chained_pages(3));
    let limited = budgeted
        .get_pages_limited("personal", first(), 0, 2)
        .unwrap();
    assert_eq!(limited.bodies, vec![b"[1]".to_vec(), b"[2]".to_vec()]);
    assert!(limited.truncated);
    assert_eq!(sent.lock().unwrap().len(), 2);

    let (single, _) = client(vec![Ok(response(200, &[], "[1]"))]);
    let complete = single.get_pages_limited("personal", first(), 0, 2).unwrap();
    assert!(!complete.truncated);
    assert_eq!(complete.bodies.len(), 1);
}

#[test]
fn user_facing_errors_are_german_with_a_local_clock_time() {
    assert_eq!(ApiError::Http(502).to_string(), "GitHub-Fehler 502.");
    let limited = ApiError::RateLimited { retry_at: 0 }.to_string();
    assert!(limited.starts_with("GitHub-Limit erreicht – neuer Versuch um "));
    assert!(!limited.contains("1970") && !limited.ends_with(" 0."));
    assert_eq!(local_time(u64::MAX), "--:--");
    assert!(ApiError::Transport(TransportError::BeforeSend)
        .to_string()
        .contains("nicht erreichbar"));
    assert_eq!(
        ApiError::Transport(TransportError::NoCredential).to_string(),
        "Zugang fehlt im Tresor – verbinde das Konto neu oder trenne es."
    );
    assert_eq!(
        ApiError::Transport(TransportError::VaultUnavailable).to_string(),
        "Betriebssystem-Tresor nicht verfügbar. Der Zugang kann nicht gelesen werden."
    );
    let english = crate::i18n::with_locale(crate::i18n::Locale::En, || {
        [
            ApiError::Http(502).to_string(),
            ApiError::Transport(TransportError::VaultUnavailable).to_string(),
        ]
    });
    assert_eq!(
        english,
        [
            "GitHub error 502.",
            "Operating system vault unavailable. The access cannot be read."
        ]
    );
}

#[test]
fn a_cancel_is_one_validated_bodiless_post_and_throttling_is_a_rate_limit() {
    let cancel = ApiRequest::cancel_run("personal", "octo", "demo", 77).unwrap();
    assert_eq!(cancel.method(), Method::Post);
    assert_eq!(
        cancel.url().as_str(),
        "https://api.github.com/repos/octo/demo/actions/runs/77/cancel"
    );
    assert!(cancel.body().is_none());
    assert!(!cancel.headers().contains_key("Content-Type"));
    assert_eq!(cancel.headers()["X-GitHub-Api-Version"], API_VERSION);
    for (owner, repo, run_id) in [
        ("octo/x", "demo", 1),
        ("octo", "..", 1),
        ("", "demo", 1),
        ("octo", "demo", 0),
    ] {
        assert!(ApiRequest::cancel_run("personal", owner, repo, run_id).is_err());
    }
    let (client, sent) = client(vec![
        Ok(response(202, &[], "{}")),
        Ok(response(403, &[], r#"{"message":"Forbidden"}"#)),
        Ok(response(429, &[("Retry-After", "5")], "{}")),
    ]);
    assert_eq!(client.post(&cancel, 100).unwrap().status, 202);
    assert_eq!(client.post(&cancel, 100).unwrap().status, 403);
    assert_eq!(
        client.post(&cancel, 100).unwrap_err(),
        ApiError::RateLimited { retry_at: 105 }
    );
    // During the backoff nothing is sent, and a GET never goes out as a POST.
    assert_eq!(
        client.post(&cancel, 101).unwrap_err(),
        ApiError::RateLimited { retry_at: 105 }
    );
    let read = ApiRequest::get(
        "personal",
        ApiUrl::parse("https://api.github.com/user").unwrap(),
    );
    assert!(client.post(&read.unwrap(), 200).is_err());
    assert_eq!(sent.lock().unwrap().len(), 3);
}

/// Rate limits and backoff.
mod backoff;
/// Run conversion from the wire format and the dispatch run tracker.
mod runs;
