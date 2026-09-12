# guard

The taint/scope/budget policy gate over every memory-provider call
(`docs/specs/plan-memory.md` §3.4, `docs/specs/kernel.md` §3.4,
`docs/specs/memory-guard-allowlist.md`). [`MemoryGuard`] implements
`MemoryProvider` over the bound driver, so it is the only handle product code
should hold — a caller writes the same code against the guard as against the
raw driver, and there is no second, unguarded shape to reach for instead.

## Public surface

- `pub struct MemoryGuard` (`provider.rs`) — the decorator. Fourteen `as_*`
  overrides hand back guarded family handles instead of the inner driver's;
  see `families.rs` for why that has to be an owned field per family rather
  than a value built on demand.
- `pub struct GuardPolicy` (`policy.rs`) — the resolved policy bundle the
  guard and all family decorators share: `enforce_read` / `enforce_write`
  (the `SecurityPolicy` tier check), `ambient_scope` (source-scope query
  predicate), `stamp_taint`, `redact_outbound`, `check_egress`. Binding facts
  (driver id, `DriverClass`, hook budgets, trust state) are cached at bind
  time; the `SecurityPolicy` itself is re-read live on every call so an
  autonomy change takes effect immediately.
- Family decorators (`families.rs` + `families_part_0{1..4}.rs`) — ten
  optional-capability decorators (e.g. `GuardedTree`, `GuardedProfile`,
  `GuardedGraph`), present exactly when the bound driver advertises that
  family.
- `mandatory.rs` — the three families every driver has (`MemoryCore`,
  `MemoryRecall`, `MemoryPortability`), where steps 3, 4 and 6 land for the
  always-present surface.
- `budget.rs` — pure char-budget truncation, step 6.
- `audit.rs` — the tracing span and audit event, step 7.
- `in_memory.rs` — `#[doc(hidden)]` in-memory `MemoryProvider` fake for tests
  that need a genuine round trip. Not `#[cfg(test)]`: integration tests link
  it too.
- `test_support*.rs` — a recording fake that proves a call was made without
  storing data.

## The seven enforcement steps

| # | Step | Where |
| - | ---- | ----- |
| 1 | `SecurityPolicy` tier | `GuardPolicy::enforce_read` / `enforce_write` |
| 1b | path rules | no-op — no contract method carries a path |
| 2 | source scope as a query predicate | `GuardPolicy::ambient_scope`, applied in `GuardedTree::query_source` (not recall — the bound driver refuses a scoped recall) |
| 3 | taint stamping | `GuardPolicy::stamp_taint` (raises, never overrides) |
| 4 | redaction | `GuardPolicy::redact_outbound` — a no-op for embedded drivers |
| 5 | egress + trust | `GuardPolicy::check_egress` |
| 6 | char budgets | `budget.rs`, driven by `MemoryHooksConfig` |
| 7 | audit + tracing | `audit.rs` |

See `mod.rs` for the full argument behind each departure from a naive reading
of the milestone brief, and its "Honesty clause" section for what is
deliberately *not* covered yet (`ProfileStore`'s direct SQL access to
`user_profile` has no guarded family).

## Calls into

- `crate::memory::api::provider::MemoryProvider` (the `tinymemory-api`
  contract) — what the guard decorates.
- `crate::security::SecurityPolicy` — the tier check in step 1.
- `crate::memory::source_scope` — the ambient per-turn source allowlist read
  in step 2.
- `crate::config`'s `MemoryHooksConfig` — the budgets read in step 6.

## Called by

- `crate::memory::ops::guard::active_memory_guard` — the guarded-driver
  lookup RPC handlers use instead of `ops::helpers::active_memory_client`
  when the operation has a typed contract twin.
- `CoreContext::memory` (`crate::core::runtime::context`) — resolves
  `memory_binding()` and returns its `.guard()`; this is additive next to
  `CoreContext::memory_binding()`, which still hands out the bare driver for
  callers (like the health probe) that must not run through the tier check.
- `crate::memory::binding` — constructs the `MemoryGuard` at bind time
  (`MemoryGuard::new`).

## Tests

- `budget_tests.rs`, `families_tests.rs`, `policy_tests.rs`,
  `provider_tests.rs` — unit coverage per file above.
- `../bypass_allowlist_tests.rs` — the ratchet lint: enumerates the production
  files that still reach the driver around the guard (all of them into
  `ProfileStore`/`MemoryClient`, none of them into a `MemoryProvider` family)
  and fails if that set grows.
