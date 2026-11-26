//! Native authenticated GitHub access. Secrets never enter Snapshot or the metadata cache.
//!
//! `LiveStore` is the native engine; its behavior is split by concern:
//! `transport` (HTTPS and vault-backed bearer tokens), `connect` (adding and removing
//! accounts), `refresh` (catalog, metadata and run orchestration), `schedule` (per-scope
//! refresh timers), `catalog` (repository and ref listings), `last_run` (the latest run of
//! workflows without known runs), `browse` (repositories opened from the panel search),
//! `dispatch` (claimed, exactly-once workflow starts), `cancel` (run cancellation),
//! `hidden` (repositories the user hid from the panel lists) and `owners` (the repository
//! owners each account watches).
use crate::{
    auth::NativeKeyring,
    cache::{AppSettings, Cache, CacheError},
    github::{ApiClient, ApiError, ApiUrl, Transport},
    i18n::tr,
    models::*,
    workflow::{parse_workflow_yaml, validate_non_secret_inputs},
};
use base64::Engine as _;
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

mod browse;
mod cancel;
mod catalog;
mod connect;
mod dispatch;
mod hidden;
mod last_run;
mod owners;
mod refresh;
mod schedule;
mod transport;

use browse::Browsed;
use schedule::Schedule;
pub use transport::HttpTransport;

pub fn now() -> u64 {
    chrono::Utc::now().timestamp().max(0) as u64
}
pub(crate) fn url(path: &[&str], query: &[(&str, &str)]) -> Result<ApiUrl, String> {
    let mut value =
        reqwest::Url::parse("https://api.github.com").map_err(|_| invalid_api_address())?;
    value
        .path_segments_mut()
        .map_err(|_| invalid_api_address())?
        .extend(path);
    if !query.is_empty() {
        value.query_pairs_mut().extend_pairs(query.iter().copied());
    }
    ApiUrl::parse(value.as_str()).map_err(|_| invalid_api_address().into())
}
fn invalid_api_address() -> &'static str {
    tr("API-Adresse ungültig.", "Invalid API address.")
}
fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value[key].as_str().ok_or_else(|| {
        tr(
            "GitHub-Metadaten unvollständig.",
            "GitHub metadata incomplete.",
        )
        .into()
    })
}
fn number(value: &Value, key: &str) -> Result<u64, String> {
    value[key]
        .as_u64()
        .filter(|v| *v > 0)
        .ok_or_else(|| tr("GitHub-ID ungültig.", "Invalid GitHub ID.").into())
}
pub fn empty_snapshot() -> Snapshot {
    Snapshot {
        accounts: vec![],
        repos: vec![],
        workflows: vec![],
        runs: vec![],
        pins: vec![],
        mode: Mode::Live,
        connectivity: Connectivity::Online,
        updated_at: chrono::Utc::now().to_rfc3339(),
        stale: false,
        message: Some(
            tr(
                "Verbinde dein GitHub-Konto in den Einstellungen.",
                "Connect your GitHub account in the settings.",
            )
            .into(),
        ),
        uncertain_starts: vec![],
        hidden_repos: vec![],
        owner_filters: vec![],
    }
}
fn cache_notice() -> &'static str {
    tr(
        "Lokaler Zwischenspeicher konnte nicht aktualisiert werden.",
        "Local cache could not be updated.",
    )
}
fn account_save_failed() -> &'static str {
    tr(
        "Konto konnte nicht lokal gespeichert werden. Es wurde nichts verbunden; versuche es erneut.",
        "Account could not be saved locally. Nothing was connected; try again.",
    )
}
/// The controls it names sit at the end of the Repositories page (`ReposPage.svelte`).
fn browser_setup_notice() -> &'static str {
    tr(
        "Keine Repositories freigegeben – unter Repositories die Freigabe auf GitHub ändern.",
        "No repositories shared – change the access on GitHub under Repositories.",
    )
}

fn pin_credential_refused() -> &'static str {
    tr(
        "Namen oder Werte, die wie Zugangsdaten aussehen, werden nicht gespeichert.",
        "Names or values that look like credentials are not saved.",
    )
}
fn pin_too_large() -> &'static str {
    tr(
        "Favorit ist zu groß zum Speichern – kürze seine Eingaben.",
        "Favorite is too large to save – shorten its inputs.",
    )
}
fn pin_save_failed() -> &'static str {
    tr(
        "Favorit konnte nicht lokal gespeichert werden. Versuche es erneut.",
        "Favorite could not be saved locally. Try again.",
    )
}
fn pin_delete_failed() -> &'static str {
    tr(
        "Favorit konnte nicht lokal entfernt werden. Versuche es erneut.",
        "Favorite could not be removed locally. Try again.",
    )
}

/// The panel text for a favorite the cache refused. `CacheError` text is internal English;
/// after the store's own checks (input limits included), a refused value can only be a
/// credential-looking string, and an oversized entry is named as such.
fn pin_save_error(error: CacheError) -> String {
    match error {
        CacheError::Invalid(_) => pin_credential_refused(),
        CacheError::TooLarge => pin_too_large(),
        CacheError::Database(_) | CacheError::Serialization(_) => pin_save_failed(),
    }
    .into()
}

/// Appends a notice to a panel message, once.
fn with_notice(message: Option<String>, notice: &str) -> String {
    match message {
        Some(message) if message.contains(notice) => message,
        Some(message) => format!("{message} · {notice}"),
        None => notice.into(),
    }
}

/// The native engine. `T` is the GitHub transport; tests drive it through a synthetic one.
pub struct LiveStore<T: Transport = HttpTransport> {
    data: Snapshot,
    client: ApiClient<T>,
    keyring: Arc<dyn NativeKeyring>,
    cache: Cache,
    schedule: Schedule,
    poll_at: BTreeMap<String, u64>,
    history_since: BTreeMap<String, u64>,
    /// Repositories opened from the panel search this session; never persisted.
    browsed: Browsed,
    tracked: BTreeMap<u64, crate::github::RunTracker>,
    outcomes: RecentOutcomes<String, (String, DispatchOutcome)>,
    /// Mirrors the persisted dispatch claims; kept outside `data` so no reset can hide one.
    claims: Vec<UncertainStart>,
    /// UNIX seconds for refresh timers and backoff; tests step it forward.
    clock: fn() -> u64,
    /// The user's refresh interval (`refresh_interval`), read once and kept in step on save.
    refresh_seconds: u64,
    /// Repositories the user hid (`AppSettings::hidden_repos`), kept in step on save.
    hidden: Vec<HiddenRepo>,
    /// The owners each account watches (`AppSettings::owner_filters`), kept in step on save.
    owner_filters: Vec<OwnerFilter>,
    /// The failing scopes last written to the log, so a lasting failure is logged once.
    failure_log: crate::app_log::FailureLog,
}
impl LiveStore<HttpTransport> {
    pub fn new(cache: Cache, keyring: Arc<dyn NativeKeyring>) -> Result<Self, String> {
        let client = ApiClient::new(HttpTransport::new(keyring.clone())?);
        Self::with_client(cache, keyring, client)
    }
}
impl<T: Transport> LiveStore<T> {
    fn with_client(
        cache: Cache,
        keyring: Arc<dyn NativeKeyring>,
        client: ApiClient<T>,
    ) -> Result<Self, String> {
        let mut data = empty_snapshot();
        for account in cache.account_ids().map_err(|_| {
            tr(
                "Konten konnten nicht geladen werden.",
                "Accounts could not be loaded.",
            )
        })? {
            if let Some(cached) = cache.load_snapshot(&account).map_err(|_| {
                tr(
                    "Cache konnte nicht geladen werden.",
                    "Cache could not be loaded.",
                )
            })? {
                if cached.mode != Mode::Live {
                    continue;
                }
                data.accounts.extend(cached.accounts);
                data.repos.extend(cached.repos);
                data.workflows.extend(cached.workflows);
                data.runs.extend(cached.runs);
                data.pins.extend(cached.pins);
                data.updated_at = cached.updated_at;
            }
        }
        let claims = cache.uncertain_starts().map_err(|_| {
            tr(
                "Startsperren konnten nicht geladen werden.",
                "Start locks could not be loaded.",
            )
        })?;
        // An unreadable preference keeps the default interval and hides or filters nothing.
        let settings = cache.load_app_settings().unwrap_or_default();
        let refresh_seconds = settings.refresh_interval();
        crate::app_log::set_level(settings.log_level.unwrap_or_default());
        let hidden = settings.hidden_repos;
        let owner_filters = settings.owner_filters;
        if !data.accounts.is_empty() {
            data.stale = true;
            data.connectivity = Connectivity::Offline;
            data.message = Some(
                tr(
                    "Letzter gespeicherter Stand · Verbindung wird geprüft.",
                    "Last saved state · checking the connection.",
                )
                .into(),
            );
        }
        Ok(Self {
            client,
            data,
            keyring,
            cache,
            schedule: Schedule::new(refresh_seconds),
            poll_at: BTreeMap::new(),
            history_since: BTreeMap::new(),
            browsed: Browsed::default(),
            tracked: BTreeMap::new(),
            outcomes: RecentOutcomes::new(MAX_RECORDED_OUTCOMES),
            claims,
            clock: now,
            refresh_seconds,
            hidden,
            owner_filters,
            failure_log: crate::app_log::FailureLog::default(),
        })
    }
    /// A denied repository's open runs are frozen at their last known state; they leave the
    /// panel instead of looking current. Completed runs stay as history.
    pub fn snapshot(&self) -> Snapshot {
        let mut snapshot = self.data.clone();
        snapshot.uncertain_starts = self.claims.clone();
        snapshot.hidden_repos = self.hidden_of_accounts();
        snapshot.owner_filters = self.owner_filters_of_accounts();
        let denied: BTreeSet<_> = snapshot
            .repos
            .iter()
            .filter(|repo| repo.permission == Permission::Denied)
            .map(|repo| repo.id.clone())
            .collect();
        snapshot
            .runs
            .retain(|run| run.status == RunStatus::Completed || !denied.contains(&run.repo_id));
        snapshot
    }
    /// An unreadable preference keeps the default: notifications off.
    pub fn notifications_enabled(&self) -> bool {
        self.cache
            .load_app_settings()
            .is_ok_and(|settings| settings.notifications)
    }
    pub fn set_notifications_enabled(&mut self, enabled: bool) -> Result<(), String> {
        let mut settings: AppSettings = self.cache.load_app_settings().unwrap_or_default();
        settings.notifications = enabled;
        self.cache
            .save_app_settings(&settings)
            .map_err(|_| hidden::hidden_save_failed().into())
    }
    /// Seconds between idle refreshes; an open run is still followed every 15 seconds.
    pub fn refresh_interval(&self) -> u64 {
        self.refresh_seconds
    }
    /// Validates, stores and applies a new interval; a shorter one also brings each healthy
    /// account's next catalog read forward to within the new interval.
    pub fn set_refresh_interval(&mut self, seconds: u64) -> Result<u64, String> {
        let seconds = crate::refresh_interval::validate(seconds)?;
        let mut settings: AppSettings = self.cache.load_app_settings().unwrap_or_default();
        settings.refresh_seconds = Some(seconds);
        self.cache
            .save_app_settings(&settings)
            .map_err(|_| hidden::hidden_save_failed())?;
        self.refresh_seconds = seconds;
        self.schedule.set_refresh(seconds, (self.clock)());
        Ok(seconds)
    }
    /// The stored log level; an unreadable preference keeps the default: info.
    pub fn log_level(&self) -> crate::app_log::Level {
        let settings = self.cache.load_app_settings().unwrap_or_default();
        settings.log_level.unwrap_or_default()
    }
    /// Stores the log level and applies it at once.
    pub fn set_log_level(
        &mut self,
        level: crate::app_log::Level,
    ) -> Result<crate::app_log::Level, String> {
        let mut settings: AppSettings = self.cache.load_app_settings().unwrap_or_default();
        settings.log_level = Some(level);
        self.cache
            .save_app_settings(&settings)
            .map_err(|_| hidden::hidden_save_failed())?;
        crate::app_log::set_level(level);
        crate::app_log::info(&format!("settings: log level={}", level.label()));
        Ok(level)
    }
    /// Whether the panel shows an open run (queued, running, waiting; not stuck), or a started run is still
    /// being followed, so a refresh is due every 15 seconds
    /// (`refresh_interval::OPEN_RUN_SECONDS`). Matches `snapshot`: a denied repository's
    /// frozen open runs do not count.
    pub fn has_open_run(&self) -> bool {
        let open = self.data.runs.iter().any(|run| {
            run.status.is_open()
                && !self
                    .data
                    .repos
                    .iter()
                    .any(|repo| repo.id == run.repo_id && repo.permission == Permission::Denied)
        });
        // A started run the history does not list yet, e.g. after its first follow-up read
        // failed, is followed at the open-run pace too, as long as a refresh still reads it.
        open || self.tracked.values().any(|tracker| {
            self.data.repos.iter().any(|repo| {
                repo.id == tracker.repo_id && repo.selected && repo.permission != Permission::Denied
            })
        })
    }
    fn pages(
        &self,
        account: &str,
        path: &[&str],
        query: &[(&str, &str)],
    ) -> Result<Vec<Value>, ApiError> {
        let url = url(path, query).map_err(|_| ApiError::Policy(invalid_api_address()))?;
        self.client
            .get_pages(account, url, (self.clock)())?
            .iter()
            .map(|body| serde_json::from_slice(body).map_err(|_| ApiError::InvalidResponse))
            .collect()
    }
    fn one(&self, account: &str, path: &[&str], query: &[(&str, &str)]) -> Result<Value, ApiError> {
        self.pages(account, path, query)?
            .into_iter()
            .next()
            .ok_or(ApiError::InvalidResponse)
    }
    /// Writes the public metadata cache. A failed write never marks data stale: what is in
    /// memory is still what GitHub returned, so the failure only becomes a notice.
    fn persist(&mut self) {
        let mut failed = false;
        for account in &self.data.accounts {
            failed |= self.cache.save_snapshot(&account.id, &self.data).is_err();
        }
        if failed {
            self.data.message = Some(with_notice(self.data.message.take(), cache_notice()));
        }
    }
    fn yaml(
        &self,
        repo: &Repo,
        path: &str,
        git_ref: &str,
    ) -> Result<crate::workflow::ParsedWorkflow, String> {
        if !path.starts_with(".github/workflows/")
            || path.split('/').any(|s| s == ".." || s.is_empty())
        {
            return Err(tr("Workflow-Pfad ungültig.", "Invalid workflow path.").into());
        }
        let mut segments = vec!["repos", &repo.owner, &repo.name, "contents"];
        segments.extend(path.split('/'));
        let value = self
            .one(&repo.account_id, &segments, &[("ref", git_ref)])
            .map_err(|error| match error {
                // Dispatch reads the default branch first; a 404 at another ref means that
                // branch or tag is gone or does not contain the workflow file.
                ApiError::Http(404) if git_ref != repo.default_branch => crate::trf!(
                    "Workflow auf „{git_ref}“ nicht gefunden – der Branch oder Tag fehlt oder enthält den Workflow nicht.",
                    "Workflow not found on “{git_ref}” – the branch or tag is missing or does not contain the workflow."
                ),
                other => other.to_string(),
            })?;
        let invalid = || tr("Workflow-Inhalt ungültig.", "Invalid workflow content.");
        if value["encoding"] != "base64" {
            return Err(invalid().into());
        }
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(text(&value, "content")?.replace(['\n', '\r'], ""))
            .map_err(|_| invalid())?;
        let source = std::str::from_utf8(&bytes)
            .map_err(|_| tr("Workflow ist kein UTF-8.", "Workflow is not UTF-8."))?;
        parse_workflow_yaml(source).map_err(|e| e.to_string())
    }
    /// Pins have their own cache table, so saving or deleting one never rewrites the snapshot.
    pub fn save_pin(&mut self, pin: Pin) -> Result<Snapshot, String> {
        validate_pin_fields(&pin)?;
        check_pin_room(&self.data.pins, &pin.id)?;
        validate_non_secret_inputs(&pin.inputs).map_err(|e| e.to_string())?;
        if !self
            .data
            .repos
            .iter()
            .any(|r| r.id == pin.repo_id && r.account_id == pin.account_id)
            || !self
                .data
                .workflows
                .iter()
                .any(|w| w.id == pin.workflow_id && w.repo_id == pin.repo_id)
        {
            return Err(tr(
                "Favorit passt nicht zum Konto oder Repository.",
                "Favorite does not match the account or repository.",
            )
            .into());
        }
        if self
            .data
            .pins
            .iter()
            .any(|p| p.id == pin.id && p.account_id != pin.account_id)
        {
            return Err(tr(
                "Ein Favorit kann nicht in ein anderes Konto verschoben werden.",
                "A favorite cannot be moved to another account.",
            )
            .into());
        }
        self.cache
            .save_pin(&pin.account_id, &pin)
            .map_err(pin_save_error)?;
        self.data.pins.retain(|p| p.id != pin.id);
        self.data.pins.push(pin);
        Ok(self.snapshot())
    }
    pub fn delete_pin(&mut self, id: String) -> Result<Snapshot, String> {
        if let Some(pin) = self.data.pins.iter().find(|p| p.id == id) {
            self.cache
                .delete_pin(&pin.account_id, &id)
                .map_err(|_| pin_delete_failed())?;
        }
        self.data.pins.retain(|p| p.id != id);
        Ok(self.snapshot())
    }
}

#[cfg(test)]
mod tests;
