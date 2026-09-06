# Architecture decision register

M06–M08 now have [named implementation profiles](m07_m08.md) and [complete milestone evidence](../evidence/m06-m08/README.md). Historical M05 Lambertian and [static PBR](static_pbr.md) evidence retain their original scope. External input tracks remain separately qualified in `planning/input_support.json`.

The initial proposals are preserved below. M00–M04 decisions, supporting measurements, accepted limitations and revisit triggers are recorded in [accepted foundation ADRs](../evidence/m00-m04/accepted_ADRs.md). M05 integration decisions are in [accepted agent alpha ADRs](../evidence/m05/accepted_ADRs.md). Decisions outside these demonstrated profiles remain proposed. See the full specification for consequences.

## ADR-001 — Fully Rust production implementation

**Decision:** Use Rust-owned computational implementations; generated platform artifacts and narrow environmental exceptions only.

**Rejected baseline:** Rust wrappers, Cycles reuse and Blender workers.

**Revisit trigger:** Native-link audit reveals any non-permitted computational dependency.

## ADR-002 — Typed authoring database, not ECS as durable schema

**Decision:** Persist typed components/assets with stable external identity; use private optimized tables.

**Rejected baseline:** Generic ECS world or untyped property graph as the whole project format.

**Revisit trigger:** M00 shows an alternative preserves semantics while materially improving measured locality/memory.

## ADR-003 — Distinct mesh and edit representations

**Decision:** Canonical polygon/corner arrays plus handle-based radial editing topology.

**Rejected baseline:** Triangle-only assets and manifold-only half-edge kernel.

**Revisit trigger:** Conversion/memory costs break interaction budgets; change private chunking without losing topology semantics.

## ADR-004 — Scoped persistent identity

**Decision:** Global entity IDs, geometry-local element IDs and separate generational runtime handles.

**Rejected baseline:** Name/path identity, pointer identity or global UUID per transient triangle.

**Revisit trigger:** Measured ID overhead exceeds budget or edit correspondence requirements change.

## ADR-005 — Snapshots plus deltas and provenance

**Decision:** Recovery loads valid checkpoints and committed resolved deltas.

**Rejected baseline:** Re-executing an unbounded historical command log.

**Revisit trigger:** Crash/fault tests expose ambiguous durable state; revise publication protocol before format stabilization.

## ADR-006 — Multiple specialized execution graphs

**Decision:** Share schema/editor conventions while retaining shader, geometry, time and solver semantics.

**Rejected baseline:** One untyped universal graph interpreter.

**Revisit trigger:** A real shared kernel can reduce duplication without erasing domain contracts.

## ADR-007 — CPU reference plus portable GPU kernels

**Decision:** Rust CPU implementation and Rust-owned kernel IR; software traversal is the portable baseline.

**Rejected baseline:** Mandatory native hardware ray features or hand-maintained non-Rust runtime kernels.

**Revisit trigger:** M00 kernel generation fails correctness/usability/performance requirements.

## ADR-008 — Small C-compatible ABI implemented in Rust

**Decision:** Opaque handles, version negotiation and explicit buffer ownership.

**Rejected baseline:** Stable Rust trait-object/layout ABI.

**Revisit trigger:** Independent client conformance shows a missing lifecycle/error capability before ABI v1.

## ADR-009 — Branch/revision collaboration first

**Decision:** Conservative conflicts; reviewed commutativity and explicit reconciliation.

**Rejected baseline:** Automatic generic CRDT merges of arbitrary topology.

**Revisit trigger:** A precisely scoped merge can be proved safe with complete dependency information.

## ADR-010 — Native format adapters with loss reports

**Decision:** Own format-independent authoring semantics and implement scoped adapters in Rust.

**Rejected baseline:** USD/.blend runtime core or external conversion workers.

**Revisit trigger:** A documented user workflow justifies a new format/version feature profile.

## ADR-011 — Precision is a typed policy

**Decision:** f64 transforms/time computation; local f32 geometry with explicit f64 asset class and camera-relative GPU forms.

**Rejected baseline:** f32 everywhere, f64 everywhere or forced lossy matrix decomposition.

**Revisit trigger:** Scale/pathology tests demonstrate an inadequate precision class or memory trade-off.

## ADR-012 — Human editor as client

**Decision:** Ephemeral gesture previews and validated durable operations share the public contract.

**Rejected baseline:** Editor-only mutable state paths.

**Revisit trigger:** A measured interaction bottleneck requires a faster transport, not bypassing semantics.

## ADR-013 — Typed breadth and exact-time derived evaluation

**Decision:** M06–M08 add typed snapshot fields and ordinary transactions;
procedural outputs, sweeps, displaced meshes and animated poses remain derived.
Groom/skin/morph attachments use topology digests and stable local IDs. Rig binding
requires explicit triangle realization. Visibility does not remove dependency
anchors. Time and shutter arithmetic use checked rationals. GPU support is granted
per named material/geometry profile; CPU-only media, extended scattering and
imaging return explicit unsupported errors on incompatible backends.

**Integration contract:** [M06 operators](m06_modeling_contract.md),
[M07/M08 semantics](m07_m08.md) and [execution gates](m07_m08_execution.md).
Historical Lambertian and static PBR evidence remains pinned separately. Numerical
references, native workflows, browser parity, resources and source/dependency
manifests gate the capability status update.

**Rejected baseline:** Baking authored hair/rig/graph state into anonymous triangles,
using platform-private mutation paths, or silently falling back from requested GPU
profiles. Bounded named approximations remain visible in receipts.

**Revisit trigger:** New authoring or backend workflows pass their own numerical,
resource, portability and recovery gates; existing profile names keep their meaning.


## ADR-014 — General native CLI as a filesystem adapter

**Decision:** `project-cli-v0` uses a typed one-shot JSON request and ordinary
revision-checked document transactions. Host-only paths resolve explicit user
resources; imported metadata cannot trigger fetching. Rendering/evaluation calls
the existing Rust APIs, with pinned authored revisions and no fixture-generated
scene. Source/archive creation uses a new native journal directory.

Artifact bundles publish through synced pending directories and a final status
manifest. Cancellation is checked after candidate preparation at the journal
publication boundary; an admitted commit finishes atomically. Incomplete or
cancelled sequences retain only explicitly published frame bundles. Output names
use host indices so authored labels cannot become filesystem paths.

**Evidence:** [CLI contract](project_cli.md), [independent CLI/native API equivalence,
recovery, faults and resource tests](../evidence/project-cli/README.md). At that CLI validation revision, shared Rust,
Wasm and dependency sources were unchanged from the M06–M08 validated base.

**Revisit trigger:** Interactive editor/remote transports require additional
lifecycle semantics, while retaining the same public mutation authority and
backend profiles. Native driver initialization and admitted publication remain
explicit non-interruptible boundaries; Windows durability remains compile-only.

## ADR-015 — Typed absolute animation and skin-specific bind palettes

**Decision:** glTF import produces ordinary native animation components and a
collision-rejecting `merge_animation` transaction. Snapshot v11 introduces complete
absolute TRS channel groups and explicit per-skin inverse-bind palettes; existing
pose deltas and strict native rig rest/inverse-bind validation retain their meaning.
Default morph weights are authored data. Native and browser share evaluation.

**Reason:** glTF channels replace local properties, and two skins can bind the same
joint hierarchy differently. Reinterpreting native delta tracks or deriving all
skin matrices from the skeleton rest pose would change imported motion. Imported
source indices are converted to stable scoped IDs. New fields are typed, additive
and omitted when empty; old snapshot versions reject the new features.

**Integration review:** [profile and migration contract](animated_gltf.md),
[acceptance and review evidence](../evidence/animated-gltf/README.md). The workflow
crosses importer, animation, deformation, document and host/browser boundaries.
Native JSON retains converted animation; source glTF export remains unsupported.

**Revisit trigger:** Sparse/morph-normal data, additional source formats, skinning
methods or GPU shutter accumulation require separate profiles and numerical,
resource and persistence evidence. M09 UI work remains independent.

## ADR-016 — Bounded shared opaque path transport

**Decision:** Extend the existing CPU loop and Rust kernel IR to shade 1–16
opaque PBR/diffuse vertices through the existing typed max_depth setting. Preserve
primary passes, indexed samples, ordinary render transactions and depth-one CPU
semantics. Use BSDF-only sampling for emissive surfaces/environment and direct
point-light evaluation. Declare finite-depth bias, secondary LOD0 and authored
occlusion on indirect throughput. No new runtime dependency or snapshot version.

**Integration boundary:** GPU packing carries depth and sample stride in private
parameters; no public Rust layout is exposed. Browser agent preview admits the
same depth range within its existing pixel/sample limits. GPU progressive receipts
retain the selected material/depth qualifications. Advanced surfaces/media and GPU
shutter accumulation remain separate profiles. See [contract](multibounce_pbr.md).

**Revisit trigger:** Measured small-emitter variance or texture aliasing motivates
continuous light sampling/MIS or propagated secondary footprints, with independent
numerical evidence before changing the named policy.

## ADR-017 — Exact-time GPU shutter accumulation and bounded consumers

**Decision:** Share the existing frame validation/identity semantics across Rust
CPU and GPU backends. Evaluate derived poses in Rust, accumulate weighted linear
color in a persistent device buffer, and preserve nominal-time passes. A 4-byte
intermediate readback fences each dispatch; one full readback completes the frame.
Dispose temporal evaluator caches between samples. Sequence consumers accept one
completed frame at a time, with browser Promise backpressure and explicit partial
counts. No authored schema version or runtime dependency changes.

**Integration boundary:** Core exact-time evaluation, generated kernel private
packing, GPU resource/cancellation lifecycle, optional CLI backend and browser
branch authority meet at the [GPU shutter contract](gpu_shutter.md). Animated
previews never qualify a static branch for commit. Existing CPU default requests
and exact frame identities retain their meaning.

**Revisit trigger:** Measured scene rebuild/fence overhead justifies a bounded cache
or asynchronous dispatch ring with equivalent cancellation, budget and publication
semantics. Driver preemption, unbounded consumer buffering and automatic CPU
fallback are not part of this profile.
