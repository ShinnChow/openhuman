# provider

Native TinyAgents `ChatModel` construction plus cloud/local inference policy,
auth, error taxonomy, and RPC helpers for every chat-model transport OpenHuman
supports. Was previously `providers/` (pre-consolidation `src/openhuman/`
layout); see `../README.md` for how this fits into the wider `inference`
domain.

## Public surface

- **Factory** (`factory.rs` + `factory_part_0{1..4}.rs`) — `create_chat_model`,
  `create_chat_model_from_string[_with_model_id]`,
  `create_chat_model_with_model_id`, `provider_for_role`, `role_for_model_tier`,
  `probe_inference_readiness`, `BYOK_INCOMPLETE_SENTINEL`. Parses the
  provider-string grammar (`openhuman`, `cloud`, `ollama:<model>`,
  `lmstudio:<model>`, `mlx:<model>`, `omlx:<model>`, `local-openai:<model>`,
  `claude_agent_sdk[:<model>]`, `claude-code:<model>`, `<slug>:<model>[@<temp>]`)
  and applies BYOK/access gates before building a model.
- **Models** — `OpenHumanBackendModel` + `PROVIDER_LABEL`
  (`openhuman_backend_model.rs`), plus the OpenAI-compatible and Anthropic
  crate-native builders (`crate_openai.rs`, `crate_anthropic.rs`).
- **DTOs** (`types.rs`) — `ChatRequest`, `ChatResponse`, `ProviderDelta`,
  `ToolCall`, `UsageInfo`, `AGENT_TURN_MAX_OUTPUT_TOKENS`.
- **Error classifiers** — `billing_error::is_budget_exhausted_message`,
  `chat_template::is_chat_template_rejection_message`,
  `config_rejection::{is_openai_compatible_unknown_model_message,
  is_provider_config_rejection_message}`, `error_code::{BackendErrorCode,
  extract_backend_error_code*, backend_error_code_skips_sentry, ...}`.

## Transports

| Transport | File | Provider-string prefix |
| --- | --- | --- |
| Managed OpenHuman backend | `openhuman_backend_model.rs` | `openhuman` / `cloud` (session JWT + billing metadata) |
| OpenAI-compatible (BYOK cloud slugs, local runtimes) | `crate_openai.rs` | `<slug>:<model>`, `ollama:<model>`, `lmstudio:<model>`, `mlx:<model>`, `omlx:<model>`, `local-openai:<model>` |
| Anthropic Messages API (prompt caching) | `crate_anthropic.rs` | `<anthropic-slug>:<model>` |
| Codex OAuth / Responses API | `openai_codex.rs` (`pub(crate)`) | resolved from the `openai` cloud slug once Codex OAuth is connected |
| Claude Agent SDK subprocess | `claude_agent_sdk/` (`protocol.rs`, `subprocess.rs`) | `claude_agent_sdk` / `claude_agent_sdk:<model>` |
| Claude Code CLI subprocess | `claude_code/` — see its own [README](claude_code/README.md) | `claude-code:<model>` |

## Calls into

- `tinyinference::model::ChatModel` (`vendor/tinyagents/vendor/tinyinference`)
  — the trait every transport implements.
- `crate::config` — cloud-provider schema (`AuthStyle`, slug reservation),
  `Config::claude_agent_sdk`, abstract tier model constants.
- `crate::security::credentials` — auth-profile store for BYOK keys and OAuth
  tokens.
- `crate::agent::tinyagents::{routes, thread_context}` — workload routing and
  ambient thread-context plumbing consumed while building a model.
- `crate::inference::auth_error_registry` — surfaces per-provider auth errors
  back to the UI.
- `crate::core::bus` (`BUS.publish`) / `crate::core::events::DomainEvent` —
  `ops/http_error_part_02.rs::publish_backend_session_expired` publishes
  `DomainEvent::SessionExpired` when the managed backend reports an auth
  failure, so the credentials layer can clear/refresh the session.
- `crate::mcp::server::local` (via `claude_code/driver.rs`) — the Claude Code
  provider points the sandboxed `claude` subprocess at the in-process MCP
  server so it can reach OpenHuman's memory/tools over loopback without the
  MCP server inheriting CC's OS jail.

## Called by

`grep -rn 'inference::provider::' crates/openhuman-core/src` shows the main
consumers: the agent harness (`agent/harness/session/builder/factory.rs`,
`agent/harness/session/runtime*.rs`, `agent/harness/subagent_runner/ops/*`),
`web_chat/session.rs`, `voice/factory/{helpers,mod}.rs`,
`inference/ops.rs`/`inference/schemas_part_0{1,2}.rs`, and
`inference/embeddings` (through the shared model-resolution helpers).

## Sub-modules with their own docs

- [`ops/`](ops) — `sanitize` (secret scrubbing), `http_error` (HTTP error
  classification, Sentry routing, `api_error`), `models`
  (`list_configured_models`), `provider_factory` (`ProviderRuntimeOptions`,
  `list_providers`, China-provider alias helpers). Preserves the original
  `pub use ops::*` contract split out of a single `ops.rs`.
- [`claude_code/`](claude_code/README.md) — Claude Code CLI provider.
- `claude_agent_sdk/` — Claude Agent SDK subprocess provider
  (`protocol.rs` wire types, `subprocess.rs` process management).

## Tests

- `factory_tests*.rs`, `factory_tests_part_0{1,2,3}_tests.rs`,
  `factory_test_provider_override_tests.rs` — provider-string parsing, access
  gates, and model construction.
- `ops_tests*.rs`, `ops/http_error_tests.rs`, `ops/models_tests.rs` — error
  classification and model listing.
- `error_classify_tests.rs`, `error_code_tests.rs`, `config_rejection_tests.rs`,
  `billing_error_tests.rs`, `chat_template_tests.rs`,
  `fallback_diagnostics_tests.rs` — per-classifier behavior.
- `claude_code/*_tests.rs` — per-file coverage of the CC provider (auth,
  driver, event mapper, stream parser, session store, settings, version
  check).
- `crate_openai_tests.rs`, `crate_anthropic_tests.rs`,
  `openhuman_backend_model_tests.rs`, `openai_codex_tests.rs` — per-transport
  model builders.
