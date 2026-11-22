//! Account-scoped SQLite storage for public preferences and cached metadata only.
//! Credentials have no column, API, or serialization path in this module.

use crate::i18n::tr;
use crate::models::{DispatchTarget, HiddenRepo, Pin, Snapshot, UncertainStart};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::Path;
use std::time::Duration;

mod migrations;
mod payload;

use migrations::migrate;
use payload::{
    decode, encode, public_snapshot, reject_credential_strings, validate_account_id, validate_pin,
    validate_target,
};

const MAX_CACHE_BYTES: usize = 2 * 1024 * 1024;
/// Replaces GitHub-sourced strings that look like credentials in the written cache only.
pub fn redacted() -> &'static str {
    tr("[entfernt]", "[removed]")
}
/// Settings-table row for app-wide preferences; no account can use this ID.
const APP_SETTINGS_KEY: &str = "@app";

#[derive(Debug)]
pub enum CacheError {
    Database(rusqlite::Error),
    Serialization(serde_json::Error),
    Invalid(String),
    /// An entry larger than the cache's safety limit; it is neither written nor read.
    TooLarge,
}

impl fmt::Display for CacheError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Database(_) => f.write_str("The local cache could not be read or written."),
            Self::Serialization(_) => f.write_str("The local cache contains invalid data."),
            Self::Invalid(message) => f.write_str(message),
            Self::TooLarge => f.write_str("Cache entry exceeds the safety limit."),
        }
    }
}

impl std::error::Error for CacheError {}

impl From<rusqlite::Error> for CacheError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Database(error)
    }
}

impl From<serde_json::Error> for CacheError {
    fn from(error: serde_json::Error) -> Self {
        Self::Serialization(error)
    }
}

/// App-wide preferences. Unknown fields are tolerated so an older build can read a newer row.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    pub notifications: bool,
    /// Seconds between idle refreshes; absent means `refresh_interval::DEFAULT_SECONDS`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_seconds: Option<u64>,
    /// Repositories the user hid from the panel lists (ADR-0008); not written when empty.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub hidden_repos: Vec<HiddenRepo>,
    /// The log level; absent means info, and so does a value this build does not know.
    #[serde(
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::app_log::lenient_level"
    )]
    pub log_level: Option<crate::app_log::Level>,
}

impl AppSettings {
    /// The refresh interval in effect, clamped into the allowed range.
    pub fn refresh_interval(&self) -> u64 {
        crate::refresh_interval::effective(self.refresh_seconds)
    }
}

/// Result of an explicit release request for one uncertain start.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Release {
    Released,
    /// No claim exists for the target (already released or resolved).
    Missing,
    /// The target holds a claim with another claim time than the one reviewed.
    Replaced,
}

pub struct Cache {
    connection: Connection,
    /// Last written payload per account with `updatedAt` cleared, to skip unchanged writes.
    written: HashMap<String, String>,
}

impl Cache {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, CacheError> {
        Self::from_connection(Connection::open(path)?)
    }

    pub fn in_memory() -> Result<Self, CacheError> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(mut connection: Connection) -> Result<Self, CacheError> {
        connection.busy_timeout(Duration::from_secs(5))?;
        // WAL needs FULL sync to make every commit durable: with NORMAL an OS crash or power
        // loss may roll back the last commit, which can be the dispatch claim written right
        // before the POST, so the start could be sent twice.
        connection
            .pragma_update_and_check(None, "journal_mode", "WAL", |row| row.get::<_, String>(0))?;
        connection.pragma_update(None, "synchronous", "FULL")?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        migrate(&mut connection)?;
        Ok(Self {
            connection,
            written: HashMap::new(),
        })
    }

    pub fn account_ids(&self) -> Result<Vec<String>, CacheError> {
        let mut query = self
            .connection
            .prepare("SELECT account_id FROM snapshots ORDER BY account_id")?;
        let rows = query.query_map([], |row| row.get::<_, String>(0))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(CacheError::Database)
    }

    /// Written before a dispatch is sent and kept across restarts. `None` means the target
    /// already holds a claim: its last start is unresolved, so nothing may be sent.
    pub fn claim_dispatch(
        &self,
        target: &DispatchTarget,
        claimed_at: &str,
    ) -> Result<Option<UncertainStart>, CacheError> {
        let workflow_id = validate_target(target)?;
        if claimed_at.trim().is_empty()
            || claimed_at.len() > 64
            || claimed_at.chars().any(char::is_control)
        {
            return Err(CacheError::Invalid("Invalid claim time.".into()));
        }
        let inserted = self.connection.execute(
            "INSERT OR IGNORE INTO dispatch_claims
                 (account_id, repo_id, workflow_id, git_ref, claimed_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                target.account_id,
                target.repo_id,
                workflow_id,
                target.git_ref,
                claimed_at
            ],
        )? == 1;
        Ok(inserted.then(|| UncertainStart {
            account_id: target.account_id.clone(),
            repo_id: target.repo_id.clone(),
            workflow_id: target.workflow_id,
            git_ref: target.git_ref.clone(),
            claimed_at: Some(claimed_at.into()),
        }))
    }

    /// Removes a claim after GitHub gave a definite answer (accepted or rejected).
    pub fn resolve_dispatch(&self, target: &DispatchTarget) -> Result<(), CacheError> {
        let workflow_id = validate_target(target)?;
        self.connection.execute(
            "DELETE FROM dispatch_claims
             WHERE account_id=?1 AND repo_id=?2 AND workflow_id=?3 AND git_ref=?4",
            params![
                target.account_id,
                target.repo_id,
                workflow_id,
                target.git_ref
            ],
        )?;
        Ok(())
    }

    pub fn uncertain_starts(&self) -> Result<Vec<UncertainStart>, CacheError> {
        let mut query = self.connection.prepare(
            "SELECT account_id, repo_id, workflow_id, git_ref, claimed_at FROM dispatch_claims
             ORDER BY claimed_at, account_id, repo_id, workflow_id, git_ref",
        )?;
        let rows = query.query_map([], |row| {
            Ok(UncertainStart {
                account_id: row.get(0)?,
                repo_id: row.get(1)?,
                workflow_id: row.get::<_, i64>(2)?.max(0) as u64,
                git_ref: row.get(3)?,
                claimed_at: row.get(4)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(CacheError::Database)
    }

    /// Releases exactly the claim the user reviewed on GitHub. Only an explicit user command
    /// may call this; there is no automatic release.
    pub fn release_uncertain_start(&self, start: &UncertainStart) -> Result<Release, CacheError> {
        let target = start.target();
        let workflow_id = validate_target(&target)?;
        let released = self.connection.execute(
            "DELETE FROM dispatch_claims
             WHERE account_id=?1 AND repo_id=?2 AND workflow_id=?3 AND git_ref=?4
               AND claimed_at IS ?5",
            params![
                target.account_id,
                target.repo_id,
                workflow_id,
                target.git_ref,
                start.claimed_at
            ],
        )?;
        if released == 1 {
            return Ok(Release::Released);
        }
        let other: Option<i64> = self
            .connection
            .query_row(
                "SELECT 1 FROM dispatch_claims
                 WHERE account_id=?1 AND repo_id=?2 AND workflow_id=?3 AND git_ref=?4",
                params![
                    target.account_id,
                    target.repo_id,
                    workflow_id,
                    target.git_ref
                ],
                |row| row.get(0),
            )
            .optional()?;
        Ok(if other.is_some() {
            Release::Replaced
        } else {
            Release::Missing
        })
    }

    /// Stores GitHub metadata for one account. Pins and claims live in their own tables and
    /// are never written or deleted here. An unchanged payload (ignoring `updatedAt`) is not
    /// rewritten, so a cached `updatedAt` may be older than the last successful refresh.
    pub fn save_snapshot(
        &mut self,
        account_id: &str,
        snapshot: &Snapshot,
    ) -> Result<(), CacheError> {
        validate_account_id(account_id)?;
        let mut scoped = public_snapshot(account_id, snapshot)?;
        // Defense in depth: redaction must have left no credential-looking string behind.
        reject_credential_strings(&serde_json::to_value(&scoped)?)?;
        let updated_at = std::mem::take(&mut scoped.updated_at);
        let comparable = encode(&scoped)?;
        if self.written.get(account_id) == Some(&comparable) {
            return Ok(());
        }
        scoped.updated_at = updated_at;
        self.connection.execute(
            "INSERT INTO snapshots (account_id, payload) VALUES (?1, ?2)
             ON CONFLICT(account_id) DO UPDATE SET payload=excluded.payload",
            params![account_id, encode(&scoped)?],
        )?;
        self.written.insert(account_id.into(), comparable);
        Ok(())
    }

    pub fn load_snapshot(&self, account_id: &str) -> Result<Option<Snapshot>, CacheError> {
        validate_account_id(account_id)?;
        let payload: Option<String> = self
            .connection
            .query_row(
                "SELECT payload FROM snapshots WHERE account_id=?1",
                [account_id],
                |row| row.get(0),
            )
            .optional()?;
        let Some(payload) = payload else {
            return Ok(None);
        };
        let original: Snapshot = decode(&payload)?;
        let mut snapshot = public_snapshot(account_id, &original)?;
        snapshot.pins = self.load_pins(account_id)?;
        snapshot.stale = true;
        Ok(Some(snapshot))
    }

    pub fn save_pin(&self, account_id: &str, pin: &Pin) -> Result<(), CacheError> {
        validate_account_id(account_id)?;
        if pin.account_id != account_id {
            return Err(CacheError::Invalid(
                "Pin belongs to another account.".into(),
            ));
        }
        validate_pin(pin)?;
        self.connection.execute(
            "INSERT INTO pins (account_id, pin_id, payload) VALUES (?1, ?2, ?3)
             ON CONFLICT(account_id, pin_id) DO UPDATE SET payload=excluded.payload",
            params![account_id, pin.id, encode(pin)?],
        )?;
        Ok(())
    }

    pub fn load_pins(&self, account_id: &str) -> Result<Vec<Pin>, CacheError> {
        validate_account_id(account_id)?;
        let mut statement = self
            .connection
            .prepare("SELECT payload FROM pins WHERE account_id=?1 ORDER BY pin_id")?;
        let rows = statement.query_map([account_id], |row| row.get::<_, String>(0))?;
        let mut pins = vec![];
        for row in rows {
            let pin: Pin = decode(&row?)?;
            if pin.account_id != account_id {
                return Err(CacheError::Invalid("Cached pin account mismatch.".into()));
            }
            validate_pin(&pin)?;
            pins.push(pin);
        }
        Ok(pins)
    }

    pub fn delete_pin(&self, account_id: &str, pin_id: &str) -> Result<(), CacheError> {
        validate_account_id(account_id)?;
        self.connection.execute(
            "DELETE FROM pins WHERE account_id=?1 AND pin_id=?2",
            params![account_id, pin_id],
        )?;
        Ok(())
    }

    pub fn save_app_settings(&self, settings: &AppSettings) -> Result<(), CacheError> {
        self.write_settings_row(APP_SETTINGS_KEY, &encode(settings)?)
    }

    pub fn load_app_settings(&self) -> Result<AppSettings, CacheError> {
        match self.read_settings_row(APP_SETTINGS_KEY)? {
            Some(payload) => decode(&payload),
            None => Ok(AppSettings::default()),
        }
    }

    fn write_settings_row(&self, key: &str, payload: &str) -> Result<(), CacheError> {
        self.connection.execute(
            "INSERT INTO settings (account_id, payload) VALUES (?1, ?2)
             ON CONFLICT(account_id) DO UPDATE SET payload=excluded.payload",
            params![key, payload],
        )?;
        Ok(())
    }

    fn read_settings_row(&self, key: &str) -> Result<Option<String>, CacheError> {
        Ok(self
            .connection
            .query_row(
                "SELECT payload FROM settings WHERE account_id=?1",
                [key],
                |row| row.get(0),
            )
            .optional()?)
    }

    /// Removes only an account's snapshot row, for a connect whose credential was never
    /// stored. Pins, settings and claims stay: a claim is released only by an explicit user
    /// command.
    pub fn delete_snapshot(&mut self, account_id: &str) -> Result<(), CacheError> {
        validate_account_id(account_id)?;
        self.connection
            .execute("DELETE FROM snapshots WHERE account_id=?1", [account_id])?;
        self.written.remove(account_id);
        Ok(())
    }

    /// Removes everything stored for an account, including its uncertain-start claims.
    pub fn delete_account(&mut self, account_id: &str) -> Result<(), CacheError> {
        validate_account_id(account_id)?;
        let transaction = self.connection.transaction()?;
        for sql in [
            "DELETE FROM snapshots WHERE account_id=?1",
            "DELETE FROM pins WHERE account_id=?1",
            "DELETE FROM settings WHERE account_id=?1",
            "DELETE FROM dispatch_claims WHERE account_id=?1",
        ] {
            transaction.execute(sql, [account_id])?;
        }
        transaction.commit()?;
        self.written.remove(account_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
