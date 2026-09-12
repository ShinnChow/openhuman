# prompts

Owns the bundled prompt assets and the system-prompt rendering pipeline: the
types every section renders from, the section implementations themselves, the
builder that assembles them in order, and the free `render_*` helpers other
domains use to compose their own prompts.

## Public surface (from `mod.rs`)

- `types::*` — `PromptContext`, `PromptSection` trait, `PromptTier`,
  `PromptTool`, `ToolCallFormat`, `LearnedContextData`, `UserIdentity`,
  `ConnectedIntegration` and friends. Pure data; no rendering logic, so
  editing a type doesn't recompile the whole renderer.
- `render_connected_identities` (`connected_identities.rs`) — best-effort,
  sync render of the `## Connected Identities` block; reads through the bound
  memory driver via `block_in_place`, returns empty on any failure.
- `agents_md::{load_agents_md, load_agents_md_layers, AgentsMdContent, AGENTS_MD_FILENAME}`
  — loads `AGENTS.md` at the global (`workspace_dir`) and local
  (`action_dir`, or a sub-agent's `worktree_action_dir` override) layers.
- `builder::{SystemPromptBuilder, TieredPrompt, GLOBAL_STYLE_SUFFIX}` —
  assembles ordered `PromptSection`s into a final prompt string (or a
  `TieredPrompt` with cache-breakpoint offsets).
- `sections::*` — the concrete `PromptSection` structs (`IdentitySection`,
  `ToolsSection`, `SafetySection`, `UserFilesSection`, `UserMemorySection`,
  `WorkspaceSection`, `DateTimeSection`, `RuntimeSection`,
  `AgentsInstructionsSection`, `PersonalityRosterSection`,
  `ArchetypePromptSection`, `DynamicPromptSection`, …).
- `render_helpers::*` — free `render_*` functions (thin wrappers over the
  section structs) plus workspace-file injection helpers
  (`inject_workspace_file`, `inject_inline_content`, `sync_workspace_file`,
  `default_workspace_file_content`) and the sub-agent renderer
  (`render_subagent_system_prompt[_with_format]`). Lets a caller assemble a
  prompt by calling functions directly instead of going through
  `SystemPromptBuilder`.

## Bundled assets

`IDENTITY.md`, `ROLE.md`, `SOUL.md`, `STYLE.md`, `USER.md` are the seed copies
of the master agent's identity, role, personality, writing-style, and
user-adaptation files. They are pulled in via `include_str!` (`builder.rs`
for `STYLE.md`; `render_helpers_part_02.rs` for `SOUL.md`, `IDENTITY.md`,
`ROLE.md`) and used two ways:

- **Compile-time fallback constant** — e.g. `GLOBAL_STYLE_SUFFIX` — used when
  the workspace copy can't be read.
- **Seed content** — `sync_workspace_file` copies the bundled text into the
  user's workspace (`<workspace_dir>/SOUL.md`, etc.) the first time it's
  missing, so a fresh workspace still gets the rules; a user's on-disk edits
  win from then on. `USER_FILE_MAX_CHARS` and `BOOTSTRAP_MAX_CHARS` (in
  `types.rs`) cap how much of the on-disk copy gets injected.

Editing these files changes the shipped default agent's persona/style; they
are not code and do not need a doc comment, but changes here are product
changes — read `SOUL.md`/`STYLE.md` before touching them.

## Compat shim

`agent::context::prompt` (`agent/context/prompt.rs`) is `pub use
crate::agent::prompts::*` — prompt plumbing used to live there and was moved
here so it sits next to the agents that consume it. The shim is a stable
import path only; do not add logic to it. Both `crate::agent::prompts::*` and
`crate::agent::context::prompt::*` resolve to the same types.

## Extension points (owned elsewhere)

Other domains contribute additional `PromptSection`s that plug into a
`SystemPromptBuilder` built here, rather than living in this directory:

- `agent/learning/prompt_sections.rs` — `LearnedContextSection`,
  `UserProfileSection`, `MemoryAccessSection` (config-gated, appended by the
  session builder when learning is enabled).
- `agent/profiles/prompt_section.rs` — cross-profile workspace notice.
- `tools/agent_policy/prompt.rs` — tool-policy-boundary block.

Built-in archetype system prompts (orchestrator, welcome, integrations_agent,
…) live in `agent/registry/agents/<name>/prompt.rs` and `flows/agents/*/`, not
here — those modules hand-assemble their body via the `render_*` helpers and
feed it into `SystemPromptBuilder::from_dynamic` or `from_final_body`.

## Builder entry points

- `SystemPromptBuilder::with_defaults()` — the primary-agent chain (identity,
  user files, `AGENTS.md`, user memory, tools, safety, workspace, datetime,
  runtime).
- `SystemPromptBuilder::for_subagent(...)` — narrow chain driven by a sub-agent
  definition's `omit_*` flags; deliberately excludes `DateTimeSection` so
  repeat spawns of the same definition stay byte-identical for prefix-cache
  reuse.
- `SystemPromptBuilder::from_dynamic(...)` / `from_final_body(...)` — wrap an
  already-assembled body (the ~26 `agents/<id>/prompt.rs` builders) while
  still injecting the shared `AgentsInstructionsSection` and grounding/style
  suffix.

## KV-cache / prefix stability

The rendered prompt is frozen for the life of a session
(`agent/harness/session/mod.rs`) and reused on every turn so the inference
backend's prefix cache hits. `PromptSection::tier()` (`PromptTier::Stable` /
`Context` / `Volatile`) controls emission order in
`SystemPromptBuilder::build_tiered`: stable bytes (identity, role, safety,
style) first, then per-session-stable context (`AGENTS.md`, workspace), then
per-turn-volatile bytes (memory, profile, connected integrations, the clock)
last — because a prefix is reusable only up to the first differing byte, a
volatile section rendered early invalidates every stable byte behind it.
`NamespaceSummary.updated_at` is rendered as an absolute date rather than a
relative one for the same reason.

## Used by

- `agent/harness/session/turn/context.rs` — loads `AGENTS.md` layers and
  connected identities into `PromptContext` at session start.
- `agent/debug/` — dumps/measures the rendered prompt (`dump_writer.rs`,
  `prompt_size.rs`).
- `agent/context/prompt.rs` — compat re-export shim (see above).
