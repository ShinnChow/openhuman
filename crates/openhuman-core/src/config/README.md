# Config

Authoritative TOML-backed configuration layer. Owns the `Config` schema (every domain section: agent, channels, memory, autonomy, voice, scheduler, observability, etc.), env-variable overrides, the per-user openhuman directory layout, runtime proxy settings, the daemon descriptor, and the settings CLI. ~870 files across the workspace reference `crate::config` — almost every other domain reads `Config` here.

## Layout

| Path | Purpose |
| --- | --- |
| `schema/` | The `Config` struct, every section type, loading/saving, env overrides, migrations trigger point — see [schema/README.md](schema/README.md) |
| `ops/` | RPC/CLI mutation surface (`config::rpc`) built on the schema — see [ops/README.md](ops/README.md) |
| `schemas/` | Controller schemas + thin RPC handlers for the `config` namespace |
| `migrations/` | Automatic, schema-version-gated startup data migrations — see [migrations/README.md](migrations/README.md) |
| `migration_helpers/` | User-triggered `migrate.{openclaw,hermes}` RPCs importing memory from other assistants — see [migration_helpers/README.md](migration_helpers/README.md) |
| `workspace/` | Workspace bootstrap + editable Persona Pack (`SOUL.md`/`IDENTITY.md`) file RPCs — see [workspace/README.md](workspace/README.md) |
| `daemon.rs` | `DaemonConfig` — sidecar lifecycle / port descriptor |
| `settings_cli.rs` | `openhuman settings ...` CLI section-slicing helper |
| `tools.rs` | Read-only LLM-callable wrappers over config (snapshot, autonomy, search, runtime flags, data paths) |
| `workspace_handle.rs` | Opaque, stable digest identity for a workspace directory, used on the wire (Event Log, notification broadcast) so paths never leak |

## Public surface

- `pub struct Config` — `schema/types.rs` (re-exported from `mod.rs`) — top-level user settings.
- Per-domain config structs and enums — re-exported from `mod.rs`; see [schema/README.md](schema/README.md) for the full section → struct table.
- Model constants: `DEFAULT_MODEL`, `MODEL_AGENTIC_V1`, `MODEL_CODING_V1`, `MODEL_REASONING_V1`, and others in `schema/types_part_01.rs`.
- `pub struct DaemonConfig` — `daemon.rs` — sidecar lifecycle / port descriptor.
- `pub fn apply_runtime_proxy_to_builder` / `pub fn build_runtime_proxy_client` / `pub fn build_runtime_proxy_client_with_timeouts` / `pub fn runtime_proxy_config` / `pub fn set_runtime_proxy_config` — `schema/proxy.rs`.
- Workspace identity helpers: `pub fn clear_active_user`, `default_root_openhuman_dir`, `pre_login_user_dir`, `read_active_user_id`, `user_openhuman_dir`, `write_active_user_id`, `PRE_LOGIN_USER_ID` — `schema/load/dirs.rs`.
- `pub mod ops` (re-exported as `rpc`) — `ops/` — RPC handlers and settings mutation; see [ops/README.md](ops/README.md).
- `pub mod settings_cli` — `settings_cli.rs` — `openhuman settings ...` CLI surface.
- RPC `config.{get_config, update_model_settings, update_memory_settings, update_runtime_settings, update_browser_settings, resolve_api_url, get_runtime_flags, set_browser_allow_all, workspace_onboarding_flag_exists, workspace_onboarding_flag_set, update_analytics_settings, get_analytics_settings, update_meet_settings, get_meet_settings, agent_server_status, reset_local_data, get_onboarding_completed, get_dictation_settings, update_dictation_settings, get_voice_server_settings, update_voice_server_settings, set_onboarding_completed}` — `schemas/`.

## Calls into

- Std + serde TOML for serialization.
- `crates/openhuman-core/src/security/keyring/` indirectly when secrets sections need at-rest crypto (`schema/load/secrets.rs`).
- Filesystem under `~/.openhuman/<user-id>/` via `schema/load/dirs.rs`.

## Called by

- ~870 files across the workspace pull `Config` for their slice.
- Hot consumers: `crates/openhuman-core/src/agent/` (model + autonomy), `crates/openhuman-core/src/channels/` (provider tokens), `crates/openhuman-core/src/memory/` (storage paths), `crates/openhuman-core/src/cron/` (scheduler poll), `crates/openhuman-core/src/inference/local/` (Ollama / device routing), `crates/openhuman-core/src/security/` (sandbox backend, autonomy policy), `crates/openhuman-core/src/voice/`, `crates/openhuman-core/src/desktop/notifications/`, `crates/openhuman-core/src/tools/`.
- `crates/openhuman-core/src/core/all.rs` — registers `all_config_*`.

## Tests

- Unit: `ops_tests.rs` (+ `ops_tests_part_0N_tests.rs`), `schemas_tests.rs`, plus per-section `*_tests.rs` under `schema/` (`channels_tests.rs`, `proxy_tests.rs`, etc.) and `load_tests.rs` (+ `load_tests_part_0N_tests.rs`).
- Cross-test serialization: `schema/load/impl_load.rs` round-trips against `schema/defaults.rs`.
- `TEST_ENV_LOCK` (`mod.rs`) is shared with sibling test modules that mutate `OPENHUMAN_WORKSPACE`.
