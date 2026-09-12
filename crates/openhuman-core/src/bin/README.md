# bin

Auxiliary binaries built alongside the primary `openhuman-core` binary
(`crates/openhuman-core/src/main.rs`, wired via `[[bin]]` in
`crates/openhuman-core/Cargo.toml`). None of these ship in the desktop
product; each is gated so it drops out of a default build.

## Primary binary (not in this directory)

`src/main.rs` is the actual `openhuman-core` entry point (`[[bin]] name =
"openhuman-core"`). It restores default `SIGPIPE` handling, loads `.env`
before Sentry init so a dotenv-only DSN is visible at startup, initializes
Sentry under the `crash-reporting` feature, and scrubs secrets from every
outgoing error report via `openhuman_core::core::log_redaction::scrub_secrets`
before delegating to the CLI dispatcher.

## Binaries in this directory

| Binary | Source | Required features | Purpose |
| --- | --- | --- | --- |
| `test-mcp-stub` | `test_mcp_stub.rs` | none | Tiny stdio MCP server for tests |
| `openhuman-fleet` | `fleet.rs` | `http-server`, `bin-tools` | Process-per-user supervisor + reverse proxy |
| `rss-bench` | `rss_bench.rs` | `rss-bench` | Steady-state RSS benchmark for an embedded agent roster |
| `library-profile` | `library_profile/main.rs` (+ `harness.rs`, `mock.rs`, `scenarios/`) | `rss-bench` (add `rss-bench-dhat` for heap profiles) | Hermetic library-embedding profiling scenarios |

`rss-bench` and `library-profile` share the default-OFF `rss-bench` feature so
no benchmark code enters the shipped desktop/library build; build both with
`cargo build --release --no-default-features --features rss-bench --bin
rss-bench --bin library-profile`.

### `test-mcp-stub`

Speaks just enough of MCP 2024-11-05 (`initialize`, `tools/list`,
`tools/call` for one `echo` tool) over newline-delimited JSON-RPC on
stdin/stdout, exiting when stdin closes. Dependency-free beyond `serde_json`
so it builds fast and stays reliable in CI. Used exclusively by
`tests/mcp_registry_e2e.rs`, which spawns it via
`env!("CARGO_BIN_EXE_test-mcp-stub")`.

### `openhuman-fleet`

Hosts one `openhuman-core` process per user/workspace and fronts them behind
a single endpoint, so a team server can manage many members' assistants
while every existing client (`CloudHttpTransport`) keeps working unchanged.

Design is **process-per-user, not in-process multi-tenancy**:

- Each tenant runs as its own OS process (`openhuman-core run
  --headless-api`) with its own workspace volume
  (`OPENHUMAN_WORKSPACE`) and its own core bearer
  (`OPENHUMAN_CORE_TOKEN`). Tenants are not yet isolated under distinct OS
  users or containers, so this MVP is **not a production multi-tenant
  security boundary** for arbitrary agent tools.
- The supervisor mints a distinct edge token (`EdgeToken`) per tenant for
  clients and is the only holder of the tenants' core bearers
  (`CoreBearer`); the two types are kept deliberately distinct so they
  cannot be confused with each other.
- The reverse proxy forwards `POST /{user_id}/rpc` verbatim to that tenant's
  core at `http://127.0.0.1:<port>/rpc`, so the JSON-RPC wire contract is
  unchanged end to end.

MVP scope uses explicit sequential port assignment with an authenticated
JSON-RPC readiness probe before registering a tenant (a production
supervisor would read each core's bound port from a ready file /
`EmbeddedReadySignal` and reconcile membership against
`tinyhumansai/backend`). Requires `http-server` because it embeds the axum
control-plane server; under `--no-default-features` (no `http-server`) the
target is skipped rather than failing to link.

### `rss-bench`

Steady-state RSS benchmark for an embedded `openhuman_core` agent roster
(#5046). Mirrors the OpenCompany embedding contract: a bare `Agent` built
directly via `Agent::builder` (no `CoreBuilder`, no RPC, no background
services) with an injected mock model, an in-process `"none"` memory
backend, and a per-agent temp workspace. Builds a 1-agent and an 8-agent
roster, runs one deterministic warm-up turn per agent, settles, then samples
`/proc/self/{status,smaps_rollup}`.

Two modes: `--child --roster N` builds one roster in a fresh process and
prints a single `ProcSample` JSON line (the isolated measured workload); the
default (parent) mode re-execs itself `--repeat` times per roster size for
independent cold samples, aggregates them, writes the raw JSON report
(`--out`), and prints a human summary. The pure sampling/aggregation logic
lives in `openhuman_core::platform::proc_metrics`; this binary is the
fixture and process driver. Build and run:

```
cargo build --release --features rss-bench --bin rss-bench
```

### `library-profile`

Hermetic, Rust-only library profiling workloads that measure production code
paths in fresh processes with network inference replaced by a deterministic
provider (`library_profile/mock.rs`). Never enters shipped builds.

Scenarios (`library-profile <scenario>`, `library_profile/scenarios/`):

- `agent-turn` — a single cold agent turn (minimal library unit).
- `long-agent` — N warmed sequential turns with a per-turn checkpoint series.
- `workflow` — a real flows trigger -> transform -> agent graph, end to end.
- `fleet` — N live agents: marginal RSS, idle CPU, fd/thread growth, turn latency.
- `skill-run` — a skill step executing on a real `node` child: process-tree RSS.
- `subagent-storm` — K parallel researcher subagents in one instance: marginal RSS per subagent.

`memory-ingest` and `cold-phases` were removed with the in-process memory
engine (openhuman#6161) and are not coming back as-is; re-adding them means
measuring the memory module over the bus, a different scenario.

stdout is always a single pretty-printed JSON object (the pinned schema in
`harness::ProfileResult`); every diagnostic goes to stderr with the stable
`[library-profile]` prefix. With the `rss-bench-dhat` feature, dhat's global
allocator and profiler are active: RSS/time numbers are perturbed, the
result carries `"dhat": true`, and a `dhat-<scenario>.json` heap profile is
written under `target/profile/rust-library/` (override via
`OPENHUMAN_PROFILE_DHAT_OUT`).

## See also

- [`docs/library-benchmarking.md`](../../../../docs/library-benchmarking.md) —
  the benchmark environment, driver scripts under `scripts/profile/`, and
  results, covering `rss-bench` and `library-profile`.
- `tests/mcp_registry_e2e.rs` — the sole consumer of `test-mcp-stub`.
