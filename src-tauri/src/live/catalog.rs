//! Repository catalogs and the bounded ref listing offered per startable workflow.
use super::{number, text, url};
use crate::{
    auto_repos::{auto_select, push_time},
    browser_auth,
    github::{ApiClient, Transport},
    i18n::tr,
    models::*,
    run_history::{Cause, ScopeError},
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// Pages (100 names each) read per ref kind for the ref picker.
pub(super) const REF_PAGES: usize = 3;
/// Refs offered per startable workflow, the default branch included; pinned refs come on top.
pub(super) const MAX_REFS: usize = 100;
/// Pinned refs a repository offers beyond its shared refs, all its workflows together; at
/// most 255 bytes each (`valid_git_ref`), so the selected repositories (`AUTO_REPOS` per
/// account plus those of favorites) stay far below the cache cap.
pub(super) const MAX_PINNED_REFS: usize = 20;
/// Every workflow carries its own copy of the repository's refs; all copies of one
/// repository share this budget, so the selected repositories stay far below the cache cap.
pub(super) const REPO_REF_BYTES: usize = 8 * 1024;

/// A listing within `max_pages` (`usize::MAX`: the transport's whole page budget).
/// The flag reports a cut-off listing; GitHub's first pages are returned either way.
pub(super) fn list_values<T: Transport>(
    client: &ApiClient<T>,
    account: &str,
    path: &[&str],
    key: Option<&str>,
    max_pages: usize,
    now: u64,
) -> Result<(Vec<Value>, bool), ScopeError> {
    list_sorted(client, account, path, &[], key, max_pages, now)
}

/// `list_values` with extra query pairs, such as an order that puts the entries a cut-off
/// listing must keep on its first pages.
fn list_sorted<T: Transport>(
    client: &ApiClient<T>,
    account: &str,
    path: &[&str],
    order: &[(&str, &str)],
    key: Option<&str>,
    max_pages: usize,
    now: u64,
) -> Result<(Vec<Value>, bool), ScopeError> {
    let query: Vec<_> = [("per_page", "100")].iter().chain(order).copied().collect();
    let pages = client.get_pages_limited(account, url(path, &query)?, now, max_pages)?;
    let mut all = vec![];
    for body in &pages.bodies {
        let page: Value = serde_json::from_slice(body)
            .map_err(|_| tr("GitHub-Antwort ungültig.", "Invalid GitHub response."))?;
        all.extend(
            key.map_or(&page, |k| &page[k])
                .as_array()
                .ok_or(tr("GitHub-Liste ungültig.", "Invalid GitHub list."))?
                .iter()
                .cloned(),
        );
    }
    Ok((all, pages.truncated))
}

/// A listing that must be complete (workflows, environments): one longer than the page
/// budget fails the scope, which keeps its last known data and names the cut-off listing,
/// instead of silently dropping the entries past the budget.
pub(super) fn list_all<T: Transport>(
    client: &ApiClient<T>,
    account: &str,
    path: &[&str],
    key: Option<&str>,
    now: u64,
) -> Result<Vec<Value>, ScopeError> {
    match list_values(client, account, path, key, usize::MAX, now)? {
        (_, true) => Err(ScopeError::Transient(Cause::Truncated)),
        (values, false) => Ok(values),
    }
}

/// One account's repositories as far as GitHub lists them.
pub(super) struct AccountRepos {
    pub(super) repos: Vec<Repo>,
    pub(super) truncated: bool,
}

pub(super) fn account_repos<T: Transport>(
    client: &ApiClient<T>,
    account: &Account,
    now: u64,
) -> Result<AccountRepos, ScopeError> {
    let mut values = vec![];
    let mut truncated = false;
    if browser_auth::is_browser_account(&account.id) {
        let (installations, cut) = list_values(
            client,
            &account.id,
            &["user", "installations"],
            Some("installations"),
            usize::MAX,
            now,
        )?;
        truncated |= cut;
        for installation in installations {
            if !installation["suspended_at"].is_null() {
                continue;
            }
            let installation_id = number(&installation, "id")?.to_string();
            let (repositories, cut) = list_values(
                client,
                &account.id,
                &["user", "installations", &installation_id, "repositories"],
                Some("repositories"),
                usize::MAX,
                now,
            )?;
            truncated |= cut;
            for value in repositories {
                let permission = browser_repository_permission(&installation, &value);
                values.push((value, permission));
            }
        }
    } else {
        // Newest pushes first (GitHub sorts `pushed` descending), so a listing cut at the
        // page budget still holds the repositories `auto_select` ranks highest.
        let (repositories, cut) = list_sorted(
            client,
            &account.id,
            &["user", "repos"],
            &[("sort", "pushed")],
            None,
            usize::MAX,
            now,
        )?;
        truncated |= cut;
        for value in repositories {
            let permission = if value["permissions"]["push"].as_bool() == Some(true) {
                Permission::Write
            } else {
                Permission::Read
            };
            values.push((value, permission));
        }
    }
    let mut repos = vec![];
    for (value, permission) in values {
        repos.push(Repo {
            id: format!("{}:{}", account.id, number(&value, "id")?),
            account_id: account.id.clone(),
            owner: text(&value["owner"], "login")?.into(),
            name: text(&value, "name")?.into(),
            default_branch: value["default_branch"].as_str().unwrap_or("main").into(),
            // Only a readable time is kept; it ranks the automatic selection.
            pushed_at: value["pushed_at"]
                .as_str()
                .filter(|pushed| push_time(Some(pushed)).is_some())
                .map(str::to_owned),
            selected: false,
            permission,
        });
    }
    Ok(AccountRepos { repos, truncated })
}

/// Listed accounts take their fresh repositories and select them automatically
/// (`auto_select`: the newest pushes without hidden ones or owners the account's filter leaves
/// out, plus the repositories of
/// favorites); unlisted accounts keep their last known ones and their selection. A cut-off
/// listing cannot prove that a selected repository went away, so a selected repository
/// missing from it stays and is ranked with the listed ones by its last known push.
pub(super) fn merge_catalog(
    previous: &[Repo],
    accounts: &[Account],
    mut listings: BTreeMap<String, AccountRepos>,
    pins: &[Pin],
    hidden: &[HiddenRepo],
    owners: &[OwnerFilter],
) -> Vec<Repo> {
    let mut repos = vec![];
    for account in accounts {
        let known = previous.iter().filter(|r| r.account_id == account.id);
        let Some(listing) = listings.remove(&account.id) else {
            repos.extend(known.cloned());
            continue;
        };
        let mut fresh = listing.repos;
        if listing.truncated {
            fresh.extend(known.filter(|r| r.selected).cloned());
        }
        let mut seen = BTreeSet::new();
        fresh.retain(|repo| seen.insert(repo.id.clone()));
        auto_select(&mut fresh, pins, hidden, owners);
        repos.extend(fresh);
    }
    let mut seen = BTreeSet::new();
    repos.retain(|repo| seen.insert(repo.id.clone()));
    repos
}

/// A repository's branches and tags as far as the ref picker reads them.
#[derive(Debug, Default)]
pub(super) struct RefListing {
    pub(super) branches: Vec<String>,
    pub(super) tags: Vec<String>,
    /// Either kind was cut at `REF_PAGES`.
    pub(super) truncated: bool,
}

/// The refs each startable workflow (by ID) offers, sorted. Dispatch validation in UI and
/// core checks against exactly these lists.
///
/// - Every workflow: the repository's `bounded_refs`, shared by all startable workflows.
/// - On top, on its own workflow only: the ref of a pin of that workflow, so a favorite stays
///   startable, when it is a valid ref name and the listing names it or was cut off and cannot
///   prove it gone. Pins count in their order, at most `MAX_PINNED_REFS` per repository. A
///   vanished ref is still safe: `dispatch` reads the workflow at that ref before it claims
///   or sends anything, and GitHub's 404 rejects the start.
pub(super) fn offered_refs(
    default_branch: &str,
    listing: &RefListing,
    startable: &[u64],
    pins: &[(u64, &str)],
) -> BTreeMap<u64, Vec<String>> {
    let shared: BTreeSet<_> = bounded_refs(default_branch, listing, startable.len())
        .into_iter()
        .collect();
    let mut offered: BTreeMap<_, _> = startable.iter().map(|id| (*id, shared.clone())).collect();
    let mut extras = 0;
    for (workflow, name) in pins {
        if extras >= MAX_PINNED_REFS {
            break;
        }
        let Some(refs) = offered.get_mut(workflow) else {
            continue;
        };
        let listed = listing
            .branches
            .iter()
            .chain(&listing.tags)
            .any(|known| known == name);
        if valid_git_ref(name) && (listed || listing.truncated) && refs.insert((*name).into()) {
            extras += 1;
        }
    }
    offered
        .into_iter()
        .map(|(id, refs)| (id, refs.into_iter().collect()))
        .collect()
}

/// The refs every startable workflow of a repository offers, sorted: the default branch,
/// always, then branches and tags in turns, in GitHub's order, while `MAX_REFS` and the
/// repository's byte budget (shared by `copies` workflows) allow; neither kind starves the
/// other.
pub(super) fn bounded_refs(
    default_branch: &str,
    listing: &RefListing,
    copies: usize,
) -> Vec<String> {
    let budget = REPO_REF_BYTES / copies.max(1);
    // JSON quotes and a separator per entry.
    let cost = |name: &str| name.len() + 3;
    let mut used = cost(default_branch);
    let mut refs = BTreeSet::from([default_branch.to_owned()]);
    let turns = listing.branches.len().max(listing.tags.len());
    let alternating = (0..turns)
        .flat_map(|index| [listing.branches.get(index), listing.tags.get(index)])
        .flatten();
    for name in alternating {
        if refs.len() >= MAX_REFS {
            break;
        }
        if refs.contains(name) || used + cost(name) > budget {
            continue;
        }
        used += cost(name);
        refs.insert(name.clone());
    }
    refs.into_iter().collect()
}

// Installation Actions rights and the user's repository access are separate requirements.
pub(super) fn browser_repository_permission(installation: &Value, repo: &Value) -> Permission {
    let actions = installation["permissions"]["actions"]
        .as_str()
        .unwrap_or("");
    let contents = installation["permissions"]["contents"]
        .as_str()
        .unwrap_or("");
    if !matches!(actions, "read" | "write") {
        return Permission::Denied;
    }
    if actions == "write"
        && matches!(contents, "read" | "write")
        && repo["permissions"]["push"].as_bool() == Some(true)
    {
        Permission::Write
    } else {
        Permission::Read
    }
}
