# Next input increment after sparse accessor acceptance

Extend glTF MASK/BLEND import through existing typed scattering::Surface with
Model::Principled and Opacity::{Mask,Blend}. No schema or dependency change.
OPAQUE keeps advanced=None and ignores alpha as already reported. Factor is
baseColorFactor[3], default1; MASK cutoff defaults0.5; reject wrong types/nonfinite
or out-of-profile values. Source alphaCutoff >=0 spec has no upper bound; native
opacity cutoff <=1 currently bound, so preserve the exact threshold by extending the native cutoff domain to finite
nonnegative values. Tests admit >1 and reject negative/nonfinite values; no clamp. PBR alpha texture channel stays linear despite
sRGB color decoding. Existing metadata/URI isolation and transaction paths apply.

Target is native/browser Rust CPU, matching already-implemented extended surface
profile. GPU advanced/alpha remains explicit unsupported until roadmap step3.
Native/browser CPU workflow, save/reload, analytic coverage, negative/stale/cancel
and unsupported GPU diagnostics complete this input adapter subgate. Do not call
all INPUT-02 complete or claim GPU alpha. Multiple UVs and normal morphs remain
subsequent independent input work.

Code inspection found two existing extended CPU primary-ray issues to fix and
prove with analytic tests while adding imported alpha:
- after transparent skip, camera near/far clipping is discarded; keep the original
primary ray interval and subtract accumulated origin advancement until first
accepted surface. Orthographic origin differs from camera center.
- depth currently uses Euclidean camera-position distance, wrong off-center for
orthographic rays; use distance from original camera ray origin along the ray.
Opaque perspective historical fixtures should retain bytes; test them.
Keep existing advanced material random dimensions/BSDF semantics otherwise.

Fixtures: original layered planes/boxes behind an RGBA textured quad, factors0/0.5/1,
MASK cutoff boundaries, opaque alphaignored reference. Known transmittance through
multiple alpha layers, double-sided/backface behavior, camera clip transparency,
visibility object/depth nominal passes, sampled coverage distribution seeded so
independent numerical bounds meaningful. PNG generated directly, CC0 fixturebytes.

Relevant code: crates/core/src/gltf_materials.rs::surface rejects nonOPAQUE;
crates/core/src/gltf_scene.rs material scalar factors/report; scattering.rs opacity,
visibility, render. GPU pack rejects every advanced surface today. Existing engine
CPU passes/image and transactions retain native typed semantics.

Reproduced baseline through ordinary native CLI on 2026-09-06:
`artifacts/alpha-baseline/run-20260906T050222Z/baseline_report.json`.
Scattering source verified byte-identical to 5416cfa. A transparent front box lets
all 64 pixels see an emitter beyond far=2.5, expected0 visible pixels. Orthographic
rays should report depth3 for the planar emitter but report3.005012..3.251715.
The standalone reproduction script is `/tmp/render-alpha-baseline.py`.
