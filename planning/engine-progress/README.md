# Autonomous engine roadmap progress

User direction: continue engine-first work, park concrete major blockers, keep
working on feasible remaining steps. M09 interactive editor remains deferred.
Each completed increment requires implementation, native/browser workflow evidence,
negative/cancellation/stale checks, resource/provenance records and documentation.

Base checkpoint: `4bfe27f` (validated multi-bounce opaque transport).

| Order | Increment | Status |
|---|---|---|
| 1 | GPU shutter frames and streaming sequences | Validated on feat/gpu-shutter-sequences; evidence/gpu-shutter |
| 2 | Broader glTF attributes, UV/alpha/morph semantics and export | Next: sparse accessors and normalized UV0 |
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
