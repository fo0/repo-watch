//! The logger against temporary directories; every token below is a synthetic fixture.
use super::*;

/// A token-shaped fixture, never a real credential.
const FAKE_TOKEN: &str = "ghp_FIXTUREfixtureFIXTUREfixture000000";

struct TempLog {
    dir: PathBuf,
    logger: Logger,
}
impl TempLog {
    fn new(name: &str, max_bytes: u64) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "repowatch-log-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        Self {
            logger: Logger::new(dir.join(DIR_NAME), max_bytes),
            dir,
        }
    }
    fn read(&self, name: &str) -> String {
        fs::read_to_string(self.logger.dir().join(name)).unwrap_or_default()
    }
}
impl Drop for TempLog {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn nothing_is_written_before_init() {
    // The process-wide logger is never started in tests: every call is a no-op.
    assert!(dir().is_none());
    info("ignored");
    debug(|| unreachable!("a debug message is not built without a logger"));
    request("GET", "/user", Some(200), Instant::now());
    paging_rejected("leaves", "/a", "/b");
    pages("/a", 1, false);
}

#[test]
fn info_is_the_default_and_debug_lines_need_the_debug_level() {
    let log = TempLog::new("level", MAX_BYTES);
    assert_eq!(log.logger.level(), Level::Info);
    log.logger.info("first");
    log.logger
        .debug(|| unreachable!("not built at the info level"));
    log.logger.set_level(Level::Debug);
    log.logger.debug(|| "second".into());
    log.logger.set_level(Level::Info);
    log.logger.debug(|| "third".into());
    let text = log.read(FILE_NAME);
    let lines: Vec<_> = text.lines().collect();
    assert_eq!(lines.len(), 2, "{text}");
    assert!(lines[0].ends_with(" INFO first"), "{text}");
    assert!(lines[1].ends_with(" DEBUG second"), "{text}");
    // RFC 3339 in UTC with milliseconds, then level and message.
    let stamp = lines[0].split(' ').next().unwrap();
    assert!(stamp.ends_with('Z') && chrono::DateTime::parse_from_rfc3339(stamp).is_ok());
}

#[test]
fn the_file_rotates_at_the_cap_and_keeps_one_predecessor() {
    let log = TempLog::new("rotate", 200);
    for index in 0..12 {
        log.logger.info(&format!("line {index} {}", "x".repeat(40)));
    }
    let current = log.read(FILE_NAME);
    let rotated = log.read(ROTATED_NAME);
    assert!(current.len() as u64 <= 200 && rotated.len() as u64 <= 200);
    assert!(current.contains("line 11 "), "{current}");
    assert!(!rotated.is_empty() && !rotated.contains("line 0 "));
    let names: Vec<_> = fs::read_dir(log.logger.dir())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(names.len(), 2, "{names:?}");
}

#[test]
fn a_request_line_carries_the_path_only() {
    let log = TempLog::new("request", MAX_BYTES);
    log.logger.set_level(Level::Debug);
    log.logger.request(
        "GET",
        &format!(
            "https://user:{FAKE_TOKEN}@api.github.com/repos/fixture-owner/fixture-1/actions/runs?per_page=100&access_token={FAKE_TOKEN}#frag"
        ),
        Some(200),
        Instant::now(),
    );
    log.logger
        .request("POST", "/repos/o/r/dispatches", None, Instant::now());
    log.logger.paging_rejected(
        "leaves",
        "/repos/fixture-owner/fixture-1/tags?page=1",
        "/repositories/7/branches?page=2",
    );
    log.logger
        .pages("/repos/fixture-owner/fixture-1/tags?page=1", 3, true);
    let text = log.read(FILE_NAME);
    assert!(
        text.contains(" DEBUG github GET /repos/fixture-owner/fixture-1/actions/runs -> 200 "),
        "{text}"
    );
    assert!(text.contains("github POST /repos/o/r/dispatches -> no-response "));
    assert!(text.contains(
        "github paging rejected (leaves): first=/repos/fixture-owner/fixture-1/tags next=/repositories/7/branches\n"
    ));
    assert!(
        text.contains("github pages /repos/fixture-owner/fixture-1/tags: count=3 truncated=true\n")
    );
    assert!(!text.contains("ghp_") && !text.contains("per_page") && !text.contains('?'));
}

#[test]
fn a_credential_looking_word_never_reaches_the_file() {
    let log = TempLog::new("redact", MAX_BYTES);
    log.logger.set_level(Level::Debug);
    // Even a token in the path, in a message or after a scheme word is replaced.
    log.logger.request(
        "GET",
        &format!("/repos/o/{FAKE_TOKEN}/runs"),
        Some(404),
        Instant::now(),
    );
    log.logger.info(&format!("refresh failed: {FAKE_TOKEN}"));
    log.logger
        .info("header Authorization: Bearer fixture-opaque-value end");
    log.logger
        .info("github_pat_FIXTURE token=fixture client_secret=fixture");
    log.logger
        .info("-----BEGIN PRIVATE KEY-----\nfixture\n-----END PRIVATE KEY-----");
    log.logger.info("two\nlines\tand\u{7} a bell");
    let text = log.read(FILE_NAME);
    assert!(
        !text.contains("FIXTURE") && !text.contains("fixture"),
        "{text}"
    );
    assert!(text.contains("Bearer [redacted] end\n"), "{text}");
    assert!(text.contains(" INFO [redacted]\n"));
    assert!(text.contains(" INFO two lines and a bell\n"), "{text}");
    assert_eq!(text.lines().count(), 6, "{text}");
}

#[test]
fn a_long_message_is_cut() {
    let cut = sanitise(&"a".repeat(MAX_MESSAGE_CHARS + 50));
    assert_eq!(cut.chars().count(), MAX_MESSAGE_CHARS + 1);
    assert!(cut.ends_with('…'));
}

#[test]
fn an_unwritable_directory_is_swallowed() {
    let log = TempLog::new("blocked", MAX_BYTES);
    // A file where the directory should be: every write fails, none panics.
    fs::create_dir_all(&log.dir).unwrap();
    fs::write(log.logger.dir(), b"not a directory").unwrap();
    log.logger.info("lost");
    assert_eq!(log.read(FILE_NAME), "");
}

#[test]
fn a_poisoned_lock_still_writes() {
    let log = TempLog::new("poison", MAX_BYTES);
    let _ = std::thread::scope(|scope| {
        scope
            .spawn(|| {
                let _guard = log.logger.write.lock().unwrap();
                panic!("poison the log lock (expected in this test)");
            })
            .join()
    });
    assert!(log.logger.write.is_poisoned());
    log.logger.info("after");
    assert!(log.read(FILE_NAME).ends_with(" INFO after\n"));
}

#[test]
fn url_path_drops_origin_query_and_fragment() {
    assert_eq!(url_path("https://api.github.com/user?x=1"), "/user");
    assert_eq!(url_path("https://api.github.com"), "/");
    assert_eq!(url_path("/repos/o/r/tags#a?b"), "/repos/o/r/tags");
    assert_eq!(url_path("/plain"), "/plain");
}

#[test]
fn the_level_reads_its_ipc_names_and_tolerates_a_foreign_stored_value() {
    assert_eq!(serde_json::to_string(&Level::Debug).unwrap(), "\"debug\"");
    assert_eq!(
        serde_json::from_str::<Level>("\"info\"").unwrap(),
        Level::Info
    );
    assert!(serde_json::from_str::<Level>("\"trace\"").is_err());
    #[derive(Deserialize)]
    struct Row {
        #[serde(default, deserialize_with = "lenient_level")]
        level: Option<Level>,
    }
    let level = |json: &str| serde_json::from_str::<Row>(json).unwrap().level;
    assert_eq!(level(r#"{"level":"debug"}"#), Some(Level::Debug));
    assert_eq!(level(r#"{"level":"trace"}"#), None);
    assert_eq!(level(r#"{"level":7}"#), None);
    assert_eq!(level("{}"), None);
}
