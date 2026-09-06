# Sparse glTF accessor acceptance

Passed on `feat/gltf-sparse-accessors`, based on `5416cfa`. The
[contract](../../docs/gltf_accessors.md) adds bounded sparse arrays and normalized
unsigned UV0 to the shared importer. Public requests, snapshots, reports, renderer
algorithms and dependencies retain their existing semantics. M09 remains deferred.

[Validation](validation_report.json) records 108 native tests, 89 Wasm tests,
strict native/browser Clippy, formatting, Linux/Windows compilation, native/release
browser builds and dependency auditing. Linux/Windows remain compile-only.
The full run is `artifacts/sparse-gltf/run-20260906T045410Z`; completed new workflows
are under `workflow-retry2`. Earlier failed client attempts are retained and
explicitly resolved in the validation report.

The [43-request native client](workflow/workflow_report.json) imports three source
encodings as GLB, checks retries, CPU/Metal shutter renders, document export/restore,
restored GPU pixels and stale mutations. It additionally checks malformed explicit
resources, cancellation, input budgets, atomic failures and a valid explicit glTF
bundle. The [browser client](workflow/browser_report.json) imports the same GLBs,
checks CPU/WebGPU parity, retries, OPFS recovery, restored renders, cancellation,
immutable root state and malformed-input rejection.

Dense, zero-base sparse with u16 UVs, and interleaved-base sparse with u8 UVs yield
identical geometry/image content and exact pixels on each backend. Independent
closed-form skin/morph/absolute-TRS positions constrain the result beyond backend
agreement. Sparse indices use all three unsigned widths. Negative tests cover
counts, order, duplicates, ranges, extensions, stride/target, normalization, zero
bases and component/four-byte vertex alignment without committed mutation.

For the original 64×64 textured ribbon with eight spatial and three temporal
samples, CPU/Metal RMSE is 2.30706e-5 and CPU/WebGPU RMSE is 2.30686e-5 for every
encoding. Native CPU and browser Wasm match exactly. The threshold is <0.002;
object IDs match exactly within each encoding. Different source encodings retain
different scoped IDs, so cross-encoding checks compare visibility and pixel data.
[Cross-platform results](cross_platform.json) reverse PFM rows correctly.

The original 29-request animated and 15-request GPU shutter workflows pass.
[80 previous images, passes and receipts](regression_identity.json) retain exact
bytes. The [preview](preview.png), inspected as a tilted textured ribbon with the
four color regions, is a PPM-to-PNG conversion, not a golden render.

## Resource and source evidence

[Accessor accounting](accessor_resources.json) records the fixture layouts.
Sparse-zero materializes 512 bytes across its accessors; sparse-base materializes
496 bytes; the largest individual accessor is 128 bytes. Dense binary input uses
610 bytes, sparse-zero 614, and interleaved/sparse-base 822. These small test inputs
exercise semantics and do not demonstrate a compression advantage. The production
admission is 196,608 elements and 16 MiB per expanded accessor (current roles top
out at 12 MiB), alongside existing scene and transaction limits.

[Single-process measurements](resources.json) include startup, decoding, driver and
artifact I/O on a machine doing other work; no throughput or speedup claim follows:

| Operation | Wall seconds | Maximum RSS bytes | Peak footprint bytes |
|---|---:|---:|---:|
| Sparse-zero import | 0.116 | 17,989,632 | 3,802,240 |
| Sparse-zero CPU shutter frame | 0.414 | 21,102,592 | 5,227,648 |
| Sparse-zero Metal shutter frame | 0.351 | 37,027,840 | 11,994,752 |
| Sparse-base import | 0.117 | 18,006,016 | 3,818,624 |

[Fixture reproduction](fixture_integrity.json) checks every new source byte against
a fresh generator output. [Original provenance](../../fixtures/sparse-gltf/data/provenance.json)
records CC0 authorship and generating inputs. Existing source fixtures remain
unchanged. [Dependency delta](dependency_delta.json), [inventory](dependency_inventory.json),
[package hashes](tested_package.json), [source integrity](source_integrity.json) and
[integration review](integration_review.md) identify the tested implementation.

[Development notes](development_notes.md) explain the failed client key validation
and targeted follow-up runs. The initial source manifest predates those client-only
corrections and final documentation/capability updates; all Rust/test/fixture sources
were verified unchanged. Test-log copies remove redundant EOF blank lines; raw logs
remain in the named artifact run. Reproduce with `python3 scripts/validate-sparse-gltf.py`.
