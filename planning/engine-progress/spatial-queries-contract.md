# Revision-bound spatial query candidate

Baseline: 4e8441c. Canonical architecture 6.3, 7 and 17.4 and M10 apply. This
candidate supplies a read-only mesh-asset query seam for sculpt/retopology consumers.
It does not qualify sculpt brushes, multiresolution or incremental evaluation.

Use the existing Bounds/Bvh tree for point-sphere and nearest-triangle candidates.
Keep its established ray traversal/build ordering and existing render outputs.
A geometry-owned Index retains an immutable source mesh, point/surface trees and
corner-to-face correspondence. Runtime item slots never become durable IDs. New
query/result local IDs use canonical decimal strings, including values above 2^53.

Operate in asset-local meters, without silently applying entity transforms, curves,
volumes, procedural graphs, skinning or animated deformations. The input names a
retained mesh content hash and document revision. Validate all public fields,
finite metrics and bounds before work; unchanged queries must not publish document,
journal, accepted-request or job mutations. Follow existing read-access behavior;
never require write permission for a read-only query.

The initial profile supports validated triangulatable planar polygon meshes,
including disconnected/nonmanifold surfaces and loose points/edges. Surface queries
use fixed corner-domain triangulation. Degenerate/nonplanar polygons reject through
the existing triangulator; no surface becomes empty silently. Empty valid meshes
and meshes without faces have explicit empty query results. A point sphere includes
its boundary and returns stable-ID-sorted hits. Nearest surface returns a stable
face/corner triangle identity, barycentric weights, local position, geometric normal
and distance, with an explicit tie policy. Test independently against plane/edge/
vertex examples and brute-force candidates, with residual and weight bounds.

Cache at most one complete immutable index in an agent session. A native one-shot
CLI call pays construction cost; long-lived Rust/browser clients can reuse it.
Separate global source validation/index construction from visited query work in
reports. Cancellation or a failed build must not publish a partial cache entry.
Source-content identity permits reuse across document revisions after the new
revision is admitted. Reject stale requests before index construction. This cache
is a derived query cache, not another document authority or editing manager.

Profile caps: 65,536 source points, 131,072 derived triangles, 8 MiB canonical source,
32 MiB index allocation estimate, 262,144 visited nodes, 131,072 exact item tests,
4,096 returned sphere points and 1 MiB result bytes. Coordinates/radii are finite,
absolute values at most 1e9 meters; radius and maximum distance are nonnegative.
Caps are independent. Bound temporary BVH construction with a conservative preflight
estimate, then report actual retained Vec capacities separately from the source.
The existing bounded triangulation/build steps are synchronous; cancellation is
checked at admission, build phases and query traversal, with exact guarantees stated.

Positive, invalid-input, budget, cancellation and stale tests must run on native and
Wasm. A reproducible high-ID fixture must exercise native CLI, browser cache reuse,
archive/OPFS recovery and render preservation. No new dependency or shader is
expected. Capability status remains unqualified until complete evidence passes.

## Reviewed numerical and admission choices

Traverse the existing Bvh nodes from the query module; do not change the renderer's
build or ray path. Point and surface trees remain distinct derived payloads. Index
construction reuses the existing bounded triangulator/builders with cancellation
between phases; traversal checks every visited node/item. Cap faces at 256 corners
to bound the synchronous ear-clipping phase. Compact query-owned leaf item vectors
after construction if useful, without modifying the renderer's trees.

Use a platform-independent conservative build-byte charge, accounting for BVH node
capacity growth, retained split-vector capacities, temporary bounds and triangle/
face correspondence. Native/Wasm budget decisions must not depend on usize size.
Report actual retained layout bytes separately as platform-local evidence. Source
hash/revision validation and serialization are global admission costs, not local
query work; include source bytes and do not claim end-to-end query locality from
node counts alone. A cached identical Arc source is already immutable/validated;
a different source allocation must have its content identity checked before reuse.

Nearest-triangle distance considers the interior projection and each closed edge
segment. Interior barycentrics use oriented cross products against the triangle
normal; candidates must have nonnegative weights. Return the minimum computed
squared-distance candidate. Across triangles, exact distance ties use numeric stable
face ID then the ordered stable corner-ID triple; report the number of exact tied
triangles. Normals follow authored face winding, including back-side queries. This
is an explicit bounded floating-point profile, not an exact-predicate CAD claim.

Use the already pinned Rust libm square root for returned distances/normals and
common basic arithmetic. Tests cover plane interior, edge, vertex, back side,
inclusive maximum distance, exact ties, loose points, high IDs and brute-force
comparisons. Do not alter existing mesh archive ID encoding: transport native
high-ID documents as opaque JSON strings until Rust parses them, while new result
IDs are canonical decimal strings.

## Aggregate triangulation admission correction

Review found that a 256-corner per-face cap alone admits many individually expensive
polygons. Before calling the synchronous existing triangulator, charge the sum of
cubed face corner counts and cap it at 16,777,216. Expose a positive request budget
within that cap and report the charge separately from build bytes and query visits.
Apply the same admission to warm caches. Test exact budget boundaries and multiple
256-corner polygons whose individual limits pass but aggregate work rejects. This
proxy bounds candidate/containment work, not process runtime or CPU instructions.

## Numerical underflow correction

Retained CLI probes demonstrated that direct squared-area normalization can return
a non-unit normal and the wrong closest feature on a valid 1.4e-81 meter triangle,
and that distance-squared underflow can include a distinct nearby point in a
zero-radius sphere. Use dimensionless triangle/edge frames for projection and
parameters. Retain ordinary squared-distance arithmetic where the square is at
least f64::MIN_POSITIVE; below that, compare scaled Euclidean lengths. The two
ordered ranges form the metric comparison, with stable face/corner ordering for
exact metric ties. Normalize signed zero limits. This refines the initial raw
squared-distance proposal before qualification, without changing renderer paths.

Native/Wasm tests cover tiny-to-large geometry, translated tilted planes, radii
and distances down to 1e-300, inclusive boundaries, zero-radius misses and signed
zero. The underlying triangulator and finite/nonzero cross-squared admission
remain unchanged; there is no exact-predicate or arbitrary-scale CAD claim.

The normalized cross product must itself be finite and nonzero. A retained
conditioned quad showed that rescaling may cancel a last triangle's cross product
even when the original is nonzero. Reject that unresolved frame during index
construction, including for point queries. Surface results require finite position,
barycentrics, normal and distance before delivery; preserve the prior cache on
any numerical failure. The fixture includes structured native/browser rejection.
