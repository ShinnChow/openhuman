//! WebSocket Engine.IO / Socket.IO connection loop with automatic reconnection.
//!
//! Split by responsibility rather than by line count:
//! - [`dispatch`] parses incoming Engine.IO/Socket.IO frames.
//! - [`connect`] runs the redirect-following connect and one connection's
//!   handshake and event loop.
//! - [`reconnect`] is the outer [`ws_loop`] retry/backoff loop that drives
//!   [`connect::run_connection`] and decides how to react to failures.

#[cfg(test)]
#[path = "ws_loop_tests.rs"]
mod tests;

mod connect;
mod dispatch;
mod reconnect;

pub(super) use reconnect::ws_loop;

#[cfg(test)]
use super::manager::AckRegistry;
