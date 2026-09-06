# glTF source-conformance correction

Full validation: `artifacts/indexed-attributes/run-20260906T072854Z`, based on
`d107a5f`. Tested Rust/Cargo files remained frozen throughout; source and package
hashes tie the recorded results to this correction. Final metadata and the extra
saved-artifact comparison are recorded separately.

Independent Khronos validation exposed nonconsecutive UV-set metadata in the
named-UV and morph fixtures, plus missing animation-input bounds in the morph
fixtures. The original three GLBs reported 6, 7 and 7 errors, respectively. The
first correction removed UV errors; the retained initial audit shows the remaining
bounds errors. Final validation reports zero errors across all 13 GLB fixtures.
Seven warnings are retained, including documented tangent generation and ignored
skinned-node local transforms; informational target hints and unused aliases also
remain visible. No validator diagnostics are filtered or suppressed.

The source fix declares UV3..UV6 as aliases of existing constant UV7 and declares
the unchanged [0,1] input bounds. Every .bin geometry/image/animation payload stays
byte-identical. Twelve fixture source/provenance/generator/README files change;
the exact old/new hashes and allowed paths are in `fixture_delta.json`. Eleven
corrected fixture output files reproduce exactly from their generators. These
are source metadata corrections, not regenerated renderer reference images.

- 128 native and 109 Wasm tests pass, including new indexed-name/continuity and
  input-bound negative tests, plus the existing analytic UV and morph checks.
- Both explicit Metal suites pass: named roles/mips/displacement and twelve
  dense/sparse morph time/shutter comparisons.
- Format, strict workspace/browser Clippy, Linux/Windows compile-only checks,
  native/release browser builds and dependency/link audit pass.
- The 19-request named-UV and 31-request morph CLI workflows pass, with their
  browser CPU/WebGPU, OPFS, retry, stale, malformed, budget and cancellation checks.
- Native/browser CPU color, depth and normals remain exact for named UV; WebGPU
  RMSE is 0.0000467491409961 with max normal error 0.000149488449097 and exact IDs.
  Morph native/browser CPU pixels are exact; WebGPU RMSE is 0.000118356229480.
- 162 prior artifacts remain byte-identical. For the corrected sources, 18 pass
  or receipt files change only in source-scoped IDs and their dependent revision
  or output digests. Their colors, depth, normals and per-pixel entity correspondence
  remain exact. `render_identity.json` records each comparison and identity map.

No Rust runtime dependency changes. The pinned `gltf-validator@2.0.0-dev.3.10`
package is an Apache-2.0 development-only independent oracle, installed with scripts
disabled in a temporary directory. It never implements product import, evaluation
or rendering. Its package metadata, npm integrity lock and tool hashes are retained.
Resource observations for the eight-set fixtures are in `resources.json`; these
are individual process measurements, not performance comparisons.

The full run has no failing check. The follow-up named-UV comparison uses its
already-produced saved artifacts and the same tested package. Raw logs remain in
artifacts; committed log copies trim terminal trailing spaces and redundant EOF
blank lines only. Historical evidence is retained with explicit conformance
amendments. See [the contract](../../docs/gltf_source_conformance.md). The adapter
is still a bounded glTF profile; vertex colors and broader export remain next.
