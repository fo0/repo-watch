//! The app-wide settings row: notifications, the refresh interval, hidden repositories and
//! owner filters.
use super::super::*;
use super::{remove_db, temp_db};

#[test]
fn app_settings_persist_and_the_reserved_key_is_not_an_account() {
    let path = temp_db("settings");
    {
        let cache = Cache::open(&path).unwrap();
        assert!(!cache.load_app_settings().unwrap().notifications);
        cache
            .save_app_settings(&AppSettings {
                notifications: true,
                ..AppSettings::default()
            })
            .unwrap();
    }
    let mut cache = Cache::open(&path).unwrap();
    assert!(cache.load_app_settings().unwrap().notifications);
    assert!(cache.delete_account(APP_SETTINGS_KEY).is_err());
    assert!(cache.load_app_settings().unwrap().notifications);
    drop(cache);
    remove_db(&path);
}

#[test]
fn the_refresh_interval_round_trips_and_an_older_row_keeps_the_default() {
    let cache = Cache::in_memory().unwrap();
    // A row written before the setting existed, with a field from a newer build.
    cache
        .write_settings_row(APP_SETTINGS_KEY, r#"{"notifications":true,"future":1}"#)
        .unwrap();
    let older = cache.load_app_settings().unwrap();
    assert!(older.notifications);
    assert_eq!(older.refresh_seconds, None);
    assert_eq!(older.refresh_interval(), 60);
    // An absent value is not written, so an older build reads the row unchanged.
    assert!(!encode(&older).unwrap().contains("refresh"));
    let chosen = AppSettings {
        refresh_seconds: Some(300),
        ..older
    };
    cache.save_app_settings(&chosen).unwrap();
    let loaded = cache.load_app_settings().unwrap();
    assert_eq!(loaded, chosen);
    assert_eq!(loaded.refresh_interval(), 300);
    // A foreign value outside the range is clamped, never used as is.
    cache
        .write_settings_row(APP_SETTINGS_KEY, r#"{"refreshSeconds":1}"#)
        .unwrap();
    assert_eq!(cache.load_app_settings().unwrap().refresh_interval(), 15);
}

#[test]
fn hidden_repositories_round_trip_and_an_older_row_hides_nothing() {
    let cache = Cache::in_memory().unwrap();
    // A row from before hidden repositories existed.
    cache
        .write_settings_row(
            APP_SETTINGS_KEY,
            r#"{"notifications":true,"refreshSeconds":300}"#,
        )
        .unwrap();
    let older = cache.load_app_settings().unwrap();
    assert!(older.hidden_repos.is_empty());
    // An empty list is not written, so an older build reads the row unchanged.
    assert!(!encode(&older).unwrap().contains("hidden"));
    let hidden = AppSettings {
        hidden_repos: vec![HiddenRepo {
            account_id: "github-1".into(),
            repo_id: "github-1:7".into(),
            owner: "fixture-owner".into(),
            name: "fixture-7".into(),
        }],
        ..older
    };
    cache.save_app_settings(&hidden).unwrap();
    let loaded = cache.load_app_settings().unwrap();
    assert_eq!(loaded, hidden);
    assert!(loaded.notifications && loaded.refresh_interval() == 300);
    assert!(encode(&loaded)
        .unwrap()
        .contains(r#""hiddenRepos":[{"accountId""#));
}

#[test]
fn owner_filters_round_trip_and_an_older_row_filters_nothing() {
    let cache = Cache::in_memory().unwrap();
    // A row from before owner filters existed.
    cache
        .write_settings_row(
            APP_SETTINGS_KEY,
            r#"{"notifications":true,"refreshSeconds":300}"#,
        )
        .unwrap();
    let older = cache.load_app_settings().unwrap();
    assert!(older.owner_filters.is_empty());
    // An empty list is not written, so an older build reads the row unchanged.
    assert!(!encode(&older).unwrap().contains("owner"));
    let filtered = AppSettings {
        owner_filters: vec![OwnerFilter {
            account_id: "github-1".into(),
            owners: vec!["fixture-owner".into(), "fixture-org".into()],
        }],
        ..older
    };
    cache.save_app_settings(&filtered).unwrap();
    let loaded = cache.load_app_settings().unwrap();
    assert_eq!(loaded, filtered);
    assert!(loaded.notifications && loaded.refresh_interval() == 300);
    assert!(encode(&loaded).unwrap().contains(
        r#""ownerFilters":[{"accountId":"github-1","owners":["fixture-owner","fixture-org"]}]"#
    ));
}

#[test]
fn the_log_level_round_trips_and_an_older_or_foreign_row_keeps_info() {
    use crate::app_log::Level;
    let cache = Cache::in_memory().unwrap();
    // A row from before the setting existed.
    cache
        .write_settings_row(
            APP_SETTINGS_KEY,
            r#"{"notifications":true,"refreshSeconds":300}"#,
        )
        .unwrap();
    let older = cache.load_app_settings().unwrap();
    assert_eq!(older.log_level, None);
    // An absent value is not written, so an older build reads the row unchanged.
    assert!(!encode(&older).unwrap().contains("logLevel"));
    let debug = AppSettings {
        log_level: Some(Level::Debug),
        ..older
    };
    cache.save_app_settings(&debug).unwrap();
    let loaded = cache.load_app_settings().unwrap();
    assert_eq!(loaded, debug);
    assert!(encode(&loaded).unwrap().contains(r#""logLevel":"debug""#));
    // A level from a newer build reads as info and never costs the other settings.
    cache
        .write_settings_row(
            APP_SETTINGS_KEY,
            r#"{"refreshSeconds":300,"logLevel":"trace"}"#,
        )
        .unwrap();
    let foreign = cache.load_app_settings().unwrap();
    assert_eq!(foreign.log_level, None);
    assert_eq!(foreign.refresh_interval(), 300);
}
