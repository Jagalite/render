# Morph-frame acceptance

Source checkpoint `246288c`; full run
`artifacts/morph-frames/run-20260906T071435Z`. The validation report records every
command, duration, status and log. Tested Rust/Cargo files stayed unchanged
throughout the run; source and package hashes are retained. No golden renderings
or earlier fixture inputs were changed.

- 126 native tests and 107 Wasm tests; five new analytic frame tests.
- Twelve explicit Metal comparisons: dense/sparse input, times 0, 1/2 and 1,
  instant sampling and three-sample shutters. Exact object IDs, strict depth and
  direction checks, and the existing 0.002 radiance RMSE gate pass.
- Format, strict workspace/browser Clippy, native/release browser builds, Linux
  and Windows compilation, and dependency/link audit pass.
- 31 CLI calls cover GLB/explicit glTF imports, retries, CPU/Metal shutter frames,
  archive recovery with exact pixels, stale imports, missing base attributes,
  input budgets, cancellation and immutable state after failures.
- Dense/sparse Chrome Rust CPU/WebGPU previews, OPFS recovery, retry identity,
  GPU cancellation, stale render/branch and malformed atomicity pass.
- CPU native/Wasm pixels are exact. Dense/sparse pixels are exact on each backend.
  CPU/Metal RMSE is 0.000118356824386; CPU/WebGPU is 0.000118356229480, with exact
  object IDs. The GPU binary16 normal-map oracle retains a 2e-5 direction limit.
- All 117 prior images/pass files, 39 receipts and 72 earlier fixture files are
  byte-identical. Seven new fixture files reproduce exactly from their CC0 generator.

At 32x32, 8 spatial samples, depth2 and three temporal samples, dense CPU rendering
observed RSS 18,923,520 bytes and peak footprint 4,064,384; Metal observed RSS
35,045,376 and footprint 11,437,632. The serialized inspected document was 13,374
bytes. `resources.json` records dense/sparse observations. These are individual
process measurements, not performance comparisons or allocator guarantees. The
compact target disables incremental builds and dev/test debug info; exact flags
are in the validation report.

No runtime dependency changes. The existing Rust image/half texture path remains
separate from the new Rust deformation math. Generated shader sources do not
change. Linux/Windows remain compile-only. See [integration review](integration_review.md)
and [development notes](development_notes.md), including the measured initial
normal-pass oracle mismatch and correction. The [small preview](preview.png) was
visually inspected; closed-form frame math supplies the numerical oracle.

Morph UV/colors, broad animated source export and alternate skinning modes remain
separate capabilities. M09 remains deferred.
