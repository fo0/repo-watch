//! Unit tests of the native engine, by topic. Every name is a labelled fixture.

/// The automatic selection of each account's newest repositories.
mod auto_select;
/// Repositories opened from the panel search.
mod browse;
mod browser_permission;
/// Run cancellation: one POST, mapped refusals, no local status change.
mod cancel;
mod connect;
/// The synthetic GitHub, vault and engine shared by the orchestration and connect tests.
mod harness;
/// Repositories the user hid: the freed place, restarts, favorites and disconnects.
mod hidden;
/// The latest run of workflows the run history does not show.
mod last_run;
mod local_identity;
/// The refresh and dispatch orchestration against a synthetic GitHub. No real account,
/// token or repository is involved; every name below is a labelled fixture.
mod orchestration;
/// The owners each account watches: the freed places, restarts, validation and disconnects.
mod owners;
/// The user's refresh interval: stored, validated, and followed by the catalog.
mod refresh_interval;
mod refresh_scope;
mod store;
