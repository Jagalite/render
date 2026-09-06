# Named UV acceptance

Source checkpoint `68edeb3`; final full run
`artifacts/named-uv/run-20260906T065252Z`. `validation_report.json` records every
command, status, duration and log. Tested Rust/Cargo files remained unchanged
through this run (`source_integrity.json`); final documentation is recorded
separately. No reference renderings were regenerated.

- 121 native and 102 Wasm tests; six focused named-UV analytic/regression tests.
- Explicit Metal test: independent texture roles, selected-set mip footprint and
  named-height displacement, with color/depth/normal/object comparisons.
- Formatting, strict workspace/browser Clippy, Linux and Windows compilation,
  native executable, release browser package and dependency audit pass.
- 19 CLI requests cover GLB and explicit-resource glTF import, retries, archive
  export/restore/exact pixels, invalid missing UV, cancellation, input budgets,
  stale revision and immutable failure state.
- Chrome Rust CPU/WebGPU import, preview, OPFS restore, retry identity, GPU
  cancellation, malformed atomicity, stale render and stale branch rejection pass.
- Native/browser CPU pixels are exact. CPU/Metal RMSE is 0.0000467494021463;
  CPU/WebGPU RMSE is 0.0000467491409961. Both meet 0.002 with exact object IDs.
- 108 earlier animation/shutter/sparse/alpha images and pass artifacts, plus all
  36 receipts, are byte-identical. All 66 previous fixture files are unchanged.
  Four new fixture files reproduce exactly from the original CC0 generator.

`resources.json` records individual macOS observations at 32x16, 8 samples, depth2,
two triangles, four UV sets and three 2x2 images. CPU rendering observed RSS
17,088,512 bytes and peak footprint 3,228,736; Metal observed RSS 33,554,432 and
peak footprint 11,044,416. These are process observations, not performance claims
or allocator guarantees. Eight-set bounds and existing import/displacement/device
budgets constrain the published profile. Debug information and incremental builds
were disabled for the compact validation target; exact environment is in the report.

No Cargo/runtime dependency changes. The dependency inventory, link inspection,
package hashes, fixture provenance and source hashes are retained. Generated WGSL
comes from maintained Rust kernel IR; the package manifest records tested Wasm
and generated interop assets.

See [integration review](integration_review.md) and [development notes](development_notes.md)
for compatibility decisions and failed candidate costs. The [small preview](preview.png)
was visually inspected as a four-region textured quad; numerical role samples,
not appearance, supply the oracle. Linux/Windows are compile-only; GPU alpha,
vertex colors, shader extensions and broader source export remain separate gates.
M09 remains deferred.

Source-conformance amendment: these historical analytical inputs had nonconsecutive
UV-set metadata; the morph inputs also lacked animation-input bounds. Native
numerical/render results above retain their recorded meaning, but source-format
conformance is superseded by [the correction evidence](../indexed-attributes/README.md).
All raw historical inputs/reports remain available in their recorded commits.
