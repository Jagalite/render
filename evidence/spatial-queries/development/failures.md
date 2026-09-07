# Retained development failures

- native1.log: new assertion could not infer Vec<u64> -> [u64;3]. Added the explicit array target; library compilation succeeded.
- native2.log: five tests passed; the corruption test called Snapshot::revision on an intentionally invalid mesh hash. Revision validation correctly rejected it before the query. The test now constructs the invalid revision digest without validation and confirms public-query rejection plus prior-cache preservation. native3.log: all six passed.
- run-20260906T233117Z: format, workspace/browser Clippy, native build and six native spatial tests passed. The initial CLI source transaction used the 14-character key spatial-source, below the existing 16-character minimum. Changed only fixture keys to spatial-workflow-*. Standalone workflow2 passed all 32 calls; no transaction validation was weakened.

Qualification runs and logs record actual costs and tested package/source hashes.
No image references or prior fixture outputs were regenerated.

- run-20260906T233357Z: new CLI/browser/schema/cross-platform workflow, prior GPU
  updates, numeric texture sources, painting and all Wasm tests passed. Deliberately
  interrupted the subsequent native Cargo test child with SIGINT before source
  edits to add aggregate triangulation admission found during review. Verified no
  Cargo/rustc/test-runner processes remained. native_tests exit -2 records that
  interruption, not a failing test. Initial attempt to signal the earlier Wasm
  Cargo PID found it had already exited; command verification prevented signaling
  another process. The updated package requires a fresh complete acceptance run.

- run-20260906T234457Z: updated spatial native/CLI/schema/browser/cross gates and
  prior GPU/painting checks passed. A separate CLI probe on a valid 1.4e-81 meter
  triangle demonstrated a geometric normal length of 0.8817871037304664 and an edge
  result for a plane-interior query. Retained original requests, responses, native
  package hash and source-manifest entries in tiny-probe. Stopped the run's verified
  Wasm Cargo child with SIGINT and verified compiler shutdown before changing Rust.
  Scaled triangle/edge frames avoid squared-area underflow; a new native/Wasm test
  covers scales 1.4e-81 through 1e8 and a translated tilted plane. No existing
  renderer or mesh-triangulation implementation was changed.

- The scaled-frame fix passed eight native and eight Wasm tests. A related retained
  zero-sphere-before CLI probe showed that center [0,0,1e-200], radius zero wrongly
  included the origin with distance zero. Added a scaled Euclidean comparison for
  squared distances below f64::MIN_POSITIVE, leaving normal squared arithmetic
  unchanged. A ninth native/Wasm test covers inclusive radii and actual distances
  at 1e-160, 1e-200 and 1e-300, zero-radius misses and signed zero.

- run-20260907T000058Z: nine spatial tests, 39 CLI calls, exact browser/cross results,
  GPU/painting checks and the full Wasm suite passed. A retained conditioned-frame
  CLI probe found a last triangle in a valid quad whose original cross component
  is -0.0625 but whose normalized cross cancels to zero. The response contained
  null normals. Stopped only the verified native acceptance Cargo child with SIGINT
  before editing; unrelated Cargo work in another project was observed and left
  untouched. Added normalized-frame admission and a finite-result guard, plus a
  native/Wasm test requiring structured rejection and preservation of the old cache.
  The final fixture also exercises this rejection through native/browser JSON.
