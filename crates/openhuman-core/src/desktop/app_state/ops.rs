//! Core-owned `app_state` business logic: the polled snapshot RPC, local
//! on-disk state, and the current-user cache that backs both.
//!
//! Split by responsibility:
//! - [`types`] — the serde types exchanged with the frontend.
//! - [`state_file`] — atomic on-disk persistence of `StoredAppState`.
//! - [`auth_timeout`] — the `auth_get_me` fetch timeout and its derived backoff base.
//! - [`current_user_fetch`] — the plain `GET /auth/me` HTTP call.
//! - [`current_user`] — the positive/negative current-user caches and their
//!   blocking/background refresh paths.
//! - [`current_user_generation`] — sign-out invalidation of those caches.
//! - [`staleness`] — how old the data a snapshot is serving has become.
//! - [`pending_session`] — activating or rejecting a not-yet-confirmed session.
//! - [`runtime_snapshot`] — the local-AI + service half of the snapshot.
//! - [`snapshot`] — the `app_state_snapshot` / `update_local_state` RPC handlers.

mod auth_timeout;
mod current_user;
mod current_user_fetch;
mod current_user_generation;
mod pending_session;
mod runtime_snapshot;
mod snapshot;
mod staleness;
mod state_file;
mod types;

/// Shared log prefix for every `[app_state]`-tagged debug/warn line across
/// these submodules.
pub(super) const LOG_PREFIX: &str = "[app_state]";

pub use auth_timeout::{
    parse_auth_fetch_timeout_secs, AUTH_FETCH_TIMEOUT_ENV_VAR, DEFAULT_AUTH_FETCH_TIMEOUT_SECS,
    MAX_AUTH_FETCH_TIMEOUT_SECS, MIN_AUTH_FETCH_TIMEOUT_SECS,
};
pub use current_user::peek_cached_current_user_identity;
pub use current_user_generation::{forget_current_user_caches, CURRENT_USER_SESSION_MUTATION_LOCK};
pub(crate) use state_file::load_stored_app_state;
pub use snapshot::{snapshot, update_local_state};
pub use state_file::save_app_state;
pub use types::{AppStateSnapshot, RuntimeSnapshot, StoredAppState, StoredAppStatePatch, StoredOnboardingTasks};
