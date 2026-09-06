# GPU ideal dielectric radiance transport contract

Base0330100. Apply architecture14/15, M07, docs/m07_m08.md, scattering.rs semantics,
ADR025–027 and existing alpha/secondary-footprint fixture contracts. Fully Rust
runtime, immutable transactions, native/browser shared schemas and M09 deferral persist.

Admit existing ideal dielectric with validated IOR[1,3], roughness0 and metallic0.
Use geometric interface normals, exact dielectric Fresnel in the named f32 profile,
Snell transmission, total internal reflection and radiance eta-squared throughput.
Keep emission and sampled base tint, AO on indirect throughput, existing alpha
coverage, camera/secondary footprints and bounded shutter/sequence publication.
Entering and exiting orientation must match CPU even for one-sided authoring and
mirrored transforms. Refraction does not gain directly sampled point-light caustics.

Preserve the three prior generated shaders and all earlier image/pass/receipt
artifacts. Add a fourth lazy Rust-generated variant for scenes containing dielectric,
with existing surface records plus a dielectric tag/IOR. No public schema, dependency
or source material extension changes. GPU sparse media stays explicitly unsupported.

Require analytic normal-incidence/tinted transmission, IOR1, Snell direction/target,
entry/exit eta cancellation, TIR, grazing/back-facing and camera clipping cases.
Test ideal interfaces against perturbed shading normals; refraction must use geometry.
Include mixed diffuse/Principled/conductor/coat scenes, alpha/MASK/BLEND, texture/UV
and morph/shutter workflows. Validate static/frame/sequence cancellation, stale and
invalid parameters, output budgets, partial failure and OPFS/archive recovery.
Record source provenance, resources, precision restrictions, native/Wasm tests and
actual Metal/Chrome WebGPU results before marking the capability complete.
