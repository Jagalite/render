# Animated glTF/GLB validation

`gltf2-animated-pbr-v0` passes the native CLI and browser-local import, persistence,
animation and rendering workflows. This advances INPUT-03; M09 and the wider M13
milestone remain open. [The profile contract](../../docs/animated_gltf.md) defines
supported data, limits, migration and losses.

## Results

- **100 native tests and 82 Rust Wasm tests passed.** Wasm tests execute in Node.
- Workspace formatting, strict native/browser Clippy, Linux/Windows compilation,
  current browser build and dependency audit passed. Linux/Windows are compile-only.
- The existing **36-request CLI regression workflow** passed.
- The new **29-request animated CLI workflow** passed: GLB import/retry, random-time
  evaluation, five shutter frames, exact frame/sequence equivalence, native archive
  restore, CPU/Metal renders, stale/cancel/budget failures and external glTF import.
- Chromium independently imported the GLB through Rust, round-tripped OPFS,
  evaluated out-of-order frames, preserved authored state, rejected a stale request
  and cancelled a GPU preview. Its default-pose CPU/WebGPU comparison passed.
- Native/browser shutter-frame RGB RMSE **0**, with matching output digests.
  Native CPU versus browser WebGPU default-pose RMSE **8.66e-9**.
  Native CPU/Metal sampled-frame RMSE **4.94e-9**.
  Every compared object-ID pass had **zero mismatches**.

The [validation report](validation_report.json), [CLI report](workflow/workflow_report.json),
[browser report](workflow/browser_report.json), [cross-platform comparisons](cross_platform.json),
[dependency inventory](dependency_inventory.json), [fixture integrity](fixture_integrity.json)
and [integration review](integration_review.md) retain the scope and exact checks.
The 64×64 [preview](preview.png) is a format conversion of a rendered midpoint
frame for inspection, not a golden image or a visual-quality benchmark.

## Evidence lineage

Runtime/test/build inputs passed their gates in
`artifacts/animated-gltf/run-20260906T023106Z`. The final CLI/browser clients ran in
`artifacts/animated-gltf/run-20260906T023801Z`. The first eleven checks were retained
with hash-verified unchanged inputs instead of repeating completed tests after a
client artifact-path correction. Their report entries identify `reused_from`, and
copied logs retain original commands and paths. [Validated input hashes](validated_inputs.json)
were collected after that failed client check, before the path correction; every
recorded input's modification time predates its validation run.

The final comparison reader was corrected for PFM's bottom-up row convention;
browser pass arrays are top-down. Existing images were rechecked with unchanged
thresholds. [The asymmetric reader sentinel](pfm_reader_check.json) independently
checks row order. No renderer output or image reference was regenerated to pass a gate.

[The run source manifest](source_manifest.json) captures sources at finalization.
[Final implementation integrity](final_source_integrity.json) confirms the tested
Rust, fixtures, Cargo and build inputs remain unchanged; subsequent changes are
documentation and validation-client bookkeeping. [The reviewed source manifest](reviewed_source_manifest.json)
pins the final maintained files. Previous milestone and project-CLI evidence retain
their original source scope and have not been rewritten.

## Resource measurements

Measured on macOS ARM64 with 64×64 images, 8 samples/pixel and the documented
one-bounce opaque PBR profile. The sequence has five frames with three shutter
samples each. Times include CLI startup and journal recovery. RSS is process peak
resident memory; footprint is macOS's separately reported metric. These small
fixtures do not establish general throughput or allocator bounds.

| Operation | Wall seconds | Peak RSS bytes | Peak footprint bytes |
|---|---:|---:|---:|
| GLB import | 0.145 | 17,432,576 | 3,687,552 |
| Default-pose CPU render | 0.368 | 20,217,856 | 4,965,504 |
| Five-frame CPU sequence | 2.017 | 20,627,456 | 5,293,184 |
| Single shutter frame | 0.381 | 20,496,384 | 5,145,728 |
| Sampled CPU scene | 0.271 | 20,316,160 | 4,949,120 |
| Sampled Metal scene | 0.452 | 36,339,712 | 11,781,760 |
| External glTF import | 0.129 | 17,678,336 | 3,622,016 |
| External shutter frame | 0.447 | 20,529,152 | 5,244,032 |

The synthetic snapshot's measured JSON representation is 12,537 bytes. Original
source sizes/hashes and licenses are in [fixture provenance](../../fixtures/animated-gltf/provenance.json).
There is no Cargo dependency or maintained shader delta. Generated Wasm/binding
inputs and output hashes are recorded by the dependency audit.

## Failed experiments and corrections

[Failed evidence](failed-experiments/) is retained, including command durations:

- `native-profile-mismatch`: the new integration test inherited depth two from a
  diffuse fixture; opaque PBR correctly rejected it. The fixture now authors depth
  one explicitly. Rendering support was not widened or silently downgraded.
- `degenerate-fixture`: the initial ribbon's final 180-degree pose collapsed into
  zero-area triangles. The engine correctly rejected it. Original source files and
  generator are retained under that directory. The reviewed positive fixture now
  bends and returns to rest; its independent analytic reference tracks that motion.
- `artifact-path`: the client looked under `frame/` for a scene-render artifact
  published under `image/`. The client paths were corrected.
- `pfm-orientation.txt`: the comparison reader initially compared opposite row
  orders. Correct interpretation yields exact native/browser shutter agreement.
- Initial compile/Clippy and request-key fixture errors are retained in the other
  logs. They were corrected without suppressing diagnostics or weakening gates.

## Reproduction

Build/test everything with `python3 scripts/validate-animated-gltf.py`. It expects
an isolated Chrome CDP endpoint on port 9224 with WebGPU enabled and a loopback
server serving `web/` on port 8766. It builds the current Rust/Wasm application.
`RENDER_TEST_ORIGIN` and `RENDER_TEST_CDP` override the browser client's endpoints.
The browser test uses an isolated OPFS project name; all native outputs use fresh
directories. `--reuse-checks <prior-run>` verifies recorded input hashes before
retaining previously passed build/test checks; CLI/browser acceptance runs again.

For the standalone engine workflow:

```sh
cargo build -p render-host --locked
python3 scripts/animated-gltf-workflow.py /tmp/render-animated-example --gpu
```

The path must not already exist. No demo-specific engine operations, foreign solver,
Blender process, reference-image update or remote runtime service is involved.
