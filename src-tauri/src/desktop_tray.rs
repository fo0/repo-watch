//! Tray menu labels and status tooltip in the app's language. The panel reports the language
//! with `set_locale`; the menu items stay reachable here so they can be relabelled at once.
use repowatch_core::i18n::{self, tr, Locale};
use std::sync::atomic::{AtomicU8, Ordering};
use tauri::{
    menu::{CheckMenuItem, MenuItem},
    Manager, Wry,
};

pub(crate) const TRAY_ID: &str = "repowatch";

/// The status the tooltip reports; `UNKNOWN` until the first refresh settles.
const UNKNOWN: u8 = 0;
const RUNNING: u8 = 1;
const FAILED: u8 = 2;
const READY: u8 = 3;

pub(crate) fn show_label() -> &'static str {
    tr("RepoWatch öffnen / ausblenden", "Show / hide RepoWatch")
}

pub(crate) fn notifications_label() -> &'static str {
    tr("Run-Benachrichtigungen", "Run notifications")
}

pub(crate) fn log_folder_label() -> &'static str {
    tr("Log-Ordner öffnen", "Open log folder")
}

pub(crate) fn quit_label() -> &'static str {
    tr("RepoWatch beenden", "Quit RepoWatch")
}

fn tooltip(status: u8) -> &'static str {
    match status {
        RUNNING => tr(
            "RepoWatch · Workflows laufen",
            "RepoWatch · workflows running",
        ),
        FAILED => tr("RepoWatch · Fehler", "RepoWatch · Error"),
        READY => tr("RepoWatch · bereit", "RepoWatch · ready"),
        _ => "RepoWatch",
    }
}

/// Managed state: the tray's menu items and the status its tooltip last reported.
pub(crate) struct TrayMenu {
    show: MenuItem<Wry>,
    notifications: CheckMenuItem<Wry>,
    log_folder: MenuItem<Wry>,
    quit: MenuItem<Wry>,
    status: AtomicU8,
}

impl TrayMenu {
    pub(crate) fn new(
        show: MenuItem<Wry>,
        notifications: CheckMenuItem<Wry>,
        log_folder: MenuItem<Wry>,
        quit: MenuItem<Wry>,
    ) -> Self {
        Self {
            show,
            notifications,
            log_folder,
            quit,
            status: AtomicU8::new(UNKNOWN),
        }
    }
}

/// Sets the tooltip for the worker's latest snapshot and remembers it for a relabel.
pub(crate) fn show_status(app: &tauri::AppHandle, active: bool, failed: bool) {
    let status = if active {
        RUNNING
    } else if failed {
        FAILED
    } else {
        READY
    };
    if let Some(menu) = app.try_state::<TrayMenu>() {
        menu.status.store(status, Ordering::Relaxed);
    }
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_tooltip(Some(tooltip(status)));
    }
}

/// Switches native text (errors, notifications, tray) to the panel's language and relabels
/// the tray at once. Best effort: a label that cannot be set keeps its old wording.
#[tauri::command]
pub(crate) fn set_locale(app: tauri::AppHandle, locale: Locale) {
    i18n::set_locale(locale);
    let Some(menu) = app.try_state::<TrayMenu>() else {
        return;
    };
    let _ = menu.show.set_text(show_label());
    let _ = menu.notifications.set_text(notifications_label());
    let _ = menu.log_folder.set_text(log_folder_label());
    let _ = menu.quit.set_text(quit_label());
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_tooltip(Some(tooltip(menu.status.load(Ordering::Relaxed))));
    }
}
