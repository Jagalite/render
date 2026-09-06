# Typed polyline control RGBA contract

Base 0a3ffd1. Apply architecture 8.2/14/15, M07/INPUT-04, ADR030, native curve/groom
and vertex-color contracts. Keep shared Rust computation, transactions, immutable
snapshots, source-preserving native authoring and M09 deferral.

Add optional typed linear straight RGBA f32 to stable curve controls. Missing values
mean white/alpha1 and serialize absent, preserving old asset bytes. Admit colors on
polylines only; reject colored Bezier/B-spline controls until a color-error adaptive
subdivision contract exists. Geometry-only flatness must not imply color fidelity.

Carry RGBA through existing homogeneous polyline/radius subdivision into tube rings
and end caps. Emit the existing point Vec4/color_rgba mesh semantic with stableId200;
reuse CPU/GPU material/alpha, displacement, bake and GLB consumers. Preserve guide
colors through child generation and combined groom geometry. Keep old uncolored
geometry, conversion receipts and four shader artifacts byte-identical.

Snapshot v15 requires explicit colored controls in geometry assets or groom guides;
ordinary PutGeometry/SetGroom transactions upgrade when needed. Old native snapshots
remain readable and unchanged. Invalid colors, unsupported basis, lower schema
versions, failed evaluation, stale transactions and cancellation cannot publish.

Extend HAIR Policy with an optional explicit linear_rgba_f32 point-attribute mode.
Default absent keeps the committed uniform-per-strand profile and its rejection
cases unchanged. Explicit mode selects hair-polyline-rgba-v1, preserves point color
and1-minus-transparency in authored controls and uses white surface materials with
opaque or unit-factor BLEND coverage. Report the f32 alpha conversion error (source
transparency is f32 but subtraction may need rounding); do not silently average or
texture-quantize data. Source/policy-dependent IDs remain deterministic.

Require analytic midpoint/subdivision/ring/cap and white-default tests, numeric color
bounds, schema/permission/stale/cancel/durability checks, groom guide/child transfer,
original CC0 varying-color/coverage HAIR fixtures, native/Wasm and real Metal/Chrome
WebGPU/OPFS workflows, GLB color roundtrip, resource/source/dependency evidence and
all prior artifact identity. Platform-local observed_derived_json_bytes must match
actual evaluator receipts; semantic reports/counts/revisions and qualified CPU pixels
retain exact cross-platform checks. Physical fiber, colored higher-order splines,
analytic curves, animated source grooms and GPU media remain separate next gates.
