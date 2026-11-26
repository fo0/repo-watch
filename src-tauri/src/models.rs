use crate::i18n::tr;
use crate::trf;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum InputValue {
    String(String),
    Number(serde_json::Number),
    Boolean(bool),
}

impl InputValue {
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::String(value) if value.trim().is_empty())
    }
}

pub type Inputs = BTreeMap<String, InputValue>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    pub login: String,
    pub label: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Permission {
    Write,
    Read,
    Denied,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Repo {
    pub id: String,
    pub account_id: String,
    pub owner: String,
    pub name: String,
    pub default_branch: String,
    /// GitHub's last push (RFC 3339); ranks the automatic selection. Older cache rows lack it.
    #[serde(default)]
    pub pushed_at: Option<String>,
    pub selected: bool,
    pub permission: Permission,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InputType {
    String,
    Boolean,
    Choice,
    Number,
    Environment,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputSpec {
    pub name: String,
    pub label: String,
    pub r#type: InputType,
    pub required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<InputValue>,
    #[serde(default)]
    pub options: Vec<String>,
    /// Why the input offers no choice right now (in the app's language, shown with it); no value is valid then.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Workflow {
    pub id: u64,
    pub repo_id: String,
    pub name: String,
    pub path: String,
    pub dispatch: bool,
    pub inputs: Vec<InputSpec>,
    pub refs: Vec<String>,
    /// When the workflow's latest run was created (RFC 3339), looked up only for workflows
    /// without a run in `Snapshot.runs`; `None` when unknown. Older cache rows lack it.
    #[serde(default)]
    pub last_run_at: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Queued,
    InProgress,
    Completed,
    Waiting,
    /// GitHub still lists the run as queued more than `run_history::STUCK_AFTER_SECONDS` after its
    /// last change: an orphan GitHub never starts and refuses to cancel (409). Not open — it
    /// leaves „Läuft gerade“, the open-run pace and the tray's active state.
    Stuck,
}

impl RunStatus {
    /// Queued, running or waiting: GitHub may still move it, so it is followed closely.
    pub fn is_open(self) -> bool {
        matches!(self, Self::Queued | Self::InProgress | Self::Waiting)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Conclusion {
    Success,
    Failure,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Run {
    pub id: u64,
    pub repo_id: String,
    pub workflow_id: u64,
    pub name: String,
    pub branch: String,
    pub status: RunStatus,
    pub conclusion: Option<Conclusion>,
    pub created_at: String,
    pub updated_at: String,
    pub html_url: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pin {
    pub id: String,
    pub label: String,
    pub account_id: String,
    pub repo_id: String,
    pub workflow_id: u64,
    #[serde(rename = "ref")]
    pub git_ref: String,
    pub inputs: Inputs,
    pub quick_start: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Demo,
    Live,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Connectivity {
    Online,
    Offline,
    Forbidden,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub accounts: Vec<Account>,
    pub repos: Vec<Repo>,
    pub workflows: Vec<Workflow>,
    pub runs: Vec<Run>,
    pub pins: Vec<Pin>,
    pub mode: Mode,
    pub connectivity: Connectivity,
    pub updated_at: String,
    pub stale: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Targets whose last start has an unknown outcome; each stays locked until released explicitly.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub uncertain_starts: Vec<UncertainStart>,
    /// Repositories the user hid from the panel lists; omitted when none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hidden_repos: Vec<HiddenRepo>,
    /// The owners each account watches, for accounts with a filter; omitted when none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub owner_filters: Vec<OwnerFilter>,
}

/// The repository owners (the personal account, organizations) an account watches. An account
/// without a filter watches all of its owners.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnerFilter {
    pub account_id: String,
    pub owners: Vec<String>,
}

impl OwnerFilter {
    /// Whether the filter names this owner; GitHub logins compare case-insensitively.
    pub fn watches(&self, owner: &str) -> bool {
        self.owners
            .iter()
            .any(|known| known.to_lowercase() == owner.to_lowercase())
    }
}

/// A repository the user hid from the panel lists, by account and repository. Owner and name
/// keep it nameable in Settings after it left the account's catalog.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HiddenRepo {
    pub account_id: String,
    pub repo_id: String,
    pub owner: String,
    pub name: String,
}

impl HiddenRepo {
    pub fn matches(&self, account_id: &str, repo_id: &str) -> bool {
        self.account_id == account_id && self.repo_id == repo_id
    }
}

/// The scope a dispatch claim locks: one workflow on one ref of one account's repository.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DispatchTarget {
    pub account_id: String,
    pub repo_id: String,
    pub workflow_id: u64,
    #[serde(rename = "ref")]
    pub git_ref: String,
}

impl From<&DispatchRequest> for DispatchTarget {
    fn from(request: &DispatchRequest) -> Self {
        Self {
            account_id: request.account_id.clone(),
            repo_id: request.repo_id.clone(),
            workflow_id: request.workflow_id,
            git_ref: request.git_ref.clone(),
        }
    }
}

/// A persisted claim whose start may or may not have reached GitHub.
/// `claimed_at` is `None` only for claims written before claim times were stored.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UncertainStart {
    pub account_id: String,
    pub repo_id: String,
    pub workflow_id: u64,
    #[serde(rename = "ref")]
    pub git_ref: String,
    pub claimed_at: Option<String>,
}

impl UncertainStart {
    pub fn target(&self) -> DispatchTarget {
        DispatchTarget {
            account_id: self.account_id.clone(),
            repo_id: self.repo_id.clone(),
            workflow_id: self.workflow_id,
            git_ref: self.git_ref.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DispatchRequest {
    pub account_id: String,
    pub repo_id: String,
    pub workflow_id: u64,
    #[serde(rename = "ref")]
    pub git_ref: String,
    pub inputs: Inputs,
    pub request_id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DispatchState {
    Accepted,
    Uncertain,
    Rejected,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DispatchOutcome {
    pub state: DispatchState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub html_url: Option<String>,
    pub message: String,
}

impl DispatchOutcome {
    pub fn rejected(message: impl Into<String>) -> Self {
        Self {
            state: DispatchState::Rejected,
            run_id: None,
            html_url: None,
            message: message.into(),
        }
    }

    pub fn uncertain(message: impl Into<String>) -> Self {
        Self {
            state: DispatchState::Uncertain,
            run_id: None,
            html_url: None,
            message: message.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DemoScenario {
    #[default]
    Normal,
    Offline,
    Forbidden,
    Uncertain,
}

impl std::str::FromStr for DemoScenario {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "normal" => Ok(Self::Normal),
            "offline" => Ok(Self::Offline),
            "forbidden" => Ok(Self::Forbidden),
            "uncertain" => Ok(Self::Uncertain),
            _ => Err("Unknown demo scenario.".into()),
        }
    }
}

pub const MAX_REQUEST_ID_CHARS: usize = 200;
pub const MAX_PIN_ID_CHARS: usize = 120;
pub const MAX_PIN_LABEL_CHARS: usize = 120;
/// Saved favorites across all accounts; each is one row and one entry in every snapshot.
pub const MAX_PINS: usize = 100;
/// Git's own limit for a ref name, in bytes.
pub const MAX_GIT_REF_BYTES: usize = 255;
/// Dispatch request IDs each store remembers for idempotent retries.
pub const MAX_RECORDED_OUTCOMES: usize = 256;

fn bounded_text(value: &str, max_chars: usize) -> bool {
    !value.trim().is_empty()
        && value.chars().count() <= max_chars
        && !value.chars().any(char::is_control)
}

/// Shared by the demo and live stores so both enforce the same request limits.
pub fn validate_request_id(request_id: &str) -> Result<(), String> {
    if bounded_text(request_id, MAX_REQUEST_ID_CHARS) {
        Ok(())
    } else {
        Err(tr(
            "Start-ID fehlt oder ist zu lang.",
            "Start ID is missing or too long.",
        )
        .into())
    }
}

/// Shared by the demo and live stores for the user-written pin fields.
pub fn validate_pin_fields(pin: &Pin) -> Result<(), String> {
    if !bounded_text(&pin.id, MAX_PIN_ID_CHARS) || !bounded_text(&pin.label, MAX_PIN_LABEL_CHARS) {
        return Err(tr(
            "Ein Favorit braucht eine ID und einen Namen mit höchstens 120 Zeichen.",
            "A favorite needs an ID and a name of at most 120 characters.",
        )
        .into());
    }
    if !valid_git_ref(&pin.git_ref) {
        return Err(tr(
            "Branch oder Tag des Favoriten ist ungültig.",
            "The favorite's branch or tag is invalid.",
        )
        .into());
    }
    crate::workflow::validate_pin_inputs(&pin.inputs).map_err(|error| error.to_string())
}

/// Shared by the demo and live stores: a new favorite needs room; replacing one never does.
pub fn check_pin_room(pins: &[Pin], id: &str) -> Result<(), String> {
    if pins.len() >= MAX_PINS && !pins.iter().any(|pin| pin.id == id) {
        return Err(trf!(
            "Höchstens {MAX_PINS} Favoriten – entferne zuerst einen anderen.",
            "At most {MAX_PINS} favorites – remove another one first."
        ));
    }
    Ok(())
}

/// The essentials of `git check-ref-format` for a branch or tag name: at most 255 bytes, no
/// control characters, whitespace or any of `~^:?*[\`, no `..` or `@{`, not `@`, no leading
/// `-`, no trailing `.`, and no empty component (so no leading, trailing or double `/`) or
/// component that starts with `.` or ends with `.lock`.
pub fn valid_git_ref(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= MAX_GIT_REF_BYTES
        && name != "@"
        && !name.starts_with('-')
        && !name.ends_with('.')
        && !name.contains("..")
        && !name.contains("@{")
        && !name
            .chars()
            .any(|c| c.is_control() || c.is_whitespace() || "~^:?*[\\".contains(c))
        && name
            .split('/')
            .all(|part| !part.is_empty() && !part.starts_with('.') && !part.ends_with(".lock"))
}

/// Insertion-ordered, bounded map: the oldest entry is evicted once `capacity` is exceeded.
pub struct RecentOutcomes<K, V> {
    entries: BTreeMap<K, V>,
    order: VecDeque<K>,
    capacity: usize,
}

impl<K: Ord + Clone, V> RecentOutcomes<K, V> {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: BTreeMap::new(),
            order: VecDeque::new(),
            capacity: capacity.max(1),
        }
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.entries.get(key)
    }

    pub fn insert(&mut self, key: K, value: V) {
        if self.entries.insert(key.clone(), value).is_none() {
            self.order.push_back(key);
        }
        while self.order.len() > self.capacity {
            if let Some(oldest) = self.order.pop_front() {
                self.entries.remove(&oldest);
            }
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests;
