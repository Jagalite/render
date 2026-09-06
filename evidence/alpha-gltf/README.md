# glTF alpha CPU acceptance

Implementation checkpoint `35f92f2` extends the PBR importer with MASK/BLEND and
corrects transparent camera continuation. The [contract](../../docs/gltf_alpha.md)
retains explicit GPU rejection for alpha. OPAQUE keeps its existing CPU/GPU profile.
This completes the named alpha input increment, not all INPUT-02 or M09.

[Validation](validation_report.json) records 115 native and 96 Wasm tests, seven
focused alpha regressions, strict native/browser Clippy, formatting, Linux/Windows
compilation, native/release browser builds and dependency auditing. Linux/Windows
remain compile-only. The full run is `artifacts/alpha-gltf/run-20260906T054824Z`.

The [43-request CLI workflow](workflow/workflow_report.json) imports MASK/BLEND/
OPAQUE GLBs, retries requests, renders, exports/restores documents, compares restored
pixels, rejects stale mutations and malformed explicit resources, and checks input
budgets/cancellation with unchanged documents. GPU alpha requests produce failed
manifests with zero bundles/bytes; OPAQUE renders on Metal.

The [browser workflow](workflow/browser_report.json) covers the same imports through
Rust Wasm, CPU previews, OPFS save/load, exact restored passes, retries, malformed
inputs, nonempty-import admission, stale renders and stale branch mutations. GPU
alpha rejects explicitly; OPAQUE WebGPU succeeds. [Native/browser comparisons](cross_platform.json)
are exact for linear pixels, depths, normals and IDs in all three variants.

Independent numerical tests constrain coverage rather than treating renderer
agreement as an oracle. They test linear texture alpha, factor/default/cutoff
boundaries including >1, two layers, backfaces, invalid values, atomic transactions,
cancellation, camera clipping and depth. [Original fixtures](../../fixtures/alpha-gltf/README.md)
reproduce byte for byte. Rendered PNGs are observations, not golden replacements.

The [pre-change camera workflow](camera-baseline/baseline_report.json) verified its
scattering source against `5416cfa`: a clipped rear emitter remained visible in all
64 pixels and orthographic depth had maximum error 0.25171494. The [corrected CLI
workflow](camera-regression/camera_report.json) gives zero visible pixels beyond
the far plane and exact constant depth 3. Tests also cover exhausted intervals with
media and initial perspective near distances below the continuation epsilon.

[116 previous artifacts](regression_identity.json) from animated, shutter and sparse
workflows remain byte-identical. All 54 previously tracked fixture files remain
unchanged. [Resource observations](resources.json) for the 32x16, 64-sample alpha
scene show CPU render RSS 17,907,712 bytes and peak footprint 3,163,200 bytes. These
are individual process observations, not performance comparisons or allocator limits.

[Dependency changes](dependency_delta.json) are empty. The full transitive inventory,
tested package hashes, fixture integrity, and Rust source verification are included.
Raw logs remain under artifacts; committed text copies trim redundant EOF blank
lines only. [Development notes](development_notes.md) retain the interrupted initial
candidate and the corrected browser-client assertion.
