//! Shared HTTP client for all integration tools.

mod construct;
mod download;
mod errors;
mod pricing;
mod requests;

pub use construct::IntegrationClient;
pub use pricing::{build_client, pricing_for_config};

// Re-exported at module scope (rather than only inside each submodule) so
// `#[cfg(test)] mod tests` below — and the `super::*` glob each test file
// does — can see them, mirroring what direct declaration in this file used
// to provide before the `include!` split was replaced with real submodules.
use construct::sanitize_backend_url;
use errors::{extract_error_detail, is_composio_soft_auth_path};
use requests::{backend_egress_descriptor, enforce_backend_egress, managed_budget_applies_to_path};

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
