use super::harness::*;
use crate::auto_repos::AUTO_REPOS;
use crate::live::catalog::{merge_catalog, AccountRepos};
use crate::live::schedule::METADATA_SECONDS;
use crate::live::*;

/// `github-1:<number>`, pushed on that day of October 2026 when a day is given.
fn repo(number: u64, day: Option<u64>, selected: bool) -> Repo {
    Repo {
        id: format!("github-1:{number}"),
        account_id: "github-1".into(),
        owner: "fixture-owner".into(),
        name: format!("fixture-{number}"),
        default_branch: "main".into(),
        pushed_at: day.map(|day| format!("2026-10-{day:02}T08:00:00Z")),
        selected,
        permission: Permission::Write,
    }
}
fn pin(repo_id: &str) -> Pin {
    Pin {
        id: format!("pin-{repo_id}"),
        label: "Fixture".into(),
        account_id: "github-1".into(),
        repo_id: repo_id.into(),
        workflow_id: 9,
        git_ref: "main".into(),
        inputs: Inputs::new(),
        quick_start: false,
    }
}
fn listing(repos: Vec<Repo>, truncated: bool) -> BTreeMap<String, AccountRepos> {
    BTreeMap::from([("github-1".to_string(), AccountRepos { repos, truncated })])
}
fn selected(repos: &[Repo]) -> Vec<String> {
    repos
        .iter()
        .filter(|r| r.selected)
        .map(|r| r.id.clone())
        .collect()
}
fn ids(numbers: impl IntoIterator<Item = u64>) -> Vec<String> {
    numbers
        .into_iter()
        .map(|number| format!("github-1:{number}"))
        .collect()
}

#[test]
fn a_listing_selects_the_newest_pushes_and_every_repository_of_a_favorite() {
    let accounts = [account("github-1")];
    // A selection made before (2) does not count; a missing push time (13) ranks last.
    let previous = vec![repo(2, Some(2), true)];
    let mut listed: Vec<_> = (1..=12).map(|n| repo(n, Some(n), false)).collect();
    listed.push(repo(13, None, false));
    let merged = merge_catalog(
        &previous,
        &accounts,
        listing(listed, false),
        &[pin("github-1:1")],
        &[],
        &[],
    );
    let mut expected = ids([1]);
    expected.extend(ids(3..=12));
    assert_eq!(selected(&merged), expected);
    assert_eq!(merged.len(), 13);
    assert_eq!(selected(&merged).len(), AUTO_REPOS + 1);
}

#[test]
fn a_cut_off_listing_ranks_a_kept_repository_by_its_last_known_push() {
    let accounts = [account("github-1")];
    let previous = vec![repo(99, Some(30), true), repo(98, Some(31), false)];
    let listed: Vec<_> = (1..=10).map(|n| repo(n, Some(n), false)).collect();
    let merged = merge_catalog(&previous, &accounts, listing(listed, true), &[], &[], &[]);
    // 99 stays, still among the newest; 98 was not selected, so the cut drops it.
    let mut expected = ids(2..=10);
    expected.extend(ids([99]));
    assert_eq!(selected(&merged), expected);
    assert!(!merged.iter().any(|r| r.id == "github-1:98"));
}

#[test]
fn a_repository_that_becomes_one_of_the_newest_loads_its_initial_history() {
    let mut remote = Remote::new();
    remote.repos.insert("github-1".into(), (1..=11).collect());
    for n in 2..=11 {
        remote
            .pushed
            .insert(n, format!("2026-10-{:02}T08:00:00Z", n + 10));
    }
    remote.pushed.insert(1, "2026-10-30T08:00:00Z".into());
    remote.runs.insert(1, vec![run(1, 10, "completed")]);
    let mut h = Harness::new("auto-select", remote, &["github-1"]);
    let mut first = ids([1]);
    first.extend(ids(3..=11));
    h.load(&first.iter().map(String::as_str).collect::<Vec<_>>());
    assert!(h.live.snapshot().runs.iter().any(|run| run.id == 10));
    assert_eq!(h.remote().count("github-1", "/fixture-2/"), 0);
    let initial = |n: u64| format!("/fixture-{n}/actions/runs?per_page=50");
    let window = |n: u64| format!("/fixture-{n}/actions/runs?per_page=100&created");

    // Its newest push grows old: the repository leaves the selection and is not read.
    h.remote().pushed.insert(1, "2026-09-01T08:00:00Z".into());
    advance(METADATA_SECONDS);
    h.remote().requests.clear();
    let snapshot = h.live.refresh();
    assert_eq!(selected_ids(&snapshot), ids(2..=11));
    assert!(!snapshot.runs.iter().any(|run| run.id == 10));
    assert_eq!(h.remote().count("github-1", "/fixture-1/"), 0);
    assert_eq!(h.remote().count("github-1", &initial(2)), 1);

    // A new push brings it back with its initial history, not just the runs since then.
    h.remote().pushed.insert(1, "2026-10-31T08:00:00Z".into());
    advance(METADATA_SECONDS);
    h.remote().requests.clear();
    let snapshot = h.live.refresh();
    assert_eq!(selected_ids(&snapshot), first);
    assert!(snapshot.runs.iter().any(|run| run.id == 10));
    let remote = h.remote();
    assert_eq!(remote.count("github-1", &initial(1)), 1);
    assert_eq!(remote.count("github-1", &window(1)), 0);
    assert_eq!(
        remote.count("github-1", &window(3)),
        1,
        "a kept one polls on"
    );
}
