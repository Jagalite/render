# Static PBR rendering slice — implementation contract

This is the first materials slice of M07, paired with INPUT-01 scene ingestion.
This historical slice is now extended by the completed [M07/M08 named profiles](m07_m08.md); its original evidence and importer restrictions remain unchanged.
Opaque transport is additionally extended by [multi-bounce PBR](multibounce_pbr.md); the depth-one limits below describe the original slice.
The M05 checkpoint is `de07c70`; candidate work is on `feat/static-pbr-m07`.

The public `gltf2-static-pbr-v0` profile adds opaque metallic/roughness materials,
PNG/JPEG resources, explicit color/data texture roles, samplers, shading normals,
tangents, and authored cameras. Existing Lambertian imports and rendering keep
separate semantics. Required extensions, animation, skins, morphs, transmission
and alpha coverage are outside this slice and must fail explicitly.

The BRDF follows glTF Appendix B: Schlick Fresnel, GGX distribution, separable
Smith masking, dielectric F0=0.04 and metallic base-color F0. Single-scattering
GGX is regularized to perceptual roughness >=0.05; this approximation is named
in receipts. Native/browser GPU and CPU share the same one-bounce profile.
Environment transport uses a diffuse/GGX mixture with its full mixture PDF;
point lights are evaluated directly with visibility. Energy/PDF/reciprocity and
independent numeric quadrature tests supplement real asset comparisons.

Texture assets retain encoded PNG/JPEG bytes by content identity. The Rust image
decoder is built with only PNG/JPEG features; no codec FFI or default image
format bundle is allowed. Images are decoded under dimension/pixel/allocation
budgets. Color textures are converted from sRGB before filtering; normal, ORM
and other data textures stay linear. Mip pyramids, wrap modes and filtering are
shared CPU/GPU semantics. UV footprint comes from primary camera-ray differentials.

Scene persistence uses a new snapshot version for typed images, camera components
and PBR bindings. Older snapshots remain readable; older engines reject the new
version. Commands publish these fields transactionally. No private Rust layout
is exposed as a stable schema. Branch protection includes texture and camera
resources; the M05 edit allowlist is unchanged.

Cameras are attached to authored entities and evaluated through their hierarchy.
Perspective/orthographic projection and clipping are explicit. The client chooses
an imported camera or supplies its own; importing does not silently replace the
current render settings. Normals use inverse-transpose transforms. Authored tangent
signs preserve mirrored UVs and reflected instances. If tangents are absent, a
reported per-triangle UV-derivative basis is used; MikkTSpace seam smoothing is
not claimed.

Acceptance requires licensed external textured assets, import/save/reopen/render
on CPU/Metal/browser WebGPU, useful structured passes, invalid/truncated input,
resource limits, cancellation, stale revisions, hash-pinned provenance, a runtime
dependency audit and full M05 regressions. Reference images remain untouched.

Primary specification: https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html
The complete bounded workflow and its numerical/resource evidence are recorded
in [static PBR evidence](../evidence/static-pbr/README.md).

## Reproduction and public interface

Run `cargo test --workspace --locked --offline`, then build the host and web
bindings (`cargo build -p render-host --locked --offline` and
`sh scripts/build-web.sh`). Start `target/debug/render-host serve web 8765` and
an isolated Chromium with CDP on port 9223 and WebGPU enabled, as in
[the M05 guide](agent_alpha.md). Run `python3 scripts/validate-static-pbr.py`.
This includes all M05 regression gates and writes fresh `artifacts/static-pbr/`
evidence. `render-host pbr-workflow <fresh-output-directory>` runs the native
fixture workflow independently. Python/Node orchestrate conformance only.

The additive operation-v0 methods are `agent.import_pbr_glb` (bytes),
`agent.import_pbr_scene` (JSON, indexed buffers and indexed image byte arrays),
and `agent.select_camera` (authored entity ID, base revision and idempotency key).
Imports require `{ "allow_approximations": true }` in `policy`, plus explicit
render `settings`, `base_revision` and `idempotency_key`. External URI strings
are labels; the caller supplies every resource and the engine never fetches them.
For a mixed embedded/external image table, the external array uses source image
indices, with empty entries for embedded images.

The transaction registry adds `put_image`, `set_camera` and `use_camera`.
Camera selection resolves the authored lens through the ordinary transaction
engine, so retries remain stable after later root edits. Camera transforms must
be rigid; scaled, sheared or reflected cameras return `unsupported_camera`.
Rendering uses world-space ray-distance depth; perspective clipping distances
are measured along the forward axis, as specified by the authored lens.
`use_camera` copies the evaluated view into render settings; subsequent edits to
the camera entity require another explicit selection to update that view.

Limits: 4 MiB combined source resources; 64 source nodes/meshes/materials and
primitives; 20,000 total points plus corners; 16 images, each at most 2048×2048,
12×1024² aggregate decoded pixels and 64 MiB decoder allocation per image.
The existing agent limit remains 8 MiB authored snapshot and 16 MiB control JSON,
128×128, 64 samples and depth one. GPU sampled mip texels are capped at 2²⁴,
with one RGBA16 buffer of at most 128 MiB. Each image/color-role pyramid counts
separately against this GPU limit. The job byte limit covers packed GPU buffers
and readback, not total process RSS or temporary CPU allocations.

Decoded color and data samples and CPU mipmaps use f32; GPU storage rounds mip
texels to binary16, unpacks them to f32, and applies the same sampler equations.
This precision difference is recorded in GPU receipts. It needs no f16 shader
feature and retains the portable WebGPU baseline. Both runtimes decode PNG/JPEG
using Rust `image` with defaults disabled; `half` performs Rust-owned conversion.
`image`'s PNG/JPEG graph (including zune-jpeg, png, flate2/zlib-rs, miniz_oxide and
moxcms) is audited separately from operating-system graphics services.

All three wraps, all six minification modes and perspective/orthographic lenses
have CPU/Metal comparisons using UVs outside [0,1]. Unit tests independently
check color transfer, mip values, magnification, JPEG persistence, GGX normal
incidence, reciprocal BRDF values, hemispherical quadrature, mixture sampling,
inverse-transpose normals, reflected tangent frames, culling, cancellation and
stale operations. Synchronous imports are atomic and expose no mid-decode cancel
handle; render cancellation occurs at the existing CPU row/GPU submission gates.
The approximate flat raster preview explicitly rejects this PBR/lens profile.

## Fixture attribution

Original, hash-pinned Khronos glTF Sample Assets are retained unmodified:
[BoxTextured](../fixtures/static-pbr/BoxTextured/UPSTREAM_README.md), ©2017 Cesium,
CC BY 4.0, with the ©2015 Cesium logo's
[trademark notice](../fixtures/static-pbr/BoxTextured/LicenseRef-LegalMark-Cesium.txt);
and [NormalTangentMirrorTest](../fixtures/static-pbr/NormalTangentMirrorTest/UPSTREAM_README.md),
©2018 Analytical Graphics, Inc., Ed Mackey, CC BY 4.0. Each folder includes
source URLs, byte lengths and SHA-256 hashes. The procedural sampler fixture
changes disposable evaluated UVs; original source bytes and reference images
are unchanged. Tiny codec and analytic normal fixtures are authored in Rust tests.

Back-face tangent frames reverse all three basis vectors before applying the
normal map, consistent with the Khronos
[sample renderer's normal evaluation](https://github.com/KhronosGroup/glTF-Sample-Renderer/blob/main/source/Renderer/shaders/material_info.glsl).
This is covered by an independent analytic reflected-instance test and matched
rear-view CPU/Metal renders of the mirrored fixture. The Khronos renderer is a
development reference only; no GLSL or non-Rust renderer is shipped or invoked.
