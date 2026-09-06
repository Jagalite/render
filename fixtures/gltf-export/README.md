# Evaluated PBR export workflow recipe

The cases reuse original CC0 source fixtures without modifying them. Their geometry,
images, animation and semantic oracles retain the existing generators and provenance
in named-uv, vertex-colors, morph-frames and alpha-gltf. `cases.json` is the shared
native/browser workflow recipe. The morph exports the exact half-second pose; other
cases export the default scene. An explicit render recipe is reapplied after import,
because this GLB profile does not export cameras, lights or engine settings.

Acceptance exports each case through the shared Rust API, validates every emitted
GLB with the independent Khronos tool, reimports and persists it, and compares its
CPU/GPU pixels and semantic passes with the original. Native and browser GLBs must
be byte-identical. Source-scoped imported entity IDs are mapped through export node
indices and the import report; indices are never used as durable authoring IDs.

Core analytic tests separately cover local smooth frames under nonuniform/reflected
transforms, mesh sharing, displacement, matrix/position quantization, collapsed
triangles, shear, unsupported BSDFs, precision/budget/cancel/stale failures and
immutable snapshots. No renderer output is used to generate a source fixture.
