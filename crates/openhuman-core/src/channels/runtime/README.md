# Runtime

Boots every enabled channel, keeps their listeners alive, and dispatches inbound messages into the agent loop.

## Files

| Path | Purpose |
| --- | --- |
| `startup.rs` (+ `startup_part_01.rs`, `startup_part_02.rs`) | `start_channels` — reads config, resolves per-channel secrets (email, yuanbao), constructs each enabled `Channel`, and spawns a supervised listener per channel into a shared dispatch queue |
| `supervision.rs` | `spawn_supervised_listener` — wraps a channel listener task so a crashed listener is respawned instead of silently dying |
| `dispatch/` | The inbound message pipeline that turns a received `RuntimeChannelMessage` into an agent turn and a reply |
| `test_support.rs` | Test/debug-only helpers (`#[cfg(any(test, debug_assertions))]`) shared across the runtime's own tests and `channels/tests/` |

## Inbound path

1. A provider listener (spawned in `startup_part_01.rs`, one per enabled channel) receives a message and sends it as a `RuntimeChannelMessage` on a bounded `mpsc` channel (capacity 100).
2. `spawn_supervised_listener` (`supervision.rs`) owns the listener task and restarts it if it exits unexpectedly, so a single crashed provider connection doesn't take the process down.
3. `run_message_dispatch_loop` (`dispatch/processor.rs`, called once from `startup_part_01.rs` after every listener is spawned) drains the shared receiver with bounded concurrency (`compute_max_in_flight_messages`) and hands each message to `process_channel_message`.
4. `process_channel_message` runs the full per-message pipeline: typing indicator, ACK reaction, approval-reply interception (`try_route_approval_reply`, gated by `channel_has_approval_surface`), agent-scoping (`dispatch/routing.rs`: `resolve_target_agent` + `build_visible_tool_set`), the agent turn itself, draft updates, and the reply send.

## `dispatch/`

| Path | Purpose |
| --- | --- |
| `helpers.rs` | Stateless helpers: per-turn context block for non-web channels, deterministic ACK-emoji picker, worker/typing-task lifecycle utilities |
| `routing.rs` | `AgentScoping`, `resolve_target_agent`, `build_visible_tool_set` — picks the active agent and its visible/delegation tool surface for a turn, reading `Config` + `AgentDefinitionRegistry` + connected integrations |
| `processor.rs` (+ `processor_part_01.rs..03.rs`) | `channel_has_approval_surface` (per-channel approval-context gate), `try_route_approval_reply`, `process_channel_message`, `run_message_dispatch_loop` |
| `mod.rs` | Wires the three submodules and their `#[cfg(test)]` / `#[cfg(any(test, debug_assertions))]` re-exports |

Approval-surface gating and per-sender agent scoping are the two policy points a new channel provider needs to account for: whether inbound approval replies (yes/no) short-circuit a fresh agent turn, and which agent/tools a given sender is routed to.

## Called by

`channels::start_channels` is exported from `channels/mod.rs` and invoked from `crates/openhuman-core/src/core/runtime/services.rs` during boot, unless `OPENHUMAN_DISABLE_CHANNEL_LISTENERS` is set. It is skipped for web-chat-only cores (see `core/jsonrpc.rs` boot comments and `skills/bus.rs`).

## Tests

- `startup_tests.rs`, plus secret-resolution coverage in `startup_email_secret_tests_tests.rs` and `startup_yuanbao_secret_tests_tests.rs`.
- `supervision_tests.rs`.
- `dispatch_tests.rs` (top-level dispatch loop behavior), `dispatch/mod_scoping_tests_tests.rs` (agent scoping), `dispatch/mod_approval_surface_gating_tests_tests.rs` (approval-surface gate), `dispatch/routing_connected_fallback_tests_tests.rs` (fallback routing when no connected agent matches).
