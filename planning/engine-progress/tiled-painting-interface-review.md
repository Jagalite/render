# Tiled painting interface proposal

This candidate now begins at qualified UV commit be20d1f. The protected baseline
executable matches that increment's tested-package hash. Architecture
17.3 and M10 apply; the first engine profile does not complete M10 or M09.

Keep a paint canvas as an independently editable authored asset. A stable canvas ID
binds a content hash; layers have stable IDs and explicit compositing order. Each
immutable paint asset retains pixel extent, a fixed tile size, color role, typed
layers and sparse tile references. A tile has finite premultiplied linear RGBA and
an explicit coordinate; absent tiles are transparent. Public tile references use
typed coordinates and hashes, not private buffer offsets or stringly typed state.

Brush preparation pins the canvas's previous hash, document revision and target
layer, computes touched tiles with cancellation/work/output checks, then publishes
ordinary tile/asset/binding commands. Only touched tile payloads are copied; all
other references remain identical. Transparent edge padding is canonical and cannot
hide out-of-canvas pixel data. Stable stroke IDs and typed samples/brush parameters
record the recipe alongside accepted changed-tile identities. Source-over paint and
explicit erase modes need analytic color/alpha tests; masks and layers are part of
the first useful gate, not metadata placeholders.

Use explicit pixel-space samples with pressure and monotone time. Define radius,
spacing/interpolation, clipping, coverage and blend rounding precisely. Do not expose
unimplemented tilt or projection fields. Geometry/camera-projected painting is a
separate named extension that pins both source revisions and uses native queries.

A bounded composite/bake produces a normal ImageAsset through the existing pure
Rust PNG path, with explicit straight/premultiplied alpha conversion, color encoding
and quantization. A regular material command attaches the result to the intended
UV attribute. Publishing a shared material change must be an explicit caller choice;
painting a canvas must not silently retarget other instances. The editable source
and its rendered image are distinct, with a reported source hash for stale bakes.

Respect the current 2048-pixel image dimensions and 4-MiB encoded output cap in the
initial bake adapter. Bigger virtual textures, UDIM and renderer paging need a new
profile. Check transfer-function determinism near quantization boundaries on native
and Wasm before claiming identical PNG bytes. Existing image filtering semantics
remain their own qualified profile; do not fold an unrelated renderer rewrite into
painting.

Recovery/undo switches to a retained prior paint asset through ordinary compare-and-
replace commands. Long strokes can be split at explicit checkpoints while retaining
stroke/sample continuity. Test quotas, failed publication, stale source/revision,
permission, mid/late cancellation, mask behavior, layer order, sparse tile identity,
unaltered distant pixels and recovery. Measure touched tile bytes and process costs;
source inspection is insufficient for locality or speed claims.

Further review points before coding:

- Tile content hashes should omit placement coordinates so identical pixels can be
  shared; typed layer references supply coordinates. A partial boundary tile must
  match the canonical out-of-canvas default, rather than carrying hidden data.
- Linear premultiplied RGBA is the proposed editable color representation. Baking
  explicitly converts to the existing image asset's straight-alpha encoding and
  sRGB/linear-data role. Inputs remain finite and bounded; HDR painting is separate.
- Masks need an explicit scalar coverage default (normally one), unlike transparent
  missing color tiles. A real mask-edit operation and its effect on stroke coverage
  or layer compositing must be tested; decide and document that semantic boundary.
- Existing `ImageAsset` caps are 2048 pixels and 4 MiB encoded bytes. Keep a dense
  bake as an explicit measured operation; do not call it a sparse renderer update.
- Near-half quantization values can expose native/Wasm transfer-function differences.
  Inspect current color conversion and the already pinned pure Rust libm before
  deciding a deterministic bake conversion; any expanded use needs provenance.
- Normal/source-over layer blending and explicit erase form the initial bounded
  profile. Additional blend modes, tilt, projected painting and UDIM are extensions,
  with strict rejection rather than accepted inert fields.

## Reviewed transaction integration

Use snapshot 18 with omitted-when-empty `paint_tiles`, `paint_assets` and independent
`paint_canvases` ID bindings. Ordinary `put_paint_tile`, `put_paint_asset` and
`set_paint_canvas` commands retain immutable sources and compare the old canvas hash
before replacement/removal. Validate tile kind, hash, boundary defaults, caps and
all retained recipe/layer references. No entity or shared-material mutation occurs
when a canvas changes. Existing snapshot bytes remain unchanged when painting is
absent. An actual prior client must reject painting checkpoints explicitly.

`author_paint` prepares a stroke against a retained source asset and publishes the
three ordinary command kinds, with permission/revision/source checks and normal
idempotency. Generated tile commands must depend only on the pinned source and
request; do not omit one because its bytes happen to be retained after the first
successful attempt. This keeps retry payloads stable. `bake_paint` produces a normal
image through the same transaction mechanism. Its explicit source hash/report allow
a separate ordinary material binding transaction to select a named UV attribute;
there is no implicit material replacement. The integration reuses existing
rendering/evaluation; the numeric compatibility decisions below apply at its
image-sampling boundary.

The agent entrypoints run candidate profile/evaluation and late cancellation before
commit, and durable dispatch uses the existing journal adapter. The native CLI uses
its existing cancellable store. No ABI expansion or Cargo dependency change is
required. Expanded pure Rust libm use covers brush distance and bake transfer math;
record it in numerical provenance and test native/Wasm exact data before acceptance.

Bake retry review: ordinary commands must include a compare-and-replace assertion
that keeps the selected canvas/source binding unchanged, before PutImage. Two
different authored canvases can produce identical image bytes; image-only commands
would otherwise alias their retry identities. A regression covers equal pixels
with distinct canvas/source identities and missing-canvas retry attempts.

## 8-bit sRGB evaluation review

The new varied painted palette exposed 43 native/Wasm CPU channel differences,
maximum 5.960464477539063e-8, while full snapshots, PNG bytes, authoring reports,
revisions and depth agreed. Do not relax the exact CPU gate. Probe all 256 input
codes in the unchanged pre-paint imaging transfer function and compare actual native
and Wasm results before selecting a fix.

Candidate remedy if confirmed: a portable fixed 8-bit sRGB-to-linear f32 table that
preserves all 256 qualified macOS native values. Keep the native numeric baseline,
generating inputs/script and measured error against a higher-precision analytic
transfer. Use it only at the existing decoded 8-bit image/Pyramid boundary. Linear
data, general floating transfer functions, PNG bytes, public image semantics and
shaders stay unchanged. Prove every earlier native render/pass/receipt remains byte
identical, and rerun the exact painted CPU comparison on both platforms. This is an
integration review proposal until the probes and full acceptance pass.

The sRGB table reduced the actual CPU discrepancy to 13 channels. A second probe
verified every mip image hash, UV and derivative and all 1,041 footprints matched.
Only platform f32 log2 differed (144 hits); shared Rust f64 log2 rounded to f32
matched every observed native LOD. Use that common path, retain the observed native
numeric fixture and Decimal64 check, and test the public sampler on analytic
constant-color mip levels. No filter, reference image or GPU shader is replaced.

Integration acceptance passed on be20d1f: native/Wasm tile and PNG bytes, revisions,
reports and CPU pixels agree. Earlier fixture/render/pass/receipt artifacts, Cargo
and shader outputs remain unchanged. Failure evidence retains a corrected analytic
rounding expectation, lint failure and interrupted run for a reproduced duplicate
endpoint dab, with a regression test for the corrected behavior. The browser replay harness also
retains a corrected mutable-request capture failure; unchanged native requests were
rerun, and only hash-verified compiled checks were reused for that continuation.
A subsequent fresh full run validates the source-bound bake retry fix and portable
sRGB8 conversion and common mip LOD logarithm, preserving native outputs without
relaxing exact CPU equality.
