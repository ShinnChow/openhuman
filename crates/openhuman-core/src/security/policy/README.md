# Security / Policy

`SecurityPolicy` and the autonomy/risk machinery it implements. This is where
every invariant AGENTS.md lists for the autonomy policy is actually enforced:
`action_dir` vs `workspace_dir`, workspace-internal path protection,
fail-closed command classification, always-forbidden system/credential paths,
trusted roots, and the per-hour action budget.

## Responsibilities

- Build a `SecurityPolicy` from `AutonomyConfig` (`SecurityPolicy::from_config`).
- Classify shell commands into a risk level (`CommandRiskLevel`) and a
  fail-closed gate class (`CommandClass`), and turn that into a `GateDecision`
  for the current `AutonomyLevel`.
- Decide whether a filesystem path is reachable at all
  (`is_always_forbidden`), workspace-internal state
  (`is_workspace_internal_path`), or covered by a `TrustedRoot`.
- Track and enforce the hourly action budget (`ActionTracker`).
- Own the two stable string markers (`POLICY_BLOCKED_MARKER`,
  `POLICY_DENIED_MARKER`) the agent harness's repeated-failure middleware
  keys on to stop retrying a provably-futile call.

## Key files

| File | Purpose |
| --- | --- |
| `types.rs` | `SecurityPolicy` fields, `AutonomyLevel`, `CommandRiskLevel`, `CommandClass`, `GateDecision`, `ToolOperation`, `ActionTracker`, `TrustedRoot`/`TrustedAccess`, `ActiveProfileGuard`, `WORKSPACE_INTERNAL_DIRS`/`WORKSPACE_INTERNAL_FILES`, the two policy markers |
| `path_checks.rs` | `is_workspace_internal_path`, `is_always_forbidden`, `check_cross_profile`, `is_within_trusted_root`, `is_resolved_path_allowed[_for]`, `check_resolved_against_forbidden` |
| `command_checks.rs` | `classify_command`, `gate_decision`, `check_gated_command`, `is_command_allowed`, `command_risk_level`, `parse_declared_class`, `is_command_executor`, `split_unquoted_segments` |
| `enforcement.rs` | `can_act`, `enforce_write_tier`, `enforce_tool_operation`, `record_action`/`is_rate_limited`, `from_config`, `with_active_profile`/`with_privacy_mode`, `openhuman_scratch_dir`/`ensure_openhuman_scratch_dir`, `validate_path_within_root` |
| `policy_command.rs` (`policy_command_part_01.rs`, `policy_command_part_02.rs` via `#[path]`) | Command-allowlist matching split across parts for file-size hygiene |

## Public surface

Re-exported through `policy/mod.rs` and then through `security/mod.rs`:
`SecurityPolicy`, `AutonomyLevel`, `CommandClass`, `CommandRiskLevel`,
`GateDecision`, `ToolOperation`, `ActionTracker`, `ActiveProfileGuard`,
`TrustedRoot`, `TrustedAccess`, `POLICY_BLOCKED_MARKER`,
`POLICY_DENIED_MARKER`, `validate_path_within_root`,
`ensure_openhuman_scratch_dir`, `openhuman_scratch_dir`.

`live_policy.rs` (`security/live_policy.rs`, a sibling of this directory)
holds the hot-swappable *current* `SecurityPolicy` in a process-global cell so
a runtime `[autonomy]` config change is observed by new sessions immediately
instead of only at the next full policy rebuild. `security_for_tool_context`
clones the live policy with its `action_dir` overridden to a profile's own
directory when a turn runs under a dedicated-workspace profile.

## Invariants

Each of these must not be weakened to make a feature work:

- **`action_dir` vs `workspace_dir`.** `action_dir` is the agent's action
  sandbox root — tools resolve relative paths and default their cwd there.
  `workspace_dir` holds core-internal state (memory DBs, sessions, tokens) and
  is never reachable from an agent tool call. Kept as two separate fields on
  `SecurityPolicy` (`types.rs`) specifically so the two roots can diverge.
- **`is_workspace_internal_path`** (`path_checks.rs:420`) — true for any path
  whose first component under `workspace_dir` is in `WORKSPACE_INTERNAL_DIRS`
  (`types.rs:173`) or `WORKSPACE_INTERNAL_FILES` (`types.rs:211`), or starts
  with `memory-`, `memory_tree-`, or `session_raw-`. Checked against the
  canonicalized form of both paths when possible, falling back to the raw
  paths for not-yet-existing targets.
- **`is_always_forbidden`** (`path_checks.rs:471`) — case-insensitive,
  segment-based match against `SENSITIVE_COMPONENTS` (`.ssh`, `.gnupg`,
  `.aws`, `.azure`, `.kube`, `keychains`, Windows DPAPI dirs) plus a prefix
  match against `SYSTEM_PREFIXES` (`/etc`, `/root`, `/boot`, `/proc`, `/sys`,
  `/system`, `C:\Windows`, `C:\Program Files[  (x86)]`, `C:\ProgramData`).
  Unconditional — a `trusted_root` grant can never reach these paths. Gray-area
  directories (`/usr`, `/opt`, `/var`, `~/Library`) deliberately stay in the
  user-overridable `forbidden_paths` list instead, so a grant can still reach
  e.g. `/usr/local/...`.
- **`classify_command`'s fail-closed floor** (`command_checks.rs:126`) — a
  command that is not provably read-only (and not a recognized
  network/destructive command) is at least `CommandClass::Write`. The highest
  class across `;`/`|`/`&&`/`||`/newline-separated segments wins, and any
  unquoted redirect (`>`, `>>`) or `tee` lifts the whole command to at least
  `Write`. An LLM-declared category (`parse_declared_class`) is
  escalate-only: callers combine it as `gate = classify_command(cmd).max(declared)`,
  so the model can raise the gate but never lower what the runtime determined.
- **Approval gate default-on, marker semantics.** `POLICY_BLOCKED_MARKER`
  (`[policy-blocked]`) prefixes a *permanent* rejection — the identical call
  can never succeed in the current tier (read-only blocking a write, a
  forbidden/credential path, a disallowed high-risk or hidden-execution
  command). `POLICY_DENIED_MARKER` (`[policy-denied]`) prefixes a *this-turn*
  denial — the user said no to an approval prompt, or it timed out. The agent
  harness's `RepeatedToolFailureMiddleware` keys on these to halt on the first
  verbatim repeat instead of reiterating a provably-futile call. See
  [`../approval/README.md`](../approval/README.md) for the gate itself and its
  10-minute default TTL.
- **Hourly action budget.** `ActionTracker` (`types.rs:124`) is a sliding
  one-hour window; `enforce_tool_operation`'s `Act` arm
  (`enforcement.rs`) calls `record_action` and refuses once the count exceeds
  `max_actions_per_hour`. `enforce_write_tier` alone (tier only, no budget) is
  used by callers that write at a finer grain than one tool call, e.g. the
  kernel memory guard under `MemoryCore::store`.

## Tests

- `policy_tests.rs` plus `policy_tests_part_01..05_tests.rs` — behavior tests
  for classification, path checks, and tier enforcement.
- `proptest_tests.rs` — property tests over command classification.
- `enforcement_scratch_dir_tests_tests.rs` — `openhuman_scratch_dir`/
  `ensure_openhuman_scratch_dir` symlink/TOCTOU hardening.
