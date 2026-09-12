# Channels

Multi-platform messaging integration. Owns the `Channel` trait vocabulary, host glue for every provider (Slack, Discord, Telegram, WhatsApp, WhatsApp Web, IRC, Signal, iMessage, Email, Lark, Mattermost, DingTalk, QQ, Linq, Yuanbao, and the local CLI REPL), the runtime supervisor that brings channels online, inbound dispatch into the agent loop, and proactive outbound delivery. Does NOT own the channel system prompt copy (lives in `agent/context/channels_prompt.rs`; `system_prompt.rs` here owns only when it is re-rendered), per-channel credential storage (delegated to `security/credentials/`), or provider transport implementations (vendored in `tinychannels`; see below).

## Feature gate

The domain lives behind the `channels` feature (default-ON, #4801), except two dependency-free carve-outs that always-on code reaches directly: `traits` (the `Channel`/`SendMessage` re-export, named by the always-on agent-harness interactive loop) and `cli` (`CliChannel`, the dependency-free stdin/stdout REPL that loop drives in every build). Everything else — `providers`, `host`, `controllers`, `runtime`, `bus`, `proactive`, `commands`, `context`, `routes`, `relay_runtime`, the provider re-exports, `doctor_channels`, `start_channels`, and the `test_support` re-export — is `#[cfg(feature = "channels")]`. The gate sheds zero dependencies: `tinychannels` stays load-bearing for the config schema, the `DomainEvent` inbound envelope, and security pairing, so its value is compile-time surface and binary size, not the dependency tree. See AGENTS.md "channels gate".

## Public surface

- `pub use tinychannels_bus::{Channel, ChannelMessage, ChannelSendExt, SendMessage}` — `traits.rs` — the provider contract, sourced from the transport-free `tinychannels-bus` contract crate so it resolves even in `channels`-less builds.
- `pub struct ChannelDefinition` / `pub enum ChannelAuthMode` — `controllers/definitions.rs` (re-exported from `mod.rs`) — declarative provider metadata.
- `pub fn start_channels` — `runtime/startup.rs` (re-exported from `mod.rs`) — boot all enabled channels under the supervisor.
- `pub fn doctor_channels` — `commands.rs` — diagnose connectivity for the doctor CLI.
- `pub fn build_system_prompt` — re-exported from `crate::agent::context::channels_prompt`.
- `pub(crate) enum ChannelSystemPrompt` — `system_prompt.rs` — the prompt a turn is seeded with: `fixed` (tests) or `refreshing` (production), which renders `build_system_prompt_with_identity` for the active agent profile — with the `## Project Context` identity block placed *after* the tool schemas and access context, and the profile's `system_prompt_suffix` as the trailing `## Agent profile` block — and re-renders only when an identity fingerprint (`agent_profiles.json`, root `SOUL/IDENTITY/PROFILE/MEMORY.md`, the profile's `SOUL/MEMORY.md`) changes — #6027 / #6028.
- Per-provider channel structs re-exported from `providers/<name>.rs` (all thin `pub use tinychannels::providers::…` shims): `CliChannel` (`cli.rs`, ungated), `DingTalkChannel`, `DiscordChannel`, `EmailChannel`, `IMessageChannel`, `IrcChannel`, `LarkChannel`, `LinqChannel`, `MattermostChannel`, `QQChannel`, `SignalChannel`, `SlackChannel`, `TelegramChannel`, `WhatsAppChannel`, `YuanbaoChannel`. Cargo-feature-gated: `WhatsAppWebChannel` (`whatsapp-web`).
- Stable `pub use providers::<name>` paths for every provider — `mod.rs`.
- RPC `channels.{list, describe, connect, disconnect, status, set_default, get_default, test, telegram_login_start, telegram_login_check, discord_link_start, discord_link_check, discord_list_guilds, discord_list_channels, discord_check_permissions, send_message, send_reaction, create_thread, update_thread, list_threads}` — `controllers/schemas.rs`.

## Submodule map

| Path | Purpose |
| --- | --- |
| `providers/` | Per-provider re-exports from `tinychannels`, plus Telegram's host-coupled glue ([README](providers/README.md)) |
| `host/` | `build_channel_host` / `build_provider_context`, the OpenHuman implementation of `tinychannels::host` that ported providers call back into |
| `runtime/` | Startup, supervision, and the inbound dispatch loop into the agent ([README](runtime/README.md)) |
| `controllers/` | `channels.*` RPC namespace, `OpenHumanChannelBackend`, provider definitions ([README](controllers/README.md)) |
| `tests/` | Cross-channel integration test suite (`#[cfg(all(feature = "channels", test))]`) |

Flat files: `bus.rs` (`ChannelInboundSubscriber`, publishes `DomainEvent::Channel*` onto the process bus), `cli.rs` (ungated `CliChannel`), `commands.rs` (`doctor_channels`), `context.rs`, `proactive.rs` (subscribes `DomainEvent::ProactiveMessageRequested` for scheduled/triggered outbound sends), `relay_runtime.rs` (`relay_runtime_fronts_channel` — whether a channel is fronted by the relay-websocket transport), `routes.rs`, `system_prompt.rs`, `traits.rs` (ungated `Channel`/`SendMessage` re-export).

## Calls into

- `crates/openhuman-core/src/agent/` — inbound messages spawn or resume agent runs through `runtime/dispatch/`.
- `crates/openhuman-core/src/agent/context/channels_prompt.rs` — channel system prompt rendering, re-exported as `build_system_prompt`.
- `crates/openhuman-core/src/security/credentials/` — per-channel auth tokens, refresh flow.
- `crates/openhuman-core/src/config/schema/channels.rs` — runtime channel configuration.
- `crates/openhuman-core/src/threads/` — thread state for platforms with native threading (Slack `thread_ts`).
- `crates/openhuman-core/src/desktop/notifications/` — surface inbound deliveries to the UI.
- `crates/openhuman-core/src/security/encryption/` — at-rest secret protection.
- `crates/openhuman-core/src/core/bus.rs` and `core/events.rs` — `DomainEvent` definitions and the process-wide `BUS`; `channels/bus.rs` registers `ChannelInboundSubscriber` against it.
- `vendor/tinychannels/` — provider transport implementations and the `tinychannels::host` capability boundary; `vendor/tinychannels-bus` — the transport-free trait/type contract.

## Called by

- `crates/openhuman-core/src/threads/ops.rs` — thread lifecycle uses channel send paths.
- `crates/openhuman-core/src/memory/conversations/bus.rs` — persists incoming channel messages as conversation memories.
- `crates/openhuman-core/src/cron/bus.rs` — scheduled triggers can post via channels.
- `crates/openhuman-core/src/config/schema/channels.rs` — config layer references channel types for validation.
- `crates/openhuman-core/src/core/all.rs` — registers `controllers::all_channels_registered_controllers()` under the `channels` feature gate.
- `crates/openhuman-core/src/core/runtime/services.rs` — boot path calls `channels::start_channels(config)` unless `OPENHUMAN_DISABLE_CHANNEL_LISTENERS` is set.

## Tests

- Unit, flat files: `bus_tests.rs` (+ `bus_part_01.rs`/`bus_part_02.rs` split, `bus_inbound_thread_id_tests_tests.rs`, `bus_test_support_tests.rs`), `cli_tests.rs`, `commands_tests.rs`, `context_tests.rs`, `proactive_tests.rs`, `relay_runtime_tests.rs`, `routes_tests.rs`, `system_prompt_tests.rs`, `traits_tests.rs`.
- Cross-channel integration suite: `tests/{common,context,discord_integration,health,identity,memory,personality,prompt,runtime_dispatch,runtime_tool_calls,telegram_integration}.rs` — `common.rs` holds the shared bus/agent/tool fixtures the rest of the suite drives.
- Provider host glue: `providers/telegram/{approval_surface_tests,bus_tests,remote_control_tests}.rs`.
- Controllers: `controllers/{backend_tests,ops_tests,schemas_tests}.rs` (+ `ops_tests_part_01_tests.rs`/`ops_tests_part_02_tests.rs`), `controllers/ops/connect_email_config_tests_tests.rs`.
- Runtime: `runtime/{startup_tests,supervision_tests}.rs` (+ startup secret-resolution tests for email/yuanbao), `runtime/dispatch_tests.rs`, `runtime/dispatch/{mod_scoping_tests_tests,mod_approval_surface_gating_tests_tests,routing_connected_fallback_tests_tests}.rs`.
