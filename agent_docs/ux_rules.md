# Owner UX rules (binding)

Set by the owner (#18). They override every older UI statement in `README.md`, `docs/`, `docs/adr/` and `agent_docs/`. Read them before any UI change; the UI review checks each one (`agent_docs/review_process.md → UI Review`).

1. **Compact first.** Single-line rows over cards. Smallest readable size. No decorative headings, badges or step lists.
2. **Minimal text.** Labels, not sentences. At most one short help line per form. No explanatory paragraphs. Details go into tooltips. Never describe what the UI obviously does.
3. **Identify items by what differs.** The repository name is primary, the workflow or favorite name secondary — every workflow is called „Release“.
4. **No manual configuration the app can infer.** Repositories come automatically: the 10 most recently active (latest push) per account plus the repositories of favorites. No picker and no add ([ADR-0003](../docs/adr/0003-automatic-repositories.md)). Two manual exceptions, both narrowing: „Ausblenden“ in a row's „⋯“ menu hides a noisy repository, Settings → App „Ausgeblendete Repositories“ brings it back ([ADR-0008](../docs/adr/0008-hidden-repositories.md)); Settings → App „Organisationen“ limits an account with several owners to the chosen ones ([ADR-0010](../docs/adr/0010-owner-filter.md)). The search still finds every repository of the account, hidden ones included; expanding one loads it for the session ([ADR-0006](../docs/adr/0006-search-reaches-every-repository.md)).
5. **Settings are staged.** Changes apply with one primary „Speichern“ next to „Abbrechen“. Immediate actions are explicit buttons.
6. **One primary action per form or flow, bottom right.** A completed flow closes or returns by itself — the user never has to press „Schließen“ after success.
7. **Feedback goes away.** Success notices close after 3 s (an undo offer stays 8 s). Only warnings and errors persist. Nothing may get stuck on screen.
8. **Toggles stay in place.** Maximize turns into a back arrow at the same spot; no separate back button ([ADR-0005](../docs/adr/0005-maximize-toggles-in-place.md), [ADR-0007](../docs/adr/0007-back-arrow-and-home-brand.md)). The brand at the top left leads to the overview.
9. **Updates install in the app.** „Nach Updates suchen“ → „Update verfügbar“ with at most 5 changelog bullets → „Installieren“ → progress bar → the app installs and restarts itself. Never send the user to a web page to update ([ADR-0004](../docs/adr/0004-in-app-updates-for-unsigned-builds.md)).
10. **Honest evidence.** Screenshots or previews shown to the owner come from the exact build being delivered. Never present a mock-up as the result.
11. **Owner requests beat older docs.** When a doc or ADR contradicts an owner request, supersede the doc or ADR in the same change instead of following it.

**Accessibility adds no visible prose.** Accessible names and tooltips may carry detail; visible text never grows to satisfy accessibility. An icon button gets an accessible name, not a caption.
