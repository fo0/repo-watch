//! The local diagnostic log: `logs/repowatch.log` under the app data directory, one rotated
//! predecessor, level `info` unless the user switches to `debug`.
//!
//! Logging never breaks or blocks the app: every failure is swallowed, and before [`init`]
//! (tests, the demo) every call is a no-op. Credentials never reach the file: every line
//! passes [`sanitise`], and URLs are reduced to their path by [`url_path`]. Callers log only
//! method, path, status, duration, page counts, user-facing error text, logins and
//! `owner/repo` — never headers, bodies, ETags or vault content. Lines are fixed English.

use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU8, Ordering},
        Mutex, OnceLock, PoisonError,
    },
    time::Instant,
};

/// The directory below the app data directory.
pub const DIR_NAME: &str = "logs";
pub const FILE_NAME: &str = "repowatch.log";
/// The one predecessor kept when the file reaches its cap.
pub const ROTATED_NAME: &str = "repowatch.log.1";
const MAX_BYTES: u64 = 1024 * 1024;
const MAX_MESSAGE_CHARS: usize = 2000;
const REDACTED: &str = "[redacted]";

/// What is written: `Info` is the default, `Debug` adds every GitHub request.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    #[default]
    Info,
    Debug,
}

impl Level {
    /// The fixed name in log lines, the settings row and IPC.
    pub fn label(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Debug => "debug",
        }
    }
    fn from_u8(value: u8) -> Self {
        if value == 1 {
            Self::Debug
        } else {
            Self::Info
        }
    }
}

/// Reads a stored level; a value this build does not know reads as absent (info), so a row
/// from a newer build never costs the other settings.
pub fn lenient_level<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Level>, D::Error> {
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(match value.as_ref().and_then(serde_json::Value::as_str) {
        Some("info") => Some(Level::Info),
        Some("debug") => Some(Level::Debug),
        _ => None,
    })
}

/// The path of a URL or request target: no scheme, host, userinfo, query or fragment.
pub fn url_path(url: &str) -> &str {
    let rest = match url.split_once("://") {
        Some((_, rest)) => rest.find('/').map_or("/", |start| &rest[start..]),
        None => url,
    };
    &rest[..rest.find(['?', '#']).unwrap_or(rest.len())]
}

/// The one point every line passes before it is written: a word that looks like a credential
/// is replaced, control characters and line breaks go, and the length is bounded.
pub fn sanitise(message: &str) -> String {
    if message.contains("-----BEGIN") {
        return REDACTED.into();
    }
    let mut out = String::with_capacity(message.len().min(MAX_MESSAGE_CHARS));
    let mut after_bearer = false;
    for word in message.split_whitespace() {
        if !out.is_empty() {
            out.push(' ');
        }
        let lower = word.to_ascii_lowercase();
        let secret = after_bearer
            || [
                "ghp_",
                "gho_",
                "ghu_",
                "ghs_",
                "ghr_",
                "github_pat_",
                "token=",
                "secret=",
                "password=",
                "key=",
                "code=",
            ]
            .iter()
            .any(|mark| lower.contains(mark));
        after_bearer = matches!(
            lower.trim_matches(|c: char| !c.is_ascii_alphanumeric()),
            "bearer" | "basic"
        );
        if secret {
            out.push_str(REDACTED);
        } else {
            out.extend(word.chars().filter(|c| !c.is_control()));
        }
    }
    if out.chars().count() > MAX_MESSAGE_CHARS {
        out = out.chars().take(MAX_MESSAGE_CHARS).collect();
        out.push('…');
    }
    out
}

/// One log directory. The process uses a single instance behind [`init`]; tests build their
/// own with a small cap.
pub struct Logger {
    dir: PathBuf,
    max_bytes: u64,
    level: AtomicU8,
    /// Serializes rotation and append; a poisoned lock is still used.
    write: Mutex<()>,
}

impl Logger {
    pub fn new(dir: impl Into<PathBuf>, max_bytes: u64) -> Self {
        Self {
            dir: dir.into(),
            max_bytes,
            level: AtomicU8::new(Level::Info as u8),
            write: Mutex::new(()),
        }
    }
    pub fn dir(&self) -> &Path {
        &self.dir
    }
    pub fn level(&self) -> Level {
        Level::from_u8(self.level.load(Ordering::Relaxed))
    }
    pub fn set_level(&self, level: Level) {
        self.level.store(level as u8, Ordering::Relaxed);
    }
    pub fn info(&self, message: &str) {
        self.line(Level::Info, message);
    }
    /// The message is only built when the level is `Debug`.
    pub fn debug(&self, message: impl FnOnce() -> String) {
        if self.level() == Level::Debug {
            self.line(Level::Debug, &message());
        }
    }
    /// One GitHub request: method, path without query, status and duration.
    pub fn request(&self, method: &str, url: &str, status: Option<u16>, started: Instant) {
        self.debug(|| {
            let status = status.map_or_else(|| "no-response".into(), |status| status.to_string());
            format!(
                "github {method} {} -> {status} {}ms",
                url_path(url),
                started.elapsed().as_millis()
            )
        });
    }
    /// A next link the paging refused, with both paths.
    pub fn paging_rejected(&self, reason: &str, first: &str, next: &str) {
        self.debug(|| {
            format!(
                "github paging rejected ({reason}): first={} next={}",
                url_path(first),
                url_path(next)
            )
        });
    }
    /// A finished listing: how many pages it took.
    pub fn pages(&self, first: &str, count: usize, truncated: bool) {
        self.debug(|| {
            format!(
                "github pages {}: count={count} truncated={truncated}",
                url_path(first)
            )
        });
    }
    fn line(&self, level: Level, message: &str) {
        let line = format!(
            "{} {} {}\n",
            chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            level.label().to_ascii_uppercase(),
            sanitise(message)
        );
        let _guard = self.write.lock().unwrap_or_else(PoisonError::into_inner);
        // Best effort: a full disk, a locked file or a missing directory loses the line only.
        let _ = self.append(line.as_bytes());
    }
    fn append(&self, line: &[u8]) -> std::io::Result<()> {
        fs::create_dir_all(&self.dir)?;
        let path = self.dir.join(FILE_NAME);
        let size = fs::metadata(&path).map_or(0, |file| file.len());
        if size > 0 && size.saturating_add(line.len() as u64) > self.max_bytes {
            // A predecessor that cannot be replaced (open elsewhere) must not let the file grow.
            if fs::rename(&path, self.dir.join(ROTATED_NAME)).is_err() {
                fs::File::create(&path)?;
            }
        }
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?
            .write_all(line)
    }
}

static GLOBAL: OnceLock<Logger> = OnceLock::new();
/// The level in effect, also when chosen before [`init`].
static LEVEL: AtomicU8 = AtomicU8::new(Level::Info as u8);

/// Starts the process-wide log in `dir`; a second call changes nothing.
pub fn init(dir: &Path) {
    let logger = Logger::new(dir, MAX_BYTES);
    logger.set_level(level());
    let _ = GLOBAL.set(logger);
}
/// The log directory once [`init`] ran.
pub fn dir() -> Option<&'static Path> {
    GLOBAL.get().map(Logger::dir)
}
pub fn level() -> Level {
    Level::from_u8(LEVEL.load(Ordering::Relaxed))
}
pub fn set_level(level: Level) {
    LEVEL.store(level as u8, Ordering::Relaxed);
    if let Some(logger) = GLOBAL.get() {
        logger.set_level(level);
    }
}
pub fn info(message: &str) {
    if let Some(logger) = GLOBAL.get() {
        logger.info(message);
    }
}
pub fn debug(message: impl FnOnce() -> String) {
    if let Some(logger) = GLOBAL.get() {
        logger.debug(message);
    }
}
pub fn request(method: &str, url: &str, status: Option<u16>, started: Instant) {
    if let Some(logger) = GLOBAL.get() {
        logger.request(method, url, status, started);
    }
}
pub fn paging_rejected(reason: &str, first: &str, next: &str) {
    if let Some(logger) = GLOBAL.get() {
        logger.paging_rejected(reason, first, next);
    }
}
pub fn pages(first: &str, count: usize, truncated: bool) {
    if let Some(logger) = GLOBAL.get() {
        logger.pages(first, count, truncated);
    }
}

#[cfg(test)]
mod tests;
