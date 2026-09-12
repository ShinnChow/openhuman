# claude_code

`ChatModel` implementation that drives Anthropic's `claude` CLI as a
subprocess instead of calling the Messages API directly. Selected by the
`claude-code:<model>` provider string in `../factory.rs`
(`p.starts_with(crate::inference::provider::claude_code::PROVIDER_PREFIX)`).

> v2 will expose OpenHuman's native Rust tools back into the CLI over MCP;
> this Phase 2 cut runs the driver end-to-end with native CC built-ins
> disabled at the caller (no `--allowedTools` set means CC's own tools simply
> don't fire during a non-interactive `-p` turn). — `mod.rs`

## CLI invocation

`ClaudeCodeProvider::run_chat` spawns `claude` with
`-p --output-format stream-json --verbose --include-partial-messages
--resume <uuid>` (the resume UUID comes from `session_store.rs`, keyed by a
stable hash of the conversation's first user message — real OpenHuman
thread ids are not yet plumbed through `ChatRequest`). Up to
`MAX_CONCURRENT_TURNS` (4) child processes run at once, gated by a
`Semaphore` held on the provider.

## File map

| File | Role |
| --- | --- |
| `mod.rs` | `ClaudeCodeProvider` (`ChatModel<()>` impl), `PROVIDER_PREFIX`, `workspace_dir_from_config`, `from_env` (CLI discovery + version gate), `run_chat`, `thread_key_from_messages`. |
| `auth.rs` | Resolve `ANTHROPIC_API_KEY` for the spawned CLI. |
| `auth_status.rs` | Report the CLI's own auth state (subscription vs API key vs signed out) for the settings UI. |
| `driver.rs` | Subprocess lifecycle: turn timeout, macOS Seatbelt jail, MCP config, stdin/stdout piping. |
| `event_mapper.rs` | `ClaudeCodeEvent` → `ProviderDelta` / aggregated `ChatResponse`; tool-call blocks are tracked but deliberately not surfaced (CC's own tools don't fire in `-p` mode). |
| `input_builder.rs` | Builds the stream-json stdin payload (`--input-format stream-json`); full history on a new session, only the last user turn on `--resume`. |
| `session_store.rs` | Thread id → CC session UUID persistence (`claude-code-sessions.json`). |
| `settings.rs` | Persisted `claude_code_settings.json` — the one user-facing toggle (`bypassPermissions`/full-access vs default `acceptEdits`). |
| `stream_parser.rs` | Line-buffered JSONL parser for `claude --output-format stream-json`; permissive `serde_json::Value` payloads so a minor CLI schema bump doesn't break parsing. |
| `types.rs` | `MIN_CLI_VERSION`, `CliStatus`. |
| `version_check.rs` | Probes the `claude` binary and its version against `MIN_CLI_VERSION`. |

## Auth resolution order

1. Process env `ANTHROPIC_API_KEY` (highest precedence) — `auth.rs::resolve`.
2. `~/.claude/.credentials.json`, passed through transparently by *not*
   setting the env var — the CLI reads its own credentials file.
3. `auth_status.rs` reports richer state for the UI by spawning
   `claude auth status --json` (bounded to `AUTH_STATUS_TIMEOUT` = 10s) rather
   than reading the credentials file directly, because on macOS the CLI
   stores credentials in the Keychain (service `Claude Code-credentials`),
   not in that file — a logged-in macOS user would otherwise be
   misreported as signed out. Older CLIs without `auth status` map to
   `AuthSource::Unknown`, never to "signed out".
4. Auth-profile-store integration (an Anthropic key from OpenHuman settings)
   and full Claude Pro/Max OAuth are both future work (v1.1 / v2 per
   `auth.rs`'s module doc).

Env resolution races on `ANTHROPIC_API_KEY`/`OPENHUMAN_CLAUDE_CODE_*` between
parallel tests are serialized through `mod.rs::ENV_TEST_LOCK`.

## Sandbox and loopback MCP

On macOS, `driver.rs` wraps the `claude` spawn in a Seatbelt (`sandbox-exec`)
jail by default (opt out via `OPENHUMAN_CLAUDE_CODE_*`), denying the CLI's own
tools read/write access to the *entire* `~/.openhuman[-staging]` tree — not
just the per-user workspace subdir, since `workspace_dir` for a given user is
a subdirectory of that root and a narrower deny would leave siblings
readable.

That would also block the coding agent from OpenHuman's memory and tools, so
`driver.rs` points the CLI at the in-process HTTP MCP server
(`crate::mcp::server::local`, `ensure_local_http`) instead, via a per-turn
`--mcp-config` JSON written to a scratch dir. The MCP server runs in the
**unjailed core process** — not as a child of the sandboxed `claude` — and is
reached over an authenticated loopback HTTP connection
(`127.0.0.1:<port>`), so it keeps full access to `~/.openhuman` for memory
while CC's own raw tools stay denied that path by the jail. See
`crate::mcp::server::local`'s module doc for the same arrangement from the
server side.

`MAX_CONCURRENT_TURNS`, `DEFAULT_TURN_TIMEOUT_SECS` (900s, overridable via
`OPENHUMAN_CLAUDE_CODE_TURN_TIMEOUT_SECS`), and the jail are all defined in
`driver.rs`.

## Selection

`../factory_part_01.rs` routes a `claude-code:<model>` provider string to this
module wherever the factory needs to special-case CC alongside
`claude_agent_sdk` — e.g. `is_raw_passthrough_model`, `external_provider_label`
(labels it "Claude Code CLI" for Privacy Mode messages), and the
local/cloud/CLI dispatch branch in `create_chat_model*`.
`ClaudeCodeProvider::from_env` fails fast with an actionable error when the
CLI is missing or below `MIN_CLI_VERSION`.

## Tests

Per-file `*_tests.rs` alongside each module (`auth_tests.rs`,
`auth_status_tests.rs`, `driver_tests.rs`, `event_mapper_tests.rs`,
`input_builder_tests.rs`, `session_store_tests.rs`, `settings_tests.rs`,
`stream_parser_tests.rs`, `version_check_tests.rs`) plus `mod_tests.rs` for
the provider's `ChatModel` impl and session-key hashing.
