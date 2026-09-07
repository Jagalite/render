# Next M10 spatial/sculpt boundary under review

Canonical architecture sections 6.3, 7, 17.4 and M10 apply. This is preparatory
analysis only, with no implementation or capability claim. The preceding GPU cache
candidate reduces transfer for the measured local edit; CPU packing/evaluation
remain global and need an affected-region path for qualified sculpting.

Existing render::Bvh supports ray candidates over generic Bounds. Mesh::triangles
returns corner indices, preserving corner attribute/seam identity. Extend the
existing bounds/BVH query seam for sphere candidates and nearest-surface search,
with cancellation and visited-node/item budgets. Keep a geometry-owned revision
binding and map runtime candidates back to stable point/face/corner IDs at the
public boundary. Define tie-breaking, inclusive radius boundaries, finite local
meter coordinates and arithmetic bounds before implementation. A point index and
triangle index have different payloads; do not use triangle hits as fake point
identity. Build costs are global and should be reported separately from queries.

Local edits require refitting touched leaves and ancestors. Existing Bvh nodes have
no parent/item-leaf table. Review a derived index wrapper that owns those lookup
arrays without changing persistent identity or requiring a global rebuild per dab.
Keep the ray renderer's established ordering and outputs unchanged. Compare sphere
and closest-triangle results with analytic/brute-force fixtures, including ties,
boundaries, loose points/edges, disconnected and nonmanifold surfaces, empty input,
stale revisions, invalid metrics and cancellation. Degenerate triangles need a
named closest-feature policy, not NaN propagation. Record visited candidates and
retained index bytes; do not infer query speed from asymptotic complexity alone.

Sculpt authored state should retain an immutable base mesh and sparse f32 local
displacement/mask chunks keyed by stable point identity. Never mutate published
mesh arrays. One possible deterministic chunk partition is a block of 128 stable
point-ID values (point_id >> 7), with typed offsets/records; validate every ID against
the base topology. This is a proposal, not a frozen public schema. Keep masks in
point domain and face sets in face domain; texture masks are unrelated. Preserve
base f32/f64 precision without converting every authored mesh to dense f64.

Base n-gons can become nonplanar under a local displacement. The existing canonical
triangulator requires planar polygons, so review fixed base triangulation plus
derived displaced triangles rather than constructing an invalid canonical output
mesh. Authored normals/tangents, skin/morph/deformation ordering, smooth groups and
UV seams need explicit policies. Do not retain stale shading directions silently.
An initially bounded profile may reject unsupported interactions, but its supported
normal/affected-face update semantics need analytic and rendered evidence.

Brush admission should pin document, base mesh, sculpt state and evaluation revision;
prepare sparse changes with pressure/time/resampling, symmetry and point masks,
then use ordinary transactions. Refit disposable spatial state only after accepting
a matching revision, and rebuild from retained authored state after failure/recovery.
Inflation, directed displacement and local smoothing have different neighborhoods
and must report touched points/chunks/faces. Long gestures use bounded checkpoints
and normal undo/recovery. Multiresolution correspondence and dynamic remesh loss are
separate interfaces that must be designed before their consumers are implemented.

Review public 64-bit element-ID encoding explicitly. Existing Mesh stores u64 IDs;
new query/sculpt inputs must preserve values above JavaScript's 2^53 integer limit
when a client constructs a request or reads a result. Prefer an explicit decimal or
hex string field for the new typed ID domain, consistent with established local-ID
mapping adapters, and test high-ID native/browser round trips. Do not silently
change the existing mesh archive encoding or route an opaque native document
through lossy JavaScript numeric parsing.

Further locality review: ordinary stroke preparation can be sparse while a naive
Evaluator still rebuilds every derived triangle afterward. Do not use that split
to imply whole-workflow locality. Inspect Geometry/Bvh ownership and chunk sharing
before rendering sculpt updates; the current public runtime Vec<Triangle>/Vec<Node>
clone deeply. A private chunked representation or revision-matched delta path needs
explicit integration review and measured unaffected-region reuse. The GPU cache
prerequisite does not by itself fix global host serialization. A fresh one-shot
render has unavoidable initialization, which must remain separate from incremental
stroke-frame observations.
