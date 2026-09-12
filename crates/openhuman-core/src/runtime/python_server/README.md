# runtime/python_server

The long-lived Python worker that keeps model-backed backends warm. This
module owns the process lifecycle and wire protocol; interpreter resolution
(finding or provisioning a `python` binary) belongs to the sibling
[`runtime/python`](../python/README.md) client, which this module calls into
to pick the interpreter it launches.

Two backends currently run inside the one worker process:

- **spaCy** (`spacy.rs`) — NER extraction for the memory tree's query
  extractor, gated by `config.memory_tree.spacy_enabled`.
- **Kompress** (`kompress.rs`) — TokenJuice's ModernBERT/torch plain-text
  compressor, gated by `config.tokenjuice.ml_compression_enabled`.

Both are gated behind `config.runtime_python.enabled`; `registry::enabled_backends`
computes the active set from all three flags together.

## Key files

| File | Role |
| --- | --- |
| `mod.rs` | Submodule decls and `pub use` re-exports. |
| `server.rs` | Process lifecycle: `RuntimePythonServer` (spawn, request/response, restart-on-failure, idle expiry), the process-wide `ensure_started`/`status` cache. |
| `protocol.rs` | JSONL wire types: `PythonServerRequest`, `PythonServerResponse`, `PythonServerError`, `ReadyLine`, `PROTOCOL_VERSION`. |
| `registry.rs` | `RuntimePythonBackend` enum (`Spacy`, `Kompress`) and `enabled_backends(config)`. |
| `kompress.rs` | Kompress venv provisioning (`ensure_kompress`, `install_into`) and the compress request (`request_kompress`). |
| `spacy.rs` | spaCy venv provisioning (`ensure_spacy`) and the extract request (`extract`). |
| `types.rs` | `BackendStatus`, `RuntimePythonServerStatus` serde types. |
| `server.py` | The worker script itself, embedded via `include_str!` and written to the cache dir at launch. |

## Lifecycle

`ensure_started(config)` is the single entry point. It caches one
`Arc<RuntimePythonServer>` behind a process-wide `OnceLock<Mutex<ServerCache>>`
(`Empty` / `Ready` / `Failed { message, retry_after }`):

- If the enabled backend set has changed since the cached server launched
  (e.g. Kompress toggled on after a spaCy-only start), the cache is discarded
  and a new server is started — the running process was never provisioned for
  the new backend.
- If the Kompress backend has been idle longer than
  `config.tokenjuice.ml_sidecar_idle_timeout_secs`, the server is torn down and
  restarted on the next request, freeing the torch process's memory.
- A startup failure caches `Failed` with a five-minute (`START_FAILURE_BACKOFF`)
  retry window so a broken venv does not retry on every call.
- `RuntimePythonServer::request` sends one request, and on failure resets the
  child and retries once before giving up.

`prepare_launch` picks **one** interpreter for the whole worker: if spaCy is
enabled it owns the venv and Kompress (if also enabled) installs torch +
transformers into it via `kompress::install_into`; if only Kompress is enabled
it gets its own dedicated venv via `kompress::ensure_kompress`; if neither
backend needs a venv the worker still runs under the base interpreter from
`runtime::python::PythonBootstrap`.

## Wire protocol

JSONL over the child's stdin/stdout (`protocol.rs`), one line per message:

- Startup handshake: the worker writes a `ReadyLine` (`ready`, `protocol`,
  `backends`, optional `error`) before any request is sent; a `protocol`
  mismatch against `PROTOCOL_VERSION` or `ready: false` fails the launch.
- Requests: `PythonServerRequest { id, method, params }`, methods namespaced by
  backend (`spacy.extract`, `kompress.compress`).
- Responses: `PythonServerResponse { id, ok, result, error }`, matched back to
  the request by `id`; the read loop skips lines with a stale/unparseable `id`
  and enforces a 60s per-request timeout (`REQUEST_TIMEOUT`, sized for the
  slower Kompress backend, not added latency for spaCy).

stderr is drained continuously by a background task and only logged at
`debug`/`trace`, never surfaced to callers.

## Backends

**spaCy** (`spacy.rs`): `ensure_spacy` provisions a dedicated or shared venv
(`pip install spacy click`, `python -m spacy download en_core_web_sm`),
tracked by a versioned ready marker (`SPACY_READY_MARKER_VERSION`) so a
package-set change forces re-provisioning; `spacy_provisioned` is a cheap,
network-free check used by `harness_init`. `extract` sends `spacy.extract` and
returns the shared `tinymemory_api::host::SpacyResponse` type. Called from
`modules::memory_host` for the memory tree's query extractor.

**Kompress** (`kompress.rs`): `ensure_kompress` provisions a CPU-only torch +
transformers venv and pre-downloads `config.tokenjuice.ml_model_id`;
`install_into` does the same into an existing (spaCy) venv when both backends
share one interpreter. The worker loads the model fully offline
(`HF_HUB_OFFLINE=1`, `TRANSFORMERS_OFFLINE=1`) so startup never depends on the
network once provisioned. `request_kompress` sends `kompress.compress` with
`target_ratio` / `max_input_chars` from `config.tokenjuice`. Called from
`inference::tokenjuice::ml`.

Both provisioning paths are serialized behind their own `tokio::sync::Mutex`
(`provision_lock` / `spacy_provision_lock`) so concurrent callers don't race
the same venv build.

## Status

`RuntimePythonServerStatus { enabled, running, backends: Vec<BackendStatus>, message }`
is returned by `status()` and reflects the cache directly — `Empty` reports
`disabled`, `Failed` reports the last error, `Ready` reports each backend's
`ready` flag from the worker's handshake `backends` list. There is no public
RPC method for this; `crates/openhuman-core/src/agent/harness_init/registry.rs`
polls it during startup to decide whether the `runtime_python_server` init
step is done. The about-app capability catalog
(`platform/about_app/catalog_part_02.rs`, id `local_ai.python_runtime_installer`,
domain `runtime_python`) describes the managed-interpreter capability this
worker depends on, not this module's own status.

## Persistence

No domain store. Provisioned venvs and the written `server.py` live under
`runtime_python.cache_dir` (or the OS cache dir, or `workspace_dir` as a last
resort) via `spacy::python_server_cache_root`; the Kompress HF cache is a
subdirectory of the same root.

## Security

The worker runs under whichever interpreter `runtime::python::PythonBootstrap`
or the venv provisioning resolved — it does not choose or sandbox that
interpreter itself. Environment is explicit and minimal: the enabled backend
list (`OPENHUMAN_RPS_BACKENDS`) and, for Kompress, the model id, device,
compression settings, and `HF_HOME`/offline flags. `process_util::apply_no_window`
suppresses the Windows console flash on every spawned step. Sandbox/approval
policy for what reaches this worker (e.g. text passed to `spacy.extract`) is
decided by the caller before the request is sent, not here.

## Used by

- `crates/openhuman-core/src/agent/harness_init/registry.rs` — the `runtime_python_server`,
  `spacy`, and `kompress` init steps.
- `crates/openhuman-core/src/modules/memory_host.rs` — `extract_spacy` for the memory
  tree's query extractor.
- `crates/openhuman-core/src/inference/tokenjuice/ml/mod.rs` — `request_kompress` for
  plain-text compression.

## Notes / gotchas

- The server always runs a **single** interpreter: enabling Kompress after
  spaCy is already running installs torch into the spaCy venv rather than
  starting a second process.
- Restart-on-failure in `RuntimePythonServer::request` means a transient
  worker crash is invisible to callers except for added latency on the retried
  call.
- `python_server_cache_root` also holds legacy `memory-nlp` spaCy venvs;
  `spacy.rs` migrates or reuses them so upgrades don't force re-provisioning.
