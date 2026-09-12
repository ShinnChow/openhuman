# Flows

Saved automation workflows — the graphs a user builds on the canvas or the
copilot builds for them. Owns CRUD/enable/run/resume/cancel for saved flows,
the trigger → run bridge, the authoring tools (propose/create/edit/validate/
dry-run/save), discovery/suggestion tools, and the medulla workflow-plane
bridge. Does NOT own the workflow engine itself (`tinyflows`, vendored) or the
state-graph runtime it lowers onto (`tinyagents`, via
`crates/openhuman-core/src/agent/tinyagents/`).

See [gitbooks/developing/architecture/flows-on-tinyagents.md](../../../../gitbooks/developing/architecture/flows-on-tinyagents.md)
for how one flow run lowers onto tinyagents; this file is the directory map.

## Gate shape — leaf, not facade

The whole family (`flows` + `flows::tinyflows`) is gated at `pub mod flows;`
in `crates/openhuman-core/src/lib.rs` behind `#[cfg(feature = "flows")]`, and
every submodule inherits that gate. There is deliberately **no `stub.rs`**:
every symbol reached from outside is a registration site —
`core::all::all_flows_registered_controllers`, `core::jsonrpc`'s
`FlowTriggerSubscriber`, `core::runtime::services`' boot reconcile
(`sweep_orphaned_running_runs_on_boot`, `reconcile_schedule_triggers_on_boot`),
`medulla_bridge::install`, the agent-tool `vec!` in `tools::ops`, and the
`workflow_builder` / `flow_discovery` built-ins in `agent_registry`. A
registration site wants *absence* when the feature is off, not a
disabled-error stub, or `flows.*` becomes a known method that fails at
runtime. See `voice/` for the facade+stub shape used when a domain is called
from always-compiled code.

## Public surface

- `pub mod ops` (split `ops_part_01..12.rs`) — CRUD (`flows_create/get/list/update/delete/duplicate/import/validate`), run lifecycle (`flows_run`, `flows_run_detached`, `flows_resume`, `flows_cancel_run`, `flows_list_runs`, `flows_get_run`, `flows_prune_runs`), `flows_set_enabled` (arms/disarms the trigger via `cron::add_flow_schedule_job` for schedule triggers and `bus`'s trigger-config helpers for the rest), builder (`flows_build`, `flows_build_cancel`), discovery (`flows_discover`, `flows_list_suggestions`, `flows_dismiss_suggestion`, `flows_mark_suggestion_built`), drafts (`flows_draft_create/get/update/list/delete/promote`), and boot reconciliation (`sweep_orphaned_running_runs_on_boot`, `reconcile_schedule_triggers_on_boot`).
- `pub mod bus` — `FlowTriggerSubscriber`: matches `DomainEvent::FlowScheduleTick` / `ComposioTriggerReceived` / `WebhookIncomingRequest` against enabled flows' trigger nodes and spawns `ops::flows_run`; its matching helpers are reused by `flows_set_enabled` to bind/unbind dispatch.
- `pub mod medulla_bridge` — backs the medulla harness protocol's workflow plane (`platform::socket::medulla::workflows::WorkflowBridge`) with this store: projects saved `Flow`s onto the wire `WorkflowDescriptor`, serves the three read RPCs, and runs a `workflow_builder` copilot turn with host-enforced approval guards (creates always `require_approval: true`, updates never lower it, automatic-trigger creates are saved disabled).
- `pub mod catalogue` — lists saved flows as `Workflow` entries with `WorkflowScope::Flow` in the shared skill catalogue, so `skill_search` sees one list instead of skills and flows separately.
- `pub mod node_contracts` — host overlay on `tinyflows::catalog`'s node-kind contracts: attaches host-specific facts (which `tool_call` slugs resolve to Composio vs. native `oh:` tools, which trigger kinds actually dispatch here) without touching the portable contracts. Re-exports `all_node_kind_contracts`, `node_kind_contract`, `NODE_KINDS`, `ConfigField`, `PortSpec`, `NodeKindContract`.
- `mod store` / `mod draft_store` (private) — bind `tinyflows_sqlite::flows` / `tinyflows_sqlite::drafts` to `<workspace_dir>/flows`; `kv_get`, `kv_set`, and `upsert_flow_run_step` are re-exported from `store` for the `tinyflows::caps::FlowStateStore` seam and the run observer.
- `mod schemas` (private, re-exported) — RPC/CLI controller surface under the `flows` namespace; handlers split across `schemas_handlers.rs`, `flows_schema_part_01.rs` (create/get/list/update/delete/run/resume/cancel_run/list_runs/get_run/prune_runs/build/build_cancel/discover/list_suggestions/dismiss_suggestion and more), and `flows_schema_part_02.rs` (mark_suggestion_built/approval_manifest/required_connections/search_tool_catalog/get_tool_contract/get_history/rollback/draft_create/draft_get/draft_update/draft_list/draft_delete/draft_promote) — 37 functions total.
- `pub mod tools` — `ProposeWorkflowTool`, `RunFlowTool`.
- `pub mod builder_tools` (split `builder_tools_part_01..07.rs`) — the authoring toolset: `ReviseWorkflowTool`, `EditWorkflowTool`, `ValidateWorkflowTool`, `GetFlowHistoryTool`, `ListFlowRunsTool`, `ResumeFlowRunTool`, `CancelFlowRunTool`, `CreateWorkflowTool`, `DuplicateFlowTool`, `ListConnectableToolkitsTool`, `ListFlowsTool`, `GetFlowTool`, `GetFlowRunTool`, `ListFlowConnectionsTool`, `SearchToolCatalogTool`, `GetToolContractTool`, `GetToolOutputSampleTool`, `ListAgentProfilesTool`, `ListNodeKindsTool`, `GetNodeKindContractTool`, `DryRunWorkflowTool`, `SaveWorkflowTool`.
- `pub mod discovery_tools` — `SuggestWorkflowsTool`.
- `pub mod memory_tools` — `FlowMemoryRecallTool`, `FlowMemoryRememberTool`, plus `flow_namespace` / `FLOW_MEMORY_NAMESPACE_PREFIX` / `cross_flow_recall` (re-exported from `mod.rs` because the tinyflows `memory` node's `OpenHumanMemory` adapter needs byte-identical `scope: "flows"` results).
- `pub mod agents` — first-class built-in sub-agents: `workflow_builder` (authoring copilot) and `flow_discovery` (read-only suggestion scout), registered as `BUILTINS` in `agent/registry/agents/loader.rs`.
- `pub mod skills` (needs both `flows` and `skills` features) — bundles `skills/flow-authoring/WORKFLOW.md`, a skill teaching flows authoring.
- `pub mod tinyflows` — the capability seam (`caps/`) implementing `tinyflows`'s traits over real OpenHuman services, plus `observability.rs` (`FlowRunObserver`), `memory_adapter.rs`, and `langfuse_export.rs`.
- Re-exported model types (from `tinyflows_catalog`, not owned here): `Flow`, `FlowConnection`, `FlowDraft`, `FlowImport`, `FlowRevision`, `FlowRun`, `FlowRunStep`, `FlowRunTrigger`, `FlowSuggestion`, `FlowValidation`, `FlowValidationError`, `SuggestionStatus`, `DraftOrigin`, plus `types`, `run_registry`, `build_registry`, and `n8n_import` (the format importer).

## Calls into

- `vendor/tinyflows/` — the actual workflow model, validation, compilation, and run engine; this domain never re-implements it.
- `crates/openhuman-core/src/agent/tinyagents/` — the state-graph engine both the agent harness and tinyflows lower onto.
- `crates/openhuman-core/src/cron/` — `add_flow_schedule_job` arms a schedule-triggered flow as a `JobType::Flow` cron job; the scheduler fires it by publishing `DomainEvent::FlowScheduleTick`, which `bus::FlowTriggerSubscriber` picks up.
- `crates/openhuman-core/src/platform/socket/medulla/workflows/` — `WorkflowBridge` trait implemented by `medulla_bridge`.
- `crates/openhuman-core/src/skills/` — catalogue integration (`catalogue.rs`) and the shared skill-bundle mechanism used by `skills/flow-authoring/`.
- `crates/openhuman-core/src/memory/` — `memory_tools`/`tinyflows::memory_adapter` read/write agent memory under the `flows` scope.

## Called by

- `crates/openhuman-core/src/core/all.rs` — registers `all_flows_registered_controllers()` under `#[cfg(feature = "flows")]`.
- `crates/openhuman-core/src/core/jsonrpc.rs` — constructs `flows::bus::FlowTriggerSubscriber` at startup.
- `crates/openhuman-core/src/core/runtime/services.rs` — runs `sweep_orphaned_running_runs_on_boot` and `reconcile_schedule_triggers_on_boot` during core boot.
- `crates/openhuman-core/src/tools/ops.rs` — registers `ProposeWorkflowTool` and `RunFlowTool` in the agent tool list.
- `crates/openhuman-core/src/agent/registry/agents/loader.rs` — registers `workflow_builder` and `flow_discovery` as built-in archetypes.

## Tests

- Unit: `*_tests.rs` colocated with nearly every top-level file (`ops_tests*`, `bus_tests*`, `builder_tools_tests*`, `catalogue_tests.rs`, `discovery_tools_tests.rs`, `medulla_bridge_tests.rs`, `memory_tools_tests.rs`, `node_contracts_tests.rs`, `schemas_tests.rs`, `store_tests_part_01..03.rs`, `tools_tests.rs`, `types_tests.rs`), plus `import_tests.rs` for the n8n importer.
- `tinyflows/` has its own suite: `checkpoint_compat_tests.rs`, `memory_adapter_tests.rs`, `memory_node_e2e_tests.rs`, `observability_tests.rs`, `tinyflows_tests.rs`.

## Related docs

- [gitbooks/developing/architecture/flows-on-tinyagents.md](../../../../gitbooks/developing/architecture/flows-on-tinyagents.md) — the run pipeline, capability seam, and two-layer security model.
- [`../cron/README.md`](../cron/README.md) — `JobType::Flow` and the schedule-trigger binding.
