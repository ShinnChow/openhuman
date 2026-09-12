# tinyagents

The **adapter seam** between OpenHuman and the vendored [`tinyagents`](../../../../../vendor/tinyagents/) crate family (issue #4249). OpenHuman's agent turn no longer runs a hand-rolled tool-call loop; it drives the published `tinyagents` `AgentHarness` (LangGraph/LangChain-style durable graphs plus an agent-loop harness with model/tool registries, middleware, retry/fallback, and limits). This module bridges OpenHuman's `Provider`, `Tool`, and `ChatMessage` types onto the crate's `ChatModel`, `Tool`, and `Message` traits and enforces every OpenHuman-specific policy (approval, taint, redaction, budget) on the way in and out.

The chat/channel/sub-agent routes all call [`run_turn_via_tinyagents_shared`] (default ON in production) and are therefore guaranteed not to drift from each other. It is at functional parity with the legacy in-house engine: [`observability::OpenhumanEventBridge`] mirrors the harness event stream onto `AgentProgress` (live tool timeline, incremental text deltas, cost footer), native model streaming forwards true token streaming, multimodal markers are expanded, and history is trimmed/summarized to the context window. Mid-flight steering, sub-agent child-progress deltas (including thinking), and the `ask_user_clarification` early-exit pause are all re-wired onto the tinyagents harness.

## Responsibilities

- Assemble a per-turn harness ([`assemble_turn_harness`] in `mod_part_04.rs`): register the turn's `ChatModel`(s), every shared tool, and the full middleware stack, then drive it via `AgentHarness::invoke`.
- Convert between OpenHuman and crate types: `Provider` → `ChatModel` (`model.rs`), `Tool` → crate `Tool` (`tools.rs`, `convert.rs`), `ChatMessage`/`ConversationMessage` ↔ crate `Message` (via `crate::agent::message_convert`).
- Enforce cross-cutting policy as harness middleware: approval/security gating, tool allow-listing and CLI/RPC-only denial, cost budgets, context compaction/summarization, credential scrubbing, malformed-argument recovery, and the repeated-tool-failure circuit breaker (`middleware*.rs`).
- Route workloads to model tiers and record the resolved provider/model for audit (`routes.rs`, `resolved_route.rs`).
- Make turns durable and replayable: a JSONL event journal + status store (`journal.rs`) and a read-only RPC surface over it (`replay/`).
- Provide graph-layer helpers for multi-stage sub-agent orchestration (`orchestration.rs`, `delegation.rs`) and expose their structure for debugging (`topology.rs`).
- Host adapters (`host/`) for the crate's ten pluggable host-capability traits — not yet wired into the live turn path (see [Status](#host-adapters) below).

## Key files

| File / group | Role |
| --- | --- |
| `mod.rs` + `mod_part_01..05.rs` | Module root (`include!`-assembled). `mod_part_01.rs`: imports and re-exports (`SharedToolAdapter`, `TurnContextMiddleware`, `HandoffConfig`, …). `mod_part_02.rs`: `run_turn_via_tinyagents_shared`, the shared harness-drive entry point every caller uses. `mod_part_03.rs`: `TurnModelSource`/`TurnModels` — the per-turn crate `ChatModel` bundle built by `build_turn_models`. `mod_part_04.rs`: `assemble_turn_harness` — registers models, tools, and middleware in order. `mod_part_05.rs`: `record_unobserved_turn_usage`, the cost-tracker fallback for fire-and-forget turns. |
| `run_turn_via_tinyagents` | Legacy/simple entry point (see `mod_part_01.rs`); production traffic goes through `run_turn_via_tinyagents_shared`. |
| `host/` | OpenHuman's implementations of the crate's ten host-capability traits: `agent_memory`, `budget_gate`, `context_composer`, `definition_registry`, `experience_store`, `learning_sink`, `model_resolver`, `progress_sink`, `security_gate`, `tool_outcome_classifier`. Each file adapts one trait onto the OpenHuman domain that actually implements it. |
| `replay/` | Read-only agent-run replay/status RPC (`mod.rs`, `ops.rs`, `schemas.rs`) — three `agent`-namespace controllers over `journal.rs`. |
| `journal.rs` | Durable per-run event journal (`StoreEventJournal`) and status store (`HarnessStatusStore`) under `{workspace}/tinyagents_store`; runs alongside the live `observability` bridge, best-effort and non-fatal. |
| `reaper.rs` | Startup sweep that marks orphaned (non-terminal) runs `Cancelled` in the durable status store; the only *writer* over the status seam replay reads. |
| `middleware.rs` + `middleware_part_01..07.rs` | The named OpenHuman middleware stack. Parts hold, in rough order: tool-output/context budget middleware, `ToolPolicyMiddleware` (policy/permission enforcement), cost-budget pre-checks, inference/delegation failure-envelope detection, the `FinalCallWrapUpMiddleware` (issue #6014) and `CredentialScrubMiddleware`/`ToolPolicyMiddleware` split-outs (issue #4453, credential scrubbing on every tool result). |
| `model.rs` | OpenHuman wrappers over the crate `ChatModel` trait (streaming, usage translation, route recording). |
| `convert.rs` | Tool-schema conversion (`ToolSpec` → crate `ToolSchema`); durable message conversion lives in `crate::agent::message_convert`. |
| `tools.rs` | `SharedToolAdapter` — wraps `Arc<dyn crate::tools::Tool>` as a crate `Tool` so the harness invokes the exact tools the legacy loop ran. |
| `routes.rs` / `resolved_route.rs` / `topology.rs` | Workload-tier routing middleware, per-turn resolved provider/model bookkeeping, and graph topology export for debug/inspection. |
| `observability.rs` + `observability_part_01/02.rs` | `OpenhumanEventBridge` — translates crate `AgentEvent`s into `AgentProgress` and feeds per-call usage into the cost tracker. |
| `orchestration.rs` | Shared seam onto `tinyagents_graph::orchestration` task primitives for the detached-sub-agent control plane. |
| `delegation.rs` | OpenHuman-facing seam onto `tinyagents_graph::delegation`'s plan→execute⇄review→finalize graph. |
| `payload_summarizer.rs` | `PayloadSummarizer` trait + default sub-agent-backed impl; compresses oversized tool results instead of hard-truncating them. Public since issue #6014 so embedders can supply their own. |
| `policy_denial.rs` | Renders structured, actionable denial messages (what was blocked, why, workaround) for policy/permission-blocked tool calls; records into `crate::tools::registry::denials`. |
| `abort_guard.rs` | RAII `AbortOnDrop` guard tying a detached streaming-producer task's lifetime to its consumer stream, so cancellation actually stops the provider call. |
| `run_cancellation_context.rs` | Task-local carrier for the current run's `CancellationToken`, for tools that fan out nested graph work. |
| `steering_forwarder.rs` | Forwards OpenHuman's `RunQueue` steer/collect messages into the harness `SteeringHandle`; abort-on-drop so cancellation (drop-based, not just normal return) always deregisters it. |
| `stop_hooks.rs` | `StopHookMiddleware` — runs OpenHuman's `StopHook`s (budget cap, thread-goal budget, iteration ceiling) between model calls, pausing the run via steering on the first `Stop` decision. |
| `summarize.rs` | LLM-backed `Summarizer` + context-window-aware policy driving the crate's `ContextCompressionMiddleware`, replacing lossy front-trim-only truncation. |
| `embeddings.rs` | `ProviderEmbeddingModel` — adapts OpenHuman's `EmbeddingProvider` onto the crate's `EmbeddingModel` trait. |
| `retriever.rs` | Retrieval seam wrapping `Memory::recall`, projecting onto the crate's `ScoredDoc` shape and applying the `path_scope` dedupe rule. |
| `thread_context.rs` | Task-local ambient `thread_id` so the OpenAI-compatible provider can thread it into request bodies without touching every call site. |
| `todos.rs` | Opens the durable crate `Store` backing per-thread task boards (`tinyagents_graph::todos`). |
| `config.rs` | Maps OpenHuman's `Config` (including per-team/per-agent model pins) onto `tinyagents_harness::config` structs. |
| `*_tests.rs` | Sibling test suites for each file/part group above. |

## Public surface

Selected `pub`/`pub(crate)` items re-exported or defined at the module root (see `mod.rs` for the full `pub mod` list):

- **Turn entry points**: `run_turn_via_tinyagents`, `run_turn_via_tinyagents_shared`, `TurnModels`, `TurnModelSource`.
- **Config mapping**: `config::*` (public — the host half of the generic-harness config seam).
- **Payload summarization**: `payload_summarizer::*` — `pub` since issue #6014 so an embedder can supply `AgentBuilder::payload_summarizer` with its own `Arc<dyn PayloadSummarizer>` (the default dispatches a sub-agent, which some embedders cannot do).
- **Route metadata**: `resolved_route::ResolvedProviderRoute` (public — read by the agent bus after a turn to persist the actually-used provider/model).
- **Thread context**: `thread_context::*`.
- **Task boards**: `todos::todos_store`.
- **Host adapters**: `host::*` (`pub mod host`), all ten `OpenHuman*` adapter structs.

Everything else (`middleware`, `model`, `tools`, `journal`, `observability`, `orchestration`, `replay`, `reaper`, …) is `pub(crate)` or private — internal to the seam.

## RPC (`replay/`)

Namespace `agent`, three read-only controllers (workstream 05.x) wired through the standard registry, all over the durable journal/status stores in `journal.rs`:

| Method | Purpose |
| --- | --- |
| `openhuman.agent_run_events` | Paged late-attach replay of a run's durable event stream (`run_id`, `offset`, `limit`). |
| `openhuman.agent_run_status` | Latest `HarnessRunStatus` for a run, or `null` if unknown. |
| `openhuman.agent_runs_active` | Active runs, optionally filtered by `thread_id` and/or `root_run_id`. |

Responses project the crate's own `AgentObservation` and `HarnessRunStatus` serde shapes directly — no bespoke DTO — since both are already what the durable store persists as JSON. Neither carries prompt text, tool arguments, or provider payloads.

## Dependencies

- The vendored `tinyagents` crate family under `vendor/tinyagents/crates/` (`tinyagents-harness`, `tinyagents-graph`, `tinyagents-registry`, `tinyagents-session`, plus `tinyinference` message/model/tool types) — patched via a git submodule so upstream changes can be tested in-tree before being PR'd. Per AGENTS.md, use this vendored copy; a second path to the same types creates incompatible Rust types.
- `crate::agent::message_convert` — durable `ChatMessage` ↔ crate `Message` conversion.
- `crate::agent::harness::{run_queue, tool_result_artifacts}` and `crate::agent::messages` / `crate::agent::progress` — the OpenHuman-side turn plumbing this seam plugs into.
- `crate::tools` — the `Tool` trait wrapped by `SharedToolAdapter`, and `tools::registry::denials` recording policy blocks.
- `crate::platform::cost` — the global cost tracker fed by `observability.rs` and `mod_part_05.rs`.
- `crate::config` — model-tier constants and the `Config` mapped by `config.rs`.

## Used by

- `crates/openhuman-core/src/agent/harness/session/turn/graph.rs` — the chat-turn route into `run_turn_via_tinyagents_shared`.
- `crates/openhuman-core/src/agent/harness/graph.rs` — the channel/CLI bus turn route.
- `crates/openhuman-core/src/agent/harness/subagent_runner/ops/graph.rs` — the sub-agent spawn route (also uses `tinyagents::summarize` for context-window summarization ahead of front-trim).
- `crates/openhuman-core/src/tools/README.md` — documents `SharedToolAdapter` and `ToolPolicyMiddleware` as the primary tinyagents-side tool consumers.
- `crates/openhuman-core/src/flows/tinyflows/` — flow tool/checkpoint compatibility code references this seam.

## Host adapters — status

`host/` implements ten crate host-capability traits (`docs/specs/plan-agents.md` Phase 4), each collapsing what used to be direct calls into ~45 OpenHuman domains down to ten seams. The adapters are implemented and tested, but **`agent/` does not call them yet** — repointing the live call sites is the remaining half of Phase 4 and is gated on other in-flight work (the session still holds `AgentConfig` until Phase 2's reader flip; `builder/factory.rs` is split rather than repointed). Some adapter methods carry `TODO(phase4)` where a domain surface was not yet reachable; these are documented gaps, not silent stubs.

## Notes

- The taint/scope/redaction/approval/egress-budget guarantees the crate deliberately does not know about are enforced entirely in `host/` and `middleware*.rs`. Widening a permission or dropping a scope filter to make an adapter signature fit would silently disable a guarantee the rest of the system assumes.
- `journal.rs` and `reaper.rs` are best-effort: journal writes and the startup sweep swallow errors behind a `[journal]` log line rather than failing a turn.
- `defer_turn_completed_to_caller` (a `run_turn_via_tinyagents_shared` parameter) exists because the chat/session path emits its own post-run `TurnCompleted` after streaming a checkpoint; callers without that extra step (channel/CLI) rely on this seam's own emit.
- [gitbooks/developing/architecture/agent-harness.md](../../../../../gitbooks/developing/architecture/agent-harness.md) is the narrative overview of the whole turn lifecycle and where this seam sits in it; as of this writing it still references a `SqlRunLedgerCheckpointer` at `tinyagents/checkpoint.rs`, which no longer exists in this directory — that page needs a follow-up fix.
