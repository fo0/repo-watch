//! Adding and removing accounts: a token, a gh CLI import or a browser sign-in.
use super::{
    account_save_failed, empty_snapshot, now, number, text, url, with_notice, HttpTransport,
    LiveStore,
};
use crate::{
    auth::{NativeKeyring, SecretToken},
    browser_auth::{self, BrowserCredential, BrowserTokens, NativeOAuthTransport, OAuthTransport},
    github::{ApiClient, ApiRequest, Transport},
    i18n::tr,
    models::*,
    trf,
};
use serde_json::Value;
use std::collections::BTreeSet;

fn sign_in_cancelled() -> String {
    tr("Anmeldung wurde abgebrochen.", "Sign-in was cancelled.").into()
}

impl LiveStore<HttpTransport> {
    pub fn connect_token(
        &mut self,
        token: SecretToken,
        still_current: impl Fn() -> bool,
    ) -> Result<Snapshot, String> {
        self.connect_credential(token, None, still_current)
    }
    pub fn connect_local_token(
        &mut self,
        token: SecretToken,
        expected_login: &str,
        still_current: impl Fn() -> bool,
    ) -> Result<Snapshot, String> {
        self.connect_credential(token, Some(expected_login), still_current)
    }
    fn connect_credential(
        &mut self,
        token: SecretToken,
        expected_login: Option<&str>,
        still_current: impl Fn() -> bool,
    ) -> Result<Snapshot, String> {
        if !still_current() {
            return Err(sign_in_cancelled());
        }
        let mut transport = HttpTransport::new(self.keyring.clone())?;
        token.with_secret(|v| {
            if v.is_empty()
                || v.len() > 2048
                || v.bytes()
                    .any(|b| b.is_ascii_control() || b.is_ascii_whitespace())
            {
                Err(tr(
                    "Token ist ungültig. Füge den Token ohne Leerzeichen oder Zeilenumbrüche ein.",
                    "Token is invalid. Paste the token without spaces or line breaks.",
                ))
            } else {
                Ok(())
            }
        })?;
        transport.bootstrap = Some(token);
        let client = ApiClient::new(transport);
        let request = ApiRequest::get("pending", url(&["user"], &[])?)
            .map_err(|_| tr("Anfrage ungültig.", "Invalid request."))?;
        let response = client.execute(&request, now()).map_err(|_| {
            tr(
                "GitHub nicht erreichbar. Token wurde nicht gespeichert – prüfe deine Verbindung und versuche es erneut.",
                "GitHub unreachable. Token was not saved – check your connection and try again.",
            )
        })?;
        if response.status != 200 {
            return Err(tr(
                "Token wurde von GitHub nicht bestätigt. Prüfe Gültigkeit und Organisationsfreigabe.",
                "GitHub did not confirm the token. Check its validity and organization approval.",
            )
            .into());
        }
        let user: Value = serde_json::from_slice(&response.body)
            .map_err(|_| tr("GitHub-Konto ungültig.", "Invalid GitHub account."))?;
        let login = text(&user, "login")?.to_string();
        verify_local_identity(expected_login, &login)?;
        // Separate app-owned imports from PAT entries. Disconnect only deletes our copy;
        // it never changes, deletes or revokes the gh CLI's credential or active account.
        let id = credential_account_id(number(&user, "id")?, expected_login.is_some());
        // Consume the verified temporary token without serialization or UI round-trip.
        let mut guard = client.into_transport().map_err(|_| {
            tr(
                "Anmeldung konnte nicht abgeschlossen werden.",
                "Sign-in could not be completed.",
            )
        })?;
        let token = guard
            .bootstrap
            .take()
            .ok_or(tr("Token fehlt.", "Token is missing."))?;
        if !still_current() {
            return Err(sign_in_cancelled());
        }
        let account = Account {
            id: id.clone(),
            login,
            label: if expected_login.is_some() {
                tr("GitHub · gh CLI (Kopie)", "GitHub · gh CLI (copy)")
            } else {
                "GitHub · Token"
            }
            .into(),
        };
        self.connect_account(account, |keyring| {
            browser_auth::commit_credential(keyring, &id, &token, &still_current)
        })
    }
    pub fn connect_browser(
        &mut self,
        tokens: BrowserTokens,
        still_current: impl Fn() -> bool,
    ) -> Result<Snapshot, String> {
        if !still_current() {
            return Err(sign_in_cancelled());
        }
        let (user_id, login) = NativeOAuthTransport::new()?.identity(&tokens.access)?;
        let id = browser_auth::account_id(user_id);
        let credential = BrowserCredential {
            tokens,
            client_id: browser_auth::publisher()?.0.into(),
            user_id,
            rotation_pending: false,
        };
        if !still_current() {
            return Err(sign_in_cancelled());
        }
        let encoded = credential.encode()?;
        let account = Account {
            id: id.clone(),
            login,
            label: "GitHub · Browser".into(),
        };
        self.connect_account(account, |keyring| {
            browser_auth::commit_credential(keyring, &id, &encoded, &still_current)
        })
    }
}
impl<T: Transport> LiveStore<T> {
    /// Adds a verified account. Its cache row is written before `commit` stores the
    /// credential, so a connected account always survives a restart and stays removable from
    /// the panel even when a later full snapshot write fails. A failed row write leaves the
    /// vault untouched; a failed commit never leaves a vault entry that no account can remove.
    pub(super) fn connect_account(
        &mut self,
        account: Account,
        commit: impl FnOnce(&dyn NativeKeyring) -> Result<(), String>,
    ) -> Result<Snapshot, String> {
        let id = account.id.clone();
        let stored = self
            .cache
            .account_ids()
            .map_err(|_| account_save_failed())?
            .contains(&id);
        if !stored {
            let mut minimal = empty_snapshot();
            minimal.accounts = vec![account.clone()];
            minimal.message = None;
            self.cache
                .save_snapshot(&id, &minimal)
                .map_err(|_| account_save_failed())?;
        }
        if let Err(error) = commit(self.keyring.as_ref()) {
            return Err(if stored {
                error
            } else {
                self.abandon_new_account(account, error)
            });
        }
        self.schedule.forget_account(&id, &self.data.repos);
        self.data.accounts.retain(|a| a.id != id);
        self.data.accounts.push(account);
        self.refresh();
        Ok(self.snapshot())
    }
    /// Cleans up after a new account whose credential commit failed. Its row goes (best
    /// effort; never pins, settings or claims) unless the credential may still be in the vault,
    /// as after a cancel whose rollback failed, or when the vault cannot even be read: then the
    /// entry is deleted once more, and if that fails too the row and the account stay, so the
    /// panel can disconnect it. A row left behind by a failed delete shows an account that asks
    /// to be reconnected or disconnected.
    fn abandon_new_account(&mut self, account: Account, error: String) -> String {
        let read = self.keyring.read_token(&account.id);
        let present = !matches!(read, Ok(None));
        let stranded = present
            && browser_auth::disconnect_credential(self.keyring.as_ref(), &account.id).is_err();
        if !stranded {
            let _ = self.cache.delete_snapshot(&account.id);
            return error;
        }
        let maybe = if read.is_ok() {
            ""
        } else {
            tr("möglicherweise ", "possibly ")
        };
        let notice = trf!(
            "Der Zugang von {} liegt {maybe}noch im Tresor – trenne das Konto, um ihn zu entfernen.",
            "The access of {} is {maybe}still in the vault – disconnect the account to remove it.",
            account.login
        );
        self.data.accounts.retain(|known| known.id != account.id);
        self.data.accounts.push(account);
        self.data.message = Some(with_notice(self.data.message.take(), &notice));
        with_notice(Some(error), &notice)
    }
    pub fn disconnect(&mut self, account: &str) -> Result<Snapshot, String> {
        if !self.data.accounts.iter().any(|a| a.id == account) {
            return Err(tr("Konto unbekannt.", "Unknown account.").into());
        }
        browser_auth::disconnect_credential(self.keyring.as_ref(), account)?;
        self.cache.delete_account(account).map_err(|_| {
            tr(
                "Kontometadaten konnten nicht entfernt werden.",
                "Account metadata could not be removed.",
            )
        })?;
        self.schedule.forget_account(account, &self.data.repos);
        self.claims.retain(|claim| claim.account_id != account);
        self.forget_hidden(account);
        self.forget_owner_filter(account);
        self.data.accounts.retain(|a| a.id != account);
        self.data.repos.retain(|r| r.account_id != account);
        self.data.pins.retain(|p| p.account_id != account);
        let ids: BTreeSet<_> = self.data.repos.iter().map(|r| r.id.clone()).collect();
        self.data.runs.retain(|r| ids.contains(&r.repo_id));
        self.data.workflows.retain(|w| ids.contains(&w.repo_id));
        self.poll_at.retain(|id, _| ids.contains(id));
        self.history_since.retain(|id, _| ids.contains(id));
        self.browsed.retain(|id| ids.contains(id));
        self.tracked
            .retain(|_, tracker| tracker.account_id != account);
        Ok(self.snapshot())
    }
}
pub(super) fn credential_account_id(id: u64, local_import: bool) -> String {
    format!("github-{}{id}", if local_import { "gh-" } else { "" })
}

pub(super) fn verify_local_identity(expected: Option<&str>, actual: &str) -> Result<(), String> {
    if expected.is_some_and(|login| !login.eq_ignore_ascii_case(actual)) {
        return Err(tr(
            "GitHub hat ein anderes Konto gemeldet. Prüfe die lokale Anmeldung erneut.",
            "GitHub reported a different account. Check the local sign-in again.",
        )
        .into());
    }
    Ok(())
}
