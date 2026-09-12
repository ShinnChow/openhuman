# profiles

Persistent, user-selectable agent "flavours". A profile carries a name,
description, an `agent_id`, runtime defaults (model override, temperature,
system-prompt suffix, SOUL.md), and configurable allowlists for tools, skills,
MCP servers, Composio integrations, and memory sources. Selecting a profile
changes how the agent introduces itself, what it remembers, and what it can do.
State persists under `<workspace>/agent_profiles.json`; the module also owns
each profile's on-disk "home" (identity/memory files under `workspace_dir`,
optional dedicated workspace under `action_dir`) and the guard that keeps
dedicated-workspace profiles from writing into a sibling's directory.

## Responsibilities

- Define the `AgentProfile` / `AgentProfilesState` serde types and the
  built-in profile set (`default`, `reasoning`, `research`, `planner`,
  `review`).
- Persist and normalise profile state (`AgentProfileStore`): merge built-ins
  into any loaded/saved state, slugify ids, drop empty allowlist entries to
  `None`, and auto-assign a stable numeric `memory_dir_suffix` to new custom
  profiles.
- Materialize and reconcile each profile's on-disk "home" — `SOUL.md`,
  `MEMORY.md`, a private `skills/` dir, and (opt-in) a dedicated workspace.
- Resolve profile-scoped paths for prompt building: which SOUL.md content and
  MEMORY.md to use, the effective memory subtree suffix, and a session
  signature that changes when a profile's resolved inputs change.
- Enforce the cross-profile write guard: block a dedicated-workspace profile's
  tool calls from targeting a sibling profile's workspace.
- Expose the `profiles` RPC namespace (list/select/upsert/delete), wired into
  the core registry.

## Key files

| File | Role |
| --- | --- |
| `mod.rs` | Module docs, `mod` decls, and the re-export surface. |
| `types.rs` | `AgentProfile`, `AgentProfilesState`, `DEFAULT_PROFILE_ID`, `profile_signature` (a serialized cache key for prompt construction). |
| `store.rs` | `AgentProfileStore` (load/save/select/upsert/delete/resolve) over `<workspace>/agent_profiles.json`; `built_in_profiles`, `load_profiles`, id normalisation/slugification, and the numeric `memory_dir_suffix` allocator. |
| `home.rs` | Per-profile home materialization: `profile_home`, `profile_action_workspace`, `profile_skills_dir`/`profile_skills_root`, `validate_profile_id`, `ensure_profile_home` (idempotent seed), `sync_soul_md_on_upsert` (reconcile an edited inline soul into the on-disk file), `dedicated_workspace_dir`. |
| `paths.rs` | Personality-scoped path/content resolution: `resolve_personality_soul`, `resolve_personality_memory_md`, `effective_memory_suffix`, the `*_subdir_for_suffix` helpers, `profile_session_signature`, `PersonalityContext`, `filter_integrations`/`HasToolkit`. |
| `guard.rs` | Cross-profile identity plumbing and write guard: `workspace_policy_id`/`profile_id_from_policy_id` (encode/decode the `WorkspaceDescriptor::policy_id`), `classify_cross_profile_target` (file tools), `scan_command_for_cross_profile` (shell/process tools, best-effort). |
| `prompt_section.rs` | `AgentProfilePromptSection` (renders the `## Agent profile` prompt block) and `cross_profile_workspace_notice`. |
| `ops.rs` | `list`/`select`/`upsert`/`delete` business logic: `agent_id` validation against the global agent registry, home materialization, SOUL.md reconciliation, and read-only path enrichment (`soulMdFile`, `skillsDir`, `workspaceDir`) on the returned payload. |
| `schemas.rs` | Controller schemas + thin handlers for the `profiles` namespace; re-exported as `all_profiles_controller_schemas` / `all_profiles_registered_controllers`. |
| `*_tests.rs` | Sibling test suites per file (via `#[path]`). |

## Profile home layout (hermes-agent style)

```text
<workspace>/personalities/<id>/SOUL.md              identity (hot-read each prompt)
<workspace>/personalities/<id>/MEMORY.md            curated per-profile memory
<workspace>/personalities/<id>/skills/              private skills (owner-only discovery)
<workspace>/{memory,memory_tree,session_raw}-<id>/  dedicated memory subtree (opt-in)
<action_dir>/profiles/<id>/                         agent-writable workspace (opt-in)
```

Identity/memory files live under `workspace_dir`, which the agent's write
tools cannot reach; the writable working dir lives under `action_dir`, which
acting tools are allowed to touch. `SOUL.md` is re-read on every prompt build
so identity edits take effect live. `dedicated_memory` derives a `-<id>`
suffix and wins over the auto-assigned numeric suffix; `dedicated_workspace`
roots a per-profile default cwd for acting tools. `ensure_profile_home` never
overwrites a user's edited files.

## RPC / controllers

Namespace `profiles`, registered via `all_profiles_registered_controllers()`:

| Method | Inputs | Output |
| --- | --- | --- |
| `profiles.list` | — | `{ profiles, activeProfileId }`, each profile enriched with `soulMdFile`/`skillsDir`/`workspaceDir` when present on disk |
| `profiles.select` | `profile_id: string` | updated state payload |
| `profiles.upsert` | `profile: AgentProfile` (JSON) | updated state payload |
| `profiles.delete` | `profile_id: string` | updated state payload |

`upsert` fails closed on a non-empty, non-`orchestrator` `agent_id` when the
global `AgentDefinitionRegistry` is not yet initialised, rather than persist a
reference it cannot validate. Built-in profiles have their home materialized
on first `select`; custom profiles are materialized on `upsert`. Soul edits are
reconciled into the on-disk `SOUL.md` on every `upsert`, built-in included.

## Cron attribution

A cron job may carry a `profile_id` (`cron::CronJob::profile_id`). When set and
the profile still exists, the scheduled run is built under that profile (soul,
memory scope, dedicated workspace, allowlists) via the same profile-aware
session path the task dispatcher uses; a deleted profile falls back to a
profile-less run rather than failing the job.

## Used by

- `crates/openhuman-core/src/core/all.rs` registers the controllers.
- `agent/harness/session/builder/factory.rs` is the primary consumer: resolves
  memory suffix/subdirs, soul/memory content, the dedicated workspace
  descriptor (`workspace_policy_id`), the cross-profile prompt notice, and
  integration filtering when building a session.
- `agent/harness/session/turn/tools.rs` resolves `profile_skills_root` for
  workflow discovery.
- `security/policy/path_checks.rs` and `tools/impl/system/mod.rs` call
  `classify_cross_profile_target` / `scan_command_for_cross_profile` to enforce
  the write guard at the file-tool and process-tool call sites.
- `cron/scheduler_part_02.rs` resolves a job's attributed profile
  (`from_config_for_agent_with_profile`).
- `skills/ops_discover.rs` and `agent/tools/{run_workflow,delegate_to_personality}.rs`
  scope skill discovery and delegation to the active profile.
- `channels/system_prompt.rs` and `web_chat/session.rs` resolve profile-scoped
  prompt content and the session signature for cache invalidation.

## Notes / gotchas

- `validate_profile_id` (hermes grammar `^[a-z0-9][a-z0-9_-]{0,63}$`) gates
  every home read/write path; a legacy id that fails validation gets no home,
  no dedicated workspace, and no profile-local skills — reads and writes stay
  symmetric.
- `scan_command_for_cross_profile` is explicitly best-effort defense in depth
  for process tools (static token scan, not a real shell parser); the hard
  boundary for file mutations is `SecurityPolicy::validate_path`.
- The mod-level doc comment's note about being "relocated from
  `openhuman::agent::profiles` / `::personality_paths`" is historical residue
  from the pre-flattening layout; the current path is `crate::agent::profiles`.
