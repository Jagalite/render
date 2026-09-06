# GPU shutter acceptance

Passed on `feat/gpu-shutter-sequences`, based on `4bfe27f`. The
[contract](../../docs/gpu_shutter.md) adds native Metal/browser WebGPU frames and
streaming sequences to the exact-time animation API. CLI backend selection is
optional and defaults to CPU. M09 remains deferred.

The fresh run is `artifacts/gpu-shutter/run-20260906T042706Z`.
[Validation](validation_report.json) records 105 native tests, 86 Rust Wasm tests,
two explicit Metal test groups, strict native/browser Clippy, format, native/web
builds, dependency audit, and Linux/Windows compilation. Those two platforms have
compile evidence only.

The [15-request CLI workflow](workflow/workflow_report.json) imports the original
rigged/morphed character, compares CPU/GPU shutter frames, renders five GPU frames,
exports/restores the document, reproduces frame pixels, rejects stale/invalid and
output-limited requests, and cancels after exactly one published frame. Existing
32-request general CLI and 29-request animated-input workflows also pass.
[84 prior artifacts](cpu_and_static_regression_identity.json), including images,
passes and receipts, are byte-identical to the previous validated run.

The [browser workflow](workflow/browser_report.json) tests the actual release Wasm
package in Chrome WebGPU: frame/sequence identity, awaited consumer backpressure,
immediate cancellation, cancellation after one acknowledged frame, rejected
consumer acknowledgement, stale frame completion, stale sequence completion after
one acknowledged frame, and OPFS restore. Animated previews cannot qualify a static
commit. Authored root state remains unchanged.

Beyond backend agreement, the real Metal test independently averages complete
static GPU images on the CPU and compares with device temporal accumulation. It
covers rigid motion and native LBS/morph deformation, exact nominal passes, random
access, sequence order/identity, real mid-shutter cancellation and a fresh render
afterward. Shared tests preflight an overflowing later time before any publication.
No golden output is rewritten.

For this 64×64, eight-spatial-sample, three-temporal-sample, depth-two fixture,
CPU/Metal RGB RMSE is 4.59515e-9 and CPU/WebGPU RMSE is 4.17908e-9. Native CPU and
browser Wasm match exactly; all object IDs match. The tolerance is RMSE <0.002 with
exact object IDs. [Cross-platform comparisons](cross_platform.json) account for
bottom-up PFM rows. The [preview](preview.png) is a PPM-to-PNG format conversion,
visually inspected as the bent ribbon at nominal time; it is not a reference image.

[Resource observations](resources.json) include process startup, driver and artifact
I/O, measured once on a machine doing other work. They are not speed benchmarks:

| Operation | Wall seconds | Maximum RSS bytes | Peak footprint bytes |
|---|---:|---:|---:|
| CPU shutter frame | 0.374 | 20,594,688 | 5,063,808 |
| Metal shutter frame | 0.339 | 36,929,536 | 12,273,216 |
| Metal five-frame sequence | 0.986 | 37,437,440 | 13,010,560 |

This fixture uses a 131,072-byte persistent output buffer, one full readback of that
size, and three intermediate four-byte fences per frame. Actual packed scene,
texture and parameter buffers are also admitted against the job/device limits.
Disposable temporal evaluators bound retained derived geometry; their rebuild cost
has not been separately benchmarked. The engine does not claim a whole-process
allocator cap or dispatch preemption.

[Dependency delta](dependency_delta.json) confirms unchanged Cargo manifests and
lockfile. [Inventory](dependency_inventory.json) retains feature/platform exceptions.
The [generated WGSL](generated_path.wgsl) has [Rust generation inputs](generated_shader.json).
[Package hashes](tested_package.json) identify the tested native binary and release
browser bindings. Original fixture provenance and hashes are unchanged.

[Integration review](integration_review.md), [development notes](development_notes.md)
and [source integrity](source_integrity.json) record the review boundaries, retained
failed preliminary run and source lineage. Full new workflow artifacts are retained;
regression folders retain reports with complete outputs in the named local run.
Reproduce with `python3 scripts/validate-gpu-shutter.py` and the local browser/server
setup in the contract. Capability updates were made after native/browser acceptance.
