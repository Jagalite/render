# Spatial query acceptance

`mesh-local-spatial-v1` passed on baseline 4e8441c. It supplies read-only asset-local
sphere and nearest-surface queries with pinned revisions and stable decimal point,
face and corner IDs. Ordinary placement consumes query results through the existing
transaction API. This is an M10 prerequisite; sculpting, incremental evaluation,
multiresolution, remeshing and the deferred editor remain open.

The final run passed 94 acceptance checks, 227 native tests
and 198 Wasm tests. Ten query tests cover plane/edge/vertex analytics,
scaled and tilted triangles, grid enumeration, nonmanifold/disconnected surfaces,
empty geometry, high IDs, exact ties, invalid/stale/budget rejection, read permissions,
exhaustive cancellation checkpoints and cache atomicity. Aggregate triangulation
charges and underflow boundaries down to 1e-300 have native/Wasm tests.

The reproducible fixture passes 40 CLI calls and 8 queries across a
291-point/512-triangle grid and a tiny triangle. Native and packaged browser results,
portable admission/traversal counts, placement revisions and CPU pixels/passes are
exact. The browser verifies one-index eviction and OPFS recovery while transporting
high-ID archives as opaque JSON strings. The old binary rejects the new operation
without changing state. Both demonstrated numerical bugs also pass through the wire.

The grid's first nearest query visits 15 nodes and tests 4
triangles. Its source is 97591 canonical bytes; the portable build charge
is 691080 bytes and triangulation charge 13824.
Actual retained index layout is 62303 bytes native and
44099 bytes Wasm, excluding source allocation, Arc headers
and allocator overhead. Native one-shot query processes peak between 24330240 and
24985600 resident bytes, including project loading and global snapshot validation.
Warm agents reuse the index, but document admission still validates/hashes every
retained asset. These are resource observations, not a comparative speedup claim.

Browser CPU/WebGPU RMSE is 1.6204745174669953e-08; Metal/WebGPU maximum channel
error is 5.960464477539063e-08. Depth/normal and object-ID comparisons
pass their existing tolerances. Native archive recovery preserves CPU images exactly.
All 1992 prior render/pass/receipt artifacts (including GPU-fixture
cpu.pfm files) and 213 prior fixture files remain byte identical.
All five shaders, Cargo files, resolved packages/features and native links are
unchanged. No computational FFI, package or implicit external service was added.

Runtime qualification is macOS ARM64 Metal and Chrome WebGPU/OPFS; Linux and Windows
have compile checks. Exact native/Wasm package hashes, source and evidence manifests,
fixture reproduction and representation costs are retained. Development evidence
includes corrected test/fixture setup failures, deliberately interrupted runs for
review fixes, and original CLI probes demonstrating squared-area/distance underflow.
No rendering references were regenerated and no gate was weakened.

Reproduce with `scripts/validate-spatial-queries.py`, localhost:8783 and Chrome CDP
9224. Set the existing Blend reference/Blend/UV/painting baseline variables and
RENDER_SPATIAL_BASELINE_HOST to the protected 4e8441c host. Collect with
`scripts/collect-spatial-queries-evidence.py <run> <fresh-directory>` and set
RENDER_SPATIAL_PREVIOUS_RUN to the qualified GPU geometry-update run. Snapshot18,
persistent render receipts and ordinary mutation authority remain unchanged.
