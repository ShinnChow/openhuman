//! Serde DTOs mirroring `tinyhumansai/backend` wire payloads.
//!
//! These are wire shapes only — they must track the backend's JSON, not the
//! other way around. Route implementations belong in `vendor/tinyhumans-sdk`,
//! not here.
//!
//! - [`auth`] — auth/session payloads: `Session`, `User`, `AuthErrorResponse`,
//!   `AuthState`.
//! - [`socket`] — Socket.IO realtime payloads: `ConnectionStatus`,
//!   `SocketState`, `SocketMessage`, and the JSON-RPC 2.0 MCP envelope types
//!   `McpRequest` / `McpResponse` / `McpError`.

pub mod auth;
pub mod socket;
