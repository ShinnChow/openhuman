# Triage

Classification pipeline for external triggers. Turns a source-specific event
(Composio webhook, incoming HTTP webhook, task-source card, desktop
notification, ad hoc RPC) into a `drop` / `acknowledge` / `react` / `escalate`
decision, then dispatches the sub-agent that decision implies. Does NOT own
the trigger sources themselves (Composio sync, webhook tunnels, notification
ingest) — those build a [`TriggerEnvelope`] and call in.

## Pipeline

1. **Envelope** (`envelope.rs`) — caller builds a [`TriggerEnvelope`] via one
   of the `from_*` constructors (`from_composio`, `from_webhook`, `from_cron`,
   `from_external`, …). Carries the [`TriggerSource`], a truncated payload,
   and an optional [`TaskCardLink`] when the trigger concerns a task-board
   card.
2. **Evaluator** (`evaluator*.rs`) — [`run_triage`] runs the `trigger_triage`
   agent turn through a tiered fallback chain: cloud, one retry on
   transient/429 failure, then a local-model fallback arm; if both fail it
   returns `TriageOutcome::Deferred` instead of an error so the caller can
   retry the whole chain later. `decision.rs::parse_triage_decision` parses
   the reply tolerantly (small local models routinely emit malformed JSON).
3. **Escalation** (`escalation.rs`) — [`apply_decision`] publishes
   `TriggerEvaluated`, and for `react`/`escalate` builds a real `Agent` and
   dispatches `trigger_reactor` or `orchestrator` via
   `agent::harness::subagent_runner::run_subagent`. A card-linked trigger
   routes to `agent::task_dispatcher` (claim + autonomous run + write-back)
   instead of the one-shot sub-agent.
4. **Origin** (`origin.rs`) — every caller scopes an `AgentTurnOrigin` around
   `apply_decision` so the approval gate has something other than `Unknown`
   to read: [`local_trigger_origin`] (`Cli`, trust root, no audit row) for
   triggers the machine generated itself, [`remote_trigger_origin`]
   (`TrustedAutomation::Workflow { require_approval: true }`) for anything
   whose payload came from outside.
5. **Events** (`events.rs`) — thin wrappers around `DomainEvent::Trigger*`
   (`TriggerEvaluated`, `TriggerEscalated`, `TriggerEscalationFailed`) so the
   field list lives in one place.

`routing.rs` resolves the provider for a turn. The remote (cloud) arm is
always used for the *initial* attempt — `resolve_provider` never returns a
local provider — but `build_local_provider_with_config` still builds the
local-model fallback the evaluator's tiered chain falls back to when cloud
fails twice.

Both the cloud and local arms call `cron::scheduler_gate::wait_for_capacity`
before spending an LLM turn, so triage cooperates with the same
host-capacity throttle background AI work respects; see
[`../../cron/scheduler_gate/README.md`](../../cron/scheduler_gate/README.md).

## Not routed through triage

`TriggerEnvelope::from_cron` still exists and is exercised by
`agent.triage_evaluate` for manual testing, but the cron scheduler itself
does not call `run_triage` — scheduled jobs run their agent directly and only
reuse the `triage_action`/`triage_reason` *fields* on the resulting
notification for display, not the classifier.

## Public surface

- `pub struct TriggerEnvelope` / `pub enum TriggerSource` / `pub struct
  TaskCardLink` — `envelope.rs`.
- `pub enum TriageAction` / `pub struct TriageDecision` / `pub fn
  parse_triage_decision` / `pub enum ParseError` — `decision.rs`.
- `pub async fn run_triage(envelope) -> anyhow::Result<TriageOutcome>` /
  `pub enum TriageOutcome` / `pub struct TriageRun` / `pub enum
  TriageResolutionPath` — `evaluator*.rs`.
- `pub async fn apply_decision(run, envelope) -> anyhow::Result<()>` —
  `escalation.rs`.
- `pub fn local_trigger_origin()` / `pub fn
  remote_trigger_origin(envelope)` — `origin.rs`.
- `pub async fn resolve_provider()` / `pub fn
  build_local_provider_with_config(config)` / `pub struct ResolvedProvider`
  — `routing.rs`.

## Called by

- `crates/openhuman-core/src/skills/webhooks/ops.rs`,
  `skills/webhooks/bus.rs` — webhook-tunnel requests routed to an agent
  tunnel.
- `crates/openhuman-core/src/memory/sync/composio/bus*.rs` — Composio
  trigger events (env/config flags can disable the pipeline per-toolkit).
- `crates/openhuman-core/src/integrations/task_sources/route.rs` — proactive
  task-source cards targeting `SourceTarget::AgentTodoProactive`.
- `crates/openhuman-core/src/desktop/notifications/rpc.rs` — background
  triage spawned after `notification_ingest`, score persisted via
  `store::update_triage`.
- `crates/openhuman-core/src/agent/schemas.rs` — `agent.triage_evaluate` RPC,
  used to exercise the pipeline against a synthetic trigger from any source
  including cron.

## Related

- `crates/openhuman-core/src/agent/registry/agents/trigger_triage/` — the
  classifier agent definition and prompt this pipeline's parser contract is
  built against.
- `crates/openhuman-core/src/agent/registry/agents/trigger_reactor/` — the
  narrow single-step sub-agent `react` decisions dispatch to.
- `crates/openhuman-core/src/cron/scheduler_gate/README.md` — the LLM-permit
  gate both triage arms wait on.

## Tests

- `envelope_tests.rs`, `decision_tests.rs`, `escalation_tests.rs`,
  `evaluator_tests*.rs`, `events_tests.rs`, `origin_tests.rs`,
  `routing_tests.rs`.
