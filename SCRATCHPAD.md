# Scratchpad — Short-Term

Temporary working context. **Clean up aggressively — delete when resolved.**

## Current Work

**Orca objective** — per-account owner filter: the user picks which repository owners (personal account, organizations) of an account are watched, #108 (2026-11-26)

- Out of scope: `/user/orgs` or any new GitHub request, a repository picker, tray/notification changes
- Contract: `OwnerFilter { accountId, owners }`, `Snapshot.ownerFilters`, `AppSettings.ownerFilters`, command `set_owner_filter { accountId, owners }` (empty = all); a repository outside the filter behaves like a hidden one
- Packages: Rust core · panel + demo + e2e · docs (ADR-0010); then full chain, reviewer, PR, merge

## Open Questions

_(none)_
