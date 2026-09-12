# file_storage

Agent tools for managed cloud file storage, backed by the OpenHuman backend's
`file_storage` provider (S3 under the hood). Files upload from and download
into the agent's `action_dir`; the backend owns the bucket, billing, quota,
and TTL enforcement.

## Responsibilities

- Upload a file from the workspace, list stored files, generate presigned
  download links, change visibility, and delete files — all proxied through
  `/agent-integrations/file-storage/*`.
- Enforce that uploads read from, and downloads write into, the agent's
  `action_dir` (the agent's canonical read/write root); reject paths that
  escape it, including via symlinks.
- Track backend-owned limits: 1 GiB quota per user, TTL 7 days on the free
  plan / up to 1 year on paid plans. Billing goes through the standard
  integration billing flow (S3 rates plus margin), charged upfront for the
  whole TTL on upload and as egress on download/link generation.
- Block all six tools when autonomy is read-only (`SecurityPolicy::can_act`).

## Key Files

| File | Role |
| --- | --- |
| `mod.rs` | Export-only module root; re-exports `build_file_storage_tools` and the six `Storage*Tool` structs. |
| `types.rs` | Serde types for backend responses (`UploadResponse`, `ListFilesResponse`, `FileMeta`, `LinkResponse`, `DeleteResponse`). |
| `tools.rs` | Module doc listing the backend endpoints and billing model; wires in `tools_part_01.rs` / `tools_part_02.rs` via `include!`. |
| `tools_part_01.rs` | Shared helpers (path resolution/validation, filename/mime handling) plus `StorageUploadFileTool`, `StorageDownloadFileTool`, `StorageListFilesTool`. |
| `tools_part_02.rs` | `StorageGetLinkTool`, `StorageSetVisibilityTool`, `StorageDeleteFileTool`, and the `build_file_storage_tools` builder. |
| `tools_tests.rs` | Tool metadata/schema tests and path-resolution/sanitization unit tests. |

## Agent Tools

| Tool name | Struct | Backend route |
| --- | --- | --- |
| `storage_upload_file` | `StorageUploadFileTool` | `POST /agent-integrations/file-storage/files` (multipart) |
| `storage_download_file` | `StorageDownloadFileTool` | `GET /agent-integrations/file-storage/files/{id}/download` |
| `storage_list_files` | `StorageListFilesTool` | `GET /agent-integrations/file-storage/files` |
| `storage_get_link` | `StorageGetLinkTool` | `POST /agent-integrations/file-storage/files/{id}/link` |
| `storage_set_visibility` | `StorageSetVisibilityTool` | `PATCH /agent-integrations/file-storage/files/{id}` |
| `storage_delete_file` | `StorageDeleteFileTool` | `DELETE /agent-integrations/file-storage/files/{id}` |

`storage_list_files`, `storage_get_link`, `storage_set_visibility`, and
`storage_delete_file` take a `file_id`, validated against an
alphanumeric/`-`/`_` charset before it is interpolated into the URL path
(`validate_file_id`).

## Security Notes

- `resolve_upload_path` (`tools_part_01.rs`) resolves the `path` argument
  relative to `action_dir` if not absolute, canonicalizes both the workspace
  root and the candidate, and rejects the upload unless the canonicalized
  path starts with the canonicalized `action_dir` — this also rejects a
  symlink that resolves outside the workspace, and rejects non-regular files.
- Downloads always land under `<action_dir>/storage-downloads/`; the target
  filename is sanitized (`sanitize_filename`) to strip path separators and
  traversal before it is joined onto that fixed directory, so a malicious
  `filename` argument or server-supplied name cannot escape it.
- `execute_with_context` prefers the TinyAgents workspace root from
  `ToolRunContext` over the tool's default `action_dir`
  (`action_dir_for_context`), mirroring the pattern used by media generation
  tools.
- `StorageUploadFileTool`, `StorageDownloadFileTool`, `StorageGetLinkTool`,
  `StorageSetVisibilityTool`, and `StorageDeleteFileTool` all check
  `SecurityPolicy::can_act()` up front and return a
  `[policy-blocked]` `ToolResult::error` under read-only autonomy;
  `StorageListFilesTool` is read-only and unguarded.

## Wiring

`build_file_storage_tools(root_config, action_dir)` (`tools_part_02.rs`) is
the sole construction entry point. It returns an empty tool list when
`crate::integrations::build_client` yields no client (no backend URL
configured, or the user is not signed in), otherwise builds a
`SecurityPolicy` from `root_config.autonomy` and returns all six tools.

Called from `crates/openhuman-core/src/tools/ops.rs` (around line 905):

```rust
tools.extend(crate::integrations::file_storage::build_file_storage_tools(...));
```

The tool structs themselves are re-exported through
`crates/openhuman-core/src/integrations/mod.rs` and
`crates/openhuman-core/src/tools/mod.rs` (`pub use crate::integrations::tools::*;`).

## Dependencies

- `crate::integrations::IntegrationClient` (and `build_client`) — the shared
  backend-proxied HTTP client; see the [parent README](../README.md).
- `crate::security::SecurityPolicy` — gates the mutating tools under
  read-only autonomy.
- `crate::tools::traits` — `Tool`, `ToolResult`, `PermissionLevel`,
  `ToolCategory`.
- `tinytools::ToolRunContext` — supplies the TinyAgents workspace root when
  running inside an agent turn.

## Tests

`tools_tests.rs` covers tool name/permission/category/schema metadata for all
six tools plus unit tests for `resolve_upload_path` and `sanitize_filename`
(traversal and symlink-escape rejection, filename sanitization).
</content>
