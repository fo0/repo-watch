//! Explicit, read-only local account discovery. No startup credential reads, shell, or login.
//! Only gh has a supported API token adapter. GCM's Git helper is deliberately not invoked.
use crate::auth::SecretToken;
use serde::Serialize;
#[cfg(any(windows, test))]
use std::path::PathBuf;
use std::{
    io::Read,
    path::Path,
    process::{Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

mod gh;
mod programs;
mod text;

use gh::{gh_help, gh_status, valid_login};
pub use programs::Programs;
#[cfg(test)]
use programs::{safe_executable, windows_programs};
use text::{cancelled, failed};

const LIMIT: u64 = 64 * 1024;
const TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalAccount {
    pub source: String,
    pub host: String,
    pub login: String,
    pub usable: bool,
    pub active: bool,
    pub detail: String,
    pub actions_read: String,
    pub actions_write: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalDiscovery {
    pub accounts: Vec<LocalAccount>,
    pub notices: Vec<String>,
}

// Secrets, stderr and raw process output deliberately have no Debug/Serialize implementation.
pub struct Output {
    pub success: bool,
    pub bytes: Vec<u8>,
    stderr: Vec<u8>,
}
impl Drop for Output {
    fn drop(&mut self) {
        self.bytes.fill(0);
        self.stderr.fill(0);
        std::hint::black_box(&mut self.bytes);
        std::hint::black_box(&mut self.stderr);
    }
}
pub trait Runner {
    fn run(
        &self,
        program: &Path,
        args: &[&str],
        current: &dyn Fn() -> bool,
    ) -> Result<Output, String>;
}
pub struct ProcessRunner;
impl Runner for ProcessRunner {
    fn run(
        &self,
        program: &Path,
        args: &[&str],
        current: &dyn Fn() -> bool,
    ) -> Result<Output, String> {
        run_process(program, args, current, TIMEOUT)
    }
}

fn run_process(
    program: &Path,
    args: &[&str],
    current: &dyn Fn() -> bool,
    timeout: Duration,
) -> Result<Output, String> {
    if !program.is_absolute() || !current() {
        return Err(cancelled().into());
    }
    let mut command = Command::new(program);
    command
        .args(args)
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // No inherited GH_TOKEN/GITHUB_TOKEN, GH_HOST, GH_DEBUG, GIT_CONFIG_*,
    // askpass, tracing, extension paths, or CI credentials. Use the desktop user's normal store.
    for name in [
        "HOME",
        "USERPROFILE",
        "APPDATA",
        "LOCALAPPDATA",
        "SystemRoot",
        "WINDIR",
        "TEMP",
        "TMP",
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "NO_PROXY",
        "http_proxy",
        "https_proxy",
        "no_proxy",
        "SSL_CERT_FILE",
        "SSL_CERT_DIR",
    ] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    // Honor the user's explicitly configured gh store; never substitute a different account store.
    if let Some(directory) = std::env::var_os("GH_CONFIG_DIR") {
        if !Path::new(&directory).is_absolute() {
            return Err(text::config_dir_relative().into());
        }
        command.env("GH_CONFIG_DIR", directory);
    }
    command
        .env("GH_PROMPT_DISABLED", "1")
        .env("GH_NO_UPDATE_NOTIFIER", "1")
        .env("GH_NO_EXTENSION_UPDATE_NOTIFIER", "1")
        .env("NO_COLOR", "1")
        .env("GCM_INTERACTIVE", "0")
        .env("GCM_TRACE", "0")
        .env("GCM_TRACE_SECRETS", "0");
    // Never search the app/repository/current directory, even for child utilities.
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
        if let Some(root) = std::env::var_os("SystemRoot") {
            let system = PathBuf::from(root).join("System32");
            command.env("PATH", &system).current_dir(system);
        } else {
            return Err(failed().into());
        }
        // GCM itself locates Git to read configuration. Supply only the validated
        // installed executable directory, never an inherited GIT_EXEC_PATH or PATH.
        if args == ["github", "list", "--url", "https://github.com"] {
            if let Some(git) = Programs::installed().git {
                if let Some(directory) = git.parent() {
                    command.env("GIT_EXEC_PATH", directory);
                }
            }
        }
    }
    #[cfg(not(windows))]
    command.env("PATH", "/usr/bin:/bin").current_dir("/");
    let mut child = command.spawn().map_err(|_| failed().to_string())?;
    let stdout = child.stdout.take().ok_or(failed())?;
    let stderr = child.stderr.take().ok_or(failed())?;
    let (send, receive) = mpsc::sync_channel(2);
    capture(stdout, send.clone(), false);
    capture(stderr, send, true);
    let deadline = Instant::now() + timeout;
    let mut output = None;
    let mut diagnostics = None;
    loop {
        if !current() || Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(if !current() {
                cancelled()
            } else {
                text::timed_out()
            }
            .into());
        }
        match receive.try_recv() {
            Ok((false, Some(value))) => output = Some(value),
            Ok((true, Some(value))) => diagnostics = Some(value),
            Ok((_, None)) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(failed().into());
            }
            Err(mpsc::TryRecvError::Disconnected) if output.is_none() || diagnostics.is_none() => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(failed().into());
            }
            _ => {}
        }
        match child.try_wait() {
            Ok(Some(status)) if output.is_some() && diagnostics.is_some() => {
                let mut value = output.take().ok_or(failed())?;
                value.stderr = std::mem::take(&mut diagnostics.take().ok_or(failed())?.bytes);
                value.success = status.success();
                return Ok(value);
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(failed().into());
            }
            _ => std::thread::sleep(Duration::from_millis(20)),
        }
    }
}

fn capture(
    stream: impl Read + Send + 'static,
    send: mpsc::SyncSender<(bool, Option<Output>)>,
    stderr: bool,
) {
    std::thread::spawn(move || {
        let mut output = Output {
            success: false,
            bytes: Vec::new(),
            stderr: Vec::new(),
        };
        let result = stream.take(LIMIT + 1).read_to_end(&mut output.bytes);
        let _ = send.send((
            stderr,
            if result.is_ok() && output.bytes.len() <= LIMIT as usize {
                Some(output)
            } else {
                None
            },
        ));
    });
}

pub fn discover(
    programs: &Programs,
    runner: &dyn Runner,
    current: &dyn Fn() -> bool,
) -> Result<LocalDiscovery, String> {
    if !current() {
        return Err(cancelled().into());
    }
    let mut result = LocalDiscovery {
        accounts: Vec::new(),
        notices: Vec::new(),
    };
    if let Some(gh) = &programs.gh {
        result.notices.push(text::gh_found().into());
        match gh_status(gh, runner, current) {
            Ok((accounts, legacy)) => {
                if legacy {
                    result.notices.push(text::gh_legacy().into());
                }
                if accounts.is_empty() {
                    result.notices.push(text::gh_no_accounts().into());
                }
                result.accounts.extend(accounts);
            }
            Err(error) => result.notices.push(error),
        }
    } else {
        result.notices.push(text::gh_missing().into());
    }
    if let Some(gcm) = &programs.gcm {
        match runner.run(
            gcm,
            &["github", "list", "--url", "https://github.com"],
            current,
        ) {
            Ok(out) if out.success => {
                // GCM v2 writes names to stdout; v3's ConsoleService uses stderr.
                // Only valid GitHub handles from a successful fixed list command leave
                // this boundary. Never return diagnostic text or process errors.
                let names = std::str::from_utf8(&out.bytes).unwrap_or("");
                let names_v3 = std::str::from_utf8(&out.stderr).unwrap_or("");
                let before = result.accounts.len();
                for login in names
                    .lines()
                    .chain(names_v3.lines())
                    .map(str::trim)
                    .filter(|v| valid_login(v))
                    .take(50)
                {
                    result.accounts.push(LocalAccount {
                        source: "gcm".into(),
                        host: "github.com".into(),
                        login: login.into(),
                        usable: false,
                        active: false,
                        detail: text::gcm_account_detail().into(),
                        actions_read: text::gcm_actions_read().into(),
                        actions_write: text::gcm_actions_write().into(),
                    });
                }
                result.notices.push(
                    if result.accounts.len() == before {
                        text::gcm_no_names()
                    } else {
                        text::gcm_names()
                    }
                    .into(),
                );
            }
            _ => result.notices.push(text::gcm_list_unavailable().into()),
        }
    } else {
        result.notices.push(text::gcm_missing().into());
    }
    result.notices.push(
        if programs.git.is_some() {
            text::git_found()
        } else {
            text::git_missing()
        }
        .into(),
    );
    if !current() {
        return Err(cancelled().into());
    }
    Ok(result)
}

/// Called only after explicit selection and confirmation of OS-keyring import in the UI.
pub fn selected_token(
    programs: &Programs,
    runner: &dyn Runner,
    login: &str,
    current: &dyn Fn() -> bool,
) -> Result<SecretToken, String> {
    if !valid_login(login) {
        return Err(text::invalid_login().into());
    }
    if !current() {
        return Err(cancelled().into());
    }
    let gh = programs.gh.as_deref().ok_or(failed())?;
    if !gh_status(gh, runner, current)?
        .0
        .iter()
        .any(|a| a.login == login && a.usable)
    {
        return Err(text::selection_gone().into());
    }
    if !current() {
        return Err(cancelled().into());
    }
    let help = gh_help(gh, "token", runner, current)?;
    if !help.contains("--user") {
        return Err(text::no_safe_selection().into());
    }
    let mut args = vec!["auth", "token", "--hostname", "github.com", "--user", login];
    if help.contains("--no-refresh") {
        args.push("--no-refresh");
    }
    let out = runner.run(gh, &args, current)?;
    if !out.success || !current() {
        return Err(failed().into());
    }
    let token = std::str::from_utf8(&out.bytes)
        .map_err(|_| failed())?
        .trim_end_matches(['\r', '\n']);
    if token.is_empty()
        || token.len() > 2048
        || token
            .bytes()
            .any(|b| b.is_ascii_whitespace() || b.is_ascii_control())
    {
        return Err(failed().into());
    }
    Ok(SecretToken::new(token.to_owned()))
}

#[cfg(test)]
mod tests;
