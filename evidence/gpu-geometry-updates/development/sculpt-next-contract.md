# Proposed next M10 engine work after painting qualification

Architecture 17.4 and representation decisions require localized spatial queries,
dirty chunks and affected-region tracking. Do not implement a sculpt stroke by
calling the existing dense modeling conversion and replacing the whole canonical
mesh. Current render::Bvh only exposes ray candidates; a sphere/nearest query seam
and bounds/refit contract need review before sculpt or retopology use it.

Separate retained authored topology from sparse displacement/mask chunks keyed by
stable point identities. Start with a bounded topology-preserving sculpt profile,
with explicit asset-local metric positions, radius, falloff, pressure/time and brush
direction. Base geometry must remain immutable and recoverable. f32 displacement
chunks can coexist with the base mesh's authored f32/f64 precision class; do not
force all sculpt data into dense f64 coordinates. Corner UVs and seam identity stay
with unchanged topology. Tangent/normal updates must invalidate affected faces.

Useful first brush families are displacement along a specified direction, normal
inflation and bounded local smoothing; symmetry, point masks and face sets need real
semantics and analytic tests. Reuse rational sample time and ordinary transaction
publication, but keep geometry-domain masks distinct from texture layer masks.
Checkpointed gestures pin the source mesh, sculpt asset and evaluation revision.
Track touched/query vertices, changed chunks, affected faces and output bytes.

Multiresolution over a base mesh is separate from dynamic topology. Review explicit
level correspondence and displacement transfer before implementing level creation,
selection and edits; test round trips, seams and level changes. Remeshing must name
its loss/correspondence policy and reject stale face/edge/point selections. Retopology
uses spatial queries and existing modeling operations, not a separate privileged
editor path. No interactive M09 claim follows from an engine fixture.

Before qualification, demonstrate localized edits, mask/symmetry/smoothing behavior,
unchanged distant chunks, local normal changes, cancellation and long-stroke undo,
source/topology invalidation, actual renderer behavior and native/browser recovery.
A fresh one-shot render may require initial conversion/upload; report that cost
separately. Do not claim incremental GPU geometry updates unless an existing renderer
instance actually reuses untouched geometry and an instrumented test proves it.

## GPU prerequisite after painting integration

The existing Renderer holds Option<(content_digest, Buffer)> and geometry_uploads.
pack_geometry writes BVH nodes followed by leaf triangle rows; pack rebuilds the
host Vec and deduplicates by geometry identity while traversing instances. Changes
in sharing or packed sizes can therefore move distant ranges even when one edited
mesh keeps its topology. A first bounded cache change
may preserve buffers and upload changed aligned ranges; it must report the whole
CPU packing/comparison cost and retained shadow bytes. It alone is not full sculpt
locality. Later stable geometry slots and affected-region evaluator updates are
needed if packing movement causes distant writes.

Use a renderer-owned byte shadow with exact bit equality (including signed zero),
16-byte aligned rows, contiguous changed-row ranges and COPY_DST | STORAGE buffer
usage. Same bytes skip upload; same length applies bounded changed ranges; changed
length reallocates fully. Define range-count/coalescing and dense fallback policy
with transferred byte counts, not a timing claim. Cache invalidates on GPU failure
or device destruction. Queue writes occur inside existing validation/OOM scopes;
pre-submit cancellation performs no upload, post-submit cancellation drains and
leaves a valid disposable cache for the next request. Do not mutate evaluated
geometry, document snapshots, packed formats or shader code.

Expose observational upload statistics through Rust and the browser conformance
adapter, separate from persisted render receipts to preserve existing bytes.
Validate persistent-renderer output against a fresh renderer after local edits,
unchanged repeats, topology resize, camera/material changes, alternating surface
pipelines, cancellation and device recreation. Add a reproducible fixture large
enough that touched/uploaded bytes can demonstrate a localized edit. Check retained
host shadow bytes and whole pack bytes explicitly. Use analytic geometry/pixels and
exact prior artifact comparisons; no automatically regenerated reference images.

After that, review a revision-pinned immutable spatial query index and sparse sculpt
asset schema against the geometry/evaluator owners before building brush admission.

### Reviewed cache accounting and failure seam

`pack` guarantees a nonempty geometry buffer, including empty scenes. Its geometry
rows also contain texture headers, legacy texels, material descriptors and media;
name reports packed geometry-buffer bytes, not only mesh vertices. Instance traversal
and content-ID sharing determine geometry packing. No persistent schema changes are
needed for the initial upload path.

Keep existing `geometry_uploads` as changed-buffer upload operations. Add an immutable
statistics accessor with allocation/patch/rewrite/reuse counts, queue write counts,
transferred bytes and visited 16-byte comparison record bytes. A last-operation
report can distinguish first allocation, changed size, same-size sparse patch,
dense/fragmented rewrite and exact reuse. Compute retained shadow bytes from the
actual live cache so a device failure does not leave a misleading retained-size
report. These are observations of admitted work, including later cancelled frames;
pre-admission errors leave counters unchanged. Do not insert cache statistics into
persisted RenderReceipt, because existing artifact bytes are contractual.

The private planner prototype now stops after 257 disjoint ranges or reaching the
75-percent dirty-byte threshold; it returns one whole-buffer rewrite. This bounds
range-list allocation as well as driver calls. Record visited comparison spans on
early fallback honestly. Exact bit comparison preserves packed integer/half-float
bits and signed zero; never use numeric float equality on packed rows.

Queue writes and create_buffer_init stay inside the existing error scopes. The
binding always references the same cache buffer after a same-size patch/rewrite.
Validation/allocation failures clear the shadow and GPU buffer together. Mapping
failure marks the renderer lost; destroy clears the cache; no document mutation
occurs. Post-submit cancellation still drains submitted work and may retain a valid
cache for a subsequent render. Check these paths against a fresh renderer.

Host overhead increases by one retained byte shadow, bounded by the admitted packed
buffer size. Existing GPU job `max_bytes` is not an allocator cap for these host
forms. Report full packed Vec bytes, serialization bytes, retained shadow and peak
simultaneous logical representations separately. A later sculpt pipeline must avoid
full host packing/evaluation for every stroke; this prerequisite only reduces
measured GPU transfer and allocation churn where the actual fixture demonstrates it.

### Existing consumers and bounded scope

Native/browser GPU frame and sequence methods already retain one Renderer across
samples/frames, so changed geometry can exercise the upload cache through current
engine consumers. BrowserAgent static previews currently construct a new Renderer
per call. Retaining a renderer across those preview calls is a separate client
resource-lifetime change; do not fold it into the first cache patch without reviewing
busy/cancellation/future-drop/device-loss behavior. Use the existing sequence path
plus a persistent-renderer conformance workflow for initial native/browser evidence.
There is no need to introduce another manager or persistent document state.
