# Search Domain

Top-level home for web search selection and agent-facing search tool registration.

## Shape

- `registry.rs` builds the active search tool surface from `Config.search`.
- `engines/` contains one file per search engine (`managed`, `parallel`, `brave`, `querit`, `exa`, `tavily`, and `disabled`) so provider-specific registration stays isolated.
- `tools/` contains all search-owned agent tools: `WebSearchTool`, Parallel, Brave, Querit, Exa, Tavily, SearXNG, Seltz, and TinyFish.
- Search tools may use the shared `IntegrationClient` for backend-proxied requests, but their implementations live in this module.

## Engine Behavior

`search.engine` accepts:

- `disabled` — register no search tools.
- `managed` — register backend-proxied `web_search_tool`.
- `parallel` — register the Parallel family plus `web_search_tool` when configured.
- `brave` — register Brave web/news/image/video search when configured.
- `querit` — register Querit search plus `web_search_tool` when configured.
- `exa` — BYOK: register `exa_search`, `exa_find_similar`, `exa_get_contents` plus `web_search_tool` when configured. Calls go directly to `https://api.exa.ai` with the user's own key, never through the managed backend.
- `tavily` — BYOK: register `tavily_search` (web/news/finance, search depth, time-range/date filters, domain include/exclude) and `tavily_extract` plus `web_search_tool` when configured. Calls go directly to `https://api.tavily.com` with the user's own key, never through the managed backend.

A BYO engine with no key configured falls back to the managed surface, so `managed` stays the effective default until a key is saved.

When search is disabled, search tools are absent from the agent runtime tool list, so they do not render in agent context.

## Wiring

- `search::build_search_tools` (`registry.rs`) is called once from
  `tools/ops.rs:891` to assemble the search slice of the agent tool list.
- `tools/mod.rs:47` re-exports `crate::search::tools::*`, so callers reach
  search tools through `crate::tools` rather than importing `crate::search`
  directly.
- TinyFish tools are not tied to `search.engine`: `registry.rs` pushes them
  whenever `config.integrations.tinyfish.is_active()`, on top of whichever
  engine is active.
- `SearxngSearchTool` and `SeltzSearchTool` bypass the engine registry
  entirely. They are constructed per-request by the `tools_searxng_search`
  (`tools/schemas_part_02.rs:41`) and `tools_seltz_search`
  (`tools/schemas_part_01.rs:555`) RPC handlers, using arguments from the RPC
  call rather than saved `Config.search` state.

## `engines/`

`engines` is `pub(crate)`. Each file (`managed`, `parallel`, `brave`,
`querit`, `exa`, `tavily`, `disabled`) exports a single
`pub(crate) fn build(root_config: &Config, params: SearchToolParams) -> Vec<Box<dyn Tool>>`
that constructs that engine's tool set — e.g. `managed::build` wraps a single
`WebSearchTool` pointed at the backend-proxied `web_search` endpoint.
`registry.rs` matches `search.effective_engine()` to pick exactly one.

See [`tools/README.md`](tools/README.md) for the provider -> tool -> transport
table, and
[`gitbooks/features/native-tools/web-search.md`](../../../../../gitbooks/features/native-tools/web-search.md)
for the user-facing description of engine selection.
