//! HTTP adapter for Galleon Fleet endpoints (axum).
//!
//! Under SPEC-013, adapters provide HTTP access to fleet core domains
//! (collections, snapshots, metrics, policies, terminal, quartermaster).

pub mod collections;
pub mod fleet;
pub mod project;
pub mod snapshots;
pub mod system;

use crate::state::AppState;
use axum::Router;

/// Assembles fleet routes.
///
/// Mounted into the main gateway router in TASK-081 with its own
/// RequestBodyLimitLayer (FLEET_MAX_BODY = 1 MiB).
pub fn routes(state: AppState) -> Router<AppState> {
    Router::new()
        .merge(collections::routes(state.clone()))
        .merge(snapshots::routes(state.clone()))
        .merge(fleet::routes(state.clone()))
        .merge(system::routes(state.clone()))
        .merge(project::routes(state))
}
