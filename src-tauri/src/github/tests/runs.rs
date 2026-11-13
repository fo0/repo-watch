//! Run conversion from the GitHub wire format and the dispatch run tracker.
use super::super::*;
use super::response;
use crate::models::{Conclusion, RunStatus};

fn wire_run(status: Option<&str>, conclusion: Option<&str>) -> serde_json::Value {
    serde_json::json!({"id":77,"workflow_id":42,"name":"Build","head_branch":"main","status":status,"conclusion":conclusion,"created_at":"2026-10-31T09:00:00Z","updated_at":"2026-10-31T09:01:00Z","html_url":"https://github.com/octo/demo/actions/runs/77"})
}

#[test]
fn every_run_status_maps_once_and_unknown_values_never_stay_open_after_a_conclusion() {
    for (status, expected) in [
        ("queued", RunStatus::Queued),
        ("requested", RunStatus::Queued),
        ("in_progress", RunStatus::InProgress),
        ("waiting", RunStatus::Waiting),
        ("pending", RunStatus::Waiting),
        ("completed", RunStatus::Completed),
        ("future_status", RunStatus::Queued),
    ] {
        let run = run_from_wire("repo", &wire_run(Some(status), None)).unwrap();
        assert_eq!(run.status, expected, "{status}");
        assert_eq!(run.conclusion, None, "{status}");
    }
    let missing = run_from_wire("repo", &wire_run(None, None)).unwrap();
    assert_eq!(missing.status, RunStatus::Queued);
    // A conclusion is terminal even when the status name is unknown or absent.
    for status in [Some("future_status"), None] {
        let run = run_from_wire("repo", &wire_run(status, Some("success"))).unwrap();
        assert_eq!(run.status, RunStatus::Completed);
        assert_eq!(run.conclusion, Some(Conclusion::Success));
    }
    // An open status keeps the run open; a contradictory conclusion is not shown.
    let open = run_from_wire("repo", &wire_run(Some("in_progress"), Some("failure"))).unwrap();
    assert_eq!(
        (open.status, open.conclusion),
        (RunStatus::InProgress, None)
    );
}

#[test]
fn every_run_conclusion_maps_once_and_unknown_outcomes_complete_unclassified() {
    for (conclusion, expected) in [
        ("success", Some(Conclusion::Success)),
        ("failure", Some(Conclusion::Failure)),
        ("timed_out", Some(Conclusion::Failure)),
        ("action_required", Some(Conclusion::Failure)),
        ("startup_failure", Some(Conclusion::Failure)),
        ("cancelled", Some(Conclusion::Cancelled)),
        ("skipped", Some(Conclusion::Cancelled)),
        ("neutral", Some(Conclusion::Cancelled)),
        ("stale", Some(Conclusion::Cancelled)),
        ("future_outcome", None),
    ] {
        let run = run_from_wire("repo", &wire_run(Some("completed"), Some(conclusion))).unwrap();
        assert_eq!(run.status, RunStatus::Completed, "{conclusion}");
        assert_eq!(run.conclusion, expected, "{conclusion}");
    }
}

#[test]
fn run_conversion_rejects_missing_identity_and_malformed_timestamps() {
    let mut value = wire_run(Some("completed"), Some("success"));
    value["name"] = serde_json::Value::Null;
    value["display_title"] = serde_json::json!("Fix build");
    value["head_branch"] = serde_json::Value::Null;
    let run = run_from_wire("repo", &value).unwrap();
    assert_eq!((run.name.as_str(), run.branch.as_str()), ("Fix build", ""));
    for (key, bad) in [
        ("id", serde_json::json!(0)),
        ("workflow_id", serde_json::Value::Null),
        ("created_at", serde_json::json!("yesterday")),
        ("updated_at", serde_json::Value::Null),
        ("html_url", serde_json::Value::Null),
    ] {
        let mut value = wire_run(Some("completed"), Some("success"));
        value[key] = bad;
        assert!(run_from_wire("repo", &value).is_err(), "{key}");
    }
}

#[test]
fn tracker_completes_runs_with_every_terminal_conclusion() {
    let confirmation = parse_dispatch_confirmation(
        &response(
            200,
            &[],
            r#"{"workflow_run_id":77,"run_url":"https://api.github.com/repos/octo/demo/actions/runs/77","html_url":"https://github.com/octo/demo/actions/runs/77"}"#,
        ),
        "octo",
        "demo",
    )
    .unwrap();
    let tracker = RunTracker {
        account_id: "personal".into(),
        repo_id: "repo-id".into(),
        workflow_id: 42,
        confirmation,
    };
    for conclusion in [
        "timed_out",
        "skipped",
        "neutral",
        "startup_failure",
        "future",
    ] {
        let body = wire_run(Some("completed"), Some(conclusion)).to_string();
        let run = tracker.parse(&response(200, &[], &body)).unwrap();
        assert_eq!(run.status, RunStatus::Completed, "{conclusion}");
    }
    let pending = wire_run(Some("pending"), None).to_string();
    assert_eq!(
        tracker.parse(&response(200, &[], &pending)).unwrap().status,
        RunStatus::Waiting
    );
    assert!(resource_gone(404) && resource_gone(410) && !resource_gone(403));
}
