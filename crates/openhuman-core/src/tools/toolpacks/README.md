# toolpacks

On-demand tool disclosure. A tool's JSON schema is charged on every provider
call of every turn whether or not the tool is used; the Master Agent's belt
measured ~21.4k tokens of schema against a ~4.3k-token system prompt, so most
of that fixed cost is idle in most conversations. A pack keeps its tools
constructed and executable but unadvertised: the agent sees one small tool,
`use_skill`, instead of the pack's real schemas.

## How it works

- [`types::ToolPack`] is the unit: an `id`, a one-line `summary` shown in the
  always-on pack index, the `tools` it owns, and `owners` — agent ids the pack
  is *not* applied to, because the specialist that a family was delegated to
  (e.g. `settings_agent` for `system`) should not pay a `use_skill` round trip
  on every call of its own belt.
- [`registry::PACKS`] is the compiled-in table. Membership is a build-time
  decision on purpose: a pack config or RPC could edit would let a caller move
  a dangerous tool out of the reviewed, advertised surface without review.
  `registry::pack`, `pack_for_tool`, `all_packed_tool_names`,
  `packed_tool_names_for_agent`, and the two `pack_index_markdown*` /
  `callable_pack_ids` helpers all read this table.
- [`tools::UseSkillTool`] (name constant `USE_SKILL = "use_skill"`) is the one
  always-on proxy tool. Called with `skill` alone it renders that pack's tool
  schemas into the conversation (`render_pack_filtered`); called with `skill` +
  `tool` + `args` it resolves and executes the real tool, forwarding
  permission level, timeout policy, and external-effect classification so
  nothing is laundered through the proxy. [`tools::PackRegistryHandle`] is the
  late-bound, non-owning (`Weak`) view into the agent's tool registries the
  proxy dispatches through — two of them (durable + `synthesized_tools`),
  since `delegate_*` tools live in the second and both must be rebound after
  every registry rebuild (see `ops::bind_pack_registry` /
  `ops::bind_synthesized_pack_registry` doc comments for the exact call
  points and staleness failure mode).
- [`ops::strip_packed_from_visible`] does the actual compression: it removes
  a pack's tool names from an agent's advertised `visible` set and adds
  `use_skill` back in only if something was actually withheld.
  [`ops::is_withheld_from`] exposes the same predicate for callers building a
  tool *listing* rather than mutating a `visible` set (the collapsed
  `delegate_to` tool).

**Why a proxy instead of dynamic registration.** Registering the real schemas
mid-turn would be better (native tool calling, no nested `args` object), but
`tinyagents` registers tools into a plain `HashMap` behind `&mut` before the
turn starts and derives the provider-facing schema list once from that.
Making the registry interior-mutable and re-deriving schemas per iteration is
the upstream change that would retire this proxy.

## `ToolGroups`

[`groups::ToolGroups`] (fixed-size array, `GROUP_COUNT = PACKS.len()`) is the
`CoreBuilder`-facing narrowing control referenced from `AGENTS.md` ("tool
visibility with `ToolGroups`"). Per pack it holds a [`groups::GroupMode`]:

| Mode | Schemas on the wire | Registered and callable |
| --- | --- | --- |
| `Advertised` | yes | yes |
| `Withheld` (default) | no, reached via `use_skill` | yes |
| `Off` | no | no |

`ToolGroups::default()`/`packed()` puts every pack in `Withheld` — exactly the
compiled-in behavior before this type existed, so a host that never calls
`CoreBuilder::tool_groups` is unaffected. This axis only narrows: a pack
compiled out by a Cargo feature, or off under the ambient `DomainSet`, stays
absent regardless of the mode set here. `groups::current()` reads the ambient
`ToolGroups` off `CoreContext`, falling back to `Withheld`-everywhere when
there is no context (unit tests, pre-boot CLI paths).

## Name collision

`use_skill` here is the tool-pack disclosure proxy in this module. It is
unrelated to `skills::runtime::run_skill`, which is itself one of the tools
packed under the `skills` pack (see `registry::PACKS`) — do not conflate the
two when tracing a `run_skill` call.

## Called by

- `crates/openhuman-core/src/tools/ops.rs` — registers `use_skill` into the
  default/all tool registries.
- `crates/openhuman-core/src/core/runtime/builder.rs` and
  `crates/openhuman-core/src/core/runtime/context.rs` — `CoreBuilder::tool_groups`
  plumbing and the ambient `ToolGroups` on `CoreContext`.
- `crates/openhuman-core/src/agent/harness/session/builder/` — binds the pack
  registry (durable and synthesized) when an agent's tool `Arc`s are built or
  rebuilt, and strips packed names from the agent's visible set.
- `crates/openhuman-core/src/agent/orchestration/tools/collapsed_delegation.rs`
  — consults `is_withheld_from` so a collapsed `delegate_to` listing never
  re-advertises a route the pack table withholds.
- `crates/openhuman-embed/` — `Harness` builders expose `ToolGroups` to
  embedding hosts.

## Tests

`toolpacks_tests.rs` and `toolpacks_tests_part_02_tests.rs` (pack table
invariants, `strip_packed_from_visible`, `render_pack_filtered` behavior);
`groups_tests.rs` (`GroupMode` defaults and narrowing rules).
