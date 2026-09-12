# schema

Defines the `Config` struct — the single source of truth for `config.toml` —
and everything needed to load, save, and migrate it. 67 files at this level
plus the `load/`, `cli_overrides/`, and `tools/` subdirectories. AGENTS.md
points contributors here: "Rust configuration is defined under
`crates/openhuman-core/src/config/schema/` and loaded through its config
operations" (the operations live in `../ops/`, re-exported as `config::rpc`).

`Config` itself is split with `include!`: `types.rs` pulls in
`types_part_01.rs` / `types_part_02.rs` (the struct definition, model
constants, and small helper types) purely to keep any one file under the
repo's ~500-line guideline; treat all three as one unit.

## Layout — `[section]` → file → struct

| `config.toml` section | File | Struct |
| --- | --- | --- |
| `[agent]` | `agent.rs` | `AgentConfig` (+ `DelegateAgentConfig`, `OrchestratorModelConfig`, `TeamModelConfig`) |
| `[autonomy]` | `autonomy.rs` | `AutonomyConfig` — feeds `security::SecurityPolicy` |
| `[capability_providers]` | `capability_providers.rs` | `CapabilityProviderConfig` |
| `[channels*]`, security/sandbox sub-blocks | `channels.rs` | `ChannelsConfig`, per-provider configs, `SecurityConfig`, `SandboxConfig` |
| n/a (BYOK cloud providers) | `cloud_providers.rs` | `CloudProviderCreds`, `CloudProviderType` |
| `[context]` | `context.rs` | `ContextConfig` |
| `[dashboard]` | `dashboard.rs` | `DashboardConfig`, `DiagramViewerConfig` |
| `[dictation]` | `dictation.rs` | `DictationConfig` |
| ephemeral inference route | `ephemeral_route.rs` | `EphemeralRoute` |
| `[hooks]` | `hooks.rs` | `HooksConfig` |
| `[heartbeat]`, `[cron]` | `heartbeat_cron.rs` | `HeartbeatConfig`, `CronConfig` |
| `[hosting]` | `hosting.rs` | `HostingConfig` |
| identity/cost tracking | `identity_cost.rs` | `CostConfig`, `ModelPricing` |
| `[learning]` | `learning.rs` | `LearningConfig` |
| `[local_ai]` | `local_ai.rs` | `LocalAiConfig` |
| `[modules]` | `modules.rs` | `ModulesConfig` — controls only whether compiled-in modules load; the loadable *set* is fixed by `crate::modules::registry` |
| `[node]` (Claude Agent SDK companion) | `node.rs`, `claude_agent_sdk.rs` | `NodeConfig`, `ClaudeAgentSdkConfig` |
| `[observability]` | `observability.rs` | `ObservabilityConfig`, `AgentTracingConfig` |
| `[privacy]` | `privacy.rs` | `PrivacyConfig`, `PrivacyMode` |
| `[proxy]` | `proxy.rs` | `ProxyConfig`, `ProxyScope`, plus `runtime_proxy_config()` / `set_runtime_proxy_config()` process-wide accessors |
| model routing | `routes.rs` | `ModelRouteConfig`, `EmbeddingRouteConfig` |
| `[runtime]` | `runtime.rs` | `RuntimeConfig`, `DockerRuntimeConfig`, `ReliabilityConfig`, `SchedulerConfig`, `ShellConfig` |
| `[runtime_pool]` | `runtime_pool.rs` | `RuntimePoolConfig`, `RuntimePoolLangConfig` |
| Python runtime pool | `runtime_python.rs` | `RuntimePythonConfig` |
| scheduler gating | `scheduler_gate.rs` | `SchedulerGateConfig`, `SchedulerGateMode` |
| `[memory]`, `[storage]` | `storage_memory.rs` | `MemoryConfig`, `MemoryTreeConfig`, `StorageConfig`, `StorageProviderConfig` |
| `[subconscious]` | `subconscious.rs` | `SubconsciousConfig`, `MedullaLocalConfig` |
| `[subsystems]` | `subsystems.rs` | `SubsystemsConfig` |
| `[task_sources]` | `task_sources.rs` | `TaskSourcesConfig` |
| `[tokenjuice]` | `tokenjuice.rs` | `TokenjuiceConfig` |
| tool-related sections (see below) | `tools/` | — |
| `[update]` | `update.rs` | `UpdateConfig`, `UpdateRestartStrategy` |
| voice provider/server sections | `voice_providers.rs`, `voice_server.rs` | `VoiceServerConfig`, activation types |
| top-level `Config` | `types.rs` (+ `_part_01`/`_part_02`) | `Config` |
| built-in defaults | `defaults.rs` | `impl Default for Config` and per-field `default_*` fns |
| activity level | `activity_level.rs` | `AgentActivityLevel` |

`tools/mod.rs` groups the tool-facing sections: `browser.rs` (`BrowserConfig`,
`BrowserComputerUseConfig`), `http.rs` (`HttpRequestConfig`, `CurlConfig`),
`integrations.rs` (`IntegrationsConfig`, `ComposioConfig`, `SecretsConfig`),
`mcp.rs` (`McpServerConfig`, `McpClientConfig`, `GitbooksConfig`),
`multimodal.rs` (`MultimodalConfig`), `search.rs` (`SearchConfig`,
`WebSearchConfig`, `SearxngConfig`).

Most sections have a matching `*_tests.rs` (some further split into
`*_tests_part_0N_tests.rs`); this is the repo's file-size-splitting
convention, not separate modules.

## Loading

`Config::load_or_init` (in `load/impl_load.rs`) is the entry point used by
`config::ops::loader::load_config_with_timeout`:

1. Resolve the config and workspace directories (`load/dirs.rs`).
2. Read `config.toml`, recovering from non-UTF-8 corruption by falling back to
   the `.bak` copy and then to defaults (`impl_load.rs`).
3. Decrypt at-rest secrets (`load/secrets.rs`) — legacy `enc:` values are
   force-migrated to `enc2:` (ChaCha20-Poly1305) on read.
4. Apply environment-variable overrides (`load/env.rs`, `load/env_overlay.rs`).
5. Run pending schema migrations (`../migrations/`, via
   `migrations::run_pending`) and bump `schema_version`.
6. On save, `Config::save()` writes a temp file and hands off to
   `load/atomic_commit.rs::commit_replacement`, which fsyncs and renames the
   temp file into place and preserves the previous config as `.bak` —
   split out specifically so callers can tell "nothing written" (an `Err`
   before the rename) from "already committed" (after it) and roll back
   in-memory state accordingly.

`load/mod.rs` also exports `CONFIG_OWNER_MISMATCH_MARKER`: the loader appends
this marker to a config-read failure when the file's owner differs from the
reading process, and `core::observability::expected_error_kind` keys on it to
keep that failure paging instead of being demoted as ordinary user-environment
state.

## Workspace / identity helpers (`load/dirs.rs`)

Re-exported through `config::mod` and `config::schema::mod`:

- `default_root_openhuman_dir`, `user_openhuman_dir` — the per-user
  `~/.openhuman/<user-id>/` root.
- `resolve_action_dir`, `default_action_dir` — the agent's sandboxed
  read/write root.
- `active_workspace_dir` / `active_workspace_dir_cached` — resolve (and
  synchronously cache, via `load/active_workspace.rs`) the workspace the
  loader last resolved, for callers (like the developer Event Log's SSE
  stream) that cannot afford a disk read per lookup.
- `PRE_LOGIN_USER_ID`, `pre_login_user_dir` — the pre-authentication identity
  scope.

`config` only *describes* these roots. Per AGENTS.md: `action_dir` is the
agent's permitted read/write root, and `workspace_dir` stores internal state
and is never an acting-tool target — enforcement of that boundary lives in
`security::SecurityPolicy` (`security/policy/`), not here. `autonomy.rs`
(`AutonomyConfig`) is the config-side half of the same contract: it is read
into `SecurityPolicy` at startup and on every settings change
(`config::ops::agent::apply_autonomy_settings` calls
`security::live_policy::reload_from`).

## `cli_overrides/`

Process-local inference overrides supplied by the standalone CLI
(`set_cli_inference_overrides`, `apply_cli_inference_overrides`,
`restore_persisted_inference_fields`, `AppliedInferenceOverride`) — lets a CLI
invocation temporarily swap model/provider without touching the persisted
config.

## Tests

Per-section `*_tests.rs` files, plus `load_tests.rs` (split into
`load_tests_part_01..05_tests.rs`) for the loader/env-override/migration
surface.

## Related docs

- [../README.md](../README.md) — the `config` module overview.
- [../ops/README.md](../ops/README.md) — the mutation/RPC surface built on
  top of this schema.
- [../migrations/README.md](../migrations/README.md) — automatic
  schema-version upgrades run during load.
- [../../security/README.md](../../security/README.md) — enforces the
  `action_dir` / `workspace_dir` boundary this module only describes.
