# embeddings

Host policy and RPC surface for vector embeddings. Concrete OpenAI-compatible,
Cohere, Voyage, Ollama, cloud-transport, retry, and rate-limit implementations
live in `tinyinference::embeddings`; this domain selects and adapts those
models for OpenHuman.

## Host-owned responsibilities

- `factory.rs`: provider slug/model/dimension selection and construction of
  TinyAgents models.
- `provider_trait.rs`: compatibility contract used by existing OpenHuman
  consumers plus `TinyAgentsEmbeddingProvider`, the one trait adapter.
- `cloud_adapter.rs`: OpenHuman session-token resolution, egress disclosure,
  and local-only privacy enforcement around TinyAgents `CloudEmbeddingModel`.
- `catalog.rs`, `rpc.rs`, `schemas.rs`: Settings catalog, credentials, JSON-RPC,
  connection tests, and re-embed/wipe policy.
- `noop.rs`: unit-struct compatibility adapter over TinyAgents' no-op model.

Provider HTTP behavior and its tests belong in `vendor/tinyagents`. Do not add
new per-provider clients here.

The canonical embedding-space signature is
`provider={name};model={model};dims={dims}`. Configuration-derived and live
provider signatures must remain byte-identical or stored vectors split into
incompatible spaces.

The separately compiled TinyMemory module reaches this factory through the
`EmbeddingHost` bus interface in `modules/memory_host.rs`, which resolves the
API key and calls `create_embedding_provider_with_config` per request; its
fixed on-disk dimension remains 1024 (see `memory/tree/health` mismatch
errors). Ollama requests use TinyAgents' `RECOMMENDED_OLLAMA_CONTEXT_TOKENS`
for both `num_ctx` and `num_batch`.

## Wiring

- Controllers are registered from `all_embeddings_registered_controllers()`,
  called by `core/all.rs` alongside the other domain registrations.
- RPC handlers live in `schemas.rs` and dispatch through `rpc.rs`.
- See `mod.rs` for the current provider set (Managed default via the backend's
  `POST /openai/v1/embeddings`, Voyage, OpenAI, Cohere, Ollama, Custom, Noop)
  and `tinyinference::embeddings` for their concrete implementations.
