# Representation decisions

Selected baseline with alternatives and test conditions. Read with the canonical specification.

## 5. Representation decisions and rejected alternatives

### 5.1 Selection rubric

Compare candidates against information preservation, edit locality, traversal cost, memory amplification, parallel-read safety, GPU conversion cost, persistent identity, browser fit, numerical robustness, and testability. No aggregate score should hide a failed hard requirement such as losing non-manifold topology or requiring a C++ runtime.

| Concern | Selected baseline | Why not the tempting alternative? |
|---|---|---|
| Scene storage | Typed domain tables with explicit references | A generic ECS world is not a durable authoring schema |
| Document identity | Persistent 128-bit entity IDs; names are labels | Paths and names change during ordinary editing |
| Mesh asset | Dense polygon/corner arrays plus typed attributes | Triangle-only storage destroys important authoring structure |
| Mesh editing | Generational-handle radial connectivity | Strict two-face half-edge topology excludes non-manifold meshes |
| Evaluated geometry | Immutable, shareable, possibly lazy components | Copying full authored objects into every evaluation is wasteful |
| GPU geometry | Backend-private packed/chunked buffers | GPU layout is not a portable project format |
| History | Snapshots plus committed deltas and provenance | Replaying old operations forever couples recovery to old algorithms |
| Procedural graphs | Shared schema, domain-specific execution IRs | One universal graph interpreter erases important semantics |
| Collaboration | Revision checks, branches and explicit merge rules | A generic CRDT cannot infer correct arbitrary topology merges |
| Interchange | Native Rust adapters with declared profiles | Making `.blend`, USD or glTF the internal runtime imports their constraints |

### 5.2 Important precedent, different decisions

Blender already distinguishes compact mesh data from BMesh editing connectivity. Its BMesh is explicitly non-manifold, making that a relevant correctness requirement rather than optional sophistication. The proposed handle-based editing representation borrows this lesson without reproducing pointer ownership. [S01, S02]

USD provides useful ideas about layering, composition, instancing, and selective loading. Its documentation also explains its path-based identity trade-off and why its scene representation is not itself a rigging system. This plan adopts explicit composition while choosing persistent IDs and separate rig evaluation. It does not embed USD or claim to implement all USD semantics. [S03]

### 5.3 What remains replaceable

Chunk sizes, dense-table implementation, work scheduler, BVH layout, shader compiler internals, editor toolkit, and compression remain private. External identity, units, operation semantics, revision consistency, and supported file behavior need stronger stability guarantees. Benchmark before freezing an implementation detail into a public contract.

## 6. Scene model, identity, transforms, and composition

### 6.1 Entities and assets

An entity has a persistent ID and typed components. A component is addressed by entity ID, stable component type ID, and property ID. Renderable objects reference geometry and material assets; they do not own duplicate copies by default. Prototypes and instances are explicit, with immutable asset versions identified by content digest.

Use 128-bit persistent IDs at document boundaries and generational arena handles internally. These are different types. Numeric arena indices and pointers must never escape as durable references. Names, paths, tags and collection membership are queryable metadata. Duplicating an entity allocates a new entity ID while optionally sharing assets. Cross-document references include document/asset identity rather than assuming a locally unique number is universal.

### 6.2 Spatial relationships

A placed instance has at most one transform parent in a given composed view. Parenting cycles are invalid. The same prototype can appear through many instances. Collections can overlap and do not imply parenting. Nested prototypes must not form recursive expansion cycles; traversal also has explicit depth and resource limits.

Authored transforms use typed operation channels with explicit order: translation, rotation, scale, pivots, and an optional affine operation for imported shear or matrices. Preserve authored rotation conventions; do not silently replace Euler animation with quaternion interpolation. Evaluation produces f64 affine transforms. GPU consumers use camera-relative f32 transforms where supported precision permits.

A negative determinant is valid and affects orientation. Singular transforms are represented but diagnosed; operations requiring an inverse must reject or use a documented singular-case policy. Do not force every imported matrix through lossy translation/rotation/scale decomposition.

### 6.3 Units, time, and color identity

Use meters, seconds and radians as baseline computational units with explicit conversion at boundaries. Asset-local positions default to f32; an explicit f64 precision class is available for high-precision assets, without duplicating both forms automatically. Use rational time for frame/sample addressing and f64 during continuous evaluation. A frame rate is a ratio, not an assumed integer.

Color values carry a color-space/role identity. Data textures, radiometric quantities and display colors are not interchangeable vectors. Component schemas declare units, coordinate frames, precision and semantic roles where relevant.

### 6.4 Layers and variants

Start with an explicit ordered stack of sparse overrides keyed by stable IDs. Define set, delete, reference override, and ordered-list edit semantics. Variants select named override sets. A session view is not a saved layer unless explicitly committed. Provenance queries must explain which layer supplies a value. Complex composition is expanded only when a workflow needs it; do not recreate every USD composition arc by default.

## 7. Mesh topology, attribute domains, and provenance

### 7.1 Canonical mesh asset

Store vertex positions, explicit edges, polygon face offsets, and face-corner references in dense typed arrays. A corner identifies a vertex and its edge within a face. Preserve loose vertices/edges, n-gons, disconnected components and non-manifold edges. Initially a polygon has one boundary ring; holes require an explicit later capability or deliberate topology conversion, never accidental loss.

Use geometry-local 64-bit element identities where persistent edit correspondence is required. Runtime indices remain compact and remappable. Do not attach a 128-bit global object ID to every transient triangle. Persist enough local identity to support user-visible selections, attribute correspondence and undo, while allowing unreferenced evaluated data to use cheaper provenance ranges.

### 7.2 Editing topology

The editing kernel uses arenas of vertices, edges, faces and corners with generation-checked handles. Corners participate in face cycles and radial edge adjacency; vertex-edge adjacency supports local traversal. Mutations occur inside exclusive edit transactions. A strict manifold half-edge representation is rejected as the only editing representation because an edge may have more than two incident faces. [S01]

Convert an asset into editing topology on demand and commit back to canonical arrays with validation and correspondence maps. Cache editing state by mesh revision where beneficial. A sculpt stroke must not rebuild every mesh array or convert the entire asset every frame. Dirty chunks and affected-region tracking are required; unavoidable global algorithms advertise that cost.

### 7.3 Typed attributes

An attribute has stable identity, domain, value type, storage, semantic role, default and transfer policy. Domains include point, edge, face, corner, curve, instance and voxel/tile as appropriate. Storage can be constant, dense, sparse or indexed; the evaluator chooses a materialization strategy.

UVs are corner-domain data to preserve seams. Normals, colors, skin weights, material assignments and selection masks have explicit domains and semantics. Transfer policies distinguish interpolation, nearest, normalized vector, categorical choice, extensive quantity, and custom validated behavior. Do not average every numeric attribute blindly. Missing attributes and domain conversions have specified defaults or errors.

### 7.4 Topology-change contracts

Each topology operation reports preserved, created, deleted, split, merged and ambiguous correspondences. Stable IDs do not magically survive remeshing. A bevel or Boolean returns source relationships where known; destructive remeshing may invalidate references. Consumers must reject stale selections or explicitly resolve them again. An agent’s geometric predicate is evaluated against a revision and yields a concrete target set before mutation.

Validate cycles, index ranges, attribute lengths, orientation policies and finite coordinates. Robust geometric predicates, scale-aware tolerances and documented degeneracy behavior are part of algorithm correctness, not deferred cleanup.

## 8. Geometry beyond meshes and representation boundaries

### 8.1 Curves, surfaces, hair, and points

Curves retain basis and degree, control points, knots/weights where needed, periodicity, radius, tilt and attributes. Bezier, polyline and rational spline forms should not be flattened into one sampled representation. Tessellation is derived with explicit error tolerances. Parametric surfaces are distinct from polygon meshes; trimmed surfaces and exact CAD Boolean solids are not implied by initial spline support.

Hair/groom data uses curve primitives with strand identity, root attachment, guide relationships and per-point/per-strand attributes. Child generation remains procedural until evaluation. Point clouds store typed point attributes without fake polygon topology. Text retains content, font asset identity and shaping parameters; glyph outlines and meshes are derived. Font loading and shaping must follow the Rust dependency policy.

### 8.2 Volumes and implicit geometry

Use a sparse, paged field representation with independently addressed tiles, explicit index-to-world transform, channel types, background values and active regions. Signed-distance fields, density, temperature, velocity and labels have different semantics and interpolation rules. Sparse data remains sparse through persistence and evaluation.

Do not use a signed-distance field as the universal representation of every object: thin surfaces, exact UVs, corners, topology identity and high-frequency detail may be lost in conversion. Similarly, do not require a dense 3D texture to represent every volume. Mesh/volume conversion must report resolution, approximation and resulting loss of correspondence.

OpenVDB-shaped interoperability is not permission to link OpenVDB. A native Rust file adapter is a separate scoped project; the internal volume layout should be chosen on access patterns, compression and browser working-set measurements.

### 8.3 Geometry sets and instances

An evaluated geometry set is a typed collection of mesh, curves, points, volumes, instances and drawing components with shared immutable ownership. Components may be materialized or backed by lazy asset references. Instancing stays explicit until an operation genuinely requires realization. Blender’s GeometrySet offers a useful precedent for component sharing, but the proposed persistence and handle boundaries are independent. [S04]

### 8.4 Conversion as an observable operation

Every representation conversion declares source/target types, supported attributes, approximation budget, identity mapping and resource estimate. Conversion failures cannot quietly become empty geometry. Renderer tessellation does not overwrite authored splines; simulation collision simplification does not replace the visible mesh; paint proxies do not become the saved texture accidentally. Conversion caches are keyed by source content and policy, not only object names.

## 9. Revisions, persistence, memory, and recovery

### 9.1 Document transactions

Use multiversion document state: one serialized commit path per document, many immutable readers. A transaction prepares changes against a base revision, validates read dependencies and write constraints, then publishes atomically. Expensive evaluation happens outside the commit lock on a candidate revision. Initially reject stale-base writes conservatively; permit finer conflict detection only when read-set and predicate dependencies are correctly tracked.

Session state such as hover, viewport orbit and transient selection is separate from authored data. A tool gesture can generate many ephemeral previews and one durable commit. Branches share unchanged data; they must not copy the entire project.

### 9.2 Storage format

Use a versioned native package with a manifest, typed metadata records, content-addressed binary chunks, named checkpoints, and an append-only journal of accepted deltas. Choose deterministic encodings and explicit endian, numeric, alignment and schema rules. Rust struct layout, `serde` defaults, or a particular zero-copy archive crate are not the format specification.

Start with canonical JSON metadata and typed little-endian chunk payloads where practical. Define canonical ordering and finite-number rules before hashing. Images/volumes may carry explicit invalid-sample masks rather than forcing all scientific data into property-validation rules. Compression is optional and versioned; an uncompressed baseline must remain readable. Rust codec choices require dependency audits.

### 9.3 Crash consistency

Publish durable commit receipts only after the selected storage adapter completes its documented durability protocol. Native storage can use journal ordering, file flush and atomic publication; browser storage needs its own tested protocol. Recovery selects the last intact committed root and verifies referenced chunks. Torn writes, missing assets, quota errors and interrupted migrations are explicit states.

Persist resolved deltas and checkpoints, not only operator calls. Historical operations retain provenance and versions, but opening a saved project must not require re-executing years of changing algorithms. Migrations are explicit, tested, backed up, and must not destroy the original before a valid new root exists.

### 9.4 Memory discipline

Use typed dense tables and copy-on-write chunks rather than `Arc` per vertex or one global lock around all data. Cache evaluated results by content and maintain separate CPU/GPU budgets. Pin data while jobs/readers need it; reclaim unreachable history through explicit garbage collection with recoverable roots. Hashing deduplicates content but does not prove authorization. Enforce tenant/document permissions separately.

Chunk size and indexing structures are benchmark decisions. Require measurements of load time, traversal, small edits, wide edits, branch cost, fragmentation and conversion—not merely a theoretical claim of cheap snapshots.

## 10. Evaluation, procedural execution, and stateful work

### 10.1 Inputs and results

An evaluation request identifies document revision, composed view, rational time/subframe, output purpose, quality policy and resource budget. A result is immutable and labeled with its complete input identity. Consumers cannot accidentally mix geometry from one revision with transforms or materials from another.

The evaluator tracks property/component dependencies, not just coarse “object changed” flags. A cache key includes operation semantic version, resolved input contents, time, relevant random seed, numerical policy and backend when backend behavior affects results. Capture dynamic dependencies when an operation resolves assets, queries sets or follows references. Query predicates require invalidation when membership can change.

### 10.2 Shared graph infrastructure, specialized execution

Geometry, materials, animation and compositing share stable node/port IDs, schemas, diagnostics, groups, provenance and editor conventions. They do not share one undifferentiated evaluator. Geometry fields have domains and demand-driven materialization; shaders have derivatives and scattering semantics; compositing has tile/temporal dependencies; simulation has evolving state.

A modifier stack is a convenient linear view over an operation graph, not a separate implementation of the same algorithms. Expose reusable operations through both graph nodes and direct commands, with one semantic definition. Higher-level authoring macros expand into explicit operations and record the expansion.

### 10.3 Fields and procedural data

A field is a typed expression evaluated over a declared geometry domain, with captured context and dependencies. Distinguish a field from a materialized attribute and from a single scalar. Preserve instance data until realization is required. Cache group compilation separately from its input-dependent execution. Reject illegal field/domain combinations early and report where conversions occur.

### 10.4 Cycles, solvers, and simulations

Ordinary dependency cycles are diagnosed with a minimal causal path. Intended feedback must be inside an explicit solver or state-transition node. A solver declares convergence criteria, iteration limits and failure behavior. A simulation declares initial state, prior-state input, time step, external forces, collision dependencies and seed.

Checkpoint simulation state; random timeline access loads a compatible checkpoint and advances, rather than pretending each frame is independent. Changing an upstream parameter invalidates dependent future caches. Distributed execution is optional and must preserve these semantics.

### 10.5 Scheduling

Parallelize independent read-only tasks and bounded kernels. Batch tiny operations to avoid scheduler overhead. Prioritize interactive previews without starving durable jobs. Cancellation is cooperative at safe boundaries; GPU completion and driver preemption cannot be assumed. A cancelled evaluation may discard results, but it cannot partially mutate an authored revision.
