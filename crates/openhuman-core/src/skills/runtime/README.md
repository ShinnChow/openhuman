# skills/runtime

`crate::skills::runtime` owns execution of installed `SKILL.md` skills.
`skill_runtime` is the stable RPC/CLI namespace for this module (registered
in `core/all.rs`).

Responsibilities:

- Start and cancel skill runs.
- Read recent run metadata and run logs.
- Resolve reusable language runtimes before script-backed skills run.
- Host the built-in `skill_executor` sub-agent that carries out a run.

It deliberately reuses:

- `crate::runtime::node` for Node.js, npm, npx, and PATH injection.
- `crate::runtime::python` for Python interpreter resolution and process
  launching.
- `crate::skills` (`ops_discover`, `run_log`) for installed skill discovery,
  metadata, resources, and run-log storage.

## Key files

| File | Purpose |
| --- | --- |
| `mod.rs` | Feature gate for the `skills` Cargo feature; re-exports the real module or [`stub`] |
| `ops.rs` | `RuntimeRequirement` and `resolve_runtimes` — checks Node/Python availability |
| `run_machinery.rs` | Spawns and awaits background skill runs (`spawn_workflow_run_background[_with_profile]`, `await_run_outcome`) |
| `schemas.rs` | Controller schemas and handlers for `skill_runtime_*` (`run`, `cancel`, `recent_runs`, `read_run_log`, `resolve_runtimes`, `schemas`) |
| `tools.rs` | LLM-callable tool wrapping `resolve_runtimes` |
| `stub.rs` | Disabled-feature facade matching the public surface with empty bodies |
| `agent/skill_executor/` | Built-in `skill_executor` agent archetype that carries out a run |

## Compile-time gate (`skills` feature)

`pub mod runtime;` in `skills/mod.rs` is always compiled — it is a facade.
The real implementation is gated behind the default-ON `skills` Cargo
feature (the same gate as `skills` and `skills::catalog`). When the feature
is off, [`stub`](stub.rs) takes its place with disabled-error / empty
bodies. See `crates/openhuman-core/src/skills/mod.rs` for the pattern.

Production smoke examples:

```bash
openhuman skill_runtime schemas
openhuman skill_runtime resolve_runtimes --runtime all
openhuman skill_runtime run --skill_id git-helper --inputs '{}'
openhuman skill_runtime recent_runs --limit 10
```
