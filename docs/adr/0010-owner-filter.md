# ADR-0010: Owner filter

- **Status:** Accepted
- **Date:** 2026-11-26
- **Decider(s):** owner
- **Tags:** ui, repositories, catalog, settings
- **Supersedes:** ADR-0003, part „per account“ — the owners whose repositories the automatic rule ranks are now a user choice; the rule itself stays automatic

## Context

ADR-0003 ranks every repository an account reaches, its own and those of every organization it belongs to, so an account in several organizations fills its 10 places and the run lists with organizations the user does not follow. The owner asked that the organizations reachable by a connected account become selectable, so that e.g. only organization 1 and 2 are watched instead of everything the account reaches; an account without an organization gets no choice (#108).

## Decision

We will let the user choose per account which owners — the personal account and its organizations — are watched. The owners come from the account's existing repository listing (`Repo.owner`): no `/user/orgs` request, no new token permission. Settings → App „Organisationen“, above „Ausgeblendete Repositories“, shows one row per account that has more than one owner, with one toggle per owner, all watched by default and at least one kept on, staged until „Speichern“ (`agent_docs/ux_rules.md`, rule 5). The choice is stored natively in the app settings row (`AppSettings.ownerFilters`: `{ accountId, owners[] }`, no entry = all owners), reaches the panel as `Snapshot.ownerFilters` and is changed by the command `set_owner_filter { accountId, owners }` (empty = filter removed), which re-applies the selection to the cached catalog at once. A repository whose owner is outside the filter behaves like a hidden one (ADR-0008): it takes none of the 10 newest places, so the next one moves up, and leaves the Repositories section and page, „Läuft gerade“, the Verlauf and their counts; a favorite keeps it selected and polled, and the search still finds it (ADR-0006). Owner names compare case-insensitively. Disconnecting an account drops its filter. Repositories stay automatic: the filter narrows the scope the rule ranks, and there is still no picker.

## Consequences

### Positive

- One choice per organization instead of one per repository; new repositories of an unwatched organization stay out.
- No extra request or permission: the listing the rule already reads names the owners.

### Negative / Trade-offs

- An owner is offered only once the listing contains one of its repositories; a listing cut at the page limit (BACKLOG #10) can miss one.
- A filtered repository with a favorite is still polled, as with hiding.
- Owners are stored by name, so a renamed owner no longer matches its entry.

### Neutral

- The browser demo keeps its copy in `localStorage`.
- No SQLite migration: the settings row is JSON, and a row without the field filters nothing.
- An account with a single owner shows no row.

## Alternatives Considered

- **`/user/orgs`** — rejected: it needs `read:org` and an extra request, and lists organizations without reachable repositories; a fine-grained PAT reaches one resource owner anyway.
- **A repository picker** — rejected: ADR-0003 and rule 4 stand; the user narrows by owner, the rule still picks the repositories.
- **Hiding repositories one by one** (ADR-0008) — rejected: it does not scale to an organization, and new repositories of it reappear.

## References

- #108 (owner request), `agent_docs/ux_rules.md`
- [ADR-0003](0003-automatic-repositories.md), [ADR-0006](0006-search-reaches-every-repository.md), [ADR-0008](0008-hidden-repositories.md)
- `src-tauri/src/auto_repos.rs`, `src-tauri/src/live/owners.rs`, `src-tauri/src/cache.rs` (`AppSettings`), `src/lib/ownerFilter.ts`, `src/lib/ownerChoice.svelte.ts`, `src/lib/OwnerFilterSetting.svelte`, `src/lib/autoRepos.ts`
- `src-tauri/src/live/tests/owners.rs`, `src/lib/ownerFilter.test.ts`, `tests/owner-filter.spec.ts`
