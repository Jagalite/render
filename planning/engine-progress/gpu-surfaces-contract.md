# GPU conductor and single-interface coat contract

Basee23cf86. Applicable requirements: architecture sections14/15, M07, ADR025/026,
docs/m07_m08.md, docs/multibounce_pbr.md and secondary_textures.md. Preserve the fully
Rust runtime, native/browser shared semantics, transactions and M09 deferral.

Admit existing typed conductor and coated surfaces in a third lazy Rust-generated
GPU variant. Preserve both earlier generated kernels and existing rendered artifacts
byte for byte. No external material extension admission, public schema or dependency
changes. Dielectric refraction and sparse media retain explicit unsupported errors.

Conductor uses the existing eta/k-derived Schlick F0 and GGX distribution/PDF.
Coat uses the existing exact dielectric Fresnel attenuation, weighted single-interface
GGX lobe and matching mixture PDF. Preserve extended random dimensions2/3/4/5,
primary/secondary filtering, AO on indirect throughput, alpha, clipping and bounded
failure publication. This is the existing named approximation, not arbitrary layers.

Require analytic conductor/coat boundary checks, sampled CPU/Metal comparisons,
positive energy and changed-material observability, texture/normal/UV/color/alpha
integration, cache switching across all three pipelines, depth and shutter/sequence
workflows, cancellation/stale/invalid/budget checks, native/Wasm tests, actual Chrome
WebGPU/OPFS, reproducible source recipe, resources and package/shader provenance.
Only update capability status after complete native and browser workflow acceptance.
