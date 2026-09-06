# Autonomous engine roadmap progress

User direction: continue engine-first work, park concrete major blockers, keep
working on feasible remaining steps. M09 interactive editor remains deferred.
Each completed increment requires implementation, native/browser workflow evidence,
negative/cancellation/stale checks, resource/provenance records and documentation.

Current completed increment: GPU ideal dielectric interfaces; evidence/gpu-dielectric.

| Order | Increment | Status |
|---|---|---|
| 1 | GPU shutter frames and streaming sequences | Validated on feat/gpu-shutter-sequences; evidence/gpu-shutter |
| 2 | Broader glTF attributes, UV/alpha/morph semantics and export | Sparse/UV0, CPU alpha, named UV, morph frames, vertex colors and evaluated PBR GLB export validated |
| 3 | Richer GPU scattering and transport quality | GPU alpha, conductor, coat and ideal dielectric validated; broader lighting/sampling quality remains separate |
| 4 | External hair/volume inputs, fiber shading and sparse GPU media | Pending |
| 5 | Scoped native .blend/USD/MaterialX interchange | Pending |
| 6 | M10 engine UV/paint/sculpt/remesh/retopology operations | Pending |
| 7 | M11 solver families with checkpoint/seek semantics | Pending |
| 8 | M12 compositor/drawing/media/tracking engines | Pending |
| 9 | M13 collaboration/extensions/client compatibility hardening | Pending; expand the documented JSON Schema subset to all existing profile operations |
| 10 | M14 engine production conformance and platform runtime coverage | Pending |

M10/M12 full interactive acceptance and the full major-suite release retain their
M09 dependency. Engine subgates can advance independently; a deferred editor must
not be silently reported as completed. No major implementation blocker parked yet.

GPU shutter acceptance: 105 native tests, 86 Wasm tests, two explicit Metal test
groups, complete CLI/browser workflows and 84 unchanged regression artifacts.
Evidence: `evidence/gpu-shutter/README.md`. No dependency changes or major blockers.

Sparse accessor acceptance: 108 native tests, 89 Wasm tests, 43-request CLI and
Chrome CPU/WebGPU/OPFS workflows. Dense/sparse pixels are identical; 80 prior
render/pass/receipt artifacts are unchanged. Evidence: `evidence/sparse-gltf/README.md`.

Alpha acceptance: 115 native and 96 Wasm tests, 43-request CLI and Chrome/OPFS
workflows, exact native/browser pixels/passes and 116 unchanged prior artifacts.
The reproduced camera clipping/depth bugs are fixed; initial boundary regressions
and browser-client corrections are retained in evidence/alpha-gltf. GPU alpha
remains pending. Named UV acceptance follows in evidence/named-uv.

Named UV acceptance: 121 native and 102 Wasm tests, three Metal comparison cases,
19-request CLI and Chrome CPU/WebGPU/OPFS workflow. Native/browser CPU pixels
are exact; GPU RMSE is below 0.000047. All 108 previous images/passes and 36
receipts are unchanged; 66 existing fixture files are unchanged. Displacement
derived UV identity intentionally changes under uniform-named-uv-v1. Morph-frame
acceptance follows in evidence/morph-frames.

Morph-frame acceptance: 126 native and 107 Wasm tests, 12 explicit Metal
frame/shutter comparisons, 31 CLI requests and dense/sparse Chrome CPU/WebGPU/OPFS
workflows. CPU native/Wasm and dense/sparse pixels are exact; GPU RMSE is below
0.000119. All 117 prior images/passes, 39 receipts and 72 fixture files remain
unchanged. Native snapshot-v13 frame bindings preserve stable point/corner IDs.

Source-conformance correction: 128 native and 109 Wasm tests, both Metal suites,
both complete browser workflows, and zero independent Khronos errors across 13
GLB fixtures. UV aliases/time-bound metadata are corrected with all .bin payloads
unchanged. Historical named-UV/morph source-conformance claims are amended;
render values remain exact and source-scoped identity changes are audited.
Vertex colors and broader source export remain next.

Vertex-color acceptance: 134 native and 115 Wasm tests, ten Metal comparisons,
67 CLI requests and five Chrome CPU/WebGPU/OPFS workflows. All 18 GLB fixtures
have zero independent Khronos errors. CPU native/browser pixels are exact; workflow
GPU RMSE is below 0.000004303. All 135 previous image/pass artifacts, 45 receipts
and 81 original fixture files are unchanged. Snapshot-v14 typed RGBA attributes
retain corner seams through displacement and multiply base color/CPU coverage.
Broader glTF export is next. GPU alpha remains pending.

Evaluated PBR GLB export acceptance: 141 native and 122 Wasm tests, real Metal
roundtrip comparisons, 100 CLI requests and five Chrome export/reimport/OPFS
workflows. All five emitted GLBs pass independent validation and are byte-identical
on native/browser. Native roundtrip render pixels are exact in this corpus. The
reproduced private entity-order dependence is fixed with stable-ID table ordering.
Authored labels, local frames, UV/color/material semantics and encoded assets are
preserved within the bounded profile. Prior images, passes, receipts and fixtures
remain unchanged. GPU alpha and its related CPU occlusion check are next.

GPU alpha acceptance: 145 native and 124 Wasm tests, five dedicated Metal groups,
197-request export, 43-request alpha and 19-request shutter CLI workflows, and
actual Chrome WebGPU/OPFS checks. All 243 previous image/pass files and 81 receipts
remain byte-identical. A separate lazy alpha kernel preserves the old opaque WGSL
exactly. CPU occlusion, MASK equality/subnormal thresholds, RNG endpoints, near
clipping and bounded nominal/temporal/partial-sequence failures have regression
evidence. All 23 source GLBs and 10 exported GLBs have zero independent errors.
Secondary extended-CPU texture footprints are the next concrete correction; no
major implementation blocker is parked. Historical checkpoint notes above retain
their original validation scope.

Secondary footprint acceptance: 148 native and 127 Wasm tests, six dedicated Metal
comparisons, 65 CLI calls and ten Chrome CPU/OPFS cases with four WebGPU cases.
All 351 previous image/pass files, 117 receipts and 119 fixture files are unchanged.
Both generated shaders remain exact. Five CPU surface models satisfy secondary-only
minification invariance; primary camera filtering and alpha continuation remain active.
No dependency changes or major blocker. Richer GPU scattering is next.

GPU conductor/coat acceptance:151 native and129 Wasm tests,37 dedicated Metal
comparisons,138 native material workflow calls and two19-call shutter failure
workflows, all with actual Chrome WebGPU/OPFS coverage. Eight material cases include
named UV/normal/alpha consumers and animated morph sequences. All396 previous
image/pass files,132 receipts and135 fixture files remain unchanged. A third lazy
kernel preserves both earlier shader hashes. GPU dielectric transmission is next;
no major blocker is parked. Evidence: `evidence/gpu-surfaces/README.md`.

GPU ideal dielectric acceptance:153 native and130 Wasm tests,16 analytic interface
cases, mixed-model/four-pipeline Metal checks,375 CLI calls over22 material cases,
and a19-call alpha/shutter failure workflow, all with actual Chrome WebGPU/OPFS.
All522 prior image/pass files,174 receipts and137 fixture files remain unchanged.
Three earlier generated shaders retain their exact hashes. Native dielectric optics
now share Metal/WebGPU coverage; external volume input is next in the engine order.
Evidence: `evidence/gpu-dielectric/README.md`. No major blocker is parked.
