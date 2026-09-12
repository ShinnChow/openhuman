//! Cross-channel integration suite, declared as
//! `#[cfg(all(feature = "channels", test))] mod tests` in `channels/mod.rs`
//! (see AGENTS.md — unit tests otherwise stay beside their modules as
//! `*_tests.rs`; this tree is the deliberate exception for tests that
//! exercise more than one channel submodule end-to-end).
//!
//! `common.rs` holds the shared fixtures every file below builds on: a fake
//! `ChatModel` per scenario (`DummyModel`, `SlowModel`, `ToolCallingModel`,
//! `IterativeToolModel`, `HistoryCaptureModel`, `ModelCaptureModel`), fake
//! `Channel` implementations that record or replay sends
//! (`RecordingChannel`, `TelegramRecordingChannel`, `AlwaysFailChannel`), a
//! `NoopMemory`, a `MockPriceTool`, and `make_workspace` for identity-file
//! fixtures.
//!
//! * `context.rs` — channel runtime-context helpers: timeout clamping,
//!   history compaction, memory-context skip rules.
//! * `discord_integration.rs` — end-to-end dispatch through the Discord
//!   channel with every cross-module boundary (agent runtime, memory,
//!   provider) substituted, proving the domain stays encapsulated.
//! * `health.rs` — supervised-listener health classification when a channel
//!   listener keeps failing.
//! * `identity.rs` — `build_system_prompt` inlining workspace identity
//!   markdown into the Project Context section.
//! * `memory.rs` — conversation-history and memory-context wiring through
//!   `process_channel_message`.
//! * `personality.rs` — acceptance coverage for #6027/#6028 (channel turns
//!   carry the active personality; identity edits reach the next turn
//!   without a restart).
//! * `prompt.rs` — system-prompt section assembly and bootstrap truncation.
//! * `runtime_dispatch.rs` — the dispatch loop and `RuntimeChannelMessage`
//!   plumbing via `runtime::test_support`.
//! * `runtime_tool_calls.rs` — tool-call routing through
//!   `process_channel_message`.
//! * `telegram_integration.rs` — Telegram reactions, reply/thread roundtrip,
//!   and typing-indicator lifecycle against a recording channel.

mod common;
mod discord_integration;
mod health;
mod identity;
mod memory;
mod personality;
mod prompt;
mod runtime_dispatch;
mod runtime_tool_calls;
mod telegram_integration;
