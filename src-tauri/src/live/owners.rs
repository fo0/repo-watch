//! The repository owners (the personal account, organizations) each account watches. They live
//! in the app settings row (`AppSettings::owner_filters`) and come from the known catalog
//! (`Repo::owner`), so choosing them needs no GitHub request. A repository whose owner the
//! filter leaves out is treated like a hidden one: it gives up its automatic place, and only a
//! favorite keeps it selected.
use super::{hidden::hidden_save_failed, LiveStore};
use crate::i18n::tr;
use crate::{auto_repos::is_filtered_out, cache::AppSettings, github::Transport, models::*};

/// Owners one filter names at most; GitHub accounts rarely belong to that many organizations.
pub(super) const MAX_OWNERS: usize = 100;
/// Longest owner name a request may carry; GitHub logins are far shorter.
pub(super) const MAX_OWNER_CHARS: usize = 100;
pub(super) fn owner_account_unknown() -> &'static str {
    tr("Konto unbekannt.", "Unknown account.")
}
pub(super) fn owner_invalid() -> &'static str {
    tr("Ungültiger Besitzername.", "Invalid owner name.")
}
pub(super) fn owners_too_many() -> &'static str {
    tr("Zu viele Besitzer ausgewählt.", "Too many owners selected.")
}

/// The owners as the filter keeps them: trimmed, in the given order, each once (GitHub logins
/// compare case-insensitively, the first spelling stays). Owners need not be in the catalog:
/// the listing the panel chose from may be older than the store's.
fn normalized(owners: Vec<String>) -> Result<Vec<String>, String> {
    let mut kept: Vec<String> = vec![];
    for owner in owners {
        let owner = owner.trim();
        if owner.is_empty()
            || owner.chars().count() > MAX_OWNER_CHARS
            || owner.chars().any(char::is_control)
        {
            return Err(owner_invalid().into());
        }
        if kept
            .iter()
            .any(|known| known.to_lowercase() == owner.to_lowercase())
        {
            continue;
        }
        if kept.len() >= MAX_OWNERS {
            return Err(owners_too_many().into());
        }
        kept.push(owner.to_owned());
    }
    Ok(kept)
}

impl<T: Transport> LiveStore<T> {
    /// Sets the owners a connected account watches, replacing its earlier filter; no owners
    /// removes the filter, so the account watches all of them again. The automatic selection
    /// is re-applied to the known catalog at once; the next refresh loads a repository that
    /// moved up. A repository opened from the search stays selected unless the new filter
    /// leaves its owner out: like a hidden one, it then gives up that hold.
    pub fn set_owner_filter(
        &mut self,
        account_id: String,
        owners: Vec<String>,
    ) -> Result<Snapshot, String> {
        if !self.data.accounts.iter().any(|a| a.id == account_id) {
            return Err(owner_account_unknown().into());
        }
        let owners = normalized(owners)?;
        let mut next = self.owner_filters.clone();
        let known = next
            .iter()
            .position(|filter| filter.account_id == account_id);
        match (owners.is_empty(), known) {
            (true, None) => return Ok(self.snapshot()),
            (true, Some(index)) => {
                next.remove(index);
            }
            (false, Some(index)) => next[index].owners = owners,
            (false, None) => next.push(OwnerFilter { account_id, owners }),
        }
        if next == self.owner_filters {
            return Ok(self.snapshot());
        }
        self.save_owner_filters(next)?;
        let left_out: Vec<_> = self
            .data
            .repos
            .iter()
            .filter(|repo| is_filtered_out(&self.owner_filters, repo))
            .map(|repo| repo.id.clone())
            .collect();
        self.browsed
            .retain(|id| !left_out.iter().any(|out| out == id));
        self.reselect();
        Ok(self.snapshot())
    }

    /// Owner filters of connected accounts; a filter left by an account whose removal could
    /// not be saved never shows.
    pub(super) fn owner_filters_of_accounts(&self) -> Vec<OwnerFilter> {
        self.owner_filters
            .iter()
            .filter(|filter| self.data.accounts.iter().any(|a| a.id == filter.account_id))
            .cloned()
            .collect()
    }

    /// Drops a disconnected account's filter. Best effort: the account is gone either way,
    /// and a filter the row still holds stays out of the snapshot (`owner_filters_of_accounts`).
    pub(super) fn forget_owner_filter(&mut self, account_id: &str) {
        if !self
            .owner_filters
            .iter()
            .any(|filter| filter.account_id == account_id)
        {
            return;
        }
        let kept: Vec<_> = self
            .owner_filters
            .iter()
            .filter(|filter| filter.account_id != account_id)
            .cloned()
            .collect();
        if self.save_owner_filters(kept.clone()).is_err() {
            self.owner_filters = kept;
        }
    }

    /// Writes the filters into the settings row, keeping its other preferences.
    fn save_owner_filters(&mut self, filters: Vec<OwnerFilter>) -> Result<(), String> {
        let mut settings: AppSettings = self.cache.load_app_settings().unwrap_or_default();
        settings.owner_filters = filters;
        self.cache
            .save_app_settings(&settings)
            .map_err(|_| hidden_save_failed())?;
        self.owner_filters = settings.owner_filters;
        Ok(())
    }
}
