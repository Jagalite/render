# Linear vertex-color acceptance

Full run: `artifacts/vertex-colors/run-20260906T075516Z`, based on `99854ea`.
Rust/Cargo files stayed frozen through validation. Source integrity and tested
native/browser package hashes tie the results to this implementation; final
metadata is recorded separately. See [the contract](../../docs/vertex_colors.md).

- 134 native and 115 Wasm tests pass, including six new analytic/negative groups.
  RGB interpolation, shared materials, alpha factor/texture multiplication,
  sparse/normalized equivalence, clamping, point/corner semantics, displacement
  seams, modeling, albedo/emission bakes, versioning and atomic failures are covered.
- Ten explicit Metal comparisons pass across five encodings, with and without
  displacement. Maximum CPU/GPU RMSE is 0.0000195917 under those test settings;
  object IDs match and depth/normals remain within 0.00002. Cancellation, resource
  rejection and unsupported legacy/raster paths are checked on the real GPU.
- The 67-request CLI workflow passes import/retry, CPU/Metal rendering, native
  archive/reopen, stale requests, malformed input, cancellation and budget checks.
  All five Chrome CPU/WebGPU workflows pass OPFS recovery, cancellation, stale
  reads/branches, immutable roots and malformed-input rejection.
- Browser CPU pixels match native exactly. At the workflow's 32x16, eight-sample,
  depth-two settings, Metal RMSE is 0.000004302132 and WebGPU RMSE is
  0.000004302431; object IDs match. Saved depth/normal checks use a 0.00002 limit.
- Independent Khronos validation reports zero errors across all 18 GLB fixtures.
  The seven existing warnings remain visible. The five new source encodings and
  their provenance reproduce byte-for-byte; all 81 previous fixture files are
  unchanged. No renderer reference images are regenerated.
- All 135 previous image/pass artifacts and 45 receipts remain byte-identical.
  Named-UV, morph-frame, alpha, sparse, animated and shutter workflows pass.
- Formatting, strict workspace/browser Clippy, native/release browser builds and
  dependency/link audit pass. Linux and Windows are compile-only checks.

For the RGBA workflow, CPU rendering observed 17,022,976 bytes maximum RSS and
3,114,048 bytes peak footprint; Metal observed 33,308,672 and 10,929,728 bytes.
`resources.json` records every measured variant. These are individual process
observations, not benchmark comparisons or memory guarantees. Cargo dependency
files and runtime feature selections are unchanged. The pinned official Khronos
validator remains a development-only oracle, outside the product runtime.

Development corrections before the full run: the new kernel initially called the
IR vec4 constructor with four scalar arguments instead of vector-plus-scalar; the
first test helper assumed mesh tables were ID-keyed rather than content-addressed;
a negative test expected unsupported_profile instead of the adapter's existing
unsupported_gltf code; and a stale test's short idempotency key triggered request
validation before revision checking. Those test/compile errors were corrected;
no numerical threshold or runtime diagnostic was weakened. An initial independent
validator invocation also needed its parent output directory created. The full
acceptance run has no failing check. Raw artifacts remain in the run directory;
committed text logs trim terminal trailing spaces and redundant EOF blanks only.

Vertex colors are complete for this bounded profile. GPU alpha, additional color
sets, color morphs and broader scene/material export retain separate gates.
