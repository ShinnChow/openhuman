# progress_tracing

Turns the agent's real-time [`AgentProgress`](../progress.rs) event stream into
OpenTelemetry/Langfuse-style trace spans (`agent.turn` -> `agent.iteration` ->
`tool.*`/`subagent.*`), correlated by session id, for offline inspection,
regression analysis, and debugging long multi-agent runs (issue #3886). See
the module doc on `agent/progress_tracing.rs` for the full span-tree shape and
the content-capture privacy gate
(`observability.agent_tracing.capture_content`, default off).

## Key files

- `agent/progress_tracing.rs` (parent file, `include!`s
  `progress_tracing_impl_01_part_0{1,2}.rs`) — `SpanCollector` (pure state
  machine: feed it progress events + a timestamp, it accumulates finished
  `TraceSpan`s), `TraceContext`, `SpanKind`/`SpanStatus`, NDJSON serialization
  (`spans_to_ndjson`), local file/log export (`export_spans`), and the two
  run-completion entry points `export_run_trace` /
  `export_run_trace_from_journal`.
- `langfuse.rs` (+ `langfuse_part_01.rs`, `langfuse_part_02.rs`) — Langfuse
  ingestion exporter. POSTs to the backend's `/telemetry/langfuse/ingestion`
  proxy (derived from `effective_backend_api_url`), authenticated with the
  session bearer; the backend injects the real Langfuse project keys and
  forwards to `/api/public/ingestion`. Carries `x-sdk-name` via
  `crate::api::product::product_identity_header` per AGENTS.md. Best-effort:
  failures are logged and swallowed so tracing never breaks a turn.
- `journal_projection.rs` — reconstructs spans from the durable
  `AgentObservation` journal instead of the live `AgentProgress` stream, by
  folding journalled events through the same `SpanCollector`, so a
  UI/supervisor can attach after a run completes. Exhaustive over
  `tinyagents_harness::events::AgentEvent`; documents known parity gaps
  (estimated vs. charged cost, missing subagent prompt/output content).

Test files (`*_tests.rs`, split into `_part_0N_tests.rs` files) are colocated
per source file and carry no additional public surface.

## Called by

- `web_chat/progress_bridge.rs` — the live in-run side-observer; feeds
  `AgentProgress` into `SpanCollector` and calls `export_run_trace[_from_journal]`
  at run completion.
- `flows/tinyflows/langfuse_export.rs` — the flows-runner counterpart, reusing
  `langfuse::push_spans`'s endpoint/auth derivation for flow runs.

## Related docs

- [gitbooks/developing/agent-observability.md](../../../../../gitbooks/developing/agent-observability.md)
