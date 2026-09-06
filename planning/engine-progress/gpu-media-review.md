# GPU sparse media integration review

Candidate `feat/gpu-sparse-media`, base `35ea08b`. Review status: accepted after native/browser workflows, source/dependency integrity
and prior-artifact identity checks. See `evidence/gpu-media/README.md`.

- Core/GPU boundary: `Media::transport_cells` exposes immutable typed evaluated
  bounds, inverse transforms and RGB coefficients. Published snapshots and native
  cell construction/order are unchanged. No public serialization uses shader rows.
- GPU/kernel boundary: seven vec4 rows per cell, one parameter descriptor and fifth
  lazy kernel. Existing four constructors produce identical generated source.
  Buffer identity hashes packed content, including media; loss destroys all caches.
- Numerical boundary: f64 packing reconstructs all eight corners. Review found
  that corner accuracy alone admitted enormous exact powers of two beyond GPU far.
  Added explicit coordinate/inverse envelopes and rejection of nonzero subnormal
  packed values, with tests. This does not promise grazing-ray equivalence.
- Transport boundary: half-open unit intervals retain unit-world ray parameters.
  Ordered overlapping coefficients feed analytic Beer integration. Small optical
  depth uses a series with an explicit truncation bound. CPU finite-depth emission
  and escape policies remain the reference; unsupported joint profiles reject.
- Resource boundary: checked cell-visit work estimate, 64-cell ceiling and existing
  storage/device byte admission precede dispatch. Cancellation after submission
  drains work and discards its result; progressive cancellation retains sample ID.
- Public workflow boundary: ordinary transactions author media and animation.
  Native archives and browser OPFS recover roots; failed nominal/temporal frames
  must not publish bundles, and partial sequences retain only completed frames.
- Dependency/provenance boundary: no Cargo or FFI change; maintained kernel source
  is Rust. Analytic fixtures are original CC0. Prior fixtures/images/receipts and
  shaders require explicit identity checks before acceptance.

Development corrections are retained in raw logs: an initial receipt-scope compile
error, a leading-zero Rust test literal error and an inside-camera analytic oracle
that omitted the established 1e-5m camera minimum. The oracle now includes that
minimum; CPU implementation and test tolerances were not weakened. The Beer series
threshold increased from 0.01 to 0.1 to reduce cancellation near the former boundary;
both regions have independent analytic cases.

The first external VOL3 GPU run exposed a shared-face hole: adding camera jitter
into each cell's translated unit origin rounded the lower cell to its excluded high
face while the neighbor retained a negative origin. Preserve the tiny camera-relative
origin by applying only the inverse linear map to rays and packing translated lower
and upper bounds separately. Corner qualification now reconstructs the actual f32
bounds and rejects collapsed intervals. Exact rays on either side of a shared face
and the unchanged VOL3 corpus gate this correction. No fixture or tolerance changes.

The first complete native run found a diagnostic-order regression: an existing
large scattering fixture returned the new cell budget instead of its established
unsupported-profile error. Semantic profile rejection now precedes GPU cell/work
admission; the old test and the new 65-cell zero-scattering budget test stay intact.

GPU sparse media acceptance: 167 native and 142 Wasm tests, 376 media CLI calls over 21 cases, 117 external VOL3 CLI calls and 13 shutter/sequence calls, with actual Chrome CPU/WebGPU/OPFS recovery. All 993 prior images/passes and 331 receipts remain byte identical, as do all four prior shaders and Cargo files. Maximum browser GPU analytic channel error is 3.24428673e-07. The named profile admits 64 zero-scattering cells and ordinary opaque PBR surfaces under explicit work/precision limits. Scattering, joint advanced surfaces and larger paging remain separate gates; M09 stays deferred.
