# Accepted foundation decisions

These are implementation decisions for the experimentally validated M00–M04 profile, not a permanent ABI, file-format or capacity guarantee. The original decision register remains the proposal history. Measurements are in `benchmark_results.json`; exact validation commands and logs are retained alongside the reports.

## ADR-001: Rust computational ownership — accepted

The six application crates are Rust. CPU intersection, BVHs, sampling, triangulation, topology, image encoding, native storage and scoped OBJ/glTF codecs execute without Blender/Cycles or non-Rust compute libraries. GPU programs originate in the Rust-owned IR and are validated by Naga. The native link inventory contains operating-system libraries/frameworks only. Browser APIs, GPU drivers and generated wasm-bindgen interoperability are recorded environmental exceptions. Python/Node scripts and hecs benchmarks are external test tools, not application implementations.

Dependency inventories identify versions, enabled features, license expressions, build scripts and native links. The audit combines reviewed dependencies with a denylist; it is not a proof about future additions. Re-run it whenever runtime dependency or feature selection changes. The project has not selected its own distribution license. No Blender source was ported.

## ADR-002: typed chunked authoring state — accepted with measured costs

The experiment compares typed chunks, a flat entity vector and hecs 0.10.5 using 10,000 entities over 20 samples: construction, traversal, branch, single-property edit, wide property edit and shared topology-asset replacement across every instance. hecs reconstructs registered components and stable IDs when branching; this is explicit in the harness. It does not pretend that generic ECS worlds have an equivalent built-in persistence operation.

Chunk sharing reduces the measured branch cost but introduces index copying, per-chunk copy costs and slower wide edits. These trade-offs are retained in the raw samples rather than collapsed into a universal performance score. The private chunk size remains replaceable. The public serialized entity list contains no chunk or runtime index representation. Revisit the index/chunk implementation when larger branch or wide-edit workloads exceed declared budgets.

## ADR-003 and ADR-004: canonical polygons and owned radial handles — accepted

Canonical data preserves n-gons, corner UVs, local identities, loose elements and nonmanifold fans. The manifold-limited candidate is an explicit two-incident-face admissibility test; it fails the three-face-edge fixture. It is not a performance comparison against a complete competing modeling library. Dense adjacency scans and cached radial queries are measured separately.

Editing arenas store face/radial cycles and vertex-edge adjacency. Handles contain an arena owner, slot and generation; foreign and stale handles are rejected. Commit validates connectivity and canonical geometry. A 10,000-quad workload records load, conversion, local/wide moves, validation, triangulation and connectivity heap capacity, alongside equal-result verification against dense edits. The capacity measure excludes the retained canonical mesh and allocator overhead. Peak process RSS is recorded separately by the benchmark launcher.

The current conversion materializes an entire edit view; no local-sculpt or five-million-corner claim follows. Keep dense authored arrays and radial editing semantics; revise private storage and duplication before M06/M10 scale promises. Authored identity, runtime identity and GPU addresses remain different concepts.

## ADR-005 and ADR-009: snapshots, resolved deltas and conservative commit — accepted

Every published root contains recoverable state, retained accepted-request receipts and resolved commands. Native mesh payloads are deduplicated binary chunks. Publication orders chunk flushes before root write, file flush, atomic rename and directory flush. Actual writer-process termination and injected errors test each root boundary. Integrity failures in committed data are surfaced rather than silently skipped. A post-rename failure has uncertain acknowledgment: recover and retry the same idempotency key.

Browser storage uses OPFS close/abort under Web Locks. An actual quota override and tab reload with an unclosed replacement test preservation of the prior committed fixture. Browser origin storage is not an external backup; export is explicit. Host metadata participates in conflict detection so stale jobs/idempotency state cannot be overwritten merely because the authored revision matches.

Revision mismatch and conflicting candidates reject without partial mutation. Jobs persist pinned inputs, states and event cursors; restart turns interrupted running work into an explicit failure. Retention and queue bounds are documented. Automatic topology merges and destructive garbage collection remain outside this profile. Revisit publication and retention before format stabilization or multi-tenant deployment.

## ADR-007 and ADR-011: CPU reference and portable generated GPU — accepted

The Rust IR generates nontrivial software BVH traversal, triangle intersections, cosine sampling, texture reads, direct lighting and output passes. Native Metal, browser WebGPU and a dedicated browser worker execute these programs. Matched scenes are compared with the independent f64 CPU renderer. A constant-environment Lambertian plane has the independent solution `radiance = albedo × environment`; it exercises actual surface transport. PDF/energy/sample-mean, intersection, color, transform and seam tests supplement image comparisons.

The GPU profile has one diffuse bounce; CPU permits bounded finite depth. The separate raster preview uses hardware depth testing and intentionally approximate shading. Baseline device limits are negotiated; excessive buffers/dispatches are rejected. Cancellation is exercised both before and immediately after real queue submission; completed cancelled results are discarded. Explicit device destruction and recreation rebuild disposable state. Over-limit allocation is a real validation failure, not physical VRAM exhaustion. Spontaneous driver faults are not injected.

Affine transforms use f64; geometry declares f32/f64; GPU forms are camera-relative f32 with checked finite conversions and addressing limits. This does not establish robustness at all scales. The fixed ray epsilon, ill-conditioned transforms, large buffer partitioning and broad statistical/device corpora require further qualification. Revisit these private policies on failing scale or device profiles. Hardware ray acceleration is not required.

## ADR-008: small C-compatible ABI — accepted as alpha only

Rust owns a size/version-negotiated function table and generation-checked handles. Independent ctypes calls verify lifecycle, buffers, mutations and retry behavior. The contract is described in `abi_alpha_contract.md`. No Rust trait object layout crosses the boundary. Keep it experimental until M05 proves the product workflow and ABI v1 lifecycle requirements.

## ADR-010: scoped native interchange — accepted

OBJ polygon/UV/loose-line and glTF static mesh-only nonindexed triangle profiles round-trip through Rust codecs. Unsupported glTF features and invalid/truncated data are rejected; intentional OBJ/glTF losses are returned explicitly. PPM color conversion is Rust-owned. These adapters establish initial external asset handling, not full scene or material compatibility. Expand only against named corpus cases and loss expectations.

## ADR-006 and ADR-012: deferred domains

Only the shader IR exists today. General procedural/time/solver graphs and the interactive editor are not delivered by M00–M04. The public browser/ABI/CLI clients already use the same transaction engine; a future editor must preserve that rule. These ADRs remain proposed for their later domain milestones.

## Experiments that changed the implementation

Native recovery exposed different floating-point parses after JSON value expansion, changing mesh hashes. Enabling the pinned serializer's exact float round-trip parser fixed it; precision and native publication regressions now cover the failure. Arena reuse originally needed linear vacant-slot searches; an explicit free list removes that insertion scan. Failed compiles while expanding the benchmark also caught a test-only dependency reference; the harness now uses the mesh's typed position values directly. None of these failures was addressed by reducing acceptance tolerances or substituting expected outputs.
