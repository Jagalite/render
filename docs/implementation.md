# Rust foundation implementation

M06–M08 now have [named implementation profiles](m07_m08.md) and [complete milestone evidence](../evidence/m06-m08/README.md). Historical M05 Lambertian and [static PBR](static_pbr.md) evidence retain their original scope. External input tracks remain separately qualified in `planning/input_support.json`.

The repository contains experimental M00–M08 milestone profiles. This guide describes the original foundation; [the agent alpha guide](agent_alpha.md) specifies the added scene importer, restricted variants, authored render settings, browser recovery and narrow ABI v1. The original architecture remains the requirements source. Evidence is in `evidence/m00-m04/` and `evidence/m05/`; this is not a complete creative suite.

## Build and run

The pinned toolchain is Rust 1.95.0. Cargo.lock pins dependencies and their checksums.

```sh
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo run --release -p render-host -- demo artifacts/demo --gpu
cargo run --release -p render-host -- verify artifacts/evidence --gpu
cargo run --release -p render-host --example m00
sh scripts/build-web.sh
cargo run -p render-host -- serve web 8765
```

Open `http://127.0.0.1:8765/` to run local browser conformance. The page creates an isolated fixture in OPFS, tests aborted writes, renders through WebGPU, and exercises a dedicated module worker. Its generated bootstrap only loads the WASM module; Rust owns the application behavior. `wasm-bindgen-cli` 0.2.104 is required by the web build.

```sh
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
  cargo test -p render-core --target wasm32-unknown-unknown --locked
cargo build -p render-ffi --locked
python3 scripts/ffi-conformance.py
python3 scripts/dependency-audit.py
python3 scripts/generate-abi-header.py --check
```

The browser automation script connects only to an isolated Chrome CDP session on port 9223. It is external test tooling. Generated images, builds, and transient reports are under ignored `artifacts/`, `target/`, and `web/pkg/`. Selected evidence is copied into the repository after validation.

For automated macOS browser reproduction, keep the Rust server running in another terminal, launch a separate Chrome profile, then run the harness:

```sh
"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" \
  --headless=new --no-first-run --no-default-browser-check \
  --remote-debugging-port=9223 --user-data-dir=/tmp/render-foundation-chrome \
  --enable-unsafe-webgpu --use-angle=metal about:blank
# In a separate terminal:
node scripts/browser-conformance.mjs
```

The WebGPU flag permits headless test adapter use; it is not a production browser requirement. Use the isolated profile, because conformance intentionally changes the test origin's quota and storage. `validate-foundation.py --resume` retries only failed checks from a previous run; use a fresh full run after implementation changes.

## Boundaries

| Crate | Responsibility |
|---|---|
| `render-core` | Typed authored state, identity, geometry, radial editing connectivity, transactions, jobs, uploads, persistence contracts, reference rendering and scoped interchange |
| `render-kernel` | Private Rust expression/statement IR and generated compute/vertex/fragment programs, checked by Naga |
| `render-gpu` | Camera-relative scene packing, shared geometry buffers, software BVH traversal, diffuse transport, progressive accumulation, raster preview and readback |
| `render-host` | CLI, local transport, native durable job worker, artifact output and static development server |
| `render-web` | Browser document API, local conformance application, OPFS adapter and dedicated worker |
| `render-ffi` | Small trusted C-compatible ABI v1; exported table and generation-checked engine/result handles |

The authoring engine exposes snapshots immutably. Transactions are the mutation path. Candidate snapshots and receipts cannot be replaced by callers. Layer visibility uses the last override for each entity; hiding a composed instance does not erase its authored geometry reference. Editing handles include arena ownership and generations. GPU handles never enter persistent authored state.

## Implemented rendering profile

CPU rendering uses f64 intersection/transform math, per-geometry BVHs, an instance-level BVH, indexed random streams, two-sided Lambertian reflection, emission, one point light, constant environment lighting, nearest/repeating textures, and finite path depth. Depth truncation is explicitly biased. Point-light sampling is discrete and does not overlap the continuous BSDF sampler, so this profile does not require a light/BSDF MIS weight. Materials with metallic != 0 or roughness != 1 are rejected explicitly by the Lambertian renderer.

GPU rendering implements the matched one-bounce diffuse profile. Mesh acceleration and triangle data stay shared between instances. Instance bounds and inverse transforms are separate; camera-relative coordinates limit world-origin precision loss. GPU geometry addressing uses exactly representable f32 indices, capped at 16,777,216 packed records. Rendering negotiates baseline WebGPU limits and rejects oversized buffers/dispatches before admission. A real over-limit allocation is also tested through GPU validation error scopes. Physical VRAM exhaustion and spontaneous driver reset are not induced; device destruction/recreation is exercised explicitly.

The raster preview is a separate hardware raster pipeline with depth testing and flat approximate shading. It omits texture sampling, transport shadows and indirect light. It is never presented as an equivalent final render.

Images include scene-linear sRGB PFM, display-sRGB PPM, reproducibility receipts, depth in meters, world-space normals and persistent object IDs. The ray epsilon is currently 1e-5 meters. Extremely small features and extreme condition numbers remain numerical qualification limits. Output admission accounts for declared buffers, not a proof of total driver/allocator residency. CPU and GPU runtime measurements include the stages named in each report.

## Geometry and interoperability profiles

Canonical meshes preserve loose vertices/edges, simple planar n-gons, nonmanifold edge fans, geometry-local identities and typed point/edge/face/corner attributes. UVs are corner-domain data. Positions support f32 and f64; transforms retain an affine base and ordered translation/rotation/scale channels. Singular matrices can be authored; inverse-dependent evaluation rejects them.

Editing connectivity uses generation-checked vertex, edge, face and corner arenas, face cycles and radial cycles. The initial edit operation moves vertices and reports preserved/changed identities. Full modeling operators remain M06 work. Creating an edit view currently materializes the asset; this is not a sculpting locality or five-million-corner capacity claim.

OBJ supports polygon positions, corner UVs, loose lines and negative indices. It reports discarded groups/material references/normals and never fetches external files. glTF supports a named mesh-only static, non-indexed triangle profile with f32 POSITION/TEXCOORD_0 accessors and an explicitly supplied binary buffer. It rejects animation, skins, required extensions, sparse/indexed primitives and unsupported accessor layouts. It reports omitted scene placement, material behavior and non-UV data. Reading this profile does not imply arbitrary glTF scene fidelity. PPM texture import converts declared display-sRGB samples to linear values through Rust code.

## State, storage and concurrency

Canonical JSON v0 uses UTF-8, sorted object keys, ascending entity IDs, ordered authored operations, finite numeric values, explicit 32-digit lowercase hexadecimal IDs, and SHA-256 identities. The pinned serializer emits shortest round-tripping number spellings, retains signed zero, and parses with `float_roundtrip`. This is a project-specific experimental encoding, not an RFC 8785 claim. Private entity chunks/indexes are excluded from the wire representation.

Native packages have numbered immutable root records and content-addressed mesh chunks. Mesh chunks use the `R3DMESH0` signature, a little-endian u32 metadata length, typed JSON attribute/count metadata, and explicit little-endian f32/f64 positions, u32 topology and u64 identities. Counts and payload lengths are checked before reading. Snapshot and journal mesh payloads share chunks. Native publication writes/flushes chunks, writes/flushes a pending root, renames it atomically, and flushes the containing directory. An OS file lock serializes writers. Process-termination tests exercise all root-publication boundaries. Pending roots and unreferenced chunks are never treated as committed state. Corrupt committed content returns an integrity error; it is not silently replaced by an empty scene.

The browser adapter uses checksummed canonical envelopes and OPFS writable-stream close/abort under a Web Lock. Origin-managed durability is distinct from user-controlled backup. The general browser document API exports canonical JSON strings for clients to download or save; the conformance fixture also tests local OPFS export/reload. The native binary package and browser envelope use different host storage protocols while recovering the same document semantics. Unknown format versions are rejected. No stable migration guarantee is made for pre-alpha formats; retain the original package and export canonical JSON before format changes.

Transactions retain resolved commands, receipts, base/result revisions and delta digests. Expensive work runs on immutable candidates before commit. Idempotency keys are scoped to principal/document, retain their accepted result for the document lifetime, and reject changed payloads. Capacity is checked again at commit, including no-op candidates. Durable retries check the destination storage state; an in-memory receipt alone cannot skip publication. This profile admits at most 10,000 retained keys and 128 MiB of canonical authored state. A storage failure after root publication has an uncertain acknowledgment outcome: reopen/recover, then retry with the same key to retrieve the accepted result.

The native job host shares the exact project root used by the CLI transaction API. An OS worker lease rejects concurrent job hosts before recovery; it is released after the worker joins. Legacy job-only packages containing a nested `journal/` require explicitly opening that directory. The native job host returns accepted IDs, persists queued/running/terminal state and resumable bounded events, and renders pinned snapshots outside the state lock. Cancellation is acknowledged separately from completion. Queued cancellation releases admission capacity immediately; repeated cancellation does not append duplicate transition events. Interrupted running jobs become failed with an explicit retry diagnostic on reopen. Queue admission, per-principal limits, wall-time checks, sample/memory/output budgets and artifact receipts are enforced. Files written before a cancelled terminal transition can remain unreferenced artifacts; no automatic destructive garbage collection runs.

## Foundation API and narrow ABI

`render-host api <project>` reads newline-delimited `Request` JSON and returns serialized structured results. A request has `version: 0`, `base_revision`, `idempotency_key`, `commands`, and `max_added_bytes`. `render-core::api::registry()` describes the ten supported query/mutation entries. `render-host jobs <project>` accepts `submit`, `cancel`, `status`, and `events` messages. Both local transports authenticate as a trusted local principal; these commands are not exposed as an unauthenticated remote service.

The earlier bundled material-edit schema remains an illustrative design draft. The implementation's version-0 request contract uses the documented command enum and SHA-256 revision encoding; it does not claim conformance to the draft's proposed BLAKE3/UUID spelling or protocol version 0.1.

Browser bindings expose `BrowserDocument` create/import/export/revision/execute/prepare/inspect_candidate/commit/discard/branch and `render_document`. External JavaScript is a client language; it does not implement application semantics.

`include/render.h` is generated from the Rust ABI table. `render_entry(0, sizeof(RenderApi))` negotiates version/size. Results are engine-owned immutable buffers copied into caller-owned storage and explicitly released. Handles reject stale generations and wrong kinds. Caller pointers must be valid for their lengths; this ABI is trusted, process-local, memory-only and not a sandbox. Calls serialize under the registry lock. No callbacks into clients occur. Unwinds are caught; a poisoned registry is unusable and requires process restart. Abort/fatal memory faults are outside unwind containment. M05 adds `render_entry(1, sizeof(RenderApi))` with v0 retained; the complete [v1 contract](agent_alpha.md#narrow-abi-v1) defines ownership, limits and independent-client evidence.

## Provenance and evidence limits

Project code is independently authored Rust. No Blender/Cycles implementation was ported or linked. Test scripts, comparative hecs usage and generated browser bindings are classified separately from the bundled runtime. Dependency/license expressions and environmental links are recorded by the audit; the project itself has not yet selected a distribution license.

Validated runtime hosts are Apple M1/macOS and Chromium WebGPU. Other native targets need their own runtime evidence. The comparison is an experiment with the named workloads and machines, not a universal speed, memory, topology-robustness or production-readiness claim. M05 now has its own validated agent profile; M07 has the separately validated static PBR slice; other later gates remain open.
