# `openhuman-embed`

`openhuman-embed` is the host-facing library package for products that run the
OpenHuman core in-process, including Medulla and OpenCompany. It re-exports the
runtime builder from `openhuman-core` and owns the typed embedding facade.

Use the default contributor feature set:

```toml
[dependencies]
openhuman-embed = { git = "https://github.com/tinyhumansai/openhuman", package = "openhuman-embed" }
```

Or select a narrow host build:

```toml
[dependencies]
openhuman-embed = { git = "https://github.com/tinyhumansai/openhuman", package = "openhuman-embed", default-features = false, features = ["inference", "mcp"] }
```

```rust,no_run
use std::sync::Arc;

use openhuman_embed::{Core, CoreBuilder, DomainSet, HostKind, ServiceSet};

# async fn run() -> Result<(), Box<dyn std::error::Error>> {
let runtime = CoreBuilder::new(HostKind::Library)
    .domains(DomainSet::embedded())
    .services(ServiceSet::none())
    .build()
    .await?;
let core = Core::from_runtime(Arc::new(runtime));
let flags = core.config().runtime_flags().await?;
println!("log_prompts={}", flags.log_prompts);
# Ok(())
# }
```

Embedding products should set their product identity once during startup,
before constructing backend clients:

```rust
use openhuman_embed::{set_product_identity, ProductIdentity};

if let Some(identity) = ProductIdentity::new("opencompany") {
    set_product_identity(identity);
}
```

Use `Core::raw()` only as a temporary escape hatch when the typed facade does
not yet model a required call. A repeated raw call is a candidate for a typed
embedding method in `openhuman-embed`.

## Two entry points

`Harness` builds its own runtime from typed inputs — the right choice for a
host that has no `CoreRuntime` of its own yet:

```rust,no_run
use openhuman_embed::{Access, Harness, Provider, Workspace};

# async fn demo() -> Result<(), Box<dyn std::error::Error>> {
let harness = Harness::builder()
    .provider(Provider::openai_compatible("https://api.example/v1", "sk-…").model("gpt-5"))
    .workspace(Workspace::Ephemeral)
    .access(Access::readonly())
    .build()
    .await?;

let first = harness.run("Summarize what you can see.").await?;
println!("{}", first.reply);

let second = harness
    .turn("Now list the risks.")
    .session(&first.session_id)
    .send()
    .await?;
println!("{}", second.reply);
# Ok(())
# }
```

`Core` is the typed facade shown above: a host that already built a
`CoreRuntime` wraps it with `Core::from_runtime` and reaches sub-facades —
`config()`, `auth()`, `agent()`, and, behind the `medulla` feature,
`medulla()`.

Both share the same process-scoped core state, so **only one `Harness` (or
`Core::from_runtime`) may run per process**: the keyring master key, the RPC
bearer, the global event bus, and the `Once`-guarded domain subscribers are
process-scoped, and a second one would silently share them. `Harness::builder().build()`
returns `HarnessError::AlreadyRunning` rather than letting that happen.

Build the tokio runtime yourself — a turn is a large async state machine that
overflows tokio's default 2 MiB worker stack once a sub-agent nests inside it —
using
[`AGENT_WORKER_STACK_BYTES`](../openhuman-core/src/core/runtime.rs) and
`MAX_BLOCKING_THREADS` from `openhuman_core::core::runtime`:

```rust,no_run
use openhuman_core::core::runtime::{AGENT_WORKER_STACK_BYTES, MAX_BLOCKING_THREADS};

let runtime = tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .thread_stack_size(AGENT_WORKER_STACK_BYTES)
    .max_blocking_threads(MAX_BLOCKING_THREADS)
    .build()
    .expect("tokio runtime");
```

Other invariants worth knowing before wiring either entry point:

- Set `config_path` together with `workspace_dir`.
- Set a turn origin with its access tier; `Access::full()` configures both
  access fields at once.
- Copy skills into the harness's workspace rather than symlinking them — skill
  discovery rejects symlinked bundles.

## Feature flags

Every feature on this crate is a pass-through to the same-named feature on
`openhuman-core` (package `openhuman`): `default`, `http-server`,
`inference`, `documents`, `hosting`, `modules`, `voice`, `web3`,
`runtime-node`, `contacts`, `media`, `flows`, `skills`, `mcp`,
`crash-reporting`, `medulla`, `channels`, `sandbox-landlock`,
`sandbox-bubblewrap`, `peripheral-rpi`, `browser-native`, `whatsapp-web`,
`file-logging`, `scheduler-gate`.

Three of them also gate items on this crate's own public surface:

- `medulla` — `Core::medulla()` and the `Medulla*` types.
- `mcp` — `HttpHeader`, `McpAuthConfig`, `McpServer` (harness MCP server
  configuration).
- `skills` — harness skill loading.

See [`docs/library-minimal-recipe.md`](../../docs/library-minimal-recipe.md)
for a measured minimal-footprint feature set.

## Examples and tests

```bash
# Against any OpenAI-compatible endpoint:
OPENHUMAN_EXAMPLE_BASE_URL=https://api.openai.com/v1 \
OPENHUMAN_EXAMPLE_API_KEY=sk-… \
OPENHUMAN_EXAMPLE_MODEL=gpt-5 \
  cargo run -p openhuman-embed --example run_turn -- "What can you see in this directory?"

# Or against the machine's own configured inference, in its real workspace:
OPENHUMAN_EXAMPLE_INHERIT=1 cargo run -p openhuman-embed --example run_turn -- "Hello."
```

Optional: `OPENHUMAN_EXAMPLE_BACKEND_URL` points non-inference backend calls
somewhere specific, and `OPENHUMAN_EXAMPLE_SKILLS_DIR` supplies skill bundles.

The repository root also has `examples/embed_headless.rs` (build a core and
call plumbing methods without a harness) and `examples/embed_kernel.rs`.
`tests/harness_embed.rs` is the end-to-end proof that `Harness` runs a real
turn; `tests/public_api.rs` pins the host-facing embedding contract at compile
time. Run both with `cargo test -p openhuman-embed`.

## Relationship to other crates

`openhuman-embed` depends only on `openhuman-core` (package `openhuman`) with
`default-features = false` — every capability comes from a feature forwarded
above. It does not depend on `openhuman-rpc`; that crate is for out-of-process
callers (the TUI and, over the desktop shell's HTTP relay, the frontend), and
the Tauri shell does not use `openhuman-embed`.
