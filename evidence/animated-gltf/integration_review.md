# Animated importer integration review

Scope: the glTF adapter, typed animation/deformation schema, ordinary document
mutation, render evaluation, and independent native/browser clients. M09 remains
outside this work. Previous milestone and CLI evidence is preserved as historical
validation of its recorded sources.

## Authority and schema

The adapter emits PutMesh/PutMaterial/CreateEntity and MergeAnimation commands.
It cannot publish a snapshot. Existing transaction preparation validates the whole
candidate, checks permissions and revisions, and supplies idempotency and durable
publication. MergeAnimation rejects component collisions before extending state;
a failed transaction cannot partially replace an existing rig or clip.

Absolute TRS is an explicit property family, with complete-path and mixed-mode
checks. Existing delta clips are unchanged. Skin-specific matrices augment the
native binding model; strict rig rest/inverse-bind validation remains enforced.
Default weights are typed morph data. Snapshot v11 gates new semantics, and omitted
empty fields preserve prior snapshot hashes. The narrow ABI layout is unchanged.

Imported scene nodes and joints are separate native components. Source transform
channels are duplicated to each applicable target. This retains imported motion,
but later joint edits must use native rig/animation operations; an entity transform
edit alone does not also rewrite a copied joint rest transform. This boundary is
published rather than implying a linked editor control system.

## Input, bounds and numerical semantics

All resources are explicit byte buffers or host-selected files. No metadata text,
URI, extension payload or generator field can execute instructions or select an
unrequested resource. Source limits bound parsing and expansion. Accessors validate
component shape, stride, alignment, count and extent before reading. Skin palettes
validate hierarchy roots, joint indices, matrix shape, normalization and coverage.
Morph counts, key order, value range and expanded track/key budgets are checked.

Analytic tests compare final world positions against a separate closed form and
exercise out-of-order sampling, default pose, STEP/LINEAR/CUBICSPLINE, negative and
nonuniform scale, quaternion cubic interpolation, and scalar-packed morph vectors.
The unchanged external Khronos sample exercises interleaved unsigned-short joints
and float weights. Its rounded quaternions require the declared, measured opt-in
normalization correction; no fixture values or test tolerances were rewritten.

Deformation keeps authored topology and rest assets immutable, creates disposable
meshes, applies morphs before LBS, and recomputes geometric normals. Supplied skin
inverse-bind values are retained rather than silently replaced by rest inverses.
No-clip v11 rendering applies the authored default pose; the documented migration
also applies that behavior to older components in an upgraded document.

## Cancellation, recovery and artifacts

Import is a bounded synchronous conversion. Native cancellation checks occur at
request/read and post-preparation publication boundaries. Admitted journal commits
finish atomically. The animated CLI test checks a cancelled import leaves the
existing authored project unchanged, then proves retry identity and animation merge.
Existing fault-injection tests cover in-commit cancellation and crash boundaries.

Animation sampling and deformation poll cancellation; existing sequence tests cover
partial completed-frame publication. The independent client checks stale revisions,
output budgets, save/reopen equivalence and exact repeated-frame artifacts. Browser
validation exercises independent Rust import, OPFS recovery, immutable random-time
preview, stale revisions and GPU cancellation.

## Portability and provenance

No Cargo dependency or maintained shader change is required. Rust owns all import,
animation and skin/morph computation. Existing generated bindings/shaders are built
from the current Rust inputs and tracked in the dependency evidence. Native CPU and
Metal, Rust Wasm and Chromium WebGPU have distinct validation entries; Linux and
Windows remain compile-only. GPU shutter accumulation, sparse accessors, normal/
tangent morphs, extensions and animated glTF export remain unsupported.

Fixtures have source URLs, licenses and hashes. The original synthetic generator
writes only new directories and reproduces the checked-in bytes. External files
are unchanged. Numerical comparisons do not regenerate image goldens or claim
performance beyond the recorded small fixtures.
