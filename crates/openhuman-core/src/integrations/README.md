# integrations

Shared HTTP client, backend-proxied agent tools, and the Composio and
task-source sub-domains for third-party providers.

Search provider implementations live under `crates/openhuman-core/src/search/`. This module
keeps the backend-proxied integration client, the connector (Composio) and
task-source sub-domains, and the remaining non-search tool families.

## Responsibilities

- Provide `IntegrationClient`, a shared `reqwest` HTTP client for backend-proxied integrations: backend URL sanitization, bearer auth, `{success,data,error}` envelope parsing, bounded error-detail extraction, and pricing cache.
- Build the client from root config (`build_client`), resolving backend URL and app-session JWT; return `None` when the user is not signed in.
- Fetch per-integration pricing from `/agent-integrations/pricing`, with a Composio direct-mode short-circuit (`pricing_for_config`).
- Implement and export non-search, non-connector tools: Google Places, stock/market data, and Twilio.
- Own the [`composio`](composio/README.md) connector sub-domain and the [`task_sources`](task_sources/README.md) sub-domain as child modules.
- Own the [`file_storage`](file_storage/README.md) managed cloud file-storage tool family as a child module.
- Classify transport and user-state failures through `core::observability::report_error_or_expected`.

Every request `IntegrationClient` sends to the TinyHumans backend carries a
sanitized `x-sdk-name` (see `ProductIdentity` handling in `client_part_01.rs`
and `client_tests.rs::product_identity_seen_by_backend`), per AGENTS.md
"Backend API".

## Members

| Path | Role |
| --- | --- |
| `client.rs` + `client_part_01.rs` / `client_part_02.rs` | `IntegrationClient`: `post`/`get`/`get_bytes`/`patch`/`delete`/`upload_multipart`/`pricing`, backend URL sanitization, and client construction. |
| `types.rs` | Shared serde types for backend envelopes and pricing. |
| `tools.rs` | Non-search, non-connector agent tools: Google Places, stock/market data, Twilio. |
| [`file_storage/`](file_storage/README.md) | Managed cloud file-storage agent tools (`Storage*Tool`), backed by the backend's S3-based `file_storage` provider. |
| [`composio/`](composio/README.md) | Composio connector integration: catalogs, connections, triggers, direct-auth fallback, and the `tinyconnectors` module bridge. |
| [`task_sources/`](task_sources/README.md) | Normalizes external task feeds (via the Composio providers) into agent-facing list/fetch/filter tools. |
| `test_support.rs` + `test_support_backend.rs` | In-process fake integration backend (`spawn_fake_integration_backend`) used by `tools/ops_tests_part_02_tests.rs` / `ops_tests_part_03_tests.rs` to exercise integration tools without a real backend. |

## Key Files

| File | Role |
| --- | --- |
| `crates/openhuman-core/src/integrations/mod.rs` | Export-only module root. Declares the `client`, `composio`, `file_storage`, `task_sources`, `tools`, `types` submodules; re-exports `build_client`, `pricing_for_config`, `IntegrationClient`, pricing/envelope types. |
| `crates/openhuman-core/src/integrations/client.rs` | `IntegrationClient` construction and top-level methods; split across `client_part_01.rs` / `client_part_02.rs`. |
| `crates/openhuman-core/src/integrations/types.rs` | Shared serde types for backend envelopes and pricing. |
| `crates/openhuman-core/src/integrations/tools.rs` | Aggregates and re-exports the non-search, non-connector tool modules. |
| `crates/openhuman-core/src/integrations/tools/google_places.rs` | Google Places search + details. |
| `crates/openhuman-core/src/integrations/tools/stock_prices.rs` | Market data via backend financial APIs. |
| `crates/openhuman-core/src/integrations/tools/twilio.rs` | Outbound phone calls via backend Twilio. |

## Search Boundary

Search-owned tools are in `crates/openhuman-core/src/search/tools/`, including Parallel,
Brave, Querit, SearXNG, Seltz, TinyFish, and the managed `WebSearchTool`.
The search registry in `crates/openhuman-core/src/search/registry.rs` decides which search
tool surface is active for `search.engine`.

## Public Surface

From `crates/openhuman-core/src/integrations/mod.rs`:

- `IntegrationClient`
- `build_client(&Config) -> Option<Arc<IntegrationClient>>`
- `pricing_for_config(&IntegrationClient, &Config) -> IntegrationPricing`
- Types: `BackendResponse<T>`, `IntegrationPricing`, `IntegrationPricingEntry`, `PricingIntegrations`, `ToolScope`
- Non-search, non-connector tool structs via `tools.rs`

## Agent Tools

From `tools.rs`: `GooglePlacesDetailsTool`, `GooglePlacesSearchTool`,
`StockCommodityTool`, `StockCryptoSeriesTool`, `StockExchangeRateTool`,
`StockOptionsTool`, `StockQuoteTool`, `TwilioCallTool`. These are constructed
and registered by `crates/openhuman-core/src/tools/ops.rs`, gated by
`config.integrations.<provider>.is_active()`.

The `file_storage/` tools (see [its README](file_storage/README.md)) are
built separately via `build_file_storage_tools` (also called from
`tools/ops.rs`, around line 905) because they need `action_dir` and a
`SecurityPolicy` rather than a provider config flag.

Composio connector tools and task-source tools live in and are documented by
their own sub-domains; all three tool families are re-exported into the
global agent tool registry through `tools/mod.rs`:

```rust
pub use crate::integrations::composio::tools::*;
pub use crate::integrations::task_sources::tools::*;
pub use crate::integrations::tools::*;
```

Search tools, including TinyFish, are governed by `crates/openhuman-core/src/search/`.

## Notes

- Backend-proxied tools never see provider API keys; the backend holds them.
- Direct search APIs such as SearXNG, Brave, Querit, and Seltz are intentionally
  outside this module in `crates/openhuman-core/src/search/`.
- `IntegrationClient::new` re-runs backend URL sanitization as defense in depth.
- `IntegrationClient::pricing()` returns empty pricing on network error so tool
  registration does not fail.
</content>
