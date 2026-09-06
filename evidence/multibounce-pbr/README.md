# Multi-bounce opaque transport acceptance

Status: passed on candidate branch `feat/multibounce-pbr`, based on `8ddaa7b`.
The [contract](../../docs/multibounce_pbr.md) extends opaque PBR/diffuse rendering
to depth 1–16 through the existing CLI and browser operations. The native workflow
passed before capability status was updated. M09 remains deferred.

## Validation

The fresh run was `artifacts/multibounce-pbr/run-20260906T031706Z`.
[Validation report](validation_report.json) and [logs](logs/) record every command:

- 104 native tests and 85 Rust Wasm tests passed.
- The separate real Metal acceptance test passed (mandatory in this runner).
- Format, strict workspace/browser Clippy, Linux/Windows compilation, native/web
  builds and the dependency audit passed. Linux/Windows are compile-only.
- The existing general CLI passed 32 requests, now including a shared depth-two
  CPU/Metal render. Static PBR and 29-request animated glTF workflows passed.
- The new [41-request native workflow](workflow/workflow_report.json) exercised
  ordinary authoring, depths 1/2/4/16 on CPU/Metal, artifact hashes, native export/
  restore, invalid-depth transaction rejection, stale revisions, cancellation,
  output budgets and immutable authored state.
- [Browser checks](workflow/browser_report.json) exercised the same four depths
  through Rust Wasm/WebGPU, OPFS round trips, stale revisions and GPU cancellation.
- [Depth-one regression](depth_one_regression.json) checks unchanged CPU output
  bytes against six existing static/animated reference artifacts. No historical
  reference image or evidence file was regenerated.

Independent analytic tests constrain the result beyond backend agreement: the
rough white-metal normal-incidence hemispherical integral is `1-ln(2)`, and a
Lambertian cavity follows its finite geometric series. Mixed material paths,
primary pass identity, single environment counting and deterministic sampling are
covered. The explicit Metal test also checks a textured secondary emitter,
progressive sample offsets and reset/receipt semantics, and cancellation at the
actual queue-submission boundary.

## Results and resource evidence

All values below describe the original 16×16, 64-sample plate fixture only.
[Native/browser comparisons](cross_platform.json) use top-down pass arrays and
correctly reverse bottom-up PFM rows. Acceptance is RGB RMSE <0.002 and exact
object IDs. CPU/Metal RMSE reached 3.1232e-8; CPU/WebGPU reached 3.3442e-8.
Native CPU and browser Rust Wasm matched exactly at every depth. Object mismatches
were zero. Mean linear RGB was 0 at depth 1, 0.214945 at depth 2, 0.249116 at depth 4
and 0.256074 at depth 16: indirect emission and subsequent reflection are measurable.

`/usr/bin/time -l` output is retained under [responses](workflow/responses/).
These are single CLI process observations, including process startup and artifact
I/O on a machine running other work; they are not isolated transport benchmarks or
speedup claims. Work is bounded by the existing sample limits and depth ≤16.

| Backend/depth | Wall seconds | Maximum RSS bytes | Peak footprint bytes |
|---|---:|---:|---:|
| CPU / 1 | 0.447 | 17,448,960 | 3,228,800 |
| CPU / 4 | 3.745 | 16,760,832 | 3,441,792 |
| CPU / 16 | 2.900 | 17,760,256 | 3,523,712 |
| Metal / 1 | 0.788 | 34,357,248 | 10,995,328 |
| Metal / 4 | 5.163 | 33,030,144 | 10,929,792 |
| Metal / 16 | 5.082 | 33,816,576 | 11,241,088 |

The [small depth-four preview](preview-depth4.png) is a format conversion of the
native PPM output, not a golden reference. It is only a visual check of the fixture.

## Integrity, provenance and review

[Integration review](integration_review.md) records CPU/kernel/packer/agent and
transaction boundaries. [Development notes](development_notes.md) record initial
compile failures and measurement limits. [Fixture provenance](../../fixtures/multibounce-pbr/provenance.json)
and [reproduction check](fixture_integrity.json) cover the original CC0 fixture.
The texture test uses the unchanged, separately licensed Khronos Cesium fixture.

[Dependency delta](dependency_delta.json) verifies unchanged Cargo manifests and
lockfile; the full [dependency audit](dependency_inventory.json) records features
and platform exceptions. [Generated WGSL](generated-path.wgsl) is an artifact of
maintained Rust IR, with [generation inputs](generated_kernel.json). No maintained
shader source or computational dependency was added.

The initial [source manifest](source_manifest.json) identifies inputs captured
before the suite. The final reviewed manifest and integrity report record subsequent
documentation/schema/capability updates and verify that tested runtime and test
sources did not change. Regression folders retain reports and inventories; their
full generated outputs remain in the named local artifact run and are reproducible
with `python3 scripts/validate-multibounce.py`. The complete new workflow, requests,
responses, document journals, exported documents and renders are retained here.

Limits remain explicit: finite-depth bias, roughness ≥0.05, secondary base-level
filtering, single-scattering GGX and BSDF-only continuous-light sampling. Advanced
GPU scattering/media, alpha material import and GPU shutter accumulation are not
part of this extension. Browser GPU cancellation does not preempt an in-flight
dispatch. No production-scale capacity or general rendering-speed claim is made.
