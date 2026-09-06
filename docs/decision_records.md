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
