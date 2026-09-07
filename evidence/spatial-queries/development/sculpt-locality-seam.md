# Follow-on locality seam observations

Evaluator owns BTreeMap<String, Arc<Geometry>> keyed by evaluated geometry identity.
Geometry currently retains Vec<Triangle> and a Bvh containing Vec<Node>, whose leaf
items are Vec<usize>. An unchanged mesh can reuse Arc<Geometry>, but a changed key
rebuilds every triangle and its BVH. Ordinary sparse authored displacement alone
would not address that cost.

Potential next bounded prerequisite: fixed-size immutable runtime chunks for
triangle/node payloads, with touched chunk copy counts and a derived point-to-face,
item-to-leaf, node-to-parent mapping. Preserve established iteration/index order,
ray paths and pack output. Initial construction is global; incremental refit updates
affected leaves and ancestors. Dirty metadata traversal and chunk-root copying must
also be counted rather than hidden behind a locality label.

GPU lib.rs currently reserves three records per BVH node and packs geometry into a
single vec4 array. The previous upload candidate still creates and compares that
whole host array. A later revision-matched packing cache needs stable range mappings
and explicit invalidation for layout/material/texture/instance changes, with local
position/normal/bounds updates mapped to GPU records. Do not claim whole-pipeline
locality until this has measured unaffected-region reuse and fresh-render parity.

Canonical sculpt storage can remain immutable base mesh plus sparse f32 displacement
and scalar-mask chunks keyed by stable point IDs; face sets use stable face IDs.
Multires and remesh correspondence are distinct designs. A displaced n-gon needs
fixed base triangulation and updated geometric/shading normals, rather than forcing
an invalid nonplanar canonical polygon through the existing planar triangulator.

## Candidate authored interface questions for the next bounded change

A source mesh plus sparse local f32 displacement overlay should expose point access
without materializing dense f64 positions. Fixed base corner triangulation is shared
across displacement revisions; shading must use the displaced triangles. An initial
profile may explicitly reject authored normals/tangents or conflicting skin/morph/
material displacement rather than retaining stale directional attributes. UV/color
corner attributes and stable IDs must retain correspondence. Flat geometric normals
are a bounded profile, not broader smoothing-group support.

A possible immutable chunk partition is stable_point_id >> 7 with 128 offsets.
Separate displacement and scalar-mask chunks avoid copying displacement when only
masking. Block IDs need canonical string encoding; mask values have an explicit
scalar role independent of paint texture masks. Chunk/source hashes bind topology;
ordinary compare-replace sculpt bindings publish only a fully validated candidate.
These are design candidates, not accepted public schema or implementation.

Brush centers must query the currently displaced geometry, not silently the base
mesh. A future query source variant therefore needs shared position-overlay access,
fixed topology correspondence and refitted point/surface trees. Shared derived
payloads remain caches; authored state/transactions retain authority. Refit maps
(point-to-incident triangles, item-to-leaf and parent links) need their own bounded
memory and invalidation evidence.

Symmetry requires a defined overlap policy on symmetry planes; smoothing needs a
snapshot-based neighborhood update to avoid point-order dependence. Long strokes
need bounded checkpoint transactions and ordinary undo/recovery. Normal updates,
mask weights, UV seams and unaffected regions must have analytical and rendered
checks. Do not infer them from sparse authored JSON or reduced GPU upload counts.

## Deterministic base partition for future refits

A new sculpt profile should build triangulation and BVH partition from its immutable
base mesh, then refit bounds for displacement. A fresh reconstruction of that same
sculpt state uses the same base partition plus overlay, matching incremental tree
ordering. Rebuilding a fresh tree from displaced positions can reorder equal-distance
ray candidates at seams/coincident surfaces; do not assume exact fresh/refit parity
without addressing that. Keeping the base partition for the new profile preserves
existing renderer traversal/tie behavior and avoids changing old image references.
Bounds refits recompute affected leaves/ancestors in fixed child order, with explicit
point-to-triangle and parent maps and unchanged primitive identities.

Document admission is another separate cost: Snapshot::revision validates then
hashes canonical JSON containing asset payloads. Reusing Arc geometry does not avoid
those source scans. Instrument admission, overlay computation, evaluation, packing
and transfer separately before claiming end-to-end locality. Any verified-snapshot
cache or future content-addressed revision representation needs its own authority,
deserialization, compatibility and transaction review; do not silently change old
revision hashes to improve a benchmark. This is a measured-work investigation to
perform next, not an accepted revision-schema rewrite.
