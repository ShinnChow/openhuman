# Providers

Implementations live in `vendor/tinychannels/crates/tinychannels/src/providers/`, not here. Every file in this directory except `mod.rs` is a one-line `pub use tinychannels::providers::<name>::*;` (or a narrower named re-export) that keeps the stable `crate::channels::providers::<name>` path — and, via `channels/mod.rs`, the even shorter `crate::channels::<name>` path — resolving after the extraction. Verify with `grep -L 'tinychannels::providers' providers/*.rs`, which should print only `mod.rs`.

## Providers

| Provider | Re-exported struct | Feature gate |
| --- | --- | --- |
| `dingtalk` | `DingTalkChannel` | `channels` |
| `discord` | `DiscordChannel` | `channels` |
| `email_channel` | `EmailChannel` | `channels` |
| `imessage` | `IMessageChannel` | `channels` |
| `irc` | `IrcChannel` | `channels` |
| `lark` | `LarkChannel` | `channels` |
| `linq` | `LinqChannel` | `channels` |
| `mattermost` | `MattermostChannel` | `channels` |
| `qq` | `QQChannel` | `channels` |
| `signal` | `SignalChannel` | `channels` |
| `slack` | `SlackChannel` | `channels` |
| `telegram` | `TelegramChannel` (+ host glue, below) | `channels` |
| `whatsapp` | `WhatsAppChannel` | `channels` |
| `whatsapp_web` | `WhatsAppWebChannel` | `whatsapp-web` (forwards to `tinychannels/whatsapp-web`) |
| `yuanbao` | `YuanbaoChannel` | `channels` |

`CliChannel` is not a provider re-export: it lives in the ungated `channels/cli.rs` as a dependency-free local implementation.

## Telegram is the exception

`providers/telegram/mod.rs` re-exports the transport (`session_store`, `TelegramChannel`) from `tinychannels::providers::telegram`, but keeps three pieces here because they depend on the OpenHuman event bus and runtime rather than on transport:

- `remote_control` — `/status /sessions /new` command handling, using the agent runtime context and web-session invalidation.
- `bus::TelegramRemoteSubscriber` — busy-state event handler, registered against the process bus in `channels/runtime/startup_part_01.rs`.
- `approval_surface::TelegramApprovalSurfaceSubscriber` (+ `TELEGRAM_APPROVAL_CLIENT_ID`) — approval-reply routing for Telegram, also registered in `channels/runtime/startup_part_01.rs`.

Ported providers reach host capabilities (voice, approvals, conversation history, shutdown, event sink) through `channels::host::ProviderContext` instead of calling OpenHuman internals directly; see `channels/host/mod.rs`. Lean providers that don't need the host ignore it.

## Adding a provider

Provider transport and registration (`ChannelDefinition` metadata in `tinychannels::controllers`) belong upstream in `vendor/tinychannels`. Once a provider exists there:

1. Add the one-line `pub use tinychannels::providers::<name>::*;` re-export here.
2. Add the matching `pub use providers::<name>` / `pub use <name>::<Name>Channel` pair to `channels/mod.rs`.
3. If the provider needs host-coupled glue that must stay in OpenHuman (event bus subscribers, runtime-context command handling), follow the Telegram pattern: keep the glue in a submodule here and re-export the transport from `tinychannels`.

Do not reimplement provider transport logic in this crate — it is out of scope and will not be picked up by the shared config schema or `tinychannels::host` capability checks.
