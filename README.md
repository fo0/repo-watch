# RepoWatch

[![Desktop Release](https://github.com/fo0/repo-watch/actions/workflows/desktop-release.yml/badge.svg?branch=main)](https://github.com/fo0/repo-watch/actions/workflows/desktop-release.yml)
[![Windows release](https://github.com/fo0/repo-watch/actions/workflows/desktop-release.yml/badge.svg?branch=main&event=workflow_dispatch)](https://github.com/fo0/repo-watch/actions/workflows/desktop-release.yml)

<img src="src-tauri/icons/source.svg" alt="" width="64" height="64" align="right" />

**GitHub Actions within reach.** RepoWatch is a compact Windows tray companion for monitoring runs, launching workflows and keeping frequently used launches as pinned templates.

Built with **Tauri 2, Rust, Svelte 5 and TypeScript**. Windows x64 is the primary target; other desktop platforms require their own native validation.

| Dark                                                                                                                                                                                               | Light                                                                                                                                                                                                 |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| <img src="docs/images/repowatch-dark.png" alt="RepoWatch panel in the dark theme, opened above its tray icon on a dark taskbar, showing running workflows, a favorite and example repositories" /> | <img src="docs/images/repowatch-light.png" alt="RepoWatch panel in the light theme, opened above its tray icon on a light taskbar, showing running workflows, a favorite and example repositories" /> |

<sub>Browser demo with synthetic example data on an illustrative taskbar; regenerate with `pnpm capture:readme`.</sub>

## What it does

Features in detail: [What RepoWatch does](docs/FEATURES.md).

## Sign in

The native app starts without demo accounts. Credentials are owned by the Rust service and stored in the operating system's credential vault; browser preferences contain only nonsecret selections.

| Method                            | Current behavior                                                                                                                                                                                                                                                                    |
| --------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **GitHub browser sign-in**        | Temporarily disabled in this version, including when publisher configuration is present. Use an existing gh CLI account or a personal access token.                                                                                                                                 |
| **Personal access token**         | Enter an existing token; RepoWatch verifies the identity before storing its own credential entry. Repository access, Actions permissions and organization/SSO restrictions still apply.                                                                                             |
| **Existing local gh CLI account** | On Windows, explicitly discover and select a gh account, then confirm before RepoWatch verifies and copies it into its own vault namespace. Older gh versions are supported; discovery alone never connects an account. Disconnect removes the RepoWatch copy, not the CLI account. |
| **Git Credential Manager**        | Discovery shows account metadata only. GCM credentials are not imported and Git/SSH access is not treated as GitHub API authentication.                                                                                                                                             |

Saved connections visibly show login, source and whether they are active in RepoWatch. Browser, token and imported CLI credentials remain distinct. No automatic credential-source cascade is used. See [connection and permissions](docs/GITHUB_SETUP.md), [browser publisher configuration](docs/BROWSER_AUTH.md) and [local sign-in](docs/LOCAL_AUTH.md).

## Install or build

Published Windows installers, when available, are listed on the [Releases page](https://github.com/fo0/repo-watch/releases). If no release is listed, build the installer from source below.

The standard Windows x64 NSIS installer is **unsigned** and can be built and released without signing keys. Download and run the public EXE to install; later updates install from inside the app. The separately configured signed-updater path keeps its signature verification enabled; the standard Desktop Release Action does not create those updater artifacts. See [release and update behavior](docs/RELEASES.md).

### Native development

Prerequisites:

- **Node.js 24**, as pinned in `.node-version` (and `engines` in `package.json`), and **pnpm 11.19.0** (`packageManager` in `package.json`).
- The **Rust** toolchain pinned in `rust-toolchain.toml` (1.99.1 with clippy and rustfmt); rustup installs it on first use.
- The [Tauri system prerequisites](https://v2.tauri.app/start/prerequisites/). On Windows, install Microsoft C++ Build Tools with **Desktop development with C++**, use the MSVC Rust toolchain and provide the WebView2 runtime. Linux/macOS need their corresponding native development dependencies.

From the repository root:

```sh
pnpm install --frozen-lockfile
pnpm desktop:dev
```

No environment variables or `.env` file are needed to build, test or run RepoWatch. The optional compile-time variables (browser sign-in Client ID and app slug, signed-updater public key) are described in [browser authentication](docs/BROWSER_AUTH.md) and [release and update behavior](docs/RELEASES.md); the full list is in [`agent_docs/env-vars.md`](agent_docs/env-vars.md).

To build the unsigned installer **on Windows**:

```sh
pnpm build:installer
```

Output is written under `src-tauri/target/release/bundle/nsis/`. Browser sign-in configuration is a separate publisher responsibility; building the app does not register a GitHub App or create authorization grants.

### Browser demo

The browser preview needs Node/pnpm only, without Rust or a GitHub account:

```sh
pnpm install --frozen-lockfile
pnpm dev
```

Open **http://127.0.0.1:1420**. This preview uses labelled synthetic accounts and local simulations. It does not authenticate with GitHub or start real workflows.

## CI and releases

CI jobs and the release pipeline: [CI and releases](docs/RELEASES.md#ci-and-releases).

[Desktop Release](.github/workflows/desktop-release.yml) handles explicitly requested unsigned Windows EXE releases with generated versions and changelogs. No additional signing key is required; see the release documentation. Both badges above follow the manual release runs on `main`; CI runs on pull requests and inside each release run. See [release documentation](docs/RELEASES.md) for the exact triggers and publishing process.

Local verification:

```sh
pnpm verify
pnpm exec playwright install chromium
pnpm test:e2e
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --locked --manifest-path src-tauri/Cargo.toml --no-default-features --features network,native-credentials
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --no-default-features --features network,native-credentials --all-targets -- -D warnings
```

With Windows native prerequisites installed:

```sh
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --features desktop --all-targets -- -D warnings
cargo test --locked --manifest-path src-tauri/Cargo.toml --features desktop --bin repowatch
```

`pnpm verify` includes type checks, unit tests, release-helper tests, formatting, icon byte reproduction and the frontend production build. Fixture/browser tests and compilation do not establish real GitHub permissions, OS credential-store behavior, WebView2 login or interactive tray/DPI acceptance. Those require an owner-controlled native Windows session; see [verification boundaries](docs/VALIDATION.md) and [Windows popup checks](docs/WINDOWS_POPUP_VALIDATION.md).

## Documentation

- [Architecture](docs/ARCHITECTURE.md) and [roadmap](docs/ROADMAP.md)
- [Account selection](docs/ACCOUNT_SELECTION.md), [browser authentication](docs/BROWSER_AUTH.md) and [local authentication](docs/LOCAL_AUTH.md)
- [UX checks and local screenshots](docs/UX_REVIEW.md)
- [Asset sources and reproduction](docs/ASSETS.md)
- [Source-distribution review boundaries](docs/DISTRIBUTION.md)

## License and provenance

[MIT](LICENSE). Dependencies retain their own licenses. Application icons and UI glyphs are defined from repository-native geometry; their editable sources and deterministic generator are documented in [Asset sources](docs/ASSETS.md).

RepoWatch is an independent project and is not affiliated with GitHub. OpenQuota informed the desktop architecture.
