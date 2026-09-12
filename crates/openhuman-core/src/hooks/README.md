# Hooks

Configurable, file-based hooks: user-authored scripts (or model-evaluated
prompts) that observe or gate the agent at specific moments, discovered from
`hooks.json` files rather than compiled against the core. The contract is
Cursor's `hooks.json` (<https://cursor.com/docs/hooks>) — same event names,
stdin envelope, stdout decision object, and exit-code semantics — so a script
written for either host runs on the other unchanged.

This is a different "hook" from two other things in the codebase: the
in-process Rust traits an *embedding host* installs by compiling against the
core (`agent/hooks.rs`'s `PostTurnHook`, `agent/stop_hooks.rs`), and inbound
webhook ingestion (`skills/webhooks/`). This module bridges onto the first via
`bridge.rs`; it is unrelated to the second.

## Submodule map

| Module | Owns |
| --- | --- |
| `types.rs` | Wire contract: events, the stdin envelope, the decision object (`HookOutput`, `HookPermission`) |
| `config.rs` | `hooks.json` parsing and the four-layer merge |
| `matcher.rs` | Which occurrences of an event reach a given hook |
| `exec.rs` | Running one hook: stdin, timeout, exit codes, fail-open/closed |
| `engine.rs` | Selection, ordering, aggregation, session state |
| `context.rs` | Assembling the envelope from ambient host facts |
| `bridge.rs` | Mounting the engine on the harness's existing tool/turn seams |
| `ops.rs` | Lifecycle moments (init, prompt submission, subagent start) that have no existing seam |
| `prompt_eval.rs` | Model evaluation for `prompt`-kind hooks |
| `followup.rs` | Queueing what a `stop` hook asks for next |
| `schemas.rs` | RPC namespace `hooks`: `list`, `reload`, `test` |

## Two rules worth knowing before changing anything here

**The strictest verdict wins.** Layers concatenate rather than override, and
`types::HookOutput::merge` folds denial over ask over allow. Adding a hook can
therefore never loosen a policy another one set — an operator-managed
system-wide deny hook cannot be overridden by a repository shipping its own
`hooks.json`.

**Gating costs a turn's latency; observing does not.** `types::HookEvent::is_gating`
is the single place that split is encoded, and `engine.rs` reads it to decide
between running hooks sequentially in the turn's path and spawning them onto a
background task. Do not weaken this: an audit hook that hangs must not hang
the agent, and a gating hook must not be demoted to fire-and-forget.

`exec.rs` fails open on a hook error (timeout, missing interpreter,
unparseable stdout) unless the hook definition sets `fail_closed`, in which
case a failure denies. Do not change this default without reading
`AGENTS.md`'s autonomy-policy section — a hook allowing an action does not
bypass the approval gate underneath it; both still apply.

## Layering (`config.rs`)

Four `hooks.json` layers are read and concatenated, lowest trust last:
system (machine-wide, operator-managed), user (`~/.openhuman/hooks.json`),
workspace (the core's workspace directory), project (`<project>/.openhuman/hooks.json`
inside the action dir). This is the opposite of how `config.toml` merges
(override, not concatenate) — deliberately, since concatenation combined with
`HookOutput::merge`'s strictest-wins rule is the only composition that can't
be used to loosen policy.

## `prompt`-kind hooks

Most hooks are `command`: spawn a program, hand it the event JSON on stdin,
read a decision from stdout. A `prompt` hook is a policy written in English
instead — `prompt_eval.rs` asks the configured model to judge a condition,
via a one-shot `inference::ops::inference_prompt` call. A hook definition may
override the model; the override is applied by cloning the loaded `Config`
rather than mutating it, so nothing persists and a concurrent turn on the
real config is unaffected. Reserve `prompt` hooks for rare, high-stakes
moments — they cost a model call per event.

## Bridge (`bridge.rs`)

The harness already carries in-process hook seams (`ToolHook`, `PostTurnHook`
in `agent/hooks.rs`). Rather than adding a second set of call sites, the
engine registers itself through those seams once at bootstrap
(`ConfiguredHookBridge::install`/`uninstall`). Cursor's `beforeShellExecution`,
`beforeReadFile`, and `afterFileEdit` are not separate call sites here — the
bridge derives them from the ordinary tool seam by matching tool names
(`shell`/`run_command`/`bash`/... , `file_read`/`read_diff`, `file_write`/`edit`/...)
and firing both the generic `preToolUse` event and the specialised one with a
Cursor-shaped payload.

## Wiring

- `crate::hooks::init(&cfg)` is called from `core/jsonrpc.rs` during core
  boot: it sets host context, reads config, and installs or uninstalls the
  bridge. Disabled (`config::schema::hooks::HooksConfig::enabled = false`)
  or empty config uninstalls the bridge entirely so an unconfigured host pays
  no per-tool-call cost.
- `[hooks]` in `config/schema/hooks.rs` carries only host-level switches
  (`enabled`, `default_timeout_secs`) — the hooks themselves live in
  `hooks.json`, not in `config.toml`.
- RPC namespace `hooks` (`schemas.rs`) is registered via
  `all_hooks_registered_controllers` in `core/all.rs`.
- `web_chat/ops_part_02.rs` calls `hooks::ops::prompt_submitted` before a
  submitted prompt reaches the agent.
- `agent/harness/subagent_runner/ops/runner.rs` calls
  `hooks::ops::subagent_starting` before spawning a sub-agent.

## Tests

`bridge_tests.rs`, `exec_tests.rs`, `followup_tests.rs`, `matcher_tests.rs`,
and `hooks_tests.rs` (aggregated via `#[path]` in `mod.rs`).

## Related docs

- [gitbooks/developing/hooks.md](../../../../gitbooks/developing/hooks.md) — user-facing `hooks.json` guide
- [gitbooks/developing/architecture/security.md](../../../../gitbooks/developing/architecture/security.md) — approval gate and autonomy policy that still applies after a hook allows
