# Mesh-local spatial queries: mesh-local-spatial-v1

`query_geometry` is a read-only CLI and agent method. Its request pins
`base_revision` and a retained `mesh` content hash. `points_in_sphere` takes
`center` and `radius_meters`; `nearest_surface` takes `point` and
`max_distance_meters`. Coordinates are asset-local meters. Apply transforms
explicitly when consuming a hit in an ordinary placement transaction.

The version-zero request and budget fields are specified by
`schemas/spatial_query.schema.json`. Sphere results include the closed boundary and
sort by numeric stable point ID. Surface results include the face, three ordered
corner IDs, barycentric weights, position, geometric winding normal and distance.
Interior projection and segment parameters use a dimensionless triangle/edge frame
to avoid squared-area underflow on admitted tiny triangles. Triangle cross-product
squared length must still be finite and nonzero in the existing floating-point
profile, and the dimensionless frame must retain a finite nonzero cross product.
Numerically unresolved frames reject explicitly, including during sphere-index
construction; non-finite computed surface results never serialize as null metrics. Distance comparisons retain squared values in the normal f64 range and
use scaled Euclidean lengths below it, preserving nonzero distances and zero-radius
boundaries down to subnormal squared values. Exact computed metric ties use face ID,
then ordered corner IDs; the
response counts all tied triangles. New IDs are positive canonical decimal strings,
including values above 2^53. Existing mesh archive IDs remain numbers: pass native
archive JSON as an opaque string until Rust parses it in a browser client.

The profile accepts validated planar polygons (at most 256 corners per face), loose
points/edges, disconnected and nonmanifold surfaces. Existing triangulation rejects
degenerate/nonplanar polygons even for sphere queries, because the shared index
contains both domains. Empty surfaces return null and zero ties. Metrics must be
finite, coordinates within +/-1e9 and distances within 0..=1e9. This floating-point
profile is not an exact-predicate CAD kernel.

Caps are independent: 65,536 points, 131,072 derived triangles, 393,216 corners/edges,
8 MiB canonical source, 32 MiB conservative build charge, 16,777,216 aggregate
triangulation-work units, 262,144 visited nodes,
131,072 exact item tests, 4,096 sphere hits and 1 MiB serialized result. Smaller
request budgets apply to warm caches too. Triangulation work charges the sum of
cubed face corner counts before entering the synchronous ear-clipping phase. This
bounds aggregate candidate/containment work even when each face meets its 256-corner
limit; it is an admission proxy, not measured CPU instructions or elapsed time. Build charge is
`sum(n * (640 + 8 * (bit_length(n) + 2))) + 64 * corners + 4096`, over point and
triangle counts. It covers index builder vectors, their growth and split capacity,
bounds, triangle correspondence and sorting scratch. It is a portable admission
charge, not an allocator/process-memory cap. Source serialization/validation and
JSON Value allocation are separate global costs. A counting writer bounds initial
serialization before canonical JSON allocation; canonical f32 promotion can change
byte length, so the canonical result is checked again.

A session retains one complete immutable index and its Arc source. Native one-shot
CLI calls construct it anew. Warm agents reuse by content hash; a different source
Arc requires content revalidation. The cost report separates source bytes, portable
build charge, constructed point/triangle counts, visited nodes/items and actual
retained index layout bytes (excluding the separately retained source allocation).
The latter reflects native/Wasm layout, excludes allocator and Arc allocation headers,
and is not a portable equality gate. The old cache can coexist with a candidate
index until successful publication; its memory is additional to the new-build charge.
Document revision validation and serialization are global on every request: existing
Snapshot::revision validates and rehashes all retained mesh assets. The report
therefore marks whole_snapshot_validated even on a warm cache. The additional
source-identity flag describes only the separate index admission check. No end-to-end locality
or speedup follows from traversal counts alone.

Cancellation checks run at admission, build boundaries, every visited node/item and
before response/cache publication. Existing triangulation and BVH construction are
bounded synchronous phases; they cannot be interrupted midway by this interface.
Synchronous browser dispatch cannot process a later UI cancellation event until it
returns. Failed/cancelled queries never replace the prior cache or change document,
journal, accepted-request state or jobs. Read access follows `inspect` semantics.

The bounded profile passed [native/browser qualification](../evidence/spatial-queries/README.md).
Sculpt displacement/mask chunks, affected-region
refitting/evaluation, multiresolution and dynamic topology are separate work.
