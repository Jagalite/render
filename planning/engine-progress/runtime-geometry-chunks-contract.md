# Immutable runtime geometry chunks (candidate)

Baseline c5e7233. Architecture sections 6.3, 7, 11 and 17.4 and the M10 locality
acceptance apply. This bounded prerequisite changes disposable evaluated storage,
not document schemas, revision hashes, authored topology or operation authority.

Geometry triangle arrays and BVH node arrays retain their exact index and iteration
order in immutable 64-item chunks behind shared roots. Cloning a root shares all
payloads. Replacement validates all indices, creates a candidate, copies only
touched chunks, checks cancellation before publication, and returns explicit counts
of root references copied, chunks copied and elements cloned. Indices are private
runtime slots, never durable mesh identities. No mutation method exposes published
payloads. Empty replacements share the root. Nested triangle UV arrays and BVH leaf
item vectors still clone when their containing chunk is copied; report their cost.

BVH construction and ray traversal preserve the old algorithm and ordering. Query
indices retain compact leaf vectors. Account for chunk roots and payloads in actual
retained query-layout observations; portable query admission limits must remain
conservative. Generic element storage byte counts exclude owned nested allocations,
Arc allocator headers and allocator overhead, and must say so.

Test boundary sizes, forward/reverse iteration, duplicate-free sorted replacements,
invalid indices and every cancellation checkpoint. Analytic renderer/BVH fixtures
must show unchanged candidates, pixels, passes and GPU packing; all previous engine
workflows run on native and packaged browser. Observe payload clone counts and
retained layout plus full workflow process memory/time, without an unmeasured speed
claim. Dependencies, shaders and all existing image/receipt fixtures remain exact.

This is storage groundwork, not localized sculpting. Initial BVH/triangle construction,
root-table copying for edits, snapshot validation and GPU host packing remain global.
Refit correspondence, displacement/mask authoring, brush operations and long gesture
recovery are subsequent bounded changes. Do not advance full M10 status.
