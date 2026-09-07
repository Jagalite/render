# Spatial query integration review

The geometry domain owns the index, stable point/face/corner result mapping and
local numerical policy. Existing render::Bvh structures are traversed read-only;
no renderer, GPU buffer, triangle packing or shader code changes. The agent owns
one disposable cache. CLI calls use the same core operation with existing native
cancellation controls and no journal publication. Snapshot18 retains authority.

Read-only access is consistent with Inspect. Stale revisions reject before index
construction. Snapshot::revision validates the entire snapshot, including all mesh
content hashes; a cached Arc only skips the additional spatial-source check. This
is explicitly a global admission path followed by a localized tree traversal.
Index construction retains the previous index until complete query success; two
indices can coexist transiently. Build charge bounds the new index construction,
not aggregate process memory, existing source assets, JSON values or previous cache.
Actual retained layout counts omit Arc allocator headers and allocator overhead.

Barycentric projection plus closed edge candidates handles nearest interior, edge
and vertex features. Winding normals remain stable on back-side queries. Every
returned triangle has finite nonzero cross-product squared length. Exact computed
distance ties are counted and ordered by stable face/corner identities. Bounds
prune only strictly greater lower distances. This is a bounded floating-point
profile with numerical fixtures, not an exact-predicate geometry claim.

New element-ID strings avoid JavaScript rounding. The archive's established u64
number encoding stays unchanged; the browser fixture imports and persists whole
JSON as opaque strings. Ordinary placement only consumes returned local positions.
A transformed source entity would need explicit world-space conversion, outside
this initial query profile.

Initial development failures are retained: a test assertion needed an explicit
array type, and corruption was detected earlier by snapshot revision validation
than the test expected. Neither required weakening the geometry or integrity gate.
The initial six native tests, workspace Clippy, CLI/browser placement and exact
native/Wasm result/render comparisons passed. Review then found aggregate polygon
triangulation work needed its own cap. Added sum-of-cubed-corner admission and a
seventh native/Wasm test; final qualification must use this updated package.

The final preservation collector also includes prior `cpu.pfm` artifacts introduced
by the GPU-update fixture, alongside image/pass/receipt files. This adds comparison
coverage for those exact baseline CPU images; no references are regenerated.

Further numerical review retained two actual failing CLI probes: area-squared
underflow selected an edge for a valid tiny-triangle interior query, and distance-
squared underflow included a distinct point in a zero-radius sphere. Dimensionless
projection/segment frames and a scaled-distance comparison below the normal f64
range address these cases. Ordinary squared-distance arithmetic is preserved.
Ten native/Wasm numerical/state tests must pass on the final package.

A further retained quad probe demonstrated normalized-frame cancellation despite
a nonzero original cross product. Index admission now rejects that unresolved
frame for both point and surface queries, preserves any previous cache, and a
finite-result guard prevents non-finite surface metrics from serializing as null.
This boundary is exercised by native/Wasm tests and the CLI/browser fixture.

## Final qualification

The final package passed 94 acceptance checks, 227 native tests and 198 Wasm tests,
including all ten spatial-query tests and the 40-call native workflow. CLI/browser
results and portable costs agree exactly; Metal/WebGPU and OPFS workflows pass.
The collector verified 1,992 previous render artifacts and 213 previous fixtures,
with unchanged dependencies and generated shaders. See evidence/spatial-queries.
The build cache was subsequently removed at the user's request; the tested host
and its hash provenance are preserved under ignored artifacts/spatial-queries/qualified.
