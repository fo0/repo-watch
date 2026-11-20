//! Scope failures: denied, unauthorized and transient errors, page budgets and their texts.
use super::super::*;
use super::{pace, parse_run, repo, run, Mock, Remote};
use crate::github::TransportError;
use std::sync::{Arc, Mutex};

#[test]
fn scope_errors_separate_denied_repositories_from_accounts_and_transient_failures() {
    assert!(matches!(
        ScopeError::from(ApiError::Http(404)),
        ScopeError::Denied(_)
    ));
    assert!(matches!(
        ScopeError::from(ApiError::Http(401)),
        ScopeError::Unauthorized(_)
    ));
    assert!(matches!(
        ScopeError::from(ApiError::Http(403)).for_account(),
        ScopeError::Unauthorized(_)
    ));
    for transient in [
        ApiError::Http(500),
        ApiError::RateLimited { retry_at: 1 },
        ApiError::Transport(TransportError::BeforeSend),
    ] {
        assert!(matches!(
            ScopeError::from(transient),
            ScopeError::Transient(_)
        ));
    }
    // A missing or unreadable credential never reads as a network problem.
    assert_eq!(
        ScopeError::from(ApiError::Transport(TransportError::NoCredential)),
        ScopeError::Unauthorized(Cause::MissingCredential)
    );
    assert_eq!(
        ScopeError::from(ApiError::Transport(TransportError::VaultUnavailable)),
        ScopeError::Unauthorized(Cause::VaultUnavailable)
    );
    assert_eq!(
        ScopeError::from(CredentialError::Missing),
        ScopeError::Unauthorized(Cause::MissingCredential)
    );
    assert_eq!(
        ScopeError::from(CredentialError::Unusable("Fixture-Text.".into())),
        ScopeError::Unauthorized(Cause::Text("Fixture-Text.".into()))
    );
}
#[test]
fn an_active_run_list_past_the_page_budget_fails_the_repository_instead_of_hiding_runs() {
    let remote = Arc::new(Mutex::new(Remote {
        active: vec![run(2, "queued")],
        endless_active: true,
        ..Remote::default()
    }));
    let client = ApiClient::new(Mock(remote));
    let known = vec![parse_run("fixture-repo", &run(1, "in_progress")).unwrap()];
    let error = refresh_repo_runs(&client, &repo(), &known, Some(100), 115).unwrap_err();
    assert_eq!(error, ScopeError::Transient(Cause::Truncated));
    assert_eq!(
        error.describe("fixture-alex", Some("fixture-owner/fixture-name")),
        "Liste von fixture-owner/fixture-name unvollständig – zu viele Einträge."
    );
    let mut blocked = BTreeSet::new();
    let pass = refresh_selected(
        &client,
        &[repo()],
        &known,
        &BTreeMap::new(),
        &BTreeMap::from([("fixture-repo".to_string(), 100)]),
        &mut blocked,
        pace(115),
    );
    // The repository keeps its last known runs and is reported, not polled.
    assert_eq!(pass.runs, known);
    assert!(pass.polled.is_empty());
    assert_eq!(pass.failures.len(), 1);
}
#[test]
fn a_discovery_window_past_the_page_budget_keeps_its_newest_runs_and_the_repository_polling() {
    let remote = Arc::new(Mutex::new(Remote {
        created: vec![run(3, "completed")],
        endless_created: true,
        ..Remote::default()
    }));
    let client = ApiClient::new(Mock(remote.clone()));
    let mut blocked = BTreeSet::new();
    let first = refresh_selected(
        &client,
        &[repo()],
        &[],
        &BTreeMap::new(),
        &BTreeMap::from([("fixture-repo".to_string(), 100)]),
        &mut blocked,
        pace(200),
    );
    // Only the newest runs of the window matter; the cut is accepted, not a failure.
    assert!(first.failures.is_empty(), "{:?}", first.failures);
    assert_eq!(first.polled, vec![("fixture-repo".to_string(), 200)]);
    assert_eq!(first.runs.iter().map(|run| run.id).collect::<Vec<_>>(), [3]);
    let windows = |remote: &Remote| {
        remote
            .calls
            .iter()
            .filter(|call| call.contains("created="))
            .count()
    };
    assert_eq!(windows(&remote.lock().unwrap()), 20, "the page budget");
    // The next window opens where this pass ended, so the repository keeps updating.
    remote.lock().unwrap().created = vec![run(4, "in_progress"), run(3, "completed")];
    let second = refresh_selected(
        &client,
        &[repo()],
        &first.runs,
        &BTreeMap::from([("fixture-repo".to_string(), 200)]),
        &BTreeMap::from([("fixture-repo".to_string(), 200)]),
        &mut blocked,
        pace(300),
    );
    assert!(second.failures.is_empty(), "{:?}", second.failures);
    assert_eq!(second.polled, vec![("fixture-repo".to_string(), 300)]);
    let mut ids: Vec<_> = second.runs.iter().map(|run| run.id).collect();
    ids.sort_unstable();
    assert_eq!(ids, [3, 4]);
    let opened = format!(
        ">={}",
        chrono::DateTime::from_timestamp(198, 0)
            .unwrap()
            .to_rfc3339()
    );
    assert!(remote.lock().unwrap().calls.iter().any(|call| {
        reqwest::Url::parse(call)
            .unwrap()
            .query_pairs()
            .any(|(key, value)| key == "created" && value == opened)
    }));
}

#[test]
fn every_scope_failure_reads_as_german_and_names_its_scope() {
    let describe = |error: ApiError, repo| ScopeError::from(error).describe("fixture-alex", repo);
    assert_eq!(
        describe(ApiError::Http(401), Some("fixture-owner/fixture-name")),
        "Zugang von fixture-alex abgelaufen oder widerrufen – verbinde das Konto neu."
    );
    assert_eq!(
        describe(ApiError::Http(502), Some("fixture-owner/fixture-name")),
        "GitHub-Fehler 502 bei fixture-owner/fixture-name."
    );
    assert_eq!(
        describe(ApiError::Http(500), None),
        "GitHub-Fehler 500 bei fixture-alex."
    );
    assert_eq!(
        describe(ApiError::Transport(TransportError::OutcomeUnknown), None),
        "GitHub nicht erreichbar – prüfe deine Verbindung."
    );
    assert_eq!(
        describe(
            ApiError::Transport(TransportError::NoCredential),
            Some("fixture-owner/fixture-name")
        ),
        "Zugang von fixture-alex fehlt im Tresor – verbinde das Konto neu oder trenne es."
    );
    assert_eq!(
        describe(ApiError::Transport(TransportError::VaultUnavailable), None),
        "Betriebssystem-Tresor nicht verfügbar – Zugang von fixture-alex kann nicht gelesen werden."
    );
    let limited = describe(
        ApiError::RateLimited {
            retry_at: 1_793_462_400,
        },
        None,
    );
    assert!(limited.starts_with("GitHub-Limit erreicht – neuer Versuch um "));
    assert!(!limited.contains("1793462400"));
    let time = limited
        .trim_start_matches("GitHub-Limit erreicht – neuer Versuch um ")
        .trim_end_matches('.');
    assert!(
        chrono::NaiveTime::parse_from_str(time, "%H:%M").is_ok(),
        "{time}"
    );
    assert_eq!(
        ScopeError::from("Run-Liste ungültig.").describe("fixture-alex", Some("o/r")),
        "o/r: Run-Liste ungültig."
    );
    for error in [
        ApiError::Http(404),
        ApiError::InvalidResponse,
        ApiError::QueueUnavailable,
        ApiError::Policy("Nur https://api.github.com ist erlaubt."),
    ] {
        let text = describe(error, Some("o/r"));
        assert!(
            !text.contains("GitHub returned") && !text.contains("paused"),
            "{text}"
        );
    }
}
