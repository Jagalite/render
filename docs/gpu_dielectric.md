# GPU ideal dielectric interfaces

The `gpu-f32-dielectric-v1` profile implements the existing native ideal dielectric
model on Metal and browser WebGPU. It accepts IOR[1,3], roughness0 and metallic0,
with the ordinary typed surface and alpha policies. Static scenes, exact-time frames
and streaming shutter sequences use the same immutable evaluator and publication
paths. Sparse participating media remains an explicit unsupported GPU profile.
This does not add a glTF transmission/volume material extension importer.

Each geometric interface selects air→material or material→air from its front/back
orientation. Exact dielectric Fresnel determines stochastic reflection; Snell's law
determines transmission, with deterministic total internal reflection. Transmission
multiplies sampled base tint by eta² for radiance transport. Reflection keeps unit
weight. Entry/exit factors cancel through a parallel slab; tint applies at each
transmitted interface. The model assumes an air/material interface at each face,
with no nested-medium IOR stack, absorption thickness or rough transmission.

Authored one-sided material flags do not cull an ideal dielectric's exit interface,
matching the CPU model. Geometric normals determine optics and the signed secondary
ray offset. Smooth or normal-mapped shading normals remain available for diagnostic
passes but cannot bend refraction. PBR mirrored-transform orientation follows the
existing inverse-transpose policy. Camera clipping applies to primary traversal;
subsequent rays use the existing secondary epsilon and unbounded camera interval.

Emission is accumulated before scattering; dielectric has no directly evaluated
point-light BRDF. Alpha MASK/BLEND coverage precedes optics and does not consume
BSDF depth when discarded. Indirect AO multiplies outgoing throughput. Primary
textures retain camera footprints and secondary textures use base-level magnification.
The visibility approximation treats covered glass as opaque to direct point-light
shadow rays; directly sampled glass caustics and MIS remain separate transport gates.

The private material record adds tag3 and IOR using the existing two-vec4 shape.
Param9.w selects profile3. A fourth Rust-generated variant and lazy pipeline preserve
the prior opaque, Principled-alpha and conductor/coat shaders exactly. The new variant
can render all existing surface models and legacy diffuse together. Its own pipeline
layout, packed geometry cache identity and allocation/error scopes follow existing
contracts. No public schema, ABI or runtime dependency change is introduced.

Optical parameters, Fresnel, refraction and sampled branch thresholds use f32.
Near critical angles, grazing incidence, very close IORs or branch thresholds,
CPU f64 and GPU paths can diverge. Finite depth, geometric epsilon and base-level
secondary filtering remain explicit approximations. Analytic and cross-platform
acceptance qualifies the named corpus; it is not universal pixel identity.

Reproduce with `python3 scripts/validate-gpu-dielectric.py`, serving the exact
packaged browser at localhost:8774 and isolated Chrome CDP at9224. Original CC0
source geometry and analytic recipes regenerate with
`python3 fixtures/gpu-dielectric/generate.py <new-directory>`. The independent
Python recipe computes sampled analytic expectations without reading renderer
outputs. `render-host kernel --dielectric` emits the generated artifact.
See [acceptance evidence](../evidence/gpu-dielectric/README.md).
