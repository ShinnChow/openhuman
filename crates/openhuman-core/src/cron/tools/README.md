# Cron tools

Agent-facing tools over the `cron` domain. Six per-operation tools
(`add.rs`, `list.rs`, `update.rs`, `remove.rs`, `run.rs`, `runs.rs`) are the
implementation; `collapsed.rs` wraps them into the single `CronTool` the
model actually sees. Re-exported wholesale by `crate::tools::mod` (`pub use
crate::cron::tools::*`).

## Collapsed dispatch (`collapsed.rs`)

`CronTool` (`CRON_TOOL_NAME = "cron"`) advertises one schema with an `action`
field (`list` / `add` / `update` / `remove` / `run` / `runs`) instead of six
near-duplicate schemas — four of the six take only `job_id`. Each action
forwards to the matching per-operation tool via `crate::tools::implementations
::meta::collapse`, so validation and business logic live in exactly one
place. `permission_level_with_args` / `external_effect_with_args` resolve the
real per-action answer once `action` is known; the argument-free
`permission_level` reports the strictest member (`CronAddTool`'s `Execute`)
so an unparseable call over-restricts rather than under-.

The six per-operation tools stay registered as `ToolExposure::Hidden`
(`fn exposure`) — off the wire, but still dispatchable — so a replayed
transcript or saved skill that names `cron_add` etc. keeps working.

## Per-operation tools

| Tool | Name | Permission | `external_effect` | Delegates to |
| --- | --- | --- | --- | --- |
| `CronAddTool` | `cron_add` | `Execute` | `true` | `cron::add_shell_job` / `cron::add_agent_job` |
| `CronListTool` | `cron_list` | default (read-only) | default (`false`) | `cron::list_jobs` |
| `CronUpdateTool` | `cron_update` | `Execute` | `true` | `cron::update_job` |
| `CronRemoveTool` | `cron_remove` | `Write` | `true` | `cron::remove_job` |
| `CronRunTool` | `cron_run` | `Execute` | `true` | `cron::get_job` + `cron::scheduler::execute_job_now`, then `cron::record_run` / `cron::record_last_run` |
| `CronRunsTool` | `cron_runs` | default (read-only) | default (`false`) | `cron::list_runs` |

The mutating tools (`add`, `update`, `remove`, `run`) all set
`external_effect = true` and are gated by the approval flow
(GHSA-f46p-6vf9-64mm): they persist or immediately execute a stored command
or agent prompt on the host.

### `CronAddTool` details

- Accepts `schedule` (`cron` / `at` / `every`), `job_type` (`shell` |
  `agent`, inferred from the presence of `prompt` when omitted), `command` or
  `prompt`, `session_target`, `model`, `delivery`, and `delete_after_run`.
- Shell commands are checked against `SecurityPolicy::is_command_allowed`
  before being scheduled.
- `delivery` defaults to `DeliveryConfig { mode: "proactive", .. }` for agent
  jobs. `validate_delivery` enforces that `mode: "announce"` carries both
  `channel` and `to`, and that `to` is in that channel's configured
  `allowed_users` — this blocks scheduling a cron whose output is delivered
  to an arbitrary chat id (see #928).
- `JobType::Flow` is unreachable through this tool (flow-schedule rows are
  created internally by `flows::ops::flows_set_enabled` via
  `cron::add_flow_schedule_job`); the arm returns an explicit error instead of
  `unreachable!()` in case the `job_type` heuristic above ever changes.

## Related

- `cron` domain: [`../README.md`](../README.md) — job/run model, scheduler,
  delivery modes, agent-job minimum interval.
- `crates/openhuman-core/src/tools/impl/system/schedule.rs` — a separate,
  older one-shot `schedule` tool built on `cron::add_once` /
  `cron::add_once_at` rather than these six; not part of the collapse above.
- `crates/openhuman-core/src/tools/implementations/meta/collapse.rs` — the
  generic action-collapsing helper `collapsed.rs` builds on.

## Tests

`add_tests.rs`, `list_tests.rs`, `update_tests.rs`, `remove_tests.rs`,
`run_tests.rs`, `runs_tests.rs` cover each per-operation tool;
`collapsed_tests.rs` covers action dispatch, schema merging, and the
strictest/resolved permission behavior.
