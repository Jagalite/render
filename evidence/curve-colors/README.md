# Polyline RGBA acceptance

Passed run `artifacts/curve-colors/run-20260906T130429Z`, based on `0a3ffd1`. Optional native control colors survive
polyline subdivision, tube rings/caps and groom child generation. The explicit
HAIR `linear_rgba_f32` mode accepts within-strand RGB/coverage variation while the
original absent-policy profile remains unchanged. See [profile](../../docs/curve_colors.md)
and [integration review](../../planning/engine-progress/curve-color-review.md).

- 164 native and 141 Wasm tests pass. Analytic checks cover
  interpolation, radius subdivision, caps, closed curves, white defaults and groom
  children. Negative/version/permission/stale/cancel and failed publication checks
  preserve immutable roots and native archive recovery.
- Strict workspace/browser Clippy, formatting, Linux/Windows cross-compilation,
  dependency audit and exact packaged browser build pass. Snapshot15 gates colored
  controls; no Cargo, computational dependency, FFI shape or shader change is needed.
- 157 CLI calls cover six HAIR policy/source variants and an authored colored
  guide/child groom, with CPU/Metal rendering, retries, actual conversion resource
  observations, archive/recovery and evaluated GLB export/reimport.
- Actual Chrome imports the same HAIR sources/policies and restores the native groom.
  CPU native/browser pixels are exact; maximum WebGPU linear RMSE is 2.2384056021596437e-08.
  Object passes match and depth/normal bounds pass. GPU cancellation and OPFS recovery
  pass. All seven native/browser GLB byte streams match exactly; independent Khronos
  validation reports zero errors and zero warnings. Native round-trip RMSE is at most
  2.2352651266970292e-08; browser round-trip rendering also passes the unchanged tolerance.
- All 867 prior images/passes, 289 receipts, 164 fixture files and
  four generated shaders retain exact baseline bytes. A separately captured previous
  native binary also matches 20 uncolored curve/point/groom document, storage,
  render and receipt artifacts, including Bezier and rational spline evaluation.
- Six original data/provenance files reproduce exactly. The structural schema accepts
  six real opt-in HAIR DTOs and rejects 23 malformed policies/requests. Existing 30
  source GLBs and 10 prior exported GLBs retain zero independent validation errors.

The largest alpha-subtraction error in this corpus is 2.2351741790771484e-8. It is
reported explicitly; source f32 transparency can require rounding after subtraction
from one. Disposable f64 mesh JSON byte lengths remain platform-local observations,
independently checked against each actual evaluator receipt. Authored identities,
source values, revisions and CPU pixels retain exact cross-platform checks.

Development found that unnecessary explicit extended opaque PBR selected a different
sampling sequence from equivalent imported GLB. The new profile now uses canonical
ordinary opaque PBR for fully covered strands; BLEND remains explicit. The RGB-only
round-trip regression passes without changing the image tolerance. The original
HAIR profile remains byte-identical. A sandboxed resource-measurement attempt failed
before acceptance and was rerun with authorized macOS resource access.

The first full run missed the existing sequence cancellation test's eight-second
first-frame deadline. That test and its deadline were unchanged: it passed in
isolation (3.79 seconds), then in the final complete suite. Both full runs and the
isolated diagnostic are retained. No test, image tolerance or golden was weakened.

Process resource observations (RSS / peak footprint in bytes):

- gradient-le-import: 17448960 / 3245120
- gradient-le-cpu: 18759680 / 3622016
- gradient-le-gpu: 36519936 / 12912192
- groom-cpu: 20070400 / 4424832
- groom-gpu: 37715968 / 13616832

HAIR renders are 16×16 and the groom 32×32, each 16 samples/depth2. These are individual
process observations, not benchmarks or allocator guarantees. CPU/Metal runtime
qualification is macOS ARM64; Chrome covers Rust Wasm/WebGPU/OPFS; Linux and Windows
are compile-only. Browser import remains synchronous and bounded.

Colored Bezier/B-spline controls, point colors, physical fiber scattering, analytic
curve intersections, animated source grooms and GPU media retain separate gates.
Native authoring survives archives/OPFS; evaluated GLB intentionally loses curve
and groom intent. Reproduce using the profile's validation and collection commands.
Committed text logs normalize trailing whitespace only; raw logs remain in the run
directory. SHA256SUMS and final metadata identify evidence, frozen source and tested
native/browser packages.
