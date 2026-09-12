# ops

JSON-RPC / CLI controller surface for persisted config and runtime flags — the
mutation half of `config`. `crate::config` re-exports this module both under
its own name and as `rpc` (`pub use ops as rpc`), so most callers write
`config::rpc::*`. Controllers in `../schemas/` are thin wrappers around the
functions here: they deserialize RPC params into the `*SettingsUpdate` structs
and call the corresponding `apply_*` / `get_*` fn, which returns
`RpcOutcome<T>`.

## Layout

| File | Responsibility |
| --- | --- |
| `agent.rs` | Autonomy, agent, agent-paths, activity-level, and memory-sync settings. |
| `loader.rs` | Config loading/snapshotting and runtime flags; split into `loader_part_01.rs` / `loader_part_02.rs` via `include!`. |
| `model.rs` | AI-provider, memory, runtime, local-AI, and Composio-trigger settings. |
| `privacy.rs` | Privacy Mode (`[privacy]`) get/set. |
| `sandbox.rs` | Sandbox / Docker runtime (`[security.sandbox]`, `[runtime.docker]`) settings. |
| `ui.rs` | Browser, analytics, search, dictation, voice-server, and onboarding-flag settings. |

Each submodule follows the same shape: a `*SettingsPatch` struct with
`Option<T>` fields (`None` = unchanged), an `apply_*` fn that loads the config,
mutates it from the patch, and saves it, a `load_and_apply_*` convenience
wrapper that loads the config first, and a `get_*` fn that reads the relevant
section back out as `RpcOutcome<serde_json::Value>`.

## Key entry points

- `agent.rs`: `apply_autonomy_settings` / `get_autonomy_settings`,
  `add_auto_approve_tool`, `apply_agent_settings` / `get_agent_settings`,
  `apply_agent_paths_settings` / `get_agent_paths`, `ensure_usable_cwd`,
  `expand_tilde`, `redact_home`, `apply_activity_level_settings`,
  `apply_memory_sync_settings`.
- `loader.rs`: `load_config_with_timeout`, `get_config_snapshot`,
  `client_config_json`, `reload_config_from_paths`, `reset_local_data`,
  `set_browser_allow_all`, `get_runtime_flags`, `core_rpc_url_from_env`,
  `BROWSER_ALLOW_ALL_ENV`.
- `model.rs`: `apply_model_settings`, `apply_memory_settings`,
  `apply_runtime_settings`, `apply_local_ai_settings`,
  `apply_composio_trigger_settings`, `load_and_resolve_api_url`.
- `privacy.rs`: `apply_privacy_settings`, `get_privacy_mode`.
- `sandbox.rs`: `apply_sandbox_settings`, `get_sandbox_settings`.
- `ui.rs`: `apply_browser_settings`, `apply_analytics_settings`,
  `apply_search_settings`, `apply_voice_server_settings` /
  `get_voice_server_settings`, `apply_dictation_settings` /
  `get_dictation_settings`, `set_onboarding_completed` /
  `get_onboarding_completed`, `workspace_onboarding_flag_exists` /
  `workspace_onboarding_flag_set`.

## Security-relevant behavior

- `add_auto_approve_tool` persists the workspace's "always allow" tool list
  (`config.autonomy.auto_approve`), and `apply_autonomy_settings` and
  `add_auto_approve_tool` both call
  `crate::security::live_policy::reload_from(&config.autonomy)` so the live
  `SecurityPolicy` picks up the change without a core restart. `apply_agent_paths_settings`
  likewise calls `crate::security::live_policy::set_action_dir` when
  `action_dir` changes. Do not weaken these settings mutators — they gate the
  same autonomy invariants AGENTS.md requires of `security/`.
- `apply_privacy_settings` hot-swaps the live `SecurityPolicy`'s privacy mode
  the same way, so an inference chokepoint enforces a new mode immediately.
- `reset_local_data` deletes the workspace's persisted config, memory, and
  session state; it is invoked only via the explicit `config.reset_local_data`
  RPC, never automatically.

## `#[cfg(test)]` re-exports

`mod.rs` re-exports several otherwise-private items (`Config`,
`active_workspace_marker_path`, `resolve_backend_api_url`, etc.) behind
`#[cfg(test)]` purely so `ops_tests.rs` and its `ops_tests_part_0N_tests.rs`
siblings can reach them through `use super::*`; they carry no runtime meaning
outside test builds.
