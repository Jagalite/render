# Autonomous engine roadmap progress

User direction: continue engine-first work, park concrete major blockers, keep
working on feasible remaining steps. M09 interactive editor remains deferred.
Each completed increment requires implementation, native/browser workflow evidence,
negative/cancellation/stale checks, resource/provenance records and documentation.

Current checkpoint: `2fc718f` (validated sparse glTF accessors and normalized UV0).

| Order | Increment | Status |
|---|---|---|
| 1 | GPU shutter frames and streaming sequences | Validated on feat/gpu-shutter-sequences; evidence/gpu-shutter |
| 2 | Broader glTF attributes, UV/alpha/morph semantics and export | Sparse accessors/normalized UV0 validated; alpha import in progress |
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

Next alpha work has two reproduced pre-existing CPU camera bugs to fix: far
clipping after transparency, and orthographic ray-distance depth. Baseline evidence
is `artifacts/alpha-baseline/run-20260906T050222Z`; these do not affect the opaque
sparse-input profile.
