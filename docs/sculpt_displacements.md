# Sparse sculpt displacements — unqualified candidate

The candidate profile `sparse-displacement-v1` adds direct stable-point editing over
an immutable triangular mesh. Snapshot19 contains typed chunks, assets and entity
bindings. `author_sculpt` replaces absolute f32 displacement in local meters; zero
removes an entry. This is a headless operation shared by CLI and browser agent
dispatch. Brushes, masks, symmetry, multiresolution, remeshing and retopology remain
outside this candidate, and M10 remains open.

The request pins document revision, entity and source asset, names point IDs as
canonical positive decimal u64 strings, and supplies explicit budgets. Chunks group
64 stable IDs, with a decimal block coordinate and slot. Existing entries in a
changed block survive unless explicitly replaced. Ordinary PutSculptChunk,
PutSculptAsset and compare-replace SetSculpt commands publish the result through the
transaction engine. Failed preparation, cancellation, stale selection and failed
durable publication preserve the document. Retained old assets support undo.

Base geometry must have at least one triangular face and at most 65,536 points, 131,072 faces
and 393,216 corners. Canonical sources referenced by any retained sculpt chunk or
asset are limited to 8 MiB individually and 16 MiB combined. The snapshot retains
at most 1,024 chunks, 64 sculpt assets and 32 bindings. UV/color corner attributes
remain authored on the base. Flat displaced normals are evaluated; authored
normal/tangent fields and conflicting procedural, typed geometry, skin/morph,
shading-frame, groom and material displacement bindings are rejected. Material
layer overrides and groom roots on sculpted anchors are included in admission.
Triangles must retain numerically resolvable area in both f64 and packed f32.

The Evaluator fixes correspondence and BVH partition from the base mesh. Refits
copy touched 64-item triangle/node chunks plus complete root reference tables.
Fresh reconstruction applies displacements to the same partition and reduces
bounds in the same order. The cache keeps one base and one complete current entry;
it retains referenced old chunks so checkpoint pruning does not break a subsequent
diff. Source replacement evicts the prior basis. A session's root CPU render keeps
only this bounded sculpt cache, after successful rendering. Browser agent dispatch
uses that shared core path. One-shot CLI renders and current GPU previews are cold.

Costs report basis construction, changed points/triangles, refit leaves/nodes,
leaf triangle-bound visits, copied payload chunks/elements/root references,
retained chunk-reference copies, and remaining instance-bound vertex scans.
Actual geometry/nested-vector layout is separate from portable byte charges;
allocator overhead, Arc allocation headers, source storage and private BTreeMap
allocation layout are excluded from those layout observations. Point-map entry
counts are reported separately. Snapshot admission, hashing, block reference
comparison, root copies, instance bounds and GPU host packing retain global work.
The high-level operation deliberately checks cold reconstruction before returning
its transaction, adding global work so accepted edits can reopen independently.
Discovery sets are bounded by source/profile caps; the copy-byte preflight applies
to the subsequent geometry-copy phase and includes an allowance for these sets.

Qualification is pending. The existing runtime-chunk baseline had higher observed
whole-process footprint in repeated PBR probes; the cause was not isolated. This
candidate makes no whole-process memory or speed improvement claim.
