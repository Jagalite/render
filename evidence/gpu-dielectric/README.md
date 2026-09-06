# GPU ideal dielectric acceptance

Final run: `artifacts/gpu-dielectric/run-20260906T105317Z`, base0330100.
153 native tests and 130 Rust Wasm tests pass, plus two dedicated Metal test groups
and all prior GPU suites. Formatting, strict Clippy, dependency/link audit,
Linux/Windows compile checks, exact browser packaging and all prior CLI/browser
workflows pass. Linux/Windows runtime qualification remains a separate gate.

A 375-call persisted CLI workflow covers 22 cases: six interface/texture/morph cases
and 16 analytic recipes. Chrome CPU/WebGPU/OPFS repeats all 22, including the animated
three-sample shutter and ordered two-frame sequence. CPU native/browser pixels are
exact; maximum WebGPU RMSE is 0.00003262625689217006. Analytic CPU error is below 2e-6
and GPU error below 2e-5, with exact archive/recovered GPU pixels.

Analytic cases cover IOR 1/1.5/3, tint, entry/exit eta² cancellation, Snell's law with
a narrow emitter target, TIR, grazing, tilted shading frames, X/Z mirrored transforms,
near/far clipping and MASK/BLEND. Geometry and sampled optical factors supply the
oracle. Rust core tests run on native/Wasm; Python independently generates the same
expectations without reading rendered images. Metal also renders a mixed scene that
explicitly contains dielectric, conductor, coat, Principled and legacy diffuse,
and verifies exact dielectric image/receipt recovery after using all other pipelines.

The additional 19-call dielectric shutter workflow and Chrome client retain nominal
failure, temporal failure followed by valid dispatches, partial-sequence counts,
cancellation and archive/OPFS recovery. The earlier secondary-filter workflow now
admits all five GPU cases and passes 71 calls. Invalid IOR, stale requests and output
budgets preserve authored roots. Valid sparse media retains unsupported GPU coverage.

All 522 prior image/pass files and 174 receipts are byte-identical to 0330100.
All 137 earlier fixture files and Cargo manifests/lockfile are unchanged. Eleven
new generated source/provenance/analytic files reproduce byte for byte. Thirty source
GLBs and ten regression exports have zero independent Khronos errors; all warnings
and informational findings remain in the saved reports.

All three earlier generated shaders retain their hashes. The new dielectric SHA256 is
afa6f881610497e4c8cdf63f6a28c10ff2a243a23c4f23dcac7aae5179130eea.
Four generated artifacts, their maintained Rust inputs and exact tested native/browser
packages have hashes. Rust/Cargo remained frozen during final acceptance; documentation,
capability and collection metadata were added afterward. No dependency changes.

The single-interface at 8x8,32-sample,depth4 CPU process used RSS 17,055,744 bytes and peak
footprint 3,064,960; Metal used 35,012,608 and 12,387,904. Full process observations are
retained; these are not benchmarks or guarantees of total driver residency. Dielectric
uses the existing 32-byte private surface-record shape and declared allocation budgets.

## Development evidence

Logs retain fixture-helper field/import compile errors, then selecting the transform-only
glTF root instead of its material-bearing primitive. The corrected recipe resolves
that primitive through its stable parent. Receipt comparison uses canonical serialization
because the public receipt type does not implement PartialEq. An initial mixed fixture
only supplied two PBR models; the final fixture explicitly asserts all intended models
and keeps a legacy diffuse instance. These corrections changed no numerical tolerance
or approved reference image. The focused sparse-media CLI preflight/deadline test passes.

The [profile](../../docs/gpu_dielectric.md) and
[integration review](../../planning/engine-progress/gpu-dielectric-review.md) record
critical-angle/f32, air/material interface, finite-depth and direct-light limitations.
Nested media, rough transmission, external material extensions and GPU participating
media remain separate gates. M09 stays deferred; no major blocker is parked.
