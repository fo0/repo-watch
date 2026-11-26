# Windows releases without signing requirements

The standard **Desktop Release** Action builds and publishes the unsigned Windows x64 NSIS EXE. It requires only the repository-provided `GITHUB_TOKEN` for publication. No Tauri signing key, updater public key, certificate, `.sig`, or `latest.json` is required. `src-tauri/tauri.release.conf.json` explicitly sets `createUpdaterArtifacts: false`; the Action has no signing preflight or signing-secret bindings.

## App updates

Standard unsigned builds update inside the app ([ADR-0004](adr/0004-in-app-updates-for-unsigned-builds.md)):

1. **Check.** At startup, every 6 hours, after connecting an account and on **Nach Updates suchen**, the app reads the public `releases/latest` API of `fo0/repo-watch` on `api.github.com`, unauthenticated, with a bounded response. It accepts only a canonical stable `vX.Y.Z` newer than the running version whose release has the asset `RepoWatch_<version>_x64-setup.exe` with GitHub's `sha256` digest and size. Drafts and prereleases are ignored; the active account does not matter. Automatic checks fail silently; only **Nach Updates suchen** shows an error.
2. **Offer.** A banner **Update <version> verfügbar** with **Ansehen** opens **Einstellungen → Updates**; **Hinweis schließen** hides it for that version until the next start. The Updates tab shows **Version <version> verfügbar**, the release date, at most 5 changelog bullets and **Installieren**.
3. **Download.** **Installieren** calls one native command, `install_update`, which streams progress over an IPC channel (**Lädt … n %**, **Wird geprüft …**, **Wird installiert – RepoWatch startet neu …**). It downloads the asset unauthenticated over HTTPS, following at most 5 redirects and only to GitHub release hosts.
4. **Verify.** Size and SHA-256 must match the asset metadata; otherwise nothing is written or run, and **Erneut versuchen** retries.
5. **Install.** Only verified bytes are written, to `%TEMP%\RepoWatch-update-<version>\RepoWatch_<version>_x64-setup.exe` (a leftover folder of the same version is replaced, folders of other versions are removed). The app runs it passively (`/P /R /UPDATE`, current-user install) and exits; the installer restarts RepoWatch without launch arguments, so the panel opens even after an `--autostart` launch.

There is no release-page link. Trust equals a manual download from the Releases page: GitHub's TLS plus the asset digest, no publisher signature. Code: `src-tauri/src/updates.rs`, `src-tauri/src/updates/installer.rs`, `src-tauri/src/desktop_updates.rs`, `src/lib/updater.svelte.ts`, `src/lib/UpdatePanel.svelte`.

Builds compiled with an updater public key (`REPOWATCH_UPDATER_PUBLIC_KEY`, separately configured) use the Tauri updater behind the same `install_update` command and verify its signature; the plugin passes the launch arguments on (`/ARGS`). The standard Desktop Release Action generates no signed updater artifacts and publishes no `latest.json`, so the built-in signed feed (`releases/latest/download/latest.json`) currently resolves to nothing. Once signing is introduced, signed builds need a dedicated feed URL that serves only signed manifests.

A Tauri updater signature and a Windows Authenticode certificate are different mechanisms; neither is needed to publish the standard EXE or to update it in the app.

## CI and release triggers

Two workflows, nothing else:

- **CI** (`.github/workflows/ci.yml`) runs on pull requests. It always runs the frontend job (lint, typecheck, unit/release tests, build, Playwright). The Rust job runs only when the pull request touches `src-tauri/`, `fixtures/`, `rust-toolchain.toml` or `ci.yml`; the Windows job (desktop clippy, Win32 and credential-store tests, release tests on Windows) only when it touches those, `scripts/`, any workflow, `package.json`, `pnpm-lock.yaml`, `pnpm-workspace.yaml` or `.node-version`. Renamed files count with their old and new path. A new push cancels the pull request's running CI.
- **Desktop Release** (`.github/workflows/desktop-release.yml`) starts only by hand: **Actions → Desktop Release → Run workflow** on `main` (on any other branch nothing runs). It first runs the complete CI for that exact commit, then builds the unsigned NSIS EXE on Windows and publishes it with a generated changelog covering everything merged since the previous release. Merges never start a release. Releases run one at a time.

Release publication uses the existing repository GITHUB_TOKEN; no additional signing secret is required. Rust caches are separated for Linux core, Windows checks and the release build; only runs on `main` save them.

## Automatic version and changelog

1. The workflow uses its immutable GitHub run creation time, preserving the version on retries. Numeric versions use UTC `YYYY.MMDD.HHmm`, with no leading zeroes in numeric components. For example, `2026-10-31T15:33Z` becomes `2026.1031.1533`; the release title displays `2026-10-31 16:33 (Berlin, GMT+1)`. This is semver/Windows compatible and stays chronological over midnight, year rollover and Berlin clock changes.
2. The same version is stamped into package.json, Tauri config, Cargo.toml and the root Cargo.lock entry in the build workspace. Development versions in Git stay unchanged; there is no bump commit or extra CI run. The generated `v…` tag targets the exact checked-out main source SHA.
3. GitHub generates the changelog from merged changes. An empty generated changelog uses a short default note and does not block an otherwise valid release. Release notes include timestamp, version and source SHA, plus a fixed installer note that still says to install the EXE manually (BACKLOG #31); no manual version-specific changelog entry or pre-created tag is required. The build workspace records its timestamp, version and source SHA for reproduction.
4. New builds reject existing tags/releases, stale versions, source mismatches or altered build plans. Two release runs created in the same minute get the same version: the second stops before publishing; start it again a minute later. Existing assets/drafts are never overwritten by a blind retry.
5. It builds with `createUpdaterArtifacts: false` from `src-tauri/tauri.release.conf.json`. It runs `gh release create TAG INSTALLER --target SOURCE_SHA --title TITLE --notes-file NOTES --latest`. The CLI handles the upload and publication; no custom draft URL or installer-metadata gate is used. The workflow confirms that the result is published, downloads the EXE and compares its hash with the local build. A successful build is also retained as a CI artifact if publication fails.

If publication fails after the upload, `gh release create` can leave a draft with the EXE. Delete that draft, then re-run the failed release job (same run, same version) or start a new release; the workflow never overwrites or republishes an existing draft or asset.

The installer is a direct `.exe` release asset; no signature or updater-manifest sidecar is required. GitHub's optional CI artifact transport may wrap files in ZIPs; the user-facing release download is the EXE itself. Generated files live in ignored `release-work/`; no screenshot, private key or token is published.

## Runtime acceptance

Automated fixtures and Windows compilation do not prove a live account, native credential manager, Windows tray/DPI behavior or an actual installation. Those remain owner-controlled Windows checks. Browser OAuth still needs a separately approved GitHub App registration; the existing token alternative is implemented. Installations and account setup are user-controlled and are not performed by CI.

## Regression tests

`pnpm release:test` includes tests that execute the actual PowerShell run blocks from the Action against a temporary local Git repository and simulated GitHub/build commands. They cover the manual run's Checks gate (green, failed, missing), a complete keyless EXE release, and a failed or altered download of the published EXE. Nothing is published or installed. PowerShell 7 is required for these workflow tests on CI (provided by both GitHub runner images); local runs without PowerShell report them as skipped.

## CI and releases

[CI](../.github/workflows/ci.yml) runs on pull requests: frontend, browser interactions, reproducible icons, Rust formatting/tests/Clippy and Windows desktop Clippy with Win32 popup/window-ownership and isolated autostart-registry tests. Pull requests skip the Rust and Windows jobs when they touch nothing those jobs check. A failed browser test run uploads the Playwright report as an artifact. [Desktop Release](../.github/workflows/desktop-release.yml) is started by hand on `main`: the full CI for that commit, then the unsigned Windows EXE with a generated changelog on the [Releases page](https://github.com/fo0/repo-watch/releases).
