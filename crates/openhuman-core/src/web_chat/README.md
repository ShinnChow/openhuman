# Web chat

The web/desktop channel's turn runner. Owns the `channel.web_*` RPC namespace,
Socket.IO's `chat`/`cancel` events, and the whole request lifecycle from a raw
message to a delivered reply. Distinct from `channels/`, which drives external
messaging providers (Telegram, WhatsApp, …) through the same agent harness —
this module is the desktop/web UI's own path and the hottest one in the
product.

## Request lifecycle

1. `core/socketio.rs` receives a `chat` socket event and calls
   [`start_chat`] (`ops_part_02.rs`) with the raw message, thread/client ids,
   and any model/profile/locale/queue-mode overrides.
2. `start_chat` preprocesses `[FILE:…]`/`[IMAGE:…]` attachment markers
   *before* prompt-injection scanning or persistence (a multi-MB base64 blob
   must never reach those stages), runs `enforce_prompt_input`, and checks
   for a parked chat-native approval reply before treating the message as a
   new turn.
3. It resolves or builds the session `Agent` (`session.rs`), reusing a cached
   one from `THREAD_SESSIONS` when the `SessionCacheFingerprint` still
   matches, and hands off to `run_task::run_chat_task`, tracked in
   `InFlightEntry`/`ParallelEntry` (`types.rs`) for cancellation and
   interrupt/steer/queue semantics.
4. `run_chat_task` spawns [`spawn_progress_bridge`] (`progress_bridge.rs`)
   alongside the agent run. The bridge forwards `AgentProgress` into
   `WebChannelEvent` socket events, mirrors turn state into
   `crate::threads::turn_state::TurnStateStore`, and emits an
   `inference_heartbeat` beat every `INFERENCE_HEARTBEAT_SECS` (20s) so a long
   silent prefill doesn't trip the frontend's disconnect timer.
5. On completion, `presentation.rs` formats and segments the reply for
   delivery (natural-language prose only; code/structured output is never
   split) and calls `reply_persistence::persist_delivered_reply` to write the
   reply to the thread's conversation store *before* announcing it, so the
   answer survives a client reconnect or reload (#6034).
6. Errors from the provider are normalized through `web_errors.rs`
   (`classify_inference_error`) into user-facing copy — budget-exhausted,
   non-retryable rate limit, fallback-chain-exhausted, etc. — using a
   per-thread `BudgetCorrelation` to reclassify an empty 200 as an
   out-of-credits turn when the last signal was on the same provider binding
   (#3386).

## Public surface

- Event bus (`event_bus.rs`): `subscribe_web_channel_events`,
  `publish_web_channel_event`, `approval_request_event`,
  `register_approval_surface_subscriber`, `register_artifact_surface_subscriber`,
  `register_egress_surface_subscriber` — bridge `DomainEvent`s onto the
  in-process `WebChannelEvent` broadcast bus consumed by both Socket.IO and
  the JSON-RPC `/events` SSE stream.
- Operations (`ops.rs` + `ops_part_0{1,2,3}.rs`): `start_chat`, `cancel_chat`,
  `cancel_chat_scoped`, `cancel_should_target`, `channel_web_chat`,
  `channel_web_cancel`, `channel_web_queue_status`, `channel_web_queue_clear`,
  `invalidate_thread_sessions`, `ChatRequestMetadata` — the turn's request
  surface and its in-flight/session state (`THREAD_SESSIONS`,
  `THREAD_BUDGET_SIGNALS`, in-flight/parallel maps).
- Schemas (`schemas.rs`): `all_web_channel_controller_schemas`,
  `all_web_channel_registered_controllers`, `schemas` — RPC contract for the
  `channel` namespace.
- Debug/test-only hooks: `set_test_forced_run_chat_task_error`,
  `RUN_CHAT_TASK_TEST_LOCK`, `set_test_run_chat_task_block`,
  `TestRunChatTaskBlock`, `parallel_in_flight_entries_for_test`,
  `in_flight_entries_for_test`, `fresh_approval_surface_subscription` —
  compiled only under `#[cfg(test, debug_assertions)]`/`debug_assertions`, so
  none of them link into a release binary.

## Files

| File | Purpose |
| --- | --- |
| `mod.rs` | Module wiring and re-exports; no business logic |
| `ops.rs`, `ops_part_01.rs`, `ops_part_02.rs`, `ops_part_03.rs` | `start_chat`/`cancel_*`/`channel_web_*` operations, session cache, in-flight tracking, budget-signal correlation |
| `run_task.rs` | `run_chat_task` — builds the session agent, runs the turn, spawns the progress bridge |
| `session.rs` | Builds/fingerprints the cached session `Agent`, resolves target agent id, locale directive, provider role for a model override |
| `progress_bridge.rs` | Forwards `AgentProgress` into `WebChannelEvent`s and `TurnStateStore`, emits the `inference_heartbeat` liveness beat |
| `presentation.rs` | Local-model response segmentation into chat bubbles and emoji-reaction decisions; calls `reply_persistence` before announcing |
| `reply_persistence.rs` | Durable write of the reply about to be announced, under a deterministic id shared with the client's own append |
| `event_bus.rs` | The `WebChannelEvent` broadcast channel plus approval/artifact/egress `DomainEvent` surface subscribers |
| `web_errors.rs`, `web_errors_part_0{1,2,3}.rs` | Classifies raw provider error strings into user-facing copy; budget-exhausted / rate-limit / fallback-exhausted detection |
| `schemas.rs` | `ControllerSchema`/`RegisteredController` definitions for the `channel.web_*` RPC functions |
| `types.rs` | `SessionEntry`, `SessionCacheFingerprint`, `InFlightEntry`, `ParallelEntry`, `WebChatTaskResult`, `ChatRequestMetadata`, RPC param structs |

## RPC

Namespace `channel`, registered via
`all_web_channel_registered_controllers()` in `core/all.rs`:

| Function | Handler |
| --- | --- |
| `web_chat` | `channel_web_chat` |
| `web_cancel` | `channel_web_cancel` |
| `web_queue_status` | `channel_web_queue_status` |
| `web_queue_clear` | `channel_web_queue_clear` |

## Events

- Broadcasts `WebChannelEvent` (defined in `core/socketio.rs`) over an
  in-process `tokio::sync::broadcast` channel. `core/socketio.rs` forwards it
  to the connected Socket.IO client; `core/jsonrpc.rs` forwards the same
  stream to the JSON-RPC `/events` SSE endpoint.
- Subscribes to `DomainEvent` on `crate::core::bus::BUS` via three
  process-lifetime, `OnceLock`-guarded subscribers registered from
  `core/jsonrpc.rs`: `register_approval_surface_subscriber`
  (`ApprovalRequested`/`PlanReviewRequested` → `approval_request` /
  `plan_review_request`), `register_artifact_surface_subscriber`
  (`ArtifactPending`/`Ready`/`Failed` → `artifact_*`), and
  `register_egress_surface_subscriber` (`ExternalTransferPending` →
  `external_transfer_pending`, only when the transfer carries chat routing).

## Calls into

- `crate::agent::harness` — `Agent::from_config_for_agent_with_profile`,
  `run_queue::RunQueue`, and the tool-calling loop itself.
- `crate::agent::profiles::AgentProfileStore` — resolves the active profile
  for `build_session_agent`.
- `crate::threads::turn_state::{TurnStateStore, TurnStateMirror}` — the
  progress bridge mirrors turn state here for cross-surface visibility.
- `crate::inference::provider` — `provider_for_role` resolves the
  provider binding used both for session fingerprinting and for `role_for_model_override`.
- `crate::security::approval::APPROVAL_CHAT_CONTEXT` — scoped for the
  duration of a turn. `crate::web3::wallet::execution::current_owner()`
  relies on this task-local staying scoped through the inline `.await` chain
  in `run_chat_task`; detaching the tool loop onto a fresh `tokio::spawn`
  without re-scoping it would silently disable the quote-owner gate (see
  `web3/wallet/README.md`).
- `crate::memory::conversations` — `reply_persistence` appends the durable
  reply row; `crate::memory::agent::memory_loader::MemoryCitation` carries
  citations through to that row's metadata.

## Called by

- `core/socketio.rs` — the `chat` and `cancel` Socket.IO handlers call
  `start_chat` / `cancel_chat_scoped`, and forward `WebChannelEvent`s to the
  client.
- `core/jsonrpc.rs` — subscribes the event stream for `/events` SSE and
  registers the three `DomainEvent` surface subscribers at startup.
- `core/all.rs` — registers `all_web_channel_registered_controllers()` under
  the `channels` feature gate.

## Tests

- `web_tests.rs` (+ `web_tests_part_01..04_tests.rs`) — end-to-end coverage
  of `start_chat`/`cancel_*`/queue behavior.
- `mod_test_support_tests.rs` — `test_support` module helpers re-exported
  through `mod.rs`.
- Per-file unit tests: `event_bus_tests.rs`, `ops_budget_correlation_tests_tests.rs`,
  `presentation_tests.rs` + `presentation_test_support_tests.rs`,
  `progress_bridge_tests.rs`, `reply_persistence_tests.rs`, `run_task_tests.rs`.
