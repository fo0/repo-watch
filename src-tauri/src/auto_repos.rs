//! The automatic repository selection: every account shows its most recently pushed
//! repositories, plus each repository a favorite starts in, so favorites keep working.
//! A repository the user hid gives up its place among the newest (ADR-0008), and so does one
//! whose owner the account's owner filter leaves out.
use crate::models::{HiddenRepo, OwnerFilter, Pin, Repo};
use chrono::{DateTime, FixedOffset};
use std::cmp::Reverse;
use std::collections::BTreeMap;

/// Repositories each account shows by its newest pushes.
pub const AUTO_REPOS: usize = 10;

/// A push time as GitHub writes it (RFC 3339); anything else counts as missing.
pub fn push_time(value: Option<&str>) -> Option<DateTime<FixedOffset>> {
    value.and_then(|text| DateTime::parse_from_rfc3339(text).ok())
}

/// Whether the user hid this repository of its account.
pub fn is_hidden(hidden: &[HiddenRepo], repo: &Repo) -> bool {
    hidden
        .iter()
        .any(|entry| entry.matches(&repo.account_id, &repo.id))
}

/// Whether the account's owner filter leaves this repository's owner out; an account without
/// a filter, or with one that names no owner, watches all owners.
pub fn is_filtered_out(filters: &[OwnerFilter], repo: &Repo) -> bool {
    filters
        .iter()
        .find(|filter| filter.account_id == repo.account_id)
        .is_some_and(|filter| !filter.owners.is_empty() && !filter.watches(&repo.owner))
}

/// Selects, per account, the `AUTO_REPOS` repositories with the newest push (a missing or
/// unreadable push time last, ties by `owner/name`) and every repository of a favorite of
/// that account. A hidden repository, and one whose owner the account's filter leaves out,
/// takes none of the newest places, so the next one moves up; a favorite still selects it.
/// Every other repository is deselected.
pub fn auto_select(
    repos: &mut [Repo],
    pins: &[Pin],
    hidden: &[HiddenRepo],
    owners: &[OwnerFilter],
) {
    let mut order: Vec<usize> = (0..repos.len()).collect();
    order.sort_by_cached_key(|&index| {
        let repo = &repos[index];
        (
            Reverse(push_time(repo.pushed_at.as_deref())),
            repo.owner.to_lowercase(),
            repo.name.to_lowercase(),
            repo.id.clone(),
        )
    });
    let mut shown: BTreeMap<String, usize> = BTreeMap::new();
    for index in order {
        let repo = &mut repos[index];
        let pinned = pins
            .iter()
            .any(|pin| pin.repo_id == repo.id && pin.account_id == repo.account_id);
        if is_hidden(hidden, repo) || is_filtered_out(owners, repo) {
            repo.selected = pinned;
            continue;
        }
        let count = shown.entry(repo.account_id.clone()).or_default();
        let newest = *count < AUTO_REPOS;
        if newest {
            *count += 1;
        }
        repo.selected = newest || pinned;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Inputs, Permission};

    fn repo(account: &str, name: &str, pushed_at: Option<&str>) -> Repo {
        Repo {
            id: format!("{account}:{name}"),
            account_id: account.into(),
            owner: "fixture-owner".into(),
            name: name.into(),
            default_branch: "main".into(),
            pushed_at: pushed_at.map(str::to_owned),
            selected: false,
            permission: Permission::Write,
        }
    }
    fn pin(account: &str, repo_id: &str) -> Pin {
        Pin {
            id: format!("pin-{repo_id}"),
            label: "Fixture".into(),
            account_id: account.into(),
            repo_id: repo_id.into(),
            workflow_id: 9,
            git_ref: "main".into(),
            inputs: Inputs::new(),
            quick_start: false,
        }
    }
    fn selected(repos: &[Repo]) -> Vec<&str> {
        let mut ids: Vec<_> = repos
            .iter()
            .filter(|r| r.selected)
            .map(|r| r.id.as_str())
            .collect();
        ids.sort();
        ids
    }

    #[test]
    fn the_newest_pushes_win_and_a_missing_push_time_ranks_last() {
        let mut repos: Vec<_> = (0..12)
            .map(|day| {
                let pushed = format!("2026-10-{:02}T08:00:00Z", day + 1);
                repo("a", &format!("r{day:02}"), Some(&pushed))
            })
            .collect();
        repos.push(repo("a", "never", None));
        repos.push(repo("a", "garbled", Some("yesterday")));
        // An offset is read as the moment it names: 2026-10-13T01:00Z, the newest push.
        repos.push(repo("a", "offset", Some("2026-10-13T03:00:00+02:00")));
        // An earlier selection does not count.
        repos[0].selected = true;
        auto_select(&mut repos, &[], &[], &[]);
        let mut expected: Vec<_> = (3..12).map(|day| format!("a:r{day:02}")).collect();
        expected.push("a:offset".into());
        expected.sort();
        assert_eq!(selected(&repos), expected);
        assert_eq!(selected(&repos).len(), AUTO_REPOS);
    }

    #[test]
    fn ties_go_by_owner_and_name_and_each_account_ranks_alone() {
        let same = Some("2026-10-01T08:00:00Z");
        let mut repos: Vec<_> = (0..AUTO_REPOS + 2)
            .rev()
            .map(|index| repo("a", &format!("Name-{index:02}"), same))
            .collect();
        repos.push(repo("b", "lonely", None));
        auto_select(&mut repos, &[], &[], &[]);
        let picked = selected(&repos);
        assert!(picked.contains(&"a:Name-00") && picked.contains(&"a:Name-09"));
        assert!(!picked.contains(&"a:Name-10") && !picked.contains(&"a:Name-11"));
        assert!(
            picked.contains(&"b:lonely"),
            "another account keeps its own slots"
        );
    }

    #[test]
    fn a_favorite_keeps_its_repository_beyond_the_newest() {
        let mut repos: Vec<_> = (0..AUTO_REPOS)
            .map(|index| repo("a", &format!("new-{index}"), Some("2026-10-02T08:00:00Z")))
            .collect();
        repos.push(repo("a", "old", Some("2020-01-01T00:00:00Z")));
        repos.push(repo("a", "other", None));
        // A favorite of another account never selects this account's repository.
        auto_select(
            &mut repos,
            &[pin("a", "a:old"), pin("b", "a:other")],
            &[],
            &[],
        );
        assert_eq!(selected(&repos).len(), AUTO_REPOS + 1);
        assert!(selected(&repos).contains(&"a:old"));
        assert!(!selected(&repos).contains(&"a:other"));
    }

    fn hide(account: &str, name: &str) -> HiddenRepo {
        HiddenRepo {
            account_id: account.into(),
            repo_id: format!("{account}:{name}"),
            owner: "fixture-owner".into(),
            name: name.into(),
        }
    }

    #[test]
    fn a_hidden_repository_gives_its_place_to_the_next_newest() {
        let mut repos: Vec<_> = (0..AUTO_REPOS + 2)
            .map(|day| {
                let pushed = format!("2026-10-{:02}T08:00:00Z", day + 1);
                repo("a", &format!("r{day:02}"), Some(&pushed))
            })
            .collect();
        repos.push(repo("b", "r11", Some("2026-10-12T08:00:00Z")));
        // The newest of account a is hidden; the same name of account b is not.
        auto_select(&mut repos, &[], &[hide("a", "r11")], &[]);
        let picked = selected(&repos);
        assert!(!picked.contains(&"a:r11"));
        assert!(
            picked.contains(&"a:r01"),
            "the 11th newest moves up into the freed place"
        );
        assert!(!picked.contains(&"a:r00"));
        assert!(picked.contains(&"b:r11"), "hiding is per account");
        assert_eq!(picked.len(), AUTO_REPOS + 1);
    }

    #[test]
    fn a_favorite_keeps_a_hidden_repository_polled_without_taking_a_place() {
        let mut repos: Vec<_> = (0..AUTO_REPOS + 1)
            .map(|day| {
                let pushed = format!("2026-10-{:02}T08:00:00Z", day + 1);
                repo("a", &format!("r{day:02}"), Some(&pushed))
            })
            .collect();
        auto_select(
            &mut repos,
            &[pin("a", "a:r10")],
            &[hide("a", "r10"), hide("a", "r09")],
            &[],
        );
        let picked = selected(&repos);
        assert!(picked.contains(&"a:r10"), "its favorite keeps working");
        assert!(!picked.contains(&"a:r09"));
        assert!(
            picked.contains(&"a:r00"),
            "both hidden places went to older ones"
        );
        assert_eq!(picked.len(), AUTO_REPOS);
    }

    /// Account a's repositories r00..r(count-1), the higher the number the newer its push;
    /// the odd ones belong to `fixture-org`, the even ones to `fixture-owner`.
    fn two_owners(count: usize) -> Vec<Repo> {
        (0..count)
            .map(|day| {
                let pushed = format!("2026-10-{:02}T08:00:00Z", day + 1);
                let mut repo = repo("a", &format!("r{day:02}"), Some(&pushed));
                if day % 2 == 1 {
                    repo.owner = "fixture-org".into();
                }
                repo
            })
            .collect()
    }
    fn watch(account: &str, owners: &[&str]) -> OwnerFilter {
        OwnerFilter {
            account_id: account.into(),
            owners: owners.iter().map(|owner| owner.to_string()).collect(),
        }
    }

    #[test]
    fn an_owner_outside_the_filter_gives_its_places_to_the_next_newest() {
        let mut repos = two_owners(2 * AUTO_REPOS + 2);
        repos.push(repo("b", "r21", Some("2026-10-22T08:00:00Z")));
        // GitHub logins compare case-insensitively; account b has no filter.
        auto_select(&mut repos, &[], &[], &[watch("a", &["Fixture-Owner"])]);
        let picked = selected(&repos);
        let mut expected: Vec<_> = (2..=2 * AUTO_REPOS)
            .step_by(2)
            .map(|day| format!("a:r{day:02}"))
            .collect();
        expected.push("b:r21".into());
        assert_eq!(picked, expected, "only fixture-owner fills a's places");
        assert!(
            !picked.contains(&"a:r00"),
            "the 11th of its owner stays out"
        );
        assert!(picked.contains(&"b:r21"), "the filter is per account");
        // A filter that names no owner filters nothing.
        assert!(!is_filtered_out(&[watch("a", &[])], &repos[1]));
    }

    #[test]
    fn a_favorite_keeps_a_repository_outside_the_filter_without_taking_a_place() {
        let mut repos = two_owners(2 * AUTO_REPOS + 2);
        auto_select(
            &mut repos,
            &[pin("a", "a:r21")],
            &[],
            &[watch("a", &["fixture-owner"])],
        );
        let picked = selected(&repos);
        assert!(picked.contains(&"a:r21"), "its favorite keeps working");
        assert!(!picked.contains(&"a:r19"));
        assert!(
            picked.contains(&"a:r02"),
            "all ten places went to fixture-owner"
        );
        assert_eq!(picked.len(), AUTO_REPOS + 1);
    }

    #[test]
    fn a_hidden_repository_of_a_watched_owner_still_gives_up_its_place() {
        let mut repos = two_owners(2 * AUTO_REPOS + 2);
        auto_select(
            &mut repos,
            &[],
            &[hide("a", "r20")],
            &[watch("a", &["fixture-owner"])],
        );
        let picked = selected(&repos);
        assert!(!picked.contains(&"a:r20"));
        assert!(
            picked.contains(&"a:r00"),
            "the next of the watched owner moves up"
        );
        assert!(picked.iter().all(|id| {
            repos
                .iter()
                .any(|repo| repo.id == *id && repo.owner == "fixture-owner")
        }));
        assert_eq!(picked.len(), AUTO_REPOS);
    }

    #[test]
    fn a_cached_repository_from_before_push_times_still_loads() {
        let cached = serde_json::json!({
            "id": "a:old", "accountId": "a", "owner": "fixture-owner", "name": "old",
            "defaultBranch": "main", "selected": true, "permission": "write"
        });
        let repo: Repo = serde_json::from_value(cached).unwrap();
        assert_eq!(repo.pushed_at, None);
        // The panel contract carries the field as `pushedAt: null`.
        assert!(serde_json::to_value(&repo).unwrap()["pushedAt"].is_null());
    }
}
