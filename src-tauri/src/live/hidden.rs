//! Repositories the user hid from the panel lists (ADR-0008). They live in the app settings
//! row (`AppSettings::hidden_repos`); a hidden repository gives up its automatic place, and
//! only a favorite keeps it selected.
use super::LiveStore;
use crate::i18n::tr;
use crate::{auto_repos::auto_select, github::Transport, models::*};
use std::collections::BTreeSet;

/// Hidden repositories kept at most; far below the settings row's size limit.
pub(super) const MAX_HIDDEN_REPOS: usize = 1000;
/// Longest account or repository ID a request may name.
const MAX_ID_CHARS: usize = 200;
pub(super) fn hidden_too_many() -> &'static str {
    tr(
        "Zu viele ausgeblendete Repositories.",
        "Too many hidden repositories.",
    )
}
pub(super) fn hidden_unknown() -> &'static str {
    tr("Repository nicht gefunden.", "Repository not found.")
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty()
        && value.chars().count() <= MAX_ID_CHARS
        && !value.chars().any(char::is_control)
}

impl<T: Transport> LiveStore<T> {
    /// Hides a repository of a connected account from the panel lists, or shows it again,
    /// and re-applies the automatic selection to the known catalog at once; the next refresh
    /// loads a repository that moved up. Hiding needs a known repository; showing one that is
    /// no longer hidden changes nothing.
    pub fn set_repo_hidden(
        &mut self,
        account_id: String,
        repo_id: String,
        hidden: bool,
    ) -> Result<Snapshot, String> {
        if !valid_id(&account_id) || !valid_id(&repo_id) {
            return Err(hidden_unknown().into());
        }
        let known = self
            .hidden
            .iter()
            .position(|entry| entry.matches(&account_id, &repo_id));
        let mut next = self.hidden.clone();
        match (hidden, known) {
            (true, Some(_)) | (false, None) => return Ok(self.snapshot()),
            (true, None) => {
                let repo = self
                    .data
                    .repos
                    .iter()
                    .find(|repo| repo.id == repo_id && repo.account_id == account_id)
                    .filter(|repo| self.data.accounts.iter().any(|a| a.id == repo.account_id))
                    .ok_or(hidden_unknown())?;
                if next.len() >= MAX_HIDDEN_REPOS {
                    return Err(hidden_too_many().into());
                }
                next.push(HiddenRepo {
                    account_id,
                    repo_id: repo_id.clone(),
                    owner: repo.owner.clone(),
                    name: repo.name.clone(),
                });
            }
            (false, Some(index)) => {
                next.remove(index);
            }
        }
        self.save_hidden(next)?;
        // The repository hidden now also loses the session's „opened from the search“ hold;
        // a hidden one opened from the search later keeps its own.
        if hidden {
            self.browsed.retain(|id| id != repo_id);
        }
        self.reselect();
        Ok(self.snapshot())
    }

    /// Hidden repositories of connected accounts; an entry left by an account whose removal
    /// could not be saved never shows.
    pub(super) fn hidden_of_accounts(&self) -> Vec<HiddenRepo> {
        self.hidden
            .iter()
            .filter(|entry| self.data.accounts.iter().any(|a| a.id == entry.account_id))
            .cloned()
            .collect()
    }

    /// Drops a disconnected account's entries. Best effort: the account is gone either way,
    /// and an entry the row still holds stays out of the snapshot (`hidden_of_accounts`).
    pub(super) fn forget_hidden(&mut self, account_id: &str) {
        if !self
            .hidden
            .iter()
            .any(|entry| entry.account_id == account_id)
        {
            return;
        }
        let kept: Vec<_> = self
            .hidden
            .iter()
            .filter(|entry| entry.account_id != account_id)
            .cloned()
            .collect();
        if self.save_hidden(kept.clone()).is_err() {
            self.hidden = kept;
        }
    }

    /// Writes the list into the settings row, keeping its other preferences.
    fn save_hidden(&mut self, hidden: Vec<HiddenRepo>) -> Result<(), String> {
        let settings = self.update_app_settings(|settings| settings.hidden_repos = hidden)?;
        self.hidden = settings.hidden_repos;
        Ok(())
    }

    /// The automatic selection over the known catalog, as the next listing would make it
    /// (hidden repositories and owner filters included), keeping the repositories opened from
    /// the search. A newly selected repository loads its initial history on the next refresh;
    /// a deselected one's runs leave the snapshot, as a refresh would drop them.
    pub(super) fn reselect(&mut self) {
        let before: BTreeSet<_> = self
            .data
            .repos
            .iter()
            .filter(|repo| repo.selected)
            .map(|repo| repo.id.clone())
            .collect();
        auto_select(
            &mut self.data.repos,
            &self.data.pins,
            &self.hidden,
            &self.owner_filters,
        );
        self.keep_browsed();
        let selected: BTreeSet<_> = self
            .data
            .repos
            .iter()
            .filter(|repo| repo.selected)
            .map(|repo| repo.id.clone())
            .collect();
        for id in selected.difference(&before) {
            self.history_since.remove(id);
            self.poll_at.remove(id);
        }
        self.data.runs.retain(|run| selected.contains(&run.repo_id));
        self.persist();
    }
}
