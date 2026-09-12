# ops

RPC handlers for the memory system — each returns `RpcOutcome<T>`
(`crate::rpc::RpcOutcome`, `openhuman_rpc::RpcOutcome` re-exported through
`pub use openhuman_rpc as rpc;` in `crates/openhuman-core/src/lib.rs`).
`memory::ops` is re-exported flat from `crate::memory::mod` (`pub use
ops::*;`), and `memory::rpc` is an alias of this module (`pub use ops as
rpc;`) kept for call sites that predate the tinymemory-core extraction.

## Submodules

| Module | RPC family |
| ------ | ---------- |
| `envelope.rs` | `ApiEnvelope`/`ApiError` wrapping shared by every envelope-style handler (init, list_documents, query_namespace, recall_*, ai_*_memory_file). |
| `helpers.rs` | Formatting, default constants, path validators, and `active_memory_client` — the unguarded driver lookup used where no typed contract twin exists yet. |
| `guard.rs` | `active_memory_guard` — the guarded-driver lookup handlers use instead of `helpers::active_memory_client` when the operation has a typed contract twin (`docs/specs/memory-guard-allowlist.md`). |
| `documents.rs` | Document/namespace direct API and the envelope-style façade (`memory_init`, `memory_list_documents`, `memory_query_namespace`, `recall_*`). |
| `kv_graph.rs` | Key-value and knowledge-graph handlers. |
| `sync.rs` | `memory_sync_*` and `memory_ingestion_status`. |
| `learn.rs` | `memory_learn_all`. |
| `provider.rs` | `memory_provider_status` / `memory_subsystem_status` — reports what the memory driver slot is bound to; deliberately bypasses the guard (a liveness probe is not product code). |
| `files.rs` | `ai_*_memory_file` handlers (`tokio::fs`). |
| `maintenance.rs` | Scheduler-driven housekeeping against the `Maintenance` capability family. |
| `tool_memory.rs` | Tool-scoped rule read/write handlers. |
| `test_support/` | `shared_memory_test_workspace` — one shared, leaked workspace so concurrent family tests agree on a path instead of racing to bind different ones. |

## The ops ↔ schemas mirror

`memory::ops` and [`memory::schemas`](../schemas/) mirror each other
one-to-one by RPC family (`documents`, `kv_graph`, `sync`, `learn`,
`provider`, `files`, `tool_memory`, plus `core_recall`/`ingest` split out of
`documents`' schema side). `schemas` defines the wire-facing
`ControllerSchema`s and thin handler glue; `ops` holds the actual business
logic each handler calls into. Each schema family publishes its own
`all_<family>_controller_schemas()` / `all_<family>_registered_controllers()`
pair so `core::all` can register one capability family at a time rather than
the namespace as a whole.

Do not confuse `memory::schemas/` (this mirror, one submodule per `ops`
family) with `memory::schema/` (singular) — that is a different module: the
controller-schema *definitions* (`definitions.rs`), handler glue
(`handlers.rs`) and registry (`registry.rs`) that stayed host-side from
before the family split, exposing its own `all_controller_schemas` /
`all_registered_controllers`.

## Wiring

`crates/openhuman-core/src/core/all.rs` (around lines 730–790) registers each
family behind its own alias re-exported from `memory::mod`:
`all_memory_core_recall_registered_controllers`,
`all_memory_documents_registered_controllers`,
`all_memory_ingest_registered_controllers`,
`all_memory_files_registered_controllers`,
`all_memory_kv_graph_registered_controllers`,
`all_memory_sync_registered_controllers`,
`all_memory_learn_registered_controllers`,
`all_memory_provider_registered_controllers` (never capability-gated — it is
what *reports* the bound driver's capabilities), and
`all_memory_tool_memory_registered_controllers`. Registering a subset (or
none) is how a build can turn a capability family off without losing the
`memory.provider_status` surface that explains why.

## Tests

Each family has a `*_tests.rs` sibling (`documents_tests.rs`,
`envelope_tests.rs`, `files_tests.rs`, `guard_tests.rs`, `helpers_tests.rs`,
`kv_graph_tests.rs`, `learn_tests.rs`, `maintenance_tests.rs`,
`provider_tests.rs`, `sync_tests.rs`, `tool_memory_tests.rs`), plus the
legacy `../ops_tests.rs` (gated on `feature = "modules"`) that predates the
per-family split and exercises private helpers via `super::*`. Tests that
drive the process-wide memory client serialize on
`GLOBAL_MEMORY_TEST_LOCK` to avoid racing on one SQLite connection.
