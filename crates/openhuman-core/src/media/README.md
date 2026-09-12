# Media

Family root for media-related agent tool contracts. Agent-tools-only: no
controller, store, or bus subscriber is tagged `DomainGroup::Media`.

## Members

- [`generation`](generation/mod.rs) — the `media_generate_*` agent tools
  (image/video via GMI, proxied through the TinyHumans backend). Wired: built
  by `build_media_tools()` and registered from
  `crates/openhuman-core/src/tools/ops.rs`.
- [`image`](image/README.md) — image tool contracts scaffold. Currently
  unwired (#2997); no tool registers these contracts yet.

## Gate

Both children are wholly gated behind the `media` feature
(`#[cfg(feature = "media")] pub mod media;` in
`crates/openhuman-core/src/lib.rs`). `media` is a default feature
(`crates/openhuman-core/Cargo.toml`) and is forwarded explicitly from
`crates/openhuman-app/Cargo.toml` and listed in
`scripts/ci/product-features.txt`, per the feature-forwarding rule in
`AGENTS.md`.

It is a **surface-only** gate: media generation is backend-proxied over the
shared `reqwest` client, and the `image` crate's types are shared with channel
upload, so no exclusive dependency is shed by disabling it.

## `generation`

The backend (`/agent-integrations/media-generation/*`) owns provider keys,
billing, and the standardized contract. The tools here submit a generation
request, block with progress until it completes, download the resulting media
into `generated-media/` under the agent's `action_dir`
(`download.rs::GENERATED_MEDIA_DIR`), and return local file paths.

Exported tools (`tools.rs`):

- `MediaGenerateImageTool` — `name() == "media_generate_image"`
- `MediaGenerateVideoTool` — `name() == "media_generate_video"`
- `MediaListModelsTool` — `name() == "media_list_models"`

Tests: `generation/download_tests.rs` (extension/path derivation),
`generation/tools_tests.rs` (tool schema/execution behavior).

## Related docs

- [`image/README.md`](image/README.md)
- [`gitbooks/features/native-tools/media-generation.md`](../../../../gitbooks/features/native-tools/media-generation.md)
