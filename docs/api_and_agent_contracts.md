# API, ABI and agent contracts

The additive [static PBR slice](static_pbr.md) now has [separate evidence](../evidence/static-pbr/README.md): opaque textures/materials, normals and authored cameras on CPU, Metal and browser WebGPU. M07 and INPUT-01/02 remain partial; the M05 Lambertian profile and historical evidence retain their original scope.

Architectural contracts. The implemented M05 subset, operation versions and platform limits are specified in [the agent alpha guide](agent_alpha.md); broader capabilities below remain requirements.

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

The additive [M06–M08 contracts](m07_m08.md) expose ordinary typed authoring commands, pinned animated previews, CPU extended root rendering and imaging products. These operations preserve the existing narrow ABI and restricted static-variant commit contract.
