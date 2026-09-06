# GPU Principled alpha acceptance

Final run: `artifacts/gpu-alpha/run-20260906T093429Z`, based on commit07720c5.
Validated on macOS ARM64 CPU/Metal, Rust Wasm in Node and Chrome152 WebGPU/OPFS.
Linux/Windows checks compile the native host; they are not runtime qualification.

- 145 native tests and 124 Wasm tests pass, plus five dedicated Metal alpha groups
  and the existing export, color, named-UV and morph Metal suites. Strict native
  and browser Clippy, formatting, dependency/link audit and packaged browser checks pass.
- 197 CLI export calls cover ten source/reimport cases; 43 alpha and 19 shutter
  calls cover import/recovery, cancellation, stale requests and failed publication.
  Chrome validates all ten exports, three analytic alpha inputs and the authored
  65-layer shutter/partial sequence fixture, including OPFS recovery.
- Native/browser export bytes are identical. Cross-platform CPU pixels are exact;
  maximum export-workflow WebGPU RMSE is0.00003083258112877856. Alpha source/reimport
  comparisons keep the published .00002 roundtrip tolerance, without changed goldens.
- All 243 previous image/pass artifacts and 81 receipts are byte-identical.
  All 100 previous fixture files other than the expanded export capability recipe
  are unchanged. Sixteen new fixture/provenance files reproduce byte for byte.
- Independent Khronos validation finds zero errors in23 source GLBs and10 exported
  GLBs. All warnings/informational findings remain in the saved reports.
- No Cargo manifest/lockfile or runtime dependency changes. Both generated WGSL
  artifacts, generating Rust inputs and exact tested packages have hashes.

Named-BLEND source rendering at32x16,8 samples,depth2 measured CPU RSS17,612,800
bytes and peak footprint3,228,800; Metal RSS34,979,840 and footprint12,207,680.
These are individual process observations, not benchmarks or total job limits.
The alpha pipeline is compiled lazily; the old opaque pipeline remains unchanged.

## Reproduced failures and review corrections

`occlusion-before.txt` reproduces missing indirect AO in the extended CPU path.
The corrected tests select the used material, then verify zero/full/partial AO
for four BSDF models, direct/emission independence, MASK/BLEND and recovery.

`random-endpoint-isolated-before.txt` reproduces the largest midpoint rounding to1
in f32. Fully covered surfaces now bypass that random comparison. The preceding
combined probe also exposed the BVH near epsilon; perspective camera near values
below1e-5 now survive both BVH and triangle admission in the alpha variant.

`mask-equality-before.txt` reproduces cutoff/factor rounding an exact multiplication
equality upward. A <=30-step search compiles the original CPU multiplication
predicate, including subnormal factors. Zero coverage rejects a positive cutoff
even if a GPU flushes the packed threshold. Interpolated f32 products and subnormal
GPU arithmetic remain qualified precision differences, not exact f64 emulation.

The first pipeline implementation passed functional tolerances but changed one
old opaque PFM by at most5.960464477539063e-8, changing its receipt digest. Its
`opaque_regression_before.json` is retained under development runs. The final
implementation generates a separate alpha variant and preserves the baseline
opaque WGSL byte for byte; `opaque-recovery.json` and the full identity audit prove
recovery. A unit test pins the baseline shader hash to commit07720c5.

Development corrections also include an unused-material test selection, a dyadic
background for exact accumulation assertions, native rather than glTF65-node
stack construction, increasing sequence times, compile-time builder plumbing,
and checking referenced rather than unused imported images in export roundtrips.
No source node budget, numerical acceptance tolerance or reference image was weakened.
All superseded runs retain their logs, costs and reasons. Final Rust/Cargo hashes
were unchanged throughout acceptance; documentation/evidence metadata was added later.

The [profile](../../docs/gpu_alpha.md) retains separate gates for secondary texture
footprint parity, richer GPU scattering and sparse media. M09 remains deferred.
