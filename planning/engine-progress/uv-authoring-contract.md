# Proposed M10 engine subgate after static Blender acceptance

Proceed with UV authoring because corner attributes, named texture coordinates,
mesh ownership, ordinary commands and CPU/GPU texture rendering already exist.
Additional USD/MaterialX profiles remain future format work, rather than a reason
to defer the first UV-to-painted-asset workflow. This is a proposal, not capability
status or completion of M10. The interactive demonstration remains tied to M09.

Read architecture 17.3, M10 and native geometry/modeling contracts before coding.
Keep corner IDs and explicit source mesh/revision selections. Do not make point
indices public UV identities: seam vertices need distinct per-chart coordinates.
Preserve other named attributes, positions, topology and IDs exactly. UV changes
must invalidate/recompute derived tangent frames through existing evaluation rules.

Candidate numerical profile: bounded LSCM on explicit orientable disk charts cut
along authored stable edge IDs; at least two distinct pinned chart vertices with
explicit UV positions. Reject singular/ill-conditioned solves, degenerate triangles,
flips, chart self-overlaps, unsupported non-manifold fans and work exhaustion.
Use deterministic Rust least-squares computation with measured residual/rank and
scale sensitivity. Do not infer universal overlap freedom from one paper's claim.
Packing is a separate deterministic bounded operation with explicit padding,
resolution, scale policy and pin behavior; do not silently move pinned coordinates
or claim optimal packing. Report actual occupancy and texel scale.

Primary research: Levy, Petitjean, Ray and Maillot, SIGGRAPH 2002,
https://www.cs.jhu.edu/~misha/ReadingSeminar/Papers/Levy02.pdf
The paper separates segmentation, parameterization and packing. Section 2 forms a
least-squares system from per-triangle Cauchy-Riemann residuals, with pinned UVs
removed from the unknowns. Use the mathematical description as a reference; do not
copy an existing C/C++ implementation or introduce a native computational library.

Important integration issue: document::ModelMesh currently invokes modeling::apply
with a noncancelling closure. A new long UV solver must not inherit this behavior.
First establish cancellable preparation, or expose a bounded cancellable computation
that publishes its validated result through ordinary transactions with a fresh
revision check. Preserve idempotency and mutation permissions in both entrypoints.
Review that seam before choosing the public request interface.

Acceptance: analytic triangle/quad/developable strip and bent disk; explicit seams,
pins, multiple UV names, deterministic packing and exact preservation of topology;
negative rank/orientation/overlap/budget cases; pre/mid/late cancellation, stale
selection/revision, permission, retry, undo/recovery; a native/browser authored mesh
receiving a checker texture and rendering after save/reopen; process/solver/asset
resource observations and dependency/provenance audit. Extend into tiled painting,
sculpt locality and retopology in separate reviewable subgates.

Interface design to review next: retain seam and pin intent in a typed immutable
UV layout asset bound to an entity and its authored mesh hash, following existing
geometry/rig asset patterns. Put generated coordinates in the existing named corner
Vec2 attribute. Store constraint and chart provenance in the typed layout rather
than encoding relationships into ad hoc attribute-name strings. Changing the mesh
must explicitly update/clear that binding; do not silently reinterpret stale corner
or edge selections after topology edits. A new snapshot version may be needed.

The standalone `qr.rs` development probe passed three tests: an independently solved
rectangular least-squares problem, pivot permutation, and rank/pre/late cancellation.
This is not a UV implementation, numerical conformance claim or integration gate.
