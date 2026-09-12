# Search Tools

Agent-facing `Tool` implementations for every search provider. One file per
provider family; `mod.rs` re-exports the full public surface with
`pub use crate::search::tools::*` consumed from `tools/mod.rs`.

## Providers

| File | Exported tools | `name()` | Transport |
| --- | --- | --- | --- |
| `brave.rs` | `BraveWebSearchTool`, `BraveNewsSearchTool`, `BraveImageSearchTool`, `BraveVideoSearchTool` | `web_search_tool`, `brave_news_search`, `brave_image_search`, `brave_video_search` | Direct to `api.search.brave.com`, `X-Subscription-Token` header |
| `exa.rs` | `ExaSearchTool`, `ExaFindSimilarTool`, `ExaGetContentsTool` | `exa_search` or `web_search_tool` (constructor-selected), `exa_find_similar`, `exa_get_contents` | BYOK, direct to `api.exa.ai`, `x-api-key` header — never proxied |
| `parallel.rs` (+ `parallel_part_01.rs`, `parallel_part_02.rs`) | `ParallelSearchTool`, `ParallelExtractTool`, `ParallelChatTool`, `ParallelResearchTool`, `ParallelEnrichTool`, `ParallelDatasetTool` | `parallel_search`, `parallel_extract`, `parallel_chat`, `parallel_research`, `parallel_enrich`, `parallel_dataset` | Backend-proxied via `crate::integrations::IntegrationClient` (`/agent-integrations/parallel/*`) |
| `querit.rs` | `QueritSearchTool` | `querit_search` or `web_search_tool` (constructor-selected) | Direct to `api.querit.ai`, `Authorization: Bearer` header |
| `searxng.rs` | `SearxngSearchTool`, plus `normalize_categories`, `SearxngSearchArgs`, `SearxngSearchResponse`, `MAX_RESULTS` (re-exported as `SEARXNG_MAX_RESULTS`) | `searxng_search` | Direct to a user-configured, self-hosted SearXNG instance (`GET /search?format=json`) |
| `seltz.rs` | `SeltzSearchTool` | `seltz_search` | Direct to `api.seltz.ai`, `x-api-key` header |
| `tavily.rs` (+ `tavily_part_01.rs`, `tavily_part_02.rs`) | `TavilySearchTool`, `TavilyExtractTool` | `tavily_search` or `web_search_tool` (constructor-selected), `tavily_extract` | BYOK, direct to `api.tavily.com`, `Authorization: Bearer` header — never proxied |
| `tinyfish.rs` | `TinyFishSearchTool`, `TinyFishFetchTool`, `TinyFishAgentRunTool` | `tinyfish_search`, `tinyfish_fetch`, `tinyfish_agent_run` | Backend-proxied via `IntegrationClient` (`/agent-integrations/tinyfish/*`); search/fetch are read-oriented, agent-run drives goal-based browser automation |
| `web_search.rs` | `WebSearchTool` (+ crate-internal `resolve_managed_provider`) | `web_search_tool` | Backend-proxied managed search; resolves the actual provider (Exa by default) from the backend response for UI attribution |

Several providers construct their primary tool with a `tool_name` field so the
same struct can register under either its own name (e.g. `exa_search`,
`querit_search`, `tavily_search`) or the canonical `web_search_tool` slot when
that engine is the active `search.engine` — see `exa.rs`, `querit.rs`, and
`tavily_part_02.rs`.

## Registration

Most families are selected by `search::registry::build_search_tools` based on
`Config.search.engine`, via `search/engines/`
([README](../README.md)). `tinyfish.rs` tools are pushed separately by
`registry.rs` whenever `config.integrations.tinyfish.is_active()`, independent
of the chosen search engine.

`SearxngSearchTool` and `SeltzSearchTool` are not reachable through the engine
registry at all — they are constructed directly by the `tools_searxng_search`
(`tools/schemas_part_02.rs:41`) and `tools_seltz_search`
(`tools/schemas_part_01.rs:555`) RPC handlers, one call at a time, from
per-request arguments rather than saved config. `SEARXNG_MAX_RESULTS` and
`normalize_categories` are also reused by `mcp/server/tools/` to keep the MCP
SearXNG surface consistent with the RPC handler.

## Tests

Each provider has a sibling `*_tests.rs` (`brave_tests.rs`, `exa_tests.rs`,
etc.) wired in via `#[cfg(test)] mod tests;`. Tests that need a backend use the
shared mock backend described in `AGENTS.md`, not real network calls.
