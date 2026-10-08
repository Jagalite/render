# Milestones and release gates

M06–M08 now have [named implementation profiles](m07_m08.md) and [complete milestone evidence](../evidence/m06-m08/README.md). Historical M05 Lambertian and [static PBR](static_pbr.md) evidence retain their original scope. External input tracks remain separately qualified in `planning/input_support.json`.

This is a focused extract of the canonical architecture specification. Current implementation status and evidence are in `planning/milestones.json` and `evidence/m00-m04/README.md`; the acceptance criteria below remain the release gates.

## 24. Milestone map and release definitions

### 24.1 Dependency map

```text
M00 -> M01 -> M02 -> M03 -> M04 -> M05 [Agent-rendering alpha]
M02 -> M06                    M05 -> M07
(M06 + M07) -> M08
(M05 + M06 + M08) -> M09 -> M10
(M06 + M07 + M08) -> M11
(M07 + M08 + M09) -> M12 [four independent subtracks]
(M05 + M06) -> M13
(M10 + M11 + M12 + M13) -> M14 [Major-suite baseline]
Early IO, security, verification and provenance run throughout.
```

The diagram expresses major dependencies, not a requirement that all work be serial. Precise dependencies and acceptance items are in `planning/milestones.json`. No calendar estimate is asserted without staffing, velocity and prototype evidence.

### 24.2 Releases

**Foundation preview (M00–M03):** a native document and CPU-rendered scene, accessible through reviewed API contracts. No editing-suite parity claim.

**Agent-rendering alpha (M04–M05):** local native/browser GPU rendering, inspect/branch/preview/commit workflows, small ABI v1 boundary and explicit feature profiles. No Blender runtime.

**Authoring beta (M06–M10):** meaningful native modeling, procedural content, richer rendering, character animation and interactive editing/UV/sculpt workflows. Capability flags still identify unsupported advanced features.

**Major-suite baseline (M11–M14):** representative native workflows across simulation, compositor, drawing, tracking and media, plus interoperability/collaboration hardening. “Major-suite” means published coverage and quality gates, not every Blender feature or existing add-on.

### 24.3 Common definition of done

Each milestone requires working code, documented semantics, positive/negative tests, a reproducible demo, resource measurements, language/dependency compliance, migration considerations, and a capability entry. A schema, screenshot, mock UI, stub, or optional fallback is not completion. An unsupported case must produce a meaningful diagnostic.

Acceptance evidence is saved as machine-readable reports and human-readable release notes. Suggested owners are subsystem roles rather than invented people. Parallel agents work within explicit API/fixture contracts and submit evidence with changes.

## 25. Milestones M00–M05: prove the independent foundation

### M00 — Architecture and Rust-only feasibility spikes

Prove dense/chunked scene storage against a generic ECS candidate; radial handles against a manifold-limited topology candidate; Rust kernel generation on native and browser; reference BVH traversal; crash-safe storage adapters; and the runtime dependency inventory. Record measured results and accept/revise ADRs. Exit only with a nontrivial generated GPU kernel and a WASM core executing tests. No full editor or premature stable ABI.

### M01 — Authoritative document and geometry core

Implement persistent IDs, typed schema, units/time, transforms, assets, instances, canonical polygon meshes, attributes, minimum topology validation, snapshots and branches. Create a scene with shared geometry and a UV seam; rename/reparent without breaking references; preserve negative transforms and loose geometry. Reject invalid indices and attribute lengths. Exit with native/WASM conformance and round-trip native storage fixtures.

### M02 — Transaction/API/ABI alpha and durable jobs

Implement the registry, revision checks, idempotency, structured errors, asset uploads, candidate deltas, durable commits, journal recovery, jobs and events. Publish a small unstable ABI candidate. Test duplicate requests, key/payload mismatch, stale targets, conflicts, disconnects and crash boundaries. Exit with no partial committed state and identical semantics through in-process and wire clients. Do not freeze every domain operation.

### M03 — CPU reference renderer and first external assets

Implement camera/light semantics, triangles/instancing, BVH, surface scattering subset, texture/color handling, direct-light and BSDF sampling, image output and receipts. Import/export selected static glTF/OBJ profiles natively in Rust. Render analytic and constructed scenes without Blender. Test energy/PDF behavior, seams, transforms, intersections and supported round trips. This is an independent renderer, not a Cycles bridge.

### M04 — Portable GPU rendering and browser execution

Implement Rust-authored generated kernels, software GPU traversal, progressive accumulation, raster preview, pass output, device-limit negotiation and resource recovery. Native and browser build the same scene and execute matched test kernels. Compare numerical/statistical behavior to CPU and analytic references. Test cancellation, allocation failure and device loss. Hardware ray acceleration is optional and cannot be required for baseline completion.

### M05 — Agent-rendering product slice and narrow ABI v1

An agent creates a project, branches three variants, modifies only permitted lighting/material properties, previews, renders, inspects passes, chooses a variant and commits. Test budget limits, retries, interrupted streams and conflicting edits. Freeze only the small ABI/transport core proven by independent clients; operations retain their own versions. Publish feature profiles, receipts and local browser export/recovery behavior. This is the first recommended public alpha.

**Implemented profile:** M05 is validated for trusted local clients with the bounded static Lambertian scene importer, restricted variants, native jobs, browser GPU previews/save/recovery and narrow ABI v1. See [the profile](agent_alpha.md) and [evidence](../evidence/m05/README.md). Full input support and later authoring domains retain their own gates.

## 26. Milestones M06–M10: authoring and interactive creation

### M06 — Modeling kernel and procedural geometry

Deliver a coherent modeling set: primitive creation, extrude/inset/bridge, bevel, split/weld/dissolve, plus selected mirror/array/subdivision/solidify and Boolean operations. Add typed field evaluation, node groups and an operation-graph view of modifiers. Every operation documents input restrictions, attribute transfer and topology correspondences. A procedural asset must survive save/reload and direct-command/node execution comparisons. Remeshing and advanced operators can remain explicit sub-capabilities until proven.

### M07 — Broader materials, rendering and geometry types

Add authored curves/points/hair representation, sparse volume transport where supported, richer layered scattering, displacement, baking, render layers/passes and native color-transform profiles. Denoising is Rust-native and explicitly named. Each addition has analytic/numerical/image tests and truthful CPU/GPU support. A hair or volume schema without a usable evaluated/rendered workflow does not satisfy its subgate. Motion-blur authoring integration completes with M08.

### M08 — Animation, rigs and deformation

Implement clips, channels, interpolation, skeletal transforms, constraints/IK, skinning, morph targets, time remapping and sequence rendering. Validate bind/rest behavior, scale/shear, random timeline access and solver diagnostics. Add shutter sampling and supported rigid/deforming motion blur. Produce a short animated character workflow using only native components. Advanced retargeting and additional constraint families require named subgates rather than implied parity.

### M09 — Shared interactive editor

Deliver object/asset inspection, viewport navigation, transforms, material/node editing, timeline controls and mesh editing through the existing API. Add curve/text workflows as their native components mature. Prove that human gestures yield the same accepted deltas as equivalent API operations. Test accessibility, keyboard/text input, scaling, long operations and conflicts. UI responsiveness must not bypass transaction validation.

### M10 — UVs, painting, sculpting and retopology

Deliver UV charts/unwrap/packing, texture paint layers, baking targets, local sculpt brushes, masks, multiresolution and a documented dynamic-topology/remesh profile. Require sparse updates, undo under long gestures, seam preservation, brush locality and recovery tests. Add snapping/retopology workflows using the same spatial queries and topology operations. An interactive demonstration must cover asset creation through textured/sculpted output, not only independent brush widgets.

### Integration checkpoint

Before declaring authoring beta, rerun agent workflows on newly introduced feature families. Ensure procedural edits, rig changes and sculpt operations return meaningful costs and diagnostics. Measure conversion/memory amplification across authored, editing, evaluated and GPU forms. Revise private layouts if costs exceed budgets; do not weaken published data guarantees to hide performance problems.

## 27. Milestones M11–M14: complete the major-suite baseline

### M11 — Simulation families

Use independent subgates for rigid bodies; cloth/soft bodies; particle/hair dynamics; smoke/fire; and liquids/meshing. Each requires physical/numerical tests, bounded jobs, checkpoint invalidation and a rendered end-to-end scene. Timeline seeking and branch changes must not reuse incompatible state. Coarse previews are labeled. Native Rust implementations are mandatory; no C++ solver fallback.

### M12 — Image, drawing, editorial and tracking workspaces

**M12A compositor:** typed image graph, masks, grading, keying, pass composition and tile/global scheduling. **M12B drawing:** timed strokes, fills, layers, mixed 2D/3D rendering and editing. **M12C media:** image/audio sequences, timeline edits, retiming, transitions, sample-accurate synchronization and approved Rust codec profiles. **M12D tracking:** feature observations, lens models, camera solve and reconstruction with diagnostics. These subtracks can ship independently but all declared major-suite commitments need demonstrated workflows.

### M13 — Interoperability, collaboration and extension hardening

Expand native format profiles, including selected `.blend`/USD/MaterialX data only where tested. Add loss reports and source-preserving round trips. Harden multi-client branches, reviewed merge rules, asset sharing and isolated plugin capabilities. Test protocol downgrade/upgrade, tenant boundaries, hostile input and old-client support. No universal Python-add-on or arbitrary-file compatibility claim. Interoperability begins earlier; this gate expands and hardens it rather than delaying all IO until the end.

### M14 — Major-suite conformance and production readiness

Publish the feature matrix by platform and backend, compatibility corpus, security/dependency audit, crash-recovery results, performance profiles and known limitations. Run representative workflows across every committed family from authoring through final artifacts. Fix severe data-loss, correctness and stability issues before release. Provide migration/export procedures, user documentation and maintainer ownership.

M14 does not mean every Blender command has been cloned. It means the project has a credible independent creative suite with measurable improvements, native implementations and honest boundaries. Advanced remaining features become explicit roadmap entries, not hidden debt behind a “100% compatible” label.

### Program controls

Do not assign a completion date until M00 establishes the most consequential risks and actual team capacity. Re-estimate after M05 and authoring beta. Staffing needs span geometry, graphics/numerics, platform systems, UX and verification; agent coding throughput does not replace those review responsibilities. Limit parallel work-in-progress around unstable contracts.

## 28. Feature coverage and compatibility policy

| Feature family | First useful native gate | Broader workflow gate | Principal qualification |
|---|---|---|---|
| Scene/asset assembly | M01 | M05/M13 | Explicit layers, instances and stable identity |
| API/ABI and agent jobs | M02 | M05 | Narrow ABI stable; operation versions independent |
| CPU/GPU rendering | M03/M04 | M07/M08 | Backend capability and approximation profiles |
| Mesh modeling/modifiers | M06 | M09/M10 | Named operators; degeneracy and topology policies |
| Procedural nodes/fields | M06 | M08/M11 | Domain-specific execution, state and solver nodes |
| Curves, points, hair, volumes | M07 | M09/M11 | Representation, evaluation and rendering tested separately |
| Materials, textures, baking | M03 | M07/M10 | No implicit full Blender/MaterialX equivalence |
| Animation and rigging | M08 | M09/M14 | Clips, constraints and deformation profiles |
| UVs, painting, sculpting | M10 | M14 | Multiresolution and dynamic-topology limits explicit |
| Simulation | M11 | M14 | Independent solver-family subgates |
| Compositing | M12A | M14 | Typed color/alpha and temporal semantics |
| 2D/3D drawing | M12B | M14 | Stroke intent remains native data |
| Video/audio sequencing | M12C | M14 | Codec coverage distinct from timeline capability |
| Motion tracking | M12D | M14 | Camera solve distinct from loading footage |
| Native file interchange | M03 | M13 | Format/version/feature-level loss reporting |
| Collaboration/plugins | M05 basics | M13 | No automatic arbitrary topology merging |

The detailed machine-readable matrix records tested profiles and remaining planned features. Update status per tested capability; do not mark a feature supported because its data type exists.

### 28.1 Compatibility axes

For each feature, record native authoring, evaluation, final render, preview, human editor, import and export. Separate native desktop and browser-local support. Record minimum resources, known numerical limitations and supported operation/schema versions. A native import may preserve unsupported source data read-only without implementing its behavior.

### 28.2 Definition of an improvement release

A release qualifies as improved when its evidence demonstrates the intended workflow gains: explicit operations, safer experimentation, fewer unintended changes, lower incremental costs or more reliable integration. Do not claim superior rendering speed, memory use, topology robustness or user productivity across all workloads without comparative measurements. A strong architecture provides mechanisms for improvement; the benchmark and user workflow establish whether they work.

## 29. Expanded breadth and depth roadmap (authorized 2026-09-30)

The original sections above and M00–M14 acceptance history remain unchanged. The user requested a broader, deeper independent creative suite after reviewing the Blender gaps. M15–M46 below are new requirements, all initially **planned**. They do not turn existing bounded profiles into general support, close M09/M10/M12/M14, or claim Blender parity. The machine-readable requirements are in `planning/milestones.json`.

The comparison taxonomy is the [Blender 5.2 LTS manual](https://docs.blender.org/manual/en/5.2/index.html), reviewed 2026-09-30. It guides breadth; native implementations, file/version fidelity and model accuracy each need their own evidence. Exact CAD, universal multiphysics, arbitrary historical files/add-ons and unrestricted executable metadata are not implied.

### 29.1 Sequence and evidence rules

Start with M15 general region/selection authoring, then M16 construction/cutting and M18 reusable deformation foundations; M17 robust bevel/Boolean proceeds with independent geometry review. Scene assets, procedural graphs, materials and rigs form parallel dependency-ready tracks. Painting/sculpting, physics, compositing/drawing/media and tracking follow their prerequisites. M46 is the integration/release gate and explicitly retains all original open requirements. Milestone dependencies constrain completion, not useful early investigation of an unblocked subgate.

No complete milestone is inferred from one successful operator. Each stage owns all criteria below, a capability inventory, versioned numerical/format contracts and reproducible evidence. Codec feasibility, format licensing and solver review are explicit risks; a blocker leaves the requirement open. New physical models require reviewed equations, units and numerical contracts before implementation.

- Every capability has a finite versioned input/semantic inventory and explicit unsupported cases before completion.
- Record authoring, evaluation, final render, preview, human UI, import and export independently, with native OS/hardware GPU/browser evidence separate.
- Use passed/failed/blocked/not-run states; schemas, test counts, screenshots and small demos do not substitute for full criteria.
- Require public transactions, native Rust computational implementations, independent positive/negative/property/numerical fixtures and cancellation/stale/recovery tests.
- Pin representative workload tiers and accuracy/latency/resource thresholds before final measurement; larger caps alone do not establish scalability.
- Preserve old profile bytes, fixtures and receipts; version new behavior and document migration/losses.
- Resolve codec/format/numerical feasibility through explicit reviewed decisions; unresolved mandatory capabilities remain open rather than silently excluded.
- No calendar promise or unmeasured Blender/Cycles/Eevee superiority/parity percentage.

### 29.2 New milestone requirements

#### M15 — General polygon selection and region authoring

**Status:** in progress (partial implementation; acceptance remains open). **Dependencies:** M02, M06. **Owner role:** Geometry + topology + verification.

Baseline at roadmap creation: Box-first authoring, single-face extrusion and strictly convex planar inset; topology selections are mostly caller-supplied IDs.

Current increment: stable source-pinned selection, connected/open/closed region extrusion and planar pre-event metric inset have scoped native/WASM and public workflow evidence. Explicit source-edge realization now preserves parallel surface/wire identities and payloads, including closed/wire/MAX-ID cases. Broader inset depth and combined qualification remain open. See `evidence/modeling-expansion/acceptance.json`.

Acceptance:

1. Provide stable-ID point, edge, face, connected-region, boundary and loop selection queries with deterministic ordering and explicit non-manifold ambiguity diagnostics.
2. Extrude connected non-coplanar and concave face regions, regions with holes and disjoint components without duplicate internal walls; preserve existing single-face operation semantics.
3. Implement region inset with explicit offset, concavity, collision and collapse policies; reject invalid results transactionally rather than silently replacing the operation.
4. Carry point, edge, face and corner attributes through the supported operations with explicit created-element defaults, seam/material rules and complete correspondence receipts.
5. Verify rotated and nonconvex fixtures, boundary orientation, area/volume oracles, selection-order invariance, seeded property cases and malformed/stale selections on native and WASM.
6. Public commands and modifier nodes produce equivalent results; a multi-region modeled asset saves/reopens, renders and supports cancellation, retry and undo without changing unselected state.

End-to-end result: Model a stepped hard-surface asset from connected and holed polygon patches through public transactions and procedural modifiers.

Evidence directory: `evidence/m15/`; required inventory, conformance, workflow, negative/recovery, resources and independent review. All requirements remain open until their evidence passes.

#### M16 — Mesh construction, cutting and topology repair

**Status:** in progress (partial implementation; acceptance remains open). **Dependencies:** M15. **Owner role:** Geometry + numerics.

Baseline at roadmap creation: Few primitives, equal-length boundary bridge, coplanar dissolve, and no general cut/fill/repair workflow.

Current increment: Eight primitive families, uncapped plane bisect, previewed distance welding, coherent winding and structural boundary inspection have scoped evidence. Explicit triangulation and triangle corner-normal reconstruction also have recorded native/WASM/public/editor workflows. Certified outward orientation of closed embedded authored-triangle shells now passes 25 scoped native and 25 Node-WASM cases, strict scoped lint, genus-one analytic persistence/render/GLB workflow and bounded public work-exhaustion checks; 68 predecessor geometry cases pass both targets. Outward Studio controls pass seven focused tests including IME-safe loss consent; actual Linux native-window inspect/prepare/commit/undo/redo/save/reopen/reinspect workflow passes. Exact-planar authored-polygon outward orientation now has 54 scoped native and 54 Node-WASM checks, independent rational geometry oracles, bounded resource fixtures, Snapshot 26/schema compatibility, and explicit consent for changed canonical-cover interpolation plus separate frame-layer loss. Polygon Studio controls and narrow report layouts pass scoped tests; the actual Linux native-window payload fixture passes inspect/prepare/cancel/commit/undo/redo/save/reopen/reinspect. Stored UV/color retention is distinct from interpolated appearance preservation. Five held review cases remain unpassed. General cleanup, knife/loop/ring cuts, fill, unequal bridging, full release qualification and M15 dependencies remain open. The polygon c389b9e candidate subsequently passed 53 aggregate stages with source-verified executor recovery, including 873 native and 660 Node-WASM tests; five held cases and three browser gates leave that aggregate incomplete. A separate read-only cleanup inspection increment passes 34 scoped native and 34 Node-WASM checks, bounded resources, 178 Studio package tests with one exact held exclusion, strict lint and Windows compile checks. Its actual Linux native workflow diagnoses seven source faces and 24 edges even when viewport triangulation rejects, exercises exact scopes and explicit canonical NotChecked status, preserves saved document bytes, and clears stale selections/reports on reopen. That C0 increment remains diagnostic-only; the bounded action qualification below does not complete C0 or C1–C4. See `evidence/m16/cleanup-inspection/` for the bounded cleanup inspector and `evidence/m16/polygon-outward-public/` and `evidence/m16/polygon-outward/studio/runtime/native-live-final.json` for the polygon checkpoint and `evidence/m16/outward-shells/` for the preceding triangle checkpoint and `evidence/modeling-expansion/acceptance.json` for earlier increment evidence.

Bounded C1a qualification now covers explicit rank-deficient face deletion, same-explicit-boundary duplicate consolidation and exact-zero original source-wire deletion through public source-bound action/loss review and ordinary transactions. Core passes 56 native and 56 Node-WASM cases, strict scoped lint and eight resource tiers; a later test-reporting-only parity supplement compares 23 complete previews and 15 receipts totaling 208,747 exact bytes per target without changing production or adding distinct tests. Studio passes 196 package tests with one exact held exclusion, strict all-target lint and fresh Windows compile-only. The actual Linux native workflow passes default blocking, independent face/edge scopes, separate payload/frame consent, Prepare/Cancel/reinspect/Commit, actual four-face/14-point to one-face/14-point selection transfer and Undo/Redo, save/reopen with reset consent/history and a fresh admitted no-op. An independent saved comparison passes 72 checks, preserving the first operation's unchanged second binding; a separate explicitly chosen second-entity repair then restores viewport scene drawing and saves/reopens independently, with 68 further positive saved-document checks that preserve the first verification artifacts. At that C1a checkpoint, C1b parallel-wire/orphan/diagnosed-face-discard work, C2–C4, durable baked-loss provenance, the five held cases, browser/Windows-runtime gates and the original remaining M16/M15 requirements remain open. See [native action qualification](native_cleanup_apply.md), [actual workflow](../evidence/m16/cleanup-apply/studio/runtime/qualification-001/native-workflow.md), [first saved-document verification](../evidence/m16/cleanup-apply/studio/runtime/qualification-001/saved-document-verification/REPORT.md), [second saved-document verification](../evidence/m16/cleanup-apply/studio/runtime/qualification-001/second-entity-saved-document-verification/REPORT.md) and [exact byte parity](../evidence/m16/cleanup-apply/canonical-byte-parity/execution-summary.md).

Bounded C1b qualification now covers explicit disjoint groups of unused immediate-source wires with identical stable endpoint pairs. Each group retains its own smallest ID and exact survivor payload; unlisted parallel edges, zero-length survivors, isolated points and all face/point/corner records remain. Fresh final-source qualification passes 77 native and 77 Node-WASM tests, 104 complete C1b records totaling 225,177 identical bytes, unchanged C1a/C0 parity, strict core lint and 139 fixed resource runs (110 admitted, 22 budget rejections and seven cancellations). Studio passes 221 package tests with one exact held exclusion, strict lint and Windows compile-only. The actual Linux native workflow exercises separate payload/frame consent, prepared Cancel, Commit, genuine 11-point/one-face selection transfer, Undo/Redo and save/reopen/reset. Independent first and second saved-document comparisons pass 128 and 225 positive checks; the explicitly repaired second binding restores observed viewport drawing. Its prepared selection transfer reported unavailable/stale, consistent with the intentional empty-selection path, and is not counted as a nonempty transfer pass. Orphan pruning, diagnosed-face discard, broader C1/C2–C4, durable baked-loss provenance, the five held cases, browser/Windows runtime and all other original M16/M15 requirements remain open. See [source-wire contract](modeling_source_wires.md), [native workflow](native_source_wires.md) and [qualification evidence](../evidence/m16/source-wire-consolidation/QUALIFICATION.md).

Bounded C1c qualification now covers three explicit immediate-source deletion roles: unused Edges, source-isolated Points, and Points made isolated by exactly the requested Edge deletions. Unlisted records remain, including newly isolated endpoints. Core passes 102 native and 102 Node-WASM tests (25 new C1c and 77 predecessor cases), 137 complete C1c records totaling 318,468 identical bytes, unchanged C1b/C1a/C0 parity and strict scoped lint. The 214 fixed resource processes match 154 admitted, 54 budget-rejected and six cancelled outcomes. Studio passes 248 package tests with one exact held exclusion, strict lint and Windows compile-only. The actual Linux native workflow reviews three separate roles and all ordinary/global frame losses, exercises prepared Cancel, Commit, six-to-four Point selection transfer with Face500 retained, Undo/Redo and save/reopen/reset. An independent comparison passes 151 positive saved-document checks and preserves the unmodified second binding. Only this bounded C1c operation is complete. At that C1c checkpoint, strict newly-orphaned-edge pruning after the same face action remained open; the separately reviewed C1d qualification below now covers only its bounded exact-rank-face action. Diagnosed-face discard, general cleanup, C2–C4, durable baked-loss provenance, all five held cases and standalone/equivalent probes, browser/Windows runtime and all remaining original M15/M16/release requirements remain open. See [loose-record contract](modeling_loose_elements.md), [native workflow](native_loose_elements.md) and [qualification evidence](../evidence/m16/loose-element-deletion/QUALIFICATION.md).

Bounded C1d qualification now covers selected exact-rank-zero/one whole-face deletion and three explicit same-action dependency policies: preserve all Edges/Points, delete only newly unused Edges, or additionally delete Points newly isolated by those actual Edge removals. Original source wires and isolated Points remain; retained payload bits, full causal witnesses and original layer identities are preserved. Final-source core qualification passes 130 native and 130 Node-WASM tests (28 new C1d and 102 predecessors), 270 complete C1d records totaling 1,752,717 identical bytes per target, unchanged predecessor byte anchors and strict scoped lint. All 329 fixed resource processes match 206 admissions, 115 budget rejections and eight cancellations, with independent source/output hash, cost, phase and callback checks. Studio passes 279 tests with one exact held exclusion, strict lint and Windows compile-only. The actual Linux native workflow reviews all three policies, separate frame consent and exact losses, transfers 11 selected Points and three Faces to seven Points and one Face, preserves original Wire30/31 and isolated Point8, and exercises Prepare/Cancel, fresh Commit, Undo/Redo, separate Save and same-revision Reopen/reset/no-op. The positive saved-document comparison passes 169 frozen checks and preserves the independent valid asset and original source. Only this bounded C1d operation is complete. Diagnosed-face discard, general cleanup, C2–C4, durable baked-loss provenance, all five held cases and standalone/equivalent probes, browser/Windows runtime and the remaining original M15/M16/release requirements remain open. See [rank-face contract](modeling_rank_face_deletion.md), [native workflow](native_rank_orphans.md) and [qualification evidence](../evidence/m16/rank-face-deletion/QUALIFICATION.md).

Acceptance:

1. Add plane/grid, circle/disk, cylinder/cone, sphere and torus primitives with parameter, winding, seam, material and budget contracts.
2. Implement knife/bisect with edge/face intersections, loop/ring cuts and fill/bridge operations for unequal boundary sampling, holes and mixed polygon meshes.
3. Provide weld-by-distance, duplicate/degenerate cleanup, normal/winding repair and boundary inspection with previewed losses and stable selection transfer.
4. Exercise touching cuts, cuts through vertices, tiny and large coordinate scales, repeated edits, open/non-manifold inputs and attribute conflicts against independent topology/geometry oracles.
5. Produce and repair a multi-primitive asset with holes through API and editor-ready operations, retaining a reproducible edit history and explicit global/local resource costs.

End-to-end result: Construct, cut, bridge and repair a non-box asset without external modeling software.

Evidence directory: `evidence/m16/`; required inventory, conformance, workflow, negative/recovery, resources and independent review. All requirements remain open until their evidence passes.

#### M17 — General bevel and mesh Boolean operations

**Status:** planned. **Dependencies:** M16. **Owner role:** Robust geometry + topology review.

Current limit: Bevel and Boolean accept only un-attributed axis-aligned boxes.

Acceptance:

1. Bevel selected edges and vertices on rotated, nonconvex mixed-polygon meshes, with segment/profile controls, boundary rules and overlapping-bevel handling.
2. Perform union, intersection and difference on independently authored closed meshes beyond boxes, including curved tessellations and nonconvex operands.
3. Publish robust orientation/intersection predicates and policies for coplanar, touching, coincident, thin and near-degenerate geometry; unsupported topology must fail before publication.
4. Preserve or explicitly resolve UVs, materials, colors, normals and source correspondence on cut/beveled faces; do not invent durable identity through ambiguous topology.
5. Pass set/volume and manifold invariants, operand-order relations, scale/rotation metamorphic tests, adversarial cancellation and resource limits, plus a multi-operation hard-surface workflow.

End-to-end result: Build a rounded mechanical asset with non-axis-aligned Boolean cuts and retained material/UV boundaries.

Evidence directory: `evidence/m17/`; required inventory, conformance, workflow, negative/recovery, resources and independent review. All requirements remain open until their evidence passes.

#### M18 — Iterative subdivision and deformation modifiers

**Status:** planned. **Dependencies:** M15, M16. **Owner role:** Geometry + animation + numerics.

Current limit: One manifold quad-cage refinement is triangulated; mirror/array/solidify and deformation policies remain narrow.

Acceptance:

1. Retain reusable subdivision control cages and support repeated refinement of mixed polygons with boundary/crease rules and limit-surface error evidence.
2. Expand mirror/array/solidify to documented welding, material, thickness, transform and instancing policies with collision/degeneracy diagnostics.
3. Implement shrinkwrap, lattice and curve deformation plus a reviewed inventory of common deformation modifier families, with explicit order and coordinate-space semantics.
4. Preserve attributes and stable references or report correspondence loss; compare modifier-stack execution with equivalent direct operations at animated times.
5. Author an organic deformed asset, change its cage non-destructively, save/reload and render at multiple refinement levels with measured memory/work scaling.

End-to-end result: Create an organic asset using a reusable cage and several composable deformation modifiers.

Evidence directory: `evidence/m18/`; required inventory, conformance, workflow, negative/recovery, resources and independent review. All requirements remain open until their evidence passes.

#### M19 — Scene assembly, linked assets and variants

**Status:** planned. **Dependencies:** M02, M13. **Owner role:** Document + assets + editor.

Current limit: Shared typed assets and layers exist; richer collection/view-layer/library authoring and asset management remain incomplete.

Acceptance:

1. Implement collections, view layers, hierarchical visibility, material slots and variants with explicit composition order and stable references.
2. Support linked/instanced asset editing, reviewed local overrides, library relinking and missing-dependency recovery without accidental realization.
3. Persist selections, asset metadata and reusable presets; enumerate dependency and ownership changes before cross-document updates.
4. Verify nested/negative transforms, visibility overrides, cycles, conflicting revisions and stale links through API, storage and render/export paths.
5. Assemble a reusable multi-asset scene, branch variants, update one shared source and prove intended propagation plus unrelated-instance preservation at increasing instance counts.

End-to-end result: Assemble and revise a product scene from linked assets and independently selectable view layers.

Evidence directory: `evidence/m19/`; required inventory, conformance, workflow, negative/recovery, resources and independent review. All requirements remain open until their evidence passes.

#### M20 — Curves, surfaces, text and implicit geometry authoring

**Status:** planned. **Dependencies:** M16, M18, M19. **Owner role:** Geometry families + text + graphics.

Current limit: Curve/point/hair types mostly realize sweep meshes; general parametric surfaces, editable shaped text and implicit authoring are absent.

Acceptance:

1. Author and edit Bezier/NURBS curves and parametric surfaces with handles, knots, weights, trims and tolerance-controlled tessellation.
2. Implement editable text, native font shaping and curve conversion with declared font/script coverage and glyph/layout conformance fixtures.
3. Provide point, implicit/metaball-style, sparse paged field and groom representations with explicit conversions and no hidden dense fallback.
4. Preserve representation, attributes and identity across save/reload; every conversion reports approximation and correspondence losses.
5. Demonstrate animated deformation, preview and final rendering for each representation and measure tessellation/conversion costs at increasing declared sizes.

End-to-end result: Build a mixed curve/surface/text/implicit scene, edit its native representations and export declared evaluated forms.

Evidence directory: `evidence/m20/`; required inventory, conformance, workflow, negative/recovery, resources and independent review. All requirements remain open until their evidence passes.

#### M21 — Domain-aware procedural geometry and field breadth

**Status:** planned. **Dependencies:** M15, M19, M20. **Owner role:** Procedural geometry + API.

Current limit: Fields expose a small point scalar/vector vocabulary and the editor exposes a root graph subset.

Acceptance:

1. Publish a finite node-family catalog covering typed domain adaptation, named attributes, selections, joins/separation, instances/realization, distribution/scatter and geometry conversion.
2. Implement mesh, curve, point and volume sampling/spatial-query nodes and deterministic seeded distributions with explicit interpolation and coordinate semantics.
3. Support reusable nested groups, typed interfaces and inspectable intermediate geometry without flattening authored group identity.
4. Compare node and independently assembled direct operations, reject cycles/type/domain errors and bound retained geometry plus expanded execution work.
5. Create a nested procedural mixed-geometry environment, change upstream parameters, save/reload and render while retaining instance sharing and deterministic results.

End-to-end result: Author a reusable nested procedural asset with scatter, selection and mixed geometry outputs.

Evidence directory: `evidence/m21/`; required inventory, conformance, workflow, negative/recovery, resources and independent review. All requirements remain open until their evidence passes.

#### M22 — Stateful geometry nodes and interactive node tools

**Status:** planned. **Dependencies:** M21, M11. **Owner role:** Procedural evaluation + editor + simulation.

Current limit: No general repeat/simulation zones or nested node-tool/gizmo workflow.

Acceptance:

1. Add repeat and stateful simulation zones with explicit state schemas, frame boundaries, random seeds, invalidation and checkpoint lineage.
2. Expose node tools, typed parameters, spatial gizmos and nested graph editing through public operations and versioned interfaces.
3. Inspect intermediate values/geometry and causal errors without mutating authored state or evaluating unsafe arbitrary code.
4. Test random-time seek, branch/parameter changes, pause/resume, cancellation, lost checkpoints and bounded growth against independent stepwise execution.
5. Build, interactively edit and recover a stateful procedural animation through API and actual editor hosts; report each platform runtime separately.

End-to-end result: Create an editable stateful procedural shot and recover it after interruption with identical declared state.

Evidence directory: `evidence/m22/`; required inventory, conformance, workflow, negative/recovery, resources and independent review. All requirements remain open until their evidence passes.

#### M23 — Typed shader graphs and texture authoring

**Status:** planned. **Dependencies:** M07, M19, M21. **Owner role:** Shading + GPU + editor.

Current limit: Materials are bounded scalar/PBR profiles without a general shader-node canvas or broad texture/coordinate vocabulary.

Acceptance:

1. Implement a typed shader graph with reusable groups, sockets and a reviewed texture/procedural/math/vector/coordinate node catalog.
2. Support image channels, mapping, derivatives/filter footprints and attribute-driven inputs with explicit color-role and normal/tangent semantics.
3. Compile/evaluate supported graphs across declared CPU/GPU profiles, preserve source identity and give precise unsupported-node/backend diagnostics.
4. Verify graph/direct equivalence, derivative/filtering oracle cases, seams, aliasing, invalid cycles/types and resource/cancellation behavior.
5. Author and edit layered textured materials in a node UI, save/reload, render multiple lights/views and export or report exact per-format losses.

End-to-end result: Build a textured material library and reuse editable grouped materials in a multi-object scene.

Evidence directory: `evidence/m23/`; required inventory, conformance, workflow, negative/recovery, resources and independent review. All requirements remain open until their evidence passes.

#### M24 — Advanced surface, fiber and layered scattering

**Status:** planned. **Dependencies:** M23. **Owner role:** Scattering + transport + numerical review.

Current limit: Ideal glass, approximate conductor/single coat and bounded principled profiles do not cover rough/nested/subsurface/fiber transport.

Acceptance:

1. Implement reviewed rough transmission, nested dielectric/media boundaries, richer principled/layered interactions, subsurface and fiber scattering models.
2. Version every physical/numerical model and its sampling/PDF, units, approximations and allowed combinations; do not alias unsupported combinations to a simpler BSDF.
3. Pass independent energy/reciprocity/limiting/PDF tests and statistical mean/variance/convergence image cases including difficult glass, coated conductors, skin-like and hair scenes.
4. Verify textured and animated use, CPU/GPU model intersections, material changes and mixed-model cache transitions without altering original frozen profiles.
5. Publish quality/time/memory measurements on pinned scenes/settings/hardware; no Cycles/Eevee equivalence or speed claim without matched evidence.

End-to-end result: Render a material stress scene containing rough glass, layered surfaces, subsurface material and groom fibers.

Evidence directory: `evidence/m24/`; required inventory, conformance, workflow, negative/recovery, resources and independent review. All requirements remain open until their evidence passes.

#### M25 — Camera, lighting, volume and motion transport depth

**Status:** planned. **Dependencies:** M20, M24, M08. **Owner role:** Rendering + geometry + animation.

Current limit: Camera/light coverage, indirect volume transport and complex hair/volume/deforming shutter combinations are limited.

Acceptance:

1. Add declared lens/projection/depth-of-field camera models and area/environment/light-sampling families with validated PDFs and coordinate/unit rules.
2. Support indirect heterogeneous volume scattering, emission and boundary transitions plus native groom/curve transport in documented surface-volume combinations.
3. Integrate rigid, deforming, groom and volume motion with explicit shutter intervals, interpolation and temporal sampling/reproducibility policies.
4. Pass analytic light/camera/media limits, independent multiple-scattering images, temporal oracles, random-frame sequence consistency and failure/cancellation tests.
5. Render a lit animated groom/volume shot with depth of field and motion blur and report actual backend capability intersections and resource scaling.

End-to-end result: Produce an animated groom-and-volume shot with physically reviewed camera and light sampling.

Evidence directory: `evidence/m25/`; required inventory, conformance, workflow, negative/recovery, resources and independent review. All requirements remain open until their evidence passes.

#### M26 — Render quality controls and measured backend scaling

**Status:** planned. **Dependencies:** M24, M25. **Owner role:** Graphics + performance + verification.

Current limit: Uniform dicing, fixed sampling, simple bilateral filtering and software-adapter evidence do not establish production render quality or speed.

Acceptance:

1. Implement adaptive displacement/tessellation, adaptive sampling and reviewed spatial/temporal denoising with explicit bias and temporal stability limits.
2. Provide render layers, AOVs and object/material matte channels with stable identity and correct alpha/time/color metadata.
3. Measure CPU, actual native hardware GPU and actual browser WebGPU separately across small/medium/large scenes, including acceleration, upload/cache costs and memory/VRAM.
4. Test device loss, allocation pressure, cancellation/resume, shader/model combinations and repeated renders without authored-state corruption.
5. Publish matched quality-versus-time curves and residual/error images against independent or high-sample references; comparison versions/settings/hardware and uncertainty are mandatory.

End-to-end result: Deliver reproducible preview/final renders with quality controls and measured cross-backend tradeoffs.

Evidence directory: `evidence/m26/`; required inventory, conformance, workflow, negative/recovery, resources and independent review. All requirements remain open until their evidence passes.

#### M27 — HDR color management and image interchange

**Status:** planned. **Dependencies:** M23. **Owner role:** Color science + imaging + IO.

Current limit: Linear-sRGB/D65 matrices, Reinhard and basic image formats lack broad HDR/config/LUT/ICC support.

Acceptance:

1. Implement tagged HDR/wide-gamut scene/display transforms and reviewed LUT/ICC/config subsets in Rust with named format/version coverage.
2. Support image channel/metadata preservation and approved native HDR/image codecs, keeping decode, working space, display view and output transform distinct.
3. Pass independent transform vectors, negative/HDR/out-of-gamut values, premultiplication edges, round-trip and invalid-profile cases.
4. Ensure render, compositor, paint, image UI and exports use equivalent declared color/alpha semantics with explicit unsupported transforms.
5. Produce a multi-display/HDR image workflow with measured accuracy, image-size scaling, cache invalidation and recovery.

End-to-end result: Import tagged HDR imagery, grade/display it consistently and export channels with verified metadata.

Evidence directory: `evidence/m27/`; required inventory, conformance, workflow, negative/recovery, resources and independent review. All requirements remain open until their evidence passes.

#### M28 — Rig construction, constraints and deformation depth

**Status:** planned. **Dependencies:** M18, M08. **Owner role:** Animation + numerics + editor.

Current limit: CopyPosition, TwoBoneIk, LBS and basic morphs omit broad rigs, bind/weight tools and deformation variants.

Acceptance:

1. Provide editable hierarchical rigs, rest/bind transforms, weight tools and automatic binding with explicit skeleton/mesh/topology compatibility policies.
2. Implement reviewed constraint and multi-joint IK families, limits, poles and typed drivers with causal cycle/unreachable/singular diagnostics.
3. Add dual-quaternion or equivalent reviewed deformation variants, corrective morphs and facial-control workflows while retaining LBS behavior.
4. Validate numerical deformation under negative/nonuniform scale, shear, bind transforms, animated weights, unreachable constraints and topology changes using independent fixtures.
5. Construct a character rig through public operations, edit weights/poses, save/reload, animate and render/export with an explicit fidelity report.

End-to-end result: Rig an independently modeled character with editable weights, constraints and corrective deformation.

Evidence directory: `evidence/m28/`; required inventory, conformance, workflow, negative/recovery, resources and independent review. All requirements remain open until their evidence passes.

#### M29 — Animation editing, blending and retargeting

**Status:** planned. **Dependencies:** M28, M19. **Owner role:** Animation + editor + interchange.

Current limit: Rational clips exist, but dope-sheet/graph/pose/NLA authoring, action reuse and retargeting are incomplete.

Acceptance:

1. Implement dope-sheet, curve/handle/tangent/extrapolation, pose and nonlinear action editing with exact rational timing and public operations.
2. Support layered/additive clip blending, reusable actions, time remapping and reviewed skeleton retargeting semantics.
3. Verify interpolation/blend oracles, tangent discontinuities, random/out-of-order evaluation, constraint ordering and shutter behavior.
4. An original character sequence is authored, blended, retargeted, scrubbed, saved/reloaded, rendered and exported through independent API and actual UI workflows.
5. Measure animation evaluation and interaction latency at declared rig/track/frame tiers; test cancellation, conflicting edits and undo/recovery.

End-to-end result: Animate and retarget a character performance using editable graph and nonlinear action workflows.

Evidence directory: `evidence/m29/`; required inventory, conformance, workflow, negative/recovery, resources and independent review. All requirements remain open until their evidence passes.

#### M30 — General UVs, painting and material-aware baking

**Status:** planned. **Dependencies:** M15, M18, M23, M27, M28. **Owner role:** UV + paint + imaging + devices.

Current limit: Constrained disk charts, rectangular atlases, center dabs and a 16-recipe discontinuity cap limit complete painting workflows.

Acceptance:

1. Implement general chart topology, distortion/texel-density-aware unwrap/packing, seam/pin/island editing, named sets and multiple UDIM tiles.
2. Paint full projected footprints across seams and materials with occlusion, brush masks/stencils/textures, layers/blending and image/vertex/color/weight targets.
3. Replace accidental retained-recipe/sample cliffs with an explicit scalable chunk/checkpoint/storage policy; long seam-crossing strokes must recover and undo as one gesture within negotiated resources.
4. Bake normal/AO/light/material channels using reviewed cage/ray, sampling, coordinate and color-role policies rather than center-sample surrogates.
5. Pass independent coverage/distortion/color/bake oracles, overlap/degeneracy diagnostics and pressure/tilt input tests on actual supported devices separately from normalized unit tests.
6. Complete a multi-material UDIM asset through edit, bake, save/reopen, render and export with measured brush/packing costs at increasing resolutions.

End-to-end result: Texture and bake a seam-crossing multi-material asset using full-footprint layered paint and multiple tiles.

Evidence directory: `evidence/m30/`; required inventory, conformance, workflow, negative/recovery, resources and independent review. All requirements remain open until their evidence passes.

#### M31 — Production sculpt, remeshing and retopology

**Status:** planned. **Dependencies:** M16, M17, M18, M30. **Owner role:** Sculpt + topology + spatial queries.

Current limit: Triangle-centroid refinement, selected split/lossy clustering and static nearest-point snapping remain small bounded profiles.

Acceptance:

1. Add reviewed sculpt brush families, masks/face sets, symmetry and local updates with explicit surface/volume and pressure semantics.
2. Implement reusable multiresolution displacement and level transitions with measured silhouette/detail reconstruction error.
3. Provide adaptive dynamic topology and voxel/isotropic remeshing with reviewed numerical, attribute and correspondence-loss contracts.
4. Support continuous surface picking, retopology edge/face creation, snapping and shrinkwrap with editable topology and projected attributes.
5. Complete an organic asset sculpt-to-remesh-to-retopology-to-UV-to-bake workflow with independent geometry/detail tests and stable unrelated state.
6. Measure local work, memory and p95 gesture latency at increasing declared sizes; long high-resolution gestures must cancel, recover and undo without replaying invalid input.

End-to-end result: Create a detailed organic asset and a textured retopologized production mesh through one recoverable workflow.

Evidence directory: `evidence/m31/`; required inventory, conformance, workflow, negative/recovery, resources and independent review. All requirements remain open until their evidence passes.

#### M32 — Rotational rigid bodies, collision and joints

**Status:** planned. **Dependencies:** M16, M11. **Owner role:** Physics + collision + numerical review.

Current limit: Existing rigid bodies are frictionless spheres/floor and do not establish general contact dynamics.

Acceptance:

1. Publish and implement a versioned rotational rigid-body model with inertia, convex/mesh colliders, broad/narrow phase and continuous-collision policy.
2. Add friction/restitution contact stacks and reviewed joint/motor families with units, stabilization and solver-order contracts.
3. Pass analytic free motion/impact, momentum and declared dissipation, stack/contact penetration and timestep/refinement tests on independently authored scenes.
4. Handle animated colliders, thin/fast bodies, degenerate shapes, quota failures and cancellation without corrupting caches or authored revisions.
5. Author, simulate, branch, seek/resume/reload and render a mechanically constrained scene with distinct preview/final profiles and measured scale tiers.

End-to-end result: Simulate and render a jointed mechanical scene with frictional contact and animated colliders.

Evidence directory: `evidence/m32/`; required inventory, conformance, workflow, negative/recovery, resources and independent review. All requirements remain open until their evidence passes.

#### M33 — Cloth and soft-body collision robustness

**Status:** planned. **Dependencies:** M16, M11, M32. **Owner role:** Deformable physics + collision + animation.

Current limit: Small XPBD membranes/tetrahedra lack cloth self-collision and robust general deforming collider handling.

Acceptance:

1. Publish reviewed stretch, bend, volume and damping/material models with units and timestep/iteration semantics before implementation.
2. Implement deforming-mesh collision and cloth self-collision, including vertex-face/edge-edge contacts, thickness and friction policies.
3. Pass independent hanging/folding/impact fixtures, resolution/time refinement and declared energy/dissipation bounds; self-contact regressions bound residual penetration relative to configured thickness.
4. Test fast motion, folded initial states, pinned/animated boundaries, tangled contacts, invalid topology and solver failure with explicit diagnostics and no silent explosion acceptance.
5. Create a clothed/deformable animated shot through authoring, simulation, interruption/recovery, random seek, save/reopen and rendering at increasing mesh sizes.

End-to-end result: Drape and animate a self-colliding garment against a moving character with recoverable simulation state.

Evidence directory: `evidence/m33/`; required inventory, conformance, workflow, negative/recovery, resources and independent review. All requirements remain open until their evidence passes.

#### M34 — Particle, groom and surface-effect dynamics

**Status:** planned. **Dependencies:** M20, M22, M32. **Owner role:** Particles + groom + simulation nodes.

Current limit: Fixed inertial particles and simple strands lack general emitters, lifecycle, groom collision and Dynamic Paint workflows.

Acceptance:

1. Implement timed emitters, birth/death/lifecycle, force fields and collision with deterministic seeding and instance/source semantics.
2. Provide editable groom dynamics and strand/mesh/self-collision profiles with constraint, friction and parameter-invalidating cache rules.
3. Add a typed surface-effect/Dynamic Paint workflow connecting moving emitters/colliders to persistent paint or geometry fields.
4. Declare each supported coupling and its synchronization/energy-exchange policy; unsupported multiphysics combinations must diagnose rather than silently decouple.
5. Pass limiting/conservation/dissipation and seeded-repeatability tests, animated-collider and hostile budget/recovery tests, plus authored/rendered particle, groom and surface-effect scenes.

End-to-end result: Animate a groom and emitted particles, drive a retained surface effect, then scrub/recover/render the shot.

Evidence directory: `evidence/m34/`; required inventory, conformance, workflow, negative/recovery, resources and independent review. All requirements remain open until their evidence passes.

#### M35 — Pressure-projected smoke and free-surface liquids

**Status:** planned. **Dependencies:** M20, M32. **Owner role:** Fluid numerics + sparse storage + meshing.

Current limit: Prescribed-flow sparse smoke and weakly compressible SPH/floor meshing do not establish general fluid behavior.

Acceptance:

1. Publish separate native-Rust numerical contracts for incompressible pressure-projected smoke/fire and free-surface liquid transport with obstacle boundaries.
2. Implement advection, pressure, buoyancy/combustion and sparse boundary handling for smoke/fire, plus liquid free surfaces, moving obstacles and verified render meshing.
3. Track turbulence, surface tension and multiphase/coupled extensions as named subprofiles with separate equations, units and evidence; do not imply universal multiphysics.
4. Pass divergence/volume/mass and declared energy/dissipation bounds, analytic limiting cases, spatial/time convergence and independent obstacle/free-surface scenes.
5. Verify sparse-resource behavior, boundary conditions, hostile parameters, branch/cache invalidation, cancellation/resume and random seeking across authored-to-rendered workflows.

End-to-end result: Produce obstacle-driven smoke/fire and liquid shots from native simulation through checkpointed renderable fields/meshes.

Evidence directory: `evidence/m35/`; required inventory, conformance, workflow, negative/recovery, resources and independent review. All requirements remain open until their evidence passes.

#### M36 — Compositing graphs and temporal shot finishing

**Status:** planned. **Dependencies:** M23, M26, M27. **Owner role:** Compositing + imaging + scheduling.

Current limit: Current source/mask/grade/key/over/add/translation/box-blur/global-normalize graph is too small for broad shot finishing.

Acceptance:

1. Expand typed masks/rotoscoping/keying, affine transforms/filtering, lens/distortion/effects and color-math/grade node families with reusable groups.
2. Consume render layers/AOVs/mattes and preserve HDR/color/alpha/channel metadata through exported results.
3. Add named temporal dependency and optical-flow-style processing profiles with cache invalidation, frame-neighbor and failure semantics.
4. Verify independent filter/edge/key/alpha oracles, ROI-versus-full-frame agreement, temporal changes, invalid graphs and cancellation/recovery.
5. Complete render-passes-plus-footage to keyed/layered/graded shot through an editable graph UI/API workflow and measure tile/global/GPU scheduling separately.

End-to-end result: Finish a keyed composited shot from footage and native render passes using reusable editable groups.

Evidence directory: `evidence/m36/`; required inventory, conformance, workflow, negative/recovery, resources and independent review. All requirements remain open until their evidence passes.

#### M37 — Editable mixed 2D and 3D drawing animation

**Status:** planned. **Dependencies:** M20, M27, M29. **Owner role:** Drawing + animation + editor.

Current limit: Retained timed strokes/fills and four onion times lack full point/brush/timeline/modifier authoring.

Acceptance:

1. Provide stroke/point selection and editing, brush/material/fill rules, layer masks and surface attachment with stable stroke identity.
2. Implement keyframed/multiframe drawing animation, interpolation, onion skins and reviewed drawing modifier/effect stacks.
3. Preserve camera/depth/occlusion and screen/world/surface coordinate semantics in mixed 2D/3D scenes.
4. Pass independent curve/fill/alpha/occlusion/timing tests, identity/correspondence tests and interrupted gesture/undo/save-reopen scenarios.
5. Author, edit, animate, render and export an original mixed 2D/3D scene through a full drawing timeline and public operations with measured resource tiers.

End-to-end result: Create an editable animated drawing integrated with an animated 3D scene.

Evidence directory: `evidence/m37/`; required inventory, conformance, workflow, negative/recovery, resources and independent review. All requirements remain open until their evidence passes.

#### M38 — Compressed audiovisual media and editorial workflows

**Status:** planned. **Dependencies:** M27, M29, M36, M37. **Owner role:** Media codecs + audio + timeline.

Current limit: PPM/PFM and PCM16 WAV sequences are not compressed video/container, streaming playback or long-form editing support.

Acceptance:

1. Publish a reviewed native-Rust codec/container feasibility, licensing and version matrix; implement selected compressed video and audio decode/encode profiles without FFmpeg or OS codec fallback.
2. Support accurate seek, variable-frame-rate/timebase mapping, proxy/relink and streaming playback/scrubbing with explicit decode/reorder/error policies.
3. Provide editable tracks, transitions/keyframes, waveform/scopes, channel mixing/fades/effects and editable scene/compositor/drawing sources.
4. Import an independently authored compressed clip and audio, cut/retime/mix, save/reopen and export a compressed result that an independent decoder validates.
5. Measure frame/sample mapping, synchronization drift, playback latency and memory/storage on declared durations/resolutions; test corrupt/truncated media and interrupted decode/export recovery.

End-to-end result: Edit and deliver a synchronized compressed film from footage, audio and native scene sources.

Evidence directory: `evidence/m38/`; required inventory, conformance, workflow, negative/recovery, resources and independent review. All requirements remain open until their evidence passes.

#### M39 — Robust tracking, calibration and reconstruction

**Status:** planned. **Dependencies:** M20, M29, M36, M38. **Owner role:** Computer vision + numerics + compositing.

Current limit: Integer 5x5 SSD and nearby calibrated known-3D-control poses are not general tracking, SfM or bundle adjustment.

Acceptance:

1. Implement robust multiscale/subpixel tracking with scale/rotation support, manual corrections, occlusion handling and reacquisition diagnostics.
2. Add lens calibration/distortion and named camera/object/planar/tripod solve profiles, unknown-point reconstruction and joint bundle adjustment with outlier policies.
3. Provide scene scale/origin alignment and compositor integration; rolling-shutter behavior stays explicitly unsupported until its separately tested profile exists.
4. Validate feature, reprojection, pose and reconstruction errors against synthetic ground truth and independently authored real clips, including poor parallax, repeated patterns and textureless failure cases.
5. Complete footage-to-tracks-to-solve-to-inserted-3D-to-composite workflow, preserving editable observations and cache/recovery semantics with measured optimization costs.

End-to-end result: Track a moving shot, reconstruct unknown points and integrate a 3D insertion into the finished footage.

Evidence directory: `evidence/m39/`; required inventory, conformance, workflow, negative/recovery, resources and independent review. All requirements remain open until their evidence passes.

#### M40 — Complete shared editor workflows and device qualification

**Status:** planned. **Dependencies:** M09, M15, M19, M21, M23, M29, M30, M31, M36, M37, M38, M39, M22, M32, M33, M34, M35. **Owner role:** Editor + accessibility + platforms.

Current limit: Native/window and shared browser implementations exist, but many editing workflows and actual device/browser qualification remain incomplete.

Acceptance:

1. Deliver coherent object/outliner/asset, transform/mesh/UV/paint/sculpt/rig/animation and shader/geometry/compositor/drawing/media/tracking workspaces including nested graphs and curve handles.
2. Support contextual selection, gizmos/snapping, searchable commands, asset browser/presets, full undo/redo and clear unsupported/conflict feedback through public operations only.
3. Provide simulation authoring, forces/emitters/colliders, solver settings, cache/bake/seek/recovery controls and stateful-node tools/gizmos in the shared editor; complete artist workflows must not depend on hidden API-only physics controls.
4. Independent task scripts complete representative scenes without hidden API intervention, and equivalent gestures/API requests yield equivalent accepted changes.
5. Exercise modal/IME/focus/keyboard/pointer/tablet/screen-reader/HiDPI behavior on actual declared native platforms and real browser DOM/Worker/WebGPU/storage/navigation.
6. Measure p95/p99 interaction, brush, graph and scrub latency at fixed workload tiers, including cold paths, long gestures, cancellation and recovery; retained-state cost must be visible.
7. Record physical/browser/OS cases as passed, failed, blocked or not-run independently; synthetic accessibility trees and software GPUs do not satisfy actual device/runtime gates.

End-to-end result: Complete unfamiliar-user hard-surface, character and audiovisual tasks entirely in the shared editor.

Evidence directory: `evidence/m40/`; required inventory, conformance, workflow, negative/recovery, resources and independent review. All requirements remain open until their evidence passes.

#### M41 — Authored interchange and native format breadth

**Status:** planned. **Dependencies:** M19, M20, M23, M29, M27, M38. **Owner role:** Interchange + security + provenance.

Current limit: OBJ/glTF subsets and evaluated GLB exports omit broad authored animation/skin/morph and other interchange families.

Acceptance:

1. Implement authored glTF animation/skin/morph and reviewed extension import/export in addition to evaluated mesh export, with per-feature round-trip evidence.
2. Stage native-Rust mesh/curve/point/volume/image/audio/video adapters, including individual PLY/STL/SVG, OpenVDB-style sparse-volume, USD, Alembic, MaterialX and FBX version/semantic profiles with feasibility/licensing decisions explicit.
3. For every required format track import, editable authoring, evaluation/render, evaluated export, authored round-trip and opaque preservation separately; unresolved adapters remain open requirements.
4. Use independently created external fixtures, external validators where available and structural/numeric/render comparisons; preserve or report exact material, timing, topology and attribute losses.
5. Reject hostile paths/dependencies, invalid sizes/versions and executable metadata; verify missing-asset relink, interruption, migration and source-preserving recovery.

End-to-end result: Exchange a textured animated asset and multi-domain shot with independent external tools using documented native adapters.

Evidence directory: `evidence/m41/`; required inventory, conformance, workflow, negative/recovery, resources and independent review. All requirements remain open until their evidence passes.

#### M42 — Broader native Blender file semantics and edited export

**Status:** planned. **Dependencies:** M17, M21, M23, M28, M29, M41. **Owner role:** Native formats + geometry/shading/animation integration.

Current limit: Only uncompressed Blender2.93 static small scenes are evaluated; source-byte extraction is not edited .blend export.

Acceptance:

1. Publish a pinned .blend version/feature corpus and implement independently parsed data-block/link/library semantics for broader selected versions and compressed input.
2. Evaluate supported materials/nodes, modifiers, rigs, animation and relevant scene data natively, recording unsupported simulation/add-on behavior rather than importing an inert placeholder as support.
3. Implement a separate edited authored-export profile with correct data-block references and explicit losses; original-byte extraction must retain its distinct name.
4. Verify authored/evaluated/rendered round trips against independently produced external fixtures and reference outputs, including linked libraries, missing dependencies and version migration.
5. Enforce bounded untrusted parsing and no embedded Python/add-on execution; arbitrary historic files and third-party add-ons remain outside any unqualified compatibility claim.

End-to-end result: Open, edit, animate/render and export supported nontrivial .blend scenes with explicit compatibility reports.

Evidence directory: `evidence/m42/`; required inventory, conformance, workflow, negative/recovery, resources and independent review. All requirements remain open until their evidence passes.

#### M43 — Discoverable API, SDKs and isolated extensions

**Status:** planned. **Dependencies:** M13, M21, M23. **Owner role:** API + sandbox security + developer experience.

Current limit: Transactional CLI/ABI is strong, but domain discovery, bulk/streaming clients and a real isolated extension runtime are incomplete.

Acceptance:

1. Expose schemas, capability/version negotiation, bulk/streaming jobs, inspectable costs and cancellation across every implemented domain.
2. Provide maintained independent SDK/client examples and migration tools that author complete workflows without private editor APIs.
3. Implement typed extension registration, permissions, compatibility and an independently reviewed isolated runtime with explicit fuel/memory/data capabilities.
4. Pass malicious extension escape/resource tests, malformed inputs, denied-capability attempts and upgrade/downgrade compatibility corpora.
5. Verify retries, disconnects, stale conflicts, crash/recovery and cancellation preserve transaction guarantees; no Blender bpy/add-on runtime compatibility is implied.

End-to-end result: Use independent clients and a constrained extension to author and inspect a full creative workflow safely.

Evidence directory: `evidence/m43/`; required inventory, conformance, workflow, negative/recovery, resources and independent review. All requirements remain open until their evidence passes.

#### M44 — Authenticated collaboration and shared asset services

**Status:** planned. **Dependencies:** M19, M43. **Owner role:** Security + collaboration + storage.

Current limit: Trusted local scoped sessions and conservative merges are not an authenticated deployed collaboration service.

Acceptance:

1. Design and implement authenticated identities, grants/revocation, project/asset ownership and durable shared-service boundaries with explicit threat and deployment models.
2. Support reviewed multi-client branch/merge/conflict workflows and immutable asset sharing without silently overwriting topology or authority changes.
3. Pass cross-tenant/authentication/revocation, malicious-client, reconnect/replay and concurrent-update tests with external security review.
4. Verify standalone native and browser-local work remains usable without any required remote service, and local-to-shared migration preserves provenance.
5. Demonstrate a recoverable multi-client author/review/merge workflow in an explicitly authorized test deployment; production deployment remains a separate user/release approval.

End-to-end result: Two authorized clients collaborate on branches and shared assets with visible conflicts and revocation.

Evidence directory: `evidence/m44/`; required inventory, conformance, workflow, negative/recovery, resources and independent review. All requirements remain open until their evidence passes.

#### M45 — Production asset pipeline, scale and operability

**Status:** planned. **Dependencies:** M19, M26, M29, M31, M35, M38, M41, M43. **Owner role:** Integration + performance + operations.

Current limit: Small bounded fixtures and local resource receipts do not establish large-scene, long-form or operational readiness.

Acceptance:

1. Provide reusable asset/preset catalogs, dependency relink/packaging, migration and batch/headless pipeline tools with explicit provenance.
2. Define realistic fixed small/medium/large workload tiers for scenes, geometry, textures, rigs, simulations and media before measuring or tuning.
3. Measure cold load/save, edit/pick/brush/graph/scrub, render quality/time, RAM/VRAM/storage and incremental/wide edit amplification with attributable receipts.
4. Test storage exhaustion, process crashes, corrupt roots, interrupted exports, device loss and long jobs, including recovery after resource pressure.
5. Publish operational diagnostics, reproducible command/UI workflows, support/version policies, migration/export procedures and subsystem ownership candidates without inventing accepted maintainers.

End-to-end result: Run and recover a multi-domain production-sized asset/shot pipeline with inspectable resource and provenance reports.

Evidence directory: `evidence/m45/`; required inventory, conformance, workflow, negative/recovery, resources and independent review. All requirements remain open until their evidence passes.

#### M46 — Expanded suite qualification and measured comparison

**Status:** planned. **Dependencies:** M09, M10, M12, M14, M15, M16, M17, M18, M19, M20, M21, M22, M23, M24, M25, M26, M27, M28, M29, M30, M31, M32, M33, M34, M35, M36, M37, M38, M39, M40, M41, M42, M43, M44, M45. **Owner role:** Release + all subsystem maintainers.

Current limit: Historical bounded milestone completion does not imply Blender breadth, unrestricted workflows or production parity.

Acceptance:

1. Pass end-to-end hard-surface product, textured sculpted asset, animated character, groom/volume/physics shot, mixed2D/3D scene, tracked composite and synchronized edited-film workflows.
2. Complete the frozen original M09/M10/M12/M14 runtime/dependency/ownership gates and every new milestone criterion; blockers and unrun environments remain visible until satisfied.
3. Publish a fixed reference inventory with per-capability native authoring/evaluation/final-render/preview/UI/import/export axes and per-platform/backend outcomes.
4. Compare matched Blender reference tasks only with exact versions, scenes, settings and hardware, reporting quality/fidelity, latency, throughput, memory and uncertainty; never infer feature-percent from milestone/test totals.
5. Pass cross-domain property/fuzz/regression/numerical tests, dependency/provenance and security reviews, resource/recovery qualification and compatibility/migration corpora against exact release source.
6. Named people accept maintainer/support and release ownership; sign-off, packaging and any production publication occur only after explicit approval and successful required gates.

End-to-end result: Qualify a coherent independent creative suite through the complete frozen workflow and comparison corpus.

Evidence directory: `evidence/m46/`; required inventory, conformance, workflow, negative/recovery, resources and independent review. All requirements remain open until their evidence passes.

### 29.3 Coverage cross-reference

| Comparison family | New milestones |
|---|---|
| Scene/asset assembly | M19, M45 |
| Geometry representations | M20 |
| General modeling/modifiers | M15, M16, M17, M18 |
| Procedural geometry | M21, M22 |
| Materials/lighting/rendering | M23, M24, M25, M26 |
| Color/imaging | M27 |
| Animation/rigging | M28, M29 |
| UV/texture/vertex/weight paint/baking | M30 |
| Sculpt/retopology | M31 |
| Simulation and surface effects | M32, M33, M34, M35, M22 |
| Compositing | M36 |
| Mixed2D/3D drawing | M37 |
| Compressed video/audio | M38 |
| Tracking/reconstruction | M39 |
| Editor/accessibility/platforms | M40 |
| Interchange/.blend | M41, M42 |
| API/extensions/collaboration/ecosystem | M43, M44, M45 |
| Production comparison and release | M46 |

Actual device/browser/platform and human sign-off gates cannot be replaced by compilation or synthetic tests. Source and evidence are committed locally while GitHub publication is access-blocked; this does not change any engineering acceptance criterion.


Bounded C1e qualification now covers source-pinned explicit whole-Face deletion for exact nonplanarity, named positive planar defects, and direct Point-cycle/different-Edge or coordinate-cycle/different-Point pairs with an explicit retained witness. Every Point and Edge remains; no implicit keeper, payload transfer or orphan pruning occurs. Core passes 155 native and 155 Node-Wasm tests (25 new C1e and 130 predecessors), with 189 complete C1e records totaling 806,906 identical bytes per target, unchanged predecessor anchors and strict lint. The 252 ordinary resource cases match 198 admissions and 54 budget rejections; five separate cancellation cases pass, comprising four exact callback cancellations and one late-threshold completion. Studio passes 310 tests with one exact held exclusion, strict lint and Windows compile-only. The actual Linux native workflow authors four explicit reasons from current exact facts, reviews Reject blockers and separate whole-frame Drop consent, prepares/cancels/reprepares/commits, transfers selected Faces20/30/90 to90 while retaining Points10/40/160, and completes Undo/Redo and separate Save/Reopen. The frozen saved-document comparison passes245 checks, preserving original source and the independent valid asset. Reopen clears transient selection, requests, facts, history and consent. Only this bounded C1e operation is complete; broader general cleanup, C2–C4, durable baked-loss provenance, all five held cases and standalone/equivalent probes, browser/Windows runtime and remaining original M15/M16/release requirements remain open. See [diagnosed-face contract](modeling_diagnosed_face_discard.md), [native workflow](native_diagnosed_faces.md) and [qualification evidence](../evidence/m16/diagnosed-face-discard/QUALIFICATION.md).


## 30. Full Blender 5.2.2 audit acceptance ledger (7 October 2026)

The long-term target is 100% parity against a published, versioned and evolving Blender capability inventory. The complete captured source-audit gap list is now tracked in the maintained [full acceptance roadmap](blender_parity_roadmap.md) and canonical [machine-readable matrix](../planning/blender_parity_matrix.json): 104 open semantic requirements, all 16 audited domains plus NPR, and all eight explicit audit refinements. The audit is a source-grounded starting inventory, not an exhaustive enumeration of every Blender behavior.

Each matrix row is additional scope owned by an existing M15–M46 milestone, with explicit prerequisite profiles, subsystem owner, concrete acceptance, seven independent authoring/evaluation/final-render/preview/UI/import/edited-export axes and required fixture/resource/recovery/platform/review reports. Every owning milestone links its exact row IDs in `planning/milestones.json`. M46 incorporates this acceptance ledger alongside all unchanged original release gates; passing a historical bounded milestone cannot satisfy broader rows by implication.

Blender baseline: 5.2.2 commit `d13f752e3b9c4f8c261cda552b1021f8bcc0382c`; qualified Render baseline: `7fff667deb967728c56d539b48361323502143aa`. The separately implemented M19-A collection/view-layer candidate is not retroactively part of that audit baseline and does not close all M19 scope. M19 remains dependency-ready from M02/M13 without an artificial C2 dependency.

The matrix explicitly includes bundles/lists, grid/SDF/advection, sound-driven geometry, rotational/collision/friction XPBD nodes, state/cache/tool semantics, compositor groups in Sequencer, interactive-preview fidelity, Freestyle-style lines/toon/drawing integration, HDR display, and an explicit bpy/RNA/operator compatibility program separate from native SDKs and migration. Rust-only, no-Blender/Cycles-runtime and independent implementation/provenance boundaries remain binding.

All M00–M46 IDs, statuses, prior dependencies/criteria/exclusions and historical evidence remain unchanged. The new ledger is planned scope, not runtime qualification; existing held tests/probes, browser gates and remote-publication restrictions remain in force. No milestone, test, node or code count is a parity percentage. New findings and future Blender releases require explicit inventory versions and retained history.


## 31. Post-audit M19 implementation progress (7 October 2026)

The historical M19 entry remains planned and its acceptance criteria are unchanged.
The additive [M19 implementation progress record](m19_implementation_progress.md)
now links completed bounded A–E core increments, one generic CLI/API process
workflow and four isolated native components to exact commits and evidence.
The canonical [104-row matrix](../planning/blender_parity_matrix.json) carries
separate implementation_progress notes on affected scene, editor, API and pipeline
requirements. All full-row qualification statuses and seven-axis results remain
not_run; this records real implementation progress without promoting full parity.

Evidence through source commit `eb202ff6e61732e7f83af5488c6c7de2cbbb0774` includes
an earlier 44-test core/slot/GLB run, one successful native process workflow,
36 isolated component/controller tests and seven synthetic linked-mesh lifecycle
tests. These are distinct revision-pinned runs, not a combined cutoff-tree
qualification or a completion percentage. The linked profile is static, cached
and one-hop; the UI checks do not run full App::show or actual OS input.

The later typed-property controls complete another bounded existing-API UI gap;
the earlier 30-test component run remains historical evidence. A separately
designed one-hop cross-document CLI workflow is now in progress, with no completed
interface or qualification claim at this record's cutoff.
Full M19, general/nested libraries, all platform/resource/security/release gaps,
reserved unintegrated snapshot 32 and all existing execution holds remain open.


## 32. M19 checkpoint CLI and collection hierarchy progress (7 October 2026)

The second additive [M19 progress record](m19_implementation_progress.md),
`m19-post-audit-progress.2`, advances the recorded source cutoff to
`748b82997f7cf5584c5843433be1d101ac35d9dc`. The prior eleven-increment checkpoint
above remains historical; thirteen bounded increments are now recorded.

The trusted-local linked-checkpoint CLI is committed at
`6eb44c2aaaa72704fdf4a1b00baf490f7584aa8f` (evidence whitespace cleanup at
`9c8027ac5aa0ea20da4142511f055976aa3b5d15`). Three new actual-process tests,
the existing one-process M19 regression and a saved-input replay pass. It captures
two explicit read-only checkpoint files and writes a fresh no-clobber destination;
it is not in-place project refresh or remote source authentication.

Existing-collection parent/root controls are committed at the cutoff. Five new
tests extend the isolated four-target native component run to 41 tests: 12
collections, 15 catalog, 7 variants and 7 slots. Scoped checks and independent
reviews report no remaining blocker for either bounded increment. A single debug
test-process resource observation does not qualify production scale or latency.

Next bounded implementation is local native linked-cache override/detach controls
over existing public transactions, subject to a small explicit UI contract.
Separately, a combined variant/view/slot scene with shared-source refresh remains
an integration-evidence gap across existing core operations. General/nested
libraries, broader migration/presets, actual full-editor/platform, production
resource, security and original release gates remain open. All 47 historical
milestone statuses, 104 full-parity requirements and seven full-row axes are
unchanged; no M19 or parity completion is claimed.


## 33. M19 cached-link controls and combined core evidence (7 October 2026)

The third additive [M19 progress record](m19_implementation_progress.md),
`m19-post-audit-progress.3`, advances the source cutoff to
`63701eb1c87fa9c106a90c75ae738e1891c96a83`. The preceding thirteen-increment
checkpoint remains historical. Fifteen bounded implementation/evidence increments
are now recorded; an evidence-only increment is not a new engine capability.

Commit `44d13ae9f15ec5cc5de0bd064d505f7ad562ddba` adds one focused synthetic core
workflow combining source refresh, variants/views, explicit slot replacement,
local overrides, negative nested transforms and unrelated-object preservation.
The full unrelated-object mask, visible hero coverage, native recovery and cached
CPU/GLB checks pass. This closes that small core integration gap only; native
recovered-image equality is not an independent renderer or Blender oracle.

Commit `63701eb1c87fa9c106a90c75ae738e1891c96a83` adds native destination-local
cached-link controls. The five named component targets pass 51 tests, followed by
10 strengthened linked-control tests with unchanged production code. A real CLI
checkpoint loads through Studio; Set/Clear and acknowledged whole-link Detach
use public candidates, reset on same-byte reopen and retain slot-pin validation.
Both increments have scoped checks and independent reviews with no blockers.

Next evidence work is narrowly scoped full headless App::show M19 input routing;
next small implementation is existing-core variant-material Set/Clear. Neither
requires general-library or schema expansion. Full combined UI/CLI tasks,
physical OS/browser/device, reference, resource, security and release gates remain
open. All 47 historical milestones, 104 requirements and full-row axes stay intact.


## 34. M19 meshless app routes and variant-material progress (7 October 2026)

Progress format `m19-post-audit-progress.4` records seventeen bounded increments
through `95643c3f461d1fdefac7589b599518615620dc24`; the prior fifteen-increment
record is retained at `ea746724bfc32d676c7675219042fe8ba0010db5`.

Commit `fae3f84e9cc468ed21c11d5a5ee06594be296102` adds four passing tests through
actual headless App::show with synthetic IME/keyboard/pointer batches. The declared
meshless Commit/Cancel, focus, first-frame close-modal and fresh-file toolbar
save/reopen routes are now checked. This is not physical OS input or mesh-bearing,
all-dialog, general-app or artist-workflow qualification.

The cutoff commit adds retained-material Set/Clear through existing PutVariant.
Fifteen variant-component tests and 59 named M19 component regressions pass,
separately from the four app tests. Candidate CPU output matches a direct-material
oracle; native/CPU/GLB persistence, other-pixel preservation and current slot/UV
rejections pass. Independent reviews report no blockers within both scopes.

The new evidence also exposes an open diagnostic: evaluated GLB bakes selected
appearance but lacks variant-specific disclosure that native definitions/order
are lost. That narrow reporting fix precedes proposed per-object view controls.
A combined source-refresh/variant/view/slot actual CLI workflow is separate next
evidence work. General-library expansion is not the automatic next step.
All 47 historical milestones, 104 requirements and full-row axes remain unchanged;
full M19, reference, resource, platform/security and original release gates stay open.


## 35. M19 disclosure, composed CLI and object-view progress (7 October 2026)

Progress format `m19-post-audit-progress.5` records twenty bounded increments
through `bc48cd4e759181914f93b02fd56564a50fb84c48`; the prior seventeen-increment
record remains at `7d644a9305c73a50a21c2ed0c95db25588153b9c`.

Commit `a85d20ddf23bbce1d3049cf9995ae9a92cdd2a23` fixes the named native variant-loss
diagnostic and adds analogous collection/view/catalog disclosures. Ten core export
and nineteen named native/app tests pass. A fresh report is true while the original
false report remains unchanged; GLB bytes/profile/consent are unchanged.

Commit `fc93d1e6bbe53f95f4c8f498bc9aa953e8181ac1` closes the stated tiny composed CLI
gap with one test using 73 actual processes. Five complete Snapshots, three PFMs,
lower GLB and native recovery match the stated oracles. Actual CLI object passes
show hero coverage growing from 107 to 298 pixels with an exact unrelated 15-pixel
mask. These are engine-fixture consistency checks, not independent render fidelity.

Commit `bc48cd4e759181914f93b02fd56564a50fb84c48` adds selected-object view Hide/Show/
Clear; seven new cases extend the named component run to 66 tests. Editing versus
active views, inheritance and collection/transform gates remain explicit.
Independent scoped reviews report no blockers for all three increments.
The unchanged four meshless App::show tests and scoped Clippy also pass on this
source; evidence-only commit `3175786b2c82aef5178ca0c87d8942a102d0180c` records
that rerun without adding an increment or expanding its qualification.

Routine control expansion stops as the default recommendation. Native explicit
source-checkpoint refresh/relink is a deliberate next design boundary, requiring
shared public candidate preparation and no authority from metadata; another
milestone may instead take priority. General-library, physical/platform, reference,
resource/security and original release gaps remain open. All 47 historical
milestones, 104 requirements and full-row qualification axes stay unchanged.


## 36. M19 native captured-source checkpoint (7 October 2026)

Progress format `m19-post-audit-progress.6` records twenty-one bounded increments
through `bf778fa1a684a77345310e6ac03f2eb5f7dbe56a`. The prior twenty-increment
record remains at `9dd53233156866f3658557f33b69529ef896dda4`.

The native source design is now implemented within a static one-hop profile:
explicit immutable checkpoint capture, complete plan inspection, all-target
acknowledgement and the exact shared-builder Request staged through ordinary
Studio candidates. Refresh preserves source identity; Relink changes it explicitly.
No source is written/watched/fetched, no committed Host Document is transplanted,
and recorded IDs/labels confer no live authority or ownership. Required slot
rebinds are disclosed and rejected by this component rather than inferred.

Forty-five named tests pass: 8 core builder, 13 new native source, 7 core-link,
3 linked-CLI, 10 cached-control and 4 meshless App::show tests. Scoped core/Studio/
Host Clippy, formatting and independent review pass. The old 66-component run
remains historical; no new aggregate or physical/runtime qualification is implied.
Cached native/CPU/GLB recovery is exact and has nonzero target/override coverage.

Full M19 remains partial and in progress while its historical planned status and
all 104 full-row qualification states remain unchanged. Native Create, source
hierarchy/general/nested libraries, automatic recovery, catalogs/presets, broader
migration, resource/security and physical/OS/browser gates remain open. Next work
is a read-only hierarchy-instancing/general-library contract assessment, not
implementation. Dependencies and all original release/held gates stay intact.


## 37. M19 local-static hierarchy-reference checkpoint (7 October 2026)

Progress format `m19-post-audit-progress.7` records twenty-two bounded increments
through `35498845bf14a426519e059ae6ccf5c255f76fa1`. The prior twenty-one-increment
ledger remains at `200a158c2fd13129ee9a0f405987c67ed076b83d`.

Optional snapshot 38 adds local collection hierarchy references from authored
meshless placements. Typed evaluated occurrence keys distinguish owner and source
without copying authored entities. Source visibility gates occurrences; admitted
member transform/material replacements and inherited static slots feed shared
CPU geometry and occurrence-aware GLB. A new evaluate_occurrences route exposes
structured occurrence counts/keys; old evaluate rejects active instances.

Twenty core tests, two host tests including 27 real CLI calls and CPU-only packing
admission, and two exactly named legacy regressions pass. Scoped Clippy/format/diff
and independent review pass; Studio and Wasm checks are compilation only. Existing
interactive editor, agent/product/paint/sequence and GPU consumers explicitly
reject this active profile, including all-hidden cases. No native/UI qualification
is inherited from earlier M19 controls.

The profile is local/static with visible source exemplars. Captured multi-entity
libraries, linked/nested instance sources, template-only visibility, animation,
geometry replacements, native editing and resource/security/runtime qualification
remain open or unsupported. Captured hierarchy integration is a possible design
direction only, not approved implementation. Full M19 remains partial; all original
47 milestone records, 104 full-row states, dependencies and execution holds remain
unchanged.


## 38. M19 captured static-library checkpoint (7 October 2026)

Progress format `m19-post-audit-progress.8` records twenty-three bounded increments
through `a17950700a39e501874bfbdde9ea9aa5e2e68d55`; the prior twenty-two-increment
ledger remains at `f9274f71e14fe22241ba3e526f3fef0f7704eb49`.

Optional snapshot 39 retains immutable namespaced static hierarchy captures,
source revision/closure pins and shared mesh/image dependencies. Source collection/
view/variant appearance is frozen once; captured source IDs/materials do not collide
with destination namespaces or create synthetic authored children. Public core/
Host/checkpoint CLI Create/Refresh/Relink preserves explicit replacement intent;
Relink requires complete maps even through ordinary Document preparation.

Twenty-five new core tests, one new CLI workflow using 108 real processes and
22 unchanged local-instance regressions pass: 48 tests total. A fresh saved fixture
replays byte-exactly with unchanged inputs. Scoped checks/review pass; Studio/Wasm
are compile-only. Source-free cached CPU/GLB/native recovery and texture/ID-collision
oracles do not establish interactive native, device or general-library parity.

Active local/captured documents still reject native/agent/GPU occurrence consumers.
One-hop static sources exclude nested/linked instances; automatic discovery/refresh,
geometry overrides, broad migration/remote authority and runtime/resource/security
qualification remain open. Native owner-level CPU preview is only a possible future
assessment. Full M19 stays partial; all original 47 milestone fields/statuses,
104 full-row states, dependencies and execution holds are unchanged.


## 39. M19 explicit native CPU owner-preview checkpoint (7 October 2026)

Progress format `m19-post-audit-progress.9` records twenty-four bounded increments
through `622fef7e60598b86d5d86d1b11d1f956fb681d11`; the prior twenty-three-increment
ledger remains at `827b55e517486967078a26122a9fd15d4853979b`.

The separate `native-static-owner-preview-v1` mode admits globally static polygon/
meshless instance/capture documents under declared workload bounds. Source-free
CPU Image and first-sample owner masks support owner selection; only placements
permit Rename/complete affine Transform candidates with ordinary Commit/Cancel
and native save/reopen. Direct objects are read-only. Frame/draft/job/input scopes
reject stale or reserved actions without exposing source/component editing.

Nine new core and nine headless Studio tests pass, plus four separately reviewed
unchanged meshless App-route regressions. Scoped lint/format/review pass. This named
native exception replaces blanket rejection only within its profile; legacy
interactive/source/library/material/agent/product/GPU paths remain unavailable.
Small tests do not qualify physical windows/OS/browser/devices or resource/security
thresholds. An excluded resource-input proposal was removed before execution with
no runnable artifact or replacement. Build disk exhaustion and scoped authorized
regenerable-cache recovery are recorded, without changing source/evidence/dependencies.

Native captured-library Create/Refresh/Relink in owner mode is next read-only
assessment, not an implementation decision. Full M19 remains partial; all original
47 milestone fields/statuses, 104 full-row states, dependencies and holds stay intact.


## 40. M19 bounded native captured-library authoring (7 October 2026)

Progress format `m19-post-audit-progress.10` records twenty-five bounded increments
through `2d36549fb286a504c1098c623ef9b3718908dd1e`; the prior twenty-four-increment
ledger remains at `71d202f8182b1f65432bc425b9a6a47154466663`.

A tiny native workflow now starts from an ordinary globally static/empty document:
explicit owner-mode entry, separate empty-owner candidate/Commit and selection,
immutable source load, complete all-placement plan/acknowledgement and exact
Create staging. A second retained placement needs no source; whole-library Refresh
preserves maps, and Relink accepts only already-empty maps with explicit complete
empties. Ordinary Commit/Cancel and source-free save/reopen/CPU rendering pass.
No core/Host/schema/dependency change or source authority is introduced.

Sixteen new tests pass alongside nine unchanged owner-preview and four unchanged
meshless App regressions: 29 separately scoped tests. Lint/format/review pass;
seven tiny workflow artifacts and initial compile/layout failures are retained.
No private pending/intent/review/candidate alteration, brush setup or excluded
resource-input equivalent is part of this evidence. Physical/runtime/resource/
security and full M19 qualification remain open.

Native member/source/material/geometry/hierarchy editing remains absent; nonempty
replacement Relink requires explicit CLI/API migration. Per-placement member art
direction with explicit source-update migration is the highest-value unresolved
artist workflow candidate, subject to assessment rather than an implementation
promise. All original 47 milestone fields/statuses, 104 row states and holds remain.


## 41. M19 native member art direction and explicit Relink losses (7 October 2026)

Progress format `m19-post-audit-progress.11` records twenty-six bounded increments
through `722b7f80437e14014a74c957b5763ad1b16e496b`; the prior twenty-five-increment
ledger remains at `b9340bf2b9478ec44703de47083653491181c962`.

An explicit source-member chooser preserves authored-owner selection. Local and
captured members now support full transform replacements with base-translation
editing, destination material selection or fresh complete material copies with a
linear color factor, and Clear-to-current-source. These are pinned full values,
not sparse live-field overrides; source/other placements, namespaces, textures
and shared material values remain intact, and old copies stay retained.

Relink defaults RequireEmpty. Explicit ClearAll lists losses for every affected
placement and needs separate current-plan all-placement and loss acknowledgements.
Refresh stays preserve-only; selective correspondence/remapping remains absent.
Thirteen new tests pass with sixteen unchanged native-library and nine unchanged
owner-preview regressions: 38 tests. Scoped compile/lint/format and independent
review pass. Pre-test membership-resolution/recomposition fixes and tiny valid
texture evidence are retained without performance or forbidden-input claims.

Full M19 remains partial. Source geometry/hierarchy, sparse live-field semantics,
selective migration, general/nested/live libraries and physical OS/browser/GPU/
resource/security qualification remain open. A readiness review may reprioritize
next work; no implementation is promised. All original 47 milestone fields/statuses,
104 full-row states, dependencies and holds remain unchanged.

Acceptance interpretation for this checkpoint: current general/nested/live-library
and native selective-remapping limits are not automatically extra literal M19
blockers. Bounded migration exists through core/Host/CLI. Saved sets, inert entity
metadata and named reusable variants need explicit mapping to the existing
selections/asset-metadata/presets clause; no distinct combined preset subsystem is
presumed mandatory, and broad asset/preset catalogs remain M45 scope.

Readiness checkpoint priority: map the six required evidence categories to literal
criteria before automatic feature expansion. Existing resource_profile coverage is
partial (canonical byte counts and historical tiny debug RSS/timing), not absent
or final captured/owner-capacity qualification. Named reactivatable variants support
a bounded preset interpretation. Owner-mode view/variant selection remains a native
UI integration gap despite existing core/CLI support. No new execution is admitted.


## 42. Current M19 descriptor clarification (7 October 2026)

This corrects wording at the same twenty-six-increment checkpoint; it adds no
implementation or qualification. Only Transform replacements are full value pins.
Material replacement maps store destination Material IDs and resolve current
destination table values. A fresh copy clones full values at creation, but later
destination material edits through applicable public operations remain possible.
Old copies remain retained after Clear or map replacement. Earlier shorthand
about pinned material values must be read with this distinction.

Earlier “one-process” wording means one workflow test using multiple actual
processes. All source-specific test/process counts are unchanged. The prior
chronological entries, source evidence, original 47 milestone fields/statuses and
104 full-row states are preserved rather than retroactively rewritten.

The [read-only acceptance crosswalk](m19_acceptance_checkpoint.md) is now
consolidated at `b8a10b4e5c28fd7179627f2b196a43df75daa014`. Its six-category index
adds no runtime result, measurement, increment or qualification.


## 43. M27 fixed-size color-math prerequisite (7 October 2026)

The separate [M27 prerequisite progress record](m27_color_math_progress.md),
`m27-color-math-prerequisite-progress.1`, records the opt-in
`rec2020-d65-st2084-absolute-v1` fixed-sample foundation at
`3e3abcdc69e4ebd3b915e6a8204190356de24fba`. It covers linear Rec.2020/D65 to/from
XYZ D65, caller-selected relative/absolute reference-white mapping, and bounded
absolute PQ EOTF/inverse EOTF. The [contract](color_math.md) and
[scoped verification](../evidence/m27/color-math/verification.md) retain the exact
units, domains, source precision and independent/supplemental evidence limits.

M27 remains planned and unqualified with unfinished prerequisite M23. This is
prerequisite math progress, not an HDR image/display workflow or a dependency-ready
milestone. No renderer, image, pipeline, document-schema or dependency integration
is claimed. The frozen M19 acceptance checkpoint
`2caf3f954e919688e6a736e317258a55a1969cf7`, its twenty-six-increment history and
acceptance crosswalk are unchanged. The machine-readable additions are separate
`prerequisite_progress` records, with no original milestone or full-parity row/axis
qualification change.


## 44. M28 planar-chain IK prerequisite (7 October 2026)

The separate [M28 prerequisite progress record](m28_planar_chain_ik_progress.md),
`m28-planar-chain-ik-prerequisite-progress.1`, records `PlanarChainIkV1` at
`57d6b02bee76a56f3a17ebace354cc43b8512dc8`. Snapshot 40 adds a bounded proper-rigid planar
3–16-link solver with limits, causal goals and deterministic CCD through the direct
Rig API. Geometry-free public authoring and authored-Snapshot JSON reload are
covered by nineteen isolated native tests and one public example, with scoped
lint/format checks; see the [contract](planar_chain_ik_v1.md) and
[execution record](../evidence/m28/planar-chain-ik/execution.md).

M28 remains planned and unqualified with unfinished prerequisite M18. This is
pose-only prerequisite progress, not general constraint stacks, a character rig
workflow, deformation or render/export integration. The aggregate animation guard
rejects the profile before composition. Snapshot reload does not qualify journals,
history or crash/storage recovery; no resource, security, browser/GPU/device or
full-parity axis qualification is inferred.

The additive `prerequisite_progress` entries preserve all existing M27 metadata,
all 47 historical milestone records and all 104 full-parity row states. Frozen M19
checkpoint `2caf3f954e919688e6a736e317258a55a1969cf7`, its twenty-six-increment
history, cutoffs and acceptance crosswalk remain unchanged. No new implementation
or execution is promised, and existing holds remain active.
