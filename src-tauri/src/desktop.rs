use crate::desktop_tray::{self, TrayMenu, TRAY_ID};
use repowatch_core::{
    app_log,
    auth::{DeviceFlow, OsKeyring},
    cache::Cache,
    i18n::tr,
    live::LiveStore,
    DispatchOutcome, DispatchRequest, Pin, RunStatus, Snapshot, UncertainStart,
};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
};
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem},
    tray::TrayIconBuilder,
    Emitter, Manager,
};
use tauri_plugin_notification::NotificationExt;
pub(crate) struct NativeState {
    pub(crate) engine: Mutex<LiveStore>,
    notifications: AtomicBool,
    /// The saved refresh interval in seconds; the worker reads it on every wait tick.
    refresh_seconds: AtomicU64,
    /// Whether the last snapshot, from the worker or any command, shows an open run; the
    /// worker reads it on every wait tick, so a run started from the panel is followed at
    /// the 15 s pace without waiting out the idle interval.
    open_run: AtomicBool,
    pub(crate) auth_generation: AtomicU64,
    pub(crate) browser_flow: Mutex<(u64, DeviceFlow)>,
}
pub(crate) async fn work<T: Send + 'static>(
    app: tauri::AppHandle,
    operation: impl FnOnce(&mut LiveStore) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<NativeState>();
        let mut engine = state
            .engine
            .lock()
            .map_err(|_| tr("Dienst nicht verfügbar.", "Service unavailable."))?;
        let result = operation(&mut engine);
        state
            .open_run
            .store(engine.has_open_run(), Ordering::Relaxed);
        result
    })
    .await
    .map_err(|_| {
        tr(
            "Aufgabe konnte nicht abgeschlossen werden.",
            "Task could not be completed.",
        )
        .to_string()
    })?
}
#[tauri::command]
async fn snapshot(app: tauri::AppHandle) -> Result<Snapshot, String> {
    work(app, |engine| Ok(engine.snapshot())).await
}
/// F5 and the refresh button: every repository polls its runs now, also an idle one.
#[tauri::command]
async fn refresh_snapshot(app: tauri::AppHandle) -> Result<Snapshot, String> {
    work(app, |engine| Ok(engine.refresh_now())).await
}
#[tauri::command]
async fn refresh_repositories(app: tauri::AppHandle) -> Result<Snapshot, String> {
    work(app, |engine| Ok(engine.refresh_repositories())).await
}
/// Read without the engine lock, which a running refresh may hold.
#[tauri::command]
fn refresh_interval(state: tauri::State<'_, NativeState>) -> u64 {
    state.refresh_seconds.load(Ordering::Relaxed)
}
/// Validates and stores the interval; the worker picks it up within a few seconds.
#[tauri::command]
async fn set_refresh_interval(app: tauri::AppHandle, seconds: u64) -> Result<u64, String> {
    let handle = app.clone();
    let seconds = work(app, move |engine| engine.set_refresh_interval(seconds)).await?;
    if let Some(state) = handle.try_state::<NativeState>() {
        state.refresh_seconds.store(seconds, Ordering::Relaxed);
    }
    Ok(seconds)
}
/// Waits until the worker's next refresh is due. The interval and the open-run state are
/// read again every few seconds without the engine lock, so a saved change or a run started
/// from the panel applies without waiting out the old interval.
fn wait_for_refresh(app: &tauri::AppHandle) {
    use repowatch_core::refresh_interval::{next_sleep, DEFAULT_SECONDS};
    let started = std::time::Instant::now();
    loop {
        let (refresh, open_run) =
            app.try_state::<NativeState>()
                .map_or((DEFAULT_SECONDS, false), |state| {
                    (
                        state.refresh_seconds.load(Ordering::Relaxed),
                        state.open_run.load(Ordering::Relaxed),
                    )
                });
        match next_sleep(started.elapsed(), open_run, refresh) {
            Some(sleep) => std::thread::sleep(sleep),
            None => return,
        }
    }
}
#[tauri::command]
async fn save_pin(app: tauri::AppHandle, pin: Pin) -> Result<Snapshot, String> {
    work(app, |engine| engine.save_pin(pin)).await
}
#[tauri::command]
async fn delete_pin(app: tauri::AppHandle, pin_id: String) -> Result<Snapshot, String> {
    work(app, |engine| engine.delete_pin(pin_id)).await
}
#[tauri::command]
async fn dispatch_workflow(
    app: tauri::AppHandle,
    request: DispatchRequest,
) -> Result<DispatchOutcome, String> {
    work(app, |engine| Ok(engine.dispatch(request))).await
}
/// Called only after the user confirmed they checked the runs on GitHub; it never dispatches.
#[tauri::command]
async fn release_uncertain_start(
    app: tauri::AppHandle,
    start: UncertainStart,
) -> Result<Snapshot, String> {
    work(app, |engine| engine.release_uncertain_start(start)).await
}
/// One POST per click; the next poll reports whether GitHub cancelled the run.
#[tauri::command]
async fn cancel_run(
    app: tauri::AppHandle,
    repo_id: String,
    run_id: u64,
) -> Result<Snapshot, String> {
    work(app, move |engine| engine.cancel_run(repo_id, run_id)).await
}
/// Hides a repository from the panel lists or shows it again; the selection applies at once.
#[tauri::command]
async fn set_repo_hidden(
    app: tauri::AppHandle,
    account_id: String,
    repo_id: String,
    hidden: bool,
) -> Result<Snapshot, String> {
    work(app, move |engine| {
        engine.set_repo_hidden(account_id, repo_id, hidden)
    })
    .await
}
/// Sets the repository owners an account watches; no owners watches all of them again. The
/// selection applies at once.
#[tauri::command]
async fn set_owner_filter(
    app: tauri::AppHandle,
    account_id: String,
    owners: Vec<String>,
) -> Result<Snapshot, String> {
    work(app, move |engine| {
        engine.set_owner_filter(account_id, owners)
    })
    .await
}
#[tauri::command]
async fn open_repository(app: tauri::AppHandle, repo_id: String) -> Result<Snapshot, String> {
    work(app, |engine| engine.open_repository(repo_id)).await
}
#[tauri::command]
fn hide_panel(app: tauri::AppHandle) -> Result<(), String> {
    crate::tray_popup::hide(&app)
}
#[tauri::command]
fn open_run(app: tauri::AppHandle, url: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    // Open only the parser's canonical form, never the raw IPC string.
    let url = repowatch_core::run_link::canonical_run_url(&url).ok_or(tr(
        "Nur GitHub-Run-Links sind erlaubt.",
        "Only GitHub run links are allowed.",
    ))?;
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|_| browser_failed().into())
}
#[tauri::command]
fn open_repo(app: tauri::AppHandle, url: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    // Open only the parser's canonical form, never the raw IPC string.
    let url = repowatch_core::run_link::canonical_repo_url(&url).ok_or(tr(
        "Nur GitHub-Repository-Links sind erlaubt.",
        "Only GitHub repository links are allowed.",
    ))?;
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|_| browser_failed().into())
}
fn browser_failed() -> &'static str {
    tr(
        "Browser konnte nicht geöffnet werden.",
        "Browser could not be opened.",
    )
}
fn tray_image(active: bool, failed: bool) -> tauri::image::Image<'static> {
    let color = if active {
        [197, 237, 139, 255]
    } else if failed {
        [242, 150, 124, 255]
    } else {
        [143, 165, 148, 255]
    };
    let mut pixels = vec![0; 32 * 32 * 4];
    for y in 3..29 {
        for x in 3..29 {
            let offset = (y * 32 + x) * 4;
            let glyph = ((8..12).contains(&x) && (8..25).contains(&y))
                || ((9..24).contains(&x) && (15..19).contains(&y))
                || ((20..24).contains(&x) && (8..18).contains(&y));
            pixels[offset..offset + 4].copy_from_slice(if glyph {
                &[20, 24, 22, 255]
            } else {
                &color
            });
        }
    }
    tauri::image::Image::new_owned(pixels, 32, 32)
}

pub fn run() {
    let show_panel = repowatch_core::startup::show_on_launch(std::env::args_os());
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _| {
            // A duplicate login launch must neither show nor hide the existing panel.
            if repowatch_core::startup::show_on_launch(args) {
                if let Err(error) = crate::tray_popup::show(app) {
                    crate::desktop_log::popup_failed(&error);
                }
            }
        }))
        .manage(crate::desktop_startup::StartupState::default())
        .manage(crate::tray_popup::Popup::default())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(crate::desktop_updates::UpdateState::default())
        .setup(move |app| {
            let cache_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&cache_dir)?;
            // Started before the engine, which applies the stored level.
            app_log::init(&cache_dir.join(app_log::DIR_NAME));
            let cache = Cache::open(cache_dir.join("repowatch-live.db"))?;
            let store =
                LiveStore::new(cache, Arc::new(OsKeyring::new())).map_err(std::io::Error::other)?;
            app_log::info(&format!(
                "app start: version={} level={}",
                app.package_info().version,
                app_log::level().label()
            ));
            let baseline = store.snapshot();
            let notify = store.notifications_enabled();
            let refresh_seconds = store.refresh_interval();
            app.manage(NativeState {
                engine: Mutex::new(store),
                notifications: AtomicBool::new(notify),
                refresh_seconds: AtomicU64::new(refresh_seconds),
                open_run: AtomicBool::new(false),
                auth_generation: AtomicU64::new(0),
                browser_flow: Mutex::new((0, DeviceFlow::new())),
            });
            let show = MenuItem::with_id(
                app,
                "toggle",
                desktop_tray::show_label(),
                true,
                None::<&str>,
            )?;
            let notifications = CheckMenuItem::with_id(
                app,
                "notifications",
                desktop_tray::notifications_label(),
                true,
                notify,
                None::<&str>,
            )?;
            let log_folder = MenuItem::with_id(
                app,
                "log-folder",
                desktop_tray::log_folder_label(),
                true,
                None::<&str>,
            )?;
            let quit =
                MenuItem::with_id(app, "quit", desktop_tray::quit_label(), true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &notifications, &log_folder, &quit])?;
            app.manage(TrayMenu::new(
                show.clone(),
                notifications.clone(),
                log_folder.clone(),
                quit.clone(),
            ));
            TrayIconBuilder::with_id(TRAY_ID)
                .icon(tray_image(false, false))
                .tooltip("RepoWatch")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "toggle" => crate::tray_popup::toggle(app),
                    "quit" => app.exit(0),
                    "log-folder" => {
                        if let Err(error) = crate::desktop_log::open_folder(app) {
                            app_log::info(&format!("log folder: {error}"));
                            // The tray has no surface of its own; tell the user who clicked.
                            let _ = app
                                .notification()
                                .builder()
                                .title("RepoWatch")
                                .body(error)
                                .show();
                        }
                    }
                    "notifications" => {
                        if let Some(state) = app.try_state::<NativeState>() {
                            let enabled = !state.notifications.load(Ordering::Relaxed);
                            state.notifications.store(enabled, Ordering::Relaxed);
                            let _ = notifications.set_checked(enabled);
                            // Persist off the UI thread; the engine lock may be held by a refresh.
                            // The value is read under the lock, so the last write wins.
                            let handle = app.clone();
                            tauri::async_runtime::spawn_blocking(move || {
                                let Some(state) = handle.try_state::<NativeState>() else {
                                    return;
                                };
                                // A `let` binding drops the guard before `state`; a trailing
                                // `if let` expression would outlive it (E0597).
                                let Ok(mut engine) = state.engine.lock() else {
                                    return;
                                };
                                // Best effort: the toggle still applies to this session.
                                let _ = engine.set_notifications_enabled(
                                    state.notifications.load(Ordering::Relaxed),
                                );
                            });
                        }
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    crate::tray_popup::tray_event(tray.app_handle(), event);
                })
                .build(app)?;
            // Keep visible:false throughout a login launch; there is no show/hide flash.
            if show_panel {
                crate::tray_popup::show(app.handle()).map_err(std::io::Error::other)?;
            }
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let mut previous: HashMap<_, _> = baseline
                    .runs
                    .iter()
                    .map(|run| (run.id, run.status))
                    .collect();
                // One serialized worker. Its first wait is a few seconds, so the panel's
                // first `snapshot` gets the cached state before the heaviest refresh takes
                // the engine lock, yet a long interval never keeps it stale; then every
                // 15 s while a run is open, otherwise at the user's refresh interval.
                if let Some(state) = handle.try_state::<NativeState>() {
                    std::thread::sleep(repowatch_core::refresh_interval::startup_wait(
                        state.refresh_seconds.load(Ordering::Relaxed),
                    ));
                }
                while let Some(state) = handle.try_state::<NativeState>() {
                    let value = {
                        let Ok(mut engine) = state.engine.lock() else {
                            break;
                        };
                        let value = engine.refresh();
                        state
                            .open_run
                            .store(engine.has_open_run(), Ordering::Relaxed);
                        value
                    };
                    let selected = |repo_id: &str| {
                        value
                            .repos
                            .iter()
                            .any(|repo| repo.id == repo_id && repo.selected)
                    };
                    let active = value
                        .runs
                        .iter()
                        .any(|run| selected(&run.repo_id) && run.status.is_open());
                    let failed = value.runs.iter().any(|run| {
                        selected(&run.repo_id)
                            && run.conclusion == Some(repowatch_core::Conclusion::Failure)
                    });
                    if let Some(tray) = handle.tray_by_id(TRAY_ID) {
                        let _ = tray.set_icon(Some(tray_image(active, failed)));
                    }
                    desktop_tray::show_status(&handle, active, failed);
                    for run in &value.runs {
                        if selected(&run.repo_id)
                            && run.status == RunStatus::Completed
                            && previous
                                .get(&run.id)
                                .is_some_and(|status| *status != RunStatus::Completed)
                            && state.notifications.load(Ordering::Relaxed)
                        {
                            let (title, body) =
                                repowatch_core::run_notice::completion_notice(run, &value.repos);
                            let _ = handle
                                .notification()
                                .builder()
                                .title(title)
                                .body(body)
                                .show();
                        }
                    }
                    previous = value.runs.iter().map(|run| (run.id, run.status)).collect();
                    let _ = handle.emit("repowatch:snapshot", value);
                    wait_for_refresh(&handle);
                }
            });
            Ok(())
        })
        .on_window_event(crate::tray_popup::window_event)
        .invoke_handler(tauri::generate_handler![
            snapshot,
            refresh_snapshot,
            refresh_repositories,
            refresh_interval,
            set_refresh_interval,
            crate::desktop_tray::set_locale,
            crate::desktop_log::log_level,
            crate::desktop_log::set_log_level,
            crate::desktop_log::open_log_folder,
            save_pin,
            delete_pin,
            dispatch_workflow,
            release_uncertain_start,
            cancel_run,
            open_repository,
            set_repo_hidden,
            set_owner_filter,
            crate::desktop_auth::connect_token,
            crate::desktop_auth::browser_auth_config,
            crate::desktop_auth::start_browser_auth,
            crate::desktop_auth::poll_browser_auth,
            crate::desktop_auth::open_browser_auth,
            crate::desktop_auth::open_browser_installation,
            crate::desktop_auth::discover_local_auth,
            crate::desktop_auth::connect_local_auth,
            crate::desktop_auth::cancel_connect,
            crate::desktop_auth::disconnect_account,
            crate::desktop_startup::autostart_status,
            crate::desktop_startup::set_autostart,
            crate::desktop_startup::open_startup_settings,
            crate::desktop_updates::check_update,
            crate::desktop_updates::install_update,
            open_run,
            open_repo,
            hide_panel
        ])
        .run(tauri::generate_context!())
        .expect("RepoWatch konnte nicht gestartet werden");
}
