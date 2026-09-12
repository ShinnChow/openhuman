# Controllers

Owns the `channels.*` RPC namespace: provider metadata, connect/disconnect lifecycle, status, messaging, and per-provider auth flows (Telegram login, Discord OAuth link).

## Files

| Path | Purpose |
| --- | --- |
| `backend.rs` | `OpenHumanChannelBackend` — the OpenHuman implementation of `tinychannels::ChannelBackend`, the seam `tinychannels::controllers` operations call back into for config, outbound sends (routing through `relay_runtime` when the relay-websocket transport fronts a channel), and RPC-shaped results |
| `definitions.rs` | Re-exports provider metadata (`ChannelDefinition`, `ChannelAuthMode`, `ChannelCapability`, `AuthModeSpec`, `FieldRequirement`, `all_channel_definitions`, `find_channel_definition`) from `tinychannels::controllers` |
| `ops/` | Business logic behind each RPC handler, grouped by concern (connect, discord, messaging, telegram, yuanbao) |
| `schemas.rs` | `ControllerSchema`/`RegisteredController` declarations and thin RPC handlers that deserialize params, call into `ops`, and shape the `RpcOutcome` |

## RPC surface

`channels.{list, describe, connect, disconnect, status, set_default, get_default, test, telegram_login_start, telegram_login_check, discord_link_start, discord_link_check, discord_list_guilds, discord_list_channels, discord_check_permissions, send_message, send_reaction, create_thread, update_thread, list_threads}` — declared in `schemas.rs::all_registered_controllers`, wired into the global registry from `crates/openhuman-core/src/core/all.rs` (`controllers::all_channels_registered_controllers()`, gated on the `channels` feature).

## `ops/`

| Path | Purpose |
| --- | --- |
| `connect.rs` (+ `connect_part_01.rs`, `connect_part_02.rs`) | `connect_channel`, `disconnect_channel`, `channel_status`, `test_channel`, `get_default_channel`/`set_default_channel`, `connected_channel_slugs`, `merge_listener_health` (test-only re-export) |
| `discord.rs` | Discord OAuth link flow and guild/channel/permission listing |
| `messaging.rs` | `channel_send_message`, `channel_send_reaction`, `channel_create_thread`, `channel_update_thread`, `channel_list_threads` — all call the backend REST API directly (`crate::api::rest::BackendOAuthClient`), not `relay_runtime` |
| `telegram.rs` | `telegram_login_start`/`telegram_login_check` |
| `yuanbao.rs` | Yuanbao-specific connect helpers |
| `types.rs` | Shared request/response types for the ops layer |

`connected_channel_slugs` (from `connect.rs`) is re-exported at `channels::controllers::connected_channel_slugs` for callers outside the controller registry, e.g. the welcome agent's onboarding status snapshot.

## Wiring

Registered under `DomainGroup` in `crates/openhuman-core/src/core/all.rs` (~line 585) via `controllers::all_channels_registered_controllers()`, behind `#[cfg(feature = "channels")]`.

## Tests

- `backend_tests.rs`.
- `ops_tests.rs` (+ `ops_tests_part_01_tests.rs`, `ops_tests_part_02_tests.rs`), `ops/connect_email_config_tests_tests.rs`.
- `schemas_tests.rs`.
