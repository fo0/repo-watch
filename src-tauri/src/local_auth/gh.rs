//! The gh CLI adapter: auth status in its JSON and legacy forms, and the help probes that
//! decide which flags a gh version supports.
use super::text::{self, failed};
use super::{LocalAccount, Runner};
use crate::workflow::looks_like_credential;
use serde::Deserialize;
use std::path::Path;

// New gh versions can rotate expiring credentials; suppress that whenever supported.
// Older versions lack the flag and read stored tokens without automatic refresh.
pub(super) fn gh_help(
    program: &Path,
    command: &str,
    runner: &dyn Runner,
    current: &dyn Fn() -> bool,
) -> Result<String, String> {
    let output = runner.run(program, &["auth", command, "--help"], current)?;
    if !output.success {
        return Err(failed().into());
    }
    std::str::from_utf8(&output.bytes)
        .map(str::to_owned)
        .map_err(|_| failed().into())
}

pub(super) fn gh_status(
    program: &Path,
    runner: &dyn Runner,
    current: &dyn Fn() -> bool,
) -> Result<(Vec<LocalAccount>, bool), String> {
    let help = gh_help(program, "status", runner, current)?;
    if help.contains("--json") {
        let mut args = vec![
            "auth",
            "status",
            "--hostname",
            "github.com",
            "--json",
            "hosts",
        ];
        if help.contains("--no-refresh") {
            args.push("--no-refresh");
        }
        let output = runner.run(program, &args, current)?;
        if !output.success {
            return Err(failed().into());
        }
        Ok((gh_accounts(&output.bytes)?, false))
    } else {
        if help.contains("--no-refresh") {
            return Err(text::gh_status_incompatible().into());
        }
        // Legacy gh lacks structured auth status. Ask its read-only /user endpoint for
        // the active identity instead of parsing human output or reading credential files.
        let output = runner.run(
            program,
            &["api", "--hostname", "github.com", "user", "--jq", ".login"],
            current,
        )?;
        if !output.success {
            return Err(failed().into());
        }
        let login = std::str::from_utf8(&output.bytes)
            .map_err(|_| failed())?
            .trim();
        if !valid_login(login) {
            return Err(failed().into());
        }
        Ok((
            vec![LocalAccount {
                source: "gh".into(),
                host: "github.com".into(),
                login: login.into(),
                usable: true,
                active: true,
                detail: text::gh_active_confirmed().into(),
                actions_read: text::scopes_unreadable().into(),
                actions_write: text::start_unchecked().into(),
            }],
            true,
        ))
    }
}

pub(super) fn valid_login(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 39
        && !value.starts_with('-')
        && !value.ends_with('-')
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        && !looks_like_credential(value)
}
#[derive(Deserialize)]
struct GhStatus {
    hosts: std::collections::BTreeMap<String, Vec<GhAccount>>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GhAccount {
    host: String,
    login: String,
    state: String,
    #[serde(default)]
    active: bool,
    #[serde(default)]
    scopes: String,
    #[serde(default)]
    token_source: String,
}
pub(super) fn gh_accounts(bytes: &[u8]) -> Result<Vec<LocalAccount>, String> {
    let status: GhStatus = serde_json::from_slice(bytes).map_err(|_| failed())?;
    let mut accounts = Vec::new();
    for account in status
        .hosts
        .get("github.com")
        .into_iter()
        .flatten()
        .take(50)
    {
        if account.host != "github.com" || !valid_login(&account.login) {
            continue;
        }
        let usable = account.state == "success"
            && !account
                .token_source
                .to_ascii_uppercase()
                .ends_with("_TOKEN");
        let repo = account
            .scopes
            .split([',', ' ', '\''])
            .any(|scope| scope == "repo");
        accounts.push(LocalAccount {
            source: "gh".into(),
            host: account.host.clone(),
            login: account.login.clone(),
            active: account.active,
            usable,
            detail: if usable {
                text::api_sign_in_confirmed()
            } else {
                text::sign_in_invalid()
            }
            .into(),
            actions_read: if repo {
                text::scope_repo_present()
            } else if account.scopes.is_empty() {
                text::scopes_unreadable()
            } else {
                text::scope_repo_missing()
            }
            .into(),
            actions_write: text::start_unchecked().into(),
        });
    }
    accounts.sort_by(|a, b| a.login.cmp(&b.login));
    accounts.dedup_by(|a, b| a.login == b.login);
    Ok(accounts)
}
