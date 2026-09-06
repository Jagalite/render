# Autonomous engine roadmap progress

User direction: continue engine-first work, park concrete major blockers, keep
working on feasible remaining steps. M09 interactive editor remains deferred.
Each completed increment requires implementation, native/browser workflow evidence,
negative/cancellation/stale checks, resource/provenance records and documentation.

Current completed increment: glTF indexed-attribute and fixture conformance; evidence/indexed-attributes.

| Order | Increment | Status |
|---|---|---|
| 1 | GPU shutter frames and streaming sequences | Validated on feat/gpu-shutter-sequences; evidence/gpu-shutter |
| 2 | Broader glTF attributes, UV/alpha/morph semantics and export | Sparse/UV0, CPU alpha, named UV and morph frames validated; vertex colors and broader export next |
| 3 | Richer GPU scattering and transport quality | Pending |
| 4 | External hair/volume inputs, fiber shading and sparse GPU media | Pending |
| 5 | Scoped native .blend/USD/MaterialX interchange | Pending |
| 6 | M10 engine UV/paint/sculpt/remesh/retopology operations | Pending |
| 7 | M11 solver families with checkpoint/seek semantics | Pending |
| 8 | M12 compositor/drawing/media/tracking engines | Pending |
| 9 | M13 collaboration/extensions/client compatibility hardening | Pending |
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
