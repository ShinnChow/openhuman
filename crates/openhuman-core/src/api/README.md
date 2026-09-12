# API

OpenHuman-side wrapper over the vendored `tinyhumans-sdk`: URL resolution,
session-token retrieval, product attribution, the authenticated REST client,
and the Socket.IO handshake URL for the TinyHumans / AlphaHuman hosted
backend. Route implementations themselves live in
`vendor/tinyhumans-sdk`, not here — add a missing backend route there.

## Layout

| File | Purpose |
| --- | --- |
| `config.rs` | Backend/inference URL resolution and local-vs-hosted classification |
| `jwt.rs` | Session-token load and `Authorization` header formatting |
| `product.rs` | `x-sdk-name` product-attribution header |
| `rest.rs` | `BackendOAuthClient`, typed `BackendApiError`, and the error-classification chokepoint |
| `socket.rs` | Socket.IO (Engine.IO v4) WebSocket URL construction |
| `models/` | Serde DTOs shared across auth and realtime call sites |

## `config.rs`

Single source of truth for every URL the app uses to reach the hosted backend
or an LLM inference endpoint. Two families are resolved separately because a
`config.api_url` pointed at a local model runner (Ollama, vLLM, LM Studio)
only speaks `/v1/chat/completions` and 404s on every other path:

- `effective_api_url` / `effective_inference_url` — chat/inference base,
  falling back through `config.api_url` → `BACKEND_URL`/`VITE_BACKEND_URL` env
  → compile-time `option_env!` → environment default.
- `effective_backend_api_url` — base for all control-plane calls (auth,
  billing, team, integrations, voice, sockets, …). Skips the user's
  `api_url` override when it `looks_like_local_ai_endpoint` or
  `looks_like_inference_provider_endpoint` and is not the OpenHuman backend
  itself, so pointing `api_url` at Ollama or `openrouter.ai` doesn't also
  misroute `/teams/me/usage` and billing calls there.
- `normalize_api_base_url` — strips path/query/fragment so relative joins
  resolve correctly.
- `DEFAULT_API_BASE_URL` (`https://api.tinyhumans.ai`) /
  `DEFAULT_STAGING_API_BASE_URL`, selected via `OPENHUMAN_APP_ENV` /
  `VITE_OPENHUMAN_APP_ENV` (`api_base_from_env`).
- `looks_like_local_ai_endpoint` / `looks_like_inference_provider_endpoint` —
  heuristics documented in-file; both are intentionally tight to avoid
  misclassifying real custom backends or ephemeral test mock servers.

## `jwt.rs`

`get_session_token` reads the current session token out of the credentials
store (`crate::security::credentials`). Token *parsing* and header
*formatting* — `bearer_authorization_value`, `decode_jwt_payload` — are
re-exported from `tinyhumans_sdk::jwt` rather than reimplemented, since they
are properties of the backend's token format that every host needs.
`decode_jwt_exp` wraps the SDK's Unix-seconds `exp` decoder in the `chrono`
type the credentials store uses, so an expired token can be rejected locally
instead of round-tripping to a guaranteed 401.

## `product.rs`

`ProductIdentity` / `set_product_identity` attach a sanitized `x-sdk-name`
header (`PRODUCT_IDENTITY_HEADER`) to every backend-bound request, so the
backend can attribute calls to OpenHuman, OpenCompany, or Medulla even though
all three share one login and reach the backend through this crate. The
identity is process-wide (a `OnceLock<RwLock<ProductIdentity>>`), not a
constructor parameter, because `BackendOAuthClient` is built at dozens of
call sites across domains. **Call `set_product_identity` once at startup,
before building any backend client** — `BackendOAuthClient` and
`IntegrationClient` bake the identity into their default headers at
construction and do not pick up a later change (`MedullaClient` reads it
per-request, but do not rely on that difference). A build that never calls
the setter sends `DEFAULT_PRODUCT_IDENTITY` (`"openhuman"`).

Tests across `api::product`, `api::rest`, `medulla`, and `integrations` all
touch this process-global state; `product_identity_test_lock` (test-only)
serializes them to avoid cross-module races.

## `rest.rs`

`BackendOAuthClient` wraps `tinyhumans_sdk::TinyHumansClient` with an
OpenHuman-configured `reqwest::Client` (product-identity default headers,
normalized base URL). Key surface:

- `authed_json` / `fetch_billing_summary` — send an authenticated request and
  route the result through `finish_authed_json`.
- `connect`, `login_url`, `url_for`, `raw_client` — OAuth connect flow and
  URL helpers for callers that need to drive a non-JSON request (e.g.
  multipart uploads) without re-implementing TLS/proxy setup.
- `ConnectResponse`, `IntegrationSummary`, `IntegrationTokensHandoff` — typed
  backend response shapes.
- `decrypt_handoff_blob` — AES-256-GCM decrypt for integration token handoff,
  compatible with the backend's `encryptMessageFromString`.

`BackendApiError` is the typed-error surface `authed_json` callers should
match on for expected backend states rather than treating as failures:
`Unauthorized` (401 — session lapsed, not a bug), `MessageNotFound` (404 on a
channel message the provider or backend already deleted),
`ChannelEditUnsupported` (404 because the backend never implemented the
`PATCH` edit route), `AnnouncementNotFound` (404 on the best-effort
announcements fetch). `flatten_authed_error` maps `Unauthorized` onto the
`SESSION_EXPIRED` JSON-RPC sentinel so the dispatcher classifies it as session
expiry instead of reporting it to Sentry.

The private `BackendOAuthClient::finish_authed_json` is the error
classification chokepoint for every `authed_json`/`fetch_billing_summary`
call: it walks the `reqwest`/`hyper`/`rustls` error source chain (not just the
top-level message) to distinguish a transient transport failure from one
worth reporting, and turns specific status/path combinations into the typed
`BackendApiError` variants above. There is no separate `classify_sdk_error`
function — this method is the equivalent chokepoint.

## `socket.rs`

`websocket_url` converts an `http(s)` API base into the Engine.IO v4
WebSocket URL (`wss://…/socket.io/?EIO=4&transport=websocket`) the realtime
client connects to.

## `models/`

Serde DTOs (`auth.rs`, `socket.rs`) shared by auth and realtime call sites —
see [`models/mod.rs`](models/mod.rs) for the full list.

## Backend request rules (from `AGENTS.md`)

- Add missing backend routes to `vendor/tinyhumans-sdk`, not to `src/api/`.
- Every TinyHumans backend request must carry a sanitized `x-sdk-name`:
  `BackendOAuthClient`, `IntegrationClient` (except redirected file
  downloads), `MedullaClient` (including its separate SSE handshake),
  desktop `GET /auth/me`, and the agent's Langfuse ingestion request.
- Never add `x-sdk-name` to third-party endpoints, MCP servers, BYOK
  inference endpoints, or presigned storage redirects.
- When auditing hand-built backend requests, grep for
  `bearer_authorization_value` and `header(AUTHORIZATION`.

## Called by

77 files under `crates/openhuman-core/src` reference `crate::api::`. Heaviest
consumers: `platform/socket/` (realtime client), `hosted/*` (billing,
referral, announcements, team), `medulla/client/`, `integrations/` and
`integrations/composio/`, `channels/controllers/` and `channels/bus*.rs`,
`security/credentials/` (token storage feeding `jwt::get_session_token`), and
`inference/provider/`, `inference/embeddings/`, `inference/voice/` (backend
inference proxy and cloud transcription/embeddings).

## Tests

`rest_tests.rs` covers `BackendOAuthClient`, `BackendApiError` classification,
and `decrypt_handoff_blob`. `jwt.rs` and `product.rs` carry their own
`#[cfg(test)]` modules.
