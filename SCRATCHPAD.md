# Scratchpad — Short-Term

Temporary working context. **Clean up aggressively — delete when resolved.**

## Current Work

**Orca objective** — per-account owner filter: the user picks which repository owners (personal account, organizations) of an account are watched, #108 (2026-11-26)

- Out of scope: `/user/orgs` or any new GitHub request, a repository picker, tray/notification changes
- Contract: `OwnerFilter { accountId, owners }`, `Snapshot.ownerFilters`, `AppSettings.ownerFilters`, command `set_owner_filter { accountId, owners }` (empty = all); a repository outside the filter behaves like a hidden one
- State: built, reviewed, full chain green, 4 local commits; **push refused (403 `push_denied`)** — open: push, draft PR (`Closes #108`), CI incl. Windows compile of the `desktop.rs` command, merge (owner ordered it)

## Open Questions

_(none)_
