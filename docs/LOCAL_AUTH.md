# Existing local sign-in

In the connection dialog (**GitHub verbinden**) you can explicitly bring an existing **GitHub CLI (`gh`)** sign-in into RepoWatch. Open it with **GitHub-Konto verbinden** on first launch (or in **Einstellungen → Konten** while no account exists), otherwise with **Einstellungen → Konten → Konto hinzufügen** or **Konto verbinden …** in the account menu. Browser sign-in / GitHub App is temporarily disabled. A personal access token remains available as an alternative.

## Discover, connect and activate

1. In **Anmeldeart wählen**, choose **Bestehende Anmeldung auf diesem PC verwenden** and click **Lokale Anmeldung prüfen** (or press **Enter** on the option). Choosing the option alone accesses no local sign-in; only that button or **Enter** starts the check.
2. Found accounts appear as one compact list, one line each: the login and its source — **GitHub CLI**, **GitHub CLI · bereits verbunden** (RepoWatch already holds a copy) or **… · nicht übernehmbar** (Git Credential Manager and unusable gh entries, not selectable). A single importable account is preselected. gh's own active-account marker is not the active RepoWatch account.
3. One consent line names the chosen account (**Das Token von <login> wird aus der GitHub CLI übernommen und im Windows-Tresor gespeichert.**). Only the click on **Übernehmen** imports it (`connect_local_auth` with `confirmImport: true`): gh reads the selected token, RepoWatch verifies the GitHub identity and stores its own copy in the operating system's credential vault. Success closes the dialog by itself with the notice **Konto <login> verbunden.**; the account becomes active and a new one lands on the overview.

**Zurück** returns to the method step and cancels a running check; **Erneut prüfen** scans again. Importing from gh again after a token rotation requires the same explicit consent. The UI keeps this to labels and one consent line; the details below are documentation, not panel text (`agent_docs/ux_rules.md`).

The token path (**Mit Token verbinden** → **Weiter**) is one masked field, one line **Benötigte Rechte** and **Verbinden**, which stays off until a token is entered ([GitHub connection](GITHUB_SETUP.md)). Connected accounts are listed under **Einstellungen → Konten → Aktives Konto** ([account selection](ACCOUNT_SELECTION.md)).

## When gh is not found

Check in a normal terminal:

```sh
gh --version
gh auth status --hostname github.com
```

The dialog shows one line: **GitHub CLI (gh) nicht gefunden. Nach der Installation RepoWatch neu starten.** or **Keine gh-Anmeldung gefunden. Im Terminal mit „gh auth login“ anmelden.** If you are not signed in to gh yet, run `gh auth login --hostname github.com` there yourself. Restart RepoWatch after a new gh installation so it sees the updated PATH, then click **Erneut prüfen**.

Git, GitHub Desktop and GitHub CLI are different programs. A working `git push`, an SSH key or a GitHub Desktop sign-in is not an automatically usable gh API sign-in. Git Credential Manager is therefore shown as an additional Git source, not as a selectable API connection. RepoWatch runs neither Git credential helpers nor a CLI login of its own.

## Version and installation support

RepoWatch first checks the help of the installed gh commands. Newer versions receive `--no-refresh` when they support it; older versions do not receive the flag. Passing `--no-refresh` unconditionally used to break older, otherwise working installations.

- **gh with JSON status:** `auth status --hostname github.com --json hosts` lists stored accounts. `--no-refresh` is added only when supported. Environment token sources are not offered for import.
- **Older gh without JSON status:** `api --hostname github.com user --jq .login` confirms only the identity active there. Human-readable status output is not parsed and credential files are not read. To choose among further stored gh accounts, update gh.
- **After confirmation:** `auth token --hostname github.com --user <login>` reads exactly the chosen account. Here too, `--no-refresh` is added only when supported. Without `--user` support, nothing is imported. RepoWatch never invokes a login, account switch or credential refresh. Before storing, `/user` is checked again against the selected login.

Windows discovery covers Program Files, LocalAppData/Programs, Scoop including configured/global roots, WinGet links, Chocolatey and absolute PATH entries. The executable is canonicalized before it starts; relative PATH entries, the current directory, project binaries and `.cmd` wrappers are excluded. An explicit absolute `GH_CONFIG_DIR` is honored. Non-Windows platforms currently offer no local source.

## Credential ownership and process boundaries

Local imports use the stable API user ID in their own namespace `github-gh-<id>`; PAT connections use `github-<id>`. A PAT therefore never overwrites a gh connection of the same GitHub user. Disconnecting (**Trennen** in the account card, confirmed there) removes only the RepoWatch copy and that connection's cached activity, repositories, favorites and uncertain-start locks, neither gh/GCM sign-ins nor their permissions; for an imported account the confirmation says that the gh CLI sign-in stays untouched. A later gh rotation does not update an already stored copy automatically.

Child processes get a minimal environment without GitHub token variables, alternative hosts, debug/trace configuration or askpass. The explicit gh configuration and proxy and CA trust settings are kept. Stdin is closed; stdout and stderr are each capped at 64 KiB and the run time at 20 seconds. Raw process output and token values never appear in the UI, IPC, logs or snapshots. Cancelling ends the check and discards late results.

GCM's `github list --url https://github.com` is used for account names only (v2: stdout, v3: stderr). Neither `git credential fill` nor any other configured Git helper is invoked. Git/push rights do not prove Actions API permission. A `repo` scope hints at classic private API rights but does not replace repository, organization or SSO approval. No test starts a workflow to probe permissions.

## Validation and limits

Rust tests model current and older gh versions, including real unknown-flag errors, JSON status, the active legacy identity, exact account selection and token sources. Path tests cover Scoop, WinGet, PATH and the rejection of local project binaries. Synthetic processes cover timeout, cancellation, output limits and credential isolation. Browser tests (`local-auth`, `local-auth-guidance`, `browser-auth`, `account-selection`) cover the steps, selection, consent, already connected accounts, the one-line results and the paused browser path.

These tests read no real local sign-ins. The concrete gh installation and Windows Credential Manager still have to be checked on the affected Windows PC. Commands and limits are listed under [Verification](VALIDATION.md).

## Official sources

- [gh auth status](https://cli.github.com/manual/gh_auth_status), [gh auth token](https://cli.github.com/manual/gh_auth_token), [gh api](https://cli.github.com/manual/gh_api), [gh environment](https://cli.github.com/manual/gh_help_environment)
- [GCM CLI](https://github.com/git-ecosystem/git-credential-manager/blob/main/docs/usage.md)
- [Active account selection](ACCOUNT_SELECTION.md)
