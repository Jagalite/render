# Secondary texture footprint acceptance

Final run: `artifacts/secondary-textures/run-20260906T100105Z`, base1460e14.
148 native tests and127 Wasm tests pass, plus six dedicated actual Metal comparisons
and the prior GPU suites. Strict formatting/Clippy, dependency audit, exact browser
package, Linux/Windows compile checks and all prior CLI/browser workflows pass.
Linux/Windows runtime coverage remains a separate gate.

The 65-call CLI workflow covers five surface models with two minification filters,
archive restoration, stale revisions, cancellation and budget rejection. Chrome
runs all ten CPU/OPFS cases and four supported WebGPU cases; native/browser CPU
pixels are exact. Maximum WebGPU RMSE is8.724658510558582e-8. Unsupported GPU models
still return structured errors. Filter changes leave secondary images/passes exact.

All351 previous image/pass files and117 receipts are byte-identical to1460e14.
All119 previous fixture files and Cargo manifests/lockfile remain unchanged.
Fourteen new source/provenance files reproduce byte for byte. Both generated shaders
retain their baseline hashes. Source integrity records freeze Rust/Cargo through
acceptance; tested native/browser packages and generating inputs have hashes.

At8x8,32 samples,depth2, the opaque-nearest CPU process used RSS17,399,808 bytes and
peak footprint3,064,896; Metal used33,882,112 and11,060,864. Full observations are
in resources.json. These are individual runs, not benchmarks or total job limits.

`before.txt` reproduces the extended Principled error (maximum0.13204649), while
the ordinary opaque control passes. `cli-before/` reproduces the same persisted
workflow failure using the previously validated1460e14 binary, SHA256
f81adeeb1b093d729880711c208bf54af948b79118963688d7178922db6f85a2.
`after-2.txt` passes all three new core tests. The first transparent-primary control
used8x8 pixels, a0.9-texel footprint; changing its recipe to4x4 supplies the intended
1.8-texel minification test. The initial Metal test's invalid `.002` literal was
corrected to0.002. Logs retain these development failures; no tolerance or golden
was weakened. The final run includes all corrected tests and packaged workflows.

The [contract](../../docs/secondary_textures.md) explicitly retains the base-level
secondary approximation. Propagated footprints, richer GPU scattering and media
remain separate gates. M09 remains deferred.
