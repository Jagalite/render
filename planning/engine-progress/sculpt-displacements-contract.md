# Sparse displacement and refit candidate

Baseline 7bd2e25. Architecture sections 6.3, 7, 9, 11 and 17.4, M10, and ADRs
039/040 apply. This candidate must supply a complete ordinary-transaction native
and browser workflow, not only a new cache or schema. Full M10 remains open.

## Canonical authoring boundary

Introduce Snapshot19 typed sculpt assets, displacement chunks and entity bindings.
A sculpt asset references an immutable triangle-only mesh and a sparse map of typed
64-ID block coordinates to content-addressed chunks. Entries identify stable point
IDs through their block and bounded slot, and store local-meter f32 displacement
relative to the immutable base. New block IDs use canonical decimal strings (zero
is valid for the first block); runtime triangle/node indices never enter storage.
All base data and unaffected chunks remain shared. Zero displacement removes the
sparse entry. No dense f64 displaced mesh is materialized.

This initial profile accepts triangle-only polygon meshes with at least one face, including seams,
boundaries, disconnected/nonmanifold faces and loose points. Existing modeling
triangulation is the explicit preparation path for n-gons; there is no implicit
identity-losing conversion. Fixed base topology makes every face's three corner IDs
its persistent triangulation. Reject authored normals/tangents and conflicting
procedural, skin, morph, shading-frame, groom or material-displacement consumers.
Preserve UV/color corner attributes; normals use the displaced flat triangle.
Reference and compatibility checks apply to layer overrides as well as base material.

All writes use typed PutChunk/PutAsset/compare-replace binding commands. The
high-level point-displacement operation pins document revision and source asset,
checks permission, duplicate/unknown IDs and numeric/resource limits, then prepares
an ordinary idempotent transaction. Old asset/chunk references support checkpoint
recovery and undo. Old clients must explicitly reject Snapshot19. Do not change old
Snapshot0..18 canonical encodings, revisions or rendered receipts.

## Evaluation and ownership

The existing Evaluator owns disposable sculpt evaluation. Build stable point-to-
triangle, triangle-to-leaf and node-to-parent correspondence once for the immutable
base. Current triangles and BVH nodes use the qualified immutable chunks. Refit only
affected leaf bounds and unique ancestors; children, leaf items and traversal order
never change. Fresh reconstruction uses the SAME immutable base partition plus
current displacement, not a new partition built from displaced positions. This is
required for deterministic equal-distance candidates and fresh/incremental parity.

Keep basis and current state private. Do not accept an arbitrary caller-mutated BVH
as refit lineage. Bound cache retention and preserve its previous complete entry on
failure/cancellation. No derived mutable cache lives inside an authored snapshot.
One-shot CLI rendering may pay cold construction; the actual long-lived core/browser
workflow must demonstrate warm reuse. Existing per-call rendering must not silently
grow an unbounded session-wide cache for unrelated geometry families.

Report dirty point/triangle/leaf/node work, all root-table copies, nested payload
copies, source/basis construction and retained cache costs. Snapshot admission and
canonical hashing remain global and must be measured separately. Instance bounds
and GPU host packing must not be mislabeled local; either implement a conservatively
bounded local path for the new profile or report remaining global work explicitly.

## Required evidence

Analytic displaced planes/triangles, normals, UV seams, high IDs, sparse/no-op/wide
updates, degenerate/non-finite rejection and exact fresh-versus-refit candidates.
Exercise every cancellation checkpoint, stale selection/revision, permissions,
retries, cache eviction, old versions remaining immutable and storage recovery.
Use ordinary operations to author, render CPU/Metal/WebGPU, export/reopen and undo.
Native/Wasm authored identities and portable costs must agree; GPU errors use the
existing declared tolerances. All prior fixtures, image/pass/receipt artifacts,
shaders and dependency features stay exact. Record costs and failed experiments.

Masks, symmetry, brushes, multiresolution, dynamic topology/remesh and retopology
editing remain separate subsequent work. Do not report them completed by direct
point-displacement editing. Qualification and public numeric caps are pending design
review and implementation; do not advance capability status yet.
