//! The local log from the shell: the level commands of Settings → App „Protokoll“ and
//! opening the log folder (panel button and tray entry). The log itself is
//! `repowatch_core::app_log`.
use repowatch_core::{
    app_log::{self, Level},
    i18n::tr,
};

/// Read without the engine lock, which a running refresh may hold.
#[tauri::command]
pub(crate) fn log_level() -> Level {
    app_log::level()
}

/// Stores the level; it applies to every line written from now on.
#[tauri::command]
pub(crate) async fn set_log_level(app: tauri::AppHandle, level: Level) -> Result<Level, String> {
    crate::desktop::work(app, move |engine| engine.set_log_level(level)).await
}

#[tauri::command]
pub(crate) fn open_log_folder(app: tauri::AppHandle) -> Result<(), String> {
    open_folder(&app)
}

/// Creates the log folder if it is missing and shows it in the file manager.
pub(crate) fn open_folder(app: &tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let failed = || {
        tr(
            "Log-Ordner konnte nicht geöffnet werden.",
            "Log folder could not be opened.",
        )
        .to_string()
    };
    let dir = app_log::dir().ok_or_else(failed)?;
    std::fs::create_dir_all(dir).map_err(|_| failed())?;
    app.opener()
        .open_path(dir.to_string_lossy().into_owned(), None::<&str>)
        .map_err(|_| failed())
}

/// A popup that could not be shown or hidden: stderr for a development run, the log otherwise.
pub(crate) fn popup_failed(error: &str) {
    eprintln!("RepoWatch popup: {error}");
    app_log::info(&format!("popup: {error}"));
}
