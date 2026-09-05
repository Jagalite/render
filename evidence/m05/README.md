# M05 validation evidence

M05 is complete within the [local alpha profile](../../docs/agent_alpha.md).
This extends the M00–M04 foundation; existing foundation evidence was not changed.
All 17 checks in `validation_report.json` passed: **48 native tests and 38 WASM
tests**, strict Clippy, Linux/Windows compilation, native builds, generated ABI
header, Python/C clients, stream faults, dependency audit, native Metal workflow,
browser WebGPU workflow and browser foundation regressions.

| Evidence | What it establishes |
|---|---|
| [Agent workflow](agent_workflow_report.json) | External GLB, three protected variants, CPU/Metal comparison, durable native jobs, structured inspection, selection and reload/retry |
| [ABI v1](abi_v1_conformance.json) | Independent Python ctypes workflow and compiled C11 lifecycle/buffer/negotiation checks |
| [Native client faults](native_client_report.json) | Actual lost commit acknowledgment, process reconnect, incomplete input termination and bounded stream admission |
| [Browser workflow](browser_workflow_report.json) | Shared semantic API, GPU cancellation, OPFS quota/abort/conflicts, metadata-sensitive storage identity, export and unclosed-write tab recovery |
| [Capability matrix](alpha_capability_matrix.json) | Exact supported profiles, budgets, platform qualifications and exclusions |
| [Validation commands/logs](validation_report.json) | Reproducible commands, exit codes, timing and native process resources |
| [Dependency inventory](dependency_inventory.json) | Transitive runtime features, native links, generated artifact identities and narrow platform exceptions |
| [ADRs and review fixes](accepted_ADRs.md) | Integration decisions, failed checks and remaining limits |

The unmodified [Khronos Box source](../../fixtures/khronos-box/README.md) is by
Cesium, CC-BY-4.0. Synthetic index-width/hierarchy/primitive tests derive changes
in memory and retain the original fixture. This profile explicitly converts its
material to Lambertian; it does not implement glTF dielectric specular.

The selected variant is **green**, chosen by a fixed mean-green
objective. Native CPU/Metal RMSE was at most
1.75e-08; browser CPU/WebGPU RMSE
was at most 5.81e-09. The gate is
0.02 with at most four object-ID mismatches per image. These are named-fixture
measurements; analytic Lambertian, geometry, normal and depth assertions remain
in the test suite. They are not a broad rendering-fidelity claim.

The final debug native workflow took **3.792 seconds**;
maximum RSS was **33,292,288 bytes**
and peak footprint **16,139,840 bytes**.
Browser workflow timing was 0.072 seconds on
Chrome/152.0.7977.76. Neither timing generalizes to other scenes or
hardware. Browser tab total memory and physical VRAM usage were not measured.

`renders/` contains the actual CPU/GPU PPM/PFM outputs, receipts and structured
passes. These are new run evidence, not regenerated acceptance goldens. Visual
inspection confirmed a visible green cube with distinct lit faces; structural,
numerical and failure checks determine acceptance. `selected-document.json` and
`selection-request.json` make the chosen result reviewable. The native journal
and job artifacts remain reproducible under the recorded artifact directory.

`source_manifest.json` identifies the tested implementation and fixture inputs
on the working branch. Re-run `python3 scripts/validate-agent.py` using the local
server and isolated Chromium launch in the alpha guide. No push or commit was
performed for M05. Full static glTF/PBR, animation, hair, volumes, Blender input,
remote collaboration and a visual editor remain separate future gates.
