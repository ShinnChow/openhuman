# threads

Conversation thread and message management. Owns the RPC surface and controller registry for thread lifecycle (create / list / upsert / delete / purge), per-thread message CRUD, AI-assisted thread titling, persisted in-flight turn snapshots (for cold-boot recovery of interrupted agent turns), the per-thread kanban task board passthrough, and a one-shot welcome-agent → orchestrator workspace migration. Persistence is delegated to `memory::conversations` (JSONL thread/message store) and to a dedicated `turn_state` snapshot store; this module is the RPC/controller layer over both.

## Responsibilities

- List, create (`upsert` with caller-supplied id or `create_new` with auto-generated id + placeholder title), delete, and purge conversation threads.
- List, append, and metadata-patch messages within a thread.
- Update thread labels and user-specified titles.
- Generate a durable thread title from the first user message + assistant reply via the inference provider, with deterministic fallbacks (derive title from the user message; skip non-placeholder titles).
- Maintain restart-survivable snapshots of in-flight agent turns (`turn_state`): get / list / clear over RPC; written by the web-channel progress bridge via `TurnStateMirror`.
- Expose a per-thread kanban task board get/put that proxies to `agent::task_board`.
- On thread delete/purge, invalidate the in-process web-channel session and clean up orphaned turn snapshots.
- One-shot idempotent migration of legacy welcome-agent artifacts (strip `onboarding` label, rename `welcome*` session transcripts/markdown to `orchestrator*`).

## Key files

| File | Role |
| --- | --- |
| `crates/openhuman-core/src/threads/mod.rs` | Export-focused: module docstring, `mod` decls (incl. `goals`, `todos`, `transcript_view` submodules), re-exports (`ThreadsError`, `THREAD_NOT_FOUND_KIND`, controller schema/registry pair, welcome-migration entry point). |
| `crates/openhuman-core/src/threads/ops.rs` | Thin shell: `include!`s `ops_part_01.rs` and `ops_part_02.rs` (split for file-size, not module boundaries). Business logic / RPC entry points returning `RpcOutcome<ApiEnvelope<T>>` live in the parts; all thread + message + turn-state ops wrap results in `ApiEnvelope` with request-id/count meta. |
| `crates/openhuman-core/src/threads/schemas.rs` | Thin shell: `include!`s `schemas_part_01.rs` and `schemas_part_02.rs`. `ControllerSchema` definitions, `all_controller_schemas`, `all_registered_controllers`, and `handle_*` functions delegating to `ops` live in the parts, along with the `task_board_get`/`task_board_put` handlers (proxy to `agent::task_board`). |
| `crates/openhuman-core/src/threads/error.rs` | `ThreadsError` taxonomy (`NotFound` / `Message`); encodes `NotFound` as a `StructuredRpcError` (`kind: "ThreadNotFound"`, `expected_user_state: true`) at the controller boundary so the frontend handles stale thread refs without string-matching or Sentry noise. |
| `crates/openhuman-core/src/threads/title.rs` | Pure, provider-free helpers: placeholder-title detection, raw-title sanitization, whitespace collapse, fallback title from user message, prompt builder, log fingerprint. Heavily unit-tested. |
| `crates/openhuman-core/src/threads/welcome_migration.rs` | One-shot, marker-guarded migration of legacy welcome-agent threads/transcripts to orchestrator naming. |
| `crates/openhuman-core/src/threads/turn_state/mod.rs` | Submodule export hub for in-flight turn snapshots. |
| `crates/openhuman-core/src/threads/turn_state/types.rs` | Wire/storage types: `TurnState`, `TurnLifecycle`, `TurnPhase`, `ToolTimelineEntry`/`Status`, `SubagentActivity`/`ToolCall`, and the get/list/clear request/response payloads. camelCase to mirror `chatRuntimeSlice.ts`. |
| `crates/openhuman-core/src/threads/turn_state/store.rs` | `TurnStateStore` — atomic per-thread JSON snapshot store (tempfile + persist + dir fsync), process-wide mutex; `put`/`get`/`delete`/`list`/`clear_all`/`mark_all_interrupted` plus free-fn wrappers. |
| `crates/openhuman-core/src/threads/turn_state/mirror.rs` (+ `mirror_part_01.rs`/`mirror_part_02.rs`) | `TurnStateMirror` — translates `agent::progress::AgentProgress` events into `TurnState` mutations (incl. the ordered narration/thinking/tool `transcript`), flushing at iteration/tool boundaries; on completion marks the snapshot `Completed` and **keeps** it (so the "View processing" panel can replay the finished turn), flags `Interrupted` if the bridge exits without `TurnCompleted`. |
| `*_tests.rs` | Sibling test suites: `ops_tests*.rs`, `schemas_tests.rs`, `turn_state/store_tests.rs`, `turn_state/mirror_tests*.rs` (plus inline tests in `error.rs`, `title.rs`, `welcome_migration.rs`). |

## Submodules

- [`goals/`](goals/mod.rs) — thin OpenHuman host adapters over `tinyagents_graph::goals`. Tinyagents owns the goal type, lifecycle, persistence, and prompt rendering; this module owns workspace-store resolution, the `thread_goals.*` JSON-RPC schemas (`get`/`set`/`complete`/`pause`/`resume`/`clear`, 6 functions), domain events, the legacy-file migration, heartbeat dispatch, and adapters for OpenHuman's pre-tinyagents `Tool`/`StopHook` traits (`continuation.rs`, `migration.rs`, `ops.rs`, `runtime.rs`, `schemas.rs`, `store.rs`, `tools.rs`). The external `thread_goals` RPC namespace is stable regardless of the tinyagents-side implementation.
- [`todos/`](todos/README.md) — compatibility surface over `tinyagents_graph::todos` (task-board types, normalization, markdown rendering, CRUD, plan decisions, the single-`in_progress` invariant, atomic claims, durable storage). OpenHuman keeps the `todos.*` JSON-RPC API (14 functions, split across `schemas_part_01.rs`/`schemas_part_02.rs`), the granular `todo_*` agent tools (`tools.rs`), and `runs.rs`, which binds the TinyAgents autonomous-run ledger to `BoardLocation` addressing and imports retired `agent_task_boards/<hex>.runs.json` ledgers. See the submodule README for details.
- [`transcript_view/`](transcript_view/mod.rs) — projects the append-only `session_raw/*.jsonl` transcript source of truth into typed `DisplayItem`s for the chat renderer, with newest-first pagination over a bounded in-memory cache (`cache.rs`, `project.rs`, `types.rs`). Entry point `get_page` backs the `threads.transcript_get` RPC.

## Public surface

From `mod.rs`:
- `ThreadsError`, `THREAD_NOT_FOUND_KIND` (re-exported from `error`).
- `all_threads_controller_schemas` / `all_threads_registered_controllers` (re-exported from `schemas`).
- `migrate_welcome_agent_artifacts`, `WelcomeMigrationResult` (re-exported from `welcome_migration`).

From `turn_state`: `TurnStateMirror`, `TurnStateStore`, `TurnState` + lifecycle/phase/timeline/subagent types and the turn-state RPC request/response types.

## RPC / controllers

Namespace `threads` (JSON-RPC `openhuman.threads_<function>`). Schemas + handlers registered via `all_registered_controllers`:

| Function | Op |
| --- | --- |
| `list` | List thread summaries. |
| `upsert` | Create/refresh a thread (caller id, title, created_at, optional labels). |
| `create_new` | New thread with auto id + `Chat <date> <time>` placeholder title. |
| `messages_list` | List messages for a thread. |
| `message_append` | Append a message (returns typed `ThreadsError`, i.e. structured `NotFound`). |
| `message_update` | Patch `extra_metadata` on a message. |
| `generate_title` | LLM-generate title from first user + assistant message, with fallbacks. |
| `update_labels` | Replace labels (empty vec clears all labels). |
| `update_title` | Set a user-specified title (rejects empty). |
| `delete` | Delete thread + message log; invalidates web session, clears turn snapshot. |
| `purge` | Remove all threads/messages; `clear_all` turn snapshots. |
| `turn_state_get` / `turn_state_list` / `turn_state_clear` | Read / list / delete persisted in-flight turn snapshots. |
| `task_board_get` / `task_board_put` | Proxy the per-thread kanban board to `agent::task_board`. |

Wired into the registry from `crates/openhuman-core/src/core/all.rs` (controllers + schemas extended with the `all_threads_*` pair).

The `goals/` and `todos/` submodules register their own controller namespaces from the same call site:

| Namespace | Functions |
| --- | --- |
| `thread_goals` (`openhuman.thread_goals_<function>`) | `get`, `set`, `complete`, `pause`, `resume`, `clear` — thread-level goal CRUD over `tinyagents_graph::goals`, via `goals::all_thread_goals_registered_controllers`. |
| `todos` (`openhuman.todos_<function>`) | 14 functions covering task-board CRUD, plan decisions, claims, and run-ledger operations over `tinyagents_graph::todos`, via `todos::schemas` (split `schemas_part_01.rs`/`schemas_part_02.rs`). |

## Persistence

- **Threads + messages**: delegated to `memory::conversations` (JSONL store under the workspace), not owned here. Every call goes through `crate::memory::conversations::blocking::*` (`tokio::task::spawn_blocking`) — **never** the sync entry points directly, see the note below.
- **Turn snapshots** (`turn_state/store.rs`): one JSON file per thread at `<workspace>/memory/conversations/turn_states/<hex(thread_id)>.json`. Whole-file atomic overwrite (tempfile → fsync → persist → best-effort dir fsync), serialized through a process-wide `parking_lot::Mutex`. A non-terminal file surviving cold boot is marked `Interrupted`; a `Completed` snapshot is intentionally retained (for processing replay) and skipped by startup interrupted-marking. The next turn on the thread overwrites it.
- **Task board**: persisted by `agent::task_board::TaskBoardStore` under the workspace (this module only proxies).
- **Migration marker**: `state/migrations/welcome_to_orchestrator_v1.done` guards the welcome migration.

## Dependencies

- `crate::memory::conversations` — thread + message store types and CRUD (`ensure_thread`, `list_threads`, `get_messages`, `append_message`, `update_thread_*`, `ConversationStore`, etc.); also the `ApiEnvelope`/`ApiMeta`/request/response DTOs.
- `tinyagents_graph::{goals, todos}` — the vendored goal and task-board domains that `goals/` and `todos/` adapt.
- `crate::config::Config` — resolves `workspace_dir` and inference/runtime/secrets settings (`load_or_init`).
- `crate::inference::provider` — builds the intelligent-routing provider used for AI title generation (`create_intelligent_routing_provider`, `ProviderRuntimeOptions`).
- `crate::web_chat` — `invalidate_thread_sessions` on thread delete (so a deleted thread's live web session can't keep appending).
- `crate::agent::task_board` — `TaskBoard`, `TaskBoardCard`, `TaskBoardStore`, `board_for_thread` for the task-board RPCs; also the optional `task_board` field on `TurnState`.
- `crate::agent::progress::AgentProgress` — progress events consumed by `TurnStateMirror`.
- `crate::core::all` — `ControllerFuture`, `RegisteredController` for the registry.
- `crate::core::{ControllerSchema, FieldSchema, TypeSchema}` — schema definitions.
- `crate::rpc::{RpcOutcome, StructuredRpcError}` — RPC outcome wrapper and structured-error encoding.

## Used by

- `crates/openhuman-core/src/core/all.rs` — registers the controllers/schemas into the JSON-RPC + CLI registry.
- `crates/openhuman-core/src/core/jsonrpc.rs` — references threads (transport routing).
- `crates/openhuman-core/src/web_chat/` — drives `TurnStateMirror` / turn-state store during chat turns; consumes `invalidate_thread_sessions`.
- `crates/openhuman-core/src/platform/startup/ops.rs` — invokes the welcome migration and/or turn-state startup handling.

## Notes / gotchas

- **Never call the sync `memory::conversations` API from these handlers — use `memory::conversations::blocking::*`.** Each store entry point takes a process-global `parking_lot::Mutex` and then does fsync'd JSONL IO while holding it, and the per-call cost grows with the user's history (`threads.jsonl` is folded on nearly every operation and gains ~2 lines per message, uncompacted). Called inline, a handler parks a tokio *worker* thread on that mutex; once more conversation ops are queued than there are workers, the runtime stops polling anything — including the HTTP task that owes the client a response — and a one-append create blows the frontend's 30 s RPC budget (`UnhandledRejection: Core RPC openhuman.threads_create_new timed out after 30000ms`, Sentry TAURI-REACT-10 / #5156). The wrappers keep the lock wait on the blocking pool; the store is just as serialized, but the executor stays live. `welcome_migration.rs` is the deliberate exception: it is a sync, one-shot, marker-guarded boot migration, not a request path.
- `generate_title` only replaces titles matching the `Chat <Mon> <d> <h>:<mm> <AM|PM>` placeholder shape (`is_auto_generated_thread_title`); user-renamed threads are never overwritten. Provider/init/sanitization failures degrade to a deterministic fallback title derived from the first user message — never an error.
- `delete` invalidates the web-channel session **before** turn-snapshot cleanup (ordering is load-bearing per the inline comment); snapshot-cleanup failure surfaces as an RPC error so callers see a partial failure rather than silent on-disk drift.
- `purge` uses `clear_all` (not list+delete) so corrupted/half-written snapshot files — which `list()` warn-skips — are also removed.
- `ThreadsError::NotFound` is the **only** place `ThreadNotFound` becomes a wire-shaped structured error; the transport layer does not sniff method names or error strings. `from_thread_scoped_store_error` only promotes to `NotFound` when the parsed id matches the requested thread id, to avoid clearing the wrong stale thread on the frontend.
- Turn-state types intentionally serialize camelCase to mirror `app/src/store/chatRuntimeSlice.ts` so a snapshot applies to the slice without translation.
- `TurnStateMirror` flushes only at iteration/tool boundaries; high-frequency deltas (streaming text, thinking, tool args) mutate memory only, to avoid filesystem thrash under streaming load.
- Directory fsync after snapshot rename is best-effort and a no-op on Windows (relies on NTFS journaling).
- The welcome migration is idempotent (marker-guarded) and fails closed: a destination-collision or any per-item failure returns a `partial migration` error and does **not** write the marker, so a later retry can resume.
