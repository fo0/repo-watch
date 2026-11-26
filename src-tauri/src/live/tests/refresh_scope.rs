use crate::github::{ApiRequest, ApiResponse, TransportError};
use crate::live::catalog::{
    account_repos, bounded_refs, merge_catalog, offered_refs, AccountRepos, RefListing,
    MAX_PINNED_REFS, MAX_REFS, REPO_REF_BYTES,
};
use crate::live::*;
use crate::run_history::{Cause, ScopeError};

fn account(id: &str) -> Account {
    Account {
        id: id.into(),
        login: format!("{id}-login"),
        label: "GitHub · Token".into(),
    }
}
fn repo(account_id: &str, number: u64, selected: bool) -> Repo {
    Repo {
        id: format!("{account_id}:{number}"),
        account_id: account_id.into(),
        owner: "fixture-owner".into(),
        name: format!("fixture-{number}"),
        default_branch: "main".into(),
        pushed_at: None,
        selected,
        permission: Permission::Write,
    }
}

#[test]
fn failing_account_keeps_its_repositories_while_others_refresh() {
    let accounts = [account("github-1"), account("github-2")];
    let previous = vec![
        repo("github-1", 10, true),
        repo("github-1", 11, false),
        repo("github-2", 20, true),
        repo("github-2", 21, false),
    ];
    let listings = BTreeMap::from([(
        "github-1".to_string(),
        AccountRepos {
            repos: vec![repo("github-1", 10, false), repo("github-1", 12, false)],
            truncated: false,
        },
    )]);
    let merged = merge_catalog(&previous, &accounts, listings, &[], &[], &[]);
    let ids: Vec<_> = merged.iter().map(|r| (r.id.as_str(), r.selected)).collect();
    // github-1 refreshed: 11 left, 12 is new, and both listed ones are selected.
    // github-2 failed: its last known repositories and their selection stay as they were.
    assert_eq!(
        ids,
        vec![
            ("github-1:10", true),
            ("github-1:12", true),
            ("github-2:20", true),
            ("github-2:21", false)
        ]
    );
}

#[test]
fn truncated_listing_keeps_selected_repositories_beyond_the_cut() {
    let accounts = [account("github-1")];
    let previous = vec![
        repo("github-1", 10, false),
        repo("github-1", 99, true),
        repo("github-1", 98, false),
    ];
    let listing = |truncated| {
        BTreeMap::from([(
            "github-1".to_string(),
            AccountRepos {
                repos: vec![repo("github-1", 10, false), repo("github-1", 10, false)],
                truncated,
            },
        )])
    };
    let ids = |repos: Vec<Repo>| repos.into_iter().map(|r| r.id).collect::<Vec<_>>();
    assert_eq!(
        ids(merge_catalog(
            &previous,
            &accounts,
            listing(true),
            &[],
            &[],
            &[]
        )),
        vec!["github-1:10", "github-1:99"]
    );
    assert_eq!(
        ids(merge_catalog(
            &previous,
            &accounts,
            listing(false),
            &[],
            &[],
            &[]
        )),
        vec!["github-1:10"]
    );
}

#[test]
fn refs_are_bounded_and_always_offer_the_default_branch_and_existing_pins() {
    let found = RefListing {
        branches: (0..600).map(|i| format!("release/{i:04}")).collect(),
        ..RefListing::default()
    };
    let offered = offered_refs("trunk", &found, &[9], &[(9, "release/0599"), (9, "gone")]);
    let refs = &offered[&9];
    // The shared refs fill MAX_REFS; the pinned ref beyond them comes on top.
    assert_eq!(refs.len(), MAX_REFS + 1);
    assert!(refs.contains(&"trunk".to_string()));
    assert!(refs.contains(&"release/0599".to_string()));
    assert!(!refs.contains(&"gone".to_string()));
    let mut sorted = refs.clone();
    sorted.sort();
    assert_eq!(refs, &sorted);
    // Many startable workflows share one repository budget.
    let shared = bounded_refs("trunk", &found, 20);
    let bytes: usize = shared.iter().map(|name| name.len() + 3).sum();
    assert!(bytes * 20 <= REPO_REF_BYTES);
    assert!(shared.contains(&"trunk".to_string()));
    // An oversized default branch is still offered.
    let long = "x".repeat(REPO_REF_BYTES);
    assert_eq!(bounded_refs(&long, &found, 50), vec![long]);
}

#[test]
fn pinned_refs_stay_on_their_own_workflow_within_a_cap() {
    let branches: Vec<_> = (0..300).map(|i| format!("feature/{i:04}")).collect();
    let tags: Vec<_> = (0..5).map(|i| format!("v1.{i}")).collect();
    let cut = RefListing {
        branches: branches.clone(),
        tags: tags.clone(),
        truncated: true,
    };
    // Ten startable workflows leave each copy about 800 bytes, far fewer than MAX_REFS.
    let startable: Vec<u64> = (1..=10).collect();
    let pins = [
        (1, "release/beyond-the-cut"),
        (1, "feature/0299"),
        (2, "v1.4"),
        (99, "release/not-startable"),
    ];
    let offered = offered_refs("main", &cut, &startable, &pins);
    let has = |id: u64, name: &str| offered[&id].contains(&name.to_string());
    assert!(has(1, "release/beyond-the-cut") && has(1, "feature/0299"));
    assert!(
        !has(2, "release/beyond-the-cut") && !has(2, "feature/0299"),
        "a pin's ref stays on its own workflow"
    );
    assert!(!offered.contains_key(&99));
    assert!(offered
        .values()
        .all(|refs| !refs.contains(&"release/not-startable".to_string())));
    // Tags are not starved by a long branch list, and the shared refs keep the budget.
    for tag in &tags {
        assert!(has(3, tag), "{tag}");
    }
    let bytes: usize = offered[&3].iter().map(|name| name.len() + 3).sum();
    assert!(bytes <= REPO_REF_BYTES / 10);
    // A complete listing proves an unlisted pinned ref is gone.
    let complete = RefListing {
        truncated: false,
        ..cut
    };
    let offered = offered_refs("main", &complete, &startable, &pins);
    assert!(!offered[&1].contains(&"release/beyond-the-cut".to_string()));
    assert!(offered[&1].contains(&"feature/0299".to_string()));
    // Pinned extras are capped per repository, and invalid ref names are never offered.
    let many: Vec<String> = (0..MAX_PINNED_REFS + 5)
        .map(|i| format!("pin/{i}"))
        .collect();
    let mut pins: Vec<(u64, &str)> = vec![(1, "-option"), (1, "bad..ref"), (2, "a b")];
    pins.extend(many.iter().map(|name| (1, name.as_str())));
    let unknown = RefListing {
        truncated: true,
        ..RefListing::default()
    };
    let offered = offered_refs("main", &unknown, &[1, 2], &pins);
    assert_eq!(offered[&1].len(), MAX_PINNED_REFS + 1);
    assert!(offered[&1].iter().all(|name| valid_git_ref(name)));
    assert_eq!(offered[&2], vec!["main".to_string()]);
    // Both kinds share one budget in turns.
    let even = RefListing {
        branches: (0..200).map(|i| format!("b{i:03}")).collect(),
        tags: (0..200).map(|i| format!("t{i:03}")).collect(),
        truncated: false,
    };
    let refs = bounded_refs("main", &even, 1);
    let tagged = refs.iter().filter(|name| name.starts_with('t')).count();
    let branched = refs.iter().filter(|name| name.starts_with('b')).count();
    assert_eq!((branched, tagged), (50, 49));
}

/// An endlessly paginating repository listing, or a revoked token.
struct Listing {
    revoked: bool,
}
impl Transport for Listing {
    fn send(&mut self, request: &ApiRequest) -> Result<ApiResponse, TransportError> {
        let url = request.url().as_str();
        if self.revoked {
            return Ok(ApiResponse {
                status: 401,
                headers: BTreeMap::new(),
                body: b"{}".to_vec(),
            });
        }
        let page: u32 = url
            .split("&page=")
            .nth(1)
            .and_then(|page| page.parse().ok())
            .unwrap_or(1);
        if page == 1 {
            assert!(
                url.ends_with("/user/repos?per_page=100&sort=pushed"),
                "{url}"
            );
        }
        let next = format!(
            "<https://api.github.com/user/repos?per_page=100&page={}>; rel=\"next\"",
            page + 1
        );
        let body = serde_json::json!([{"id": page, "name": format!("fixture-{page}"), "owner": {"login": "fixture-owner"}, "permissions": {"push": page == 1}}]);
        Ok(ApiResponse {
            status: 200,
            headers: BTreeMap::from([("Link".to_string(), next)]),
            body: serde_json::to_vec(&body).unwrap(),
        })
    }
}

#[test]
fn endless_repository_listing_is_truncated_instead_of_failing_the_account() {
    let client = ApiClient::new(Listing { revoked: false });
    let listing = account_repos(&client, &account("github-1"), 0).unwrap();
    assert!(listing.truncated);
    assert_eq!(listing.repos.len(), 20);
    assert_eq!(listing.repos[0].permission, Permission::Write);
    assert_eq!(listing.repos[1].permission, Permission::Read);
    let revoked = ApiClient::new(Listing { revoked: true });
    let error = account_repos(&revoked, &account("github-2"), 0)
        .err()
        .unwrap();
    assert!(matches!(error, ScopeError::Unauthorized(_)));
    assert!(matches!(
        ScopeError::Denied(Cause::Http(403)).for_account(),
        ScopeError::Unauthorized(_)
    ));
}
