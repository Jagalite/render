# GPU conductor and single-interface coat acceptance

Final run: `artifacts/gpu-surfaces/run-20260906T102437Z`, basee23cf86.
151 native tests and129 Rust Wasm tests pass, alongside three dedicated Metal test
groups (37 comparisons) and all previous GPU suites. Strict Clippy/formatting,
dependency/link audit, Linux/Windows compile checks, exact browser packaging and
all prior CLI/browser workflows pass. Linux/Windows runtime coverage remains open.

Eight persisted native material cases use138 CLI calls: secondary emitters, named-UV
MASK/BLEND and authored morph frames for both models. All eight Chrome CPU/WebGPU/OPFS
cases pass; two animated cases include ordered two-frame sequences. Native/browser
CPU pixels are exact. Maximum WebGPU linear RMSE is0.00034137833780931365, within
the unchanged0.002 profile tolerance. GPU archive restoration is pixel-exact.

Two additional19-call shutter workflows and their Chrome clients verify each new
model preserves nominal failure, temporal failure followed by valid dispatches,
partial sequence counts, cancellation and recovery. The existing secondary-texture
workflow now includes both admitted GPU models and passes69 calls. Dielectric remains
an explicit unsupported GPU profile. Invalid coat parameters, stale writes, budget
failures and cancellation preserve authored roots.

Independent analytic checks cover normal-incidence conductor reflectance1/0,
coat reflectance0.04, zero weight and IOR1. A native/Wasm test proves zero-weight coat
preserves the complete base sampling distribution. Actual Metal comparisons cover
depths1/2/4/16 and opaque→alpha→conductor→coat→alpha→opaque pipeline switching,
plus named UV, normal maps, vertex colors and alpha coverage.

All396 previous image/pass files and132 receipts are byte-identical to e23cf86.
All135 original fixture files and Cargo manifests/lockfile remain unchanged. Source
validation reports zero errors in27 GLBs; ten emitted regression GLBs also have zero
errors. All warnings are retained. Eight native recipes pin original CC0 source
hashes and retain exact ordinary transaction requests; archive retries/recovery prove
reproduction. The14 existing checker source/provenance files reproduce identically.

Both old generated shader hashes remain exact. The new surface shader SHA256 is
cf5b558cef24671d760df416ccd1b82b7b8166bc26dbf2dfd4e2d3d616808f07.
All three artifacts, Rust generating inputs and exact tested native/browser package
hashes are saved. Rust/Cargo sources remained unchanged throughout the final run;
documentation and collection metadata were added afterward. No dependency changes.

At8x8,32 samples,depth2, conductor-secondary CPU used RSS17,432,576 bytes and peak
footprint3,081,280; Metal used35,274,752 and12,502,592. Full case and sequence process
observations are retained. These are individual measurements, not benchmarks or
proofs of total driver residency. The new private records add32 bytes per extended
instance within existing declared GPU allocation budgets.

## Retained development corrections

The first acceptance run stopped at an old CLI assertion that expected coated GPU
rendering to fail. The replacement dielectric fixture initially retained incompatible
nonzero roughness/metallic and was correctly rejected during material validation.
It now sets both to0, passes the focused preflight/deadline test and the final suite,
and keeps unsupported-profile coverage while new conductor/coat cases prove positive
admission. Both failed runs and their timing/logs remain under development-runs.
No numeric threshold or approved reference image was changed.

The [profile](../../docs/gpu_surfaces.md) and
[integration review](../../planning/engine-progress/gpu-surfaces-review.md) record
f32 precision, single-interface and transport restrictions. GPU dielectric,
participating media and external material extensions retain separate gates. M09
remains deferred; no major implementation blocker is parked.
