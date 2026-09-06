# VOL3 import acceptance

Passed run `artifacts/volume-import/run-20260906T113040Z`, based on `646ed8b`.
The shared Rust importer accepts scalar density and optional aligned linear RGB
emission with explicit units, bounds and cell-constant optical interpretation.
Native CLI and browser operations publish existing typed volume transactions.
See [profile](../../docs/volume_import.md) and
[integration review](../../planning/engine-progress/volume-import-review.md).

- 156 native and 133 Wasm tests pass. Strict workspace/browser Clippy, formatting,
  Linux and Windows cross-compilation, dependency audit and exact browser packaging pass.
- 103 native CLI calls cover eight density/emission, heterogeneous, sparse/empty and
  coordinate-policy cases. Imports retry, archives recover exact rendered pixels,
  and malformed, stale, cancelled, input/transaction-budget operations preserve roots.
- Actual Chrome imports the same binary inputs through `import_vol`; reports and
  revisions match native. All eight CPU images, depth, normals and object passes
  are identical across platforms and after OPFS recovery. Maximum analytic pixel
  error is 0.00000002597651849178817; underlying Beer/emission tests use 1e-13.
- The 64³ source has 262,144 values and five occupied cells. Its native asset is
  485 serialized bytes; no dense decoded array is retained. Exact 16,384-cell
  admission and over-budget occupancy are tested on native and Wasm.
- All 747 previous images/passes and 249 receipts remain byte-identical. All 151
  existing fixture files and four generated shaders retain their baseline hashes.
  Existing Metal/WebGPU surface, alpha, shutter, sequence and export workflows pass.
- Six original CC0 fixture files reproduce exactly. A separate header/data reader
  verifies analytic values and x-fast ordering. Thirty source GLBs and ten exported
  GLBs retain zero independent validator errors; existing warnings remain visible.
- The reviewed JSON Schema accepts eight real volume DTOs and rejects 18 malformed
  policy/byte cases. Rust remains authoritative for binary and semantic validation.
- No Cargo or computational-runtime dependency changes, snapshot schema changes,
  FFI layout changes or imported executable metadata are introduced.

Observed sparse import RSS is 16,138,240 bytes and peak footprint 3,720,256 bytes.
The sparse 1×1, one-sample CPU render observes 15,826,944 bytes RSS and 2,556,992
bytes footprint. These individual process measurements are not benchmarks or
allocator guarantees. Resource reports include every measured import and CPU case.

Occupied media remains explicitly unsupported on GPU. The interpretation is
cell-constant with zero outside; it does not promise source-tool trilinear/clamped
scene equivalence, spectral conversion, paging, indirect in-scattering or simulation.
Native archives retain sparse values, while source containers and import reports
remain caller-retained. Browser synchronous import does not offer asynchronous
preemption; cancellable core/native paths and failed durable publication are tested.
macOS ARM64 and actual Chrome are runtime-qualified; Linux/Windows are compile-only.

Development logs retain corrected Rust float literals and a Clippy test-initializer
failure. The initial structural-schema development check unnecessarily reused the
large sparse DTO for negative cases; the final script uses the small homogeneous
DTO while retaining the real sparse positive request and every negative condition.
The final full acceptance run passed. No tolerance, diagnostic or golden was weakened.

Reproduce: `python3 scripts/validate-volume-import.py`, then
`python3 scripts/collect-volume-import-evidence.py <passed-run-directory>`.
Use the documented compact Cargo environment, local package server at 8775 and
isolated Chrome CDP at 9224. Raw execution logs remain in the run directory;
committed text copies normalize trailing whitespace only. `SHA256SUMS` covers
committed evidence content; `source_integrity.json` verifies Rust/Cargo stayed frozen
through acceptance and `tested_package.json` identifies exact tested artifacts.
