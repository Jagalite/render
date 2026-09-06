# HAIR strand input acceptance

Passed run `artifacts/hair-import/run-20260906T121018Z`, based on `69a4882`.
The shared Rust importer preserves bounded HAIR polylines and varying thickness as
native curves, with explicit byte order, units, thickness, color and surface coverage
interpretation. Native/browser publication uses ordinary geometry/material transactions.
See [profile](../../docs/hair_import.md) and
[integration review](../../planning/engine-progress/hair-import-review.md).

- 159 native and 136 Wasm tests pass, including exact geometry against manually
  authored curves, optional/default arrays, both byte orders, metric/radius and color
  interpretation, malformed/unsupported input and aggregate sweep admission limits.
- Strict workspace/browser Clippy, formatting, Linux/Windows cross-compilation,
  dependency audit and the exact browser build pass. No Cargo, snapshot-version,
  FFI-shape or maintained/generated shader change is introduced.
- 94 CLI calls cover six import/CPU/Metal/evaluation/archive/recovery workflows.
  Retry, stale, malformed, cancelled, input/transaction and sweep-budget failures
  preserve authored state. Core tests inject failed durable publication and permissions.
- Actual Chrome imports the same sources and policies. Authored report fields,
  source identities, revisions and geometry counts match native. CPU pixels are exact;
  maximum browser GPU linear RMSE is 0.000000014191850604321853. Object passes match,
  and depth/normal comparisons satisfy existing precision bounds. OPFS recovery is exact.
- The two-strand sources have six or seven controls and generate 40–70 vertices,
  72–132 triangles in this corpus. Both native and browser clients independently check
  preflight observations against actual published-document evaluator receipts.
- All 795 previous images/passes and 265 receipts are byte-identical. All 158 existing
  fixture files and four generated shaders retain baseline hashes. Existing Metal/
  WebGPU surface, alpha, shutter, sequence, export and VOL workflows pass.
- Five original CC0 data/provenance files reproduce exactly; a separate reader verifies
  header/array order and analytic values. The schema accepts six real DTOs and rejects
  22 malformed cases. Existing 30 source and 10 exported GLBs retain zero validator errors.

Observed default import RSS is 17,072,128 bytes and peak footprint 2,933,824 bytes.
The 16×16, 16-sample, depth-2 CPU render observes 18,186,240 / 3,245,120 bytes;
Metal observes 36,143,104 / 12,682,880 bytes. These are individual process observations,
not benchmarks or allocator guarantees. Aggregate negative tests exceed the 65,536
vertex cap with otherwise valid material groups and separately exceed 128 groups.

Disposable f64 geometry serialization is a platform-local resource observation:
`observed_derived_json_bytes` differs by one to three bytes across native/Wasm in this
corpus. Each value equals the actual evaluator's conversion receipts. Authored values,
identities, counts and CPU pixels remain exact. The initial full run exposed an
incorrect assumption that this observation was cross-platform identity; the final
DTO names its meaning and validates it through independent published-document paths.
No image tolerance or golden changed. Both runs and diagnostic evidence are retained.

Other development corrections were a test referring to the wrong Scene field, a
Clippy modulo spelling, an arbitrary brightness smoke check and a stale-test key
shorter than the existing transaction minimum. Final smoke checks require actual
geometry visibility and contrast against the known background; numerical/native
geometry and CPU/GPU comparison gates remain intact. Failed workflow reports retain
costs and diagnostics. The final full acceptance run passes.

The profile uses polygon Principled surfaces and per-intersection coverage, not
physical fiber optics. Within-strand color/transparency variation, analytic curve
intersections, anchored animation sources and paging remain explicit later gates.
Native archives retain authored curves/materials; raw sources stay caller-retained.
Browser import is synchronous and bounded, not asynchronously preemptible. Runtime
qualification is macOS ARM64 and Chrome; Linux/Windows are compile-only.

Reproduce with `python3 scripts/validate-hair-import.py`, followed by
`python3 scripts/collect-hair-import-evidence.py <passed-run-directory>`.
Use the documented compact Cargo environment, local package server at 8776 and
isolated Chrome CDP at 9224. Committed text logs normalize trailing whitespace only;
raw logs remain in the run directory. `SHA256SUMS` covers evidence, source integrity
records verify frozen Rust/Cargo, and package hashes identify the tested artifacts.
