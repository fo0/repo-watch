use super::harness::*;
use crate::live::owners::{
    owner_account_unknown, owner_invalid, owners_too_many, MAX_OWNERS, MAX_OWNER_CHARS,
};
use crate::live::schedule::METADATA_SECONDS;
use crate::live::*;

/// `github-1:<number>` for each number.
fn ids(numbers: impl IntoIterator<Item = u64>) -> Vec<String> {
    numbers
        .into_iter()
        .map(|number| format!("github-1:{number}"))
        .collect()
}
fn filter(owners: &[&str]) -> OwnerFilter {
    OwnerFilter {
        account_id: "github-1".into(),
        owners: owners.iter().map(|owner| owner.to_string()).collect(),
    }
}
fn owners(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| name.to_string()).collect()
}
/// Eleven repositories of github-1, the lower the number the newer its push; the even ones
/// belong to `fixture-org`, the odd ones to `fixture-owner`. 1 and 2 have a run.
fn eleven() -> Harness {
    let mut remote = Remote::new();
    remote.repos.insert("github-1".into(), (1..=11).collect());
    for n in 1..=11 {
        remote
            .pushed
            .insert(n, format!("2026-10-{:02}T08:00:00Z", 30 - n));
        if n % 2 == 0 {
            remote.owners.insert(n, "fixture-org");
        }
    }
    remote.runs.insert(1, vec![run(1, 10, "completed")]);
    remote.runs.insert(2, vec![run(2, 20, "completed")]);
    let mut h = Harness::new("owners", remote, &["github-1"]);
    let first = ids(1..=10);
    h.load(&first.iter().map(String::as_str).collect::<Vec<_>>());
    h
}
fn selected(snapshot: &Snapshot) -> Vec<String> {
    selected_ids(snapshot)
        .into_iter()
        .map(String::from)
        .collect()
}
const ODD: [u64; 6] = [1, 3, 5, 7, 9, 11];

#[test]
fn a_filter_applies_at_once_and_survives_a_restart() {
    let mut h = eleven();
    h.remote().requests.clear();
    let snapshot = h
        .live
        .set_owner_filter("github-1".into(), owners(&["fixture-owner"]))
        .unwrap();
    assert!(h.remote().requests.is_empty(), "no network round trip");
    assert_eq!(snapshot.owner_filters, vec![filter(&["fixture-owner"])]);
    assert_eq!(selected(&snapshot), ids(ODD), "11 moves up");
    assert!(!snapshot.runs.iter().any(|run| run.repo_id == "github-1:2"));
    assert!(snapshot.runs.iter().any(|run| run.repo_id == "github-1:1"));
    // The same filter again changes nothing.
    let again = h
        .live
        .set_owner_filter("github-1".into(), owners(&["fixture-owner"]))
        .unwrap();
    assert_eq!(again.owner_filters, vec![filter(&["fixture-owner"])]);

    let restarted = h.restart().snapshot();
    assert_eq!(restarted.owner_filters, vec![filter(&["fixture-owner"])]);
    assert_eq!(selected(&restarted), ids(ODD));

    // The next refresh loads 11's initial history and leaves 2 alone, also after a
    // catalog listing.
    advance(METADATA_SECONDS);
    h.remote().requests.clear();
    let snapshot = h.live.refresh();
    assert_eq!(selected(&snapshot), ids(ODD));
    assert_eq!(
        h.remote()
            .count("github-1", "/fixture-11/actions/runs?per_page=50"),
        1
    );
    assert_eq!(h.remote().count("github-1", "/fixture-2/"), 0);
}

#[test]
fn owners_are_trimmed_kept_once_and_replace_the_earlier_filter() {
    let mut h = eleven();
    let snapshot = h
        .live
        .set_owner_filter(
            "github-1".into(),
            owners(&[" fixture-owner ", "FIXTURE-OWNER", "fixture-org"]),
        )
        .unwrap();
    assert_eq!(
        snapshot.owner_filters,
        vec![filter(&["fixture-owner", "fixture-org"])]
    );
    assert_eq!(selected(&snapshot), ids(1..=10));
    // Another filter of the same account replaces it; logins compare case-insensitively.
    let snapshot = h
        .live
        .set_owner_filter("github-1".into(), owners(&["Fixture-Org"]))
        .unwrap();
    assert_eq!(snapshot.owner_filters, vec![filter(&["Fixture-Org"])]);
    assert_eq!(selected(&snapshot), ids([2, 4, 6, 8, 10]));
    // An owner missing from the catalog is kept: the listing may be older than the choice.
    let snapshot = h
        .live
        .set_owner_filter("github-1".into(), owners(&["fixture-elsewhere"]))
        .unwrap();
    assert_eq!(snapshot.owner_filters, vec![filter(&["fixture-elsewhere"])]);
    assert!(selected(&snapshot).is_empty());
}

#[test]
fn no_owners_removes_the_filter_and_watches_all_again() {
    let mut h = eleven();
    h.live
        .set_owner_filter("github-1".into(), owners(&["fixture-owner"]))
        .unwrap();
    let snapshot = h.live.set_owner_filter("github-1".into(), vec![]).unwrap();
    assert!(snapshot.owner_filters.is_empty());
    assert_eq!(selected(&snapshot), ids(1..=10));
    assert!(h.restart().snapshot().owner_filters.is_empty());
    // Removing a filter that is gone succeeds and changes nothing.
    let unchanged = h.live.set_owner_filter("github-1".into(), vec![]).unwrap();
    assert_eq!(selected(&unchanged), ids(1..=10));
}

#[test]
fn only_a_connected_account_and_valid_owners_are_accepted() {
    let mut h = eleven();
    for account in ["github-2", "", "github-1\n"] {
        assert_eq!(
            h.live
                .set_owner_filter(account.into(), owners(&["fixture-owner"])),
            Err(owner_account_unknown().into())
        );
    }
    let too_long = "x".repeat(MAX_OWNER_CHARS + 1);
    for invalid in [
        vec![""],
        vec!["fixture-owner", "  "],
        vec!["fixture\nowner"],
        vec![too_long.as_str()],
    ] {
        assert_eq!(
            h.live.set_owner_filter("github-1".into(), owners(&invalid)),
            Err(owner_invalid().into())
        );
    }
    let many: Vec<_> = (0..=MAX_OWNERS)
        .map(|index| format!("fixture-org-{index}"))
        .collect();
    assert_eq!(
        h.live.set_owner_filter("github-1".into(), many.clone()),
        Err(owners_too_many().into())
    );
    assert!(h.live.snapshot().owner_filters.is_empty());
    assert_eq!(selected(&h.live.snapshot()), ids(1..=10));
    // The bounds themselves pass; a repeated owner does not count twice.
    let mut full = many[..MAX_OWNERS].to_vec();
    full[0] = "x".repeat(MAX_OWNER_CHARS);
    full.push(full[1].to_uppercase());
    let snapshot = h.live.set_owner_filter("github-1".into(), full).unwrap();
    assert_eq!(snapshot.owner_filters[0].owners.len(), MAX_OWNERS);
}

#[test]
fn a_favorite_keeps_its_repository_outside_the_filter_polled() {
    let mut h = eleven();
    h.live.data.pins.push(Pin {
        id: "pin-2".into(),
        label: "Fixture".into(),
        account_id: "github-1".into(),
        repo_id: "github-1:2".into(),
        workflow_id: 9,
        git_ref: "main".into(),
        inputs: Inputs::new(),
        quick_start: false,
    });
    let snapshot = h
        .live
        .set_owner_filter("github-1".into(), owners(&["fixture-owner"]))
        .unwrap();
    assert_eq!(selected(&snapshot), ids([1, 2, 3, 5, 7, 9, 11]));
    assert!(snapshot.runs.iter().any(|run| run.id == 20));
}

#[test]
fn a_repository_opened_from_the_search_stays_selected_while_its_owner_is_watched() {
    let mut h = eleven();
    // 11 was opened from the search; a filter that keeps its owner keeps it open.
    h.live.open_repository("github-1:11".into()).unwrap();
    let snapshot = h
        .live
        .set_owner_filter("github-1".into(), owners(&["fixture-owner"]))
        .unwrap();
    assert!(selected(&snapshot).contains(&"github-1:11".to_string()));
    assert!(h.live.browsed.contains("github-1:11"));
    // A filter that leaves its owner out drops the hold, as hiding does.
    let snapshot = h
        .live
        .set_owner_filter("github-1".into(), owners(&["fixture-org"]))
        .unwrap();
    assert_eq!(selected(&snapshot), ids([2, 4, 6, 8, 10]));
    assert!(!h.live.browsed.contains("github-1:11"));
}

#[test]
fn only_filters_of_connected_accounts_show() {
    let mut h = eleven();
    h.live.owner_filters = vec![OwnerFilter {
        account_id: "github-9".into(),
        owners: owners(&["fixture-owner"]),
    }];
    assert!(h.live.snapshot().owner_filters.is_empty());
    let snapshot = h
        .live
        .set_owner_filter("github-1".into(), owners(&["fixture-owner"]))
        .unwrap();
    assert_eq!(snapshot.owner_filters, vec![filter(&["fixture-owner"])]);
    assert_eq!(h.live.owner_filters.len(), 2, "the stray entry is not lost");
}

#[test]
fn disconnecting_an_account_drops_its_filter() {
    let mut h = eleven();
    h.live
        .set_owner_filter("github-1".into(), owners(&["fixture-owner"]))
        .unwrap();
    // Another preference in the same row stays.
    h.live.set_refresh_interval(300).unwrap();
    assert!(h
        .live
        .disconnect("github-1")
        .unwrap()
        .owner_filters
        .is_empty());
    assert!(h
        .live
        .cache
        .load_app_settings()
        .unwrap()
        .owner_filters
        .is_empty());
    let restarted = h.restart();
    assert!(restarted.snapshot().owner_filters.is_empty());
    assert_eq!(restarted.refresh_interval(), 300);
}
