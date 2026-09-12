# Agent

Multi-agent orchestration domain. Owns the LLM tool-calling loop, sub-agent dispatch, conversation transcripts, the trigger-triage pipeline that classifies incoming external events, and the bundled prompt assets in `agent/prompts/`. Does NOT own provider HTTP transport (`crates/openhuman-core/src/inference/provider/`), tool implementations (`tools/`), or memory storage (`memory/`).

## Public surface

- `pub struct Agent` / `pub struct AgentBuilder` — `harness/session/types.rs` — top-level conversation runtime; entry point for any chat turn.
- `pub mod harness::session::{builder, runtime, turn}` — turn lifecycle, fluent builder, `run_single` / `run_interactive`.
- `pub fn run_subagent` / `pub struct SubagentRunOptions` / `pub enum SubagentRunError` — `harness/subagent_runner/` — execute a hierarchical sub-agent from a parent tool loop.
- `pub struct AgentDefinition` / `pub struct AgentDefinitionRegistry` / `pub enum SandboxMode` / `pub enum ToolScope` — `harness/definition.rs` — sub-agent archetypes loaded from built-ins + workspace TOML.
- `pub mod harness::fork_context` — task-local parent context for KV-cache reuse.
- `pub trait ToolDispatcher` / `pub struct ParsedToolCall` / `pub struct ToolExecutionResult` — `dispatcher.rs` — pluggable tool-call format (XML / JSON / P-Format).
- `pub mod triage` (`run_triage`, `apply_decision`, `TriggerEnvelope`, `TriageDecision`, `TriageAction`) — `triage/mod.rs` — classify external triggers, escalate to sub-agents.
- `pub mod prompts::SystemPromptBuilder` — `prompts/` — system-prompt section composer.
- `pub struct ChatMessage` / `pub enum ConversationMessage` / `pub struct ToolResultMessage` — `messages.rs` — transcript wire types (not provider transport, which lives in `inference/provider/`).
- Built-in archetypes live in `crates/openhuman-core/src/agent/registry/agents/`; this module stays focused on harness/runtime behavior.
- RPC `agent.{chat, chat_simple, server_status, list_definitions, get_definition, reload_definitions, triage_evaluate, graph_topologies, registry_snapshot}` — `schemas.rs`.
- Read-only replay RPC `agent.{runs_active, run_status, run_events}` — `tinyagents/replay/schemas.rs` — pages a run's durable journal/status without holding the run open.

## Submodule map

| Path | Purpose |
| --- | --- |
| `artifacts/` | Agent-generated artifact storage, retrieval, and lifecycle ([README](artifacts/README.md)) |
| `context/` | Prompt section assembly, tool-call format selection ([README](context/README.md)) |
| `debug/` | Debug/introspection helpers for agent internals |
| `experience/` | Local procedural operating experience capture for self-learning ([README](experience/README.md)) |
| `file_state/` | Tracks files an agent has read/written within a turn |
| `git_attribution/` (`pub(crate)`) | Attributes agent-authored commits/diffs to the acting agent |
| `harness/` | `Agent`/`AgentBuilder`, session lifecycle, sub-agent runner, fork context — the tool-calling loop itself |
| `harness_init/` | One-time first-run provisioning (Python/spaCy/Node) before the harness can run |
| `learning/` | Reflection, tool-outcome tracking, user-profile inference from transcripts ([README](learning/README.md)) |
| `library/` | Shared agent-authored content library |
| `orchestration/` | Command center, workflow runs, agent teams, worktrees, subagent control ([README](orchestration/README.md)) |
| `plan_review/` | Interactive plan-review gate that parks a live turn on a thread-scoped plan |
| `profiles/` | Persistent agent profiles (name, soul, memory sources, skills, MCP, connectors) |
| `progress_tracing/` | Structured OpenTelemetry/Langfuse-style spans off the `progress::AgentProgress` stream |
| `prompts/` | Prompt types, section builders, `SystemPromptBuilder` |
| `registry/` | User-facing agent registry: defaults, enablement, custom agents, tool policy; `registry/agents/` holds built-in archetypes |
| `session_db/` | Durable run ledger backing `run_ledger` RPC |
| `session_import/` | Importing session/run history from external sources |
| `task_dispatcher/` | Background board poller that turns queued tasks into agent runs |
| `tinyagents/` | Integration with the vendored `tinyagents` loop/replay crate; owns `tinyagents/replay/schemas.rs` |
| `tools/` | Agent-domain tool implementations (`spawn_subagent`, dispatch helpers, etc.) |
| `triage/` | Classifies external `TriggerEnvelope`s and escalates to sub-agents |

Flat files: `bus.rs` (event subscribers), `cost.rs` (`pub(crate)`, token/cost accounting), `dispatcher.rs` (tool-call format dispatch), `error.rs`, `hooks.rs`, `host_runtime.rs` (native shell execution backend), `messages.rs` (transcript types), `multimodal.rs`, `pformat.rs`, `platform_shell.rs` (cross-platform shell selection shared with `host_runtime` and `sandbox::ops`), `progress.rs` (`AgentProgress` channel), `progress_sink.rs` (task-local progress sink for in-process embedders), `stop_hooks.rs`, `task_board.rs`, `task_session.rs` (`pub(crate)`), `tool_policy.rs`, `turn_origin.rs` (task-local trust/routing label read by the approval gate), `turn_workspace.rs` (task-local per-turn filesystem root).

## RPC namespaces owned by this tree

`agent`, `agent_registry`, `profiles`, `harness_init`, `session_import`, `plan_review`, `run_ledger` (session_db), `agent_experience` (experience), `ai` (artifacts), `learning`, `agent_team`, `agent_work` (orchestration/command_center), `workflow_run`, `worktree`, `subagent` (orchestration/subagent_control) — all registered under `DomainGroup::Agent` in `core/all.rs`.

`crate::rpc` is `pub use openhuman_rpc as rpc` in `lib.rs`; shared RPC contracts, response decoding, and the HTTP client now live in the separate `crates/openhuman-rpc` crate, not under `agent/`.

## Calls into

- `crates/openhuman-core/src/inference/provider/` — `Provider` trait sends/receives against LLMs; `ChatMessage`/`ConversationMessage` (defined in `agent/messages.rs`) cross this boundary.
- `crates/openhuman-core/src/tools/` — `Tool` / `ToolSpec` execution surface invoked from the tool loop.
- `crates/openhuman-core/src/memory/` — episodic indexing + memory-loader context injection.
- `crates/openhuman-core/src/agent/context/` — prompt sections, tool-call format selection.
- `crates/openhuman-core/src/inference/local/` — `agent_chat` / `agent_chat_simple` execution backend.
- `crates/openhuman-core/src/config/` — runtime config load via `config::rpc::load_config_with_timeout`.
- `crates/openhuman-core/src/core/bus.rs` (`BUS.publish`/`BUS.subscribe`) and `crates/openhuman-core/src/core/events.rs` (`DomainEvent`) — emits `DomainEvent::Agent(*)` and `Trigger*` events; subscribers in `agent/bus.rs`.

## Called by

- `crates/openhuman-core/src/channels/runtime/dispatch/` (`processor*.rs`, `routing.rs`) and `web_chat/` — drive chat turns from inbound channel messages.
- `crates/openhuman-core/src/cron/scheduler_part_02.rs::run_agent_job` — builds an `Agent` directly via `Agent::from_config_for_agent[_with_profile]` / `Agent::from_config` and delivers output through `deliver_if_configured`; it does not go through triage.
- `crates/openhuman-core/src/skills/webhooks/{ops,bus}.rs` — webhook ingestion routes through `triage::run_triage` + `apply_decision`.
- `crates/openhuman-core/src/memory/sync/composio/bus*.rs` — Composio trigger envelopes go through `agent::triage`.
- `crates/openhuman-core/src/integrations/task_sources/route.rs` — external task-source events go through the same `TriggerEnvelope` → `run_triage` → `apply_decision` path.
- `crates/openhuman-core/src/desktop/notifications/rpc.rs` — desktop notification intel routes through triage before surfacing agent runs to the UI.
- `crates/openhuman-core/src/agent/schemas.rs::triage_evaluate` — dry-run triage entry point exposed over RPC.
- `crates/openhuman-core/src/agent/learning/{reflection,tool_tracker,user_profile}.rs` — read transcripts + tool outcomes.
- `crates/openhuman-core/src/agent/orchestration/tools/{dispatch,spawn_subagent}.rs` — `spawn_subagent` tool delegates here.
- `crates/openhuman-core/src/core/runtime/services.rs` — starts `agent::task_dispatcher::start_board_poller` and runs `agent::harness_init::run_harness_init` during core startup.
- `crates/openhuman-core/src/core/all.rs` — controller registry wires all `agent`, `agent_registry`, `profiles`, `harness_init`, `plan_review`, `artifacts`, `experience`, `learning`, `session_db`, `session_import`, and `orchestration` controllers under `DomainGroup::Agent`.

## Tests

- Unit: `agent_tests.rs` + `agent_tests_part_0N_tests.rs`, `multimodal_tests.rs` + `multimodal_tests_part_0N_tests.rs`, `dispatcher_tests.rs`, plus `*_tests.rs` files colocated with `bus.rs`, `cost.rs`, `error.rs`, `hooks.rs`, `host_runtime.rs`, `message_convert.rs`, `pformat.rs`, `platform_shell.rs`, `progress_sink.rs`, `schemas.rs`, `stop_hooks.rs`, `task_board.rs`, `task_session.rs`, `tool_policy.rs`, `turn_origin.rs`, `turn_workspace.rs`, and under `harness/`, `harness/session/`, `triage/`.
- Integration: `tests/agent_builder_public.rs`, `tests/agent_harness_public.rs`, `tests/agent_harness_e2e.rs`, `tests/agent_multimodal_public.rs`, `tests/agent_turn_overrides_e2e.rs`, `tests/agent_approval_memory_coverage_e2e.rs`.
- Schema regression: `schemas_tests.rs` (`controller_schema_inventory_is_stable`).

## Related docs

- [gitbooks/developing/architecture/agent-harness.md](../../../../gitbooks/developing/architecture/agent-harness.md)
- [gitbooks/developing/agent-observability.md](../../../../gitbooks/developing/agent-observability.md)
