# Bounded multi-bounce opaque PBR

This extends M07's opaque material renderer and the ordinary CLI/browser render
operations. `Settings.max_depth` admits 1–16 surface vertices on Rust CPU and the
Rust-generated Metal/WebGPU path kernel. The existing setting and transaction
schema are unchanged. Static and animated glTF adapters retain their named input
profiles; this is a transport extension, not additional glTF format coverage.

At each surface, accumulate emission and visible point-light illumination, then
sample a continuation direction. Opaque PBR uses the existing equal-weight
cosine/GGX mixture with the full mixture PDF and `f * cos / pdf` throughput.
Lambertian surfaces use cosine sampling and albedo throughput. Mixed paths can
visit either material family in any order. Point-light contributions are evaluated
once per vertex; emissive geometry and the constant environment are reached only
by BSDF sampling. There is no separate continuous light sampler to double-count,
and no MIS claim. Small emissive sources may have high variance.

Depth counts shaded surfaces, including the camera-visible surface. After the last
surface, an escaping continuation may contribute the environment; another surface
is truncated, including its emission. Depth one therefore retains the previous
CPU results. Finite-depth truncation is biased and explicitly named. There is no
Russian roulette, firefly clamp, or energy compensation. Single-scattering GGX
retains the existing perceptual roughness floor of 0.05.

Camera near/far clipping applies only to primary rays. Depth, normal and object
passes come only from the primary hit at the first sample, even when later surfaces
are visible through reflection. Primary textures retain camera-ray differential
mip selection; secondary textures use zero UV derivatives (base-level magnification
filtering). Secondary footprint propagation is future work. Normal maps retain the
existing geometric-hemisphere visibility policy; this is not a new shading-normal
energy correction. Authored occlusion multiplies outgoing indirect throughput,
including later surfaces, but not the current emission or point-light term.

Counter dimensions 0/1 jitter the camera. In PBR-containing scenes each bounce b
uses 2+3b, 3+3b and 4+3b; all-diffuse scenes retain their old two-dimension stride.
Seeds and sample offsets are scheduling-independent. Rust owns both mathematical
implementations; WGSL is generated from `crates/kernel/src/path*.rs` and validated
by Naga. No maintained shader language or runtime dependency is added.

Depth >1 receipts identify `cpu-f64-pbr-path-v1` or `gpu-f32-{pbr,diffuse}-path-v1`.
GPU progressive receipts retain material and depth qualifications, accumulated
sample range, and the complete scene/settings identity. Changing depth resets
accumulation. CPU uses f64 geometry/BRDF and f32 accumulation; GPU uses
camera-relative f32 and RGBA16 stored texture texels. Pixel identity across those
precision policies is not promised.

Settings retain their existing limits (depth ≤16, samples ≤65,536 and admitted
image bytes). The bounded agent preview now admits depth 1–16 while retaining
128² pixels, 64 samples, 192 entities and 8 MiB snapshots. More depth increases
work without increasing per-path storage. CPU polls cancellation at each path
vertex; native/browser GPU cancellation is acknowledged at the existing submission
and completion boundaries, not by preempting a running GPU dispatch. CLI wall and
output budgets, revisions, permissions, idempotency and artifact publication retain
the ordinary adapters. GPU advanced scattering, sparse media and shutter sequence
accumulation remain separate unsupported profiles.

## Validation and reproduction

`fixtures/multibounce-pbr/create.json` authors a plate cavity through ordinary
material, entity and box commands. A camera between the plates sees an unlit floor;
an emissive ceiling illuminates it only indirectly. Depth 1 must be black, depth 2
must be lit, and depths 4 and 16 add further reflected light. Its generator writes
a new directory and does not rewrite fixtures or reference images.

`crates/core/tests/multibounce.rs` independently checks the rough white-metal
hemispherical integral `1-ln(2)`, finite diffuse geometric-series transport, mixed
material paths, single environment counting, pass identity, deterministic sampling,
invalid depths/budgets, stale revisions and cancellation. Existing BRDF quadrature,
energy, reciprocity, texture and animation tests remain part of the full suite.

The explicit GPU test gate checks depths 1/2/4/16, secondary texture sampling,
progressive sample offsets/reset/receipts and cancellation after actual queue
submission. Native/browser workflows exercise durable/OPFS recovery, immutability,
stale/cancelled operations and CPU/Metal/WebGPU parity at an RGB RMSE threshold of
0.002 with exact object identity for this fixture. Linux/Windows are compile checks.

Build the host and web bindings, start the local server and isolated browser as in
[the agent guide](agent_alpha.md) on ports 8766/9224, then run
`python3 scripts/validate-multibounce.py`. The runner writes fresh
`artifacts/multibounce-pbr/run-*` evidence, including commands, failures, process
RSS/footprint and source hashes. The GPU test is intentionally opt-in for ordinary
CPU test runs and mandatory in this acceptance runner. Existing static PBR,
animated glTF and general CLI workflows are also exercised.

The transport recurrence follows the mathematical approach described in
[PBRT's path tracer](https://pbr-book.org/4ed/Light_Transport_I_Surface_Reflection/A_Better_Path_Tracer).
Its C++ implementation is a reference, not a dependency. Existing Khronos texture
fixtures used in GPU tests retain their recorded licenses and source hashes.
