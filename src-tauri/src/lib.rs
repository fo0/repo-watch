//! RepoWatch core. Browser preview uses fixtures; the native shell uses authenticated GitHub APIs.

pub mod app_log;
pub mod auth;
pub mod auto_repos;
pub mod cache;
pub mod demo;
pub mod github;
pub mod i18n;
pub mod local_auth;
pub mod models;
pub mod popup;
pub mod refresh_interval;
pub mod run_notice;
pub mod startup;
pub mod workflow;

pub use demo::DemoStore;
pub use models::*;

#[cfg(feature = "network")]
pub mod live;
#[cfg(feature = "network")]
mod run_history;
#[cfg(feature = "network")]
pub mod run_link;
#[cfg(feature = "network")]
pub mod updates;

#[cfg(feature = "network")]
pub mod browser_auth;
