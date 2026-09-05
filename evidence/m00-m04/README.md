# M00–M04 foundation evidence

The foundation gates are implemented for the named experimental profile below. This is a local Rust document/rendering system with a small ABI alpha and browser execution. M05 product integration and ABI v1, and M06–M14 authoring breadth, remain planned.

| Gate | Delivered behavior | Evidence |
|---|---|---|
| M00 | Chunk/flat/hecs comparison; radial topology experiment; native/browser generated traversal and shading; crash/storage and dependency audit | `benchmark_results.json`, `kernel_cross_target_report.json`, `storage_recovery_report.json`, `dependency_inventory.json`, `accepted_ADRs.md` |
| M01 | Persistent IDs, typed transforms/time, polygon attributes/seams/loose/nonmanifold data, shared instances, immutable branches and native binary mesh storage | `document_conformance.json`, `geometry_invariants.json`, `roundtrip_report.json` |
| M02 | Validated candidates, atomic revision checks, retained idempotency, uploads, durable resolved journals, persistent native jobs/events and independently exercised ABI v0 | `api_conformance.json`, `idempotency_faults.json`, `durability_faults.json`, `abi_conformance.json`, `abi_alpha_contract.md`, `logs/native_tests.txt` |
| M03 | Native API-built scene, f64 reference renderer, diffuse transport/textures, image/pass receipts and scoped OBJ/glTF/PPM codecs | `renderer_cpu_conformance.json`, `reference_images_manifest.json`, `interchange_profile_report.json` |
| M04 | Rust-generated software traversal, native Metal/browser WebGPU, progressive batches, raster preview, numerical comparisons and recovery diagnostics | `cross_backend_report.json`, `device_capability_manifest.json`, `gpu_fault_report.json` |

## Validation

`validation_report.json` retains exact commands, exit status, available durations and log paths. Terminal progress padding and blank lines at the ends of logs are trimmed; diagnostic and result text is retained. `test_inventory.json` lists 40 native tests and 31 WASM tests. Native includes the real writer-process termination harness, durable job host, FFI lifecycle and generated-kernel validation. WASM core tests execute under Node; the separate Chromium suite exercises OPFS, public bindings, WebGPU and a dedicated nonshared module worker.

Strict native and WASM Clippy, formatting, native builds, wasm-bindgen generation, header correspondence, independent ctypes calls and dependency audit pass. Linux and Windows host targets compile; they have not been run. Browser evidence records Chrome 152.0.7977.76. Native GPU evidence identifies Apple M1/Metal. No remote rendering service is used.

The constructed textured instance scene gives CPU/GPU RMSE approximately 3.48e-8 on Metal and 3.30e-8 in Chromium. The independently calculated Lambertian plane has maximum channel error 2.98e-8 against a 1e-6 tolerance on both GPU hosts. PDF/energy/sample-mean and analytic ray/empty-environment tests run in the shared core. Generated images are observed artifacts with hashes and receipts; they are not substituted for analytic or invariant tests.

Native storage tests terminate the writer after pending write, file flush, rename and directory flush. Browser tests abort replacement writes, impose a real origin quota failure and reload the tab with an unclosed replacement stream. The previous committed document remains recoverable. Corrupt committed mesh chunks produce an integrity error. These tests do not simulate a physical power outage.

GPU fault tests exercise cancellation before submission and immediately after actual queue submission, a real over-limit buffer allocation, explicit device destruction and complete device/resource reconstruction. Cancellation drains submitted work and discards its result; it does not preempt the GPU. Reports record the cancelled render's total pack/submit/drain time. Spontaneous driver resets and physical VRAM exhaustion were not induced.

## Measurements and decisions

The optimized representation experiment uses 10,000 entities and 20 samples. Median branch times in this run were approximately 0.104 ms for typed chunks, 1.190 ms for a flat copy and 2.482 ms for the explicit hecs component-copy branch. Typed chunks were slower for the measured wide edits. This supports retaining replaceable chunked authoring state, not a universal ECS performance claim. After a single edit, 156 of 157 entity chunks remain shared.

A 10,000-quad/40,000-corner grid measured 11.58 ms for edit-view conversion, 7.42 ms for commit validation, and 14,543,880 bytes of connectivity heap capacity excluding the retained canonical mesh and allocator overhead. Full benchmark peak RSS was 82,411,520 bytes; that is a whole-process measurement including all candidates, not per-representation allocation. Raw timings and scope notes are retained. This is not the later five-million-corner capacity gate.

`accepted_ADRs.md` records decisions, losing cases and revisit triggers. `dependency_inventory.json` and `source_manifest.json` identify dependency and generating inputs. `reference_images_manifest.json` identifies retained output bytes. The [implementation review](review.md) records six corrected findings and their regression tests.

## Capability profile and limits

- CPU: polygon-derived triangles, shared geometry BVHs and instance BVH, two-sided Lambertian reflection, point light, constant environment, emission, nearest/repeating linear textures, bounded finite depth. GPU: the matched one-bounce profile. The Lambertian renderer rejects metallic != 0 or roughness != 1 with a typed diagnostic.
- Raster preview: depth-tested flat approximate shading; texture/transport parity with final output is not claimed. PPM display output, linear PFM, depth, normals, object IDs and receipts are available.
- Geometry: f32/f64 positions, affine/ordered transforms, polygon/corner topology, dense typed attributes, simple planar face triangulation, radial connectivity and point moves. Full modeling, sparse sculpt locality, animation and advanced geometry remain later milestones.
- Native package v0: immutable roots plus deduplicated binary mesh chunks. Browser: bounded OPFS fixture persistence plus general canonical JSON import/export. Browser storage is origin-managed; user export is the backup mechanism. Migration stability is not promised before format stabilization; keep source packages and export before upgrades.
- API v0: trusted local principal for CLI/browser/ABI; revision-checked transactions, up to 256 commands/request, 16 MiB wire requests, 128 MiB authored-state profile, 10,000 retained idempotency keys. ABI is memory-only, process-local and unstable. Jobs have bounded queues/events, pinned inputs and persisted native lifecycle; browser uses the shared state machine but has no durable browser job worker service.
- GPU: negotiated baseline limits, camera-relative f32, exact packed addressing cap, bounded samples and explicit rejection of oversized buffers/dispatches. No hardware ray feature is required. Resource admission is an estimate, not a proof of all driver/allocator residency. Fixed 1e-5-meter ray epsilon and extreme transform conditioning remain numerical limits.
- Interchange: OBJ polygons/corner UVs/loose lines; glTF 2.0 static mesh-only nonindexed triangles with caller-supplied binary and explicit losses. Unsupported animation/skins/extensions/sparse/indexed primitives are rejected. There is no arbitrary glTF, `.blend`, USD, MaterialX or shader-node compatibility claim.

## Reproduction

See [the implementation guide](../../docs/implementation.md) for build, CLI, browser, storage and ABI contracts. Run `python3 scripts/validate-foundation.py` for the offline checks. With GPU access, run `target/debug/render-host verify artifacts/evidence --gpu` and `target/debug/render-host demo artifacts/demo --gpu`. The isolated Chrome/CDP test harness is `node scripts/browser-conformance.mjs`; it connects to the local Rust server on 8765 and test Chrome on 9223.

`scripts/summarize-evidence.py` collects successful reports into ignored `artifacts/foundation-evidence/`. Updating this reviewed evidence snapshot is an explicit copy step. It never rewrites an image oracle or changes tolerances automatically.
