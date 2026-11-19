# Environment Variables

The app needs none at runtime. All of these are optional.

| Variable                                                                                  | Kind                              | Read in                                                     | Purpose                                                                                         |
| ----------------------------------------------------------------------------------------- | --------------------------------- | ----------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| `REPOWATCH_GITHUB_CLIENT_ID`                                                              | compile-time (Rust `option_env!`) | `src-tauri/src/browser_auth.rs`                             | Public Client ID of the publisher GitHub App for device-flow sign-in (sign-in currently paused) |
| `REPOWATCH_GITHUB_APP_SLUG`                                                               | compile-time                      | `src-tauri/src/browser_auth.rs`                             | App slug for the installation page link                                                         |
| `REPOWATCH_UPDATER_PUBLIC_KEY`                                                            | compile-time / script             | `src-tauri/src/desktop_updates.rs`, `scripts/updater.mjs`   | Tauri updater public key for separately signed builds; unset in the standard unsigned release   |
| `GH_CONFIG_DIR`                                                                           | runtime (honored)                 | `src-tauri/src/local_auth.rs`                               | Explicit absolute gh config dir for local gh discovery                                          |
| `PLAYWRIGHT_CHROMIUM_EXECUTABLE`                                                          | test                              | `playwright.config.ts`, `scripts/capture-*.mjs`             | Use a preinstalled Chromium                                                                     |
| `E2E_PORT`                                                                                | test                              | `playwright.config.ts`                                      | Dev-server port for the Playwright run (default `1420`); give parallel suites distinct ports    |
| `STARTUP_REVIEW_DIR`                                                                      | test                              | `tests/startup.spec.ts`                                     | When set, the startup spec writes dialog screenshots there; unset skips the captures            |
| `POWERSHELL_EXECUTABLE`                                                                   | test                              | `scripts/release-workflow.test.mjs`                         | PowerShell 7 binary for the workflow tests                                                      |
| `CI`                                                                                      | test                              | `playwright.config.ts`, `scripts/release-workflow.test.mjs` | Forbid `.only`, one retry, no server reuse; PowerShell tests mandatory                          |
| `REVIEW_BASE_URL`, `CARD_REVIEW_DIR`, `NAVIGATION_REVIEW_DIR`, `NAVIGATION_REVIEW_PREFIX` | script                            | `scripts/capture-*.mjs`                                     | Screenshot capture target / output dir and file prefix (`docs/review/`, gitignored)             |

## Formats and defaults

What the code accepts and falls back to when a variable is unset:

- `REPOWATCH_GITHUB_CLIENT_ID` — starts with `Iv`, 8–80 characters, ASCII letters, digits and `.` only. Unset or invalid: browser sign-in counts as not set up (`browser_auth.rs → validate_publisher`).
- `REPOWATCH_GITHUB_APP_SLUG` — 1–100 characters, lowercase letters, digits and `-` only; builds `https://github.com/apps/<slug>/installations/new`. Needs a valid Client ID beside it.
- `REPOWATCH_UPDATER_PUBLIC_KEY` — the base64 Tauri updater (minisign) public key: decodes to two lines, `untrusted comment: …` and a 42-byte base64 key (`scripts/updater.mjs → updaterConfig`). In the Rust build, empty or whitespace counts as unset and the app uses the unsigned in-app update path (`desktop_updates.rs → updater_key`); `node scripts/updater.mjs config` fails without it.
- `GH_CONFIG_DIR` — must be an absolute path; a relative one makes every local discovery command fail with an error instead of falling back (`local_auth.rs → run_process`). Unset: gh's own default location.
- `PLAYWRIGHT_CHROMIUM_EXECUTABLE` — unset: Playwright's own downloaded Chromium (`pnpm exec playwright install chromium`).
- `E2E_PORT` — default `1420`.
- `REVIEW_BASE_URL` — default `http://127.0.0.1:1420` (the `pnpm dev` server).
- `CARD_REVIEW_DIR` — default `docs/review/card-revision`.
- `NAVIGATION_REVIEW_DIR` — default `docs/review/navigation`; `NAVIGATION_REVIEW_PREFIX` default empty.

## Secrets Locations

Secrets are never committed. No secret is needed to build or test. Runtime GitHub tokens live only in the OS credential vault under RepoWatch's own namespaces (`github-<id>`, `github-gh-<id>`, `github-device-<id>`), never in the repo. The release workflow uses only the repository `GITHUB_TOKEN`; any further CI secret belongs in the CI secret store. Fixtures and captures are synthetic. No `.env` file is used; a local one would stay gitignored. A new secret gets a placeholder in `.env.example` and a question to the user — never `gh secret set` unprompted.

<!-- Generated by claude-code-optimizer v1.74.1 -->
