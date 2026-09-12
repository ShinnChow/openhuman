# Controllers

Owns the `channels.*` RPC namespace: provider metadata, connect/disconnect lifecycle, status, messaging, and per-provider auth flows (Telegram login, Discord OAuth link).

## Files

| Path | Purpose |
| --- | --- |
| `backend.rs` | `OpenHumanChannelBackend` — the OpenHuman implementation of `tinychannels::ChannelBackend`; `tinychannels::ChannelManager` calls back into it, and each method forwards to the matching `ops` function. `send_outbound_intent` first tries `channels::relay_runtime::send_outbound_intent` when `relay_runtime_fronts_channel` says the relay-websocket transport fronts the channel, then falls back to `ops::channel_send_message` |
| `definitions.rs` | Re-exports provider metadata (`ChannelDefinition`, `ChannelAuthMode`, `ChannelCapability`, `AuthModeSpec`, `FieldRequirement`, `all_channel_definitions`, `find_channel_definition`) from `tinychannels::controllers` |
| `ops/` | Business logic behind each RPC handler, grouped by concern (connect, discord, messaging, telegram, yuanbao) |
| `schemas.rs` | `all_registered_controllers` / `all_controller_schemas` and the thin RPC handlers: deserialize params, build a `ChannelManager<OpenHumanChannelBackend>` over the loaded config, call the manager, and shape the `RpcOutcome`. Field schemas come from `tinychannels::controllers::channel_controller_schema`, converted by `from_channel_controller_schema` |

## RPC surface

`channels.{list, describe, connect, disconnect, status, set_default, get_default, test, telegram_login_start, telegram_login_check, discord_link_start, discord_link_check, discord_list_guilds, discord_list_channels, discord_check_permissions, send_message, send_reaction, create_thread, update_thread, list_threads}` — declared in `schemas.rs::all_registered_controllers`. `list` and `describe` use a backend-less `ChannelManager::new(ChannelsConfig::default(), ())`; every other handler goes through `OpenHumanChannelBackend`.

## `ops/`

| Path | Purpose |
| --- | --- |
| `connect.rs` (+ `connect_part_01.rs`, `connect_part_02.rs` via `include!`) | `list_channels`, `describe_channel`, `connect_channel`, `disconnect_channel`, `channel_status`, `test_channel`, `get_default_channel`/`set_default_channel`, `connected_channel_slugs`, `merge_listener_health` (`pub(crate)`, re-exported from `ops/mod.rs` under `#[cfg(test)]`) |
| `discord.rs` | Discord OAuth link flow and guild/channel/permission listing |
| `messaging.rs` | `channel_send_message`, `channel_send_reaction`, `channel_create_thread`, `channel_update_thread`, `channel_list_threads` — all call the TinyHumans backend REST API (`crate::api::rest::BackendOAuthClient`); the only path that reaches `relay_runtime` is `OpenHumanChannelBackend::send_outbound_intent` in `backend.rs` |
| `telegram.rs` | `telegram_login_start`/`telegram_login_check` |
| `yuanbao.rs` | `pub(super)` Yuanbao connect helpers: required-field checks, effective config assembly, credential verification |
| `types.rs` | Shared request/response types for the ops layer |

`connected_channel_slugs` (from `connect.rs`) is re-exported at `channels::controllers::connected_channel_slugs` for callers outside the controller registry, e.g. the welcome agent's onboarding status snapshot.

## Wiring

`crates/openhuman-core/src/core/all.rs` pushes `crate::channels::controllers::all_channels_registered_controllers()` into the global registry behind `#[cfg(feature = "channels")]`. The in-app web chat (`web_chat`, RPC namespace `channel`) is registered separately and is not gated.

## Tests

- `backend_tests.rs`.
- `ops_tests.rs` (declared from `ops/mod.rs`; pulls in `ops_tests_part_01_tests.rs` and `ops_tests_part_02_tests.rs`), `ops/connect_email_config_tests_tests.rs`.
- `schemas_tests.rs`.
