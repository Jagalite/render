# Autonomous engine roadmap progress

User direction: continue engine-first work, park concrete major blockers, keep
working on feasible remaining steps. M09 interactive editor remains deferred.
Each completed increment requires implementation, native/browser workflow evidence,
negative/cancellation/stale checks, resource/provenance records and documentation.

Current completed increment: bounded UV unwrap/packing and named constraint assets; evidence/uv-authoring.
Next engine subgate: tiled texture painting, layers, masks and explicit image baking.

| Order | Increment | Status |
|---|---|---|
| 1 | GPU shutter frames and streaming sequences | Validated on feat/gpu-shutter-sequences; evidence/gpu-shutter |
| 2 | Broader glTF attributes, UV/alpha/morph semantics and export | Sparse/UV0, CPU alpha, named UV, morph frames, vertex colors and evaluated PBR GLB export validated |
| 3 | Richer GPU scattering and transport quality | GPU alpha, conductor, coat and ideal dielectric validated; broader lighting/sampling quality remains separate |
| 4 | External hair/volume inputs, fiber shading and sparse GPU media | VOL3, HAIR surface inputs, polyline control RGBA and bounded GPU media validated; physical fiber remains a separate gate |
| 5 | Scoped native .blend/USD/MaterialX interchange | Blender 2.93 static profile validated; additional USD/MaterialX profiles remain future format work |
| 6 | M10 engine UV/paint/sculpt/remesh/retopology operations | UV authoring validated; painting, sculpting, remesh and retopology pending |
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

VOL3 input acceptance: 156 native and 133 Wasm tests, 103 CLI calls over eight
analytic/source-policy cases and actual Chrome CPU/OPFS import workflows. Native
and browser reports, revisions, pixels and diagnostic passes match exactly. The
64³ sparse source retains five cells in a 485-byte native asset. All 747 prior
images/passes, 249 receipts, 151 fixture files and four generated shaders remain
unchanged. No dependency changes or major blocker. External strand input is next.
Evidence: `evidence/volume-import/README.md`. GPU media remains a separate gate.

HAIR input acceptance: 159 native and 136 Wasm tests, 94 CLI calls over six
byte-order/array/color/unit cases and actual Chrome CPU/WebGPU/OPFS workflows.
Source identities, revisions, geometry counts and CPU pixels match exactly;
platform-local disposable JSON byte observations match actual evaluator receipts.
All 795 prior images/passes, 265 receipts, 158 fixtures and four shaders remain
unchanged. No dependency changes or major blocker. Optional per-control RGBA is
the next bounded input gap; physical fiber and GPU media remain later gates.
Evidence: `evidence/hair-import/README.md`.

Polyline RGBA acceptance: 164 native and 141 Wasm tests, 157 CLI calls and seven
actual Chrome CPU/WebGPU/OPFS workflows. Native/browser CPU pixels and exported GLB
bytes match exactly; all seven GLBs pass independent validation. All 867 prior
images/passes, 289 receipts, 164 fixtures and four shaders remain unchanged.
Snapshot15 gates typed colored controls; old curve/groom documents and renders also
match the previous binary. No dependency change or major blocker. Sparse GPU media
is next; physical fibers and higher-order color approximation remain separate gates.
Evidence: `evidence/curve-colors/README.md`.

GPU sparse media acceptance: 167 native and 142 Wasm tests, 376 media CLI calls over 21 cases, 117 external VOL3 CLI calls and 13 shutter/sequence calls, with actual Chrome CPU/WebGPU/OPFS recovery. All 993 prior images/passes and 331 receipts remain byte identical, as do all four prior shaders and Cargo files. Maximum browser GPU analytic channel error is 3.24428673e-07. The named profile admits 64 zero-scattering cells and ordinary opaque PBR surfaces under explicit work/precision limits. Scattering, joint advanced surfaces and larger paging remain separate gates; M09 stays deferred.
Evidence: `evidence/gpu-media/README.md`. No major blocker is parked. Scoped native interchange is next.

Static Blender acceptance: 179 native and 154 Wasm tests; 137 CLI calls across seven cases; exact native/Wasm import revisions and CPU pixels; actual Metal/WebGPU and OPFS recovery. 1308 prior images/passes and 436 receipts remain byte identical, with all five shaders and existing fixtures unchanged.
Evidence: `evidence/blend-static/README.md`. No major blocker parked.

UV authoring acceptance: 197 native tests,
172 Wasm tests, 78 acceptance checks and
28 CLI requests. Named unwrap/packing, multiple sets, source
immutability, old-client rejection, archive/OPFS recovery and CPU/Metal/WebGPU output
passed. 1864 earlier render/pass/receipt files remain
byte identical. No Cargo or shader changes. M10 and the M09 interactive dependency
remain open; see `evidence/uv-authoring/README.md`.
