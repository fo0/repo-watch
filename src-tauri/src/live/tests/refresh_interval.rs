use super::harness::*;
use crate::live::schedule::{Schedule, METADATA_SECONDS};
use crate::live::*;
use crate::run_history::{Cause, ScopeError};

#[test]
fn the_interval_is_validated_stored_and_survives_a_restart() {
    let mut h = Harness::new("refresh-interval", Remote::new(), &["github-1"]);
    assert_eq!(h.live.refresh_interval(), 60, "the default");
    assert!(h.live.set_refresh_interval(14).is_err());
    assert!(h.live.set_refresh_interval(3601).is_err());
    assert_eq!(
        h.live.refresh_interval(),
        60,
        "a refused value changes nothing"
    );
    assert_eq!(h.live.set_refresh_interval(300), Ok(300));
    assert_eq!(h.live.refresh_interval(), 300);
    assert_eq!(h.restart().refresh_interval(), 300);
    // The other preference in the same row is kept.
    h.live.set_notifications_enabled(true).unwrap();
    assert_eq!(h.restart().refresh_interval(), 300);
    assert!(h.restart().notifications_enabled());
}

#[test]
fn the_catalog_follows_the_interval_and_metadata_keeps_its_own() {
    let mut h = Harness::new("refresh-catalog", Remote::new(), &["github-1"]);
    h.load(&["github-1:1"]);
    // With the default, the catalog is read again after one minute.
    h.remote().requests.clear();
    advance(60);
    h.live.refresh();
    assert_eq!(h.remote().count("github-1", "/user/repos"), 1);
    // A longer interval applies from the next read; metadata keeps its ten minutes.
    h.live.set_refresh_interval(1800).unwrap();
    h.remote().requests.clear();
    advance(60);
    h.live.refresh();
    assert_eq!(h.remote().count("github-1", "/user/repos"), 1);
    h.remote().requests.clear();
    advance(METADATA_SECONDS);
    h.live.refresh();
    assert_eq!(h.remote().count("github-1", "/user/repos"), 0);
    assert_eq!(h.remote().count("github-1", "/branches"), 1);
    h.remote().requests.clear();
    advance(1800 - METADATA_SECONDS);
    h.live.refresh();
    assert_eq!(h.remote().count("github-1", "/user/repos"), 1);
    // A shorter interval applies at once, not after the read the longer one scheduled.
    h.live.set_refresh_interval(60).unwrap();
    h.remote().requests.clear();
    advance(60);
    h.live.refresh();
    assert_eq!(h.remote().count("github-1", "/user/repos"), 1);
}

#[test]
fn an_explicit_refresh_polls_an_idle_repository_inside_a_long_interval() {
    let mut h = Harness::new("refresh-now", Remote::new(), &["github-1"]);
    h.load(&["github-1:1"]);
    h.live.set_refresh_interval(1800).unwrap();
    advance(60);
    h.live.refresh();
    h.remote().requests.clear();
    advance(60);
    h.live.refresh();
    assert_eq!(
        h.remote().count("github-1", "/actions/runs"),
        0,
        "a worker pass waits out the interval"
    );
    let snapshot = h.live.refresh_now();
    assert!(
        h.remote().count("github-1", "/actions/runs") > 0,
        "F5 polls now"
    );
    assert!(!snapshot.stale);
    // Catalog and metadata keep their own timers.
    assert_eq!(h.remote().count("github-1", "/user/repos"), 0);
    assert_eq!(h.remote().count("github-1", "/branches"), 0);
}

#[test]
fn the_open_run_flag_follows_the_runs_the_panel_shows() {
    let mut remote = Remote::new();
    remote.runs.insert(1, vec![run(1, 30, "in_progress")]);
    let mut h = Harness::new("refresh-open-run", remote, &["github-1"]);
    assert!(!h.live.has_open_run());
    h.load(&["github-1:1"]);
    assert!(h.live.has_open_run());
    h.remote().runs.insert(1, vec![run(1, 30, "completed")]);
    advance(15);
    h.live.refresh();
    assert!(!h.live.has_open_run());
}

#[test]
fn a_started_run_whose_first_read_failed_still_counts_as_open() {
    let mut remote = Remote::new();
    remote.dispatches.push_back(confirmation(1, 77));
    let mut h = Harness::new("refresh-open-tracker", remote, &["github-1"]);
    h.load(&["github-1:1"]);
    h.live.set_refresh_interval(1800).unwrap();
    assert!(!h.live.has_open_run());
    // GitHub accepted the start, but every run read of the repository fails for now.
    h.remote().runs_status.insert(1, 502);
    let outcome = h.live.dispatch(request("start-1", "main"));
    assert_eq!(
        outcome.state,
        DispatchState::Accepted,
        "{}",
        outcome.message
    );
    assert_eq!(h.remote().count("github-1", "/actions/runs/77"), 1);
    assert!(
        h.live.snapshot().runs.is_empty(),
        "the started run is not listed yet"
    );
    assert!(
        h.live.has_open_run(),
        "the tracker keeps the 15 s pace instead of the 30 min interval"
    );
    // The next read finds the run completed: tracking ends and the idle pace returns.
    h.remote().runs_status.clear();
    h.remote().runs.insert(1, vec![run(1, 77, "completed")]);
    advance(15);
    h.live.refresh_now();
    assert!(!h.live.has_open_run());
}

#[test]
fn a_short_interval_keeps_the_catalog_floor_and_a_long_one_widens_the_clock_guard() {
    let mut schedule = Schedule::new(15);
    schedule.catalog_read("a", None, 100);
    assert!(
        !schedule.catalog_due("a", 159),
        "never faster than a minute"
    );
    assert!(schedule.catalog_due("a", 160));
    // An hour-long interval is not mistaken for a backward clock jump.
    schedule.set_refresh(3600, 100);
    schedule.catalog_read("a", None, 100);
    assert!(!schedule.catalog_due("a", 100 + METADATA_SECONDS));
    assert!(!schedule.catalog_due("a", 3699));
    assert!(schedule.catalog_due("a", 3700));
    // Still, a deadline further ahead than any interval was set before the clock went back.
    schedule.catalog_read("b", None, 10_000);
    assert!(schedule.catalog_due("b", 5_000));
    // Shortening the interval brings a pending read forward to within the new interval.
    schedule.set_refresh(60, 200);
    assert!(!schedule.catalog_due("a", 259));
    assert!(schedule.catalog_due("a", 260));
}

#[test]
fn a_backward_clock_jump_stalls_metadata_and_holds_no_longer_than_their_own_delay() {
    let mut schedule = Schedule::new(3600);
    schedule.metadata_read("r", None, 10_000);
    assert!(!schedule.metadata_due("r", 10_000));
    // The clock went back 1000 s: further ahead than metadata ever waits, so it is due,
    // although the hour-long catalog interval is longer.
    assert!(schedule.metadata_due("r", 9_000));
    let revoked = ScopeError::Unauthorized(Cause::Http(401));
    schedule.hold("a", revoked.clone(), 10_000);
    assert_eq!(schedule.held("a", 10_000), Some(&revoked));
    assert_eq!(schedule.held("a", 9_000), None, "the hold ended");
    // A shorter interval never cuts a hold short.
    schedule.hold("h", revoked, 10_000);
    schedule.set_refresh(60, 10_000);
    assert!(!schedule.catalog_due("h", 10_100));
    assert!(schedule.catalog_due("h", 10_000 + METADATA_SECONDS));
}

#[test]
fn the_log_level_is_stored_and_survives_a_restart_beside_the_other_settings() {
    use crate::app_log::Level;
    let mut h = Harness::new("log-level", Remote::new(), &["github-1"]);
    assert_eq!(h.live.log_level(), Level::Info, "the default");
    h.live.set_refresh_interval(300).unwrap();
    assert_eq!(h.live.set_log_level(Level::Debug), Ok(Level::Debug));
    assert_eq!(h.live.log_level(), Level::Debug);
    let restarted = h.restart();
    assert_eq!(restarted.log_level(), Level::Debug);
    assert_eq!(restarted.refresh_interval(), 300);
    assert_eq!(h.live.set_log_level(Level::Info), Ok(Level::Info));
    assert_eq!(h.restart().log_level(), Level::Info);
}
