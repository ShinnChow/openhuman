# session_import

One-time migration of legacy OpenHuman session transcripts into TinyAgents
`Store`/`AppendStore` records, exposed as the explicit `session_import.run`
controller (`openhuman-core session-import run`). It is never run from a boot
hook.

> The `//!` in `mod.rs` cites `docs/tinyagents-session-migration-design.md`;
> that design doc is not checked into this repository.

## Sources → destination

- Legacy transcript JSONL under `session_raw/`, both the current flat layout
  and legacy `DDMMYYYY` date-folder layout.
- Legacy Markdown-only sessions (no JSONL twin).
- Precedence per session stem: flat JSONL > legacy-dir JSONL > Markdown-only
  (`scan.rs::discover_sources`).

All writes land under `{workspace}/tinyagents_store/`:

- `kv/sessions/{session_key}.json` — `SessionDescriptor` compatibility record
  mapping the OpenHuman session key to TinyAgents identifiers.
- `kv/migration_items/{sha256(source)}.json` — per-item idempotency ledger
  (`ItemLedgerRecord`).
- `kv/migrations/session_import_v1.json` — global run marker.
- `journal/session.{stem}.messages.jsonl` — message journal, one `StoreRecord`
  per line via TinyAgents `JsonlAppendStore`.

Source files are never mutated or deleted.

## Idempotency

A completed run writes the global marker (`MARKER_KEY`), which short-circuits
future runs unless `ImportOptions::only` or `ImportOptions::force` is set.
Independently, each source gets a per-item fingerprint (size + mtime) in the
migration-items ledger, so an unchanged source is skipped even under `--only`
or after the global marker is gone. `dry_run` plans and reports without
writing either the ledger or the store.

## Key files

| File | Role |
| --- | --- |
| `mod.rs` | Module docs and re-exports (`ImportOptions`, `ImportSummary`, controller registration). |
| `types.rs` | Serde types: `ImportOptions`, `ImportSummary`, `ItemReport`, `SessionDescriptor`, `JournalMessage`, `ItemLedgerRecord`, store-layout constants. |
| `scan.rs` | `discover_sources` — walks `session_raw/` and Markdown session dirs, dedupes by stem per the precedence order above. |
| `convert.rs` | Pure helpers: stem→parent-key lineage, store-name sanitization, journal stream naming, thread-id derivation, descriptor/message-record assembly. |
| `ops.rs` | `run_import` — scan → plan → write; opens the shared KV/journal store handles (`open_session_stores`) also used by `live.rs`. |
| `live.rs` | Ongoing dual-write of new turns into the same store layout, gated by `AgentConfig::session_dual_write` (config, default on) with an `OPENHUMAN_SESSION_DUAL_WRITE` env kill switch. Errors here are logged and swallowed — they must never affect the legacy transcript write, which stays authoritative. |
| `schemas.rs` | `session_import.run` `ControllerSchema` and handler. |
| `*_tests.rs` | Sibling test suites for `convert`, `live`, `ops`, `schemas`. |

## RPC

`session_import.run` (`openhuman.session_import_run`) accepts `dry_run`,
`only` (stem glob), `force`, `verbose`, and an optional `workspace` override,
and returns an `ImportSummary`. Registered via
`all_session_import_registered_controllers` in
`crates/openhuman-core/src/core/all.rs`.

## Used by

- `crates/openhuman-core/src/core/all.rs` — registers the controller.
- `crates/openhuman-core/src/agent/harness/session/transcript.rs` — the
  legacy transcript reader/writer this module converts from and that
  `live.rs` mirrors alongside.
