# Sparse displacement integration review — candidate

Base: `7bd2e25`. Candidate branch: `feat/sculpt-displacements`.
Acceptance is in progress; this is not a capability qualification.

## Authority and cross-domain behavior

Snapshot19 adds independent typed chunk, asset and binding maps. Schema0..18 omit
empty new maps so prior encodings remain unchanged. Both high-level authoring and
low-level commands use Document preparation/publication; agent durable dispatch and
CLI durable storage share the same commands. No editor-specific mutation, mutable
snapshot cache, untyped metadata escape, computational FFI or runtime dependency
was introduced. Point/block identities are decimal strings at the new wire seam.

The source must be a triangular mesh with a nonempty surface. Authored normal,
tangent and tangent-sign attributes are rejected; UV/color corner data stays on
the immutable base. Conflicting geometry/deformation bindings, material displacement
(including layer overrides), grooms and roots attached to sculpted anchors are
rejected. Transform animation remains outside the sculpt geometry cache. Modeling
that replaces an entity base must explicitly clear or replace its sculpt binding.

## Cache and numerical review

Private basis state fixes triangle correspondence and BVH partition from base
geometry. Refits update touched leaf bounds and unique ancestors in descending node
order; cold reconstruction uses the same partition and reduction. Parent/child and
leaf item order never change. Position/bound signed zeros normalize in the sculpt
profile. f64 positions must remain finite within 1e9 meters, delta components are
bounded f32 values within 1e6 meters, and triangles must have resolvable area in both
f64 and packed f32. Degenerate input is rejected before publication.

A cache owns one immutable base and one current state, retaining referenced old
chunks so later checkpoint pruning does not break diffing. Failed/cancelled
Evaluator runs preserve the previous complete cache. Session root CPU rendering
publishes only its sculpt cache after a successful render; unrelated geometry
families retain their existing per-call ownership. The agent cancellation callback
is now forwarded through root rendering, covered by a late-cancellation regression.
The browser's ordinary dispatch shares that core implementation. GPU preview and
one-shot CLI rendering currently reconstruct cold.

## Resource limits and remaining global work

All retained sculpt sources, including those named by currently unreferenced
chunks, are charged before allocating stable-ID lookup sets. Base/binding/asset and
chunk caps bound retention. High-level authoring also checks cold reconstruction
against its explicit budget, so accepted edits can reopen without prior cache state.
This global preparation cost is deliberate and is not a local-stroke speed claim.

Portable costs include dirty work, chunk/root copies and retained reference copies.
Actual basis/current geometry and nested payload layouts are reported separately,
with shared chunk counts in the resource fixture. These are not an allocator census:
Arc headers, allocator overhead, source storage and private map allocation layout
are excluded. Admission, hashing, block scans, complete root tables, instance bounds
and GPU host packing retain global work. A 2,048-triangle development fixture moves
one point, changes six triangles and refits 35 nodes, copying 128 triangle and 512
node payloads; the instance-bounds scan still visits 6,144 vertices. Native/Wasm
portable counters match; their actual layouts differ as expected.

The predecessor's higher whole-process PBR footprint remains an unresolved
observation. No whole-process memory or speed improvement is claimed here.

## Development evidence and completion boundary

An intermediate complete native core run passed 214 tests. The final focused suite
has 11 native/Wasm tests covering canonical IDs/chunks, invalid/empty surfaces,
seams/flat normals, shared blocks, degenerate edits, fresh/refit/undo equivalence,
cache eviction/pruning, permission/revision/budget failures, cancellation through
preparation and publication, durable failure/retry, warm public rendering and costs.

The development CLI workflow passed 28 calls including Metal output and archive
recovery. Chrome WebGPU/OPFS passed; native/Wasm CPU pixels/passes matched exactly.
Measured Metal/WebGPU maximum color error was 8.95e-8 (rounded upward), inside the
existing 2e-5 tolerance. The old binary rejected new sculpt journal commands.
These development observations precede the final combined acceptance run; attach
its final source/package identities and regression/dependency/shader evidence before
marking this subgate complete.

Brushes, masks, symmetry, multiresolution, remesh and retopology remain separate work.
M10 and authoring beta stay open. No interactive-editor completion claim is made.
