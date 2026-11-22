#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod desktop;
mod desktop_auth;
mod desktop_log;
mod desktop_startup;
mod desktop_tray;
mod desktop_updates;
mod tray_popup;

fn main() {
    desktop::run();
}
