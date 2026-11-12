//! Main-window lifecycle. Hiding never destroys the webview or cancels a form,
//! dispatch or browser authentication operation.
use repowatch_core::i18n::tr;
use repowatch_core::popup::{DismissAction, Dismissal, Rect, TrayGesture};
use std::{sync::Mutex, time::Duration};
#[cfg(not(windows))]
use tauri::PhysicalPosition;
use tauri::{tray::TrayIconEvent, Manager, WindowEvent};

#[cfg(windows)]
mod windows;

#[derive(Default)]
struct State {
    dismissal: Dismissal,
    anchor: Option<Rect>,
    gesture: TrayGesture,
    #[cfg(windows)]
    preferred_size: Option<(f64, f64)>,
    #[cfg(windows)]
    applied_size: Option<(u32, u32)>,
}

#[derive(Default)]
pub struct Popup(Mutex<State>);

fn invalidate(app: &tauri::AppHandle) -> Option<u64> {
    let popup = app.try_state::<Popup>()?;
    let mut state = popup.0.lock().ok()?;
    Some(state.dismissal.invalidate())
}

fn physical_rect(rect: tauri::Rect) -> Rect {
    // Windows tray-icon returns physical coordinates from Shell_NotifyIconGetRect.
    let position = rect.position.to_physical::<i32>(1.0);
    let size = rect.size.to_physical::<u32>(1.0);
    Rect {
        x: position.x,
        y: position.y,
        width: size.width,
        height: size.height,
    }
}

fn anchor(app: &tauri::AppHandle) -> Option<Rect> {
    app.tray_by_id("repowatch")
        .and_then(|tray| tray.rect().ok().flatten())
        .map(physical_rect)
        .filter(|rect| rect.width > 0 && rect.height > 0)
}

pub fn hide(app: &tauri::AppHandle) -> Result<(), String> {
    invalidate(app);
    app.get_webview_window("main")
        .ok_or(panel_missing())?
        .hide()
        .map_err(|_| {
            tr(
                "Panel konnte nicht ausgeblendet werden.",
                "Panel could not be hidden.",
            )
            .into()
        })
}

fn panel_missing() -> &'static str {
    tr("Panel fehlt.", "Panel is missing.")
}

fn position(window: &tauri::WebviewWindow, icon: Option<Rect>) -> tauri::Result<()> {
    #[cfg(windows)]
    {
        let hwnd = window.hwnd()?.0 as _;
        // Temporarily relax the minimum so a high-DPI/small work area can fit the
        // panel. The frontend already scrolls; keeping 560 logical px would clip it.
        window.set_min_size(None::<tauri::LogicalSize<f64>>)?;
        let preferred = window
            .app_handle()
            .state::<Popup>()
            .0
            .lock()
            .ok()
            .and_then(|state| state.preferred_size);
        let placement = windows::position(hwnd, icon, preferred).map_err(tauri::Error::Io)?;
        if let Ok(mut state) = window.app_handle().state::<Popup>().0.lock() {
            state.preferred_size = Some(placement.logical_size);
            state.applied_size = Some((placement.frame.width, placement.frame.height));
        }
        window.set_min_size(Some(tauri::LogicalSize::new(
            360.0_f64.min(f64::from(placement.work.width) / placement.scale),
            560.0_f64.min(f64::from(placement.work.height) / placement.scale),
        )))?;
    }
    #[cfg(not(windows))]
    {
        if let Some(icon) = icon {
            let point = PhysicalPosition::new(icon.x, icon.y);
            if let Some(monitor) =
                window.monitor_from_point(f64::from(point.x), f64::from(point.y))?
            {
                let work = monitor.work_area();
                let bounds = Rect {
                    x: monitor.position().x,
                    y: monitor.position().y,
                    width: monitor.size().width,
                    height: monitor.size().height,
                };
                let work = Rect {
                    x: work.position.x,
                    y: work.position.y,
                    width: work.size.width,
                    height: work.size.height,
                };
                let size = window.outer_size()?;
                let frame =
                    repowatch_core::popup::place(icon, bounds, work, (size.width, size.height), 6);
                window.set_position(PhysicalPosition::new(frame.x, frame.y))?;
            }
        }
    }
    Ok(())
}

pub fn show(app: &tauri::AppHandle) -> Result<(), String> {
    show_at(app, anchor(app))
}

fn show_at(app: &tauri::AppHandle, icon: Option<Rect>) -> Result<(), String> {
    invalidate(app);
    let window = app.get_webview_window("main").ok_or(panel_missing())?;
    // Position while hidden, before restoring focus. Every open re-queries the
    // icon and its monitor; a remembered desktop position is never authoritative.
    if window.is_minimized().unwrap_or(false) {
        window.unminimize().map_err(|e| e.to_string())?;
    }
    position(&window, icon).map_err(|e| e.to_string())?;
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    // Also covers an OS-denied focus request (there may be no Focused(false)).
    schedule_dismiss(app);
    Ok(())
}

pub fn toggle(app: &tauri::AppHandle) {
    toggle_at(app, anchor(app));
}

fn toggle_at(app: &tauri::AppHandle, icon: Option<Rect>) {
    let visible = app.get_webview_window("main").is_some_and(|window| {
        window.is_visible().unwrap_or(false) && !window.is_minimized().unwrap_or(false)
    });
    let result = if visible {
        hide(app)
    } else {
        show_at(app, icon)
    };
    if let Err(error) = result {
        eprintln!("RepoWatch popup: {error}");
    }
}

pub fn tray_event(app: &tauri::AppHandle, event: TrayIconEvent) {
    use tauri::tray::{MouseButton, MouseButtonState};
    let (rect, state, double) = match event {
        TrayIconEvent::Click {
            rect,
            button: MouseButton::Left,
            button_state,
            ..
        } => (rect, button_state, false),
        TrayIconEvent::DoubleClick {
            rect,
            button: MouseButton::Left,
            ..
        } => (rect, MouseButtonState::Down, true),
        _ => return,
    };
    let popup = app.state::<Popup>();
    let Ok(mut popup) = popup.0.lock() else {
        return;
    };
    popup.anchor = Some(physical_rect(rect));
    popup.dismissal.invalidate();
    if state == MouseButtonState::Down {
        popup.gesture.press(double);
    }
    let toggle_now = state == MouseButtonState::Up && popup.gesture.release_toggles();
    let double_up = state == MouseButtonState::Up && !toggle_now;
    let clicked_anchor = popup.anchor;
    drop(popup);
    if toggle_now {
        toggle_at(app, clicked_anchor);
    } else if double_up
        && app
            .get_webview_window("main")
            .is_some_and(|window| window.is_visible().unwrap_or(false))
    {
        // Explorer takes focus for the second click too. Keep the first click's
        // open state and restore focus instead of immediately dismissing it.
        if let Err(error) = show_at(app, clicked_anchor) {
            eprintln!("RepoWatch popup: {error}");
        }
    } else {
        schedule_dismiss(app);
    }
}

fn schedule_dismiss(app: &tauri::AppHandle) {
    if let Some(token) = invalidate(app) {
        check_later(app.clone(), token);
    }
}

fn check_later(app: tauri::AppHandle, token: u64) {
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(120));
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || {
            let Some(popup) = handle.try_state::<Popup>() else {
                return;
            };
            let Ok(popup) = popup.0.lock() else { return };
            if !popup.dismissal.is_current(token) {
                return;
            }
            drop(popup);
            let Some(window) = handle.get_webview_window("main") else {
                return;
            };
            if !window.is_visible().unwrap_or(false) {
                return;
            }
            #[cfg(windows)]
            let action = match window.hwnd() {
                Ok(hwnd) => windows::dismiss_action(hwnd.0 as _, anchor(&handle)),
                Err(_) => return,
            };
            #[cfg(not(windows))]
            let action = if window.is_focused().unwrap_or(true) {
                DismissAction::Focused
            } else {
                DismissAction::Hide
            };
            match action {
                // Keep checking an owned native dialog or in-progress tray press:
                // main may not emit another blur when focus leaves that dialog.
                DismissAction::Defer => check_later(handle, token),
                DismissAction::Hide => {
                    let _ = hide(&handle);
                }
                DismissAction::Focused => {}
            }
        });
    });
}

pub fn window_event(window: &tauri::Window, event: &WindowEvent) {
    if window.label() != "main" {
        return;
    }
    match event {
        #[cfg(windows)]
        WindowEvent::Resized(_) => {
            // Aero Snap / Win+Up can still maximize the popup despite
            // `maximizable: false`: refuse it before the lock below is taken, as
            // the restore may re-enter this handler with the restored frame.
            if window.is_maximized().unwrap_or(false) {
                let _ = window.unmaximize();
                return;
            }
            // Read the current frame, not a queued pre-DPI resize event. Preserve
            // the user's preferred logical size across temporary work-area caps.
            if let (Ok(size), Ok(scale)) = (window.outer_size(), window.scale_factor()) {
                if let Ok(mut state) = window.app_handle().state::<Popup>().0.lock() {
                    let actual = (size.width, size.height);
                    if state.applied_size.is_some_and(|applied| applied != actual)
                        && window.is_visible().unwrap_or(false)
                        && !window.is_minimized().unwrap_or(false)
                        && !window.is_maximized().unwrap_or(true)
                    {
                        state.preferred_size = Some((
                            f64::from(size.width) / scale,
                            f64::from(size.height) / scale,
                        ));
                        state.applied_size = Some(actual);
                    }
                }
            }
        }
        WindowEvent::Focused(true) => {
            invalidate(window.app_handle());
        }
        WindowEvent::Focused(false) => schedule_dismiss(window.app_handle()),
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            let _ = hide(window.app_handle());
        }
        _ => {}
    }
}
