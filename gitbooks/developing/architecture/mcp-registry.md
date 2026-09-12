---
description: >-
  The dynamic, user-facing side of MCP-client support: discover servers on
  Smithery and the official MCP registry, persist installs to SQLite, supervise
  local-spawn subprocess lifecycle, surface their tools to agents via the
  unified tool registry.
icon: plug
---

# MCP Registry (`crates/openhuman-core/src/mcp/registry/`)

`crates/openhuman-core/src/mcp/registry/` is the **host half** of the dynamic, user-facing side of OpenHuman's Model Context Protocol client support: browsing the supported upstream registries (Smithery and the official modelcontextprotocol registry), installing a chosen server, persisting that choice, and (for servers launched as local subprocesses or HTTP-remote endpoints) supervising the connection lifecycle. Installed servers' tools are surfaced to agents via the unified tool registry (`crate::tools::registry`).

The **client half** — both transports, the Smithery/official catalogs, the SQLite store, the live connection map, the subprocess supervisor, browser sign-in, and the write-audit log — moved to the vendored `tinymcp` crate (`vendor/tinymcp`). What lives in this directory is only what belongs to this application:

- `host.rs` (one level up, `crates/openhuman-core/src/mcp/host.rs`): the one `tinymcp` service this process holds per workspace, and config-to-`tinymcp` conversion.
- `registry/`: the `mcp_clients` and `mcp_setup` RPC surface, the agent-facing tools, and the prompt-injection scan applied to remote tool definitions.
- `audit/` (sibling of `registry/`): the RPC surface over `tinymcp`'s write-audit log.
- `server/` (sibling of `registry/`): the `openhuman-core mcp` stdio/HTTP server that exposes this application's own tools to external MCP hosts — see [MCP Server](../mcp-server.md). This is the *server* side and did not move.

> **Naming note**: the Rust module path is `crate::mcp::registry` (`crates/openhuman-core/src/mcp/registry/`), but the RPC namespace and on-disk SQLite filename stay `mcp_clients` for backward compatibility with existing frontend code and stored user state. Grep both names when chasing call sites.

All payload types (`InstalledServer`, `McpTool`, `ConnStatus`, the Smithery/official-registry DTOs) come from `tinymcp_bus` and are re-exported under `registry::types`, not redefined here. Types for the *static, config-declared* server set (`[[mcp_client.servers]]` in `config.toml`) and the shared HTTP/stdio transport primitives are re-exported from `crate::mcp::config_servers` and `crate::mcp::http_client` (thin `pub use tinymcp::...` modules in `mcp/mod.rs`, not directories).

```text
                 ┌────────────────────────────────────────────────┐
   Registries ───►         tinymcp (catalogs, store, supervisor)   │
                 └────────────────────┬───────────────────────────┘
                                      │ browse / install
                                      ▼
                          ┌──────────────────────┐
   Frontend (Skills UI) ─►│  ops.rs / schemas.rs │  RPC controllers (mcp_clients_*)
                          └──────────┬───────────┘
                                     │ delegates to
                                     ▼
                          ┌──────────────────────┐
                          │   host::for_config    │  the tinymcp service this
                          │   / host::try_service │  workspace's host holds
                          └──────────┬───────────┘
                                     │ tools_safe_for_agent (prompt-injection scan)
                                     ▼
                          tool_registry (agents)

                          supervisor_events.rs ── reconnect-supervisor ticks → DomainEvent
```

## Server transport model

An `InstalledServer` carries a `transport: Transport` discriminator (`types.rs`) with two variants:

- **`Stdio`**: a local subprocess launched by `npx`, `uvx`, or a direct binary (see `types::CommandKind`), speaking **stdio JSON-RPC**.
- **`HttpRemote { url }`**: a hosted server (the majority of what Smithery lists), dialled over streamable HTTP by `mcp::http_client::McpHttpClient`.

`connections.rs` dispatches on the transport. Both the manual install dialog (`mcp_clients_install`) and the setup-agent path (`mcp_setup_install_and_connect`) pick the best connection via `setup_ops::pick_connection` (published stdio → any stdio → published http_remote → any http_remote) and build the transport with `setup_ops::build_install_transport`, so the two paths behave identically.

## Boot-time spawn

`boot::spawn_installed_servers` is called from `bootstrap_core_runtime` so every installed server is connected as soon as the core comes up. Errors are logged per-server and **never block boot**; a broken MCP install should not gate the desktop app starting. The lifecycle log subscriber (`bus::init`) is registered alongside the other domain subscribers in `register_domain_subscribers` so those connect events are observed.

## Layout

| Path                        | Role                                                                                                                                                                                                                                                       |
| --------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `types.rs`                  | Data structures: `InstalledServer`, `McpTool`, `ConnStatus`, Smithery DTOs, etc.                                                                                                                                                                           |
| `store.rs`                  | SQLite persistence: `mcp_clients.db`, CRUD over `InstalledServer` rows.                                                                                                                                                                                    |
| `registry.rs`               | Smithery HTTP client with a 10-minute SQLite cache so re-browsing doesn't hammer the upstream registry.                                                                                                                                                    |
| `registries/`               | Adapters for the upstream registries this code can browse: Smithery (`smithery.rs`) + the official modelcontextprotocol registry (`mcp_official.rs`). Each reads optional auth config-first with an env-var fallback (`mcp_client.registry_auth` in TOML). |
| `connections.rs`            | Global in-process connection registry. Wraps `crate::mcp::config_servers::McpStdioClient` (there is no separate stdio client implementation here).                                                                                              |
| `boot.rs`                   | Boot-time spawn (`spawn_installed_servers`) called from `bootstrap_core_runtime`.                                                                                                                                                                          |
| `setup.rs` / `setup_ops.rs` | "Setup agent" support: the small agent that walks a user through configuring a freshly installed server (env vars, secrets, first connect).                                                                                                                |
| `ops.rs`                    | RPC handler implementations (install, uninstall, list, browse, enable / disable, etc.).                                                                                                                                                                    |
| `schemas.rs`                | Controller schemas + handler dispatch. Re-exported from `mod.rs` as `all_mcp_registry_controller_schemas` / `all_mcp_registry_registered_controllers`.                                                                                                     |
| `bus.rs`                    | `DomainEvent` subscriber for lifecycle logging.                                                                                                                                                                                                            |

## Public surface

The exports from `mod.rs` are intentionally narrow:

```rust
pub use schemas::{
    all_controller_schemas as all_mcp_registry_controller_schemas,
    all_registered_controllers as all_mcp_registry_registered_controllers,
    schemas as mcp_registry_schemas,
};

pub use types::{ConnStatus, InstalledServer, McpTool};
```

Everything else (`boot`, `bus`, `connections`, `store`, `setup`, `setup_ops`) is `pub mod` for in-crate callers but not re-exported.

## Calls into

- `crate::mcp::config_servers::McpStdioClient`: the actual stdio transport.
- `crate::tools::registry`: installed servers' tools land here so agents see them alongside native tools.
- `memory_store` / workspace SQLite, for `mcp_clients.db` persistence.
- Smithery.ai HTTP, for registry browsing.

## Called by

- `bootstrap_core_runtime` (via `boot::spawn_installed_servers`).
- Frontend Skills UI: the **MCP** tab at `/skills?tab=mcp` (`McpServersTab`) dispatches through `ops.rs` over the `openhuman.mcp_clients_*` RPC namespace: browse, install (auto-connects), connect/disconnect, status, tool*call, `update_env` (reconfigure + reconnect), and `registry_settings_get` / `registry_settings_set` (Smithery / official-registry credentials; secret values are write-only). The agent-native flow uses `openhuman.mcp_setup*\*`via the`mcp_setup`sub-agent (orchestrator delegate`setup_mcp_server`).
- The setup agent in `setup_ops.rs`, for first-connect onboarding.

## Tests

Unit tests are co-located inline under `#[cfg(test)]` blocks in `store.rs`, `connections.rs`, and `setup.rs`. There is no dedicated `*_tests.rs` sibling per file (the convention in this domain is inline).

## Related

- [`mcp/registry/mod.rs`](https://github.com/tinyhumansai/openhuman/blob/main/crates/openhuman-core/src/mcp/registry/mod.rs): the authoritative rustdoc this page mirrors.
- `crates/openhuman-core/src/mcp/config_servers/` + `crates/openhuman-core/src/mcp/http_client/`: the transport library + static config-declared server set.
- [Agent Harness](agent-harness.md): how the agent ends up calling MCP tools through `tool_registry`.
- [Architecture overview](../architecture.md): where this fits in the wider system.
