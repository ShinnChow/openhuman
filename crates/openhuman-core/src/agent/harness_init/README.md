# harness_init

First-run provisioning orchestration. Several setup steps (managed Python
runtime, spaCy + model, the runtime Python server, Kompress/torch, managed
Node) used to run lazily on first use with no user-visible feedback. This
domain runs them eagerly and non-blockingly at core startup, tracks per-step
progress in an in-memory snapshot, and exposes it over
`openhuman.harness_init_status` / `openhuman.harness_init_run` for the
frontend's initialization screen.

Steps delegate to existing idempotent provisioning code — `runtime::python`,
`runtime::node`, and `runtime::python_server` (spaCy / Kompress venvs) — this
module only orchestrates and reports; it does not reimplement downloads.

## Files

- `mod.rs` — module doc and re-exports.
- `registry.rs` — the ordered `HarnessInitStep` list (`python_runtime`,
  `spacy`, `kompress`, `runtime_python_server`, and `node_runtime` when the
  `runtime-node` feature is enabled). Each step is a durable, network-free
  `is_done` probe plus a `run` closure, and a `provisioning` flag that decides
  whether the step may trigger the blocking first-run overlay (a download or
  install) versus running silently as routine startup (e.g. relaunching an
  already-installed local server).
- `ops.rs` — `run_harness_init` / `run_harness_init_with`: walks the registry,
  marks steps `Done` instantly when already satisfied, otherwise runs them.
  `provisioning_required` decides up front whether any *provisioning* step
  still needs work, so an already-provisioned host never flashes the overlay
  on a warm restart (GH-5047). Also hosts the `harness_init_status` /
  `harness_init_run` RPC handlers.
- `store.rs` — process-lifetime `HarnessInitSnapshot` behind a mutex;
  `update_step` is the single mutation point and publishes the matching
  `DomainEvent`.
- `types.rs` — `HarnessInitSnapshot`, `StepStatus`, `OverallState`,
  `StepState`.
- `bus.rs` — currently a placeholder; progress is published directly from
  `store::update_step`, kept for the canonical module shape and any future
  health re-publishing.
- `schemas.rs` — `harness_init` namespace controller schemas (`status`,
  `run`).

## Wiring

- `core::runtime::services::spawn_background_services` spawns
  `run_harness_init` (fire-and-forget) when `ServiceSet.harness_init` is true.
  `ServiceSet` (`core::runtime::builder.rs`) enables it on the desktop preset
  and disables it on the custom/embedded presets.
- Controllers are registered via
  `agent::harness_init::all_harness_init_registered_controllers` in
  `core::all.rs`.
