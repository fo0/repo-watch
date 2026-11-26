//! What may be written: account scoping, credential redaction and rejection, identifier
//! checks and size-bounded JSON encoding.
use super::{redacted, CacheError, APP_SETTINGS_KEY, MAX_CACHE_BYTES};
use crate::models::{DispatchTarget, InputValue, Pin, Snapshot};
use crate::workflow::{looks_like_credential, sensitive_key, validate_non_secret_inputs};
use serde::Serialize;
use std::collections::BTreeSet;

/// Scopes a snapshot to one account and makes it safe to store. Pins and claims are left out
/// (they have their own tables), hidden repositories and owner filters too (the app settings
/// hold them).
/// GitHub-sourced strings that look like credentials are redacted instead of failing the
/// write: a workflow input named `tokenizer_model` or a description starting with "Bearer "
/// must not break refresh for every repository.
pub(super) fn public_snapshot(
    account_id: &str,
    snapshot: &Snapshot,
) -> Result<Snapshot, CacheError> {
    let account = snapshot
        .accounts
        .iter()
        .find(|account| account.id == account_id)
        .ok_or_else(|| CacheError::Invalid("Unknown cache account.".into()))?;
    let mut scoped = snapshot.clone();
    scoped.accounts = vec![account.clone()];
    scoped.repos.retain(|repo| repo.account_id == account_id);
    let repo_ids: BTreeSet<_> = scoped.repos.iter().map(|repo| repo.id.clone()).collect();
    scoped
        .workflows
        .retain(|workflow| repo_ids.contains(&workflow.repo_id));
    scoped.runs.retain(|run| repo_ids.contains(&run.repo_id));
    scoped.pins.clear();
    scoped.uncertain_starts.clear();
    scoped.hidden_repos.clear();
    scoped.owner_filters.clear();
    for input in scoped
        .workflows
        .iter_mut()
        .flat_map(|workflow| &mut workflow.inputs)
    {
        let secret_default = matches!(
            &input.default,
            Some(InputValue::String(value)) if looks_like_credential(value)
        );
        if sensitive_key(&input.name) || secret_default {
            input.default = None;
        }
        input
            .options
            .retain(|option| !looks_like_credential(option));
    }
    let mut value = serde_json::to_value(&scoped)?;
    redact_credential_strings(&mut value);
    Ok(serde_json::from_value(value)?)
}

fn redact_credential_strings(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::String(text) if looks_like_credential(text) => {
            *text = redacted().into();
        }
        serde_json::Value::Array(values) => values.iter_mut().for_each(redact_credential_strings),
        serde_json::Value::Object(values) => {
            values.values_mut().for_each(redact_credential_strings)
        }
        _ => {}
    }
}

/// Pins are user-written, so a credential-looking value is rejected rather than redacted.
pub(super) fn validate_pin(pin: &Pin) -> Result<(), CacheError> {
    validate_account_id(&pin.account_id)?;
    validate_identifier(&pin.id)?;
    validate_non_secret_inputs(&pin.inputs)
        .map_err(|error| CacheError::Invalid(error.to_string()))?;
    // Check all public pin strings, including a credential accidentally pasted as its label.
    reject_credential_strings(&serde_json::to_value(pin)?)
}

pub(super) fn reject_credential_strings(value: &serde_json::Value) -> Result<(), CacheError> {
    match value {
        serde_json::Value::String(value) if looks_like_credential(value) => {
            Err(CacheError::Invalid(
                "Credential-looking values cannot be stored in the local cache.".into(),
            ))
        }
        serde_json::Value::Array(values) => values.iter().try_for_each(reject_credential_strings),
        serde_json::Value::Object(values) => {
            values.values().try_for_each(reject_credential_strings)
        }
        _ => Ok(()),
    }
}

/// Returns the workflow ID as stored by SQLite.
pub(super) fn validate_target(target: &DispatchTarget) -> Result<i64, CacheError> {
    validate_account_id(&target.account_id)?;
    validate_identifier(&target.repo_id)?;
    validate_identifier(&target.git_ref)?;
    i64::try_from(target.workflow_id)
        .ok()
        .filter(|id| *id > 0)
        .ok_or_else(|| CacheError::Invalid("Invalid workflow ID.".into()))
}

pub(super) fn validate_account_id(value: &str) -> Result<(), CacheError> {
    validate_identifier(value)?;
    if value == APP_SETTINGS_KEY {
        return Err(CacheError::Invalid(
            "Invalid public cache identifier.".into(),
        ));
    }
    Ok(())
}

pub(super) fn validate_identifier(value: &str) -> Result<(), CacheError> {
    if value.trim().is_empty()
        || value.len() > 256
        || value.chars().any(char::is_control)
        || looks_like_credential(value)
    {
        return Err(CacheError::Invalid(
            "Invalid public cache identifier.".into(),
        ));
    }
    Ok(())
}

pub(super) fn encode<T: Serialize>(value: &T) -> Result<String, CacheError> {
    let payload = serde_json::to_string(value)?;
    if payload.len() > MAX_CACHE_BYTES {
        return Err(CacheError::TooLarge);
    }
    Ok(payload)
}

pub(super) fn decode<T: serde::de::DeserializeOwned>(payload: &str) -> Result<T, CacheError> {
    if payload.len() > MAX_CACHE_BYTES {
        return Err(CacheError::TooLarge);
    }
    Ok(serde_json::from_str(payload)?)
}
