# Spatial query fixture contract

Original CC0-1.0 inputs will contain a triangulated grid, analytic adjacent triangles,
loose points/edges and disconnected/nonmanifold faces. Nonmonotone stable IDs,
including IDs above 2^53, test domain identity and deterministic tie rules. Input
generation must reproduce byte for byte and must not generate image references.

Native Rust and Wasm tests compare indexed queries with independent plane/edge/vertex
examples and a brute-force candidate enumeration. Sphere membership includes the
boundary. Nearest hits report stable face/corner identities, barycentric residuals,
winding-oriented normals and exact computed-distance ties. Empty surfaces, invalid
metrics, degenerate/nonplanar polygons, budgets, stale revisions and cancellation
must have explicit results with unchanged document/history and valid cache state.

A native CLI/browser workflow queries a retained mesh, uses the result in an ordinary
placement transaction, renders, archives/reopens and repeats queries. Keep the
source entity transform identity for the asset-local-to-placement fixture; do not
claim arbitrary world-space queries. Preserve high IDs through opaque native JSON
transport and canonical decimal result fields. Report cold build, warm cache and
traversal work separately, plus real process/index/source byte observations.

Earlier fixture/render/shader/dependency outputs must remain unchanged. This
bounded profile is qualified in `evidence/spatial-queries`.

## Reproduction

Run `python3 fixtures/spatial-queries/generate.py /tmp/render-spatial-fixture` and
compare `high-id-grid.json` with `data/high-id-grid.json` byte for byte. The original
CC0 generator authors 291 points (including two additional loose coincident points)
and 512 planar grid triangles. Point/face/corner IDs exceed 2^53 and descend in their
storage arrays. Five grid queries cover a shared diagonal, coincident points, a closed
sphere, an outside edge/vertex region and a bounded miss. A separate tiny triangle
adds interior and zero-radius underflow regressions. Returning to the grid verifies
one-index eviction: eight queries in total. Generated inputs never include render
references.

`crates/core/tests/spatial_queries.rs` additionally exercises an analytic triangle,
a larger plane comparison, empty geometry, loose edges, a nonmanifold edge,
disconnected surfaces, invalid polygons, read permissions and cancellation/cache
atomicity. The final CLI/browser workflow additionally covers both numerical underflow probes
through opaque archive transport. Full preservation qualification passed; see `evidence/spatial-queries`.

A conditioned quad whose last triangle collapses in the dimensionless frame is a
negative input. It remains an ordinary mesh asset, but both spatial index profiles
reject it explicitly. Its original failing CLI response and corrected rejection
are retained separately; it is never bound to the rendered fixture scene.
