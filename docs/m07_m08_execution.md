# M07 and M08 execution contract

User request: complete M07 and M08 fully. This ledger does not narrow the canonical
requirements or change completion gates. The initial static PBR slice is pinned
in `evidence/static-pbr/source_manifest.json`; prior evidence stays historical.
Candidate branch: `feat/m07-m08-completion`.

M06, M07 and M08 canonical gates are complete for the named experimental profiles.
The [acceptance index](../evidence/m06-m08/acceptance_index.json) links native/Wasm
numerical and negative tests, seven native workflows, browser OPFS/rendering,
resource evidence, dependency inventory and independent pass comparisons.

| Gate | Completed evidence |
|---|---|
| M06 prerequisite | Modeling workflow, direct/group/field equivalence, correspondence and local/wide edit amplification |
| M07 geometry | Curves, points and anchored guide/child groom; native CPU/Metal and browser Rust/WebGPU recovery/rendering |
| M07 media | Sparse density/emission/scattering, analytic slabs and convergence; native/browser Rust CPU; explicit GPU rejection |
| M07 surfaces | Dielectric/conductor/coat/alpha numeric tests and native/browser render; displaced geometry CPU/Metal comparison |
| M07 imaging | UV bakes, views/passes, D65 color vectors and bilateral-v0 numeric/native/browser workflows |
| M08 time and rigging | Exact rational clips, interpolation, retiming, bind/rest/affine LBS/morph/IK, causal failures and random access |
| M08 rendering | Nine native shutter frames, per-time Metal comparisons and five distinct browser/native frame comparisons |
| Integration | 87 native tests, 77 Wasm tests, all 17 inherited gates, platform/ABI/dependency checks and resource reports |

The first geometry contract preserves polyline, cubic Bezier and rational B-spline
basis, authored control IDs, radius and tilt; tessellated render meshes are
private derived assets with explicit error/work budgets. Point clouds retain
point IDs/radii rather than authored fake polygon topology. Persistent attachment
uses existing entity identity; new typed geometry assets/components use a new
snapshot version and ordinary transactions. Geometry evaluation does not mutate
published snapshots. Caches include source content and evaluation policy.

Tube/sphere tessellation is a named approximation; it does not claim analytic
curve intersection or physical hair scattering. Groom/scattering completion has
its own gate. Spline evaluation and transport will use mathematical references
only; no external computational runtime is admitted. Background/unsupported
geometry must fail explicitly, never be silently removed.

References: [architecture](architecture.md), [milestones](milestones.md),
[static PBR integration ADR](../evidence/static-pbr/accepted_ADRs.md),
[PBRT curve discussion](https://pbr-book.org/4ed/Shapes/Curves),
[volume processes](https://pbr-book.org/4ed/Volume_Scattering/Volume_Scattering_Processes).

## Typed media and groom contracts

Snapshot v3 adds typed curves/points; v4 adds sparse RGB media; v5 adds grooms.
All attachments use revision-checked commands. Sparse occupied cells are addressed
by integer coordinates, with metric voxel sizes, zero background, RGB absorption,
scattering and emission. Empty cells remain absent. Extinction/emission integrate
exactly across ordered ray intervals, including overlaps. Point-light scattering
uses a named bounded midpoint policy; indirect in-scattering is unsupported.
Native and browser CPU use identical Rust transport. GPU requests reject media
explicitly. A per-ray cap bounds overlap expansion and quadrature; failure returns
an error rather than a truncated result.

Groom roots name entity, topology digest and stable triangle corner IDs with
barycentric weights. Guide coordinates use the anchor triangle's local tangent
frame; child length/radius/twist stay procedural until evaluation. Anchor transforms
(including shear) transform the derived sweeps. Position deformation updates roots;
changed topology requires explicit rebinding. Visibility is separate from dependency
availability so hidden anchors can drive visible strands. Strand rendering currently
uses the documented surface sweep material profile, without a physical fiber BSDF.

M08 clip contract: exact rational seconds and retiming; explicit clamp/repeat;
step, linear and cubic Hermite derivatives per second; Euler order retained;
quaternion linear interpolation uses shortest-arc slerp and cubic uses normalized
component Hermite. Pose deltas append T*R*S to authored affine/rest transforms.
Animation sampling never changes the published document. These semantics are
validated by the native and browser character workflow and numerical tests.
