# Integration review

The bounded change crosses imported semantics, typed rigging/animation state,
ordinary transactions, immutable evaluation, frame identity and rendering consumers.
Snapshot-v13 gating protects new direction offsets and per-entity bindings. Omitted
fields preserve prior document/command serialization. Offsets identify stable
point/corner IDs under a named attribute and topology, not runtime vector indices.
Ambiguous shading attributes, missing bindings or partial tangent/sign bindings
are rejected before publication. Direction-only source targets are meaningful;
missing source base semantics and count mismatches fail without import commits.

Additive morphs precede skinning. The existing position summation order remains
unchanged; the new direction path forms the weighted linear matrix explicitly.
Inverse transpose, determinant and normalization have independent closed-form
checks under nonuniform and negative scales. Singular blends and zero directions
return causal errors without modifying source snapshots. Cancellation is tested
at every observed boundary. Point/corner correspondence preserves seam domains.
The default profile still strips stale shading directions; prior image/pass and
receipt bytes prove that old workflows keep their behavior.

Rendering uses the existing shared geometry and shader paths. GPU binary16 texture
storage is a documented approximation independent of direction deformation. A
separate analytic rounding oracle retains strict mapped-direction assertions;
full f32 CPU/GPU radiance comparison remains in force. The derived frame policy
normalizes directions, while the renderer retains tangent orthogonalization and
its deterministic collinear fallback. Later displacement still applies its own
explicit geometric-normal reconstruction policy; authored rest frames are never
silently reused after displacement.

GLB and explicit resources pass the same conversion; browser storage and native
archives preserve complete state. Source-scoped IDs differ across dense/sparse
files, while decoded values and resulting pixels agree. No new runtime dependency,
privileged mutation path or remote service is introduced. No unresolved blocker
remains for this named frame profile.
