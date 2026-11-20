//! Stuck runs: queued a day after their last change, re-runs and late completion.
use super::super::*;
use super::{pace, repo, run, Mock, Remote};
use std::sync::{Arc, Mutex};

#[test]
fn a_run_github_still_queues_a_day_after_its_last_change_is_stuck_and_not_open() {
    let created = chrono::DateTime::parse_from_rfc3339("2026-10-31T10:01:00Z")
        .unwrap()
        .timestamp() as u64;
    let mut requested = run(3, "requested");
    requested["status"] = serde_json::json!("requested");
    let remote = Arc::new(Mutex::new(Remote {
        active: vec![run(1, "queued"), run(2, "waiting"), requested],
        ..Remote::default()
    }));
    let client = ApiClient::new(Mock(remote));
    let status = |runs: &[Run], id: u64| runs.iter().find(|r| r.id == id).unwrap().status;

    let day = created + STUCK_AFTER_SECONDS;
    let fresh = refresh_repo_runs(&client, &repo(), &[], Some(day - 15), day).unwrap();
    assert_eq!(
        status(&fresh, 1),
        RunStatus::Queued,
        "24 h exactly still waits"
    );

    let later = day + 1;
    let runs = refresh_repo_runs(&client, &repo(), &fresh, Some(day), later).unwrap();
    assert_eq!(status(&runs, 1), RunStatus::Stuck);
    assert_eq!(
        status(&runs, 3),
        RunStatus::Stuck,
        "requested counts as queued"
    );
    // An approval or concurrency wait may last days and stays open.
    assert_eq!(status(&runs, 2), RunStatus::Waiting);
    assert!(!RunStatus::Stuck.is_open());

    // Only stuck runs left: the repository returns to the idle pace instead of 15 seconds.
    let stuck: Vec<_> = runs
        .into_iter()
        .filter(|r| r.status == RunStatus::Stuck)
        .collect();
    let poll_at = BTreeMap::from([(repo().id, later)]);
    let pass = refresh_selected(
        &client,
        &[repo()],
        &stuck,
        &poll_at,
        &BTreeMap::new(),
        &mut BTreeSet::new(),
        pace(later + crate::refresh_interval::OPEN_RUN_SECONDS),
    );
    assert!(pass.polled.is_empty());
    assert_eq!(pass.runs, stuck);
}

#[test]
fn a_rerun_of_an_old_run_is_queued_not_stuck_and_a_stuck_run_github_finishes_is_updated() {
    let updated = chrono::DateTime::parse_from_rfc3339("2026-10-31T10:01:00Z")
        .unwrap()
        .timestamp() as u64;
    let now = updated + 3 * STUCK_AFTER_SECONDS;
    // A re-run keeps its old created_at; its fresh updated_at says it was queued just now.
    let mut rerun = run(1, "queued");
    rerun["updated_at"] = serde_json::json!(chrono::DateTime::from_timestamp(now as i64 - 60, 0)
        .unwrap()
        .to_rfc3339());
    let remote = Arc::new(Mutex::new(Remote {
        active: vec![rerun, run(2, "queued")],
        ..Remote::default()
    }));
    let client = ApiClient::new(Mock(remote.clone()));
    let runs = refresh_repo_runs(&client, &repo(), &[], Some(now - 15), now).unwrap();
    let status = |runs: &[Run], id: u64| runs.iter().find(|r| r.id == id).unwrap().status;
    assert_eq!(status(&runs, 1), RunStatus::Queued);
    assert_eq!(status(&runs, 2), RunStatus::Stuck);

    // GitHub finally cancels the orphan: it leaves the queued list and its exact ID resolves.
    {
        let mut remote = remote.lock().unwrap();
        remote.active.retain(|run| run["id"] != 2);
        let mut cancelled = run(2, "completed");
        cancelled["conclusion"] = serde_json::json!("cancelled");
        remote.resolved.insert(2, cancelled);
    }
    let runs = refresh_repo_runs(&client, &repo(), &runs, Some(now), now + 60).unwrap();
    let finished = runs.iter().find(|r| r.id == 2).unwrap();
    assert_eq!(finished.status, RunStatus::Completed);
    assert_eq!(
        finished.conclusion,
        Some(crate::models::Conclusion::Cancelled)
    );
}
