use super::*;
use crate::github::{ApiResponse, ApiUrl, DispatchConfirmation, TransportError};
use std::sync::{Arc, Mutex};

/// A pass at `now` with the default refresh interval.
fn pace(now: u64) -> Pace {
    Pace {
        now,
        idle_seconds: crate::refresh_interval::DEFAULT_SECONDS,
    }
}

#[derive(Default)]
struct Remote {
    calls: Vec<String>,
    active: Vec<Value>,
    recent: Vec<Value>,
    created: Vec<Value>,
    resolved: BTreeMap<u64, Value>,
    gone: BTreeSet<u64>,
    fail_resolution: bool,
    /// Further accounts besides `fixture-account` that may send requests.
    accounts: Vec<&'static str>,
    /// HTTP status answered for every request under `/repos/<owner>/<name>/`.
    failing: BTreeMap<&'static str, u16>,
    /// Every active-run list page links a next page, as a listing past the page budget does.
    endless_active: bool,
    /// Every discovery-window page links a next page, as a burst of runs past the budget does.
    endless_created: bool,
}
struct Mock(Arc<Mutex<Remote>>);
impl Transport for Mock {
    fn send(&mut self, request: &ApiRequest) -> Result<ApiResponse, TransportError> {
        let mut remote = self.0.lock().unwrap();
        assert!(
            request.account_id() == "fixture-account"
                || remote.accounts.contains(&request.account_id()),
            "unexpected account {}",
            request.account_id()
        );
        let uri = reqwest::Url::parse(request.url().as_str()).unwrap();
        remote.calls.push(uri.to_string());
        let failing = remote
            .failing
            .iter()
            .find(|(repo, _)| uri.path().starts_with(&format!("/repos/{repo}/")))
            .map(|(_, status)| *status);
        if let Some(status) = failing {
            return Ok(ApiResponse {
                status,
                headers: BTreeMap::new(),
                body: b"{}".to_vec(),
            });
        }
        let query: BTreeMap<_, _> = uri.query_pairs().into_owned().collect();
        let mut headers = BTreeMap::new();
        if (remote.endless_active && query.contains_key("status"))
            || (remote.endless_created && query.contains_key("created"))
        {
            let page = query
                .get("page")
                .map_or(1, |page| page.parse::<u64>().unwrap());
            let mut next = uri.clone();
            next.query_pairs_mut()
                .clear()
                .extend_pairs(query.iter().filter(|(key, _)| *key != "page"))
                .append_pair("page", &(page + 1).to_string());
            headers.insert("Link".to_string(), format!("<{next}>; rel=\"next\""));
        }
        let body = if let Some(status) = query.get("status") {
            serde_json::json!({"workflow_runs":remote.active.iter().filter(|run| run["status"] == *status).collect::<Vec<_>>()})
        } else if query.contains_key("created") {
            serde_json::json!({"workflow_runs": remote.created})
        } else if uri.path().ends_with("/runs") {
            serde_json::json!({"workflow_runs": remote.recent})
        } else {
            if remote.fail_resolution {
                return Err(TransportError::BeforeSend);
            }
            let id = uri
                .path_segments()
                .unwrap()
                .next_back()
                .unwrap()
                .parse::<u64>()
                .unwrap();
            if remote.gone.contains(&id) {
                return Ok(ApiResponse {
                    status: 404,
                    headers: BTreeMap::new(),
                    body: br#"{"message":"Not Found"}"#.to_vec(),
                });
            }
            remote
                .resolved
                .get(&id)
                .expect("unexpected terminal run poll")
                .clone()
        };
        Ok(ApiResponse {
            status: 200,
            headers,
            body: serde_json::to_vec(&body).unwrap(),
        })
    }
}
fn repo() -> Repo {
    Repo {
        id: "fixture-repo".into(),
        account_id: "fixture-account".into(),
        owner: "fixture-owner".into(),
        name: "fixture-name".into(),
        default_branch: "main".into(),
        pushed_at: None,
        selected: true,
        permission: crate::models::Permission::Write,
    }
}
fn other_repo(id: &str, account_id: &str, name: &str) -> Repo {
    Repo {
        id: id.into(),
        account_id: account_id.into(),
        name: name.into(),
        ..repo()
    }
}
fn run(id: u64, status: &str) -> Value {
    serde_json::json!({"id":id,"workflow_id":101,"name":"Fixture action","head_branch":"main","status":status,"conclusion":if status == "completed" {Some("success")}else{None},"created_at":"2026-10-31T10:00:00Z","updated_at":"2026-10-31T10:01:00Z","html_url":format!("https://github.com/fixture-owner/fixture-name/actions/runs/{id}")})
}
fn parse_run(repo_id: &str, value: &Value) -> Result<Run, ApiError> {
    run_from_wire(repo_id, value)
}
#[test]
fn follows_open_run_to_completion_once_and_never_polls_completed_ids() {
    let remote = Arc::new(Mutex::new(Remote {
        active: vec![run(2, "in_progress")],
        recent: vec![run(1, "completed"), run(2, "in_progress")],
        ..Remote::default()
    }));
    let client = ApiClient::new(Mock(remote.clone()));
    let initial = refresh_repo_runs(&client, &repo(), &[], None, 100).unwrap();
    assert_eq!(initial.len(), 2);
    let frozen = initial.iter().find(|r| r.id == 1).unwrap().clone();
    {
        let mut remote = remote.lock().unwrap();
        remote.calls.clear();
        remote.active.clear();
        let mut finished = run(2, "completed");
        finished["conclusion"] = serde_json::json!("failure");
        remote.resolved.insert(2, finished);
    }
    let finished = refresh_repo_runs(&client, &repo(), &initial, Some(100), 115).unwrap();
    assert!(finished.iter().all(|r| r.status == RunStatus::Completed));
    assert_eq!(finished.iter().find(|r| r.id == 1).unwrap(), &frozen);
    let calls = remote.lock().unwrap().calls.clone();
    assert!(calls.iter().any(|uri| uri.ends_with("/runs/2")));
    assert!(calls.iter().all(|uri| !uri.ends_with("/runs/1")));
    assert!(calls
        .iter()
        .filter(|uri| uri.contains('?'))
        .all(|uri| uri.contains("status=") || uri.contains("created=")));
    remote.lock().unwrap().calls.clear();
    assert_eq!(
        refresh_repo_runs(&client, &repo(), &finished, Some(115), 130).unwrap(),
        finished
    );
    assert!(remote
        .lock()
        .unwrap()
        .calls
        .iter()
        .all(|uri| !uri.ends_with("/runs/2") && !uri.ends_with("/runs/1")));
}
#[test]
fn discovers_short_new_runs_and_keeps_terminal_metadata_frozen_even_during_reseed() {
    let old = parse_run(&repo().id, &run(1, "completed")).unwrap();
    let mut changed = run(1, "completed");
    changed["updated_at"] = serde_json::json!("2026-10-31T12:00:00Z");
    let remote = Arc::new(Mutex::new(Remote {
        recent: vec![changed],
        created: vec![run(3, "completed")],
        ..Remote::default()
    }));
    let client = ApiClient::new(Mock(remote));
    let seeded =
        refresh_repo_runs(&client, &repo(), std::slice::from_ref(&old), None, 100).unwrap();
    assert_eq!(seeded, vec![old.clone()]);
    let next = refresh_repo_runs(&client, &repo(), &seeded, Some(100), 115).unwrap();
    assert_eq!(next.len(), 2);
    assert_eq!(next.iter().find(|r| r.id == 1).unwrap(), &old);
    assert!(next
        .iter()
        .any(|r| r.id == 3 && r.status == RunStatus::Completed));
}
#[test]
fn missing_open_run_is_not_guessed_completed_on_network_failure() {
    let open = parse_run(&repo().id, &run(2, "in_progress")).unwrap();
    let remote = Arc::new(Mutex::new(Remote {
        fail_resolution: true,
        ..Remote::default()
    }));
    let client = ApiClient::new(Mock(remote));
    assert!(refresh_repo_runs(
        &client,
        &repo(),
        std::slice::from_ref(&open),
        Some(100),
        115
    )
    .is_err());
    assert_eq!(open.status, RunStatus::InProgress);
}
#[test]
fn deleted_open_run_leaves_tracking_instead_of_failing_every_poll() {
    let deleted = parse_run(&repo().id, &run(2, "in_progress")).unwrap();
    let kept = parse_run(&repo().id, &run(3, "waiting")).unwrap();
    let remote = Arc::new(Mutex::new(Remote {
        active: vec![run(3, "waiting")],
        gone: BTreeSet::from([2]),
        ..Remote::default()
    }));
    let client = ApiClient::new(Mock(remote.clone()));
    let next =
        refresh_repo_runs(&client, &repo(), &[deleted, kept.clone()], Some(100), 115).unwrap();
    assert_eq!(next, vec![kept.clone()]);
    remote.lock().unwrap().calls.clear();
    assert_eq!(
        refresh_repo_runs(&client, &repo(), &next, Some(115), 130).unwrap(),
        vec![kept]
    );
    assert!(remote
        .lock()
        .unwrap()
        .calls
        .iter()
        .all(|uri| !uri.ends_with("/runs/2")));
}
#[test]
fn bounds_completed_cache_without_losing_old_active_runs_or_mixing_repositories() {
    let mut previous: Vec<_> = (1..=70)
        .map(|id| parse_run(&repo().id, &run(id, "completed")).unwrap())
        .collect();
    let active = parse_run(&repo().id, &run(80, "waiting")).unwrap();
    previous.push(active.clone());
    previous.push(parse_run("another-account-repo", &run(90, "completed")).unwrap());
    let remote = Arc::new(Mutex::new(Remote {
        active: vec![run(80, "waiting")],
        ..Remote::default()
    }));
    let client = ApiClient::new(Mock(remote));
    let history = refresh_repo_runs(&client, &repo(), &previous, Some(100), 115).unwrap();
    assert_eq!(history.len(), COMPLETED_LIMIT + 1);
    assert!(history.contains(&active));
    assert!(history.iter().all(|r| r.repo_id == repo().id));
}
#[test]
fn one_failing_repository_keeps_its_last_runs_while_the_others_refresh() {
    let healthy = repo();
    let broken = other_repo("broken-repo", "fixture-account", "broken-name");
    let denied = other_repo("denied-repo", "fixture-account", "denied-name");
    let previous = vec![
        parse_run(&broken.id, &run(5, "in_progress")).unwrap(),
        parse_run(&denied.id, &run(6, "completed")).unwrap(),
    ];
    let remote = Arc::new(Mutex::new(Remote {
        recent: vec![run(1, "completed")],
        failing: BTreeMap::from([
            ("fixture-owner/broken-name", 502),
            ("fixture-owner/denied-name", 403),
        ]),
        ..Remote::default()
    }));
    let client = ApiClient::new(Mock(remote));
    let mut blocked = BTreeSet::new();
    let pass = refresh_selected(
        &client,
        &[broken.clone(), healthy.clone(), denied.clone()],
        &previous,
        &BTreeMap::new(),
        &BTreeMap::new(),
        &mut blocked,
        pace(100),
    );
    assert_eq!(pass.polled, vec![(healthy.id.clone(), 100)]);
    assert!(pass
        .runs
        .iter()
        .any(|r| r.repo_id == healthy.id && r.id == 1));
    assert!(previous.iter().all(|old| pass.runs.contains(old)));
    assert_eq!(
        pass.failures,
        vec![
            (broken, ScopeError::Transient(Cause::Http(502))),
            (denied, ScopeError::Denied(Cause::Http(403))),
        ]
    );
    assert!(blocked.is_empty());
}
#[test]
fn unauthorized_account_skips_its_other_repositories_and_not_other_accounts() {
    let revoked_first = other_repo("revoked-1", "fixture-revoked", "revoked-one");
    let revoked_second = other_repo("revoked-2", "fixture-revoked", "revoked-two");
    let healthy = repo();
    let remote = Arc::new(Mutex::new(Remote {
        accounts: vec!["fixture-revoked"],
        failing: BTreeMap::from([
            ("fixture-owner/revoked-one", 401),
            ("fixture-owner/revoked-two", 401),
        ]),
        ..Remote::default()
    }));
    let client = ApiClient::new(Mock(remote.clone()));
    let mut blocked = BTreeSet::new();
    let pass = refresh_selected(
        &client,
        &[revoked_first.clone(), revoked_second, healthy.clone()],
        &[],
        &BTreeMap::new(),
        &BTreeMap::new(),
        &mut blocked,
        pace(100),
    );
    assert_eq!(pass.failures.len(), 1);
    assert!(matches!(
        pass.failures[0],
        (ref repo, ScopeError::Unauthorized(_)) if repo == &revoked_first
    ));
    assert_eq!(pass.polled, vec![(healthy.id, 100)]);
    assert_eq!(blocked, BTreeSet::from(["fixture-revoked".to_string()]));
    assert!(remote
        .lock()
        .unwrap()
        .calls
        .iter()
        .all(|uri| !uri.contains("revoked-two")));
}
#[test]
fn repositories_inside_their_poll_interval_keep_runs_without_requests() {
    let open = parse_run(&repo().id, &run(2, "in_progress")).unwrap();
    let remote = Arc::new(Mutex::new(Remote::default()));
    let client = ApiClient::new(Mock(remote.clone()));
    let poll_at = BTreeMap::from([(repo().id, 100)]);
    let pass = refresh_selected(
        &client,
        &[repo()],
        std::slice::from_ref(&open),
        &poll_at,
        &BTreeMap::new(),
        &mut BTreeSet::new(),
        pace(114),
    );
    assert_eq!(pass.runs, vec![open]);
    assert!(pass.polled.is_empty() && pass.failures.is_empty());
    assert!(remote.lock().unwrap().calls.is_empty());
}
#[test]
fn an_idle_repository_waits_the_chosen_interval_and_an_open_run_still_polls_every_15_seconds() {
    let done = parse_run(&repo().id, &run(1, "completed")).unwrap();
    let open = parse_run(&repo().id, &run(2, "in_progress")).unwrap();
    let client = ApiClient::new(Mock(Arc::new(Mutex::new(Remote {
        resolved: BTreeMap::from([(2, run(2, "in_progress"))]),
        ..Remote::default()
    }))));
    let poll_at = BTreeMap::from([(repo().id, 100)]);
    let polled = |previous: &Run, now: u64| {
        !refresh_selected(
            &client,
            &[repo()],
            std::slice::from_ref(previous),
            &poll_at,
            &BTreeMap::new(),
            &mut BTreeSet::new(),
            Pace {
                now,
                idle_seconds: 300,
            },
        )
        .polled
        .is_empty()
    };
    assert!(!polled(&done, 100 + 60), "the default no longer applies");
    assert!(!polled(&done, 100 + 299));
    assert!(polled(&done, 100 + 300));
    assert!(polled(&open, 100 + 15));
}
fn tracker(id: u64, repo_id: &str) -> RunTracker {
    RunTracker {
        account_id: "fixture-account".into(),
        repo_id: repo_id.into(),
        workflow_id: 101,
        confirmation: DispatchConfirmation {
            workflow_run_id: id,
            run_url: ApiUrl::parse(&format!(
                "https://api.github.com/repos/fixture-owner/fixture-name/actions/runs/{id}"
            ))
            .unwrap(),
            html_url: format!("https://github.com/fixture-owner/fixture-name/actions/runs/{id}"),
        },
    }
}
#[test]
fn dispatched_runs_stop_tracking_when_deleted_or_finished_with_any_conclusion() {
    let mut timed_out = run(11, "completed");
    timed_out["conclusion"] = serde_json::json!("timed_out");
    let remote = Arc::new(Mutex::new(Remote {
        resolved: BTreeMap::from([(11, timed_out), (12, run(12, "requested"))]),
        gone: BTreeSet::from([10]),
        ..Remote::default()
    }));
    let client = ApiClient::new(Mock(remote.clone()));
    let mut tracked = BTreeMap::from([
        (10, tracker(10, &repo().id)),
        (11, tracker(11, &repo().id)),
        (12, tracker(12, &repo().id)),
        (13, tracker(13, "unselected-repo")),
    ]);
    let mut runs = vec![];
    follow_dispatched(
        &client,
        &mut tracked,
        &BTreeSet::from([repo().id]),
        &mut runs,
        100,
    );
    assert_eq!(tracked.keys().copied().collect::<Vec<_>>(), vec![12, 13]);
    assert_eq!(runs.len(), 2);
    assert!(runs
        .iter()
        .any(|r| r.id == 11 && r.status == RunStatus::Completed));
    assert!(runs
        .iter()
        .any(|r| r.id == 12 && r.status == RunStatus::Queued));
    assert!(remote
        .lock()
        .unwrap()
        .calls
        .iter()
        .all(|uri| !uri.ends_with("/runs/13")));
    // A listed open run waits without a request; a listed completed one leaves tracking.
    remote.lock().unwrap().calls.clear();
    runs.retain(|r| r.id != 12);
    runs.push(parse_run(&repo().id, &run(12, "completed")).unwrap());
    follow_dispatched(
        &client,
        &mut tracked,
        &BTreeSet::from([repo().id]),
        &mut runs,
        115,
    );
    assert_eq!(tracked.keys().copied().collect::<Vec<_>>(), vec![13]);
    assert!(remote.lock().unwrap().calls.is_empty());
}
#[test]
fn dispatched_run_survives_a_transient_lookup_failure() {
    let remote = Arc::new(Mutex::new(Remote {
        fail_resolution: true,
        ..Remote::default()
    }));
    let client = ApiClient::new(Mock(remote));
    let mut tracked = BTreeMap::from([(10, tracker(10, &repo().id))]);
    let mut runs = vec![];
    follow_dispatched(
        &client,
        &mut tracked,
        &BTreeSet::from([repo().id]),
        &mut runs,
        100,
    );
    assert!(tracked.contains_key(&10));
    assert!(runs.is_empty());
}

/// Scope failures, page budgets and their texts.
mod scopes;
/// Stuck runs and their recovery.
mod stuck;
