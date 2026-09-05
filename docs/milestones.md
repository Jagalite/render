# Milestones and release gates

This is a focused extract of the canonical architecture specification. All states are planned.

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

The detailed machine-readable matrix contains planned states only. Update status per tested capability; do not mark a feature supported because its data type exists.

### 28.1 Compatibility axes

For each feature, record native authoring, evaluation, final render, preview, human editor, import and export. Separate native desktop and browser-local support. Record minimum resources, known numerical limitations and supported operation/schema versions. A native import may preserve unsupported source data read-only without implementing its behavior.

### 28.2 Definition of an improvement release

A release qualifies as improved when its evidence demonstrates the intended workflow gains: explicit operations, safer experimentation, fewer unintended changes, lower incremental costs or more reliable integration. Do not claim superior rendering speed, memory use, topology robustness or user productivity across all workloads without comparative measurements. A strong architecture provides mechanisms for improvement; the benchmark and user workflow establish whether they work.
