# Verification

## Local checks

```sh
pnpm verify
pnpm exec playwright install chromium
pnpm test:e2e
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --locked --manifest-path src-tauri/Cargo.toml --no-default-features --features network,native-credentials
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --no-default-features --features network,native-credentials --all-targets -- -D warnings
```

`pnpm verify` runs Svelte/TypeScript diagnostics, frontend unit tests, release-helper tests, formatting and the production frontend build. Browser tests use synthetic local fixtures and a labelled native IPC mock. New browser/GitHub App sign-in is currently paused; its native guards and disabled UI are tested while the dormant OAuth implementation keeps its Rust regression tests. Local gh tests cover legacy CLI versions, explicit consent, stored connection feedback and account activation. Rust tests exercise transport policy, input parsing, account/cache isolation, credential-adapter failure handling, uncertain dispatch persistence and release discovery.

The Windows CI job additionally compiles the actual desktop shell and tests the autostart registry adapter in an isolated test subkey (never the real login Run key). Browser IPC mocks and core tests cover startup settings and launch decisions. See [Windows autostart behavior and acceptance](WINDOWS_AUTOSTART.md). Normal checks skip installer packaging; Desktop Release or an explicit manual installer request performs the NSIS build. CI results apply to the exact tested commit, not to later changes. See [release process](RELEASES.md).

## Test boundaries

Mocks do not establish live account permissions, device-flow authorization, OS credential-store behavior or native tray/DPI acceptance. Test those on an owner-controlled Windows system with locally entered credentials. Never put credentials in fixtures, screenshots, logs or reports.

Native browser device-flow code, explicit gh credential import with metadata-only GCM discovery, and Settings-based account selection are implemented. New browser login is temporarily disabled; future re-enabling depends on publisher GitHub App configuration and owner-controlled authorization; see [browser authentication](BROWSER_AUTH.md), [local authentication](LOCAL_AUTH.md) and [account selection](ACCOUNT_SELECTION.md). Windows popup behavior has separate [native validation boundaries](WINDOWS_POPUP_VALIDATION.md). Windows EXEs have no Authenticode certificate. Standard unsigned builds install updates in the app: they download the release EXE, verify size and SHA-256 against GitHub's asset digest and run the NSIS installer, which restarts the app without `--autostart` ([ADR-0004](adr/0004-in-app-updates-for-unsigned-builds.md)). Browser IPC tests (`update-install`, `native`) exercise the offer, progress, failed verification and retry, but do not prove an actual Windows upgrade. Verify old-to-new installation, automatic restart and preservation of accounts/preferences in a Windows session.

## Native Windows smoke checklist

CI compiles the desktop shell and runs Win32 and registry tests, but it cannot prove the following. Check them on an authorized Windows test desktop with the branch build and locally entered credentials; record the build and Windows version, never credentials or screenshots with tokens.

1. **Snapshot events with the narrowed capabilities.** `src-tauri/capabilities/default.json` grants only event listen/unlisten, header dragging and the inert maximize toggle. Connect an account and start a workflow: the panel updates from `repowatch:snapshot` events without a manual refresh, and no capability error appears.
2. **Header drag.** Dragging the app bar (`data-tauri-drag-region`) on its free space or over the brand moves the panel; the account menu, refresh and settings buttons stay clickable and do not drag. Reopening the panel from the tray places it beside the icon again.
3. **No maximize.** Double-clicking the header does not maximize or resize the window (`maximizable: false`).
4. **In-app update (unsigned build).** With an older build installed and a newer stable release published, the banner **Update <v> verfügbar** appears at startup (or after **Nach Updates suchen**); **Ansehen** shows **Version <v> verfügbar** with at most 5 changelog bullets. **Installieren** shows a progress bar (IPC `Channel`), then the app exits, the installer runs without questions for the current user from `%TEMP%\RepoWatch-update-<v>\` and RepoWatch restarts on the new version with its panel open (also after an autostart launch) and accounts and preferences intact. No browser opens.
5. **Rejected download.** A release whose asset is missing, lacks a `sha256` digest or does not match size and SHA-256 installs nothing and shows an error.
6. **Clipboard in WebView2.** **Link kopieren** on a run row places the canonical run URL on the Windows clipboard and shows **Link kopiert.**; the open-on-GitHub button opens the same URL in the default browser.
7. **Light, dark and High Contrast themes.** With **Einstellungen → App → Darstellung** on **System**, switching Windows between light and dark app mode (Settings → Personalization → Colors) switches the open panel at once; **Hell** and **Dunkel** override Windows in both directions and survive quitting and restarting RepoWatch, and the hidden window never shows the other theme first when it opens. The Playwright `theme` spec only emulates `prefers-color-scheme`. Then turn on a Windows contrast theme (Settings → Accessibility → Contrast themes) with each of the three choices: switches, the live dot of **Läuft gerade**, selected settings tab, filled favorite hearts, the favorite start button, the status icons of **Läuft gerade**, the run strip cells of each repository (success, failure, running, other and empty cells tell apart), focus rings and the chosen **Darstellung** option stay visible. The Playwright `forced-colors` spec only emulates forced colors.
8. **Autostart after uninstall.** Enable **Mit Windows starten**, uninstall RepoWatch, then inspect `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`. **Known gap:** the `io.github.fo0.repowatch` value is left behind because the installer has no uninstall hook (tracked as item 11 in the [backlog](../BACKLOG.md)); see [Windows startup](WINDOWS_AUTOSTART.md).

Tray placement, DPI and focus-loss behavior are covered by the [Windows popup acceptance](WINDOWS_POPUP_VALIDATION.md#interactive-windows-acceptance).
