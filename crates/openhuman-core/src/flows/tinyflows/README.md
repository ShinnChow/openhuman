# tinyflows capability seam

Wires the vendored `tinyflows` workflow engine (validate → compile → run over
its own in-crate state-graph runtime, `vendor/tinyflows/`) to real OpenHuman
services. `tinyflows` knows nothing about OpenHuman; every effect a flow node
can have — calling an LLM, running an agent, making an HTTP request, running
code, calling a tool, reading/writing state, resolving a sub-workflow,
recalling/writing memory — is a trait the engine declares and this module
implements. See
[`gitbooks/developing/architecture/flows-on-tinyagents.md`](../../../../../gitbooks/developing/architecture/flows-on-tinyagents.md)
for the engine's own model (trigger model, run state shape, the two-gate
security model); this README covers only the host seam.

## Layout

- `mod.rs` — export-focused. Re-exports [`caps::build_capabilities`] and
  [`caps::open_flow_checkpointer`], the two entry points `flows::ops*.rs`
  calls to drive a run; re-exports `tinyflows_sqlite::checkpoint` as
  `checkpoint_sqlite` under its historical path.
- `caps/` — six of the seven capability adapters, plus construction,
  curation, preflight, and invocation logic:
  - `ops.rs` — `build_capabilities` (assembles the `Capabilities` bundle for
    one run), `open_flow_checkpointer` (opens the durable SQLite checkpointer
    at `<workspace_dir>/flows/checkpoints.db`), Composio curation/preflight
    (`is_curated_flow_tool`, `preflight_composio_args`), and the
    `OpenHumanTools` / `PreflightToolInvoker` tool-call adapters.
  - `agent.rs` — `AgentRunner`: runs an `agent` node as a nested harness
    invocation, resolving `agent_ref` and scaling the per-attempt timeout
    against the iteration cap.
  - `llm.rs` — `LlmProvider` over OpenHuman's inference stack; what an
    `agent` node falls back to without an agent runner, and what a raw
    completion node uses directly.
  - `prompt.rs` — message assembly, the `input_context` carrier and its size
    cap, and tolerant JSON extraction for structured-output nodes.
  - `http.rs` — `HttpClient`; inherits the allowlist/DNS-rebind protection of
    the underlying `HttpRequestTool` and adds `http_cred:<name>` credential
    resolution, injected server-side after the approval gate computes its
    redacted summary.
  - `code.rs` — `CodeRunner`; runs JS/Python in the sandbox under a
    wall-clock timeout.
  - `state.rs` — `StateStore` over `flows::{kv_get,kv_set}`, namespaced per
    flow so saved flows never collide on a state key.
  - `resolver.rs` — `WorkflowResolver`; resolves a `sub_workflow` node's id to
    a stored workflow (the engine only knows the id).
  - `tier.rs` — the autonomy-tier and approval gate every acting node
    (tool call, HTTP request, code run) passes through first.
  - `tools/` — `tool_call` node dispatch, split by slug namespace:
    `native.rs` for the `oh:` prefix (native OpenHuman tools, same registry
    the assistant uses), `composio.rs` for everything else (Composio
    actions; must stay the catch-all backend since Composio slugs carry no
    prefix).
- `memory_adapter.rs` — `OpenHumanMemory`, the `MemoryProvider` adapter for
  the `memory` node. Lives outside `caps/` per the repo's ~500-line
  file-size convention (`caps/ops.rs` is already large). Routes every
  operation through the same tier-gate pair as the other acting adapters
  (`Read` for `recall`/`search`/`flavour`/`people`, `Write` for
  `remember`/`forget`).
- `observability.rs` — `tinyflows::observability::RunObserver` impls:
  `TracingRunObserver` (log-only) and `FlowRunObserver`, which persists live
  steps via `flows::upsert_flow_run_step` and publishes
  `DomainEvent::FlowRunProgress` twice per non-trigger node so the frontend
  can render a run live.
- `langfuse_export.rs` — after a run settles, exports its durable
  `GraphObservation` slice as one Langfuse trace via `tinyagents`'
  `GraphLangfuseExporter`, tagged with the Langfuse Agent Graph view keys.

## Security model

Every capability adapter that reaches outside the process goes through two
independent layers before it acts: `tier.rs`'s autonomy-tier/approval gate,
and (for Composio) `caps/ops.rs`'s deny-by-default curation check, which is
intentionally stricter than the normal agent tool-call loop's curation
because a flow author's `tool_call.slug` is free-form and never round-trips
through live tool discovery first. See
[flows-on-tinyagents.md § The security model: two gates](../../../../../gitbooks/developing/architecture/flows-on-tinyagents.md#the-security-model-two-gates)
for the full contract.

## Used by

`crates/openhuman-core/src/flows/ops*.rs` calls `build_capabilities` and
`open_flow_checkpointer` to run or resume a flow, and
`langfuse_export::export_flow_run_trace` once a run settles.

## Tests

`tinyflows_tests.rs` (capability-seam smoke tests against the real engine),
`checkpoint_compat_tests.rs` (SQLite checkpoint schema/format compatibility),
`memory_node_e2e_tests.rs` (end-to-end `memory` node coverage through the
real engine, adapter, and store — kept separate from `tinyflows_tests.rs`
because it exercises a real store rather than doubles), plus a
`*_tests.rs`/`#[cfg(test)] mod tests` file alongside most modules above.
