# sources

The registry of connectors a workspace ingests from (Composio, folders,
GitHub repos, RSS, web pages, Twitter queries), the readers that pull items
out of them, per-source sync status, and the `memory_sources_*` JSON-RPC
surface over all three. See `mod.rs` for the #5560 rationale (why this used
to glob `tinymemory_core::sources::*` and what came home versus stayed
upstream) — this file is a map of the directory, not a repeat of that
history.

The vocabulary is the engine-neutral `tinymemory-sources` crate: `pub mod
types { pub use tinymemory_sources::types::{ContentType,
MemorySourceEntry, SourceContent, SourceItem, SourceKind}; }` at the bottom
of `mod.rs`.

## Files

| File | Role |
| ---- | ---- |
| `registry.rs` | Config discovery and write-locking around the source registry's CRUD. Reads/rewrites `[[memory_sources]]` in the host's own config file; the registry type itself is `tinymemory_sources::registry::SourceRegistry`. |
| `rpc.rs` + `rpc_part_0{1,2}.rs` | RPC handler implementations for memory sources. |
| `schemas.rs` + `schemas_part_0{1,2}.rs` | Controller-registry schemas for `openhuman.memory_sources_*`. |
| `status.rs` | Per-source sync status: the chunk-key prefix (derived from the registry entry) and freshness label are host-side; in-flight/chunk counts go through `MemoryChunks::source_ingest_status`. |
| `sync.rs` | `derive_scopes` — which tree scope and raw-archive id a configured source maps onto; the only production-reached piece of the old engine sync pipeline. |
| `reconcile.rs` | Startup/list-time reconciliation of active Composio connections into the registry, built on `memory::sync::composio::scan_active_sync_targets`. |
| `readers/mod.rs` | `SourceReader` trait plus one implementation per `SourceKind`: `composio`, `conversation`, `folder`, `github`, `rss`, `twitter`, `web_page`. Network readers (`twitter`, `web_page`, ...) are never handed out by kind-dispatch — a caller constructs them explicitly, keeping egress/OAuth/cost decisions with the host. |

## RPC surface (`memory_sources_*`)

`list`, `get`, `add`, `update`, `remove`, `list_items`, `read_item`, `sync`,
`reconcile`, `status_list`, `supported_toolkits`, `sync_audit_log`,
`estimate_sync_cost`, `monthly_cost_summary`, `apply_all_in`,
`coding_session_status`, `ingest_coding_sessions`.

## Wiring

`crates/openhuman-core/src/core/all.rs` (around line 844) registers this
family through `crate::memory::sources::all_memory_sources_registered_controllers()`,
re-exported from `schemas::all_registered_controllers`.

## Related modules

- [`../sync/`](../sync/) — the bus-driven side: Composio subscribers and
  providers (including Slack) that actually move data, plus `sync_status/`
  for per-connection sync progress. `sources/` owns *which* connectors are
  configured and *what* they map onto; `sync/` owns moving data for them.
- [`../read_rpc/`](../read_rpc/) — dashboard reads (list/inspect/search) over
  the memory tree; distinct from the write/ingest surface here.

## Tests

`rpc_tests.rs`, `rpc_budget_tests_tests.rs`, `rpc_filter_tests_tests.rs`,
`rpc_monthly_summary_tests_tests.rs`, `rpc_supported_toolkits_tests_tests.rs`,
`schemas_tests.rs`, `status_tests.rs`, `sync_tests.rs`, `reconcile_tests.rs`,
and per-reader tests under `readers/` (`composio_tests.rs`,
`twitter_tests.rs`).
