# tree

Host layer over the memory tree engine, which now lives in the TinyMemory
module (see [`memory/README.md`](../README.md) for the extraction). Every
handler and schema here names OpenHuman's `RpcOutcome` and `ControllerSchema`,
which the engine crate cannot see, so this directory is what stayed behind:
RPC surface, not tree mechanics.

`mod.rs` carries the module's own #5560 accounting for why the old `pub use
tinymemory_core::tree::*` glob was deleted — read it there rather than here
for that history. What resolves under `memory::tree::…` today is the four
submodules below plus the controller-registry re-exports `mod.rs` aggregates
from them (they cannot live in the extracted crate alongside the rest of
`tree`, since aggregation is inherently host-side).

## Submodules

| Module | Role |
| --- | --- |
| [`tree/`](tree/) | Host surface over what used to be `tinymemory_core::tree::tree` — the per-source summary tree's persistence-adjacent RPC (`rpc.rs`, `canonicalize_types.rs`). |
| [`tree_runtime/`](tree_runtime/) | The markdown time tree's host surface: JSON-RPC handlers (`ops.rs`, `schemas.rs`), the `tree-summarizer` CLI (`cli.rs`), and an event subscriber (`bus.rs`). Reaches the tree through the contract's six runtime doors (`runtime_buffer_write`, `runtime_read_node`, `runtime_read_children`, `runtime_tree_status`, `runtime_summarize`, `runtime_rebuild`) rather than building an engine config host-side. |
| [`retrieval/`](retrieval/) | Host layer over `tinymemory_core::tree::retrieval`: LLM-callable retrieval primitives (`query_source`, `cover_window`, `search_entities`, `drill_down`, `fetch_leaves`) as `memory_tree.*` JSON-RPC methods, delegating to the bound driver via `crate::memory::api::provider::retrieval`. |
| [`health/`](health/) | The pipeline failure taxonomy (`FailureCode`, `FailureClass`, `PipelineFailure`, `DegradedState`) and the `user_error` wire payload for a web channel, plus the doctor `report` that reads the bound driver's `MemoryMaintenance::{diagnose, degraded_state}`. Not to be confused with `tinymemory_api::health::MemoryHealth` (driver liveness) — see the module doc for the name collision. |

## Registered RPC namespaces

`mod.rs` re-exports three controller registries, wired into `core/all.rs`:

- `all_memory_tree_registered_controllers` (from [`memory/schema/`](../schema/),
  `mod.rs`'s `pub use crate::memory::schema::{...}`) — the core `memory_tree`
  namespace's ingest/query controllers.
- `all_retrieval_registered_controllers` (`retrieval/schemas.rs`) — Phase 4
  retrieval tools, also under the `memory_tree` namespace (kept in the same
  namespace deliberately, "to keep the tool surface tightly grouped with the
  Phase 1-3 ingest controllers"): `query_source`, `cover_window`,
  `search_entities`, `drill_down`, `fetch_leaves`.
- `all_tree_summarizer_registered_controllers` (`tree_runtime/schemas.rs`) —
  the `tree_summarizer` namespace: `ingest`, `run`, `query`, `status`,
  `rebuild`.

All three are registered in `crates/openhuman-core/src/core/all.rs` alongside
the rest of the RPC registry.

## Related surfaces

- [`memory/query/`](../query/) is the agent-facing `memory_tree` tool
  (`MemoryQueryTool`) that calls into this directory's retrieval RPC through
  `memory/query/backend.rs`, not directly into `retrieval::rpc`.
- [`memory/read_rpc/`](../read_rpc/) is the Memory tab dashboard's read
  surface; it shares the `memory_tree` namespace with the controllers
  registered here rather than defining its own.
