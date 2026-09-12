# announcements

Thin RPC adapter for the product-announcements feed. Like its `hosted`
siblings (`billing`, `referral`, `team`) it owns no business logic, state, or
authorization — it forwards an authenticated request to the TinyHumans
backend and passes the response through verbatim.

## Responsibilities

- Fetch the latest active announcement for the signed-in user via
  `GET /announcements/latest`.
- Resolve and require a live backend session token before calling out; fail
  closed with a clear error when none is stored.
- Fold the backend's 404 (`BackendApiError::AnnouncementNotFound`, no
  qualifying announcement) into the same "no announcement" success outcome
  instead of surfacing it as an error — this is a best-effort/cosmetic
  feature and treating the 404 as a failure only flooded Sentry with no
  actionable signal.

## Key files

| File | Role |
| --- | --- |
| `mod.rs` | Re-exports `ops::*` and the schema/controller pair. |
| `ops.rs` | `require_token`, `get_latest_announcement`. Builds a `BackendOAuthClient` against the effective backend URL and issues the authed GET. |
| `schemas.rs` | Controller schema + handler that loads `Config` and delegates to `ops`. |

## RPC / controllers

One controller in the `announcements` namespace, registered into the global
registry via `crates/openhuman-core/src/core/all.rs`:

| Method | Inputs | Output | Backend call |
| --- | --- | --- | --- |
| `announcements_get_latest` (`announcements.get_latest`) | none | `announcement` (JSON, may be `null`) | `GET /announcements/latest` |

## Persistence

None. The domain reads the stored session token but does not persist
anything; the UI is responsible for tracking dismissal locally by
announcement id — this module has no notion of "dismissed".

## Dependencies

- `crate::security::credentials::session_support::require_live_session_token`
  — rejects an expired token locally instead of firing a doomed backend 401
  (same guard as `billing/ops.rs`).
- `crate::api::config::effective_backend_api_url`, `crate::api::BackendOAuthClient`
  — resolve the backend base URL and issue the authed JSON request, carrying
  the sanitized `x-sdk-name` product identity (`crate::api::product`) on every
  call.
- `crate::api::flatten_authed_error` — flattens any non-404 backend/session
  error for the RPC caller.
- `crate::rpc::RpcOutcome` — return wrapper carrying value + log line.

## Gating

`announcements` is part of `DomainGroup::Hosted` (`crates/openhuman-core/src/core/all.rs`).
A build that never dials the managed backend — including `DomainSet::embedded` —
drops the whole `Hosted` group together, so this controller and its
`hosted` siblings simply don't register.
