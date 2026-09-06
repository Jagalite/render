# Evaluated PBR GLB export acceptance

Final full run: `artifacts/gltf-export/run-20260906T082746Z`, based on `6c3a969`.
Rust/Cargo files remained frozen through this run. Source/package hashes identify
the exact native and browser builds. Final docs/schema metadata is recorded
separately. See [the contract](../../docs/gltf_export.md).

- 141 native and 122 Wasm tests pass, including seven new analytic/negative groups.
  They cover UV/color/material/alpha semantics, posed morph frames, displacement,
  nonuniform/reflected transforms with local smooth frames, shared geometry,
  position quantization, collapsed output, shader/precision/label/budget rejection,
  cancellation boundaries, stale requests, archive recovery and immutable reads.
- Metal compares UV, color and posed-morph scenes before/after export and against
  the CPU renderer. Source and roundtrip GPU pixels are exact in all three cases;
  maximum CPU/GPU RMSE in that test is 0.000234105 under its stated settings.
  Existing named-UV, morph and vertex-color Metal suites also pass.
- All 100 CLI requests pass. Every successful output manifest has verified byte
  counts and hashes. Stale/cancelled work publishes no output directory; failed
  core/vertex/output-budget and consent checks publish no completed scene bundle.
  Native archive recovery reproduces rendered pixels exactly.
- Five Chrome export/reimport/OPFS workflows pass. Native/browser GLBs are
  byte-identical for UV, color, exact half-second morph, MASK and BLEND. Render
  comparisons map source-scoped IDs through file-node and importer reports.
  Browser CPU pixels match native exactly; maximum workflow CPU/WebGPU RMSE is
  0.0000308326. Alpha GPU rejection remains explicit at this checkpoint.
- All five emitted GLBs have zero independent Khronos errors. The UV case retains
  the documented missing-tangent-generation warning; other cases have no warnings.
  Unused-UV informational diagnostics remain visible. All 18 original corpus GLBs
  also pass independent source validation; seven existing warnings are retained.
- All 180 prior image/pass artifacts, 60 receipts and 99 original fixture files
  remain byte-identical. Cargo dependencies/features remain unchanged. Strict
  workspace/browser Clippy, formatting, native/release browser builds and
  dependency/link audit pass. Linux and Windows are compile-only coverage.

The emitted files are 3,732 / 2,732 / 3,996 / 2,052 / 2,036 bytes for UV, color,
morph, MASK and BLEND respectively. Encoded image assets are unchanged after
reimport. The color export observed 17,629,184 bytes maximum RSS and 2,819,072 bytes
peak footprint; the morph export observed 19,283,968 and 3,654,784 bytes. The color
roundtrip CPU/Metal processes observed 17,629,184 / 33,865,728 bytes RSS. Full
observations are in `resources.json`; they are not benchmark comparisons or limits.

The initial full run passed native/Wasm, GPU and CLI checks but failed exact
browser GLB identity on BLEND. Retained browser debug files show equivalent scene
content with different node/material order and binary placement: an in-memory
import used private insertion order, while archive recovery reconstructed entity
chunks in stable-ID order. The before-fix regression in `ordering-before.txt`
reproduces different hashes for equal authored revisions. Export now traverses
stable IDs, independently of private layout. The regression and final native/Wasm
and browser workflows pass without relaxing byte identity or image tolerances.
Authored node labels are also retained with an explicit 1024-byte bound.

Other development corrections were compile-time Time visibility/type inference,
a test helper's Session name and ambiguous Request import, and test expectations
for importer primitive-child IDs and the existing unsupported-profile diagnostic.
The collector initially omitted its workflow path prefix; that saved-artifact
lookup was corrected without changing runtime files or rerunning successful tests.
No reference image or original source fixture was regenerated. Raw artifacts remain
in the run directories; committed text logs only trim terminal trailing whitespace.

A saved-artifact follow-up validates 20 actual request shapes and nine malformed
DTO mutations against the independent JSON Schema validator. The schema explicitly
states its foundation/export subset; remaining profile-method schema coverage is
tracked under M13. This follow-up changes no tested Rust source. Khronos validation
and Python JSON Schema checks are development tools, never product runtimes.

The profile exports evaluated surfaces and embedded PBR resources with measured
f32 position error and explicit authoring/render-recipe losses. Authored animation,
full hierarchy/camera/light export, shader extensions and GPU alpha retain separate
gates. Native archives remain the lossless authoring path.
