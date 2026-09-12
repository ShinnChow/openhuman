// Composio Tool Provider — optional managed tool surface with 1000+ OAuth integrations.
//
// When enabled, OpenHuman can execute actions on Gmail, Notion, GitHub, Slack, etc.
// through Composio's API without storing raw OAuth tokens locally.
//
// This is opt-in. Users who prefer sovereign/local-only mode skip this entirely.
// The Composio API key is stored in the encrypted secret store.

#[cfg(test)]
#[path = "direct_tests.rs"]
mod tests;

mod connections;
mod construction;
mod discovery;
mod execution;
mod http_errors;
mod tool_impl;
mod types;

pub use connections::ComposioConnectedAccount;
pub use discovery::{ComposioAction, ComposioToolSchemaV3};
pub use types::ComposioTool;
