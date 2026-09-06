# GPU alpha coverage and shared Principled transport increment

Base: 07720c5. Preserve fully Rust/native/browser semantics, immutable snapshots,
ordinary transaction/CLI/browser paths, shader generation and existing opaque
profiles. M09 remains deferred. Read docs/gltf_alpha.md, multibounce_pbr.md,
vertex_colors.md, gpu_shutter.md and ADR019/023/024 before implementation.

First reproduce and correct the identified extended-CPU Principled indirect
occlusion omission with analytic zero/full occlusion tests. Keep direct illumination
and emission independent. Existing opaque CPU transport already applies occlusion;
the extended path should preserve the same material meaning.

GPU target: Model::Principled OPAQUE/MASK/BLEND only; other advanced BSDFs and sparse
media stay explicitly unsupported. Preserve base factor, selected UV and vertex
alpha. MASK is inclusive; BLEND stochastic path continuation and multiplicative
shadow/end-of-depth visibility. Primary continuation retains the original camera
interval and per-pixel origin for depth. Transparent crossings do not consume BSDF
bounce depth. Align the extended CPU random dimension schedule for mixed legacy/PBR
scenes. Occlusion affects indirect throughput consistently.

Proposed private packing: use the existing unused instance row15 for opacity mode,
factor/cutoff; retain zero bytes for old materials. Global param9.w indicates the
extended profile; param9.z selects its existing CPU stride16 (legacy defaults stay
2/3). Alpha sampling should preserve base-level f32 alpha, separate from the
existing RGBA16 RGB texture approximation, to avoid large coverage changes at
constant texture thresholds. Audit exact threshold cases before freezing policy.
No source/material/document schema changes are expected.

Proposed shader continuation: existing closest-hit trace plus a bounded covered-hit
wrapper and a multiplicative visibility function. Core path allows 64 discarded
surfaces; CPU shadow traversal has 64 iterations. Preserve those bounded failures
rather than returning fabricated pixels. A reserved negative depth marker can carry
per-pixel continuation failure through GPU temporal accumulation, then become a
structured budget error at full readback. It must survive nominal-pass preservation
and later shutter dispatches. Opaque outputs and packed bytes must remain unchanged.
Choose implementation after inspecting kernel/renderer seams; this is a plan, not
an accepted functionality claim.

Acceptance: original licensed/CC0 alpha, vertex-color and named-UV fixtures; analytic
MASK threshold/factor/texture/vertex cases, BLEND coverage statistics and shadow
products, camera clipping/depth, zero/full AO, dense transparency limits, mixed
materials, stale/cancel/budget and shutter/sequence failure propagation. Native
CPU/Metal, Rust Wasm and actual Chrome WebGPU/OPFS workflows; positive GLB exports
of alpha scenes now render on GPU. Retain old explicit unsupported-profile tests
for genuinely unsupported BSDFs/media and amend alpha-only capability expectations.
Resource measurements, generated-source/provenance hashes, dependencies, complete
regression comparisons and honest profile documentation precede status updates.

Accepted decisions and evidence: see docs/gpu_alpha.md and evidence/gpu-alpha.
MASK uses a <=30-comparison search over positive f32 bit patterns against the CPU
multiplication predicate, rather than cutoff division. Alpha has its own lazily
compiled kernel variant; the default opaque WGSL is byte-identical to07720c5.
The private orthographic near parameter now records the existing effective1e-5m
minimum; below-epsilon raw packed parameters change, while clipping semantics and
all observed opaque outputs remain unchanged. No public layout depends on this row.
Final acceptance passes145 native/124 Wasm tests, real Metal/Chrome/OPFS workflows,
243 unchanged image/pass files and81 unchanged receipts. Secondary extended-CPU
texture footprints remain explicitly unqualified and are the next correction.
