# Rust-native 3D Creation Platform
## Architecture, Representation Decisions, and Delivery Roadmap

**Specification version:** 0.1 — proposed baseline  
**Prepared:** September 5, 2026  
**Implementation status:** Experimental M00–M05 foundation and agent-rendering profile delivered; current tested profiles and later gates are recorded in `planning/milestones.json`, `docs/agent_alpha.md` and `evidence/m05/README.md`.
**Product direction:** Agent-native, browser-capable, native desktop, fully Rust application implementation.  
**Working name:** “Rust-native 3D Platform”; crate names in this document are illustrative.

> Build a programmable creative system first, a reliable renderer second, and rich interactive editors on the same foundation. Preserve creative intent instead of treating a rendered mesh as the entire project.

## 1. Executive decision and project charter

### 1.1 The product

Build an independent 3D content-creation platform encompassing the major functional families associated with Blender: scene assembly; modeling; procedural geometry; materials; real-time and final rendering; animation and rigging; sculpting, UVs and painting; simulation; 2D/3D drawing; compositing; tracking; and video/audio assembly. Feature families are long-term commitments, not a claim that an early release is a complete replacement.

The defining improvement is a **versioned, inspectable authoring system shared by people, agents, and ordinary applications**. The human editor must not own functionality that the public operation layer cannot express. Agents must not need to emulate clicks, modify arbitrary memory, or execute unrestricted scripts to perform normal work.

The implementation will be Rust throughout the project-owned runtime: document model, geometry, evaluation, rendering, material compilation, persistence, operations, services, and editors. The earlier proposal to reuse Cycles or a Blender compatibility worker is superseded. Neither is a production dependency, fallback, or milestone shortcut. Blender may be an external development reference and test oracle only.

### 1.2 Development order

Establish representation and operation contracts; deliver a Rust-owned scene and CPU reference renderer; deliver native/browser GPU rendering; demonstrate a complete agent workflow; then expand procedural authoring and interactive editing. Rendering first means postponing editor tools, not postponing the document model, topology semantics, time model, or mutation rules.

Browser capability is preserved from the beginning. The shared core must build and run in WebAssembly early; an optional remote Rust worker can increase capacity but must not be required for the browser’s declared local baseline. Native macOS, Linux, and Windows builds use the same semantics.

### 1.3 Meaning of “better”

The design optimizes explicit intent, safe change, numerical correctness, incremental work, interoperability, and independently testable modules. It does not assume that Rust alone makes every operation faster or that a new data structure is automatically superior. Decisions below are selected hypotheses with falsification tests. A failed benchmark should change an internal representation before compatibility makes it expensive to change.

**First useful release:** an agent or program creates a native scene, branches it, changes lighting/materials, renders variants locally, evaluates structured results, and commits a selected variant. It must work without Blender, Cycles, Python, or a GUI controlling the document.

## 2. Scope contract: what fully Rust means

### 2.1 Required boundary

All project-maintained executable application logic is authored in Rust. Prefer Rust runtime dependencies; inspect their transitive native dependencies and enabled features. A Rust wrapper around a C/C++ implementation does not satisfy the requirement. No bundled or service-based Blender, Cycles, Embree, OpenSubdiv, OpenVDB, OpenColorIO, OpenImageDenoise, FFmpeg, or CPython may silently implement a core feature.

The OS, browser, GPU driver, and platform graphics/shader-compilation services are environmental dependencies, not components this project rewrites. Bindings to these services are allowed and documented. Build compilers are also tools, not the application’s implementation language. These narrow exceptions must not expand into “any library exposed through a system API.” In particular, baseline media decoding must use Rust implementations or supported image/audio sequences, not an undocumented native-code codec fallback.

### 2.2 GPU programs without a second maintained implementation language

Construct a deliberately limited typed kernel representation through Rust code: arithmetic, buffers, control flow, sampling primitives, and explicit resource access. Lower it through a private compiler boundary to generated WGSL and supported native shader formats. A generated shader is a compilation artifact, like WebAssembly, not a second hand-maintained source implementation.

Do not attempt to compile unrestricted Rust to every GPU. Prove the required subset with traversal and shading kernels in M00. Naga is a candidate Rust translation/validation component, not the permanent public kernel schema. Its documentation exposes shader translation and multiple backends; it does not provide the project-specific kernel authoring system. [S07]

Minimal generated browser JavaScript bindings and generated C headers are permitted interoperation artifacts. The browser shell and editor behavior remain Rust-owned. Python/TypeScript clients may consume the API externally; they are not embedded application runtimes.

### 2.3 Enforceable release evidence

Each release includes an SPDX-style dependency inventory, native-link inspection, enabled-feature list, generated-artifact manifest, and explicit platform exceptions. Audit browser and native builds independently. A non-Rust computational dependency blocks the “fully Rust” release profile rather than being excused by an optional Cargo feature.

Treat third-party plugin code as external user-supplied software, outside the base distribution’s implementation guarantee. The default bundled plugins must comply. Untrusted plugin execution is a separate safety problem; language choice alone does not provide isolation.

### 2.4 Non-goals for initial releases

No immediate arbitrary `.blend` fidelity, Python add-on compatibility, pixel-identical Blender output, universal CAD-solid modeling, unlimited browser scene sizes, or automatic conflict-free mesh collaboration. These exclusions prevent inaccurate compatibility claims; they do not remove major creative feature families from the roadmap.

## 3. Improvements that must be observable

| Improvement | Concrete architectural mechanism | Evidence required |
|---|---|---|
| Agent operations without editor context | Explicit IDs, parameters, revisions and units | Identical operation suite through CLI, API and editor |
| Safe experimentation | Branches, preview revisions, atomic commits | Failed/cancelled jobs never alter committed state |
| Explainable changes | Read/write effects, provenance and topology maps | Every committed change has structured before/after evidence |
| Incremental large-scene work | Shared chunks, dependency tracking, instance preservation | Transform-only change does not rebuild mesh buffers |
| Reliable integration | Versioned operation schemas, wire protocol and small ABI | Old supported clients pass conformance tests |
| Better representation fidelity | Distinct authored, edit, evaluated and GPU forms | UV seams, instances and authoring intent survive round trips |
| Predictable rendering | Capability profiles and approximation reports | Unsupported features fail or degrade only by explicit policy |
| Recoverable projects | Durable snapshots, journal and content checks | Crash injection recovers a valid advertised revision |
| Human/agent collaboration | Same transactions; explicit conflicts and branches | No privileged editor mutation path; no silent overwrite |
| Reusable subsystems | Acyclic crates and narrow interfaces | Headless core links without editor or LLM dependencies |

These are acceptance criteria, not claims about limitations in every Blender workflow. The project must compare against a well-engineered scripted Blender baseline, not an artificially poor click-automation baseline.

### 3.1 Prioritization rules

Correctness and honest capability reporting outrank visual polish. Preserve information at boundaries before optimizing representation. Optimize measured hot paths, not the number of abstractions. Require an end-to-end user workflow at each release, not merely isolated technical demonstrations.

Do not let “agent-native” become “LLM-dependent.” Deterministic tools and ordinary scripts must receive the same functionality. Natural-language interpretation, aesthetic ranking, model selection, and planning are optional external clients; the engine is responsible for executable semantics and verifiable effects.

### 3.2 Deliberate improvements with trade-offs

Persistent IDs make rename/reparent operations safer but add identity-management costs. Immutable chunks support branching but require reclamation and careful write granularity. Separate editing topology improves local algorithms but requires validated conversion. A fully Rust renderer gives architectural independence but creates substantial rendering, color, device, and media work. The roadmap budgets these as real subsystems rather than assuming a wrapper solves them.

## 4. System architecture and dependency direction

### 4.1 Logical execution flow

```text
Rust editor / agents / CLI / external SDKs
                  |
       schema + auth + semantic operations
                  |
        validation + transaction preparation
                  |
    revisioned authored document + asset store
                  |
       composition + time-aware evaluation
                  |
         immutable evaluated snapshot
                  |
      Rust renderer / simulation / export
                  |
       images + diagnostics + provenance
```

A candidate branch follows the same path as the main document. The renderer consumes an evaluated snapshot, not a mutable editor database. Simulation consumes explicit state and writes new state/caches through its own job contract. Exporters are consumers, not special mutation engines.

### 4.2 Keep six graphs distinct

**Document relationships** describe ownership and references. **Transform parenting** determines spatial inheritance. **Asset composition** handles prototypes, instances, layers and variants. **Evaluation dependencies** determine computational invalidation. **Node graphs** are user-authored programs. **Render graphs** schedule GPU passes and resource hazards.

These graphs may refer to the same entities but are not interchangeable. A collection membership edge is not a transform parent. A shader node edge is not a document transaction. A render pass dependency is not persisted artistic intent.

### 4.3 Modularity without a microservice tax

Start with a modular monolith/library workspace and a local service host. Enforce acyclic crate dependencies. Use optional worker processes for expensive or unsafe workloads, not a distributed service for every operation. In-process APIs and remote APIs share semantics while using different efficient transports.

Domain crates define data and algorithms. The host provides scheduling, capabilities, assets, clocks, and storage through narrow interfaces. Platform crates provide OS/browser services. Editors consume public operations and read snapshots. No geometry crate may import editor widgets, an LLM provider, a network client, or a concrete GPU backend.

### 4.4 Evaluation is a compiler boundary

Authored intent is composed and lowered into executable work. Evaluated output is then lowered into renderer-specific resources. Each boundary has diagnostics, content identities, and versioning. This avoids exposing internal Rust structs as the permanent external format and avoids storing driver handles or transient caches in the project document.

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

## 11. Semantic API, transactions, and asynchronous jobs

### 11.1 One operation registry

Each operation declares a stable name/version, input/output schemas, defaults, units, explicit targets, preconditions, effects, required capabilities, cancellation class and diagnostics. Generate Rust client types, documentation, external tool descriptions and protocol validators from a reviewed schema source. Do not derive the entire public contract blindly from private Rust structs.

Representative groups include `document.*`, `scene.query`, `mesh.*`, `curve.*`, `material.*`, `graph.*`, `animation.*`, `simulation.*`, `render.*`, `image.*`, `timeline.*`, `asset.*` and `job.*`. Queries are side-effect free apart from permitted cache population. Mutations require a revision context. Rendering uses immutable input and creates artifacts, not hidden document edits.

### 11.2 Transaction lifecycle

Prepare against a base revision; resolve predicates to concrete IDs; validate types, permissions, resource estimates and invariants; produce a candidate delta; evaluate where requested; commit only after rechecking the relevant base. A preview receipt includes effects, warnings, estimated/actual resource use and the candidate revision.

Idempotency keys are scoped to authenticated principal, document and operation class. Reusing a key with a different normalized payload is an error. Persist the accepted result so a network retry returns the prior outcome. Specify retention; after expiry a client must not assume safe replay. Concurrent requests with the same live key converge on one execution.

### 11.3 Job lifecycle

Use explicit states: accepted, queued, running, cancel-requested, succeeded, failed and cancelled. Return a job ID before long execution. Events have monotonically increasing sequence numbers, resumable cursors and bounded retention. A cancellation acknowledgment is not a claim that GPU work has stopped. Terminal receipts identify usable partial artifacts, if any.

Jobs carry wall-time, memory, geometry-growth, sample and output-size budgets. Separate admission estimates from measured use. Stage large uploads as assets before committing references; use checksums and finalization. Enforce queue fairness and per-principal limits.

### 11.4 Results and errors

Structured errors include code, stage, entity/property/node references, safe explanation, retryability and relevant revision. Geometry operations return correspondence data. Render results return image artifacts, pass metadata and a reproducibility receipt. Do not return only prose, screenshots, or a boolean success flag.

The bundled JSON schema/examples are a narrow draft of a material-edit transaction, not a complete API implementation. Schema validation proves document shape only; semantic validation and atomicity require the future engine.

## 12. Native ABI, wire protocol, and extension safety

### 12.1 Three independent contracts

The semantic API defines meaning. The wire protocol transports requests, events and bulk assets. The native ABI lets trusted compiled consumers access the engine. Each is separately versioned. The Rust Reference states that the native Rust ABI has no stability guarantee; use a deliberately small C-compatible ABI implemented in Rust for external binary interoperability. C calling conventions do not require a C implementation. [S05]

### 12.2 ABI design rules

Expose an entry-point negotiation function returning a versioned function table with structure sizes and capability information. Use fixed-width integers, opaque handles, explicit buffer lengths and documented alignment. External IDs use defined byte encodings rather than platform-dependent `u128` passing. Never expose `Vec`, `String`, trait-object layouts or private scene structs.

Specify allocation/free ownership, buffer lifetimes, thread affinity, concurrency, callback reentrancy and shutdown ordering. Prefer polling event queues over arbitrary callbacks into a locked engine. Memory loaned to a client is read-only and pinned until release. Reject stale handles. Checked lengths do not make arbitrary caller pointers safe; an in-process ABI is a trusted boundary.

Contain panics at exported boundaries under an explicit panic strategy. A caught unwind does not imply corrupted state is reusable; discard the failed transaction or quarantine the instance. Fatal faults, aborts and malicious native code require process isolation. Rust’s FFI documentation distinguishes unwind-permitted and non-unwind ABIs. [S06]

### 12.3 Transport and bulk data

Use JSON control messages initially with a specified envelope and separately transferred typed binary blobs. Avoid per-vertex RPC and giant JSON geometry arrays. Local zero-copy or shared-memory optimizations require ownership, leases and validation; the remote protocol never transports pointers. Buffer descriptors include element format, count, stride, endian, alignment and content identity.

Stabilize a small ABI at the agent-rendering release after conformance tests. Keep domain operations independently versioned so a stable entry point does not freeze every geometry algorithm. Never silently change a successful operation’s units or semantics under the same version.

### 12.4 Plugins

Provide schema-registered operations and explicitly granted host capabilities. Prefer isolated WebAssembly extensions for untrusted logic; evaluate a Rust-native interpreter/runtime and its native dependencies before selection. Browser plugin execution must also be isolated from host imports. A Web Worker alone is not a complete permission sandbox. Native trusted plugins are opt-in and process-isolated where possible. Signatures establish provenance, not harmlessness.

Do not let a plugin’s schema registration bypass document invariants, resource admission or audit logging. A user’s external code may consume the language-neutral API, but bundled extensions follow the fully Rust policy.

## 13. Agent workflows, observability, and security

### 13.1 Canonical workflow

Inspect a pinned revision; discover relevant operations; create a branch; prepare a bounded batch; evaluate; render; inspect structured results; commit against the expected base. No LLM is inside the authoritative transaction engine. The host authenticates the client; caller-supplied strings cannot grant identity or permissions.

For example, an agent creates three lighting variants while preserving product geometry, renders them under a sample and memory budget, and returns artifact references and changed-property summaries. The invariant checker verifies that geometry digests did not change. The agent or human chooses a variant; the engine commits it only if its base remains valid.

### 13.2 Structured perception

Expose object/instance IDs, bounding boxes, visibility, topology statistics, attribute summaries, material bindings, normals, depth, motion vectors and named render passes. State pass units and coordinate conventions. A pixel-to-entity hit is revision- and camera-specific. Do not use screenshots as the sole inspection interface.

Support queries such as “why did this geometry change?”, “which nodes depend on this asset?”, and “what prevented this render from using the requested backend?” Trace input dependencies, conversion approximations, cache hits, timings and memory charges. Per-pixel full path traces are optional debug captures, not mandatory storage for every sample.

### 13.3 Permission and resource model

Capabilities separately control document read/write, filesystem locations, asset upload/download, network access, process spawning, plugin installation and compute budgets. Default to no network and no arbitrary script execution during import. A remote asset URL is resolved only through an authorized asset service with size and recursion limits.

Treat project text, node labels, imported metadata and external files as untrusted content, never as new instructions for the agent. The API must not allow imported instructions to escalate permission. Redact secrets from logs; make telemetry opt-in and separate it from local audit records.

### 13.4 Multi-agent conflict policy

Begin with branches and conservative revision checks. Permit automatic merges only for operations with reviewed commutativity rules and complete dependency information. Two writes to different properties can still conflict when they share an invariant. Arbitrary mesh edits, rig restructuring and simulation changes require explicit reconciliation; do not advertise universal CRDT merging.

A successful agent benchmark must measure task validity, destructive mistakes, retry behavior, cost, convergence and ability to explain changes. Human preference can rank aesthetics, but must not replace machine-verifiable correctness for scene invariants.

## 14. Rust rendering architecture and GPU strategy

### 14.1 Independent renderers, shared semantics

Implement a CPU reference path tracer in Rust first. Then provide a Rust GPU path tracer and a real-time preview renderer sharing material semantics, scene extraction, asset resolution, camera conventions and render-job contracts. A renderer backend is not allowed to own the document model.

The CPU path supplies debuggable reference behavior, small headless jobs and numerical tests. It is not assumed to be fastest. The GPU path lowers Rust-authored kernels to generated shader programs. `wgpu` is the candidate abstraction across native and browser targets; its official documentation describes both graphics and compute portability. [S08]

### 14.2 Baseline and enhanced capabilities

The portable baseline uses compute traversal of a software acceleration structure rather than assuming hardware ray queries, bindless resources, f64 shaders or a particular vendor API. Native acceleration can later be an explicit backend capability. Current wgpu documentation contains native/experimental ray-related limits; their presence must not be interpreted as browser-baseline availability. [S09]

Query and request actual device limits. Chunk buffers, texture arrays and work queues to fit the negotiated device. Handle device loss by rebuilding disposable resources from immutable snapshots. WebGPU exposes asynchronous validation and allocation behavior; availability below a numerical buffer limit does not guarantee successful allocation. [S10, S09]

### 14.3 Acceleration and incremental synchronization

Use a two-level acceleration design: per-geometry structures and a scene-level instance structure. Choose the CPU construction algorithm using build/traversal benchmarks; begin with a conventional bounded binary hierarchy and evolve privately. Share immutable geometry between instances. Transform-only changes update scene-level data; deformation can refit when valid; topology changes rebuild affected geometry.

Do not reuse acceleration caches across incompatible motion, displacement, visibility or numerical policies. Bound traversal stack/queue use. Test rays at tiny/large scales, grazing intersections, degenerate triangles and negative transforms. A fast wrong intersection kernel is not an acceptable optimization.

### 14.4 Kernel compilation as a bounded project

The kernel IR needs typed scalar/vector operations, structured control flow, buffer access, atomics where supported and source mapping. It does not need the full Rust type system. Pin translation versions behind a private adapter. Compare CPU and generated GPU functions on randomized inputs, but also use analytic tests: sharing one bug across CPU and GPU is not independent validation.

M00 must prove a small traversal/shading kernel on native and browser targets before the project commits to this authoring route. Performance and compiler usability are explicit risks, not assumed solved by Naga.

## 15. Materials, lighting, color, and image quality

### 15.1 Material representation

Use a typed material graph separating surface scattering, emission, volume response, displacement and ordinary numeric/texture expressions. Preserve graph intent; compile into renderer-specific programs. MaterialX offers a relevant precedent for typed shading relationships and scattering-related types, but neither its runtime nor all of its semantics are automatically adopted. [S11, S12]

Initial support includes diffuse, dielectric, conductor and a documented layered/principled material subset, image textures, normal mapping and emission. Later add subsurface, hair, heterogeneous volumes, advanced layering and displacement. Each node declares coordinate assumptions, units, derivatives/footprint needs and supported backends. Unknown nodes remain preserved in imported data but cannot silently render as black or disappear.

### 15.2 Reference light transport

Start with direct-light sampling, BSDF sampling, multiple importance sampling, emission, Russian roulette, environment sampling and correct PDFs. The PBRT reference provides a primary technical basis for this staged path-tracing design; copying code is not required. [S13]

Fixed finite path-depth truncation, firefly clamping, approximate transparency, denoising and other bias-inducing choices are named policies. Do not label every image unbiased. Use counter-based or otherwise explicitly indexed sample streams keyed by pixel, sample, dimension and seed so task scheduling does not accidentally change sample allocation.

### 15.3 Color pipeline

Keep scene-linear working values distinct from display transforms. Use tagged linear-sRGB working color for the first release, with explicit source conversions, and design the type system to support additional working spaces later. This is a scope decision, not a claim that one space is universally optimal. Do not silently interpret data textures as color or combine incompatible spaces.

Implement supported transforms in Rust with test vectors; disclose the supported configuration subset instead of claiming full OpenColorIO equivalence. Preserve exposure, display transform, alpha convention and output metadata in render receipts. Use explicit premultiplied/straight-alpha boundaries. Save high-dynamic-range linear output through a Rust codec where supported, alongside display-referred preview output.

### 15.4 Quality and diagnostics

Support named passes, depth, normals, albedo, object IDs and motion when implemented. A first denoiser can be a documented Rust spatial filter, followed by temporal/guided approaches with ghosting tests. No non-Rust neural denoiser fallback is permitted. Test energy behavior, PDF consistency, white-furnace cases, image means and variance; never validate correctness only by attractive demo images.

Raster preview is explicitly approximate. Changes in quality profile must be visible in the API and UI; an agent cannot unknowingly compare images rendered under different material or color policies.

## 16. Animation, rigging, and deformation

### 16.1 Authored animation

Store clips, typed channels, keyframes, interpolation/tangent rules, events, layers and time mappings as first-class data. Address channels by stable entity/component/property IDs rather than fragile string paths. Rational time handles frame-rate conversion and sequencing; floating time is a computation detail.

Retain authored Euler orders and quaternion modes. Support stepped, linear and cubic curves with explicit extrapolation. Do not simplify every animation into sampled transforms. Retargeting and non-linear clip blending are named operations with documented reference poses, spaces, weights and additive semantics.

### 16.2 Rig representation

A skeleton has stable joint IDs, parent relationships, rest/bind transforms, coordinate conventions and constraints. Pose is evaluated data, not a mutation of rest state. Control rigs and deformation skeletons may be separate, linked by explicit operations. Constraints form a computation graph with solver islands for intended cycles.

Define linear blend skinning first; add dual-quaternion or other deformation modes as separate semantics, including behavior under scale/shear. Bind matrices and weight normalization are explicit. Shape keys/morph targets reference a defined topology and fail clearly when correspondence is lost. Corrective deformation and drivers use a typed expression system, not arbitrary embedded Python.

### 16.3 Evaluation and caching

Compile frequently evaluated rigs into compact execution plans rather than traversing the generic document schema at every joint. Schedule independent branches and vectorize measured hot paths. Cache keyframe lookup, constant channels, dependency plans and deformation resources separately.

Motion blur samples evaluated motion over a declared shutter interval. Rigid transforms, deformation and changing topology need different representations. Interpolating a matrix or two unrelated meshes is not a universal motion-blur solution. Changing topology can require explicit time samples and documented support restrictions.

### 16.4 Acceptance criteria

Reference fixtures cover channel interpolation, parent changes, constraint order, IK reachability, non-uniform/negative scale, bind poses, weight edge cases, shape keys and time remapping. Test forward and random-access timeline evaluation. A rig graph must explain a solver failure rather than silently produce a last-known pose as current output.

The first animation release can target a clear character workflow: rig a mesh, animate a clip, scrub it, render a sequence and export supported deformation. Advanced retargeting, sophisticated constraint families and facial systems expand through separately tested capability groups, not one “animation complete” checkbox.

## 17. Modeling, UVs, painting, and sculpting

### 17.1 Operations before editor tools

Implement mesh creation, transforms, extrude, inset, bridge, dissolve, bevel, split/weld and selected repair operations through the topology kernel and semantic API. Add subdivision, mirror, array, solidify, Boolean, remesh and deformation operators as separate contracts. Specify degeneracy handling, attribute propagation and resource growth for each.

A modifier’s node form and direct command share implementation semantics. Destructive “apply” captures the evaluated result and provenance; the original procedural graph remains recoverable through revision history. Exact-ish predicates where needed do not imply an exact CAD kernel or globally exact arithmetic.

### 17.2 Selection and human interaction

A selection is a revision-bound set or query result over explicit domains. Object, face, edge, point, UV-corner and paint-mask selections are distinct. The future editor converts gestures into these targets. Tools can maintain ephemeral state while previewing, but the committed operation is reproducible without the gesture stream.

The mesh kernel permits non-manifold content where meaningful. Individual operators may require manifold input and must report a precondition error; they must not corrupt the asset while trying to guess a repair.

### 17.3 UV and painting systems

UV unwrap and packing operate on corner-domain charts with seams, constraints, texel-density goals and pinned coordinates. Support multiple named UV sets. Painting uses tiled image assets, brush definitions, masks, layers and explicit color roles. Projected painting records its geometry/camera revision. UDIM-style tiling is a later named IO/asset capability, not a reason to bake every texture into one atlas.

Brush strokes are parameterized input with sampled position, pressure, tilt and time when available. Record enough information to reproduce the accepted delta, while using sparse tile updates rather than repainting entire images. Baking is a resource-bounded render/evaluation job with explicit source and target assets.

### 17.4 Sculpting and remeshing

Support multiresolution displacement over a base mesh and localized spatial indices for brush queries. Dynamic topology/remeshing is a distinct mode with weaker correspondence guarantees. Do not force all sculpt data through dense f64 vertices or update all GPU geometry for a local stroke.

Symmetry, masks, face sets, smoothing and brushes share the transaction model. Large gestures can use checkpoints and chunk deltas so cancellation/undo remain practical. Tests cover locality, normal updates, level transitions, seams, topology invalidation and long-stroke recovery. Retopology tooling is a separate consumer of spatial queries and editing operations.

## 18. Simulation and physically meaningful state

### 18.1 Common simulation contract

A simulation component declares solver type/version, physical units, initial conditions, collision inputs, forces, timestep policy, numerical tolerances and random seed. Evaluated state is checkpointable and versioned. Renderable geometry is derived from simulation output; collision proxies and solver discretization are not the authoritative visible asset.

A solver step consumes prior state and produces new state plus diagnostics. It must not mutate arbitrary scene properties while the dependency scheduler is running. Parameter changes invalidate the correct future cache range. Replay depends on the solver version and numerical profile; cross-device bitwise identity is not a universal guarantee.

### 18.2 Delivery families

Begin with rigid bodies, contacts and constraints; add cloth/soft bodies, particles and hair dynamics; then sparse-grid smoke/fire and liquids with meshing. Each family has independent acceptance evidence. A common force/collision interface enables interaction but does not erase solver-specific needs. Coupled solvers require an explicit synchronization and energy-exchange policy.

Use Rust implementations throughout. Do not hide a C++ physics engine or fluid solver behind an FFI crate. Evaluate Rust ecosystem options against required contacts, stability, features, WASM support and licensing; fill gaps natively instead of assuming every library is a production-ready Blender replacement.

### 18.3 Robustness and budgets

Validate mass/density, collision shapes, time steps and boundary conditions according to each solver’s physical model. Handle unit conversion once at input boundaries. Admit jobs against memory and state-growth estimates. Adaptive substeps are bounded and report when stability criteria cannot be met under budget.

Checkpoints have content identities, dependency signatures and integrity validation. Seeking backward should not accidentally reuse a checkpoint from another branch. Intermediate state can be discarded without altering authored inputs. Browser previews may run a reduced-resolution explicit profile; they must not claim final-simulation equivalence.

### 18.4 Tests

Use analytic limiting cases, conservation/tolerance checks, penetration/error bounds, convergence under timestep refinement, and regression scenes. Some algorithms intentionally damp or dissipate energy, so tests must match their model rather than assert perfect conservation everywhere. Separate physical plausibility, numerical stability and artistic controllability.

The major-suite milestone requires a representative scene for every promised family: not merely schema definitions for future solvers. Simulations too expensive for the local browser profile can run on an explicitly selected Rust worker while the local baseline remains independently usable.

## 19. Compositing, drawing, tracking, and audiovisual assembly

### 19.1 Compositor

Use a typed image graph with explicit resolution, pixel aspect, channels, color role, alpha convention and temporal dependencies. Execute demand-driven tiles/regions where operations permit; global filters advertise whole-image requirements. Fuse compatible kernels without changing mathematical semantics. Preserve source render-pass metadata and never assume every image is display-encoded RGB.

Deliver transforms, masks, grading, blur, keying, layer operations and pass combination before advanced temporal effects. Cache by input images, time, node semantics and color policy. Compositor UI and agent API invoke the same graph.

### 19.2 2D/3D drawing

Represent drawing layers as timed stroke/curve assets with pressure, radius, opacity, materials, fills, placement and optional surface attachment. Preserve strokes rather than immediately baking them into meshes or images. Derived tessellation/fill geometry is disposable. Layer compositing and animation have explicit semantics.

This covers the creative family associated with Grease Pencil without requiring a byte-compatible internal clone. Stroke editing, onion-skin views, modifiers, drawing animation and mixed 2D/3D rendering receive separate tests and capability flags.

### 19.3 Tracking and reconstruction

Store footage identity, camera intrinsics/distortion models, feature tracks, observations, confidence, solved poses and reconstruction artifacts. Implement native Rust detection/tracking and optimization with robust losses and convergence reports. Manual edits are authored constraints, not overwritten by a background solve.

Lens models and coordinate conventions must be consistent with rendering. Validate synthetic scenes with known camera motion before relying on real footage. Do not imply that opening video or loading tracks constitutes a camera solver.

### 19.4 Sequencing and audio

A timeline contains clips, tracks, rational-time edits, retiming, transitions, effects, audio channels and synchronization rules. Keep scene animation time and editorial time mappings explicit. Audio processing is stream/block-based with deterministic sample addressing; it is not a render-frame callback.

Start with image sequences and uncompressed or approved Rust-decoded audio. Add native Rust containers/codecs only when their dependencies and behavior satisfy the language policy. Broad video-codec parity is a significant milestone risk. Do not substitute FFmpeg or OS codec services under the fully Rust baseline. Export unsupported media as supported sequences with a clear report, not a false claim of video-format support.

### 19.5 Independent subgates

Compositing, drawing, sequencing/audio and tracking can progress independently after shared image/time foundations. Each requires a complete user workflow and round-trip test. These are major features, not optional leftovers after the mesh editor is finished.

## 20. Native and browser editors, platform services, and UX

### 20.1 Shared application, different hosts

The native host and browser host use the same document, operation, evaluation and renderer semantics. The browser build must not become a separate simplified scene engine. Host interfaces cover storage, execution, time, entropy, input, networking, logging, clipboard and graphics surfaces.

Keep the CPU reference path and schema validation usable without a window. Use feature-gated platform adapters rather than pervasive platform conditionals inside algorithms. Rust desktop/browser UI libraries are candidates, not permanent architectural commitments. Prove text input, docking, accessibility, large inspectors, node canvases and timeline interactions before freezing a toolkit.

### 20.2 Browser resource model

Run substantial evaluation away from the UI event loop using workers where supported. Threading is an optimization, not a correctness requirement. Shared-memory web execution has security/context requirements; provide a valid non-shared-memory execution profile rather than assuming every deployment enables it. [S14]

Use chunked assets and bounded working sets. Browser persistence can use origin-private storage, which is subject to browser-managed quota and lifecycle behavior. It is not a substitute for a user-controlled backup/export. Test storage exhaustion, interrupted writes and recovery. [S15]

GPU resources and WASM heap budgets are separate. Chunk uploads and avoid unnecessary CPU readbacks. Do not assume native hardware features appear through WebGPU. Publish minimum feature requirements and actual negotiated capabilities instead of broad claims that every browser runs every scene.

### 20.3 Human editor principles

Provide object/asset browsing, viewport inspection, property editing, node graphs, timeline, modeling, UV, painting and sculpting workspaces progressively. Workspaces are views of a document, not independent mutable worlds. Use the same diagnostics, operation schemas and history that agents see.

Gestures must be responsive through transient previews; one final intent is committed where appropriate. Display a visible pending state for expensive evaluation. On a conflict, show the candidate changes and current revision rather than silently replacing either.

### 20.4 Accessibility and input

Define keyboard navigation, command search, focus behavior, screen-reader semantics, configurable scale, tablet pressure and high-DPI behavior as acceptance items. Agent-first does not excuse a second-class human experience. Provide inspectable controls and operation descriptions that assistive tools can use without scraping custom GPU pixels.

Remote execution remains opt-in. Local browser create/save/render workflows must work under their named profile without sending project data to a server.

## 21. Native interoperability and source provenance

### 21.1 Interchange strategy

Make the native document the source of truth. Implement Rust import/export adapters with explicit format/version profiles, supported features, approximation rules and loss reports. Initial interchange targets can be glTF/GLB and OBJ with limited material scope; retain source assets and conversion reports. Interchange support is a matrix, not an extension-name checkbox.

The requested expansion beyond the foundation is tracked in `planning/input_support.json` and `docs/input_support.md`: complete static scenes, richer materials, animation/rigs, hair, volumes and native `.blend` profiles. Each track requires an import-save-reopen-render workflow and native/browser evidence before capability status changes. Existing milestone dependencies and the Rust runtime boundary continue to apply.

Later support selected USD/USDA, MaterialX, Alembic-style caches, volume formats, and `.blend` subsets where justified. These are native implementations, not wrappers around the corresponding C++ libraries. Full USD composition or arbitrary `.blend` evaluation is not an implied requirement of reading a mesh from such a file.

### 21.2 `.blend` limits

Blender’s loader includes substantial versioning and dependencies on broader application subsystems; loading file blocks is not enough to reproduce procedural evaluation. The earlier source review identified this in `blenloader/CMakeLists.txt` and the evaluated-data APIs. [S16, S17]

A Rust `.blend` reader starts with selected versions and static data. Report unsupported modifiers, node semantics, drivers, simulations, linked libraries and add-on data individually. Never claim complete fidelity by flattening a scene without saying so. Preserve unknown source payloads where safe, but do not treat preserved bytes as editable or executable support.

No production Blender conversion worker is permitted. Development fixtures may be exported by Blender outside the product and committed as data with provenance and permissions. Their existence must not become an undocumented runtime prerequisite.

### 21.3 Compatibility classification

Track native authoring, evaluation, rendering, editing, import and export independently. Use states such as planned, implemented-experimental, supported, approximated, read-only-preserved and unsupported. A feature is supported only for named semantics and tested profiles.

### 21.4 Licensing and provenance

Blender is GPL-licensed. Reading or translating source does not automatically establish an independently licensable implementation; neither does changing languages. Decide the project’s license and contribution policy before code reuse, maintain file-level provenance, and obtain legal review for derivative-work and distribution questions. This document is an engineering plan, not a legal determination. [S18]

A practical baseline is an open-source, GPL-compatible strategy while recording whether code is independently implemented, adapted or directly ported. Do not promise a permissive license for the entire project while porting GPL implementation details. Audit references, test assets, fonts, shaders, codecs and model weights independently; do not assume the application license covers them.

## 22. Crate structure, dependencies, and engineering governance

### 22.1 Suggested workspace boundaries

```text
crates/
  foundation/     ids, units, math, diagnostics, hashes
  schema/         domain and operation definitions
  document/       typed tables, revisions, composition
  assets/         chunks, manifests, resolution, storage
  geometry/       canonical meshes, curves, fields
  topology/       mutable edit connectivity, validation
  evaluation/     dependency plans, caches, scheduling API
  operations/     reviewed semantic operation registry
  kernel_ir/      Rust-authored GPU program construction
  shading/        material semantics and lowering
  render_cpu/     reference light transport and BVHs
  render_gpu/     generated kernels and GPU resources
  animation/     channels, rigs and deformation
  simulation/    solver families and checkpoint contracts
  imaging/       image/color/compositor primitives
  media/         drawing, tracks, timeline, Rust codecs
  interchange/   scoped native format adapters
  protocol/      wire messages and artifact transfer
  ffi/           small Rust-implemented C ABI
  host_native/   OS services, job workers, CLI/service
  host_web/      WASM services and browser bindings
  editor/        shared Rust interaction/UI layer
```

These are logical boundaries, not an instruction to create 21 empty crates before the first vertical slice. Start with enough crates to enforce real dependency direction; split as interfaces become concrete. Cross-domain integration belongs in orchestration, not circular imports.

### 22.2 Dependency decisions

Candidate foundations include wgpu/Naga for graphics translation, Rust math/storage/serialization utilities, Rust image codecs and Rust UI/windowing libraries. No candidate is adopted solely because its package description says “Rust.” Review runtime native linkage, browser support, license, maintenance, determinism needs and malicious-input behavior.

Pin versions and enabled features. Keep external types out of persistent/public schemas. Dependency updates require reproducible builds, conformance tests and shader/image regression checks. Keep unsafe code small and audited; it is permitted where justified, not used to recreate pervasive mutable pointer graphs.

### 22.3 API and architecture governance

An architecture decision record states the problem, alternatives, selection, consequences and revisit trigger. Stable operation changes require semantic version review and migration guidance. Every feature declares resource effects and failure modes before entering the public registry.

An integration owner reviews cross-crate changes. Agents may implement bounded tasks against fixtures, but cannot weaken tests, hide unsupported features, add native dependencies or change public semantics to make a demonstration pass. Reference outputs must not be regenerated by the same patch without independent review.

## 23. Verification, performance, and release quality

### 23.1 Layered verification

Use unit tests for math and schemas; property tests for topology/revision invariants; metamorphic tests for valid transformations; numerical tests for solvers/rendering; fuzzing for parsers and FFI descriptors; concurrency/model tests for commit and cache coordination; and end-to-end creative workflows.

Blender is one external oracle, not the only authority. Compare normalized geometry, attributes and evaluated motion where semantics match. Use analytic cases and independent reference formulations to catch shared implementation mistakes. Rendering comparisons require matching camera, materials, color transforms, sampling and intentional approximations. Do not demand pixel identity across unrelated integrators.

### 23.2 Reproducibility levels

**Exact document behavior:** accepted deltas, canonical snapshots, IDs and deterministic pure operations under the same semantic version. **Numerically bounded behavior:** CPU/GPU evaluation and geometry algorithms with stated tolerances. **Statistical behavior:** stochastic rendering and selected simulation outputs. Every test declares its class.

Reproducibility receipts record code/operation versions, asset digests, seed, backend/device class, numerical policy and color configuration. A seed alone is not a complete experiment specification.

### 23.3 Proposed benchmark profiles

| Profile | Initial fixture target | What it proves |
|---|---|---|
| Small | 100 objects; 100,000 triangles | Browser/native correctness and interactive overhead |
| Mixed | 10,000 objects; 1 million unique triangles | Scene traversal, metadata queries and incremental updates |
| Instanced | 100,000 instances of 10,000 unique triangles | No accidental instance realization |
| Edit stress | Mesh with 5 million corners | Local-edit memory amplification and topology traversal |
| Recovery | Repeated commits with injected termination/quota failures | Valid durable roots and no partial accepted changes |
| Agent batch | Multiple branches with bounded render variants | Idempotency, isolation, budgets and provenance |

Counts are proposed fixtures, not demonstrated capacity. M00 records actual hardware/browser/toolchain and tunes useful stress points. Add hair, sparse-volume, rig and media fixtures with those features.

### 23.4 Initial service-level targets

Propose p95 metadata query latency below 25 ms and uncontended small transaction commit below 50 ms on a recorded reference machine, excluding expensive evaluation and disk durability where separately measured. Target interactive transform evaluation within a 16.7 ms budget for the declared profile; do not assert a universal 60 FPS guarantee.

Require no mesh-buffer rebuild on transform-only changes, bounded queue growth, cancellation acknowledgment independent of heavy work, and memory charges within configured budgets. GPU cancellation completion is measured, not promised as hard preemption. Correctness failures block release; performance regressions require explanation and explicit budget review.

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

## 29. Risk register, research decisions, and first backlog

### 29.1 Highest-risk items

**GPU kernel authoring:** a Rust-only maintained source path may be awkward or slow. Prove representative kernels early; keep the compiler boundary private. Do not silently relax the language policy.

**Geometry robustness:** Booleans, bevels, subdivision, remeshing and attribute correspondence contain difficult degeneracies. Use adversarial fixtures and independent numerical validation; restrict unsupported inputs explicitly.

**Renderer breadth:** scattering, volumes, displacement, denoising, color and device behavior are separate engineering areas. Grow measured capability profiles instead of claiming Cycles equivalence after a path-tracing demo.

**Media and interoperability:** native Rust codec/format coverage may be incomplete. Ship supported image/audio/cache profiles and develop missing functionality; neither wrappers nor a Blender worker satisfy the requirement.

**Revision memory costs:** topology changes can defeat locality and content sharing. Measure snapshots, compaction and history retention under wide edits; preserve semantics while replacing private layouts.

**Browser scale:** memory, storage, threading and device capabilities vary. Maintain a named local baseline, test recovery, and expose limits before accepting oversized jobs.

**Scope growth:** track native implementation and end-to-end workflows separately. Never add placeholder schemas to claim coverage. Apply release gates and a limited work-in-progress policy.

### 29.2 First implementable backlog

Create the workspace and language/dependency gate; define IDs, units/time and error codes; implement canonical mesh fixtures and radial topology experiments; prototype chunked revisions against flat tables; build Rust-generated intersection/shading kernels; specify transaction/idempotency behavior; prototype durable storage on both hosts; implement a small CPU-rendered scene; and record ADR outcomes before broad API freeze.

Each task needs an owner role, input/output contract, positive and negative tests, benchmark fixture and artifact proving completion. Tasks are not complete when code compiles alone. Initial task details are included in the planning bundle.

### 29.3 Decision changes

A change to a private representation needs evidence and regression tests. A public semantic change needs a version, compatibility impact and migration. A change to the fully Rust requirement, native/browser product scope or external-runtime prohibition requires an explicit product decision; it cannot be made opportunistically by an implementation agent.

**Recommendation:** execute M00–M05 as a genuine product-building program. Preserve the full-suite data and contract boundaries now, but implement each major creative capability through measurable native Rust milestones.

## 30. Sources and evidence boundaries

This is a proposed architecture, not a claim that the described engine or prototypes exist. Blender source observations come from the targeted review in the preceding discussion at commit `52f4f930332e5fe8a3e0bb33648a1b1672acdf06`. Supporting public documentation was checked on September 5, 2026. Some Blender pages were available through indexed descriptions rather than successful full-page fetches. No new full repository audit, compilation, renderer comparison or hardware benchmark was performed for this document.

Architecture choices, milestone definitions and numerical targets are original recommendations. Sources establish relevant existing behavior or technical constraints; they do not endorse this design or prove its performance. Live documentation can change; implementation work must pin source/toolchain versions and archive applicable specifications.

### Blender and scene representation

**[S01] Blender Developer Documentation — BMesh.** Non-manifold editing-topology precedent. https://developer.blender.org/docs/features/objects/mesh/bmesh/

**[S02] Blender Developer Documentation — Mesh Data Structures.** Distinction between compact mesh storage and editing structures. https://developer.blender.org/docs/features/objects/mesh/mesh/

**[S03] OpenUSD — Introduction to USD.** Composition, layers, instancing, identity trade-offs and distinction from rigging. https://openusd.org/release/intro.html

**[S04] Blender source — BKE_geometry_set.hh, reviewed pinned commit.** Typed geometry components and shared ownership. https://github.com/blender/blender/blob/52f4f930332e5fe8a3e0bb33648a1b1672acdf06/source/blender/blenkernel/BKE_geometry_set.hh

### Rust, GPU, and material contracts

**[S05] The Rust Reference — External blocks.** Native Rust ABI instability and C-compatible ABI distinction. https://doc.rust-lang.org/reference/items/external-blocks.html

**[S06] The Rustonomicon — FFI.** Ownership and unwinding considerations. https://doc.rust-lang.org/nomicon/ffi.html

**[S07] Naga crate documentation.** Shader translation, validation and backend facilities. https://docs.rs/naga/latest/naga/

**[S08] wgpu official site.** Rust graphics/compute across native and browser backends. https://wgpu.rs/

**[S09] wgpu — Limits.** Device limits, allocation caveats and native/experimental capabilities. https://docs.rs/wgpu/latest/wgpu/struct.Limits.html

**[S10] GPU for the Web — WebGPU Explainer.** Device negotiation, asynchronous errors and device loss; explanatory document, not itself the normative standard. https://gpuweb.github.io/gpuweb/explainer/

**[S11] MaterialX official site.** Typed representation of appearance and shading relationships. https://materialx.org/

**[S12] MaterialX PBR specification.** Scattering, emission and volume-related types. https://github.com/AcademySoftwareFoundation/MaterialX/blob/main/documents/Specification/MaterialX.PBRSpec.md

**[S13] Physically Based Rendering, fourth edition — A Better Path Tracer.** Light transport and sampling reference. https://pbr-book.org/4ed/Light_Transport_I_Surface_Reflection/A_Better_Path_Tracer

### Browser storage, compatibility, and license

**[S14] MDN — SharedArrayBuffer.** Shared-memory security and deployment requirements. https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/SharedArrayBuffer

**[S15] MDN — Origin private file system.** Browser-managed private storage and quota behavior. https://developer.mozilla.org/en-US/docs/Web/API/File_System_API/Origin_private_file_system

**[S16] Blender source — blenloader/CMakeLists.txt, reviewed pinned commit.** Loader dependencies and versioning breadth. https://github.com/blender/blender/blob/52f4f930332e5fe8a3e0bb33648a1b1672acdf06/source/blender/blenloader/CMakeLists.txt

**[S17] Blender source — DEG_depsgraph_query.hh, reviewed pinned commit.** Authored/evaluated data and dependency queries. https://github.com/blender/blender/blob/52f4f930332e5fe8a3e0bb33648a1b1672acdf06/source/blender/depsgraph/DEG_depsgraph_query.hh

**[S18] Blender — License.** Blender’s GPL licensing statement. https://www.blender.org/about/license/
