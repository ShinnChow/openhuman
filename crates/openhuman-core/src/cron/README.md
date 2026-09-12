# Cron

Scheduled-job runtime. Owns cron-expression and human-delay parsing, the persistent job + run store, the polling scheduler that fires due jobs (`shell`, `agent`, and `flow` types), and the delivery layer that publishes events into the agent / channel pipelines. Does NOT own shell sandboxing (`security::SecurityPolicy`) or flow trigger dispatch (`flows::bus::FlowTriggerSubscriber`).

## Public surface

- `pub struct CronJob` / `pub struct CronJobPatch` / `pub struct CronRun` / `pub enum JobType` / `pub enum Schedule` / `pub enum SessionTarget` / `pub struct DeliveryConfig` — `types.rs` — durable job + run model.
- `pub fn add_once` / `pub fn add_once_at` / `pub fn parse_human_delay` / `pub fn pause_job` / `pub fn resume_job` / `pub fn update_cron_job` — `ops.rs` (also re-exported as `pub use ops as rpc`).
- `pub fn schedule_cron_expression` / `pub fn next_run_for_schedule` / `pub fn normalize_expression` / `pub fn validate_schedule` / `pub fn validate_agent_schedule` / `pub fn runs_closer_than` / `pub const MIN_AGENT_JOB_INTERVAL` — `schedule.rs`.
- `pub fn add_job` / `pub fn add_agent_job` / `pub fn add_agent_job_with_definition` / `pub fn add_shell_job` / `pub fn add_flow_schedule_job` / `pub fn find_flow_schedule_job` / `pub fn due_jobs` / `pub fn get_job` / `pub fn list_jobs` / `pub fn list_runs` / `pub fn record_last_run` / `pub fn record_run` / `pub fn remove_job` / `pub fn reschedule_after_run` / `pub fn update_job` — `store.rs` (split into `store_part_01.rs` / `store_part_02.rs` via `include!`).
- `pub mod scheduler` (`pub async fn run(config: Config)`) — `scheduler.rs`, split into `scheduler_part_01.rs` (poll loop, `run`), `scheduler_part_02.rs` (per-job-type execution, `run_agent_job`), `scheduler_part_03.rs` (`deliver_if_configured`) via `include!`.
- `pub mod scheduler_gate` — host-condition gating (battery / AC / thermal) for whether a due job may run now; see its own [README](scheduler_gate/README.md).
- `pub mod seed` — `seed.rs` — install built-in jobs on first launch.
- `pub mod bus` — `bus.rs` — `CronDeliverySubscriber` for the event bus.
- `pub mod tools` — `tools.rs` + `tools/` — agent-facing tools: `CronAddTool`, `CronListTool`, `CronUpdateTool`, `CronRemoveTool`, `CronRunTool`, `CronRunsTool`, and the collapsed `CronTool` / `CRON_TOOL_NAME` (see `tools/collapsed.rs` for why all six stay registered as hidden schemas behind the one advertised tool).
- RPC namespace `cron`: `add`, `list`, `update`, `remove`, `run`, `runs` — `schemas.rs` (re-exported via `all_cron_controller_schemas` / `all_cron_registered_controllers`).

## Job types

- **`shell`** — runs a command under `SecurityPolicy::from_config`.
- **`agent`** — the scheduler builds an `Agent` directly and runs a turn (see below); it does not go through `agent::triage`.
- **`flow`** — a `flows::Flow` schedule-trigger binding created by `flows::ops::flows_set_enabled` via `add_flow_schedule_job`. Its `command` column carries the bound flow's id; on fire the scheduler publishes `DomainEvent::FlowScheduleTick { flow_id }` instead of running anything itself. `flows::bus::FlowTriggerSubscriber` does the actual dispatch. Never created via the `cron_add` agent tool.

### Agent jobs

`scheduler_part_02.rs::run_agent_job` builds the `Agent` for a job: `Agent::from_config` by default, `Agent::from_config_for_agent` when `job.agent_id` names a registered definition, or `Agent::from_config_for_agent_with_profile` when a profile applies — the same profile-aware path `agent::profiles` and the interactive dispatcher use, so a cron run inherits the same tool/permission shape as an interactive session with that agent. A per-job `model` override is applied to a cloned `Config` before building. The built agent then runs the prefixed prompt (`[cron:<id> <name>] <prompt>`) as a normal turn; failures are classified (session-expired, insufficient-credits, security-policy) to decide whether the run is retried.

## Event bus

Cron publishes through `core/bus.rs` using variants declared in `core/events.rs`: `DomainEvent::CronJobTriggered`, `CronJobCompleted`, `CronDeliveryRequested`, `ProactiveMessageRequested` (shared with the proactive-message pipeline), and `FlowScheduleTick`.

## Calls into

- `crates/openhuman-core/src/agent/` — `Agent::from_config[_for_agent[_with_profile]]` for agent jobs; `agent::harness::definition::AgentDefinitionRegistry` to resolve `agent_id` overrides; `agent::profiles` for profile-aware construction.
- `crates/openhuman-core/src/security/` — `SecurityPolicy::from_config` sandboxes shell jobs.
- `crates/openhuman-core/src/config/` — `Config` provides poll interval, workspace dir, autonomy policy, and per-job model overrides.
- `crates/openhuman-core/src/inference/` — `provider::create_chat_model_with_model_id` resolves workload-hint model specs on agent-definition overrides.
- `crates/openhuman-core/src/platform/health/` — `health::bus::register_health_subscriber` on startup.
- `crates/openhuman-core/src/channels/` — `bus.rs` fans delivery events into channels; `channels::proactive` handles `ProactiveMessageRequested`.
- `crates/openhuman-core/src/flows/` — `flows::bus::FlowTriggerSubscriber` consumes `FlowScheduleTick`.
- `crates/openhuman-core/src/core/bus.rs` / `core/events.rs` — the process-wide event bus and `DomainEvent` variants cron publishes.

## Called by

- `crates/openhuman-core/src/tools/impl/system/schedule.rs` — `schedule` tool exposes one-shot cron scheduling to agents.
- `crates/openhuman-core/src/core/all.rs` — controller registry wires `all_cron_*`.
- `crates/openhuman-core/src/flows/ops.rs` — `flows_set_enabled` creates/removes flow schedule jobs via `add_flow_schedule_job`.
- Channel and agent runtimes consume `Cron*` and `ProactiveMessageRequested` events via the bus.

## Delivery modes

A cron job's `DeliveryConfig.mode` decides where its output ends up:

- **`proactive`** (default for agent jobs) — `deliver_if_configured` publishes
  `DomainEvent::ProactiveMessageRequested`. The proactive subscriber
  (`channels::proactive`) always pushes to the in-app web stream and additionally
  mirrors to `channels_config.active_channel` when set. Use for jobs whose
  natural surface is the desktop UI (briefings, app-pushed notifications).
- **`announce`** — explicit channel-targeted delivery. Requires `channel` and
  `to`; publishes `DomainEvent::CronDeliveryRequested` and lands only in that
  channel. The agent layer should pick this mode when a cron is created from a
  non-web channel (Telegram, Discord, Slack, …) so the reminder ends up where
  the user asked for it. The `cron_add` tool validates `to` against the
  channel's `allowed_users` to reject cross-tenant targets.
- **`none`** — silent; output is stored in `last_output` only.

The `[Channel context]` block injected by `channels::runtime::dispatch` for
non-web inbound turns instructs the model to default to `announce` with the
current channel + reply target — that is the routing path for the Telegram
"remind me to drink water" use case in #928.

## Agent-job minimum interval

An agent job is a full inference turn per run, so `schedule.rs` enforces a floor
of `MIN_AGENT_JOB_INTERVAL` (5 minutes) between consecutive runs of an agent job.
`validate_agent_schedule` is applied by `add_agent_job*` and by `update_job`
whenever an agent job's schedule is set, so every creation path (`cron_add`
tool, `cron.add` RPC, the one-shot `schedule` tool, the settings form) gets the
same rejection, and the message names the two runs that would be too close.
Shell and flow jobs are not subject to it.

The check is `runs_closer_than`: it walks consecutive occurrences (bounded) and
reports the first pair closer than the floor, so an irregular expression such as
`1,2,30 * * * *` is judged by its tightest gap and the verdict does not depend
on the instant it runs at. Wrap-around gaps count: `*/7 * * * *` fires at :56
and then at :00. Rows that predate the floor keep running; the scheduler logs
`Cron agent job '<id>' is scheduled more frequently than every 5 minutes` with
that evidence on each run instead (#6158).

## Tests

- Unit: `ops_tests.rs`, `scheduler_tests_part_01_tests.rs` .. `_04_tests.rs`, `store_tests.rs` + `store_tests_part_01_tests.rs`, `schedule_tests.rs`, `types_tests.rs`, `seed_tests.rs`, `bus_tests.rs`.
- Schema/parsing coverage lives inside `schemas_tests.rs`.
- Tool coverage: `cron::tools::{add,list,remove,run,runs,update}::tests` and `collapsed_tests.rs` (announce-mode `allowed_users` checks, action dispatch, permission resolution).
