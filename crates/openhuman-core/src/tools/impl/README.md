# tools/impl

Built-in implementations of the cross-cutting tool families that `tools/ops.rs`
assembles into the agent registry. Per the ownership rule in `AGENTS.md`: only
genuinely cross-cutting capabilities (filesystem, browser, generic
process/system, generic network, document/presentation generation, tool-search
meta tools) live here. Domain-owned tools (memory, cron, wallet, composio,
codegraph, integrations, voice, agent sub-dispatch) live in their own domain's
`tools.rs` and are only re-exported through `tools/mod.rs` — do not add a new
family under `impl/` for a domain capability.

`impl/mod.rs` aggregates the seven families and gates `document`/`presentation`
behind the `documents` Cargo feature; the other five are unconditional. See
[`../README.md`](../README.md) for the registry, policy, and RPC layer this
feeds, and [`../../security/README.md`](../../security/README.md) for the
`SecurityPolicy` threaded through nearly every tool here (path validation,
trusted roots, egress gating).

## Families

| Family | Tools | Registration gate |
| --- | --- | --- |
| `system/` | `ShellTool`, `NodeExecTool`, `NpmExecTool`, `PythonExecTool`, `InstallToolTool`, `DetectToolsTool`, `CurrentTimeTool`, `ResolveTimeTool`, `ScheduleTool`, `ProxyConfigTool`, `PushoverTool`, `LspTool`, `ToolStatsTool`, `UpdateCheckTool`, `UpdateApplyTool`, `InsertSqlRecordTool`, `WorkspaceStateTool`, plus internal `command_output`/`retrieve_tool_output` helpers | `node_exec`/`npm_exec` and `shell`'s PATH injection require `node.enabled`; `LspTool` requires the `OPENHUMAN_LSP_ENABLED` env var (`lsp_capability_enabled`) |
| `filesystem/` | `FileReadTool`, `FileWriteTool`, `EditFileTool`, `ApplyPatchTool`, `GrepTool`, `GlobTool`, `ListFilesTool`, `ReadDiffTool`, `CsvExportTool`, `GitOperationsTool`, `RunLinterTool`, `RunTestsTool`, `UpdateMemoryMdTool` | unconditional |
| `browser/` | `BrowserTool` (DOM-snapshot automation, pluggable backend), `BrowserOpenTool`, `ImageInfoTool` | `browser.enabled`; allow-all reach requires `OPENHUMAN_BROWSER_ALLOW_ALL`, otherwise `browser_allowed_domains` narrows the shared `http_request` allowlist by stripping `"*"`. Ships a Playwright backend (`playwright_backend.rs` + `playwright_runner.mjs`) alongside `native_backend.rs` |
| `network/` | `HttpRequestTool`, `WebFetchTool`, `CurlTool`, `GitbooksSearchTool`/`GitbooksGetPageTool`, `GmailUnsubscribeTool`, and (feature `mcp`) `McpListServersTool`/`McpListToolsTool`/`McpCallTool` + the `mcp_setup` family (`McpSetupSearchTool`, `McpSetupGetTool`, `McpSetupInstallAndConnectTool`, `McpSetupRequestSecretTool`, `McpSetupTestConnectionTool`); `url_guard` is the shared SSRF/allowlist validator, not a tool | `gitbooks_*` requires `gitbooks.enabled` plus the docs-domain MCP allowlist; the `mcp`/`mcp_setup` tools are compiled only under the `mcp` Cargo feature |
| `document/` (feature `documents`) | `DocumentTool` (`generate_document`) | builds a `.docx` via the native `engine` module (no subprocess); writes an artifact through `agent::artifacts` (`create_artifact`/`finalize_artifact`/`fail_artifact`, kind `Document`) and persists `args.json` for the Retry path |
| `presentation/` (feature `documents`) | `PresentationTool` (`generate_presentation`) | builds a `.pptx` via the native `engine` module; same artifact lifecycle as `document/` (kind `Presentation`), so both stay parallel producers |
| `meta/` | `ToolSearchTool` (`tool_search`), `collapse` helpers (`resolve`, `strictest_permission`, `any_external_effect`, `unknown_action_message`, `CollapsedAction`) | unconditional; `tool_search` is the lookup half of `ToolExposure::Deferred` — it lets the model discover tools that are registered but hidden from its default catalog. `collapse` merges a multi-action tool's per-action schemas/permissions into one entry |

## Notes

- `document/` and `presentation/` are deliberately kept structurally parallel
  (validate input → allocate artifact → generate bytes off the async runtime
  via `spawn_blocking` + timeout → finalize/fail the artifact) so future
  artifact-producing tools can follow the same shape.
- `meta/` is not under `system/` because it isn't a host capability the user
  cares about — it's the model introspecting its own tool surface.
- Tool struct names above are re-exported through `impl/mod.rs` and then
  `tools/mod.rs`; import via `crate::tools::*`, not `crate::tools::implementations::<family>::*`.
