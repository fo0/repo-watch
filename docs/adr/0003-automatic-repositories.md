# ADR-0003: Automatic repositories

- **Status:** Accepted; partly superseded — the search that filters only the automatic set by [ADR-0006](0006-search-reaches-every-repository.md), the removal of a repository by the user (now allowed: hiding) by [ADR-0008](0008-hidden-repositories.md), the scope of every owner the account reaches (now a user choice: the owner filter) by [ADR-0010](0010-owner-filter.md)
- **Date:** 2026-11-01
- **Decider(s):** owner
- **Tags:** ui, repositories, catalog
- **Supersedes:** ADR-0002, part „Inline account and repository management“ — its repository half; the account menu stays in force

## Context

ADR-0002 made the tray's repositories a manual choice in the panel: the page „Repositories im Tray“ (opened by „+ Hinzufügen“, the empty state „Repositories auswählen“ and a fresh connection), „Nicht im Tray“ while searching, „Aus Tray entfernen“ in each row's „⋯“ menu, instant toggles with „Rückgängig“, a cap of 50 sent through a native selection command. The owner asked three times to remove that manual step: the app can tell which repositories matter, so asking the user is configuration it can infer (`agent_docs/ux_rules.md`, rule 4).

## Decision

We will derive the tray's repositories automatically, per account: the 10 repositories with the newest `pushed_at` (a missing push time last, ties by owner/name), plus every repository of a favorite of that account _(a repository the user hid gives up its place, ADR-0008)_ _(the owners whose repositories it ranks: superseded by ADR-0010)_. The set is recomputed whenever an account's repository listing arrives; an account without a fresh listing keeps its last set. The overview shows up to 10 repositories; the maximized Repositories page shows the whole set, and the search filters it _(superseded by ADR-0006)_. Removed: the native selection command, the picker page „Repositories im Tray“, „+ Hinzufügen“, „Nicht im Tray“, „Aus Tray entfernen“, the 50-repository cap and the empty state „Repositories auswählen“. The account menu, account scoping (natively the active account in the foreground, the other accounts observed in the background) and the remaining „⋯“ actions are unchanged.

## Consequences

### Positive

- No setup step: a fresh connection shows its repositories at once.
- The set follows the user's work without maintenance.
- Polling is bounded by 10 repositories per account plus the favorites' repositories.

### Negative / Trade-offs

- A rarely pushed repository drops out of the tray unless it has a favorite; a favorite is the way to keep it.
- Activity means the latest push, not the latest workflow run, so a busy repository without dispatchable workflows still takes a slot; so does one whose Actions access is denied.
- `/user/repos` is read without `sort=pushed`: a listing cut at the page limit ranks only the pages that arrived plus the previously selected repositories (BACKLOG #10).

### Neutral

- A manual selection stored by an earlier version no longer decides anything.
- Browser (`github-device-*`) accounts keep „Repository-Freigabe auf GitHub ändern“ and „Repository-Liste aktualisieren“ at the bottom of the Repositories page.
- The browser demo applies the same rule (`src/lib/autoRepos.ts`): `demo-work` shows 10 of its 12 repositories.

## Alternatives Considered

- **Keep the picker as an optional override** — rejected: the owner asked for no manual repository selection anywhere _(removing by hiding: superseded by ADR-0008)_.
- **Rank by the latest workflow run** — not chosen: the owner named the latest push, which comes with the repository listing (`pushed_at`); run times need requests per repository before the set is known.

## References

- #18 (owner feedback), `agent_docs/ux_rules.md`
- [ADR-0002](0002-single-overview-with-maximizable-sections.md)
- `src-tauri/src/auto_repos.rs`, `src-tauri/src/live/catalog.rs` (`merge_catalog`), `src/lib/autoRepos.ts`, `src/lib/ReposPage.svelte`
- `src-tauri/src/live/tests/auto_select.rs`, `src/lib/autoRepos.test.ts`, `tests/overview.spec.ts`, `tests/overview-caps.spec.ts`, `tests/repository-access.spec.ts`
