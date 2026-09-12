# Medulla

OpenHuman as a Medulla **client**: the HTTP/SSE surface that talks to the
Medulla orchestration backend, the wire vocabulary it speaks, and the
harness-contract types the client and the harness share. Do not confuse this
with `crates/openhuman-core/src/platform/socket/medulla`, which is the opposite
direction — OpenHuman as a Medulla **worker** answering inbound Socket.IO from
a remote operator. A single binary can be both at once; see `mod.rs` for the
full split.

Gated on the `medulla` Cargo feature and tagged
`DomainGroup::Medulla` (`crate::core::all::DomainGroup`) at runtime. `contract`
and `events` are ungated carve-outs: they are inert serde/std types with no
runtime coupling, and `crates/openhuman-embed/src/` names them in public
signatures, so gating them would take the embed facade down with them.

The `medulla_local` engine (a supervised `medulla-serve` child process) has
been removed; its `subconscious.engine = "medulla"` config keys are still
accepted as inert serde (`config::schema::subconscious`) so existing configs
keep booting while that behaviour is re-ported onto this domain.

## Layout

- `mod.rs` — feature gate, the client/worker split, `RESERVED_TOOL_NAMES` (the
  harness's built-in memory/task-tracker tool names a module author must not
  collide with).
- `contract.rs` — ungated. `WorkerContract` and `VerificationEvidence`: the
  advisory boundaries and completion evidence Medulla transports verbatim for
  a delegated worker lane. `camelCase` wire shapes.
- `events/` — ungated. `SessionEvent` (with an `Unknown` fallback so a newer
  backend never drops rows on an older host) and `EventEnvelope`; `types.rs`
  is the data model, `serde_impl.rs` the compact-JSON codec. Presentation
  (transcript rendering, last-message lookup) deliberately stays out — that's
  the host's concern.
- `client/` — `MedullaClient` / `MedullaClientBuilder`, `DEFAULT_BASE_URL`.
  Unwraps the backend's `{success, data}` envelope; API errors surface as
  `ClientError::Api`, preserving `errorCode`. Submodules: `account`,
  `sessions`, `orchestration`, `routing` (`RoutingStrategy`), `program`,
  `feedback` (the public feedback board), `sse` (event streaming — attaches
  the `x-sdk-name` product-identity header itself in its connect path, since
  the SSE handshake authenticates via `?token=` and never reaches the
  client's normal `authed()` helper), and `types`/`error`.
- `chat/` — on-disk chat thread-tree store (`medulla_chat`), migrated from
  medulla-public. Kept separate from `threads/` and `session_db/`: its on-disk
  format is a live user-data contract, and folding it into an existing store
  would mean migrating every existing chat tree.
- `resolve.rs` — resolves a configured `MedullaClient` from ambient config and
  credentials. There is no `[medulla]` config section: the Medulla API and the
  OpenHuman backend are the same deployment, so `api_url` and the existing
  session token already address it.
  `OPENHUMAN_MEDULLA_BASE_URL` overrides the base URL for pointing a dev host
  at a different Medulla deployment.
- `ops.rs` / `schemas.rs` — the `medulla` RPC namespace: `medulla_status`,
  `medulla_roster`, `medulla_create_session`, `medulla_get_session`,
  `medulla_list_sessions`, `medulla_list_messages`, `medulla_send_message`,
  `medulla_list_events`, `medulla_abort`. Handlers delegate straight to `ops`;
  transport and client-construction failures become `StructuredRpcError`s so a
  host can branch on a stable `data.kind`.

## Called by

- `crates/openhuman-core/src/flows/medulla_bridge.rs` — bridges the
  `platform::socket::medulla` worker's workflow events to this domain's
  session/event surface; never talks to a runtime worker directly.
- `crates/openhuman-core/src/api/product.rs` — `MedullaClient` is one of the
  callers required to send `x-sdk-name` on every backend request, including
  its separate SSE handshake (see AGENTS.md).
- `crates/openhuman-core/src/desktop/app_state/` — reads the resolved config
  snapshot `MedullaClient` uses per request.

## Tests

`mod_tests.rs`, `resolve_tests.rs`, `ops_tests.rs`, `schemas_tests.rs`,
`chat/chat_tests.rs`, `client/tests/`, `client/routing_routing_strategy_tests_tests.rs`,
`client/feedback/feedback_tests.rs`, `events/tests/`.
