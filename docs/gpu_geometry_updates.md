# GPU geometry updates: gpu-buffer-updates-v1

This bounded renderer prerequisite passed its native/browser acceptance gates. It changes
only disposable GPU caching and observations. Scene authoring retains ordinary
transactions, and evaluated scenes remain immutable.

The renderer serializes packed geometry records as before and compares them against
a retained byte shadow. Identical bytes reuse the buffer. Same-size changes patch
contiguous changed 16-byte records; at 75 percent changed bytes or more than 256
separate ranges, the renderer rewrites the whole existing buffer. A different size
allocates a replacement. Exact byte comparison preserves signed zero and packed
half-float/integer bit patterns. No shader or buffer layout changes are required.

`geometry_upload_statistics()` reports allocation, partial-update, full-rewrite and
reuse operations; upload byte counts for admitted work; queue write calls; visited
comparison record bytes; and retained shadow bytes. The last admitted upload decision includes
its packed size and transfer cost. These are work observations, including uploads
for later-cancelled frames. Pre-admission errors leave counters unchanged. GPU
execution failures discard invalid cache contents; lost devices cannot be reused.
Statistics remain separate from persisted render receipts and document state.

The packed buffer contains BVH/triangle data and texture, material and media records.
Geometry sharing, size changes or packing movement may cause distant writes. This
profile does not guarantee local uploads for arbitrary edits. Existing host packing
is global and the shadow adds host memory equal to the current packed byte size;
record those costs separately from GPU bytes. The existing GPU job allocation budget
is not a host allocator cap. No timing improvement is claimed without measurements.

Existing Rust frame/sequence APIs retain a renderer across evaluated inputs. Browser
static previews currently create a renderer per call; retaining one across preview
calls requires a separate client lifetime/cancellation review. This work does not
introduce an editor or complete M10 sculpting.

The [fixture contract](../fixtures/gpu-geometry-updates/README.md) and
[integration review](../planning/engine-progress/gpu-geometry-updates-contract.md)
define the qualification gates. Native/browser GPU measurements and compatibility
evidence are retained in [the acceptance report](../evidence/gpu-geometry-updates/README.md).

The Wasm `gpu_geometry_update_conformance` diagnostic accepts version 0 and 1..16
uniquely named canonical documents with expected revisions, within a combined
16 MiB JSON input. It validates inputs before creating a renderer and caps each
fixture render at 32 by 32 pixels, eight samples, depth four and 64 MiB GPU budget.
It returns shared Rust CPU/GPU passes and cache observations and exercises fault
paths. It performs no persistent writes; ordinary agent job APIs retain their
existing admission and cancellation behavior.
