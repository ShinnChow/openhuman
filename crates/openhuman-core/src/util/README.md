# util

Kernel helper family — always compiled, never feature-gated. These are
dependency-free helpers reused across domains; nothing here may reach into a
domain (`use crate::<domain>::...`). `bm25` and `redact` say why explicitly in
their own module docs; the same rule applies to every file in this directory.

## Layout

| File | Purpose |
| --- | --- |
| `bm25.rs` | BM25 ranking over short documents, shared by `tool_search` and `skill_search`. Deliberately names nothing from `crate::` so it stays extraction-ready for a loadable module. |
| `redact.rs` | `redact()` — SHA-256-based PII redaction for log output (source ids, entity ids, content paths). Kept independent of `tinymemory_core::util::redact`, the engine's own copy, so the host does not link the memory engine just for a log formatter. |
| `retry.rs` | `retry_with_backoff` / `retry_with_backoff_async` and `is_transient_fs_error` — exponential-backoff retry for filesystem operations, mainly to ride out Windows mandatory-file-locking errors (`ERROR_SHARING_VIOLATION`, `ERROR_ACCESS_DENIED`). |
| `sanitize.rs` | Re-exports `tinymcp_bus::sanitize` (`sanitize_for_llm`, `strip_control_chars`, `strip_instruction_fences`, `truncate_utf8_safe`, `MAX_DESCRIPTION_BYTES`, `MAX_TITLE_BYTES`) under the path callers have always used. LLM-facing text sanitization for tool and skill descriptions; the real implementation lives in `tinymcp_bus` so MCP transports and the orchestrator prompt builder share one stripping rule. |
| `text.rs` | UTF-8-safe string helpers: `truncate_with_ellipsis` / `truncate_with_suffix` (char-boundary-safe truncation), `floor_char_boundary` / `ceil_char_boundary` / `utf8_safe_prefix_at_byte_boundary` (byte-boundary rounding), `provenance_tag` (non-leaky `chat:xxxxxxxx` tag hashed from a session id for the cross-chat context block). |
| `types.rs` | `MaybeSet<T>` — tri-state `Set(T)` / `Unset` / `Null` for optional-update payloads. |
| `tls/` | Platform-conditional TLS backend selection for `reqwest` clients — see [tls/README.md](tls/README.md). |

## Public surface

Everything is re-exported at the module root (`pub use` in `mod.rs`), so
`crate::util::<fn>` resolves for every helper above without naming the
submodule.

## Notes

- No file here may depend on another domain. `bm25` and `redact` exist as
  separate copies from otherwise-similar engine/vendor code specifically to
  avoid pulling in a dependency for a few lines of logic — keep new additions
  to the same standard.
- `sanitize.rs` is a thin re-export; the sanitization rule itself is owned by
  `tinymcp_bus` and must not be forked back into this crate.
