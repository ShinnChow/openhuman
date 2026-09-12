# registry

User-facing agent registry. Owns the `agent_registry` RPC namespace: the
merged list of shipped default agents and user-authored custom agents, their
enable/disable state, and their tool visibility policy. This is a config
layer over the harness — the actual tool-calling loop and prompt/runtime
implementation live in [`agent/harness`](../harness/).

## Files

- [`types.rs`](types.rs) — wire types: `AgentRegistryConfig` (the persisted
  `entries: Vec<AgentRegistryEntry>`), `AgentRegistryEntry` (id, name,
  description, `source: AgentRegistrySource` — `Default` vs `Custom` —
  enabled, model, optional system prompt, `tool_allowlist`/`tool_denylist`,
  `subagents: AgentSubagentPolicy` allowlist, tags, free-form metadata),
  `AgentRegistryPatch` (partial update), `AgentRegistrySource`.
  `AgentRegistryEntry::validate` enforces id/name/description and
  ASCII-safe id charset for both the entry and its subagent allowlist.
- [`defaults.rs`](defaults.rs) — `default_agents()` loads the built-ins from
  [`agents::load_builtins`](agents/mod.rs) and maps each `AgentDefinition` to
  a registry entry. `definition_from_registry_entry` is the inverse: it
  synthesizes an `AgentDefinition` from a *custom* entry (one with no shipped
  harness definition) so a custom agent runs through the same
  `Agent::from_config_for_agent` factory path as a built-in, with a real tool
  belt instead of a persona-only completion. Every harness-only field
  (`omit_*`, temperature, iteration policy, sandbox mode, tier) takes the
  harness's own safe worker default; see the function's doc comment for the
  exact mapping and why an empty `tool_allowlist` must stay `Named(vec![])`
  rather than collapsing to `Wildcard`.
- [`ops.rs`](ops.rs) — config-backed CRUD: `list_agents`, `get_agent`,
  `upsert_custom_agent`, `update_agent`, `set_agent_enabled`, `remove_agent`,
  `merge_entries` (defaults overlaid with persisted config entries,
  default-first), `find_custom_in_config`. Reads/writes through
  `config::rpc::load_config_with_timeout` and `Config::save`. Tool listing
  (`available_tools`) is sourced from the `tools_agent` built-in — its tool
  scope is the full catalog, unlike the orchestrator's curated subset.
- [`schemas.rs`](schemas.rs) — `ControllerSchema`/`RegisteredController`
  definitions for namespace `agent_registry`: `list`, `get`,
  `available_tools`, `create_custom`, `upsert_custom`, `update`,
  `set_enabled`, `remove`.
- [`rpc.rs`](rpc.rs) — request/response payload types and the `*_rpc` handler
  functions schemas.rs wires up, delegating into `ops.rs`.
- [`tools.rs`](tools.rs) — backwards-compatible re-export of
  `agent::orchestration::tools::*` (tool synthesis now lives there).
- [`agents/`](agents/) — the built-in agent archetypes themselves.

## `agents/`

Each built-in agent owns a subfolder with an `agent.toml` (id, `when_to_use`,
model, tool allowlist, sandbox, iteration cap, `omit_*` flags — parsed
directly into `AgentDefinition`), a `prompt.rs` exposing
`pub fn build(&PromptContext) -> anyhow::Result<String>` wired into
`PromptSource::Dynamic`, and optionally a `graph.rs` for a bespoke
`AgentGraph` runner (agents without one use `AgentGraph::Default`). See the
per-archetype contract documented on [`agents/mod.rs`](agents/mod.rs).

[`agents/loader.rs`](agents/loader.rs) owns `load_builtins` (walks the
`BUILTINS` slice, parses each `agent.toml`, replaces `system_prompt` with the
built-in's static `prompt.md` inline text, stamps
`DefinitionSource::Builtin`) and `validate_tier_hierarchy` (worker-tier
agents may not list subagents; a higher-tier agent may not delegate up the
hierarchy; unknown subagent ids are tolerated here — they're a separate
integrity concern). Workspace-level overrides
(`$OPENHUMAN_WORKSPACE/agents/*.toml`) are merged in separately by
`agent::harness::definition_loader`, replacing built-ins on id collision.

| Archetype | Role |
| --- | --- |
| `archivist` | Background: extracts lessons from a completed session into `MEMORY.md` and FTS5 |
| `code_executor` | Repo-scoped worker: locate/read/edit/build/test/git for any repo work |
| `context_scout` | Read-only pre-flight context bundle (memory, goals, integrations, web) |
| `critic` | Adversarial, read-only reviewer of diffs/code against project rules |
| `crypto_agent` | Wallet/market specialist: balances, swaps, contract calls, x402 paid requests |
| `flow_memory_agent` (feature `flows`) | Read-only context/memory retrieval for automation-flow `agent` nodes |
| `goals_agent` | Background: keeps `MEMORY_GOALS.md` fresh from session context |
| `help` | Answers "how does OpenHuman work" questions from the bundled GitBook docs |
| `image_agent` | Image generation/edit specialist |
| `integrations_agent` | Drives a single Composio toolkit (gmail, notion, github, …) per spawn |
| `mcp_agent` (feature `mcp`) | Calls tools on an already-connected MCP server |
| `mcp_setup` | Walks the user through installing/connecting a new MCP server |
| `morning_briefing` | Proactive scheduled daily summary (tasks, calendar, email, skills) |
| `orchestrator` | Default user-facing agent; direct-first, delegates only when it materially helps |
| `planner` | Read-only architect: breaks a task into a DAG of subtasks with acceptance criteria |
| `presentation_agent` | Builds decks from evidence; owns grounding/citations/image verification |
| `profile_memory_agent` | Profile, persona, preferences, people-graph specialist |
| `researcher` | Web/docs crawler that compresses findings to dense markdown; has a custom `graph.rs` |
| `scheduler_agent` | Reminders, recurring jobs, cron — time/cron tools only, no live calendar reads |
| `settings_agent` | App/core config, health/model diagnostics, service lifecycle, security policy |
| `skill_creator` | Creates/updates SKILL.md packages and Node-backed JS helpers |
| `summarizer` | Runtime-dispatched only: compresses oversized tool results for the orchestrator |
| `task_manager_agent` | Task-board/task-source specialist: cards, feeds, artifacts, status |
| `tool_maker` | Narrow self-healer: writes a polyfill when a host command is missing |
| `tools_agent` | Generalist heavy execution (shell/HTTP/web/files) that never touches a repo or git |
| `trigger_reactor` | One or two tool calls in direct reaction to an external trigger, no planning |
| `trigger_triage` | Classifies an external trigger into drop/acknowledge/react/escalate; never acts |
| `video_agent` | Video generation/animation specialist |
| `vision_agent` | Read-only image understanding: describe, OCR, locate UI elements |

`flow_memory_agent` and `mcp_agent` are compiled out entirely when their
gating feature (`flows`, `mcp`) is disabled; the orchestrator's `agent.toml`
subagent list still names `mcp_agent` unconditionally (TOML can't be
`cfg`'d), and both the tool-synthesis path and `validate_tier_hierarchy`
tolerate that dangling id rather than failing boot.

## Called by

- `agent/harness/definition_loader.rs` — merges built-ins with workspace TOML
  overrides into the running `AgentDefinitionRegistry`.
- `agent/library/` — shared agent-authored content referenced by archetypes.
- `tools/orchestrator_tools.rs` — synthesizes one named delegation tool per
  id in the orchestrator's `subagents` allowlist, pulling each tool's
  description live from the target's `when_to_use` so it never drifts from
  a hardcoded table.

## Tests

`defaults_tests.rs`, `ops_tests.rs`, `schemas_tests.rs`, `types_tests.rs`,
and under `agents/`: `loader_tests.rs` (+ `loader_tests_part_0N_tests.rs`).
