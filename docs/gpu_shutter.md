# GPU shutter frames and streaming sequences

`gpu-f32-animation-v0` extends the existing exact-time M08 frame contract to native
Metal and browser WebGPU. Rust evaluates each rigid, LBS and POSITION-morph pose;
the Rust-generated path kernel integrates its supported opaque diffuse/PBR color.
Authored snapshots, animation schemas, sample indexing and transaction authority
retain their existing semantics. This profile adds no input formats or runtime
dependencies. Extended surfaces and sparse media still reject on GPU.

A frame pins the authored revision, clip, nominal rational time and relative
shutter interval. Midpoint quadrature admits 1–32 temporal samples. A sequence
admits 1–1024 strictly increasing nominal times, and validates all shutter-time
arithmetic before emitting any frame. Invalid dimensions reject before working
storage arithmetic. The authored render settings supply spatial samples, camera,
light, depth and memory admission. Depth/normal/object passes use the primary hit
at nominal time; only linear RGB is shutter averaged. A single shifted shutter
sample evaluates its shifted time for color while retaining nominal passes.

Instant exposure uses the ordinary GPU render. Otherwise a nominal dispatch
initializes one persistent 32-byte-per-pixel device output buffer, with zero color
weight and nominal diagnostic passes. Ordered temporal dispatches each add 1/N
of their color and preserve those passes. No atomic reduction is required: each
invocation owns one pixel. Intermediate 4-byte readbacks fence submission and
expose execution errors and cancellation; only the last dispatch reads the full
image. Object indices are decoded against the nominal scene. The private packing
row controlling accumulation is not a serialized Rust layout or public ABI.

CPU temporal averaging retains f64 sums of f32 images; GPU averages in f32.
Both retain the same request-derived frame identity and evaluation receipt.
Backend and approximation fields distinguish numerical policies. Pixel identity
between precision policies is not promised. GPU instant-frame pixels equal a
static GPU render at the same evaluated pose and settings.

Shutter evaluation retains the nominal scene and current temporal scene, with a
fresh disposable evaluator per temporal sample. This also bounds the CPU path's
formerly accumulating deformation cache. Rebuilding derived geometry can cost
more than retaining every sample; there is no speedup claim. Spatial storage
admission remains 128 bytes per pixel, and actual GPU buffer totals and negotiated
device limits are checked separately. These are rendering resource limits, not a
whole-process allocator cap. Decoded textures, scene evaluation and driver memory
remain subject to their existing profiles and recorded process measurements.

Cancellation checks occur before evaluation, between temporal dispatches, after
GPU completion and before frame publication. A running dispatch is not preempted;
an unfinished frame is discarded. A cancelled frame cannot seed later color.
Native sequence consumers receive only completed frames and receipts. Consumer
failure stops delivery. Earlier accepted frames remain valid partial output.

## CLI and browser interfaces

CLI `frame` and `sequence` accept optional `backend: "cpu" | "gpu"`; omission
preserves the CPU default. They retain their typed `request` and fresh `output`
directory fields. The native adapter monitors its existing deadline/cancellation
file and uses the ordinary atomic artifact bundles plus terminal status manifest.
A failed or cancelled sequence never claims completeness. GPU device creation is
still a non-interruptible driver boundary.

`BrowserAgent.preview_frame_gpu(branch, request_json)` returns a Promise of a
JSON string with receipt, evaluation, temporal times, inspection and passes,
matching the CPU `preview_at` result fields. Admission registers cancellation
synchronously, so immediate `cancel_preview()` is not lost. The branch revision
is checked again after GPU completion. Animation previews do not qualify a static
variant for commit.

`BrowserAgent.preview_sequence_gpu(branch, request_json, emit)` returns a Promise
of a terminal JSON summary. `emit(index, frame_json)` receives one completed frame
and may return a Promise; Rust awaits it before rendering the next frame. This
provides backpressure without retaining the whole sequence. Clients own artifact
publication and any data they retain. The success summary includes `complete`,
authored revision, frame count and receipts. Rejection is a JSON string naming
`partial`, delivered callback attempts, acknowledged callbacks, and the error.
A callback can publish then reject; delivery is therefore distinguished from
acknowledgement, and no claim is made about the consumer's filesystem state.

Cancellation and branch revision are checked before and after each callback. A
pending consumer Promise is not forcibly interrupted; cancellation is acknowledged
at the next boundary. One preview or sequence per agent may run concurrently.
Requests are capped at 1 MiB; existing agent scene/pixel/sample budgets remain.
No remote service or browser filesystem capability is required to render. OPFS
save/reload uses the ordinary agent methods and locks.

## Acceptance contract

The existing original `fixtures/project-cli` animated box and
`fixtures/animated-gltf/character.glb` provide rigid and two-joint morph inputs;
their provenance is unchanged. `feature_fixtures::character_document` also covers
native-authored constraints, LBS and morph geometry. No golden images are rewritten.

`crates/host/tests/gpu_sequence.rs` compares device accumulation to independently
averaged static GPU renders, checks CPU parity, nominal passes, random access,
sequence/frame identity, cancellation after a real temporal submission, consumer
failure, stale revisions, order and memory/sample bounds. Shared native/Wasm tests
exercise overflowing later times before publication and malformed dimensions.
The existing multi-bounce GPU and full core suites remain regression gates.

`python3 scripts/validate-gpu-shutter.py` creates a fresh artifact directory and
records exact commands, failures, source hashes, dependency inventory and resource
measurements. It requires the local browser/server described in the agent guide.
The independent CLI workflow imports, renders, streams, exports/restores, checks
budgets and cancels after a published frame. The browser client exercises actual
WebGPU, consumer backpressure/failure, immediate cancellation, stale completion,
OPFS recovery and cross-platform numeric/pass comparisons. Linux and Windows are
compile-only lanes until their own runtime evidence exists.
