# mcp/audit — write-audit RPC surface

RPC surface over the MCP write-audit log. The log itself — its store and
schema — moved to [`tinymcp`](https://github.com/tinyhumansai/tinymcp). What
is here is the `mcp_audit` controller family and the payload types it
speaks, re-exported from the wire contract.

## Where the rows are

The audit table used to be created inside this application's memory-tree
chunk database, which was precisely what made it unmovable. It has its own
file now, `<workspace>/mcp_audit/mcp_audit.db`, under the same workspace
directory. Rows written before the move stay in the old table: an audit log
is history rather than operational state, and nothing reads that table any
more.

## Key files

| File | Role |
| --- | --- |
| `mod.rs` | Facade: re-exports the payload types (`types` module, from `tinymcp_bus`), `record_write`/`list_writes` (delegating to the `mcp::host` service's `AuditStore`), and the schema re-exports. |
| `schemas.rs` / `schemas_tests.rs` | The `mcp_audit.list` controller: schema (`limit`/`offset`/`since_ms`/`client_filter`/`tool_filter`/`success_only` inputs, `records` output) and its handler. |
| `stub.rs` | The `mcp`-less mirror: `record_write` is a no-op returning `Ok(0)`, `list_writes` returns `Ok(vec![])`. |

## RPC surface

`mcp_audit.list` is the only controller, registered as **internal-only**
(`all_mcp_audit_internal_controllers`, used by the desktop UI/CLI, not the
public schema) at `core/all.rs` (~lines 1015-1020). Its filter and limit
semantics (`limit` default 50 / max 500, `offset`, `since_ms`,
`client_filter`, `tool_filter`, `success_only`) are enforced by the handler
in `schemas.rs` against the query it hands `list_writes`; whether the
underlying `tinymcp::AuditStore` applies those same bounds internally is the
contract's concern, not this layer's.

## Compile-time gate (`mcp` feature)

`pub mod audit;` is always compiled — it is a facade. The RPC surface
(`schemas`) is gated; with the `mcp` feature off, `stub` mirrors the
consumed surface (`record_write`, `list_writes`,
`all_mcp_audit_internal_controllers`) so the audited subsystem simply
appears never to have happened, rather than erroring.

## Dependencies

- `tinymcp` (`AuditStore`) — the write-audit store, opened per workspace by
  `crate::mcp::host`.
- `tinymcp_bus` — `McpWriteListQuery`, `McpWriteRecord`, `NewMcpWriteRecord`.
- `crate::mcp::host` — resolves the per-workspace `AuditStore` via
  `for_config`.
- `crate::core::all` — `ControllerSchema`, `RegisteredController` types for
  the internal-controller registration.
- `crate::rpc::RpcOutcome` — the response envelope other RPC domains use;
  `mcp_audit.list`'s handler returns a raw JSON object rather than
  `RpcOutcome` (see `schemas.rs::handle_list`).

## Used by

- `crates/openhuman-core/src/mcp/server/write_dispatch.rs` — calls
  `audit::record_write` for every MCP write-tool attempt (success and
  rejection), which opens the per-workspace `mcp::host` service and writes
  through its `AuditStore`.
- `crates/openhuman-core/src/core/all.rs` — registers
  `all_mcp_audit_internal_controllers()`.
