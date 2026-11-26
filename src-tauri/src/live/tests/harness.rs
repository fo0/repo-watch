//! The synthetic GitHub, vault and engine harness shared by the orchestration tests.
use crate::auth::{AuthError, SecretToken};
use crate::browser_auth;
use crate::github::{ApiRequest, ApiResponse, Method, TransportError};
use crate::live::*;
use std::cell::Cell;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard};

pub(super) const START: u64 = 1_800_000_000;
thread_local! {
    static NOW: Cell<u64> = const { Cell::new(START) };
}
pub(super) fn clock() -> u64 {
    NOW.with(Cell::get)
}
pub(super) fn advance(seconds: u64) {
    NOW.with(|now| now.set(now.get() + seconds));
}
/// The system clock jumps back, as after a time-zone or NTP correction.
pub(super) fn rewind(seconds: u64) {
    NOW.with(|now| now.set(now.get() - seconds));
}

pub(super) fn json(status: u16, body: Value) -> Result<ApiResponse, TransportError> {
    Ok(ApiResponse {
        status,
        headers: BTreeMap::new(),
        body: serde_json::to_vec(&body).unwrap(),
    })
}
fn names(values: &[String]) -> Value {
    values
        .iter()
        .map(|name| serde_json::json!({ "name": name }))
        .collect()
}
pub(super) fn run(repo: u64, id: u64, status: &str) -> Value {
    serde_json::json!({
        "id": id, "workflow_id": 9, "name": "Fixture build", "head_branch": "main",
        "status": status,
        "conclusion": if status == "completed" { Some("success") } else { None },
        "created_at": "2026-11-01T10:00:00Z", "updated_at": "2026-11-01T10:01:00Z",
        "html_url": format!("https://github.com/fixture-owner/fixture-{repo}/actions/runs/{id}"),
    })
}
pub(super) fn confirmation(repo: u64, id: u64) -> Result<ApiResponse, TransportError> {
    json(
        200,
        serde_json::json!({
            "workflow_run_id": id,
            "run_url": format!("https://api.github.com/repos/fixture-owner/fixture-{repo}/actions/runs/{id}"),
            "html_url": format!("https://github.com/fixture-owner/fixture-{repo}/actions/runs/{id}"),
        }),
    )
}

/// Synthetic GitHub: accounts list `fixture-owner/fixture-<n>` repositories (or another
/// owner's, see `owners`), each with
/// one startable workflow (ID 9) on every known branch and tag.
#[derive(Default)]
pub(super) struct Remote {
    pub(super) repos: BTreeMap<String, Vec<u64>>,
    /// `pushed_at` of a listed repository; unlisted ones carry none.
    pub(super) pushed: BTreeMap<u64, String>,
    /// Owner login of a listed repository; unlisted ones belong to `fixture-owner`.
    pub(super) owners: BTreeMap<u64, &'static str>,
    /// Status every request of an account is answered with (401: revoked).
    pub(super) account_status: BTreeMap<String, u16>,
    /// Accounts whose vault entry is gone: the transport fails before sending, as
    /// `HttpTransport` does.
    pub(super) missing: BTreeSet<String>,
    /// Status the Actions run endpoints of a repository answer with.
    pub(super) runs_status: BTreeMap<u64, u16>,
    pub(super) runs: BTreeMap<u64, Vec<Value>>,
    pub(super) branches: Vec<String>,
    pub(super) tags: Vec<String>,
    /// Branch listings paginate without end, so the ref picker cuts them off.
    pub(super) endless_branches: bool,
    /// Workflow listings paginate without end, past the transport's page budget.
    pub(super) endless_workflows: bool,
    /// The workflow file on every known ref; `on: workflow_dispatch` without inputs if unset.
    pub(super) workflow_source: Option<&'static str>,
    pub(super) environments: Vec<String>,
    /// Environment listings paginate without end, past the transport's page budget.
    pub(super) endless_environments: bool,
    pub(super) dispatches: VecDeque<Result<ApiResponse, TransportError>>,
    /// Answers to run cancellations, in order; an unexpected one fails the test.
    pub(super) cancels: VecDeque<Result<ApiResponse, TransportError>>,
    /// Workflows listed after the startable fixture workflow (ID 9), as GitHub's JSON.
    pub(super) extra_workflows: Vec<Value>,
    /// Older runs per repository that only a workflow's run listing still shows: the
    /// repository's run list no longer reaches them.
    pub(super) archived: BTreeMap<u64, Vec<Value>>,
    /// Status a workflow's run listing answers with, by workflow ID.
    pub(super) workflow_runs_status: BTreeMap<u64, u16>,
    /// Every request as `(account, method, path?query)`.
    pub(super) requests: Vec<(String, Method, String)>,
    /// Claim rows in the cache file at the moment each dispatch POST left.
    pub(super) claims_at_post: Vec<i64>,
    pub(super) db: PathBuf,
}
impl Remote {
    pub(super) fn new() -> Self {
        Self {
            repos: BTreeMap::from([("github-1".into(), vec![1])]),
            branches: vec!["main".into(), "feature".into()],
            tags: vec!["v1".into()],
            ..Self::default()
        }
    }
    pub(super) fn posts(&self) -> usize {
        self.requests
            .iter()
            .filter(|(_, method, _)| *method == Method::Post)
            .count()
    }
    pub(super) fn count(&self, account: &str, fragment: &str) -> usize {
        self.requests
            .iter()
            .filter(|(owner, _, target)| owner == account && target.contains(fragment))
            .count()
    }
    fn repository(
        &mut self,
        method: Method,
        repo: u64,
        rest: &[&str],
        query: &BTreeMap<String, String>,
    ) -> Result<ApiResponse, TransportError> {
        match rest {
            ["branches"] if self.endless_branches => {
                let page: u32 = query.get("page").and_then(|p| p.parse().ok()).unwrap_or(1);
                let listed: Vec<_> = (0..100).map(|i| format!("branch-{page}-{i}")).collect();
                let next = format!("<https://api.github.com/repos/fixture-owner/fixture-{repo}/branches?per_page=100&page={}>; rel=\"next\"", page + 1);
                Ok(ApiResponse {
                    status: 200,
                    headers: BTreeMap::from([("Link".to_string(), next)]),
                    body: serde_json::to_vec(&names(&listed)).unwrap(),
                })
            }
            ["actions", "workflows"] if self.endless_workflows => {
                let page: u32 = query.get("page").and_then(|p| p.parse().ok()).unwrap_or(1);
                let next = format!("<https://api.github.com/repos/fixture-owner/fixture-{repo}/actions/workflows?per_page=100&page={}>; rel=\"next\"", page + 1);
                Ok(ApiResponse {
                    status: 200,
                    headers: BTreeMap::from([("Link".to_string(), next)]),
                    body: br#"{"workflows": []}"#.to_vec(),
                })
            }
            ["environments"] if self.endless_environments => {
                let page: u32 = query.get("page").and_then(|p| p.parse().ok()).unwrap_or(1);
                let listed: Vec<_> = (0..100).map(|i| format!("env-{page}-{i}")).collect();
                let next = format!("<https://api.github.com/repos/fixture-owner/fixture-{repo}/environments?per_page=100&page={}>; rel=\"next\"", page + 1);
                Ok(ApiResponse {
                    status: 200,
                    headers: BTreeMap::from([("Link".to_string(), next)]),
                    body: serde_json::to_vec(
                        &serde_json::json!({ "environments": names(&listed) }),
                    )
                    .unwrap(),
                })
            }
            ["environments"] => json(
                200,
                serde_json::json!({ "environments": names(&self.environments) }),
            ),
            ["branches"] => json(200, names(&self.branches)),
            ["tags"] => json(200, names(&self.tags)),
            ["actions", "workflows"] => {
                let mut listed = vec![
                    serde_json::json!({"id": 9, "name": "Fixture build", "path": ".github/workflows/build.yml", "state": "active"}),
                ];
                listed.extend(self.extra_workflows.iter().cloned());
                json(200, serde_json::json!({ "workflows": listed }))
            }
            ["actions", "workflows", id, "runs"] => {
                let id: u64 = id.parse().unwrap();
                if let Some(status) = self.workflow_runs_status.get(&id) {
                    return json(*status, serde_json::json!({}));
                }
                let mut listed: Vec<_> = [&self.runs, &self.archived]
                    .into_iter()
                    .filter_map(|runs| runs.get(&repo))
                    .flatten()
                    .filter(|run| run["workflow_id"] == id)
                    .cloned()
                    .collect();
                listed.sort_by(|a, b| b["created_at"].as_str().cmp(&a["created_at"].as_str()));
                let per_page: usize = query
                    .get("per_page")
                    .and_then(|p| p.parse().ok())
                    .unwrap_or(30);
                let mut headers = BTreeMap::new();
                if listed.len() > per_page {
                    let next = format!("<https://api.github.com/repos/fixture-owner/fixture-{repo}/actions/workflows/{id}/runs?per_page={per_page}&page=2>; rel=\"next\"");
                    headers.insert("Link".to_string(), next);
                }
                listed.truncate(per_page);
                Ok(ApiResponse {
                    status: 200,
                    headers,
                    body: serde_json::to_vec(&serde_json::json!({ "workflow_runs": listed }))
                        .unwrap(),
                })
            }
            ["contents", ".github", "workflows", _] => {
                let git_ref = query.get("ref").cloned().unwrap_or_default();
                if self.branches.contains(&git_ref) || self.tags.contains(&git_ref) {
                    let source = self.workflow_source.unwrap_or("on: workflow_dispatch\n");
                    let content = base64::engine::general_purpose::STANDARD.encode(source);
                    json(
                        200,
                        serde_json::json!({"encoding": "base64", "content": content}),
                    )
                } else {
                    json(404, serde_json::json!({"message": "No commit found"}))
                }
            }
            ["actions", "workflows", "9", "dispatches"] => {
                assert_eq!(method, Method::Post);
                let claims = rusqlite::Connection::open(&self.db)
                    .unwrap()
                    .query_row("SELECT COUNT(*) FROM dispatch_claims", [], |row| row.get(0))
                    .unwrap();
                self.claims_at_post.push(claims);
                self.dispatches
                    .pop_front()
                    .expect("unexpected dispatch POST")
            }
            ["actions", "runs", _, "cancel"] => {
                assert_eq!(method, Method::Post);
                self.cancels.pop_front().expect("unexpected cancel POST")
            }
            ["actions", "runs", ..] if self.runs_status.contains_key(&repo) => {
                json(self.runs_status[&repo], serde_json::json!({}))
            }
            ["actions", "runs"] => {
                let listed = self.runs.get(&repo).cloned().unwrap_or_default();
                let listed: Vec<_> = match query.get("status") {
                    Some(status) => listed
                        .into_iter()
                        .filter(|run| run["status"] == *status)
                        .collect(),
                    None => listed,
                };
                json(200, serde_json::json!({ "workflow_runs": listed }))
            }
            ["actions", "runs", id] => {
                let id: u64 = id.parse().unwrap();
                let found = self
                    .runs
                    .get(&repo)
                    .and_then(|runs| runs.iter().find(|run| run["id"] == id));
                match found {
                    Some(run) => json(200, run.clone()),
                    None => json(404, serde_json::json!({"message": "Not Found"})),
                }
            }
            other => panic!("unexpected repository request {other:?}"),
        }
    }
}

pub(super) struct Fake(Arc<Mutex<Remote>>);
impl Transport for Fake {
    fn send(&mut self, request: &ApiRequest) -> Result<ApiResponse, TransportError> {
        let mut remote = self.0.lock().unwrap();
        let uri = reqwest::Url::parse(request.url().as_str()).unwrap();
        let account = request.account_id().to_owned();
        let target = match uri.query() {
            Some(query) => format!("{}?{query}", uri.path()),
            None => uri.path().to_owned(),
        };
        remote
            .requests
            .push((account.clone(), request.method(), target));
        if remote.missing.contains(&account) {
            return Err(TransportError::NoCredential);
        }
        if let Some(status) = remote.account_status.get(&account) {
            return json(*status, serde_json::json!({}));
        }
        let query: BTreeMap<String, String> = uri.query_pairs().into_owned().collect();
        let segments: Vec<&str> = uri.path_segments().unwrap().collect();
        match segments.as_slice() {
            ["user", "repos"] => {
                let listed: Vec<_> = remote
                    .repos
                    .get(&account)
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|n| serde_json::json!({"id": n, "name": format!("fixture-{n}"), "owner": {"login": remote.owners.get(&n).copied().unwrap_or("fixture-owner")}, "default_branch": "main", "pushed_at": remote.pushed.get(&n), "permissions": {"push": true}}))
                    .collect();
                json(200, Value::Array(listed))
            }
            ["repos", _owner, name, rest @ ..] => {
                let repo = name.trim_start_matches("fixture-").parse().unwrap();
                remote.repository(request.method(), repo, rest, &query)
            }
            other => panic!("unexpected request {other:?}"),
        }
    }
}

/// Synthetic vault that only records which accounts hold a credential.
#[derive(Default)]
pub(super) struct Vault {
    pub(super) held: Mutex<BTreeSet<String>>,
    /// Every delete fails, as a vault that refuses to remove an entry.
    pub(super) refuse_delete: AtomicBool,
    /// Every read fails, as a vault that is unavailable.
    pub(super) refuse_read: AtomicBool,
}
impl Vault {
    pub(super) fn holds(&self, account: &str) -> bool {
        self.held.lock().unwrap().contains(account)
    }
}
impl NativeKeyring for Vault {
    fn read_token(&self, account: &str) -> Result<Option<SecretToken>, AuthError> {
        if self.refuse_read.load(Ordering::SeqCst) {
            return Err(AuthError::KeyringUnavailable);
        }
        Ok(self
            .holds(account)
            .then(|| SecretToken::new("synthetic-credential".into())))
    }
    fn write_token(&self, account: &str, _: &SecretToken) -> Result<(), AuthError> {
        self.held.lock().unwrap().insert(account.into());
        Ok(())
    }
    fn delete_token(&self, account: &str) -> Result<(), AuthError> {
        if self.refuse_delete.load(Ordering::SeqCst) {
            return Err(AuthError::KeyringUnavailable);
        }
        self.held.lock().unwrap().remove(account);
        Ok(())
    }
}

/// The selected repository IDs in catalog order.
pub(super) fn selected_ids(snapshot: &Snapshot) -> Vec<&str> {
    snapshot
        .repos
        .iter()
        .filter(|repo| repo.selected)
        .map(|repo| repo.id.as_str())
        .collect()
}
pub(super) fn account(id: &str) -> Account {
    Account {
        id: id.into(),
        login: format!("fixture-{id}"),
        label: "GitHub · Token".into(),
    }
}
pub(super) fn request(id: &str, git_ref: &str) -> DispatchRequest {
    DispatchRequest {
        account_id: "github-1".into(),
        repo_id: "github-1:1".into(),
        workflow_id: 9,
        git_ref: git_ref.into(),
        inputs: Inputs::new(),
        request_id: id.into(),
    }
}
pub(super) fn commit(id: &'static str) -> impl FnOnce(&dyn NativeKeyring) -> Result<(), String> {
    move |keyring| {
        browser_auth::commit_credential(
            keyring,
            id,
            &SecretToken::new("synthetic-credential".into()),
            || true,
        )
    }
}
/// A synthetic disk failure for every snapshot row whose payload matches `pattern`.
pub(super) fn refuse(path: &Path, pattern: &str) {
    let connection = rusqlite::Connection::open(path).unwrap();
    for event in ["INSERT", "UPDATE"] {
        connection
            .execute_batch(&format!(
                "CREATE TRIGGER refuse_{event} BEFORE {event} ON snapshots
                     WHEN NEW.payload LIKE '{pattern}'
                     BEGIN SELECT RAISE(ABORT, 'synthetic write failure'); END;"
            ))
            .unwrap();
    }
}

pub(super) struct Harness {
    pub(super) live: LiveStore<Fake>,
    pub(super) remote: Arc<Mutex<Remote>>,
    pub(super) vault: Arc<Vault>,
    pub(super) path: PathBuf,
}
impl Harness {
    pub(super) fn new(name: &str, remote: Remote, accounts: &[&str]) -> Self {
        NOW.with(|now| now.set(START));
        let path = std::env::temp_dir().join(format!(
            "repowatch-live-{name}-{}-{}.db",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let remote = Arc::new(Mutex::new(Remote {
            db: path.clone(),
            ..remote
        }));
        let vault = Arc::new(Vault::default());
        let mut live = Self::open(&path, &remote, &vault);
        live.data.accounts = accounts.iter().map(|id| account(id)).collect();
        Self {
            live,
            remote,
            vault,
            path,
        }
    }
    /// Another engine on the same cache file, as after a restart.
    pub(super) fn open(
        path: &Path,
        remote: &Arc<Mutex<Remote>>,
        vault: &Arc<Vault>,
    ) -> LiveStore<Fake> {
        let mut live = LiveStore::with_client(
            Cache::open(path).unwrap(),
            vault.clone(),
            ApiClient::new(Fake(remote.clone())),
        )
        .unwrap();
        live.clock = clock;
        live
    }
    pub(super) fn restart(&self) -> LiveStore<Fake> {
        Self::open(&self.path, &self.remote, &self.vault)
    }
    /// Lists the catalogs, which select their repositories automatically, and reads the
    /// metadata and runs of the selected ones; asserts exactly these were selected.
    pub(super) fn load(&mut self, selected: &[&str]) {
        let snapshot = self.live.refresh();
        assert_eq!(selected_ids(&snapshot), selected);
    }
    pub(super) fn remote(&self) -> MutexGuard<'_, Remote> {
        self.remote.lock().unwrap()
    }
}
impl Drop for Harness {
    fn drop(&mut self) {
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{suffix}", self.path.display()));
        }
    }
}
