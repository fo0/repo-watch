# UX checks and reproduction

Binding rules: [owner UX rules](../agent_docs/ux_rules.md). This page lists the panel's target behaviour as short facts, the automated coverage and the screenshot scripts.

## Panel

**Frame**

- 360–480 px wide. App bar: brand (icon only below 400 px), account menu, refresh, settings. Footer: connection status and data time.
- One overview, no tabs ([ADR-0002](adr/0002-single-overview-with-maximizable-sections.md)): **Läuft gerade**, **Favoriten**, **Repositories** — each a region named by its heading, with a count and a maximize toggle (**Strg+1/2/3** in its tooltip).
- Overview caps: 3 runs, 4 favorites, 10 repositories. **+N weitere** opens the maximized page; a search lifts the caps and filters every section.
- Maximized: the section header stays; its maximize button turns into a back arrow at the same spot (**Zurück zur Übersicht**, tooltip **Zurück · Esc**), there is no second back button ([ADR-0005](adr/0005-maximize-toggles-in-place.md), [ADR-0007](adr/0007-back-arrow-and-home-brand.md)); **Esc**, the section's Ctrl key and the brand **RepoWatch** (tooltip **Zur Übersicht**) also return. Opening focuses the page title; returning focuses the opener, else the section's toggle. Disclosures stay open.
- Empty overview: only the next step — **GitHub-Konto verbinden** or **Konto wählen** (opens the account menu).

**Läuft gerade** — running, queued and waiting runs as single-line rows: status icon, repository in bold (owner in the tooltip), **· workflow #run ID**, branch, elapsed time, and with write access a stop button (**Run abbrechen** → **Abbrechen?** for 4 s → cancel; then a spinner, **wird abgebrochen**, until GitHub reports the run closed); **Gerade läuft nichts.** when idle. A click on the row opens the run in the **Verlauf**, whose open-run details offer **Run abbrechen** the same way. No live region: the demo refreshes every second.

**Favoriten** — one row list for the overview and the Favoriten page (`FavoriteTiles`), 34 px single-line rows: repository name first, the favorite's name small beside it, branch, settings, heart, start (far right). The start sends the stored request at once, once per press: a held key, a double-click or a repeat of the same favorite within 1.5 s after it settled sends nothing more (another favorite of the same workflow and ref still starts); when an input is missing or the ref is gone it opens the start dialog with the favorite section open; an uncertain lock, offline or a running start disables it (reason in the tooltip). The settings button (**Standardwerte bearbeiten**) opens the dialog to update ref, inputs and name. The account appears only in the tooltip (repository · name · branch · account). Long names shorten. The name opens the stored configuration; the heart removes, and **Rückgängig** (8 s) restores label and inputs; favorites persist per account.

**Repositories** — automatic ([ADR-0003](adr/0003-automatic-repositories.md)): per account the 10 newest by `pushed_at` (missing push time last, ties by owner/name) plus the favorites' repositories, recomputed when that account's listing arrives; no picker, no add. „Ausblenden“ in the „⋯“ menu hides one (8 s **Rückgängig**), Settings → App **Ausgeblendete Repositories** shows it again on **Speichern** ([ADR-0008](adr/0008-hidden-repositories.md)). Natively the active account's repositories are shown. Demo: `demo-work` has 12 repositories, 10 of them shown; with `demo-personal` the all-accounts overview counts 12 and shows 10 with **+2 weitere**. While a browser (`github-device-*`) account is connected, the Repositories page ends with **Repository-Freigabe auf GitHub ändern** and **Repository-Liste aktualisieren**.

- One row each, sorted by latest activity (newest of latest run, last run start and last push; re-sorted on every refresh): `owner/` dimmed, workflow count, status icon and text (**2 aktiv** while several runs are open), **Nur Lesen** or **Kein Zugriff** where starts are not allowed, a strip of the last 8 runs (newest right, empty cells when fewer, a running cell pulses unless reduced motion). The strip is an image with a summarizing accessible name.
- A disclosure starts with one quiet line: default branch, **Push vor …** (exact time in the tooltip) and **GitHub** at the right end (accessible name „owner/name auf GitHub öffnen“; disabled in the demo); then the workflows: configure, start, heart.
- **⋯** (also right click or the context-menu key): **Letzten Run auf GitHub öffnen** (only with a valid run link), **Link kopieren** (repository page; disabled in the demo). Same keys as the account menu; stays inside the panel.

**Verlauf**

- Groups **Jetzt** and **Abgeschlossen** with counts; info button **Hinweis zum Verlauf** in the first head. Chips **Alle**, **Aktiv**, **Fehlgeschlagen**, **Abgeschlossen**; a chip that lists nothing offers **Alle anzeigen**, which returns to **Alle** and focuses it.
- A row is one toggle: status tile (spinner unless reduced motion), repository without owner first (owner in the tooltip), then workflow with **#run ID**, branch chip and, once completed, the duration; status text and time on the right. One line at 360 px.
- Open runs count up (**seit 3 min**); completed runs read relative (**gerade eben**, **vor 3 Min.**, **vor 2 Std.**, **gestern**, then the date) with their duration; one shared minute clock; exact time in the tooltip.
- Expanded: account, update time, status, **Auf GitHub öffnen**, **Link kopieren** (both disabled in the demo).
- Every „show run“ path opens the Verlauf unfiltered with that run expanded and focused; a focused run that completes keeps focus as it moves to **Abgeschlossen**.

**Account menu** — switches the active account on click, **Enter** or **Space**; arrows, **Home**, **End** move; **Esc** closes and returns focus. Also **Konto verbinden …** and **Konten verwalten …**.

**Keyboard** — **F5** / **Ctrl+R** refresh (never a webview reload), **Ctrl+1/2/3** toggle **Repositories**, **Verlauf**, **Favoriten**, **/** or **Ctrl+F** (which also selects the query) focuses the search, **↓** in the search moves focus to the first result (a live run, favorite, repository row or Verlauf run) and **↓** / **↑** on a result to the next or previous one in page order (**↑** on the first returns to the search), **Ctrl+,** opens **Einstellungen**; refresh, section, **Ctrl+F** and settings keys also work from the search field, whose **Tastenkürzel** hint lists them. **Esc** order: open menu or dialog, then the search (clear, then leave), then a maximized page. Dialogs trap Tab and own the keyboard; closing returns focus to the opener (a chained dialog to the first opener). A start result takes focus when it appears.

**Starts** — a disabled start button names its one reason in the tooltip and accessible description: **Kein manueller Start**, **Kein Schreibzugriff auf dieses Repository**, **Offline – Starts sind deaktiviert**, **Zugriff verweigert – Starts sind deaktiviert**, **Start läuft …**, **Startstatus ungeklärt – prüfe die Runs auf GitHub**. A running start locks only its own target. An uncertain start stays locked across reopen and restart; its help opens on hover, focus or touch and closes with Escape or an outside click without enabling a start. Only **Sperre aufheben…** with the confirmation **Ich habe die Runs auf GitHub geprüft – Sperre aufheben** releases exactly the lock shown, and releasing never starts anything. Saving a template during a dispatch, or Enter in a template name, never starts another run.

**Notices** — plain success notices close after 3 s, hovered or not. A notice with an action keeps its own timeout (**Rückgängig** 8 s, **Im Verlauf anzeigen** 3 s) and pauses while hovered or focused; window blur or a hidden window ends the pause. While a dialog is open no notice closes; the timeout restarts in full afterwards. Warnings and errors stay until closed. The update banner has no timeout; **Hinweis schließen** hides it for that version.

**Settings** — tabs Konten, App, Updates; every tab ends in **Abbrechen** and **Speichern** (off with tooltip **Keine Änderungen** while nothing is staged). Staged: the active account (radio cards under **Aktives Konto**, **Aktiv** tag), the demo scenario, **Darstellung** (live preview, reverted unless saved), **Aktualisierung** (preset slider, recommended stop marked, **Eigener Wert** in seconds; an out-of-range value blocks **Speichern** with one error line, a red dot on the **App** tab and the tooltip **Aktualisierung ungültig**) and **Mit Windows starten**. Immediate: **Konto hinzufügen**, **Trennen**, **Demo zurücksetzen**, **Nach Updates suchen**, **Installieren**. **Speichern** closes on success and keeps the dialog open with one error line on failure.

**Connection dialog** — one step at a time: **Anmeldeart wählen** (local sign-in, token, disabled **Mit GitHub anmelden**) → local: **Lokale Anmeldung prüfen**, the found accounts (login with **GitHub CLI**, **· bereits verbunden** or **· nicht übernehmbar**), one consent line naming the account, **Übernehmen**; token: field, one line **Benötigte Rechte**, **Verbinden**. **Zurück** returns to the first step. Success closes the dialog with **Konto <login> verbunden.**; a new account lands on the overview.

**Updates** — banner **Update <v> verfügbar** · **Ansehen** (opens the Updates tab) · close **Hinweis schließen** (hidden for that version this session). Updates tab: **Nach Updates suchen** → **Version <v> verfügbar** with at most 5 changelog bullets → **Installieren** → progress bar, **Wird geprüft …**, **Wird installiert – RepoWatch startet neu …** → the app installs and restarts with its panel open ([ADR-0004](adr/0004-in-app-updates-for-unsigned-builds.md)). A failure offers **Erneut versuchen**; no update reads **Aktuell.** No release-page link.

**Styling**

- Body text 14 px, metadata at least 12 px.
- Colors only from the tokens in `src/styles/00-tokens.css`: a dark and a light set. **Einstellungen → App → Darstellung**: **System** (follows the OS live), **Hell**, **Dunkel**; stored per device in `localStorage`, applied before the panel mounts.
- WCAG AA (≥ 4.5:1) for text, metadata, accent and status colors on panel, card and hover surfaces in both themes; `src/lib/tokens.test.ts` checks the pairs and rejects color literals elsewhere.
- 2 px focus ring, drawn inset inside clipped containers. Forced colors redraw state with system colors (`src/styles/14-forced-colors.css`).

## Automated coverage

Playwright specs (`tests/<name>.spec.ts`) run against the browser demo and, where noted, a labelled native IPC mock (`tests/support/tauriMock.ts`). The mocks perform no real authorization, download or installation.

| Area                   | Specs                                                                                                                            |
| ---------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| Layout at 360/480 px   | `panel`, `repositories`, `run-actions`, `uncertain-help`, `demo-connection`, `templates`                                         |
| Overview, maximize     | `overview`, `overview-caps`, `overview-visuals`                                                                                  |
| Repositories           | `repositories`, `repo-menu`, `repository-access`                                                                                 |
| Favorites              | `templates`, `favorite-start`, `favorite-undo`, `launch-names`                                                                   |
| Verlauf                | `activity`, `run-actions`, `forced-colors`                                                                                       |
| Starts and locks       | `launch`, `start-result`, `start-locks`, `start-reasons`, `uncertain-help`, `uncertain-release`, `reconnect-locks`, `input-note` |
| Keyboard and focus     | `shortcuts`, `settings`, `favorite-undo`, `account-menu`, `focus-keep`                                                           |
| Notices and refresh    | `notices`, `panel-actions`, `refresh-recovery`                                                                                   |
| Accounts (native mock) | `account-menu`, `onboarding`, `account-selection`, `account-disconnect`, `native`, `token-permissions`                           |
| Sign-in                | `local-auth*`, `browser-auth`, `demo-connection`                                                                                 |
| Settings               | `settings`, `settings-save`, `startup`, `theme`                                                                                  |
| Updates (native mock)  | `update-install`, `native`                                                                                                       |

Unit tests (`src/lib/*.test.ts`) cover the model, run lists, formatting, account selection, the panel controller, the page state (`view`), starts and start reasons, favorites, notices, shortcut and menu keys, the demo, the run-link parser (`fixtures/run-links.json`) and repository link check, the IPC contract, the automatic repository rule (`autoRepos`), staged autostart (`autostart`), the update flow and changelog summary (`updates`), the theme preference and the design tokens.

## Screenshots

```sh
pnpm dev
# In another terminal:
pnpm exec playwright install chromium
node scripts/capture-review.mjs
node scripts/capture-card-review.mjs
node scripts/capture-navigation-review.mjs
```

- Screenshots shown to the owner come from the exact commit being delivered; never present a mock-up as the result.
- The scripts assert demo mode, fail on any request leaving the dev server, use only synthetic fixtures and capture each view in dark and light (`-dark` / `-light`). `REVIEW_BASE_URL` targets another port; `NAVIGATION_REVIEW_DIR` and `NAVIGATION_REVIEW_PREFIX` redirect the navigation capture.
- `pnpm capture:readme` (`scripts/capture-readme.mjs`) captures the demo overview at 480×760 in both themes, places it unmodified above a generic taskbar drawn from plain shapes and writes the committed README images `docs/images/repowatch-{dark,light}.png`.
- Output: `docs/review/` (gitignored, never published) — `screenshots/` with `manifest.json`, `card-revision/` with layout metrics, `navigation/` (480×760, overview also 360×560). Review captures before sharing.

## Native acceptance

Browser checks cannot establish Windows tray placement, multi-monitor/DPI behavior, sleep/resume, OS notifications, credential-manager behavior or an installed application's lifecycle. Those need a native Windows session: [native Windows smoke checklist](VALIDATION.md#native-windows-smoke-checklist). Platform support beyond Windows requires equivalent testing.
